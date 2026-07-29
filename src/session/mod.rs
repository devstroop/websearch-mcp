// ---------------------------------------------------------------------------
// session/mod.rs — Session module root
//
// This module owns the `SessionManager` struct (the stateful brain between
// MCP tools and the raw browser), its constructor, internal helpers, and
// the HTML→Markdown conversion pipeline.
//
// Method implementations are spread across sub-modules by concern:
//   tabs.rs     — Tab lifecycle (open/close/focus/list/idle/recovery)
//   search.rs   — Persistent search tab with reuse semantics
//   navigation.rs — Nav (navigate/back/forward/reload)
//   interaction.rs — Behavioral actions (click/type/hover/select/drag/upload)
//   dialog.rs   — CDP-based JavaScript dialog handling
//   wait.rs     — Wait-for-condition and wait-for-selector
//   content.rs  — Content extraction, snapshot, find, screenshot, evaluate
//   stealth.rs  — Anti-detection stealth script constant
//   types.rs    — Shared types (TabInfo, ManagedTab, SearchResult, etc.)
// ---------------------------------------------------------------------------

pub(crate) mod dialog;
pub(crate) mod interaction;
pub(crate) mod navigation;
pub(crate) mod search;
pub(crate) mod tabs;
pub(crate) mod wait;

mod content;
mod stealth;
mod types;

use std::collections::HashMap;
use std::sync::Arc;

use chromiumoxide::browser::Browser;
use chromiumoxide::page::Page;
use tokio::sync::Mutex;
use tracing::info;

use crate::error::{Error, Result as LibResult};

// Re-export public types.
pub use self::types::{FormField, SearchResult, TabInfo};

// Re-export stealth for sub-modules (via super::STEALTH_JS).
pub(crate) use self::stealth::STEALTH_JS;

// Re-export internal types for sub-modules (via super::types::{...}).
pub(crate) use self::types::ManagedTab;

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
    /// The dedicated search tab — created once, reused across `search()` calls.
    /// Reset to `None` if the tab is closed, then recreated on next search.
    search_tab_id: Option<String>,
    /// Origin coordinates from the last `drag_from` call (for follow-up `drop_to`).
    drag_origin: Option<(f64, f64)>,
    /// Seconds to wait for JS rendering.
    wait_seconds: u64,
    /// Whether stealth patches should be applied (headless mode only).
    headless: bool,
}

impl SessionManager {
    /// Create a new session manager, recovering existing tabs if possible.
    pub async fn new(
        browser: Arc<Mutex<Browser>>,
        wait_seconds: u64,
        headless: bool,
    ) -> LibResult<Self> {
        let mut session = Self {
            browser,
            tabs: HashMap::new(),
            active_tab_id: None,
            search_tab_id: None,
            drag_origin: None,
            wait_seconds,
            headless,
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

    // ----- Internal Helpers (used by all sub-modules) -----

    /// Get a mutable reference to the active managed tab.
    pub(crate) fn get_active_tab_mut(&mut self) -> LibResult<&mut ManagedTab> {
        let id = self.active_tab_id.clone().ok_or_else(|| {
            Error::Tab("no active tab — use browser_open to create a tab first".into())
        })?;
        self.tabs
            .get_mut(&id)
            .ok_or_else(|| Error::Tab(format!("active tab {id} not found in session")))
    }

    /// Get a reference to the active page.
    pub(crate) fn get_active_page(&self) -> LibResult<&Page> {
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
    pub(crate) fn get_tab_info(&self, id: &str) -> TabInfo {
        let tab = self.tabs.get(id);
        TabInfo {
            id: id.to_string(),
            url: tab.map(|t| t.url.clone()).unwrap_or_default(),
            title: tab.map(|t| t.title.clone()).unwrap_or_default(),
            active: self.active_tab_id.as_deref() == Some(id),
        }
    }
}

// ---------------------------------------------------------------------------
// HTML → Markdown pipeline
// ---------------------------------------------------------------------------

/// Convert raw HTML to clean Markdown using the cleanup pipeline.
pub(crate) fn html_to_markdown(html: &str) -> LibResult<String> {
    let cleaned = crate::cleanup::strip_noise(html);
    let result = html_to_markdown_rs::convert(&cleaned, None)
        .map_err(|e| Error::MarkdownConversion(e.to_string()))?;
    let md = result.content.unwrap_or_default();
    let md = crate::cleanup::clean_markdown(&md);
    Ok(md)
}
