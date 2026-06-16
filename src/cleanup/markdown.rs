// ---------------------------------------------------------------------------
// cleanup/markdown.rs — Post-process Markdown output to strip remaining
// UI chrome, ads, and tracking junk that survives HTML→Markdown conversion.
// ---------------------------------------------------------------------------

use std::sync::LazyLock;

use super::RE_NEWLINES;

static RE_DDG_ICON: LazyLock<regex::Regex> = LazyLock::new(|| {
    regex::Regex::new(r"^!\[\]\(.*external-content.*duckduckgo\.com.*\)$")
        .expect("Invalid RE_DDG_ICON regex")
});

/// Post-process Markdown output to strip remaining UI chrome, ads, and
/// tracking junk that survives HTML→Markdown conversion.
pub fn clean_markdown(md: &str) -> String {
    let mut lines: Vec<&str> = md.lines().collect();

    // Patterns that identify junk lines to remove.
    let junk_patterns: &[&str] = &[
        // Google: navigation tabs and filter chrome
        "Skip to main content",
        "Accessibility help",
        "Quick Settings",
        "AI Mode",
        "Sign in",
        "Past hour",
        "Past 24 hours",
        "Past week",
        "Past month",
        "Past year",
        "Custom range",
        "Customised date range",
        "All results",
        "Verbatim",
        "Advanced Search",
        "Ctrl+Shift+X",
        "Refine results",
        "Product rating",
        "See more",
        // Brave: AI answer chrome
        "AI-generated answer",
        "Please verify critical facts",
        "Elaborate",
        "Copy",
        "Share",
        "View all",
        "People also ask",
        "Learn more",
        // DuckDuckGo: ad indicators
        "Ad Viewing ads is privacy protected",
        "Viewing ads is privacy protected",
        "Ad clicks are managed by Microsoft",
        "truncated",
        // Generic junk
        "About ", // "About 36,90,00,000 results"
        "profile picture",
        "Profile Picture",
        "Privacy",
        "Terms",
        "Feedback",
    ];

    // Remove lines that contain junk patterns or are empty image references.
    lines.retain(|line| {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            return true; // Keep empty lines for spacing
        }
        // Remove pure image references like `![]([image])`
        if trimmed.starts_with("![](") || trimmed == "![]([image])" {
            return false;
        }
        // Remove lines with only an icon/favicon image
        if RE_DDG_ICON.is_match(trimmed) {
            return false;
        }
        // Remove tracking/ad URLs
        if trimmed.contains("duckduckgo.com/y.js") || trimmed.contains("bing.com/aclick") {
            return false;
        }
        // Remove lines matching known junk text
        !junk_patterns.iter().any(|pat| trimmed.contains(pat))
    });

    // Clean up: strip leading empty lines, trailing empty lines, and
    // collapse runs of >2 consecutive empty lines.
    let mut result = lines.join("\n");
    // Remove leading blank lines
    while result.starts_with('\n') {
        result.remove(0);
    }
    // Remove trailing blank lines
    while result.ends_with('\n') {
        result.pop();
    }
    // Collapse 3+ consecutive newlines to 2
    let result = RE_NEWLINES.replace_all(&result, "\n\n");

    result.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_removes_junk_lines() {
        let md = "some content\nSign in\nmore content\nPrivacy\nfeedback";
        let result = clean_markdown(md);
        assert!(result.contains("some content"));
        assert!(result.contains("more content"));
        assert!(!result.contains("Sign in"));
        assert!(!result.contains("Privacy"));
    }

    #[test]
    fn test_removes_image_refs() {
        let md = "text\n![](https://example.com/img.png)\nmore";
        let result = clean_markdown(md);
        assert!(result.contains("text"));
        assert!(result.contains("more"));
        assert!(!result.contains("![](https://example.com/img.png)"));
    }

    #[test]
    fn test_keeps_empty_lines_for_spacing() {
        let md = "line1\n\n\nline2";
        let result = clean_markdown(md);
        assert!(result.contains("line1"));
        assert!(result.contains("line2"));
        assert!(
            !result.contains("\n\n\n"),
            "should not have 3+ consecutive newlines"
        );
    }

    #[test]
    fn test_strips_leading_trailing_blank_lines() {
        let md = "\n\ncontent\n";
        let result = clean_markdown(md);
        assert_eq!(result, "content");
    }

    #[test]
    fn test_removes_google_chrome() {
        let md = "results\nQuick Settings\nAI Mode\nPast hour\nreal result";
        let result = clean_markdown(md);
        assert!(result.contains("results"));
        assert!(result.contains("real result"));
        assert!(!result.contains("Quick Settings"));
        assert!(!result.contains("AI Mode"));
        assert!(!result.contains("Past hour"));
    }

    #[test]
    fn test_removes_brave_ai_chrome() {
        let md = "answer\nAI-generated answer\nPlease verify critical facts\nreal content";
        let result = clean_markdown(md);
        assert!(result.contains("answer"));
        assert!(result.contains("real content"));
        assert!(!result.contains("AI-generated answer"));
        assert!(!result.contains("Please verify critical facts"));
    }

    #[test]
    fn test_removes_ddg_ad_indicators() {
        let md = "result\nAd Viewing ads is privacy protected\nreal";
        let result = clean_markdown(md);
        assert!(result.contains("result"));
        assert!(result.contains("real"));
        assert!(!result.contains("Ad Viewing ads is privacy protected"));
    }

    #[test]
    fn test_preserves_normal_text() {
        let md = "Hello world\nThis is a result\nAnd another one";
        let result = clean_markdown(md);
        assert_eq!(result, md);
    }
}
