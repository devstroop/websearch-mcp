use anyhow::Context;
use chromiumoxide::browser::Browser;

use super::{navigate_and_get_markdown, SearchProvider};

pub struct Google;

#[async_trait::async_trait]
impl SearchProvider for Google {
    fn provider_kind(&self) -> &'static str {
        "google"
    }

    async fn search(&self, browser: &Browser, query: &str) -> anyhow::Result<String> {
        let url = url::Url::parse_with_params("https://www.google.com/search", &[("q", query)])
            .context("failed to build google URL")?;
        navigate_and_get_markdown(browser, url.as_str()).await
    }
}
