// ---------------------------------------------------------------------------
// session/wait.rs — Wait-for-condition and wait-for-selector methods
// ---------------------------------------------------------------------------

use std::time::Duration;

use tracing::info;

use super::{Error, LibResult, SessionManager};

impl SessionManager {
    /// Wait for a page condition.
    ///
    /// Supported conditions:
    /// - `"load"` — wait for `document.readyState === 'complete'`
    /// - `"domcontentloaded"` — wait for `document.readyState !== 'loading'`
    /// - `"networkidle"` — sleep for the configured wait duration
    /// - `selector:CSS_SELECTOR` — wait for element to exist in DOM
    pub async fn wait_for(&mut self, condition: &str, timeout_ms: Option<u64>) -> LibResult<()> {
        self.guard_no_dialog().await?;
        let timeout = Duration::from_millis(timeout_ms.unwrap_or(10_000));
        let poll_ms = 200u64;

        if condition == "networkidle" {
            let wait = Duration::from_secs(self.wait_seconds);
            tokio::time::sleep(wait).await;
            return Ok(());
        }

        let js = if condition == "load" {
            "document.readyState === 'complete'".to_string()
        } else if condition == "domcontentloaded" {
            "document.readyState !== 'loading'".to_string()
        } else if let Some(sel) = condition.strip_prefix("selector:") {
            let sel_json = serde_json::to_string(sel).unwrap();
            format!("document.querySelector({sel_json}) !== null ")
        } else {
            return Err(Error::Browser(format!(
                "unknown wait condition: {condition}"
            )));
        };
        let js = js.trim_end().to_string();

        let start = std::time::Instant::now();
        loop {
            self.guard_no_dialog().await?;
            let done = {
                self.touch_active_tab();
                let page = self.get_active_page()?;
                let result = page
                    .evaluate(js.as_str())
                    .await
                    .map_err(|e| Error::Browser(format!("wait_for evaluate failed: {e}")))?;
                result.into_value::<bool>().unwrap_or(false)
            };
            if done {
                info!("wait_for completed: {condition}");
                return Ok(());
            }
            if start.elapsed() >= timeout {
                return Err(Error::Browser(format!(
                    "wait_for condition not met within {timeout:?}: {condition}"
                )));
            }
            tokio::time::sleep(Duration::from_millis(poll_ms)).await;
        }
    }

    /// Wait for an element matching `selector` to reach a specific DOM state.
    ///
    /// States: "attached", "detached", "visible" (default), "hidden".
    pub async fn wait_for_selector(
        &mut self,
        selector: &str,
        state: Option<&str>,
        timeout_ms: Option<u64>,
    ) -> LibResult<()> {
        self.guard_no_dialog().await?;
        let state = state.unwrap_or("visible");
        let timeout = Duration::from_millis(timeout_ms.unwrap_or(10_000));
        let poll_ms = 200u64;
        let sel = serde_json::to_string(selector).unwrap();

        let condition = match state {
            "attached" => format!("document.querySelector({sel}) !== null "),
            "detached" => format!("document.querySelector({sel}) === null "),
            "visible" => format!(
                "const el = document.querySelector({sel}); \
                 el !== null && el.offsetParent !== null "
            ),
            "hidden" => format!(
                "const el = document.querySelector({sel}); \
                 el === null || el.offsetParent === null "
            ),
            _ => return Err(Error::Browser(format!("unknown wait state: {state}"))),
        };
        // Trim trailing space added for Rust 2021 reserved-prefix compat.
        let condition = condition.trim_end().to_string();

        let start = std::time::Instant::now();
        loop {
            self.guard_no_dialog().await?;
            self.touch_active_tab();
            let page = self.get_active_page()?;
            let done = page
                .evaluate(condition.as_str())
                .await
                .map_err(|e| Error::Browser(format!("wait_for_selector evaluate failed: {e}")))?;
            if done.into_value::<bool>().unwrap_or(false) {
                info!("wait_for_selector completed: {selector} state={state}");
                return Ok(());
            }
            if start.elapsed() >= timeout {
                return Err(Error::Browser(format!(
                    "wait_for_selector condition not met within {timeout:?}: \
                     selector={selector} state={state}"
                )));
            }
            tokio::time::sleep(Duration::from_millis(poll_ms)).await;
        }
    }
}
