// ---------------------------------------------------------------------------
// main.rs — Entrypoint: parse args, wire modules, serve over stdio.
//
// Responsibilities:
//   - Parse CLI arguments
//   - Launch the persistent browser (delegated to browser::BrowserManager)
//   - Register search providers (delegated to registry::SearchEngine)
//   - Wire everything into WebSearchServer and start the MCP transport
//   - On shutdown, BrowserGuard (inside BrowserManager) kills Chrome
//
// Module boundaries:
//   browser.rs  — Browser lifecycle (launch, hold, kill)
//   registry.rs — Provider registry (register, resolve, list)
//   providers/  — Search implementations (brave, google, duckduckgo)
// ---------------------------------------------------------------------------

use std::path::PathBuf;
use std::sync::Arc;

use clap::Parser;
use rmcp::{
    handler::server::wrapper::Parameters,
    schemars, serve_server, tool, tool_router,
    transport::stdio,
};
use serde::Deserialize;
use tracing::info;

mod browser;
mod cleanup;
mod providers;
mod registry;

use browser::BrowserManager;
use registry::SearchEngine;

// ---------------------------------------------------------------------------
// CLI arguments
// ---------------------------------------------------------------------------

/// MCP server that provides web search via a visible or headless Chromium
/// browser. All search results are returned as Markdown for the LLM to
/// interpret naturally — no fragile CSS selectors needed.
#[derive(Parser, Debug, Clone)]
#[command(name = "websearch", version, about)]
pub struct Args {
    /// Path to the Chrome/Chromium user data directory.
    /// Defaults to $DATA_DIR/websearch-mcp/chrome-profile.
    #[arg(long, env = "WEBSEARCH_PROFILE")]
    pub profile: Option<PathBuf>,

    /// Run browser in headless mode (no visible window). Useful for CI or
    /// when you don't need to visually debug CAPTCHAs.
    #[arg(long, env = "WEBSEARCH_HEADLESS")]
    pub headless: bool,

    /// Chrome/Chromium executable path. Autodetected if not set.
    #[arg(long, env = "WEBSEARCH_CHROME")]
    pub chrome: Option<PathBuf>,

    /// Debug port for Chrome DevTools (e.g. 9222). If not set, a random
    /// free port is used.
    #[arg(long)]
    pub port: Option<u16>,

    /// How many seconds to wait for pages to render before extracting HTML.
    #[arg(long, default_value = "4", env = "WEBSEARCH_WAIT")]
    pub wait_seconds: u64,
}

impl Args {
    /// Resolve the user data directory, with nice default.
    fn profile_dir(&self) -> PathBuf {
        if let Some(ref p) = self.profile {
            p.clone()
        } else {
            dirs::data_dir()
                .unwrap_or_else(|| PathBuf::from("."))
                .join("websearch-mcp")
                .join("chrome-profile")
        }
    }
}

// ---------------------------------------------------------------------------
// MCP parameter schemas
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct SearchParams {
    /// The search query.
    query: String,
    /// Search engine to use: "brave" (default, recommended), "duckduckgo",
    /// or "google".
    #[serde(default = "default_provider")]
    provider: String,
}

fn default_provider() -> String {
    "brave".into()
}

// ---------------------------------------------------------------------------
// MCP server — holds wired dependencies, exposes tools
// ---------------------------------------------------------------------------

#[derive(Clone)]
struct WebSearchServer {
    engine: Arc<SearchEngine>,
    browser_mgr: Arc<BrowserManager>,
}

#[tool_router(server_handler)]
impl WebSearchServer {
    /// Search the web using a pluggable search engine provider.
    /// Supports: brave (default, recommended), duckduckgo, and google.
    /// Returns the rendered search page as clean Markdown for the AI
    /// to interpret naturally.
    #[tool(
        description = "Search the web using a pluggable search engine provider. \
                       Supports: brave (default, recommended), duckduckgo, \
                       and google. Returns the rendered search page as \
                       clean Markdown for the AI to interpret naturally."
    )]
    async fn search(
        &self,
        Parameters(SearchParams { query, provider }): Parameters<SearchParams>,
    ) -> String {
        let prov = match self.engine.resolve(&provider) {
            Some(p) => p,
            None => {
                let available = self.engine.available_providers().join(", ");
                return format!(
                    "Unknown provider \"{provider}\". Available: {available}"
                );
            }
        };

        let browser = self.browser_mgr.handle().lock().await;
        match prov.search(&browser, &query).await {
            Ok(markdown) => {
                if markdown.trim().is_empty() {
                    format!(
                        "{} returned empty results for \"{query}\". \
                         The page may be blocking automated access. \
                         Try a different provider.",
                        prov.provider_kind()
                    )
                } else {
                    format!(
                        "--- Results from {} ---\n\n{}",
                        prov.provider_kind(),
                        markdown
                    )
                }
            }
            Err(e) => format!("Search on {} failed: {e}", prov.provider_kind()),
        }
    }
}

// ---------------------------------------------------------------------------
// Entrypoint
// ---------------------------------------------------------------------------

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .init();

    let args = Args::parse();
    let profile_dir = args.profile_dir();
    std::fs::create_dir_all(&profile_dir).ok();

    // Launch the persistent browser — lives for the server's lifetime.
    // BrowserGuard inside BrowserManager kills Chrome on shutdown.
    info!(
        "starting browser (headless={}, wait={}s)",
        args.headless, args.wait_seconds
    );
    let browser_mgr = Arc::new(
        BrowserManager::launch(
            args.headless,
            profile_dir,
            args.chrome,
            args.port,
        )
        .await?,
    );

    // Build provider registry.
    let engine = Arc::new(SearchEngine::new());
    info!("providers ready: {:?}", engine.available_providers());

    // Store wait_seconds so navigate_and_get_markdown can use it.
    providers::set_page_wait_seconds(args.wait_seconds);

    let server = WebSearchServer {
        engine,
        browser_mgr,
    };

    info!("websearch-mcp starting on stdio …");
    serve_server(server, stdio()).await?.waiting().await?;

    info!("server stopped");
    Ok(())
}
