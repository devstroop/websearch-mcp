// ---------------------------------------------------------------------------
// tools/search.rs — Search tool handler
//
// Delegated from tools/mod.rs's `#[tool]` method. Kept separate so the
// logic is testable and the tool routing file stays focused on wiring.
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

    let browser = server.browser_mgr.handle().lock().await;
    match prov.search(&browser, &query, server.wait_seconds).await {
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
