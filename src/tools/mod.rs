// ---------------------------------------------------------------------------
// tools/mod.rs — MCP tool definitions and server struct
//
// This module defines the MCP server struct (`WebSearchServer`) and its
// tool handlers via the rmcp `#[tool_router]` macro. The actual handler
// logic lives in sibling modules (`search.rs`, `fetch.rs`) — this file is
// the routing layer only.
//
// Responsibilities:
//   - Define MCP parameter schemas (SearchParams, FetchParams)
//   - Own WebSearchServer struct (shared state for all tools)
//   - Annotate tools with `#[tool]` and route to handler functions
// ---------------------------------------------------------------------------

pub mod fetch;
pub mod search;

use std::sync::Arc;

use rmcp::{handler::server::wrapper::Parameters, schemars, tool, tool_router};
use serde::Deserialize;

use crate::browser;
use crate::registry;

// ---------------------------------------------------------------------------
// MCP parameter schemas
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
// MCP server — holds wired dependencies, exposes tools
// ---------------------------------------------------------------------------

/// Shared state for all MCP tool handlers.
///
/// Constructed once in `websearch::serve()` and cloned into each
/// request handler by rmcp.
#[derive(Clone)]
pub struct WebSearchServer {
    /// Provider registry (resolves "brave", "duckduckgo", "google").
    pub engine: Arc<registry::SearchEngine>,
    /// Shared browser handle (persistent Chrome/Chromium instance).
    pub browser_mgr: Arc<browser::BrowserManager>,
    /// Seconds to wait for JS rendering before extracting HTML.
    pub wait_seconds: u64,
}

#[tool_router]
impl WebSearchServer {
    /// Search the web using a pluggable search engine provider.
    /// Supports: brave (default, recommended), duckduckgo, and google.
    /// Returns the rendered search page as clean Markdown for the AI
    /// to interpret naturally.
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

    /// Fetch a URL and return its rendered content as clean Markdown.
    /// Uses the same browser-driven pipeline as search — loads the page,
    /// waits for JavaScript to render, strips non-content elements, and
    /// converts to Markdown for the AI to interpret naturally.
    #[tool(description = "Fetch a URL and return its rendered content as clean \
                       Markdown. The page is loaded in a real browser, \
                       JavaScript is executed, and non-content elements \
                       (nav, headers, footers, ads, tracking) are stripped \
                       automatically. Only http:// and https:// URLs are \
                       supported.")]
    async fn fetch(&self, Parameters(FetchParams { url }): Parameters<FetchParams>) -> String {
        fetch::handle(self, url).await
    }
}

#[::rmcp::tool_handler(name = "websearch", router = Self::tool_router())]
impl ::rmcp::ServerHandler for WebSearchServer {}
