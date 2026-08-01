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
use crate::session::FormField;

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
        Ok(()) => {
            if let Ok(Some(dialog)) = session.dialog_pending().await {
                format!(
                    "Clicked: {selector}. WARNING: a JavaScript dialog is now \
                     pending: {dialog}. Use browser_handle_dialog to accept \
                     or dismiss it."
                )
            } else {
                format!("Clicked: {selector}")
            }
        }
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

// ----- Phase 3 — Behavioral interaction handlers -----

/// Hover over an element on the active tab by CSS selector.
pub async fn hover_element(server: &WebSearchServer, selector: String) -> String {
    let mut session = server.session.lock().await;
    match session.hover(&selector).await {
        Ok(()) => format!("Hovered over: {selector}"),
        Err(e) => format!("Hover failed: {e}"),
    }
}

/// Select an option in a `<select>` dropdown by CSS selector.
pub async fn select_option_element(
    server: &WebSearchServer,
    selector: String,
    value: String,
) -> String {
    let mut session = server.session.lock().await;
    match session.select_option(&selector, &value).await {
        Ok(()) => format!("Selected '{value}' in {selector}"),
        Err(e) => format!("Select option failed: {e}"),
    }
}

/// Fill multiple form fields at once.
pub async fn fill_form_fields(server: &WebSearchServer, fields: Vec<FormField>) -> String {
    let mut session = server.session.lock().await;
    match session.fill_form(&fields).await {
        Ok(()) => format!("Filled {} form field(s)", fields.len()),
        Err(e) => format!("Fill form failed: {e}"),
    }
}

/// Start dragging an element from its current location.
pub async fn drag_element(server: &WebSearchServer, from_selector: String) -> String {
    let mut session = server.session.lock().await;
    match session.drag_from(&from_selector).await {
        Ok(()) => format!("Dragging from: {from_selector}"),
        Err(e) => format!("Drag failed: {e}"),
    }
}

/// Drop a previously dragged element onto a target element.
pub async fn drop_element(server: &WebSearchServer, selector: String) -> String {
    let mut session = server.session.lock().await;
    match session.drop_to(&selector).await {
        Ok(()) => format!("Dropped onto: {selector}"),
        Err(e) => format!("Drop failed: {e}"),
    }
}

/// Upload a file via an `<input type="file">` element.
pub async fn upload_file(server: &WebSearchServer, selector: String, file_path: String) -> String {
    let mut session = server.session.lock().await;
    match session.file_upload(&selector, &file_path).await {
        Ok(()) => format!("Uploaded {file_path} to {selector}"),
        Err(e) => format!("File upload failed: {e}"),
    }
}

/// Press a keyboard key or key combination on the active tab.
pub async fn press_key_input(server: &WebSearchServer, key: String) -> String {
    let mut session = server.session.lock().await;
    match session.press_key(&key).await {
        Ok(()) => format!("Pressed: {key}"),
        Err(e) => format!("Press key failed: {e}"),
    }
}

/// Resize the browser viewport.
pub async fn resize_viewport_size(server: &WebSearchServer, width: u32, height: u32) -> String {
    let mut session = server.session.lock().await;
    match session.resize_viewport(width, height).await {
        Ok(()) => format!("Viewport resized to {width}x{height}"),
        Err(e) => format!("Resize failed: {e}"),
    }
}

/// Wait for a page condition (load, domcontentloaded, networkidle, selector:...).
pub async fn wait_for_condition_met(
    server: &WebSearchServer,
    condition: String,
    timeout_ms: Option<u64>,
) -> String {
    let mut session = server.session.lock().await;
    match session.wait_for(&condition, timeout_ms).await {
        Ok(()) => format!("Condition met: {condition}"),
        Err(e) => format!("Wait for failed: {e}"),
    }
}

// ----- Phase 5 — Dialog handling -----

/// Handle a pending browser dialog (alert/confirm/prompt).
pub async fn handle_dialog_action(
    server: &WebSearchServer,
    action: String,
    prompt_text: Option<String>,
) -> String {
    let mut session = server.session.lock().await;
    match session.handle_dialog(&action, prompt_text.as_deref()).await {
        Ok(info) => format!("Dialog handled ({action}): {info}"),
        Err(e) => format!("Handle dialog failed: {e}"),
    }
}

/// Wait for an element to reach a specific DOM state.
pub async fn wait_for_selector_state(
    server: &WebSearchServer,
    selector: String,
    state: Option<String>,
    timeout_ms: Option<u64>,
) -> String {
    let mut session = server.session.lock().await;
    let state_ref = state.as_deref().unwrap_or("visible");
    match session
        .wait_for_selector(&selector, Some(state_ref), timeout_ms)
        .await
    {
        Ok(()) => format!("Selector condition met: {selector} (state={state_ref})"),
        Err(e) => format!("Wait for selector failed: {e}"),
    }
}

// ----- Phase 2 — Accessibility snapshot -----

/// Get the accessibility tree of the active tab.
pub async fn get_snapshot(server: &WebSearchServer, max_nodes: Option<usize>) -> String {
    let mut session = server.session.lock().await;
    match session.accessibility_snapshot(max_nodes).await {
        Ok(tree) => tree,
        Err(e) => format!("Snapshot failed: {e}"),
    }
}

// ----- Phase 4 — Find -----

/// Search the active page for interactive elements matching text/role.
pub async fn find_elements(server: &WebSearchServer, text: String, role: Option<String>) -> String {
    let mut session = server.session.lock().await;
    match session.find_in_page(&text, role.as_deref()).await {
        Ok(result) => result,
        Err(e) => format!("Find failed: {e}"),
    }
}

// ----- Phase 6 — Tab lifecycle -----

/// Close idle tabs that haven't been interacted with.
pub async fn close_idle_tabs_action(server: &WebSearchServer, idle_seconds: u64) -> String {
    let mut session = server.session.lock().await;
    let timeout = std::time::Duration::from_secs(idle_seconds);
    let count = session.close_idle_tabs(timeout).await;
    if count == 0 {
        format!("No idle tabs to close (timeout: {idle_seconds}s)")
    } else {
        format!("Closed {count} idle tab(s) (inactive > {idle_seconds}s)")
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
