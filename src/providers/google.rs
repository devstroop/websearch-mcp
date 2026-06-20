use super::SearchProvider;

pub struct Google;

#[async_trait::async_trait]
impl SearchProvider for Google {
    fn provider_kind(&self) -> &'static str {
        "google"
    }

    fn search_url(&self, query: &str) -> String {
        url::Url::parse_with_params("https://www.google.com/search", &[("q", query)])
            .map(|u| u.to_string())
            .unwrap_or_else(|_| format!("https://www.google.com/search?q={query}"))
    }
}
