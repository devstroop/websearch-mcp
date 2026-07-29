// ---------------------------------------------------------------------------
// session/tabs.rs — Tab lifecycle management
//
// Responsibilities:
//   - Open, close, focus, list browser tabs
//   - Track active tab and search tab IDs
//   - Idle tracking and auto-close of stale tabs
//   - Recover existing tabs on startup
// ---------------------------------------------------------------------------

use std::time::Duration;

use chromiumoxide::cdp::browser_protocol::page::{self as page_cdp, EventJavascriptDialogOpening};
use tracing::{info, warn};

use super::types::ManagedTab;
use super::{Error, LibResult, SessionManager, STEALTH_JS};
use crate::session::TabInfo;

impl SessionManager {
    /// Open a new tab, optionally navigating to a URL.
    ///
    /// Returns info about the newly created tab.
    /// If `activate` is true (or no tab exists yet), the new tab becomes active.
    pub async fn open_tab(&mut self, url: Option<&str>, activate: bool) -> LibResult<TabInfo> {
        // STEP 1: Create the page at about:blank first (fast, no risk of missing dialogs).
        let page = {
            let browser = self.browser.lock().await;
            browser
                .new_page("about:blank")
                .await
                .map_err(|e| Error::Browser(format!("failed to open new tab: {e}")))?
        };

        // STEP 2: Enable Page domain and register dialog listener BEFORE navigating
        // to the real URL. This ensures we don't miss alert()/confirm()/prompt()
        // that fire on page load.
        if let Err(e) = page.execute(page_cdp::EnableParams::default()).await {
            warn!("failed to enable Page domain for dialog events: {e}");
        }

        let dialog_listener = match page
            .event_listener::<EventJavascriptDialogOpening>()
            .await
        {
            Ok(listener) => Some(listener),
            Err(e) => {
                warn!("failed to register dialog event listener: {e}");
                None
            }
        };

        // STEP 3: Apply anti-detection stealth patches before navigation.
        if self.headless {
            let _ = page.evaluate_on_new_document(STEALTH_JS).await;
        }

        // STEP 4: Now navigate to the real URL (if any).
        if let Some(target_url) = url {
            let wait = Duration::from_secs(self.wait_seconds);
            match tokio::time::timeout(wait, page.goto(target_url)).await {
                Ok(Ok(_page)) => {} // success
                Ok(Err(e)) => {
                    let _ = page.close().await;
                    return Err(Error::Navigation(e.to_string()));
                }
                Err(_) => {
                    let _ = page.close().await;
                    return Err(Error::NavigationTimeout(self.wait_seconds));
                }
            }
        }

        let target_id = page.target_id().as_ref().to_string();
        let current_url = url.unwrap_or("about:blank").to_string();

        let tab = ManagedTab {
            page,
            url: current_url.clone(),
            title: String::new(),
            last_active: std::time::Instant::now(),
            dialog_listener,
            pending_dialog: None,
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

        // If we closed the search tab, reset search_tab_id.
        if self.search_tab_id.as_deref() == Some(&id) {
            self.search_tab_id = None;
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
        self.touch_tab(tab_id);
        Ok(())
    }

    /// List all open tabs.
    pub fn list_tabs(&self) -> Vec<crate::session::TabInfo> {
        self.tabs
            .iter()
            .map(|(id, tab)| crate::session::TabInfo {
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

    /// Get the dedicated search tab ID, if any.
    pub fn search_tab_id(&self) -> Option<&str> {
        self.search_tab_id.as_deref()
    }

    // ----- Tab Lifecycle — idle tracking -----

    /// Mark the active tab as recently used.
    pub(crate) fn touch_active_tab(&mut self) {
        if let Some(id) = self.active_tab_id.clone() {
            self.touch_tab(&id);
        }
    }

    /// Mark a specific tab as recently used.
    pub fn touch_tab(&mut self, tab_id: &str) {
        if let Some(tab) = self.tabs.get_mut(tab_id) {
            tab.last_active = std::time::Instant::now();
        }
    }

    /// Close all tabs that have been idle longer than `timeout`, except the
    /// search tab and the currently active tab. Returns the number closed.
    pub async fn close_idle_tabs(&mut self, timeout: std::time::Duration) -> usize {
        let now = std::time::Instant::now();
        let active = self.active_tab_id.clone();
        let search = self.search_tab_id.clone();
        let mut to_close: Vec<String> = Vec::new();

        for (id, tab) in &self.tabs {
            if Some(id.as_str()) == active.as_deref() {
                continue;
            }
            if Some(id.as_str()) == search.as_deref() {
                continue;
            }
            if now.duration_since(tab.last_active) >= timeout {
                to_close.push(id.clone());
            }
        }

        let count = to_close.len();
        for id in &to_close {
            if let Some(tab) = self.tabs.remove(id) {
                if let Err(e) = tab.page.close().await {
                    warn!("failed to close idle tab {id}: {e}");
                }
            }
            info!("closed idle tab {id}");
        }
        count
    }

    /// Try to discover and register existing browser tabs.
    ///
    /// Called once during `new()`. Uses `Browser::pages()` to enumerate
    /// all existing CDP targets and register them as managed tabs.
    pub(crate) async fn recover_tabs(&mut self) {
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

            // Enable Page domain and set up dialog listener for recovered tabs too.
            if let Err(e) = page.execute(page_cdp::EnableParams::default()).await {
                warn!("failed to enable Page domain on recovered tab: {e}");
            }
            let dialog_listener = match page
                .event_listener::<EventJavascriptDialogOpening>()
                .await
            {
                Ok(listener) => Some(listener),
                Err(e) => {
                    warn!("failed to register dialog listener on recovered tab: {e}");
                    None
                }
            };

            info!("recovered tab {target_id} → {url}");

            let tab = ManagedTab {
                page,
                url,
                title,
                last_active: std::time::Instant::now(),
                dialog_listener,
                pending_dialog: None,
            };
            self.tabs.insert(target_id.clone(), tab);

            if self.active_tab_id.is_none() {
                self.active_tab_id = Some(target_id);
            }
        }
    }
}
