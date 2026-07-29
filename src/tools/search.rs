// ---------------------------------------------------------------------------
// tools/search.rs — Search tool handler
//
// Delegated from tools/mod.rs's `#[tool]` method. Uses the SessionManager
// to reuse the persistent search tab, navigate to the search URL, extract
// content, and return structured results.
// ---------------------------------------------------------------------------

use super::WebSearchServer;

/// Execute a search using the persistent search tab.
///
/// The search tab is created on first call and reused for subsequent searches.
/// The active tab is NOT changed — the agent can continue working on other tabs
/// while the search tab is navigated in the background.
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

    match session.search_or_reuse(prov.provider_kind(), &query, &url).await {
        Ok(result) => {
            if result.raw_markdown.trim().is_empty() {
                format!(
                    "{} returned empty results for \"{query}\". \
                     The page may be blocking automated access. \
                     Try a different provider.",
                    result.provider
                )
            } else {
                format!(
                    "--- Results from {} ---\n\n{}\n\n[Search tab `{}` reused — stays open for next search]",
                    result.provider, result.raw_markdown, result.tab_id
                )
            }
        }
        Err(e) => format!("Search on {} failed: {e}", prov.provider_kind()),
    }
}
