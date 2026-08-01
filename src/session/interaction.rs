// ---------------------------------------------------------------------------
// session/interaction.rs — Page interaction methods
//
// Behavioral-level actions (click, type, hover, select, drag, upload, etc.)
// ---------------------------------------------------------------------------

use chromiumoxide::cdp::browser_protocol::dom::SetFileInputFilesParams;
use chromiumoxide::cdp::browser_protocol::input::{
    DispatchMouseEventParams, DispatchMouseEventType, MouseButton,
};
use tracing::info;

use super::types::FormField;
use super::{Error, LibResult, SessionManager};

impl SessionManager {
    /// Click an element by CSS selector on the active tab.
    ///
    /// The click is dispatched via raw CDP input events rather than
    /// chromiumoxide's `element.click()`: if the click opens a JavaScript
    /// dialog (alert/confirm/prompt), the page main thread pauses and the
    /// final mouse-release command never completes. Time-boxing the release
    /// and polling for the dialog keeps this call from hanging — the pending
    /// dialog is surfaced to the agent so it can call `browser_handle_dialog`.
    pub async fn click(&mut self, selector: &str) -> LibResult<()> {
        self.guard_no_dialog().await?;
        self.touch_active_tab();
        let page = self.get_active_page()?;
        let element = page
            .find_element(selector)
            .await
            .map_err(|e| Error::ElementNotFound(format!("{selector}: {e}")))?;

        element
            .scroll_into_view()
            .await
            .map_err(|e| Error::Browser(format!("scroll into view failed on {selector}: {e}")))?;
        let center = element
            .clickable_point()
            .await
            .map_err(|e| Error::Browser(format!("click failed on {selector}: {e}")))?;

        let builder = DispatchMouseEventParams::builder()
            .x(center.x)
            .y(center.y)
            .button(MouseButton::Left)
            .click_count(1);

        page.execute(
            builder
                .clone()
                .r#type(DispatchMouseEventType::MouseMoved)
                .build()
                .unwrap(),
        )
        .await
        .map_err(|e| Error::Browser(format!("CDP mousemove failed on {selector}: {e}")))?;

        page.execute(
            builder
                .clone()
                .r#type(DispatchMouseEventType::MousePressed)
                .build()
                .unwrap(),
        )
        .await
        .map_err(|e| Error::Browser(format!("CDP mousedown failed on {selector}: {e}")))?;

        // If the mouseup fires alert()/confirm(), the page pauses and Chrome
        // never answers the release command. Treat that as success — the
        // pending dialog is detected below.
        let release = builder
            .r#type(DispatchMouseEventType::MouseReleased)
            .build()
            .unwrap();
        let _ =
            tokio::time::timeout(std::time::Duration::from_secs(2), page.execute(release)).await;

        info!("clicked element: {selector}");
        Ok(())
    }

    /// Type text into an element by CSS selector, optionally pressing Enter.
    pub async fn type_text(&mut self, selector: &str, text: &str, submit: bool) -> LibResult<()> {
        self.guard_no_dialog().await?;
        self.touch_active_tab();
        let page = self.get_active_page()?;
        let element = page
            .find_element(selector)
            .await
            .map_err(|e| Error::ElementNotFound(format!("{selector}: {e}")))?;

        // Focus the element first by clicking it.
        element
            .click()
            .await
            .map_err(|e| Error::Browser(format!("focus click failed on {selector}: {e}")))?;

        // Type the text.
        element
            .type_str(text)
            .await
            .map_err(|e| Error::Browser(format!("type failed on {selector}: {e}")))?;

        // Optionally submit with Enter.
        if submit {
            element
                .press_key("Enter")
                .await
                .map_err(|e| Error::Browser(format!("submit key failed: {e}")))?;
        }

        info!("typed into {selector} (submit={submit})");
        Ok(())
    }

    /// Hover over an element by CSS selector on the active tab.
    pub async fn hover(&mut self, selector: &str) -> LibResult<()> {
        self.guard_no_dialog().await?;
        self.touch_active_tab();
        let page = self.get_active_page()?;
        let element = page
            .find_element(selector)
            .await
            .map_err(|e| Error::ElementNotFound(format!("{selector}: {e}")))?;
        element
            .hover()
            .await
            .map_err(|e| Error::Browser(format!("hover failed on {selector}: {e}")))?;
        info!("hovered over: {selector}");
        Ok(())
    }

    /// Select an option in a `<select>` dropdown by CSS selector.
    pub async fn select_option(&mut self, selector: &str, value: &str) -> LibResult<()> {
        self.guard_no_dialog().await?;
        let js = format!(
            r#"(function() {{
                const sel = document.querySelector({sel});
                if (!sel) throw new Error('element not found');
                if (!sel.matches('select')) throw new Error('element is not a <select>');
                const options = sel.querySelectorAll('option');
                let found = false;
                for (const opt of options) {{
                    if (opt.value === {val} || opt.textContent.trim() === {val}) {{
                        sel.value = opt.value;
                        found = true;
                        break;
                    }}
                }}
                if (!found) throw new Error('option not found: ' + {val});
                sel.dispatchEvent(new Event('change', {{ bubbles: true }}));
                return sel.value;
            }})()"#,
            sel = serde_json::to_string(selector).unwrap(),
            val = serde_json::to_string(value).unwrap()
        );
        self.evaluate(&js).await?;
        info!("selected '{value}' in {selector}");
        Ok(())
    }

    /// Fill multiple form fields at once.
    pub async fn fill_form(&mut self, fields: &[FormField]) -> LibResult<()> {
        self.touch_active_tab();
        for field in fields {
            self.guard_no_dialog().await?;
            let page = self.get_active_page()?;
            let element = page
                .find_element(&field.selector)
                .await
                .map_err(|e| Error::ElementNotFound(format!("{}: {e}", field.selector)))?;
            element.click().await.map_err(|e| {
                Error::Browser(format!("focus click failed on {}: {e}", field.selector))
            })?;
            element
                .type_str(&field.value)
                .await
                .map_err(|e| Error::Browser(format!("type failed on {}: {e}", field.selector)))?;
        }
        info!("filled {} form field(s)", fields.len());
        Ok(())
    }

    /// Start dragging an element from its location using CDP input simulation.
    pub async fn drag_from(&mut self, from_selector: &str) -> LibResult<()> {
        self.guard_no_dialog().await?;
        self.touch_active_tab();
        let page = self.get_active_page()?;
        let element = page
            .find_element(from_selector)
            .await
            .map_err(|e| Error::ElementNotFound(format!("{from_selector}: {e}")))?;
        let center = element.clickable_point().await.map_err(|e| {
            Error::Browser(format!(
                "failed to get clickable point for {from_selector}: {e}"
            ))
        })?;

        element
            .hover()
            .await
            .map_err(|e| Error::Browser(format!("hover failed on {from_selector}: {e}")))?;

        page.execute(
            DispatchMouseEventParams::builder()
                .x(center.x)
                .y(center.y)
                .button(MouseButton::Left)
                .r#type(DispatchMouseEventType::MousePressed)
                .build()
                .unwrap(),
        )
        .await
        .map_err(|e| Error::Browser(format!("CDP mousedown failed: {e}")))?;

        self.drag_origin = Some((center.x, center.y));
        info!("drag started from: {from_selector}");
        Ok(())
    }

    /// Drop a previously dragged element onto a target element via CDP.
    pub async fn drop_to(&mut self, to_selector: &str) -> LibResult<()> {
        self.guard_no_dialog().await?;
        let origin = self
            .drag_origin
            .take()
            .ok_or_else(|| Error::Browser("no active drag — call browser_drag first".into()))?;

        let page = self.get_active_page()?;
        let element = page
            .find_element(to_selector)
            .await
            .map_err(|e| Error::ElementNotFound(format!("{to_selector}: {e}")))?;
        let target = element.clickable_point().await.map_err(|e| {
            Error::Browser(format!("failed to get target point for {to_selector}: {e}"))
        })?;

        let (ox, oy) = origin;
        let steps = 8u32;
        for i in 1..=steps {
            let t = i as f64 / steps as f64;
            let mx = ox + (target.x - ox) * t;
            let my = oy + (target.y - oy) * t;
            page.execute(
                DispatchMouseEventParams::builder()
                    .x(mx)
                    .y(my)
                    .button(MouseButton::Left)
                    .r#type(DispatchMouseEventType::MouseMoved)
                    .build()
                    .unwrap(),
            )
            .await
            .map_err(|e| Error::Browser(format!("CDP mousemove failed: {e}")))?;
        }

        page.execute(
            DispatchMouseEventParams::builder()
                .x(target.x)
                .y(target.y)
                .button(MouseButton::Left)
                .r#type(DispatchMouseEventType::MouseReleased)
                .build()
                .unwrap(),
        )
        .await
        .map_err(|e| Error::Browser(format!("CDP mouseup failed: {e}")))?;

        info!("dropped onto: {to_selector}");
        Ok(())
    }

    /// Upload a file via an `<input type="file">` element using CDP.
    ///
    /// Uses `DOM.setFileInputFiles` to set file paths on the input element,
    /// which is more reliable than constructing File objects via JavaScript.
    pub async fn file_upload(&mut self, selector: &str, file_path: &str) -> LibResult<()> {
        self.guard_no_dialog().await?;
        self.touch_active_tab();
        let page = self.get_active_page()?;

        if !std::path::Path::new(file_path).exists() {
            return Err(Error::Other(format!("file not found: {file_path}")));
        }

        let element = page
            .find_element(selector)
            .await
            .map_err(|e| Error::ElementNotFound(format!("{selector}: {e}")))?;

        let params = SetFileInputFilesParams::builder()
            .file(file_path)
            .node_id(element.node_id)
            .build()
            .map_err(|e| Error::Browser(format!("build file upload params: {e}")))?;

        page.execute(params)
            .await
            .map_err(|e| Error::Browser(format!("CDP file upload failed: {e}")))?;

        info!("uploaded {file_path} to {selector}");
        Ok(())
    }

    /// Press a keyboard key or key combination globally.
    pub async fn press_key(&mut self, key: &str) -> LibResult<()> {
        self.guard_no_dialog().await?;
        let js = format!(
            r#"(function() {{
                const active = document.activeElement || document.body;
                const key = {key};
                const parts = key.split('+');
                const keyName = parts.pop();
                const ctrl = parts.includes('Control') || parts.includes('control');
                const alt = parts.includes('Alt') || parts.includes('alt');
                const shift = parts.includes('Shift') || parts.includes('shift');
                const meta = parts.includes('Meta') || parts.includes('meta');
                active.dispatchEvent(new KeyboardEvent('keydown', {{
                    key: keyName, code: keyName,
                    ctrlKey: ctrl, altKey: alt, shiftKey: shift, metaKey: meta,
                    bubbles: true, cancelable: true
                }}));
                active.dispatchEvent(new KeyboardEvent('keypress', {{
                    key: keyName, code: keyName,
                    ctrlKey: ctrl, altKey: alt, shiftKey: shift, metaKey: meta,
                    bubbles: true, cancelable: true
                }}));
                active.dispatchEvent(new KeyboardEvent('keyup', {{
                    key: keyName, code: keyName,
                    ctrlKey: ctrl, altKey: alt, shiftKey: shift, metaKey: meta,
                    bubbles: true, cancelable: true
                }}));
                return keyName;
            }})()"#,
            key = serde_json::to_string(key).unwrap()
        );
        self.evaluate(&js).await?;
        info!("pressed key: {key}");
        Ok(())
    }

    /// Resize the browser viewport via `window.resizeTo`.
    pub async fn resize_viewport(&mut self, width: u32, height: u32) -> LibResult<()> {
        self.guard_no_dialog().await?;
        let js = format!(
            "window.resizeTo({width}, {height}); (window.innerWidth + 'x' + window.innerHeight)"
        );
        self.evaluate(&js).await?;
        info!("viewport resized to {width}x{height}");
        Ok(())
    }
}
