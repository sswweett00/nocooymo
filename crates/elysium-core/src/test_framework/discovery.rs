//! Test discovery — automatic discovery of tests from the registry.

use super::{TestCase, TestRegistry};
use std::collections::HashSet;
use std::path::{Path, PathBuf};

// ===========================================================================
// Discovery result
// ===========================================================================

#[derive(Debug, Clone)]
pub struct DiscoveryResult {
    pub tests: Vec<TestCase>,
    pub suites: Vec<String>,
    pub tags: Vec<String>,
    pub total: usize,
    pub skipped: usize,
}

impl DiscoveryResult {
    pub fn new(tests: Vec<TestCase>) -> Self {
        let skipped = tests.iter().filter(|t| t.skip).count();
        let suites: HashSet<String> = tests.iter().map(|t| t.suite.clone()).collect();
        let tags: HashSet<String> = tests.iter().flat_map(|t| t.tags.clone()).collect();
        Self {
            total: tests.len(),
            skipped,
            suites: suites.into_iter().collect(),
            tags: tags.into_iter().collect(),
            tests,
        }
    }

    pub fn by_suite(mut self, suite: &str) -> Self {
        self.tests.retain(|t| t.suite == suite);
        self.total = self.tests.len();
        self
    }

    pub fn by_tag(mut self, tag: &str) -> Self {
        self.tests.retain(|t| t.tags.contains(tag));
        self.total = self.tests.len();
        self
    }

    pub fn exclude_tag(mut self, tag: &str) -> Self {
        self.tests.retain(|t| !t.tags.contains(tag));
        self.total = self.tests.len();
        self
    }

    pub fn by_filter(mut self, filter: &str) -> Self {
        let lower = filter.to_lowercase();
        self.tests.retain(|t| {
            t.name.to_lowercase().contains(&lower) || t.suite.to_lowercase().contains(&lower)
        });
        self.total = self.tests.len();
        self
    }

    pub fn only_skipped(mut self) -> Self {
        self.tests.retain(|t| t.skip);
        self.total = self.tests.len();
        self
    }

    pub fn only_not_skipped(mut self) -> Self {
        self.tests.retain(|t| !t.skip);
        self.total = self.tests.len();
        self
    }
}

// ===========================================================================
// Discovery helpers
// ===========================================================================

/// Discover all registered tests.
pub fn discover_all() -> DiscoveryResult {
    let tests = TestRegistry::global().all();
    DiscoveryResult::new(tests)
}

/// Discover tests from the global registry, optionally filtering by suite.
pub fn discover_suite(suite: &str) -> DiscoveryResult {
    discover_all().by_suite(suite)
}

/// Discover tests by tag.
pub fn discover_tag(tag: &str) -> DiscoveryResult {
    discover_all().by_tag(tag)
}

/// Discover tests by name filter.
pub fn discover_filter(filter: &str) -> DiscoveryResult {
    discover_all().by_filter(filter)
}

/// Discover tests by a custom predicate.
pub fn discover_with<F: Fn(&TestCase) -> bool>(predicate: F) -> DiscoveryResult {
    let tests = TestRegistry::global().filter(predicate);
    DiscoveryResult::new(tests)
}

/// Get a summary of the discovery result.
pub fn discovery_summary(result: &DiscoveryResult) -> String {
    let mut buf = String::new();
    let _ = writeln!(buf, "Discovered {} tests ({} skipped)", result.total, result.skipped);
    let _ = writeln!(buf, "Suites: {:?}", result.suites);
    let _ = writeln!(buf, "Tags: {:?}", result.tags);
    buf
}
