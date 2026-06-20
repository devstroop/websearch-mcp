// ---------------------------------------------------------------------------
// providers/mod.rs — Search provider trait and re-exports
//
// Each provider builds a search URL for its engine. The actual browser
// navigation and content extraction is handled by the SessionManager.
// ---------------------------------------------------------------------------

pub mod brave;
pub mod duckduckgo;
pub mod google;
pub mod navigate;

/// A provider-agnostic search interface.
///
/// Each implementation knows how to construct a search URL for its engine.
/// The browser navigation is handled by the SessionManager.
#[async_trait::async_trait]
pub trait SearchProvider: Send + Sync {
    /// Human-readable provider name (e.g. "duckduckgo", "google").
    fn provider_kind(&self) -> &'static str;

    /// Build the search URL for a given query string.
    fn search_url(&self, query: &str) -> String;
}
