// ---------------------------------------------------------------------------
// cleanup/html.rs — Strip noise elements from raw HTML *before* Markdown
// conversion. Removes structural elements (head, nav, header, footer, form,
// script, style, svg, iframe, img), tracking/ad anchors, and long inline
// style attributes that would produce noisy Markdown output.
// ---------------------------------------------------------------------------

use std::sync::LazyLock;

use super::RE_NEWLINES;

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_strip_script() {
        let html = "<html><script>alert('xss')</script><body>hello</body></html>";
        let result = strip_noise(html);
        assert!(!result.contains("<script"), "should remove script tags");
        assert!(result.contains("hello"), "should keep content");
    }

    #[test]
    fn test_strip_style() {
        let html = "<html><style>body { color: red; }</style><body>text</body></html>";
        let result = strip_noise(html);
        assert!(!result.contains("<style"), "should remove style tags");
        assert!(result.contains("text"), "should keep content");
    }

    #[test]
    fn test_strip_nav() {
        let html = "<nav><a href=\"/\">Home</a></nav><p>content</p>";
        let result = strip_noise(html);
        assert!(!result.contains("<nav"), "should remove nav tags");
        assert!(result.contains("content"), "should keep content");
    }

    #[test]
    fn test_strip_header() {
        let html = "<header><h1>Title</h1></header><p>body</p>";
        let result = strip_noise(html);
        assert!(!result.contains("<header"), "should remove header tags");
        assert!(result.contains("body"), "should keep content");
    }

    #[test]
    fn test_strip_footer() {
        let html = "<footer>© 2024</footer><p>main</p>";
        let result = strip_noise(html);
        assert!(!result.contains("<footer"), "should remove footer tags");
        assert!(result.contains("main"), "should keep content");
    }

    #[test]
    fn test_strip_iframe() {
        let html = "<iframe src=\"ad.html\"></iframe><p>real</p>";
        let result = strip_noise(html);
        assert!(!result.contains("<iframe"), "should remove iframe");
        assert!(result.contains("real"), "should keep content");
    }

    #[test]
    fn test_strip_img() {
        let html = "<img src=\"photo.jpg\" alt=\"pic\"><p>text</p>";
        let result = strip_noise(html);
        assert!(!result.contains("<img"), "should remove img tags");
        assert!(result.contains("text"), "should keep content");
    }

    #[test]
    fn test_strip_svg() {
        let html = "<svg><circle r=\"10\"/></svg><p>content</p>";
        let result = strip_noise(html);
        assert!(!result.contains("<svg"), "should remove svg tags");
        assert!(result.contains("content"), "should keep content");
    }

    #[test]
    fn test_collapse_newlines() {
        let html = "<p>a</p>\n\n\n\n<p>b</p>";
        let result = strip_noise(html);
        assert!(result.contains("a"));
        assert!(result.contains("b"));
        // Should have at most 2 consecutive newlines
        assert!(
            !result.contains("\n\n\n"),
            "should not have 3+ consecutive newlines"
        );
    }

    #[test]
    fn test_preserves_text_content() {
        let html = "<div><p>Hello world</p><span>more</span></div>";
        let result = strip_noise(html);
        assert!(result.contains("Hello world"));
        assert!(result.contains("more"));
    }
}
