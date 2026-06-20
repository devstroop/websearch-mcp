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
