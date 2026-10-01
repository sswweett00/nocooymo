//! Automation — CI/CD integration, test filtering, parallel execution.

use super::{RunSummary, TestConfig};
use std::path::PathBuf;

// ===========================================================================
// CI/CD integration
// ===========================================================================

/// Simulates a CI environment by wrapping the test run and producing a structured result.
#[derive(Debug, Clone)]
pub struct CiIntegration {
    pub config: TestConfig,
    pub environment: CiEnvironment,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CiEnvironment {
    GitHubActions,
    GitLabCi,
    Jenkins,
    AzurePipelines,
    Local,
}

impl CiIntegration {
    pub fn new(config: TestConfig, env: CiEnvironment) -> Self {
        Self { config, environment: env }
    }

    /// Run tests and produce a CI exit code (0 = success, 1 = failure).
    pub fn run_and_exit(&self) -> i32 {
        let config = std::sync::Arc::new(self.config.clone());
        let summary = super::runner::run_all(config);

        // Print summary
        summary.print_summary();

        // Write reports
        let generator = super::reporting::ReportGenerator::new(&self.config.output_dir);
        let _ = generator.write(&summary);

        // In CI we print annotations
        match self.environment {
            CiEnvironment::GitHubActions => self.print_github_annotations(&summary),
            CiEnvironment::GitLabCi => self.print_gitlab_annotations(&summary),
            _ => {}
        }

        if summary.failed > 0 || summary.panicked > 0 {
            1
        } else {
            0
        }
    }

    fn print_github_annotations(&self, summary: &RunSummary) {
        for result in &summary.results {
            if result.is_failure() {
                eprintln!(
                    "::error file={}::{}",
                    result.suite,
                    result.message.clone().unwrap_or_default()
                );
            }
        }
    }

    fn print_gitlab_annotations(&self, summary: &RunSummary) {
        for result in &summary.results {
            if result.is_failure() {
                eprintln!(
                    "##[error]{}: {}",
                    result.suite,
                    result.message.clone().unwrap_or_default()
                );
            }
        }
    }
}

// ===========================================================================
// Test filter
// ===========================================================================

#[derive(Debug, Clone, Default)]
pub struct TestFilter {
    pub suites: Vec<String>,
    pub tags: Vec<String>,
    pub exclude_tags: Vec<String>,
    pub name_contains: Vec<String>,
}

impl TestFilter {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn suite(mut self, suite: impl Into<String>) -> Self {
        self.suites.push(suite.into());
        self
    }

    pub fn tag(mut self, tag: impl Into<String>) -> Self {
        self.tags.push(tag.into());
        self
    }

    pub fn exclude_tag(mut self, tag: impl Into<String>) -> Self {
        self.exclude_tags.push(tag.into());
        self
    }

    pub fn name_contains(mut self, name: impl Into<String>) -> Self {
        self.name_contains.push(name.into());
        self
    }

    pub fn matches(&self, test: &super::TestCase) -> bool {
        // Suite filter
        if !self.suites.is_empty() && !self.suites.iter().any(|s| s == &test.suite) {
            return false;
        }
        // Tag filter
        if !self.tags.is_empty() && !self.tags.iter().any(|t| test.tags.contains(t)) {
            return false;
        }
        // Exclude tags
        if self.exclude_tags.iter().any(|t| test.tags.contains(t)) {
            return false;
        }
        // Name filter
        if !self.name_contains.is_empty() && !self.name_contains.iter().any(|n| test.name.contains(n)) {
            return false;
        }
        true
    }
}

// ===========================================================================
// Parallel execution strategy
// ===========================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExecutionStrategy {
    Sequential,
    Parallel { max_threads: Option<usize> },
    Streaming { batch_size: usize },
}

impl Default for ExecutionStrategy {
    fn default() -> Self {
        Self::Parallel { max_threads: None }
    }
}

// ===========================================================================
// Watch mode
// ===========================================================================

#[derive(Debug, Clone)]
pub struct WatchConfig {
    pub watch_dirs: Vec<PathBuf>,
    pub debounce: std::time::Duration,
    pub run_on_change: bool,
}

impl Default for WatchConfig {
    fn default() -> Self {
        Self {
            watch_dirs: vec![PathBuf::from("src")],
            debounce: std::time::Duration::from_millis(500),
            run_on_change: true,
        }
    }
}
