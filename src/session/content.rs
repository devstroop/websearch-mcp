// ---------------------------------------------------------------------------
// session/content.rs — Content extraction and page introspection
//
// Methods: accessibility_snapshot, find_in_page, get_html, get_content,
//          screenshot, evaluate
// ---------------------------------------------------------------------------

use base64::Engine;
use chromiumoxide::page::ScreenshotParams;

use super::{Error, LibResult, SessionManager};

impl SessionManager {
    // ----- Accessibility Snapshot -----

    /// Get the accessibility tree of the active page.
    ///
    /// Walks the DOM to find all interactive/landmark elements and formats
    /// them as a compact tree: `<role "name" attr=val>`.
    /// Returns up to `max_nodes` entries (default 200).
    pub async fn accessibility_snapshot(&mut self, max_nodes: Option<usize>) -> LibResult<String> {
        let limit = max_nodes.unwrap_or(200);
        let js = format!(
            r#"(function() {{
                const maxNodes = {limit};
                const results = [];

                function getRole(el) {{
                    const aria = el.getAttribute('role');
                    if (aria) return aria;
                    const tag = el.tagName.toLowerCase();
                    const roles = {{
                        'a': 'link', 'button': 'button',
                        'input': el.type === 'checkbox' ? 'checkbox' : el.type === 'radio' ? 'radio' : el.type === 'submit' || el.type === 'button' ? 'button' : el.type === 'range' ? 'slider' : el.type === 'search' ? 'searchbox' : 'textbox',
                        'select': 'combobox', 'textarea': 'textbox',
                        'img': 'img', 'nav': 'navigation', 'main': 'main',
                        'header': 'banner', 'footer': 'contentinfo', 'aside': 'complementary',
                        'form': 'form', 'table': 'table', 'ul': 'list', 'ol': 'list',
                        'li': 'listitem',
                        'h1': 'heading', 'h2': 'heading', 'h3': 'heading',
                        'h4': 'heading', 'h5': 'heading', 'h6': 'heading',
                    }};
                    return roles[tag] || '';
                }}

                function getAccessibleName(el) {{
                    return el.getAttribute('aria-label')
                        || (el.getAttribute('aria-labelledby') ? '#ref' : '')
                        || (el.labels && el.labels.length > 0 ? el.labels[0].textContent.trim() : '')
                        || el.title || el.placeholder || '';
                }}

                function walk(node, depth) {{
                    if (node.nodeType !== 1 || results.length >= maxNodes) return;
                    const el = node;
                    const role = getRole(el);
                    if (role) {{
                        const name = getAccessibleName(el) || el.textContent.trim().slice(0, 200) || '';
                        const info = {{ role, name: name.slice(0, 200), depth }};

                        const attrs = {{}};
                        if (el.href) attrs.url = el.href;
                        if (el.value !== undefined && el.value !== '') attrs.value = String(el.value).slice(0, 100);
                        if (el.disabled) attrs.disabled = true;
                        if (el.checked !== undefined) attrs.checked = !!el.checked;
                        if (el.hasAttribute('aria-selected')) attrs.selected = el.getAttribute('aria-selected');
                        if (el.hasAttribute('aria-expanded')) attrs.expanded = el.getAttribute('aria-expanded');
                        if (el.hasAttribute('aria-pressed')) attrs.pressed = el.getAttribute('aria-pressed');
                        const level = el.tagName.match(/^H(\d)$/i);
                        if (level) attrs.level = parseInt(level[1], 10);

                        info.attrs = attrs;
                        results.push(info);
                    }}
                    for (const child of el.children) {{
                        walk(child, role ? depth + 1 : depth);
                    }}
                }}

                walk(document.body, 0);
                return results;
            }})()"#,
            limit = limit
        );

        self.guard_no_dialog().await?;
        self.touch_active_tab();
        let page = self.get_active_page()?;
        let result = page
            .evaluate(&*js)
            .await
            .map_err(|e| Error::Browser(format!("snapshot evaluate failed: {e}")))?;
        let elements: Vec<serde_json::Value> = result
            .into_value()
            .map_err(|e| Error::Other(format!("failed to parse snapshot: {e}")))?;

        if elements.is_empty() {
            return Ok("(empty accessibility tree — no interactive elements found)".into());
        }

        let mut output = String::new();
        for el in &elements {
            let depth = el["depth"].as_i64().unwrap_or(0) as usize;
            let role = el["role"].as_str().unwrap_or("?");
            let name = el["name"].as_str().unwrap_or("");
            let attrs = &el["attrs"];
            let indent = "  ".repeat(depth);

            let mut attr_parts: Vec<String> = Vec::new();
            if let Some(obj) = attrs.as_object() {
                for (k, v) in obj {
                    let vs = match v {
                        serde_json::Value::String(s) => s.clone(),
                        serde_json::Value::Bool(b) => b.to_string(),
                        serde_json::Value::Number(n) => n.to_string(),
                        _ => continue,
                    };
                    attr_parts.push(format!("{k}=\"{vs}\""));
                }
            }

            let line = if attr_parts.is_empty() {
                format!("{indent}<{role} \"{name}\">\n")
            } else {
                format!(
                    "{indent}<{role} \"{name}\" {attrs_str}>\n",
                    attrs_str = attr_parts.join(" ")
                )
            };
            output.push_str(&line);
        }

        let count = elements.len();
        output.push_str(&format!("\n({count} nodes in accessibility tree)"));
        Ok(output)
    }

    // ----- Find -----

    /// Search the active page for interactive elements matching `text`
    /// (in name, role, or value), optionally filtered by `role`.
    ///
    /// Returns a formatted list of matched elements with their role, name,
    /// accessible name, selector, bounding box, and visibility.
    pub async fn find_in_page(&mut self, text: &str, role: Option<&str>) -> LibResult<String> {
        let text_esc = serde_json::to_string(text).unwrap();
        let role_filter = match role {
            Some(r) => {
                let r_esc = serde_json::to_string(r).unwrap();
                format!(
                    "|| (role === {r_esc} && tag !== {r_esc}) || (tag === {r_esc} && role !== {r_esc})"
                )
            }
            None => String::new(),
        };

        let js = format!(
            r#"(function() {{
                const text = {text}.toLowerCase();
                const results = [];
                const all = document.querySelectorAll(
                    'a, button, input, select, textarea, [role], [tabindex], ' +
                    'label, img, h1, h2, h3, h4, h5, h6, p, li, td, th, [contenteditable]'
                );

                for (const el of all) {{
                    const tag = el.tagName.toLowerCase();
                    const role = el.getAttribute('role') || (
                        tag === 'a' ? 'link' :
                        tag === 'button' ? 'button' :
                        tag === 'input' ? (el.type === 'checkbox' ? 'checkbox' : el.type === 'radio' ? 'radio' : el.type === 'text' ? 'textbox' : el.type === 'submit' ? 'button' : el.type) :
                        tag === 'select' ? 'combobox' :
                        tag === 'textarea' ? 'textbox' :
                        tag === 'img' ? 'img' :
                        ''
                    );
                    const ariaLabel = el.getAttribute('aria-label') || '';
                    const label = el.labels && el.labels.length > 0 ? el.labels[0].textContent.trim() : '';
                    const value = el.value || el.textContent?.trim().slice(0, 200) || '';
                    const name = ariaLabel || label || el.title || el.placeholder || '';
                    const searchStr = (role + ' ' + name + ' ' + value).toLowerCase();

                    if (!searchStr.includes(text){role_filter}) continue;

                    const rect = el.getBoundingClientRect();
                    const visible = !!(el.offsetWidth || el.offsetHeight || el.getClientRects().length);
                    const disabled = el.disabled || el.getAttribute('aria-disabled') === 'true';
                    const checked = el.checked !== undefined ? el.checked : null;

                    // Build a simple selector
                    let sel = tag;
                    if (el.id) sel += '#' + el.id;
                    else if (el.name) sel += '[name="' + el.name + '"]';
                    else if (el.className && typeof el.className === 'string') {{
                        const cls = el.className.trim().split(/\s+/).slice(0, 2).join('.');
                        if (cls) sel += '.' + cls;
                    }}

                    results.push({{
                        role, name: name.slice(0, 100), value: value.slice(0, 100),
                        selector: sel, tag,
                        x: Math.round(rect.x), y: Math.round(rect.y),
                        w: Math.round(rect.width), h: Math.round(rect.height),
                        visible, disabled, checked
                    }});
                }}
                return results.slice(0, 50);
            }})()"#,
            text = text_esc,
            role_filter = role_filter
        );

        self.guard_no_dialog().await?;
        self.touch_active_tab();
        let page = self.get_active_page()?;
        let result = page
            .evaluate(js.as_str())
            .await
            .map_err(|e| Error::Browser(format!("find evaluate failed: {e}")))?;
        let elements: Vec<serde_json::Value> = result
            .into_value()
            .map_err(|e| Error::Other(format!("failed to parse find results: {e}")))?;

        if elements.is_empty() {
            return Ok(format!(
                "No elements found matching \"{text}\"{}.",
                role.map(|r| format!(" with role \"{r}\""))
                    .unwrap_or_default()
            ));
        }

        let mut output = format!(
            "Found {} element(s) matching \"{text}\":\n\n",
            elements.len()
        );
        for (i, el) in elements.iter().enumerate() {
            let role = el["role"].as_str().unwrap_or("");
            let name = el["name"].as_str().unwrap_or("");
            let value = el["value"].as_str().unwrap_or("");
            let sel = el["selector"].as_str().unwrap_or("");
            let x = el["x"].as_i64().unwrap_or(0);
            let y = el["y"].as_i64().unwrap_or(0);
            let w = el["w"].as_i64().unwrap_or(0);
            let h = el["h"].as_i64().unwrap_or(0);
            let visible = el["visible"].as_bool().unwrap_or(false);
            let disabled = el["disabled"].as_bool().unwrap_or(false);
            let state = if disabled { " disabled" } else { "" };
            let vis = if visible { "" } else { " hidden" };
            let check = match el.get("checked") {
                Some(serde_json::Value::Bool(true)) => " [checked]",
                Some(serde_json::Value::Bool(false)) => " [unchecked]",
                _ => "",
            };

            output.push_str(&format!(
                "{i}. <{role} \"{name}\" selector=\"{sel}\" value=\"{value}\" \
                 box=({x},{y},{w}x{h}){state}{vis}{check}\n"
            ));
        }

        Ok(output)
    }

    // ----- Content Extraction -----

    /// Get the raw HTML of the active tab.
    pub async fn get_html(&mut self) -> LibResult<String> {
        self.guard_no_dialog().await?;
        self.touch_active_tab();
        let page = self.get_active_page()?;
        page.content()
            .await
            .map_err(|e| Error::Browser(format!("failed to get page HTML: {e}")))
    }

    /// Get the active tab's content as clean Markdown.
    ///
    /// Waits for the page to finish rendering (CSS, JS), then runs the full
    /// cleanup pipeline: HTML noise stripping → Markdown conversion →
    /// Markdown post-processing.
    pub async fn get_content(&mut self) -> LibResult<String> {
        // Wait for page to finish rendering before extraction.
        let wait = std::time::Duration::from_secs(self.wait_seconds);
        tokio::time::sleep(wait).await;

        let html = self.get_html().await?;
        let md = crate::session::html_to_markdown(&html)?;
        Ok(md)
    }

    /// Take a screenshot of the active tab, returning base64-encoded PNG.
    pub async fn screenshot(&mut self, full_page: bool) -> LibResult<String> {
        self.guard_no_dialog().await?;
        self.touch_active_tab();
        let page = self.get_active_page()?;
        let params = ScreenshotParams::builder()
            .full_page(full_page)
            .capture_beyond_viewport(full_page)
            .build();
        let bytes = page
            .screenshot(params)
            .await
            .map_err(|e| Error::Screenshot(e.to_string()))?;
        Ok(base64::engine::general_purpose::STANDARD.encode(&bytes))
    }

    /// Execute JavaScript in the active tab and return the result as a JSON string.
    pub async fn evaluate(&mut self, script: &str) -> LibResult<String> {
        self.guard_no_dialog().await?;
        self.touch_active_tab();
        let page = self.get_active_page()?;
        let result = page
            .evaluate(script)
            .await
            .map_err(|e| Error::Browser(format!("evaluate failed: {e}")))?;
        // Convert EvaluationResult to a JSON string.
        let value = result
            .into_value::<serde_json::Value>()
            .map_err(|e| Error::Browser(format!("evaluate result parse failed: {e}")))?;
        Ok(serde_json::to_string_pretty(&value).unwrap_or_else(|_| "null".into()))
    }
}
