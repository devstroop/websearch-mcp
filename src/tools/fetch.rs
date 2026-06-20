// ---------------------------------------------------------------------------
// tools/fetch.rs — Fetch tool handler
//
// Delegated from tools/mod.rs's `#[tool]` method. Uses the SessionManager
// to open a tab, navigate to the URL, extract content, and clean up.
// ---------------------------------------------------------------------------

use super::WebSearchServer;

/// Fetch a URL and return its rendered content as clean Markdown.
///
/// Opens a temporary tab, waits for rendering, and returns the content.
/// The tab is NOT auto-closed — the agent controls tab lifecycle via browser_close.
pub async fn handle(server: &WebSearchServer, url: String) -> String {
    let normalized = url.trim();
    if !normalized.starts_with("http://") && !normalized.starts_with("https://") {
        return "Invalid URL scheme. Only http:// and https:// are supported.".to_string();
    }

    let mut session = server.session.lock().await;

    // Open a tab for the fetch.
    let tab_result = session.open_tab(Some(normalized), true).await;
    if let Err(e) = tab_result {
        return format!("Failed to open fetch tab: {e}");
    }

    // get_content() waits for rendering then extracts.
    let content = session.get_content().await;

    match content {
        Ok(markdown) => {
            if markdown.trim().is_empty() {
                format!(
                    "The page at {normalized} returned no parseable content. \
                     Use browser_close to close this tab."
                )
            } else {
                format!(
                    "--- Content from {normalized} ---\n\n{markdown}\n\n[Tab `{}` still open — use browser_close when done]",
                    session.active_tab_id().unwrap_or("?")
                )
            }
        }
        Err(e) => format!("Failed to fetch {normalized}: {e}"),
    }
}
