use anyhow::Context;
use chromiumoxide::browser::Browser;

use super::{navigate_and_get_markdown, SearchProvider};

pub struct DuckDuckGo;

#[async_trait::async_trait]
impl SearchProvider for DuckDuckGo {
    fn provider_kind(&self) -> &'static str {
        "duckduckgo"
    }

    async fn search(&self, browser: &Browser, query: &str) -> anyhow::Result<String> {
        let url = url::Url::parse_with_params("https://html.duckduckgo.com/html/", &[("q", query)])
            .context("failed to build duckduckgo URL")?;
        navigate_and_get_markdown(browser, url.as_str()).await
    }
}
