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

use crate::providers::{
    brave::Brave, duckduckgo::DuckDuckGo, google::Google, SearchProvider,
};

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
