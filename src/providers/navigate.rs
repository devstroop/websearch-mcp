// ---------------------------------------------------------------------------
// providers/navigate.rs — Browser navigation and Markdown extraction
//
// Responsibilities:
//   - Navigate the shared browser to a URL, wait for JS render, extract HTML,
//     strip noise, convert to Markdown, post-process
//
// This is the core rendering pipeline shared by all search providers and
// the fetch tool.
// ---------------------------------------------------------------------------

use std::time::Duration;

use anyhow::Context;
use chromiumoxide::browser::Browser;

use crate::cleanup;

/// Navigate the browser to a URL, wait for the page to render, and return
/// the full rendered HTML converted to clean Markdown.
///
/// `wait_seconds` controls how long to wait for JS rendering and network
/// settlement before extracting the page HTML.
///
/// Instead of fragile CSS selectors, the LLM client receives the page content
/// as Markdown and extracts what it needs — the LLM is the only parser needed.
pub async fn navigate_and_get_markdown(
    browser: &Browser,
    url: &str,
    wait_seconds: u64,
) -> anyhow::Result<String> {
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

    let wait = Duration::from_secs(wait_seconds);
    tokio::time::timeout(wait, page.goto(url))
        .await
        .context("navigation timed out")?
        .context("failed to navigate")?;

    // Apply patches again after navigation (some sites check post-load).
    let _ = page.evaluate(conceal).await;

    // Allow JS to render and network to settle.
    tokio::time::sleep(wait).await;

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
