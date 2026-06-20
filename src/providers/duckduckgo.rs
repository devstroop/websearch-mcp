use super::SearchProvider;

pub struct DuckDuckGo;

#[async_trait::async_trait]
impl SearchProvider for DuckDuckGo {
    fn provider_kind(&self) -> &'static str {
        "duckduckgo"
    }

    fn search_url(&self, query: &str) -> String {
        url::Url::parse_with_params("https://html.duckduckgo.com/html/", &[("q", query)])
            .map(|u| u.to_string())
            .unwrap_or_else(|_| format!("https://html.duckduckgo.com/html/?q={query}"))
    }
}
