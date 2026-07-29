// ---------------------------------------------------------------------------
// tools/mod.rs — MCP tool definitions and server struct
//
// This module defines the MCP server struct (`WebSearchServer`) and its
// tool handlers via the rmcp `#[tool_router]` macro. The actual handler
// logic lives in sibling modules — this file is the routing layer only.
//
// Tool groups:
//   - search/fetch: high-level convenience tools using search providers
//   - browser_*:    granular DevTools-style browser interaction tools
// ---------------------------------------------------------------------------

pub mod browser_tools;
pub mod fetch;
pub mod search;

use std::sync::Arc;

use rmcp::{handler::server::wrapper::Parameters, schemars, tool, tool_router};
use serde::Deserialize;
use tokio::sync::Mutex;

use crate::registry;
use crate::session;

// ---------------------------------------------------------------------------
// MCP parameter schemas — search & fetch
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct SearchParams {
    /// The search query.
    pub query: String,
    /// Search engine to use: "brave" (default, recommended), "duckduckgo",
    /// or "google".
    #[serde(default = "default_provider")]
    pub provider: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct FetchParams {
    /// The URL to fetch and convert to clean Markdown.
    /// Only http:// and https:// schemes are supported.
    pub url: String,
}

fn default_provider() -> String {
    "brave".into()
}

// ---------------------------------------------------------------------------
// MCP parameter schemas — browser tab management
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct BrowserOpenParams {
    /// URL to navigate to. Opens about:blank if omitted.
    #[serde(default)]
    pub url: Option<String>,
    /// Whether to make this the active (focused) tab. Defaults to true.
    #[serde(default = "default_true")]
    pub activate: Option<bool>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct BrowserFocusParams {
    /// The tab ID to focus.
    pub tab_id: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct BrowserCloseParams {
    /// The tab ID to close. Closes the active tab if omitted.
    #[serde(default)]
    pub tab_id: Option<String>,
}

fn default_true() -> Option<bool> {
    Some(true)
}

// ---------------------------------------------------------------------------
// MCP parameter schemas — navigation
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct BrowserNavigateParams {
    /// The URL to navigate to.
    pub url: String,
}

// ---------------------------------------------------------------------------
// MCP parameter schemas — interaction
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct BrowserClickParams {
    /// CSS selector for the element to click (e.g. "button.submit", "#login").
    pub selector: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct BrowserTypeParams {
    /// CSS selector for the input element (e.g. "input[name=q]", "textarea").
    pub selector: String,
    /// Text to type into the element.
    pub text: String,
    /// Whether to press Enter after typing (useful for search boxes). Defaults to false.
    #[serde(default)]
    pub submit: Option<bool>,
}

// ----- Phase 3 — Behavioral interaction params -----

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct BrowserHoverParams {
    /// CSS selector for the element to hover over.
    pub selector: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct BrowserSelectOptionParams {
    /// CSS selector for the `<select>` element.
    pub selector: String,
    /// The option value to select.
    pub value: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct BrowserFillFormField {
    /// CSS selector for the input element.
    pub selector: String,
    /// Value to fill into the element.
    pub value: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct BrowserFillFormParams {
    /// Array of field selectors and values to fill.
    pub fields: Vec<BrowserFillFormField>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct BrowserDragParams {
    /// CSS selector of the element to start dragging from.
    pub from_selector: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct BrowserDropParams {
    /// CSS selector of the element to drop onto.
    pub selector: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct BrowserFileUploadParams {
    /// CSS selector for the `<input type="file">` element.
    pub selector: String,
    /// Absolute path to the file to upload.
    pub file_path: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct BrowserPressKeyParams {
    /// Key to press (e.g. "Enter", "Tab", "Escape", "Control+a", "ArrowDown").
    pub key: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct BrowserResizeParams {
    /// New viewport width in pixels.
    pub width: u32,
    /// New viewport height in pixels.
    pub height: u32,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct BrowserWaitForParams {
    /// Condition to wait for: "load", "domcontentloaded", "networkidle",
    /// or "selector:CSS_SELECTOR".
    pub condition: String,
    /// Maximum time to wait in milliseconds (default: 10000).
    #[serde(default)]
    pub timeout_ms: Option<u64>,
}

// ----- Phase 2 — Snapshot params -----

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct BrowserSnapshotParams {
    /// Maximum number of nodes to return (default: 200).
    #[serde(default)]
    pub max_nodes: Option<usize>,
}

// ----- Phase 6 — Tab lifecycle params -----

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct BrowserCloseIdleParams {
    /// Close tabs idle longer than this many seconds.
    pub idle_seconds: u64,
}

// ----- Phase 5 — Dialog params -----

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct BrowserHandleDialogParams {
    /// "accept" (OK/Yes) or "dismiss" (Cancel/No). Default: "accept".
    #[serde(default = "default_dialog_action")]
    pub action: String,
    /// Text to type into a prompt dialog (for prompt dialogs only).
    #[serde(default)]
    pub prompt_text: Option<String>,
}

fn default_dialog_action() -> String {
    "accept".into()
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct BrowserWaitForSelectorParams {
    /// CSS selector to wait for.
    pub selector: String,
    /// Element state: "attached", "detached", "visible" (default), or "hidden".
    #[serde(default)]
    pub state: Option<String>,
    /// Maximum time to wait in milliseconds (default: 10000).
    #[serde(default)]
    pub timeout_ms: Option<u64>,
}

// ----- Phase 4 — Find params -----

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct BrowserFindParams {
    /// Text to search for in element names, roles, or values.
    pub text: String,
    /// Optional element role to narrow the search (e.g. "button", "link",
    /// "textbox", "checkbox", "combobox", "img").
    #[serde(default)]
    pub role: Option<String>,
}

// ---------------------------------------------------------------------------
// MCP parameter schemas — content & screenshots
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct BrowserScreenshotParams {
    /// Whether to capture the full scrollable page. Defaults to false.
    #[serde(default)]
    pub full_page: Option<bool>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct BrowserEvaluateParams {
    /// JavaScript expression to evaluate in the page context.
    pub script: String,
}

// ---------------------------------------------------------------------------
// MCP server — holds wired dependencies, exposes all tools
// ---------------------------------------------------------------------------

/// Shared state for all MCP tool handlers.
///
/// Constructed once in `websearch::serve()` and cloned into each
/// request handler by rmcp.
#[derive(Clone)]
pub struct WebSearchServer {
    /// Provider registry (resolves "brave", "duckduckgo", "google").
    pub engine: Arc<registry::SearchEngine>,
    /// Browser session manager (persistent tabs, navigation, interaction).
    pub session: Arc<Mutex<session::SessionManager>>,
}

#[tool_router]
impl WebSearchServer {
    // ----- High-level convenience tools -----

    #[tool(
        description = "Search the web using a pluggable search engine provider. \
                       Supports: brave (default, recommended), duckduckgo, \
                       and google. Returns the rendered search page as \
                       clean Markdown for the AI to interpret naturally."
    )]
    async fn search(
        &self,
        Parameters(SearchParams { query, provider }): Parameters<SearchParams>,
    ) -> String {
        search::handle(self, query, provider).await
    }

    #[tool(description = "Fetch a URL and return its rendered content as clean \
                       Markdown. The page is loaded in a real browser, \
                       JavaScript is executed, and non-content elements \
                       (nav, headers, footers, ads, tracking) are stripped \
                       automatically. Only http:// and https:// URLs are \
                       supported.")]
    async fn fetch(&self, Parameters(FetchParams { url }): Parameters<FetchParams>) -> String {
        fetch::handle(self, url).await
    }

    // ----- Browser tab management -----

    #[tool(
        description = "Open a new browser tab, optionally navigating to a URL. \
                       Returns the tab ID and info. The new tab becomes active \
                       by default."
    )]
    async fn browser_open(
        &self,
        Parameters(BrowserOpenParams { url, activate }): Parameters<BrowserOpenParams>,
    ) -> String {
        browser_tools::open_tab(self, url, activate).await
    }

    #[tool(description = "List all open browser tabs with their IDs, URLs, \
                       titles, and which one is active.")]
    async fn browser_tabs(&self) -> String {
        browser_tools::list_tabs(self).await
    }

    #[tool(description = "Switch the active (focused) browser tab by tab ID. \
                       Subsequent browser operations will target this tab.")]
    async fn browser_focus(
        &self,
        Parameters(BrowserFocusParams { tab_id }): Parameters<BrowserFocusParams>,
    ) -> String {
        browser_tools::focus_tab(self, tab_id).await
    }

    #[tool(description = "Close a browser tab. If no tab_id is given, closes \
                       the currently active tab.")]
    async fn browser_close(
        &self,
        Parameters(BrowserCloseParams { tab_id }): Parameters<BrowserCloseParams>,
    ) -> String {
        browser_tools::close_tab(self, tab_id).await
    }

    #[tool(description = "Close browser tabs that have been idle (not \
                       interacted with) longer than the specified number \
                       of seconds. The search tab and active tab are \
                       never auto-closed.")]
    async fn browser_close_idle(
        &self,
        Parameters(BrowserCloseIdleParams { idle_seconds }): Parameters<BrowserCloseIdleParams>,
    ) -> String {
        browser_tools::close_idle_tabs_action(self, idle_seconds).await
    }

    // ----- Navigation -----

    #[tool(description = "Navigate the active browser tab to a URL. Waits for \
                       the page to render before returning.")]
    async fn browser_navigate(
        &self,
        Parameters(BrowserNavigateParams { url }): Parameters<BrowserNavigateParams>,
    ) -> String {
        browser_tools::navigate(self, url).await
    }

    #[tool(description = "Go back in browser history (active tab).")]
    async fn browser_back(&self) -> String {
        browser_tools::go_back(self).await
    }

    #[tool(description = "Go forward in browser history (active tab).")]
    async fn browser_forward(&self) -> String {
        browser_tools::go_forward(self).await
    }

    #[tool(description = "Reload the current page in the active tab.")]
    async fn browser_reload(&self) -> String {
        browser_tools::reload_page(self).await
    }

    // ----- Interaction -----

    #[tool(description = "Click an element on the active tab by CSS selector. \
                       Example selectors: 'button.submit', '#login', \
                       'a[href=\"https://example.com\"]'.")]
    async fn browser_click(
        &self,
        Parameters(BrowserClickParams { selector }): Parameters<BrowserClickParams>,
    ) -> String {
        browser_tools::click_element(self, selector).await
    }

    #[tool(description = "Type text into an input element on the active tab \
                       by CSS selector. Optionally press Enter to submit. \
                       Example: selector='textarea[name=q]', text='search query', \
                       submit=true.")]
    async fn browser_type(
        &self,
        Parameters(BrowserTypeParams {
            selector,
            text,
            submit,
        }): Parameters<BrowserTypeParams>,
    ) -> String {
        browser_tools::type_text(self, selector, text, submit).await
    }

    // ----- Phase 3 — Behavioral interaction -----

    #[tool(description = "Hover over an element on the active tab by CSS \
                       selector. Useful for triggering hover menus, \
                       tooltips, and hover-based UI interactions.")]
    async fn browser_hover(
        &self,
        Parameters(BrowserHoverParams { selector }): Parameters<BrowserHoverParams>,
    ) -> String {
        browser_tools::hover_element(self, selector).await
    }

    #[tool(description = "Select an option in a `<select>` dropdown by CSS \
                       selector. The option value is matched against the \
                       `value` attribute of `<option>` elements.")]
    async fn browser_select_option(
        &self,
        Parameters(BrowserSelectOptionParams { selector, value }): Parameters<BrowserSelectOptionParams>,
    ) -> String {
        browser_tools::select_option_element(self, selector, value).await
    }

    #[tool(description = "Fill multiple form fields at once. Each field \
                       specifies a CSS selector and value. Fields are \
                       filled in order.")]
    async fn browser_fill_form(
        &self,
        Parameters(BrowserFillFormParams { fields }): Parameters<BrowserFillFormParams>,
    ) -> String {
        let form_fields: Vec<session::FormField> = fields
            .into_iter()
            .map(|f| session::FormField { selector: f.selector, value: f.value })
            .collect();
        browser_tools::fill_form_fields(self, form_fields).await
    }

    #[tool(description = "Start dragging an element from its current location \
                       by CSS selector. Follow up with browser_drop to \
                       complete the drag-and-drop operation.")]
    async fn browser_drag(
        &self,
        Parameters(BrowserDragParams { from_selector }): Parameters<BrowserDragParams>,
    ) -> String {
        browser_tools::drag_element(self, from_selector).await
    }

    #[tool(description = "Drop a previously dragged element onto a target \
                       element by CSS selector. Must be called after \
                       browser_drag.")]
    async fn browser_drop(
        &self,
        Parameters(BrowserDropParams { selector }): Parameters<BrowserDropParams>,
    ) -> String {
        browser_tools::drop_element(self, selector).await
    }

    #[tool(description = "Upload a file via an `<input type=\"file\">` \
                       element by CSS selector. Provide the absolute path \
                       to the file on the local filesystem.")]
    async fn browser_file_upload(
        &self,
        Parameters(BrowserFileUploadParams { selector, file_path }): Parameters<BrowserFileUploadParams>,
    ) -> String {
        browser_tools::upload_file(self, selector, file_path).await
    }

    #[tool(description = "Press a keyboard key or key combination on the \
                       active tab. Examples: \"Enter\", \"Tab\", \"Escape\", \
                       \"Control+a\", \"ArrowDown\", \"F5\".")]
    async fn browser_press_key(
        &self,
        Parameters(BrowserPressKeyParams { key }): Parameters<BrowserPressKeyParams>,
    ) -> String {
        browser_tools::press_key_input(self, key).await
    }

    #[tool(description = "Resize the browser viewport to the specified \
                       width and height in pixels.")]
    async fn browser_resize(
        &self,
        Parameters(BrowserResizeParams { width, height }): Parameters<BrowserResizeParams>,
    ) -> String {
        browser_tools::resize_viewport_size(self, width, height).await
    }

    #[tool(description = "Wait for a page condition to be met. Supported \
                       conditions: \"load\" (page fully loaded), \
                       \"domcontentloaded\" (DOM ready), \"networkidle\" \
                       (network idle), \"selector:CSS_SELECTOR\" (element \
                       exists in DOM).")]
    async fn browser_wait_for(
        &self,
        Parameters(BrowserWaitForParams { condition, timeout_ms }): Parameters<BrowserWaitForParams>,
    ) -> String {
        browser_tools::wait_for_condition_met(self, condition, timeout_ms).await
    }

    // ----- Phase 2 — Accessibility snapshot -----

    #[tool(description = "Get the accessibility tree of the active tab. \
                       Returns interactive elements as a compact tree: \
                       `<role \"name\" attr=val>`. Use this instead of \
                       browser_get_content when you need to understand \
                       interactive element structure (roles, states, \
                       names) rather than raw text.")]
    async fn browser_snapshot(
        &self,
        Parameters(BrowserSnapshotParams { max_nodes }): Parameters<BrowserSnapshotParams>,
    ) -> String {
        browser_tools::get_snapshot(self, max_nodes).await
    }

    // ----- Phase 4 — Find -----

    #[tool(description = "Search the active page for interactive elements \
                       matching the given text. Optionally filter by ARIA \
                       role or HTML tag name. Returns matched elements \
                       with their role, name, selector, bounding box, and \
                       visibility state.")]
    async fn browser_find(
        &self,
        Parameters(BrowserFindParams { text, role }): Parameters<BrowserFindParams>,
    ) -> String {
        browser_tools::find_elements(self, text, role).await
    }

    // ----- Phase 5 — Dialog & wait-for-selector -----

    #[tool(description = "Handle a JavaScript dialog (alert, confirm, prompt) \
                       on the active page. Use 'accept' to OK/Yes or \
                       'dismiss' to cancel. For prompt dialogs, provide \
                       prompt_text with the response value.")]
    async fn browser_handle_dialog(
        &self,
        Parameters(BrowserHandleDialogParams { action, prompt_text }): Parameters<BrowserHandleDialogParams>,
    ) -> String {
        browser_tools::handle_dialog_action(self, action, prompt_text).await
    }

    #[tool(description = "Wait for an element matching the CSS selector to \
                       reach a specific DOM state. States: 'attached' \
                       (exists in DOM), 'detached' (removed from DOM), \
                       'visible' (default, attached + visible), 'hidden' \
                       (not visible or detached).")]
    async fn browser_wait_for_selector(
        &self,
        Parameters(BrowserWaitForSelectorParams { selector, state, timeout_ms }): Parameters<BrowserWaitForSelectorParams>,
    ) -> String {
        browser_tools::wait_for_selector_state(self, selector, state, timeout_ms).await
    }

    // ----- Content extraction -----

    #[tool(description = "Get the rendered content of the active tab as clean \
                       Markdown. The page HTML is stripped of noise (nav, \
                       headers, footers, ads) and converted to Markdown.")]
    async fn browser_get_content(&self) -> String {
        browser_tools::get_content(self).await
    }

    #[tool(description = "Get the raw HTML source of the active tab's page.")]
    async fn browser_get_html(&self) -> String {
        browser_tools::get_html(self).await
    }

    #[tool(description = "Take a screenshot of the active tab. Returns a \
                       base64-encoded PNG image prefixed with \
                       'data:image/png;base64,'. Set full_page=true to \
                       capture the entire scrollable page.")]
    async fn browser_screenshot(
        &self,
        Parameters(BrowserScreenshotParams { full_page }): Parameters<BrowserScreenshotParams>,
    ) -> String {
        browser_tools::take_screenshot(self, full_page).await
    }

    #[tool(description = "Execute JavaScript in the active tab's page context \
                       and return the result as JSON. Useful for reading \
                       page state, DOM queries, or custom extraction logic.")]
    async fn browser_evaluate(
        &self,
        Parameters(BrowserEvaluateParams { script }): Parameters<BrowserEvaluateParams>,
    ) -> String {
        browser_tools::evaluate_js(self, script).await
    }
}

#[::rmcp::tool_handler(name = "websearch", router = Self::tool_router())]
impl ::rmcp::ServerHandler for WebSearchServer {}
