// ---------------------------------------------------------------------------
// config.rs — CLI argument parsing and validated configuration
//
// Responsibilities:
//   - Parse CLI arguments via clap
//   - Validate argument combinations (bounds, sanitization)
//   - Produce a validated Config struct consumed by lib::serve()
//
// Module boundaries:
//   - Args is the raw clap-parsed struct, exposed for the binary entrypoint
//   - Config is the validated, resolved configuration
// ---------------------------------------------------------------------------

use std::path::PathBuf;

use clap::Parser;

use crate::error::{Error, Result};

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

    /// Connect to an existing Chrome instance via DevTools WebSocket URL
    /// (e.g. ws://localhost:9222). When set, skips local browser launch.
    #[arg(long, env = "REMOTE_URL")]
    pub remote_url: Option<String>,

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

/// Validated server configuration produced from `Args`.
///
/// All paths are resolved, all defaults applied, and all values have been
/// validated before a `Config` is constructed. This struct is the single
/// source of truth for server configuration consumed by `websearch::serve()`.
#[derive(Debug, Clone)]
pub struct Config {
    /// Resolved profile directory path.
    pub profile_dir: PathBuf,

    /// Whether to run the browser in headless mode.
    pub headless: bool,

    /// Optional path to the Chrome/Chromium executable.
    pub chrome: Option<PathBuf>,

    /// Optional debug port for DevTools.
    pub port: Option<u16>,

    /// Optional remote Chrome DevTools WebSocket URL.
    /// When set, connects to an existing browser instead of launching one.
    pub remote_url: Option<String>,

    /// Seconds to wait for page rendering (1–120).
    pub wait_seconds: u64,
}

impl Config {
    /// Parse CLI arguments and produce a validated `Config`.
    ///
    /// Validates:
    /// - `wait_seconds` must be in range [1, 120]
    pub fn from_args(args: Args) -> Result<Self> {
        let profile_dir = args.profile_dir();

        if args.wait_seconds == 0 {
            return Err(Error::Config(
                "wait_seconds must be greater than 0 (got 0)".into(),
            ));
        }
        if args.wait_seconds > 120 {
            return Err(Error::Config(format!(
                "wait_seconds must be 120 or less (got {})",
                args.wait_seconds
            )));
        }

        Ok(Config {
            profile_dir,
            headless: args.headless,
            chrome: args.chrome,
            port: args.port,
            remote_url: args.remote_url,
            wait_seconds: args.wait_seconds,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_wait_seconds_valid() {
        let args = Args {
            profile: None,
            headless: false,
            chrome: None,
            port: None,
            remote_url: None,
            wait_seconds: 4,
        };
        let config = Config::from_args(args).unwrap();
        assert_eq!(config.wait_seconds, 4);
        assert!(!config.headless);
    }

    #[test]
    fn test_zero_wait_seconds_rejected() {
        let args = Args {
            profile: None,
            headless: false,
            chrome: None,
            port: None,
            remote_url: None,
            wait_seconds: 0,
        };
        let err = Config::from_args(args).unwrap_err();
        assert!(err
            .to_string()
            .contains("wait_seconds must be greater than 0"));
    }

    #[test]
    fn test_wait_seconds_too_high_rejected() {
        let args = Args {
            profile: None,
            headless: false,
            chrome: None,
            port: None,
            remote_url: None,
            wait_seconds: 121,
        };
        let err = Config::from_args(args).unwrap_err();
        assert!(err.to_string().contains("must be 120 or less"));
    }

    #[test]
    fn test_headless_flag_passed_through() {
        let args = Args {
            profile: None,
            headless: true,
            chrome: None,
            port: None,
            remote_url: None,
            wait_seconds: 5,
        };
        let config = Config::from_args(args).unwrap();
        assert!(config.headless);
    }

    #[test]
    fn test_profile_dir_defaults_to_data_dir() {
        let args = Args {
            profile: None,
            headless: false,
            chrome: None,
            port: None,
            remote_url: None,
            wait_seconds: 4,
        };
        let config = Config::from_args(args).unwrap();
        assert!(config
            .profile_dir
            .to_string_lossy()
            .ends_with("websearch-mcp/chrome-profile"));
    }

    #[test]
    fn test_profile_dir_uses_custom_path() {
        let custom = PathBuf::from("/tmp/my-profile");
        let args = Args {
            profile: Some(custom.clone()),
            headless: false,
            chrome: None,
            port: None,
            remote_url: None,
            wait_seconds: 4,
        };
        let config = Config::from_args(args).unwrap();
        assert_eq!(config.profile_dir, custom);
    }

    #[test]
    fn test_chrome_path_passed_through() {
        let chrome = PathBuf::from("/usr/bin/chromium");
        let args = Args {
            profile: None,
            headless: false,
            chrome: Some(chrome.clone()),
            port: None,
            remote_url: None,
            wait_seconds: 4,
        };
        let config = Config::from_args(args).unwrap();
        assert_eq!(config.chrome, Some(chrome));
    }

    #[test]
    fn test_port_passed_through() {
        let args = Args {
            profile: None,
            headless: false,
            chrome: None,
            port: Some(9222),
            remote_url: None,
            wait_seconds: 4,
        };
        let config = Config::from_args(args).unwrap();
        assert_eq!(config.port, Some(9222));
    }

    #[test]
    fn test_edge_case_wait_seconds_1() {
        let args = Args {
            profile: None,
            headless: false,
            chrome: None,
            port: None,
            remote_url: None,
            wait_seconds: 1,
        };
        let config = Config::from_args(args).unwrap();
        assert_eq!(config.wait_seconds, 1);
    }

    #[test]
    fn test_edge_case_wait_seconds_120() {
        let args = Args {
            profile: None,
            headless: false,
            chrome: None,
            port: None,
            remote_url: None,
            wait_seconds: 120,
        };
        let config = Config::from_args(args).unwrap();
        assert_eq!(config.wait_seconds, 120);
    }
}
