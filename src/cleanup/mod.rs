// ---------------------------------------------------------------------------
// cleanup/mod.rs — Shared definitions and re-exports for the cleanup pipeline
//
// Child modules:
//   html.rs     — HTML pre-clean: strip noise elements before Markdown conv.
//   markdown.rs — Markdown post-clean: strip UI chrome, ads, tracking.
//
// Public API (re-exported):
//   strip_noise()     — from html.rs
//   clean_markdown()  — from markdown.rs
// ---------------------------------------------------------------------------

pub mod html;
pub mod markdown;

use std::sync::LazyLock;

/// Collapse 3+ consecutive newlines to 2. Shared by both html and markdown
/// post-processing steps.
pub(crate) static RE_NEWLINES: LazyLock<regex::Regex> =
    LazyLock::new(|| regex::Regex::new(r"\n{3,}").expect("Invalid RE_NEWLINES regex"));

pub use html::strip_noise;
pub use markdown::clean_markdown;
