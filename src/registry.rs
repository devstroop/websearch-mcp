// ---------------------------------------------------------------------------
// registry.rs — Provider registry and name resolution
//
// Responsibilities:
///   - Register all known search providers (brave, duckduckgo, google)
//   - Resolve a provider by name (exact or prefix match)
//   - List available provider names
//
// This module owns the provider registry — main.rs just delegates
// resolution to it. Providers themselves live in providers/ and implement
// the SearchProvider trait.
// ---------------------------------------------------------------------------
use std::collections::HashMap;

use crate::providers::{brave::Brave, duckduckgo::DuckDuckGo, google::Google, SearchProvider};

pub struct SearchEngine {
    providers: HashMap<&'static str, Box<dyn SearchProvider>>,
}

impl SearchEngine {
    /// Create a new registry containing all built-in providers.
    pub fn new() -> Self {
        let mut providers: HashMap<&'static str, Box<dyn SearchProvider>> = HashMap::new();

        for prov in [
            Box::new(DuckDuckGo) as Box<dyn SearchProvider>,
            Box::new(Google),
            Box::new(Brave),
        ] {
            providers.insert(prov.provider_kind(), prov);
        }

        Self { providers }
    }

    /// Resolve a provider name (case-insensitive, supports prefix matching).
    /// Returns `None` if no provider matches.
    pub fn resolve(&self, name: &str) -> Option<&dyn SearchProvider> {
        let name = name.trim().to_lowercase();
        // Exact match first.
        if let Some(p) = self.providers.get(name.as_str()) {
            return Some(p.as_ref());
        }
        // Prefix match: if input is a prefix of a key or vice versa.
        self.providers.iter().find_map(|(key, prov)| {
            if key.starts_with(&name) || name.starts_with(key) {
                Some(prov.as_ref())
            } else {
                None
            }
        })
    }

    /// List all registered provider names.
    pub fn available_providers(&self) -> Vec<&'static str> {
        self.providers.keys().copied().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_engine() -> SearchEngine {
        SearchEngine::new()
    }

    #[test]
    fn test_exact_match_brave() {
        let engine = make_engine();
        let prov = engine.resolve("brave").unwrap();
        assert_eq!(prov.provider_kind(), "brave");
    }

    #[test]
    fn test_exact_match_duckduckgo() {
        let engine = make_engine();
        let prov = engine.resolve("duckduckgo").unwrap();
        assert_eq!(prov.provider_kind(), "duckduckgo");
    }

    #[test]
    fn test_exact_match_google() {
        let engine = make_engine();
        let prov = engine.resolve("google").unwrap();
        assert_eq!(prov.provider_kind(), "google");
    }

    #[test]
    fn test_case_insensitive() {
        let engine = make_engine();
        assert_eq!(engine.resolve("Brave").unwrap().provider_kind(), "brave");
        assert_eq!(engine.resolve("BRAVE").unwrap().provider_kind(), "brave");
        assert_eq!(
            engine.resolve("DuckDuckGo").unwrap().provider_kind(),
            "duckduckgo"
        );
        assert_eq!(engine.resolve("GOOGLE").unwrap().provider_kind(), "google");
    }

    #[test]
    fn test_prefix_match() {
        let engine = make_engine();
        assert_eq!(engine.resolve("b").unwrap().provider_kind(), "brave");
        assert_eq!(engine.resolve("g").unwrap().provider_kind(), "google");
        assert_eq!(
            engine.resolve("duck").unwrap().provider_kind(),
            "duckduckgo"
        );
    }

    #[test]
    fn test_trim_whitespace() {
        let engine = make_engine();
        assert_eq!(
            engine.resolve("  brave  ").unwrap().provider_kind(),
            "brave"
        );
    }

    #[test]
    fn test_unknown_provider() {
        let engine = make_engine();
        assert!(engine.resolve("yahoo").is_none());
        assert!(engine.resolve("bing").is_none());
    }

    #[test]
    fn test_available_providers() {
        let engine = make_engine();
        let mut providers = engine.available_providers();
        providers.sort();
        assert_eq!(providers, vec!["brave", "duckduckgo", "google"]);
    }

    #[test]
    fn test_prefix_conflict_does_not_panic() {
        let engine = make_engine();
        // "br" matches "brave" prefix
        assert_eq!(engine.resolve("br").unwrap().provider_kind(), "brave");
        // "go" matches "google" prefix
        assert_eq!(engine.resolve("go").unwrap().provider_kind(), "google");
    }
}
