// ---------------------------------------------------------------------------
// session.rs — Browser session manager with persistent tab state
//
// Responsibilities:
//   - Track open tabs in a HashMap<TargetId, ManagedTab>
//   - Manage active tab context for tool calls
//   - Provide high-level interaction: navigate, click, type, get content
//   - Recover existing browser tabs on startup
//   - Apply anti-detection stealth patches
//
// This module is the stateful brain between the MCP tools and the raw
// chromiumoxide Browser/Page objects.
// ---------------------------------------------------------------------------

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use base64::Engine;
use chromiumoxide::browser::Browser;
use chromiumoxide::page::{Page, ScreenshotParams};
use tokio::sync::Mutex;
use tracing::{info, warn};

use crate::cleanup;
use crate::error::{Error, Result as LibResult};

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/// Information about a managed tab, returned to tool callers.
#[derive(Debug, Clone)]
pub struct TabInfo {
    /// Unique target ID (from Chrome DevTools Protocol).
    pub id: String,
    /// Current URL of the tab.
    pub url: String,
    /// Page title (may be empty).
    pub title: String,
    /// Whether this is the active (focused) tab.
    pub active: bool,
}

/// Internal representation of a managed tab.
struct ManagedTab {
    page: Page,
    url: String,
    title: String,
}

// ---------------------------------------------------------------------------
// SessionManager
// ---------------------------------------------------------------------------

/// Manages browser tabs and provides high-level interaction methods.
///
/// All tool calls that operate on tabs go through this manager. The manager
/// maintains a `HashMap<TargetId, ManagedTab>` to track all open tabs and
/// an `active_tab_id` for the current focus context.
pub struct SessionManager {
    /// Shared browser handle (persistent Chrome/Chromium instance).
    browser: Arc<Mutex<Browser>>,
    /// All tracked tabs, keyed by TargetId string.
    tabs: HashMap<String, ManagedTab>,
    /// The currently active (focused) tab ID.
    active_tab_id: Option<String>,
    /// Seconds to wait for JS rendering.
    wait_seconds: u64,
}

impl SessionManager {
    /// Create a new session manager, recovering existing tabs if possible.
    pub async fn new(browser: Arc<Mutex<Browser>>, wait_seconds: u64) -> LibResult<Self> {
        let mut session = Self {
            browser,
            tabs: HashMap::new(),
            active_tab_id: None,
            wait_seconds,
        };

        // Try to recover existing browser tabs from the persistent session.
        session.recover_tabs().await;

        info!(
            "session initialized — {} tabs recovered, active={:?}",
            session.tabs.len(),
            session.active_tab_id
        );

        Ok(session)
    }

    // ----- Tab Lifecycle -----

    /// Open a new tab, optionally navigating to a URL.
    ///
    /// Returns info about the newly created tab.
    /// If `activate` is true (or no tab exists yet), the new tab becomes active.
    pub async fn open_tab(&mut self, url: Option<&str>, activate: bool) -> LibResult<TabInfo> {
        let page = {
            let browser = self.browser.lock().await;
            let target_url = url.unwrap_or("about:blank");
            browser
                .new_page(target_url)
                .await
                .map_err(|e| Error::Browser(format!("failed to open new tab: {e}")))?
        };

        // Apply anti-detection stealth patches.
        let _ = page.enable_stealth_mode().await;

        let target_id = page.target_id().as_ref().to_string();
        let current_url = url.unwrap_or("about:blank").to_string();

        let tab = ManagedTab {
            page,
            url: current_url.clone(),
            title: String::new(),
        };

        self.tabs.insert(target_id.clone(), tab);

        if activate || self.active_tab_id.is_none() {
            self.active_tab_id = Some(target_id.clone());
        }

        info!("opened tab {} → {}", &target_id[..8], current_url);
        Ok(self.get_tab_info(&target_id))
    }

    /// Close a tab by ID. If no ID is given, close the active tab.
    pub async fn close_tab(&mut self, tab_id: Option<&str>) -> LibResult<()> {
        let id = match tab_id {
            Some(id) => id.to_string(),
            None => self
                .active_tab_id
                .clone()
                .ok_or_else(|| Error::Tab("no active tab to close".into()))?,
        };

        let tab = self
            .tabs
            .remove(&id)
            .ok_or_else(|| Error::Tab(format!("tab not found: {id}")))?;

        if let Err(e) = tab.page.close().await {
            warn!("failed to close tab {id}: {e}");
        }

        // If we closed the active tab, focus another one.
        if self.active_tab_id.as_deref() == Some(&id) {
            self.active_tab_id = self.tabs.keys().next().cloned();
        }

        info!("closed tab {id}");
        Ok(())
    }

    /// Focus a tab by ID, making it the active context for subsequent tools.
    pub async fn focus_tab(&mut self, tab_id: &str) -> LibResult<()> {
        if !self.tabs.contains_key(tab_id) {
            return Err(Error::Tab(format!("tab not found: {tab_id}")));
        }
        self.active_tab_id = Some(tab_id.to_string());
        Ok(())
    }

    /// List all open tabs.
    pub fn list_tabs(&self) -> Vec<TabInfo> {
        self.tabs
            .iter()
            .map(|(id, tab)| TabInfo {
                id: id.clone(),
                url: tab.url.clone(),
                title: tab.title.clone(),
                active: self.active_tab_id.as_deref() == Some(id.as_str()),
            })
            .collect()
    }

    /// Get the active tab ID, if any.
    pub fn active_tab_id(&self) -> Option<&str> {
        self.active_tab_id.as_deref()
    }

    // ----- Navigation -----

    /// Navigate the active tab to a URL.
    pub async fn navigate(&mut self, url: &str) -> LibResult<()> {
        let page = self.get_active_page()?;
        let wait = Duration::from_secs(self.wait_seconds);

        tokio::time::timeout(wait, page.goto(url))
            .await
            .map_err(|_| Error::NavigationTimeout(self.wait_seconds))?
            .map_err(|e| Error::Navigation(e.to_string()))?;

        // Re-apply stealth after navigation (some sites check post-load).
        let _ = page.enable_stealth_mode().await;

        // Allow JS to render and network to settle.
        tokio::time::sleep(wait).await;

        // Update tracked URL and title.
        let id_clone = self.active_tab_id.clone();
        if let Some(id) = &id_clone {
            if let Some(tab) = self.tabs.get_mut(id) {
                tab.url = url.to_string();
            }
        }
        // Update title separately to avoid borrow conflict.
        if let Some(id) = &id_clone {
            if let Some(tab) = self.tabs.get(id) {
                if let Ok(Some(title)) = tab.page.get_title().await {
                    if let Some(tab) = self.tabs.get_mut(id) {
                        tab.title = title;
                    }
                }
            }
        }

        info!("navigated active tab to {url}");
        Ok(())
    }

    /// Go back in browser history (active tab).
    pub async fn back(&mut self) -> LibResult<()> {
        let page = self.get_active_page()?;
        page.goto("javascript:history.back()")
            .await
            .map_err(|e| Error::Browser(format!("go back failed: {e}")))?;
        tokio::time::sleep(Duration::from_secs(2)).await;
        self.update_active_tab_metadata().await;
        Ok(())
    }

    /// Go forward in browser history (active tab).
    pub async fn forward(&mut self) -> LibResult<()> {
        let page = self.get_active_page()?;
        page.goto("javascript:history.forward()")
            .await
            .map_err(|e| Error::Browser(format!("go forward failed: {e}")))?;
        tokio::time::sleep(Duration::from_secs(2)).await;
        self.update_active_tab_metadata().await;
        Ok(())
    }

    /// Reload the current page (active tab).
    pub async fn reload(&mut self) -> LibResult<()> {
        let page = self.get_active_page()?;
        page.reload()
            .await
            .map_err(|e| Error::Browser(format!("reload failed: {e}")))?;
        tokio::time::sleep(Duration::from_secs(self.wait_seconds)).await;
        self.update_active_tab_metadata().await;
        Ok(())
    }

    // ----- Interaction -----

    /// Click an element by CSS selector on the active tab.
    pub async fn click(&mut self, selector: &str) -> LibResult<()> {
        let page = self.get_active_page()?;
        let element = page
            .find_element(selector)
            .await
            .map_err(|e| Error::ElementNotFound(format!("{selector}: {e}")))?;
        element
            .click()
            .await
            .map_err(|e| Error::Browser(format!("click failed on {selector}: {e}")))?;
        info!("clicked element: {selector}");
        Ok(())
    }

    /// Type text into an element by CSS selector, optionally pressing Enter.
    pub async fn type_text(&mut self, selector: &str, text: &str, submit: bool) -> LibResult<()> {
        let page = self.get_active_page()?;
        let element = page
            .find_element(selector)
            .await
            .map_err(|e| Error::ElementNotFound(format!("{selector}: {e}")))?;

        // Focus the element first by clicking it.
        element
            .click()
            .await
            .map_err(|e| Error::Browser(format!("focus click failed on {selector}: {e}")))?;

        // Type the text.
        element
            .type_str(text)
            .await
            .map_err(|e| Error::Browser(format!("type failed on {selector}: {e}")))?;

        // Optionally submit with Enter.
        if submit {
            element
                .press_key("Enter")
                .await
                .map_err(|e| Error::Browser(format!("submit key failed: {e}")))?;
        }

        info!("typed into {selector} (submit={submit})");
        Ok(())
    }

    // ----- Content Extraction -----

    /// Get the raw HTML of the active tab.
    pub async fn get_html(&mut self) -> LibResult<String> {
        let page = self.get_active_page()?;
        page.content()
            .await
            .map_err(|e| Error::Browser(format!("failed to get page HTML: {e}")))
    }

    /// Get the active tab's content as clean Markdown.
    ///
    /// Waits for the page to finish rendering (CSS, JS), then runs the full
    /// cleanup pipeline: HTML noise stripping → Markdown conversion →
    /// Markdown post-processing.
    pub async fn get_content(&mut self) -> LibResult<String> {
        // Wait for page to finish rendering before extraction.
        let wait = Duration::from_secs(self.wait_seconds);
        tokio::time::sleep(wait).await;

        let html = self.get_html().await?;
        let md = html_to_markdown(&html)?;
        Ok(md)
    }

    /// Take a screenshot of the active tab, returning base64-encoded PNG.
    pub async fn screenshot(&mut self, full_page: bool) -> LibResult<String> {
        let page = self.get_active_page()?;
        let params = ScreenshotParams::builder()
            .full_page(full_page)
            .capture_beyond_viewport(full_page)
            .build();
        let bytes = page
            .screenshot(params)
            .await
            .map_err(|e| Error::Screenshot(e.to_string()))?;
        Ok(base64::engine::general_purpose::STANDARD.encode(&bytes))
    }

    /// Execute JavaScript in the active tab and return the result as a JSON string.
    pub async fn evaluate(&mut self, script: &str) -> LibResult<String> {
        let page = self.get_active_page()?;
        let result = page
            .evaluate(script)
            .await
            .map_err(|e| Error::Browser(format!("evaluate failed: {e}")))?;
        // Convert EvaluationResult to a JSON string.
        let value = result
            .into_value::<serde_json::Value>()
            .map_err(|e| Error::Browser(format!("evaluate result parse failed: {e}")))?;
        Ok(serde_json::to_string_pretty(&value).unwrap_or_else(|_| "null".into()))
    }

    // ----- Internal Helpers -----

    /// Get a reference to the active page.
    fn get_active_page(&self) -> LibResult<&Page> {
        let id = self.active_tab_id.as_ref().ok_or_else(|| {
            Error::Tab("no active tab — use browser_open to create a tab first".into())
        })?;
        let tab = self
            .tabs
            .get(id)
            .ok_or_else(|| Error::Tab(format!("active tab {id} not found in session")))?;
        Ok(&tab.page)
    }

    /// Get info about a tab by its ID.
    fn get_tab_info(&self, id: &str) -> TabInfo {
        let tab = self.tabs.get(id);
        TabInfo {
            id: id.to_string(),
            url: tab.map(|t| t.url.clone()).unwrap_or_default(),
            title: tab.map(|t| t.title.clone()).unwrap_or_default(),
            active: self.active_tab_id.as_deref() == Some(id),
        }
    }

    /// Update URL and title metadata for the active tab.
    async fn update_active_tab_metadata(&mut self) {
        if let Some(id) = self.active_tab_id.clone() {
            if let Some(tab) = self.tabs.get_mut(&id) {
                if let Ok(Some(url)) = tab.page.url().await {
                    tab.url = url;
                }
                if let Ok(Some(title)) = tab.page.get_title().await {
                    tab.title = title;
                }
            }
        }
    }

    /// Try to discover and register existing browser tabs.
    ///
    /// Called once during `new()`. Uses `Browser::pages()` to enumerate
    /// all existing CDP targets and register them as managed tabs.
    async fn recover_tabs(&mut self) {
        let pages = {
            let browser = self.browser.lock().await;
            match browser.pages().await {
                Ok(pages) => pages,
                Err(e) => {
                    warn!("failed to enumerate existing tabs: {e}");
                    return;
                }
            }
        };
        // Browser lock is now released — safe to call async methods on pages.

        for page in pages {
            let target_id = page.target_id().as_ref().to_string();
            let url = page.url().await.unwrap_or(None).unwrap_or_default();
            let title = page.get_title().await.unwrap_or(None).unwrap_or_default();

            info!("recovered tab {target_id} → {url}");

            let tab = ManagedTab { page, url, title };
            self.tabs.insert(target_id.clone(), tab);

            if self.active_tab_id.is_none() {
                self.active_tab_id = Some(target_id);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// HTML → Markdown pipeline (shared with providers/navigate.rs)
// ---------------------------------------------------------------------------

/// Convert raw HTML to clean Markdown using the cleanup pipeline.
fn html_to_markdown(html: &str) -> LibResult<String> {
    let cleaned = cleanup::strip_noise(html);
    let result = html_to_markdown_rs::convert(&cleaned, None)
        .map_err(|e| Error::MarkdownConversion(e.to_string()))?;
    let md = result.content.unwrap_or_default();
    let md = cleanup::clean_markdown(&md);
    Ok(md)
}
