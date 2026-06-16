use anyhow::Context;
use chromiumoxide::browser::Browser;

use super::{navigate_and_get_markdown, SearchProvider};

pub struct Brave;

#[async_trait::async_trait]
impl SearchProvider for Brave {
    fn provider_kind(&self) -> &'static str {
        "brave"
    }

    async fn search(
        &self,
        browser: &Browser,
        query: &str,
    ) -> anyhow::Result<String> {
        let url = url::Url::parse_with_params(
            "https://search.brave.com/search",
            &[("q", query)],
        )
        .context("failed to build brave URL")?;
        navigate_and_get_markdown(browser, url.as_str()).await
    }
}
