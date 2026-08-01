// ---------------------------------------------------------------------------
// session/dialog.rs — JavaScript dialog handling (alert/confirm/prompt)
// ---------------------------------------------------------------------------

use chromiumoxide::cdp::browser_protocol::page::HandleJavaScriptDialogParams;
use futures::StreamExt;
use tracing::info;

use super::types::DialogInfo;
use super::{Error, LibResult, SessionManager};

impl SessionManager {
    /// Check if a dialog (alert/confirm/prompt) is pending on the active page.
    ///
    /// Polls the CDP `Page.javascriptDialogOpening` event stream and returns
    /// the most recent dialog info as a JSON string, or `None`.
    pub async fn dialog_pending(&mut self) -> LibResult<Option<String>> {
        self.touch_active_tab();
        match self.poll_dialog().await? {
            Some(info) => {
                Ok(Some(serde_json::to_string(info).map_err(|e| {
                    Error::Other(format!("serialize dialog info: {e}"))
                })?))
            }
            None => Ok(None),
        }
    }

    /// Handle a pending dialog — accepts or dismisses it via CDP.
    ///
    /// For prompt dialogs, `prompt_text` provides the response value.
    /// For alert/confirm, `prompt_text` is ignored.
    pub async fn handle_dialog(
        &mut self,
        action: &str,
        prompt_text: Option<&str>,
    ) -> LibResult<String> {
        self.touch_active_tab();
        let accept = action.eq_ignore_ascii_case("accept");
        let (dialog_json, page) = {
            let tab = self.get_active_tab_mut()?;

            if let Some(listener) = &mut tab.dialog_listener {
                if let Ok(Some(event)) =
                    tokio::time::timeout(std::time::Duration::from_millis(50), listener.next())
                        .await
                {
                    tab.pending_dialog = Some(DialogInfo {
                        dialog_type: event.r#type.as_ref().to_string(),
                        message: event.message.clone(),
                        default_prompt: event.default_prompt.clone(),
                    });
                }
            }

            let info = tab
                .pending_dialog
                .as_ref()
                .ok_or_else(|| Error::Browser("no dialog pending".into()))?;

            (
                serde_json::to_string(info)
                    .map_err(|e| Error::Other(format!("serialize dialog info: {e}")))?,
                tab.page.clone(),
            )
        };

        let mut builder = HandleJavaScriptDialogParams::builder().accept(accept);
        if let Some(text) = prompt_text {
            builder = builder.prompt_text(text);
        }
        let params = builder
            .build()
            .map_err(|e| Error::Browser(format!("build dialog params: {e}")))?;

        page.execute(params)
            .await
            .map_err(|e| Error::Browser(format!("CDP handle dialog failed: {e}")))?;

        // Only clear the pending dialog after the CDP command succeeds.
        if let Ok(tab) = self.get_active_tab_mut() {
            tab.pending_dialog = None;
        }

        info!("dialog handled ({action})");
        Ok(dialog_json)
    }

    /// Poll the CDP dialog event stream and return whether a dialog is pending.
    /// If one is found, it's stored in `pending_dialog` for `handle_dialog` to consume.
    pub(crate) async fn poll_dialog(&mut self) -> LibResult<Option<&DialogInfo>> {
        let tab = self.get_active_tab_mut()?;
        if let Some(listener) = &mut tab.dialog_listener {
            if let Ok(Some(event)) =
                tokio::time::timeout(std::time::Duration::from_millis(50), listener.next()).await
            {
                tab.pending_dialog = Some(DialogInfo {
                    dialog_type: event.r#type.as_ref().to_string(),
                    message: event.message.clone(),
                    default_prompt: event.default_prompt.clone(),
                });
            }
        }
        Ok(tab.pending_dialog.as_ref())
    }

    /// Check if a dialog is blocking the page. If so, return an error with dialog info.
    pub(crate) async fn guard_no_dialog(&mut self) -> LibResult<()> {
        if let Some(info) = self.poll_dialog().await? {
            let json = serde_json::to_string(info)
                .map_err(|e| Error::Other(format!("serialize dialog info: {e}")))?;
            return Err(Error::Browser(format!(
                "JavaScript dialog is blocking the page: {json}. \
                 Use browser_handle_dialog to accept or dismiss it."
            )));
        }
        Ok(())
    }
}
