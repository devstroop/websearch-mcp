// ---------------------------------------------------------------------------
// session/types.rs — Shared type definitions for the session module
// ---------------------------------------------------------------------------

use chromiumoxide::cdp::browser_protocol::page::EventJavascriptDialogOpening;
use chromiumoxide::listeners::EventStream;
use chromiumoxide::page::Page;
use serde::Serialize;

/// Information about a pending JavaScript dialog, surfaced to the LLM.
#[derive(Debug, Clone, Serialize)]
pub(crate) struct DialogInfo {
    #[serde(rename = "type")]
    pub(crate) dialog_type: String,
    pub(crate) message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) default_prompt: Option<String>,
}

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
pub(crate) struct ManagedTab {
    pub(crate) page: Page,
    pub(crate) url: String,
    pub(crate) title: String,
    /// When this tab was last interacted with (for idle tracking).
    pub(crate) last_active: std::time::Instant,
    /// CDP event stream for Page.javascriptDialogOpening events.
    pub(crate) dialog_listener: Option<EventStream<EventJavascriptDialogOpening>>,
    /// Most recent pending dialog info (cleared after handling).
    pub(crate) pending_dialog: Option<DialogInfo>,
}

/// Structured search result returned to the MCP tool handler.
#[derive(Debug, Clone)]
pub struct SearchResult {
    pub provider: String,
    pub query: String,
    pub snippets: Vec<SearchSnippet>,
    pub raw_markdown: String,
    pub tab_id: String,
}

#[derive(Debug, Clone)]
pub struct SearchSnippet {
    pub title: String,
    pub url: String,
    pub description: String,
}

/// A single field descriptor for `fill_form`.
#[derive(Debug, Clone)]
pub struct FormField {
    pub selector: String,
    pub value: String,
}
