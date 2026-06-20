// ---------------------------------------------------------------------------
// tools/browser_tools.rs — Browser interaction tool handlers
//
// Delegated from tools/mod.rs's `#[tool]` methods. Provides granular
// DevTools-style control over the persistent browser session:
//   - Tab lifecycle: open, close, focus, list
//   - Navigation: navigate, back, forward, reload
//   - Interaction: click, type
//   - Content: get_content, get_html, screenshot, evaluate
// ---------------------------------------------------------------------------

use super::WebSearchServer;

// ---------------------------------------------------------------------------
// Tab lifecycle handlers
// ---------------------------------------------------------------------------

/// Open a new browser tab, optionally navigating to a URL.
/// Returns tab info including the assigned tab ID.
pub async fn open_tab(
    server: &WebSearchServer,
    url: Option<String>,
    activate: Option<bool>,
) -> String {
    let mut session = server.session.lock().await;
    let activate = activate.unwrap_or(true);
    let url_ref = url.as_deref();

    match session.open_tab(url_ref, activate).await {
        Ok(info) => {
            format!(
                "Opened tab `{}` → {}\nTitle: {}\nActive: {}",
                info.id, info.url, info.title, info.active
            )
        }
        Err(e) => format!("Failed to open tab: {e}"),
    }
}

/// List all open browser tabs.
pub async fn list_tabs(server: &WebSearchServer) -> String {
    let session = server.session.lock().await;
    let tabs = session.list_tabs();

    if tabs.is_empty() {
        return "No open tabs. Use browser_open to create one.".to_string();
    }

    let mut lines = vec![format!("{} open tab(s):", tabs.len())];
    for tab in &tabs {
        let marker = if tab.active { " [ACTIVE]" } else { "" };
        lines.push(format!(
            "  `{}` — {} ({}){marker}",
            tab.id, tab.url, tab.title
        ));
    }
    lines.join("\n")
}

/// Switch the active tab by tab ID.
pub async fn focus_tab(server: &WebSearchServer, tab_id: String) -> String {
    let mut session = server.session.lock().await;
    match session.focus_tab(&tab_id).await {
        Ok(()) => format!("Focused tab `{tab_id}`"),
        Err(e) => format!("Failed to focus tab: {e}"),
    }
}

/// Close a browser tab. Closes the active tab if no tab_id is given.
pub async fn close_tab(server: &WebSearchServer, tab_id: Option<String>) -> String {
    let mut session = server.session.lock().await;
    let id_ref = tab_id.as_deref();
    match session.close_tab(id_ref).await {
        Ok(()) => {
            let tabs = session.list_tabs();
            let remaining = tabs.len();
            format!("Tab closed. {remaining} tab(s) remaining.")
        }
        Err(e) => format!("Failed to close tab: {e}"),
    }
}

// ---------------------------------------------------------------------------
// Navigation handlers
// ---------------------------------------------------------------------------

/// Navigate the active tab to a URL.
pub async fn navigate(server: &WebSearchServer, url: String) -> String {
    let mut session = server.session.lock().await;
    match session.navigate(&url).await {
        Ok(()) => format!("Navigated to {url}"),
        Err(e) => format!("Navigation failed: {e}"),
    }
}

/// Go back in browser history (active tab).
pub async fn go_back(server: &WebSearchServer) -> String {
    let mut session = server.session.lock().await;
    match session.back().await {
        Ok(()) => "Navigated back.".to_string(),
        Err(e) => format!("Go back failed: {e}"),
    }
}

/// Go forward in browser history (active tab).
pub async fn go_forward(server: &WebSearchServer) -> String {
    let mut session = server.session.lock().await;
    match session.forward().await {
        Ok(()) => "Navigated forward.".to_string(),
        Err(e) => format!("Go forward failed: {e}"),
    }
}

/// Reload the current page (active tab).
pub async fn reload_page(server: &WebSearchServer) -> String {
    let mut session = server.session.lock().await;
    match session.reload().await {
        Ok(()) => "Page reloaded.".to_string(),
        Err(e) => format!("Reload failed: {e}"),
    }
}

// ---------------------------------------------------------------------------
// Interaction handlers
// ---------------------------------------------------------------------------

/// Click an element on the active tab by CSS selector.
pub async fn click_element(server: &WebSearchServer, selector: String) -> String {
    let mut session = server.session.lock().await;
    match session.click(&selector).await {
        Ok(()) => format!("Clicked: {selector}"),
        Err(e) => format!("Click failed: {e}"),
    }
}

/// Type text into an element on the active tab by CSS selector.
pub async fn type_text(
    server: &WebSearchServer,
    selector: String,
    text: String,
    submit: Option<bool>,
) -> String {
    let mut session = server.session.lock().await;
    match session
        .type_text(&selector, &text, submit.unwrap_or(false))
        .await
    {
        Ok(()) => {
            if submit.unwrap_or(false) {
                format!("Typed and submitted: {text} → {selector}")
            } else {
                format!("Typed: {text} → {selector}")
            }
        }
        Err(e) => format!("Type failed: {e}"),
    }
}

// ---------------------------------------------------------------------------
// Content extraction handlers
// ---------------------------------------------------------------------------

/// Get the active tab's rendered content as clean Markdown.
pub async fn get_content(server: &WebSearchServer) -> String {
    let mut session = server.session.lock().await;
    match session.get_content().await {
        Ok(md) => {
            if md.trim().is_empty() {
                "The page returned no parseable content.".to_string()
            } else {
                md
            }
        }
        Err(e) => format!("Failed to get content: {e}"),
    }
}

/// Get the raw HTML of the active tab.
pub async fn get_html(server: &WebSearchServer) -> String {
    let mut session = server.session.lock().await;
    match session.get_html().await {
        Ok(html) => html,
        Err(e) => format!("Failed to get HTML: {e}"),
    }
}

/// Take a screenshot of the active tab (returns base64-encoded PNG).
pub async fn take_screenshot(server: &WebSearchServer, full_page: Option<bool>) -> String {
    let mut session = server.session.lock().await;
    match session.screenshot(full_page.unwrap_or(false)).await {
        Ok(b64) => format!("data:image/png;base64,{b64}"),
        Err(e) => format!("Screenshot failed: {e}"),
    }
}

/// Execute JavaScript in the active tab and return the result.
pub async fn evaluate_js(server: &WebSearchServer, script: String) -> String {
    let mut session = server.session.lock().await;
    match session.evaluate(&script).await {
        Ok(result) => result,
        Err(e) => format!("Evaluate failed: {e}"),
    }
}
