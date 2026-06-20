// ---------------------------------------------------------------------------
// browser.rs — Owns the Chrome/Chromium lifecycle
//
// Responsibilities:
//   - Launch a persistent browser instance (visible or headless)
//   - Clean stale lock files before launching
//   - Provide shared access via `Arc<Mutex<Browser>>` to all tools
//   - Kill the Chrome process on server shutdown (Drop guard)
//
// This module owns the browser process — nothing else should
// start or stop the browser.
// ---------------------------------------------------------------------------

use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;

use anyhow::{Context, Result};
use chromiumoxide::{
    browser::{Browser, BrowserConfig},
    handler::viewport::Viewport,
};
use futures::StreamExt;
use tokio::sync::Mutex;
use tracing::info;
use tracing::warn;

use crate::error::Result as LibResult;
use crate::session::SessionManager;

// ---------------------------------------------------------------------------
// SharedBrowser — type alias for the shared browser reference
// ---------------------------------------------------------------------------

pub type SharedBrowser = Arc<Mutex<Browser>>;

// ---------------------------------------------------------------------------
// Helpers — stale lock file removal
// ---------------------------------------------------------------------------

fn remove_lock_files(dir: &Path) {
    for entry in &["SingletonLock", "SingletonSocket", "SingletonCookie"] {
        let p = dir.join(entry);
        if p.exists() {
            let result = if p.is_dir() {
                std::fs::remove_dir_all(&p)
            } else {
                std::fs::remove_file(&p)
            };
            match result {
                Ok(()) => info!("removed stale lock: {}", p.display()),
                Err(e) => warn!("failed to remove lock file {}: {e}", p.display()),
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Helpers — discover child Chrome / Chromium PID by profile path
// ---------------------------------------------------------------------------

fn find_chrome_pid(profile_dir: &Path) -> Option<u32> {
    let profile = profile_dir.to_string_lossy();
    let escaped = regex::escape(&profile);
    for pattern in &[
        format!("Google Chrome.*{}", escaped),
        format!("Chromium.*{}", escaped),
    ] {
        if let Ok(out) = std::process::Command::new("pgrep")
            .args(["-f", pattern])
            .output()
        {
            if out.status.success() {
                if let Ok(s) = String::from_utf8(out.stdout) {
                    if let Some(pid_str) = s.lines().next() {
                        if let Ok(pid) = pid_str.trim().parse::<u32>() {
                            return Some(pid);
                        }
                    }
                }
            }
        }
    }
    None
}

// ---------------------------------------------------------------------------
// BrowserManager — owns browser launch, access, and shutdown
// ---------------------------------------------------------------------------

pub struct BrowserManager {
    browser: SharedBrowser,
    /// Guard is None when connected to a remote browser (we don't own it).
    _guard: Option<BrowserGuard>,
}

impl BrowserManager {
    /// Launch the browser and return a `BrowserManager` that holds it.
    pub async fn launch(
        headless: bool,
        profile_dir: PathBuf,
        chrome_path: Option<PathBuf>,
        port: Option<u16>,
    ) -> Result<Self> {
        // Clean stale lock files leftover from crashed/orphaned processes.
        remove_lock_files(&profile_dir);

        info!("browser profile: {}", profile_dir.display());

        let viewport = Viewport {
            width: 1080,
            height: 768,
            ..Default::default()
        };

        let mut builder = BrowserConfig::builder()
            .user_data_dir(&profile_dir)
            .viewport(viewport)
            .window_size(1080, 768)
            .arg("--no-first-run")
            .arg("--disable-search-engine-choice-screen")
            // Anti-bot: hide automation flags that search engines check
            .arg("--disable-blink-features=AutomationControlled")
            .arg("--lang=en-US");

        if !headless {
            builder = builder.with_head();
        } else {
            // Headless mode: override User-Agent to hide "HeadlessChrome" —
            // a massive bot detection signal. Use a real Chrome UA instead.
            builder = builder.arg("--user-agent=Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/130.0.0.0 Safari/537.36");
        }

        if let Some(ref path) = chrome_path {
            builder = builder.chrome_executable(path);
        }

        if let Some(port) = port {
            builder = builder.port(port);
        }

        let cfg = builder
            .build()
            .map_err(|e| anyhow::anyhow!("failed to build BrowserConfig: {e}"))?;

        let (browser, mut handler) = Browser::launch(cfg)
            .await
            .context("failed to launch chromiumoxide Browser")?;

        // Drive CDP messages in background so the connection stays alive.
        tokio::spawn(async move {
            while let Some(event) = handler.next().await {
                if let Err(e) = &event {
                    warn!("browser handler error: {e}");
                }
            }
            info!("browser handler ended — CDP connection closed");
        });

        let pid = find_chrome_pid(&profile_dir);
        if let Some(pid) = pid {
            info!("browser launched (child PID {pid})");
        } else {
            warn!("could not determine child browser PID, will fall back to pkill");
        }

        Ok(Self {
            browser: Arc::new(Mutex::new(browser)),
            _guard: Some(BrowserGuard::new(profile_dir, pid)),
        })
    }

    /// Connect to an existing Chrome instance via DevTools WebSocket URL.
    ///
    /// `url` should be a WebSocket URL like `ws://localhost:9222`.
    /// No browser process is launched or killed — the caller owns the remote browser.
    pub async fn connect(url: &str) -> Result<Self> {
        info!("connecting to remote Chrome: {url}");

        let (browser, mut handler) = Browser::connect(url)
            .await
            .context("failed to connect to remote Chrome")?;

        // Drive CDP messages in background so the connection stays alive.
        tokio::spawn(async move {
            while let Some(event) = handler.next().await {
                if let Err(e) = &event {
                    warn!("browser handler error: {e}");
                }
            }
            info!("browser handler ended — CDP connection closed");
        });

        info!("connected to remote Chrome");

        Ok(Self {
            browser: Arc::new(Mutex::new(browser)),
            _guard: None, // Don't kill the remote browser on drop.
        })
    }

    /// Get a reference to the shared browser handle.
    #[allow(dead_code)]
    pub fn handle(&self) -> &SharedBrowser {
        &self.browser
    }

    /// Create a new SessionManager backed by this browser instance.
    ///
    /// The session manager tracks tabs, manages navigation, and provides
    /// high-level interaction methods. It will attempt to recover any
    /// existing browser tabs from the persistent session.
    pub async fn session(&self, wait_seconds: u64) -> LibResult<Arc<Mutex<SessionManager>>> {
        let session = SessionManager::new(self.browser.clone(), wait_seconds).await?;
        Ok(Arc::new(Mutex::new(session)))
    }
}

// ---------------------------------------------------------------------------
// BrowserGuard — kills Chrome when the server shuts down
// ---------------------------------------------------------------------------

struct BrowserGuard {
    profile_dir: PathBuf,
    child_pid: Option<u32>,
}

impl BrowserGuard {
    fn new(profile_dir: PathBuf, child_pid: Option<u32>) -> Self {
        Self {
            profile_dir,
            child_pid,
        }
    }
}

impl Drop for BrowserGuard {
    fn drop(&mut self) {
        let profile = &self.profile_dir;
        info!(
            "shutdown — killing Chrome for profile: {}",
            profile.display()
        );

        if let Some(pid) = self.child_pid {
            // SIGTERM first, then SIGKILL after a short grace period.
            std::process::Command::new("kill")
                .arg(pid.to_string())
                .output()
                .ok();
            std::thread::sleep(std::time::Duration::from_secs(2));
            std::process::Command::new("kill")
                .args(["-9", &pid.to_string()])
                .output()
                .ok();
        } else {
            // Fall back to pkill with escaped regex — SIGTERM first.
            let profile_str = profile.to_string_lossy();
            let escaped = regex::escape(&profile_str);
            for name in &["Google Chrome", "Chromium"] {
                let pattern = format!("{}.*{}", name, escaped);
                std::process::Command::new("pkill")
                    .args(["-f", "-15", &pattern])
                    .output()
                    .ok();
            }
            std::thread::sleep(std::time::Duration::from_secs(2));
            for name in &["Google Chrome", "Chromium"] {
                let pattern = format!("{}.*{}", name, escaped);
                std::process::Command::new("pkill")
                    .args(["-f", "-9", &pattern])
                    .output()
                    .ok();
            }
        }

        remove_lock_files(profile);
    }
}
