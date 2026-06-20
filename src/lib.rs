// ---------------------------------------------------------------------------
// lib.rs — Public API for the websearch MCP server
//
// This is the library entry point. The binary crate (`main.rs`) is a thin
// bootstrap that parses CLI arguments and calls `serve()`.
//
// Public re-exports:
//   - config::{Config}
//   - error::{Error}
//
// Internal modules:
//   - browser    — Browser lifecycle (launch, hold, kill)
//   - session    — Tab management, navigation, interaction, content extraction
//   - cleanup    — HTML noise stripping + Markdown post-processing
//   - providers/ — Search URL builders (brave, google, duckduckgo)
//   - registry   — Provider registry (register, resolve, list)
//   - tools/     — MCP server struct and tool handlers
// ---------------------------------------------------------------------------

pub mod config;
pub mod error;

mod browser;
#[path = "cleanup/mod.rs"]
mod cleanup;
mod providers;
mod registry;
mod session;
mod tools;

use std::sync::Arc;

use rmcp::{serve_server, transport::stdio};
use tracing::info;

pub use config::Config;
pub use error::Error;

use tools::WebSearchServer;

// ---------------------------------------------------------------------------
// Server entrypoint — called from main.rs
// ---------------------------------------------------------------------------

/// Start the websearch MCP server with the given validated configuration.
///
/// This function:
/// 1. Creates the profile directory if needed
/// 2. Launches a persistent Chromium browser
/// 3. Creates a SessionManager (recovers existing tabs)
/// 4. Registers all search providers
/// 5. Starts the MCP transport over stdio
/// 6. Waits for the server to finish (SIGTERM, EOF, etc.)
///
/// Returns `Error` on failure, which automatically converts to `anyhow::Error`
/// at the binary boundary via the `std::error::Error` trait impl.
pub async fn serve(config: Config) -> error::Result<()> {
    let profile_dir = &config.profile_dir;

    let browser_mgr = if let Some(ref url) = config.remote_url {
        // Connect to existing remote Chrome instance.
        info!("connecting to remote browser at {url}");
        Arc::new(
            browser::BrowserManager::connect(url)
                .await
                .map_err(|e| Error::BrowserLaunch(e.to_string()))?,
        )
    } else {
        // Launch a local Chrome instance.
        std::fs::create_dir_all(profile_dir)?;
        info!(
            "starting browser (headless={}, wait={}s)",
            config.headless, config.wait_seconds
        );
        Arc::new(
            browser::BrowserManager::launch(
                config.headless,
                profile_dir.clone(),
                config.chrome,
                config.port,
            )
            .await
            .map_err(|e| Error::BrowserLaunch(e.to_string()))?,
        )
    };

    // Create the session manager, recovering any existing tabs.
    let session = browser_mgr.session(config.wait_seconds).await?;

    let engine = Arc::new(registry::SearchEngine::new());
    info!("providers ready: {:?}", engine.available_providers());

    let server = WebSearchServer { engine, session };

    info!("websearch-mcp starting on stdio …");
    serve_server(server, stdio())
        .await
        .map_err(|e| Error::Other(format!("failed to start server: {e}")))?
        .waiting()
        .await
        .map_err(|e| Error::Other(format!("server error: {e}")))?;

    info!("server stopped");
    Ok(())
}
