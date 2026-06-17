// ---------------------------------------------------------------------------
// tests/cleanup.rs — Integration tests for the HTML→Markdown cleanup pipeline
// ---------------------------------------------------------------------------

use websearch::Config;

/// Verifies that Config::from_args accepts valid Args.
#[test]
fn test_valid_config_from_args() {
    let args = websearch::config::Args {
        profile: None,
        headless: false,
        chrome: None,
        port: None,
        wait_seconds: 4,
    };
    let config = Config::from_args(args).unwrap();
    assert_eq!(config.wait_seconds, 4);
    assert!(!config.headless);
}

/// Verifies that Config::from_args rejects wait_seconds=0.
#[test]
fn test_zero_wait_seconds_rejected() {
    let args = websearch::config::Args {
        profile: None,
        headless: false,
        chrome: None,
        port: None,
        wait_seconds: 0,
    };
    let err = Config::from_args(args).unwrap_err();
    assert!(err
        .to_string()
        .contains("wait_seconds must be greater than 0"));
}

/// Verifies that Config::from_args rejects wait_seconds > 120.
#[test]
fn test_excessive_wait_seconds_rejected() {
    let args = websearch::config::Args {
        profile: None,
        headless: false,
        chrome: None,
        port: None,
        wait_seconds: 999,
    };
    let err = Config::from_args(args).unwrap_err();
    assert!(err.to_string().contains("must be 120 or less"));
}

/// Verifies that the Error type can be converted to String.
#[test]
fn test_error_types_are_displayable() {
    let err = websearch::Error::BrowserLaunch("test".into());
    assert_eq!(err.to_string(), "browser launch failed: test");

    let err = websearch::Error::UnknownProvider("nope".into());
    assert_eq!(err.to_string(), "unknown search provider: nope");

    let err = websearch::Error::Config("bad".into());
    assert_eq!(err.to_string(), "configuration error: bad");
}
