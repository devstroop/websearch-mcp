// ---------------------------------------------------------------------------
// tools/fetch.rs — Fetch tool handler
//
// Delegated from tools/mod.rs's `#[tool]` method. Kept separate so the
// logic is testable and the tool routing file stays focused on wiring.
// ---------------------------------------------------------------------------

use super::WebSearchServer;
use crate::providers;

/// Fetch a URL and return its rendered content as clean Markdown.
pub async fn handle(server: &WebSearchServer, url: String) -> String {
    let normalized = url.trim();
    if !normalized.starts_with("http://") && !normalized.starts_with("https://") {
        return "Invalid URL scheme. Only http:// and https:// are supported.".to_string();
    }

    let browser = server.browser_mgr.handle().lock().await;
    match providers::navigate_and_get_markdown(&browser, normalized, server.wait_seconds).await {
        Ok(markdown) => {
            if markdown.trim().is_empty() {
                format!("The page at {normalized} returned no parseable content.")
            } else {
                format!("--- Content from {normalized} ---\n\n{markdown}")
            }
        }
        Err(e) => format!("Failed to fetch {normalized}: {e}"),
    }
}
