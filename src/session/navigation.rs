// ---------------------------------------------------------------------------
// session/navigation.rs — Page navigation methods
// ---------------------------------------------------------------------------

use std::time::Duration;

use tracing::info;

use super::{Error, LibResult, SessionManager, STEALTH_JS};

impl SessionManager {
    /// Navigate the active tab to a URL.
    pub async fn navigate(&mut self, url: &str) -> LibResult<()> {
        self.guard_no_dialog().await?;
        self.touch_active_tab();
        let page = self.get_active_page()?;
        let wait = Duration::from_secs(self.wait_seconds);

        tokio::time::timeout(wait, page.goto(url))
            .await
            .map_err(|_| Error::NavigationTimeout(self.wait_seconds))?
            .map_err(|e| Error::Navigation(e.to_string()))?;

        // Re-apply stealth after navigation in headless mode (some sites check post-load).
        if self.headless {
            let _ = page.evaluate(STEALTH_JS).await;
        }

        // Allow JS to render and network to settle.
        tokio::time::sleep(wait).await;

        // Update tracked URL and title.
        let id_clone = self.active_tab_id.clone();
        if let Some(id) = &id_clone {
            if let Some(tab) = self.tabs.get_mut(id) {
                tab.url = url.to_string();
            }
        }
        // Update title separately to avoid borrow conflict.
        if let Some(id) = &id_clone {
            if let Some(tab) = self.tabs.get(id) {
                if let Ok(Some(title)) = tab.page.get_title().await {
                    if let Some(tab) = self.tabs.get_mut(id) {
                        tab.title = title;
                    }
                }
            }
        }

        info!("navigated active tab to {url}");
        Ok(())
    }

    /// Go back in browser history (active tab).
    pub async fn back(&mut self) -> LibResult<()> {
        self.guard_no_dialog().await?;
        self.touch_active_tab();
        let page = self.get_active_page()?;
        page.goto("javascript:history.back()")
            .await
            .map_err(|e| Error::Browser(format!("go back failed: {e}")))?;
        tokio::time::sleep(Duration::from_secs(2)).await;
        self.update_active_tab_metadata().await;
        Ok(())
    }

    /// Go forward in browser history (active tab).
    pub async fn forward(&mut self) -> LibResult<()> {
        self.guard_no_dialog().await?;
        self.touch_active_tab();
        let page = self.get_active_page()?;
        page.goto("javascript:history.forward()")
            .await
            .map_err(|e| Error::Browser(format!("go forward failed: {e}")))?;
        tokio::time::sleep(Duration::from_secs(2)).await;
        self.update_active_tab_metadata().await;
        Ok(())
    }

    /// Reload the current page (active tab).
    pub async fn reload(&mut self) -> LibResult<()> {
        self.guard_no_dialog().await?;
        self.touch_active_tab();
        let page = self.get_active_page()?;
        page.reload()
            .await
            .map_err(|e| Error::Browser(format!("reload failed: {e}")))?;
        tokio::time::sleep(Duration::from_secs(self.wait_seconds)).await;
        self.update_active_tab_metadata().await;
        Ok(())
    }

    /// Update URL and title metadata for the active tab.
    async fn update_active_tab_metadata(&mut self) {
        if let Some(id) = self.active_tab_id.clone() {
            if let Some(tab) = self.tabs.get_mut(&id) {
                if let Ok(Some(url)) = tab.page.url().await {
                    tab.url = url;
                }
                if let Ok(Some(title)) = tab.page.get_title().await {
                    tab.title = title;
                }
            }
        }
    }
}
