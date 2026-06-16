// ---------------------------------------------------------------------------
// cleanup.rs — HTML pre-clean and Markdown post-clean for search results
//
// Responsibilities:
//   - strip_noise(): strip known noise elements from raw HTML *before*
//     the Markdown converter runs (nav, header, footer, ads, tracking, etc.)
//   - clean_markdown(): strip UI chrome, ad labels, and tracking remnants
//     that survive the HTML→Markdown conversion
//
// Both functions are called by providers/mod.rs::navigate_and_get_markdown().
// ---------------------------------------------------------------------------

use std::sync::LazyLock;

static RE_HEAD: LazyLock<regex::Regex> = LazyLock::new(|| {
    regex::Regex::new(r"(?is)<head[^>]*>.*?</head>").expect("Invalid RE_HEAD regex")
});
static RE_SCRIPT: LazyLock<regex::Regex> = LazyLock::new(|| {
    regex::Regex::new(r"(?is)<script[^>]*>.*?</script>").expect("Invalid RE_SCRIPT regex")
});
static RE_STYLE: LazyLock<regex::Regex> = LazyLock::new(|| {
    regex::Regex::new(r"(?is)<style[^>]*>.*?</style>").expect("Invalid RE_STYLE regex")
});
static RE_SVG: LazyLock<regex::Regex> =
    LazyLock::new(|| regex::Regex::new(r"(?is)<svg[^>]*>.*?</svg>").expect("Invalid RE_SVG regex"));
static RE_NOSCRIPT: LazyLock<regex::Regex> = LazyLock::new(|| {
    regex::Regex::new(r"(?is)<noscript[^>]*>.*?</noscript>").expect("Invalid RE_NOSCRIPT regex")
});
static RE_NAV: LazyLock<regex::Regex> =
    LazyLock::new(|| regex::Regex::new(r"(?is)<nav[^>]*>.*?</nav>").expect("Invalid RE_NAV regex"));
static RE_HEADER: LazyLock<regex::Regex> = LazyLock::new(|| {
    regex::Regex::new(r"(?is)<header[^>]*>.*?</header>").expect("Invalid RE_HEADER regex")
});
static RE_FOOTER: LazyLock<regex::Regex> = LazyLock::new(|| {
    regex::Regex::new(r"(?is)<footer[^>]*>.*?</footer>").expect("Invalid RE_FOOTER regex")
});
static RE_ASIDE: LazyLock<regex::Regex> = LazyLock::new(|| {
    regex::Regex::new(r"(?is)<aside[^>]*>.*?</aside>").expect("Invalid RE_ASIDE regex")
});
static RE_FORM: LazyLock<regex::Regex> = LazyLock::new(|| {
    regex::Regex::new(r"(?is)<form[^>]*>.*?</form>").expect("Invalid RE_FORM regex")
});
static RE_TEMPLATE: LazyLock<regex::Regex> = LazyLock::new(|| {
    regex::Regex::new(r"(?is)<template[^>]*>.*?</template>").expect("Invalid RE_TEMPLATE regex")
});
static RE_DIALOG: LazyLock<regex::Regex> = LazyLock::new(|| {
    regex::Regex::new(r"(?is)<dialog[^>]*>.*?</dialog>").expect("Invalid RE_DIALOG regex")
});
static RE_DDG_TRACK: LazyLock<regex::Regex> = LazyLock::new(|| {
    regex::Regex::new(r#"(?is)<a[^>]*href="[^"]*duckduckgo\.com/y\.js[^"]*"[^>]*>.*?</a>"#)
        .expect("Invalid RE_DDG_TRACK regex")
});
static RE_BING_ACLICK: LazyLock<regex::Regex> = LazyLock::new(|| {
    regex::Regex::new(r#"(?is)<a[^>]*href="[^"]*bing\.com/aclick[^"]*"[^>]*>.*?</a>"#)
        .expect("Invalid RE_BING_ACLICK regex")
});
static RE_LONG_HREF: LazyLock<regex::Regex> = LazyLock::new(|| {
    regex::Regex::new(r#"(?is)<a[^>]*href="[^"]{400,}"[^>]*>.*?</a>"#)
        .expect("Invalid RE_LONG_HREF regex")
});
static RE_DDG_AD_DIV: LazyLock<regex::Regex> = LazyLock::new(|| {
    regex::Regex::new(
        r#"(?is)<div[^>]*>.*?Viewing ads is privacy protected by DuckDuckGo\..*?</div>"#,
    )
    .expect("Invalid RE_DDG_AD_DIV regex")
});
static RE_FAVICON: LazyLock<regex::Regex> = LazyLock::new(|| {
    regex::Regex::new(r#"(?is)<link[^>]*rel="(?:shortcut )?icon"[^>]*>"#)
        .expect("Invalid RE_FAVICON regex")
});
static RE_IFRAME: LazyLock<regex::Regex> = LazyLock::new(|| {
    regex::Regex::new(r"(?is)<iframe[^>]*>.*?</iframe>").expect("Invalid RE_IFRAME regex")
});
static RE_IMG: LazyLock<regex::Regex> =
    LazyLock::new(|| regex::Regex::new(r#"(?is)<img[^>]*>"#).expect("Invalid RE_IMG regex"));
static RE_BASE64: LazyLock<regex::Regex> = LazyLock::new(|| {
    regex::Regex::new(r#"data:image/[^;]+;base64[^"'\s)]+"#).expect("Invalid RE_BASE64 regex")
});
static RE_LONG_STYLE: LazyLock<regex::Regex> = LazyLock::new(|| {
    regex::Regex::new(r#"(?i)style="[^"]{80,}""#).expect("Invalid RE_LONG_STYLE regex")
});
static RE_NEWLINES: LazyLock<regex::Regex> =
    LazyLock::new(|| regex::Regex::new(r"\n{3,}").expect("Invalid RE_NEWLINES regex"));
static RE_DDG_ICON: LazyLock<regex::Regex> = LazyLock::new(|| {
    regex::Regex::new(r"^!\[\]\(.*external-content.*duckduckgo\.com.*\)$")
        .expect("Invalid RE_DDG_ICON regex")
});

/// Remove known noise elements from HTML so the Markdown conversion is clean.
pub fn strip_noise(html: &str) -> String {
    let s = html;

    // Remove whole <head>...</head>
    let s = RE_HEAD.replace_all(s, "");
    // Remove <script>...</script>
    let s = RE_SCRIPT.replace_all(&s, "");
    // Remove <style>...</style>
    let s = RE_STYLE.replace_all(&s, "");
    // Remove <svg>...</svg>
    let s = RE_SVG.replace_all(&s, "");
    // Remove <noscript>...</noscript>
    let s = RE_NOSCRIPT.replace_all(&s, "");
    // Remove <nav>...</nav> — navigation bars, filters, tab bars
    let s = RE_NAV.replace_all(&s, "");
    // Remove <header>...</header> — page headers with sign-in, settings
    let s = RE_HEADER.replace_all(&s, "");
    // Remove <footer>...</footer>
    let s = RE_FOOTER.replace_all(&s, "");
    // Remove <aside>...</aside> — sidebars
    let s = RE_ASIDE.replace_all(&s, "");
    // Remove <form>...</form> — search forms, sign-in forms
    let s = RE_FORM.replace_all(&s, "");
    // Remove <template>...</template>
    let s = RE_TEMPLATE.replace_all(&s, "");
    // Remove <dialog>...</dialog>
    let s = RE_DIALOG.replace_all(&s, "");

    // Strip anchor tags with tracking/ad domains in href
    let s = RE_DDG_TRACK.replace_all(&s, "");
    let s = RE_BING_ACLICK.replace_all(&s, "");

    // Strip any <a> with href longer than 400 chars — they're tracking URLs
    let s = RE_LONG_HREF.replace_all(&s, "");

    // Strip DuckDuckGo ad container <div>s (identified by ad attribution text)
    let s = RE_DDG_AD_DIV.replace_all(&s, "");

    // Remove favicon <link> tags
    let s = RE_FAVICON.replace_all(&s, "");

    // Remove <iframe>...</iframe>
    let s = RE_IFRAME.replace_all(&s, "");

    // Remove <img> tags (they convert to noisy ![](...) regardless)
    let s = RE_IMG.replace_all(&s, "");

    // Remove base64 data URIs
    let s = RE_BASE64.replace_all(&s, "[image]");
    // Remove long inline style attributes
    let s = RE_LONG_STYLE.replace_all(&s, "");

    // Collapse multiple consecutive blank lines into at most one.
    let s = RE_NEWLINES.replace_all(&s, "\n\n");

    s.to_string()
}

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
