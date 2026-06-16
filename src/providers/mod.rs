pub mod brave;
pub mod duckduckgo;
pub mod google;

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use anyhow::Context;
use chromiumoxide::browser::Browser;

use crate::cleanup;

static PAGE_WAIT_SECS: AtomicU64 = AtomicU64::new(4);

/// Set the global page-wait duration (in seconds). Called once at startup
/// from `main()` so all providers use the same timeout.
pub fn set_page_wait_seconds(secs: u64) {
    PAGE_WAIT_SECS.store(secs, Ordering::SeqCst);
}

fn get_page_wait() -> Duration {
    Duration::from_secs(PAGE_WAIT_SECS.load(Ordering::SeqCst))
}

/// Navigate the browser to a URL, wait for the page to render, and return
/// the full rendered HTML converted to clean Markdown.
///
/// Instead of fragile CSS selectors, the LLM client receives the page content
/// as Markdown and extracts what it needs — the LLM is the only parser needed.
pub async fn navigate_and_get_markdown(browser: &Browser, url: &str) -> anyhow::Result<String> {
    let page = browser
        .new_page("about:blank")
        .await
        .context("failed to open new page")?;

    // Hide automation fingerprints before navigation — this runs on the
    // blank page, overriding navigator.webdriver before the target page loads.
    let conceal = r#"
        Object.defineProperty(navigator, 'webdriver', { get: () => undefined });
        window.chrome = { runtime: {} };
        Object.defineProperty(navigator, 'plugins', { get: () => [1,2,3,4,5] });
        Object.defineProperty(navigator, 'languages', { get: () => ['en-US', 'en'] });
    "#;
    let _ = page.evaluate(conceal).await;

    tokio::time::timeout(get_page_wait(), page.goto(url))
        .await
        .context("navigation timed out")?
        .context("failed to navigate")?;

    // Apply patches again after navigation (some sites check post-load).
    let _ = page.evaluate(conceal).await;

    // Allow JS to render and network to settle.
    tokio::time::sleep(get_page_wait()).await;

    let html = page.content().await.context("failed to get page HTML")?;
    if let Err(e) = page.close().await {
        tracing::warn!("failed to close page: {e}");
    }

    // Strip HTML noise before converting — raw elements produce
    // noisy Markdown with meta blobs, tracking URLs, and SVGs.
    let cleaned = cleanup::strip_noise(&html);

    let result = html_to_markdown_rs::convert(&cleaned, None)
        .map_err(|e| anyhow::anyhow!("HTML→Markdown conversion failed: {e}"))?;

    let md = result.content.unwrap_or_default();

    // Post-process Markdown to strip remaining UI chrome, ads, tracking.
    let md = cleanup::clean_markdown(&md);

    if md.trim().is_empty() {
        // Fallback: return raw HTML if Markdown is empty
        Ok(format!(
            "Search returned no parseable content. Raw HTML ({} bytes):\n\n{}",
            html.len(),
            &html[..html.len().min(5000)]
        ))
    } else {
        Ok(md)
    }
}

// ---------------------------------------------------------------------------
// Trait
// ---------------------------------------------------------------------------

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
    async fn search(&self, browser: &Browser, query: &str) -> anyhow::Result<String>;
}
