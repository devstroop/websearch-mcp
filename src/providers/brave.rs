use super::SearchProvider;

pub struct Brave;

#[async_trait::async_trait]
impl SearchProvider for Brave {
    fn provider_kind(&self) -> &'static str {
        "brave"
    }

    fn search_url(&self, query: &str) -> String {
        url::Url::parse_with_params("https://search.brave.com/search", &[("q", query)])
            .map(|u| u.to_string())
            .unwrap_or_else(|_| format!("https://search.brave.com/search?q={query}"))
    }
}
