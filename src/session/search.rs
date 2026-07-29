// ---------------------------------------------------------------------------
// session/search.rs — Search-or-reuse tool logic
//
// The search tab is created on first call and reused across subsequent
// `search()` calls. Navigation happens in the background without switching
// the agent's active tab.
// ---------------------------------------------------------------------------

use std::time::Duration;

use tracing::info;

use super::{Error, LibResult, SearchResult, SessionManager, STEALTH_JS};

impl SessionManager {
    /// Execute a search by navigating the persistent search tab to `url`.
    ///
    /// Creates the search tab on first call, reuses it on subsequent calls.
    /// If the search tab was closed externally, a new one is opened transparently.
    /// The active tab is NOT changed — the search tab is navigated in the background.
    pub async fn search_or_reuse(
        &mut self,
        provider: &str,
        query: &str,
        url: &str,
    ) -> LibResult<SearchResult> {
        // Determine search tab ID — reuse or create.
        let tab_id = match &self.search_tab_id {
            Some(id) if self.tabs.contains_key(id) => id.clone(),
            _ => {
                let info = self.open_tab(Some("about:blank"), false).await?;
                self.search_tab_id = Some(info.id.clone());
                info.id
            }
        };

        // Navigate the search tab to the target URL without switching active tab.
        {
            let wait = Duration::from_secs(self.wait_seconds);
            let headless = self.headless;
            let page = &self
                .tabs
                .get(&tab_id)
                .ok_or_else(|| Error::Tab("search tab vanished during navigation".into()))?
                .page;

            tokio::time::timeout(wait, page.goto(url))
                .await
                .map_err(|_| Error::NavigationTimeout(self.wait_seconds))?
                .map_err(|e| Error::Navigation(e.to_string()))?;

            if headless {
                let _ = page.evaluate(STEALTH_JS).await;
            }

            // Wait for document ready instead of a fixed sleep.
            let ready_js = "document.readyState";
            for _ in 0..(wait.as_millis() / 100) {
                if let Ok(result) = page.evaluate(ready_js).await {
                    if result.into_value::<String>().unwrap_or_default() == "complete" {
                        break;
                    }
                }
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
        }

        // Update tracked URL and touch the search tab.
        if let Some(tab) = self.tabs.get_mut(&tab_id) {
            tab.url = url.to_string();
            tab.last_active = std::time::Instant::now();
        }

        // Extract rendered content as Markdown.
        let markdown = {
            let page = &self
                .tabs
                .get(&tab_id)
                .ok_or_else(|| {
                    Error::Tab("search tab vanished during content extraction".into())
                })?
                .page;

            let html = page
                .content()
                .await
                .map_err(|e| Error::Browser(format!("failed to get search page HTML: {e}")))?;
            crate::session::html_to_markdown(&html)?
        };

        info!("search_or_reuse: {provider} query=\"{query}\" tab={tab_id}");
        Ok(SearchResult {
            provider: provider.to_string(),
            query: query.to_string(),
            snippets: Vec::new(),
            raw_markdown: markdown,
            tab_id: tab_id.clone(),
        })
    }
}
