// ---------------------------------------------------------------------------
// error.rs — Typed error enum for the websearch library
//
// This module defines the public error type and a convenience Result alias.
// Internal modules may still use anyhow for convenience, but all public API
// functions return `Result<T>` (our typed error), which automatically
// converts to anyhow at the binary boundary via std::error::Error.
// ---------------------------------------------------------------------------

/// Typed error for all websearch library operations.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Browser launch or configuration failure.
    #[error("browser launch failed: {0}")]
    BrowserLaunch(String),

    /// General browser operation failure.
    #[error("browser error: {0}")]
    Browser(String),

    /// Page navigation timed out.
    #[error("navigation timed out after {0}s")]
    NavigationTimeout(u64),

    /// Navigation failed (other than timeout).
    #[error("navigation failed: {0}")]
    Navigation(String),

    /// HTML to Markdown conversion failed.
    #[error("HTML to Markdown conversion failed: {0}")]
    MarkdownConversion(String),

    /// Unknown search provider name.
    #[error("unknown search provider: {0}")]
    UnknownProvider(String),

    /// Invalid URL scheme.
    #[error("invalid URL scheme: {0}")]
    InvalidUrlScheme(String),

    /// No active tab or tab operation failed.
    #[error("tab error: {0}")]
    Tab(String),

    /// Element not found by CSS selector.
    #[error("element not found: {0}")]
    ElementNotFound(String),

    /// Screenshot capture or encoding failed.
    #[error("screenshot failed: {0}")]
    Screenshot(String),

    /// Configuration validation error.
    #[error("configuration error: {0}")]
    Config(String),

    /// I/O error (wraps std::io::Error).
    #[error(transparent)]
    Io(#[from] std::io::Error),

    /// Catch-all for errors that don't fit other variants.
    #[error("{0}")]
    Other(String),
}

/// Convenience alias for library results.
pub type Result<T> = std::result::Result<T, Error>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display_browser_launch() {
        let err = Error::BrowserLaunch("chrome not found".into());
        assert_eq!(err.to_string(), "browser launch failed: chrome not found");
    }

    #[test]
    fn test_error_display_navigation_timeout() {
        let err = Error::NavigationTimeout(30);
        assert_eq!(err.to_string(), "navigation timed out after 30s");
    }

    #[test]
    fn test_error_display_unknown_provider() {
        let err = Error::UnknownProvider("yahoo".into());
        assert_eq!(err.to_string(), "unknown search provider: yahoo");
    }

    #[test]
    fn test_error_display_config() {
        let err = Error::Config("invalid value".into());
        assert_eq!(err.to_string(), "configuration error: invalid value");
    }

    #[test]
    fn test_error_display_io_transparent() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let err = Error::Io(io_err);
        assert_eq!(err.to_string(), "file not found");
    }
}
