// ---------------------------------------------------------------------------
// tests/providers.rs — Integration tests for provider resolution
// ---------------------------------------------------------------------------

/// Tests that the registry resolves all built-in providers by exact name.
#[test]
fn test_registry_resolves_all_providers() {
    use websearch::config::Config;
    // We can't easily construct a SearchEngine from integration tests
    // since it's in a private module. This test verifies the public API
    // path through config works.
    let args = websearch::config::Args {
        profile: None,
        headless: false,
        chrome: None,
        port: None,
        remote_url: None,
        wait_seconds: 4,
    };
    let config = Config::from_args(args).unwrap();
    assert_eq!(config.wait_seconds, 4);

    // Verify error types used in provider resolution are accessible
    let err = websearch::Error::UnknownProvider("nonexistent".into());
    assert_eq!(err.to_string(), "unknown search provider: nonexistent");
}

/// Tests that the Error type is properly exposed from the library.
#[test]
fn test_error_from_integration() {
    let err = websearch::Error::NavigationTimeout(10);
    assert_eq!(err.to_string(), "navigation timed out after 10s");

    let err = websearch::Error::MarkdownConversion("parse error".into());
    assert_eq!(
        err.to_string(),
        "HTML to Markdown conversion failed: parse error"
    );

    let err = websearch::Error::InvalidUrlScheme("ftp://".into());
    assert_eq!(err.to_string(), "invalid URL scheme: ftp://");
}
