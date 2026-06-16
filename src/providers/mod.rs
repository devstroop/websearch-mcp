pub mod brave;
pub mod duckduckgo;
pub mod google;
pub mod navigate;

pub use navigate::navigate_and_get_markdown;

use chromiumoxide::browser::Browser;

/// A provider-agnostic search interface.
///
/// Each implementation receives a shared headless browser, navigates to its
/// search URL, and returns the rendered page as Markdown.
/// The LLM client handles parsing the Markdown naturally.
#[async_trait::async_trait]
pub trait SearchProvider: Send + Sync {
    /// Human-readable provider name (e.g. "duckduckgo", "google").
    fn provider_kind(&self) -> &'static str;

    /// Execute a search via the shared browser. Returns the search results page
    /// as clean Markdown text for the LLM to interpret.
    ///
    /// `wait_seconds` controls how long to wait for JS rendering.
    async fn search(
        &self,
        browser: &Browser,
        query: &str,
        wait_seconds: u64,
    ) -> anyhow::Result<String>;
}
