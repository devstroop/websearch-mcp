// ---------------------------------------------------------------------------
// tools/search.rs — Search tool handler
//
// Delegated from tools/mod.rs's `#[tool]` method. Uses the SessionManager
// to open a tab, navigate to the search URL, extract content, and clean up.
// ---------------------------------------------------------------------------

use super::WebSearchServer;

/// Execute a search and return the rendered results as Markdown.
pub async fn handle(server: &WebSearchServer, query: String, provider: String) -> String {
    let prov = match server.engine.resolve(&provider) {
        Some(p) => p,
        None => {
            let available = server.engine.available_providers().join(", ");
            return format!("Unknown provider \"{provider}\". Available: {available}");
        }
    };

    let url = prov.search_url(&query);
    let mut session = server.session.lock().await;

    // Open a temporary tab for the search, get content, then close it.
    let tab_result = session.open_tab(Some(&url), true).await;
    if let Err(e) = tab_result {
        return format!("Failed to open search tab: {e}");
    }

    let content = session.get_content().await;
    let _ = session.close_tab(None).await;

    match content {
        Ok(markdown) => {
            if markdown.trim().is_empty() {
                format!(
                    "{} returned empty results for \"{query}\". \
                     The page may be blocking automated access. \
                     Try a different provider.",
                    prov.provider_kind()
                )
            } else {
                format!(
                    "--- Results from {} ---\n\n{}",
                    prov.provider_kind(),
                    markdown
                )
            }
        }
        Err(e) => format!("Search on {} failed: {e}", prov.provider_kind()),
    }
}
