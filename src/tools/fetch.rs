// ---------------------------------------------------------------------------
// tools/fetch.rs — Fetch tool handler
//
// Delegated from tools/mod.rs's `#[tool]` method. Uses the SessionManager
// to open a tab, navigate to the URL, extract content, and clean up.
// ---------------------------------------------------------------------------

use super::WebSearchServer;

/// Fetch a URL and return its rendered content as clean Markdown.
pub async fn handle(server: &WebSearchServer, url: String) -> String {
    let normalized = url.trim();
    if !normalized.starts_with("http://") && !normalized.starts_with("https://") {
        return "Invalid URL scheme. Only http:// and https:// are supported.".to_string();
    }

    let mut session = server.session.lock().await;

    // Open a temporary tab for the fetch, get content, then close it.
    let tab_result = session.open_tab(Some(normalized), true).await;
    if let Err(e) = tab_result {
        return format!("Failed to open fetch tab: {e}");
    }

    let content = session.get_content().await;
    let _ = session.close_tab(None).await;

    match content {
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
