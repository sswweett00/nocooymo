//! Elysium Testing & Automation Framework
//!
//! Provides a complete testing framework including:
//! - Unit test infrastructure (discovery, fixtures, assertions, suites, parameters)
//! - Integration tests (world, rendering, physics, audio)
//! - Performance tests (benchmarks, frame timing, memory, throughput)
//! - Automation (test runner, filtering, parallel execution, reporting, CI/CD)
//! - Game testing (input playback/recording, screen capture, game state assertions, test scenes)

use std::any::Any;
use std::collections::{HashMap, HashSet};
use std::fmt::{self, Write as _};
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, OnceLock, RwLock};
use std::time::{Duration, Instant};



// ===========================================================================
// Public module declarations
// ===========================================================================

pub mod assertions;
pub mod fixtures;
pub mod discovery;
pub mod suite;
pub mod parametrize;
pub mod integration;
pub mod performance;
pub mod runner;
pub mod reporting;
pub mod automation;
pub mod game_testing;

pub use assertions::*;
pub use fixtures::*;
pub use discovery::*;
pub use suite::*;
pub use parametrize::*;
pub use integration::*;
pub use performance::*;
pub use runner::*;
pub use reporting::*;
pub use automation::*;
pub use game_testing::*;

// ===========================================================================
// Result type for the framework
// ===========================================================================

/// Result of a single test execution.
#[derive(Debug, Clone)]
pub struct TestResult {
    pub name: String,
    pub suite: String,
    pub status: TestStatus,
    pub duration: Duration,
    pub message: Option<String>,
    pub backtrace: Option<String>,
    pub timestamp: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TestStatus {
    Passed,
    Failed,
    Skipped,
    Panicked,
    Timeout,
}

impl fmt::Display for TestStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TestStatus::Passed => write!(f, "PASSED"),
            TestStatus::Failed => write!(f, "FAILED"),
            TestStatus::Skipped => write!(f, "SKIPPED"),
            TestStatus::Panicked => write!(f, "PANICKED"),
            TestStatus::Timeout => write!(f, "TIMEOUT"),
        }
    }
}

impl TestResult {
    pub fn passed(name: impl Into<String>, suite: impl Into<String>, duration: Duration) -> Self {
        Self {
            name: name.into(),
            suite: suite.into(),
            status: TestStatus::Passed,
            duration,
            message: None,
            backtrace: None,
            timestamp: now_nanos(),
        }
    }

    pub fn failed(
        name: impl Into<String>,
        suite: impl Into<String>,
        duration: Duration,
        message: impl Into<String>,
    ) -> Self {
        Self {
            name: name.into(),
            suite: suite.into(),
            status: TestStatus::Failed,
            duration,
            message: Some(message.into()),
            backtrace: None,
            timestamp: now_nanos(),
        }
    }

    pub fn panicked(
        name: impl Into<String>,
        suite: impl Into<String>,
        duration: Duration,
        message: impl Into<String>,
        backtrace: impl Into<String>,
    ) -> Self {
        Self {
            name: name.into(),
            suite: suite.into(),
            status: TestStatus::Panicked,
            duration,
            message: Some(message.into()),
            backtrace: Some(backtrace.into()),
            timestamp: now_nanos(),
        }
    }

    pub fn skipped(name: impl Into<String>, suite: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            suite: suite.into(),
            status: TestStatus::Skipped,
            duration: Duration::ZERO,
            message: Some(message.into()),
            backtrace: None,
            timestamp: now_nanos(),
        }
    }

    pub fn is_success(&self) -> bool {
        matches!(self.status, TestStatus::Passed)
    }

    pub fn is_failure(&self) -> bool {
        !matches!(self.status, TestStatus::Passed | TestStatus::Skipped)
    }
}

// ===========================================================================
// Test context and configuration
// ===========================================================================

/// Global test configuration.
#[derive(Debug, Clone)]
pub struct TestConfig {
    pub parallel: bool,
    pub max_parallel: Option<usize>,
    pub timeout: Duration,
    pub fail_fast: bool,
    pub filter: Option<String>,
    pub tags: Vec<String>,
    pub exclude_tags: Vec<String>,
    pub output_dir: PathBuf,
    pub report_formats: Vec<ReportFormat>,
    pub capture_logs: bool,
    pub capture_screenshots: bool,
    pub seed: Option<u64>,
}

impl Default for TestConfig {
    fn default() -> Self {
        Self {
            parallel: true,
            max_parallel: None,
            timeout: Duration::from_secs(60),
            fail_fast: false,
            filter: None,
            tags: Vec::new(),
            exclude_tags: Vec::new(),
            output_dir: PathBuf::from("test_output"),
            report_formats: vec![ReportFormat::Text, ReportFormat::JUnit],
            capture_logs: true,
            capture_screenshots: false,
            seed: None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReportFormat {
    Text,
    JUnit,
    Html,
    Json,
    Csv,
}

/// Per-test execution context.
#[derive(Debug)]
pub struct TestContext {
    pub name: String,
    pub suite: String,
    pub tags: HashSet<String>,
    pub start_time: Instant,
    pub config: Arc<TestConfig>,
    pub captured_logs: Vec<LogEntry>,
    pub screenshots: Vec<Screenshot>,
    pub metadata: HashMap<String, Box<dyn Any + Send + Sync>>,
}

#[derive(Debug, Clone)]
pub struct LogEntry {
    pub level: LogLevel,
    pub target: String,
    pub message: String,
    pub timestamp: u64,
}

#[derive(Debug, Clone)]
pub struct Screenshot {
    pub name: String,
    pub data: Vec<u8>,
    pub width: u32,
    pub height: u32,
    pub format: ScreenshotFormat,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScreenshotFormat {
    Png,
    Jpeg,
    Bmp,
    Raw,
}

impl TestContext {
    /// Elapsed time since the test started.
    pub fn elapsed(&self) -> Duration {
        self.start_time.elapsed()
    }
}

// ===========================================================================
// Test registry and discovery
// ===========================================================================

/// A registered test case.
#[derive(Debug, Clone)]
pub struct TestCase {
    pub id: u64,
    pub name: String,
    pub suite: String,
    pub tags: HashSet<String>,
    pub timeout: Option<Duration>,
    pub skip: bool,
    pub skip_reason: Option<String>,
    pub test_fn: Arc<dyn Fn(&TestContext) -> TestResult + Send + Sync>,
}

impl Hash for TestCase {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.id.hash(state);
        self.name.hash(state);
        self.suite.hash(state);
    }
}

impl PartialEq for TestCase {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
            && self.name == other.name
            && self.suite == other.suite
    }
}

impl Eq for TestCase {}

static TEST_ID_COUNTER: AtomicU64 = AtomicU64::new(0);

fn next_test_id() -> u64 {
    TEST_ID_COUNTER.fetch_add(1, Ordering::Relaxed)
}

/// Global test registry.
#[derive(Debug, Default)]
pub struct TestRegistry {
    tests: RwLock<Vec<TestCase>>,
    suites: RwLock<HashMap<String, Vec<u64>>>,
    tags: RwLock<HashMap<String, HashSet<u64>>>,
}

impl TestRegistry {
    pub fn global() -> &'static Self {
        static REGISTRY: OnceLock<TestRegistry> = OnceLock::new();
        REGISTRY.get_or_init(TestRegistry::default)
    }

    pub fn register(&self, test: TestCase) {
        let id = test.id;
        let suite = test.suite.clone();
        let tag_set: HashSet<String> = test.tags.clone();
        self.tests.write().unwrap().push(test);
        self.suites.write().unwrap().entry(suite).or_default().push(id);
        for tag in &tag_set {
            self.tags.write().unwrap().entry(tag.clone()).or_default().insert(id);
        }
    }

    pub fn all(&self) -> Vec<TestCase> {
        self.tests.read().unwrap().clone()
    }

    pub fn by_suite(&self, suite: &str) -> Vec<TestCase> {
        let ids = self.suites.read().unwrap().get(suite).cloned().unwrap_or_default();
        let tests = self.tests.read().unwrap();
        ids.iter()
            .filter_map(|id| tests.iter().find(|t| t.id == *id).cloned())
            .collect()
    }

    pub fn by_tag(&self, tag: &str) -> Vec<TestCase> {
        let ids = self.tags.read().unwrap().get(tag).cloned().unwrap_or_default();
        let tests = self.tests.read().unwrap();
        ids.iter()
            .filter_map(|id| tests.iter().find(|t| t.id == *id).cloned())
            .collect()
    }

    pub fn filter(&self, predicate: impl Fn(&TestCase) -> bool) -> Vec<TestCase> {
        self.tests.read().unwrap().iter().filter(|t| predicate(t)).cloned().collect()
    }

    pub fn len(&self) -> usize {
        self.tests.read().unwrap().len()
    }

    pub fn is_empty(&self) -> bool {
        self.tests.read().unwrap().is_empty()
    }
}

// ===========================================================================
// Test registration macros
// ===========================================================================

/// Register a unit test in the global test registry.
///
/// # Example
/// ```no_run
/// elysium_test!(suite_name, test_fn_name, |ctx| {
///     assert_eq!(2 + 2, 4);
///     TestResult::passed("test_fn_name", "suite_name", ctx.elapsed())
/// });
/// ```
#[macro_export]
macro_rules! elysium_test {
    ($suite:expr, $name:expr, $body:expr) => {{
        let id = $crate::test_framework::next_test_id();
        let test = $crate::test_framework::TestCase {
            id,
            name: $name.to_string(),
            suite: $suite.to_string(),
            tags: ::std::collections::HashSet::new(),
            timeout: None,
            skip: false,
            skip_reason: None,
            test_fn: ::std::sync::Arc::new(move |ctx: &$crate::test_framework::TestContext| -> $crate::test_framework::TestResult {
                $body(ctx)
            }),
        };
        $crate::test_framework::TestRegistry::global().register(test);
    }};
}

/// Register a parametrized test. Each tuple in the list becomes one test case.
///
/// # Example
/// ```no_run
/// elysium_test_param!("math", "addition", [(1, 2, 3), (0, 0, 0), (-1, 1, 0)], |(a, b, expected), ctx| {
///     assert_eq!(a + b, expected);
///     TestResult::passed(format!("{} + {} = {}", a, b, expected), "math", ctx.elapsed())
/// });
/// ```
#[macro_export]
macro_rules! elysium_test_param {
    ($suite:expr, $name:expr, $params:expr, $body:expr) => {{
        for (idx, param) in $params.iter().enumerate() {
            let case_name = format!("{} [{}]", $name, idx);
            let id = $crate::test_framework::next_test_id();
            let param = *param;
            let test = $crate::test_framework::TestCase {
                id,
                name: case_name.clone(),
                suite: $suite.to_string(),
                tags: ::std::collections::HashSet::new(),
                timeout: None,
                skip: false,
                skip_reason: None,
                test_fn: ::std::sync::Arc::new(move |ctx: &$crate::test_framework::TestContext| -> $crate::test_framework::TestResult {
                    $body(param, ctx)
                }),
            };
            $crate::test_framework::TestRegistry::global().register(test);
        }
    }};
}

/// Register a test that should be skipped.
#[macro_export]
macro_rules! elysium_test_skip {
    ($suite:expr, $name:expr, $reason:expr) => {{
        let id = $crate::test_framework::next_test_id();
        let test = $crate::test_framework::TestCase {
            id,
            name: $name.to_string(),
            suite: $suite.to_string(),
            tags: ::std::collections::HashSet::new(),
            timeout: None,
            skip: true,
            skip_reason: Some($reason.to_string()),
            test_fn: ::std::sync::Arc::new(|_ctx| -> $crate::test_framework::TestResult {
                $crate::test_framework::TestResult::skipped($name, $suite, $reason)
            }),
        };
        $crate::test_framework::TestRegistry::global().register(test);
    }};
}

/// Register a test with a timeout.
#[macro_export]
macro_rules! elysium_test_timeout {
    ($suite:expr, $name:expr, $timeout:expr, $body:expr) => {{
        let id = $crate::test_framework::next_test_id();
        let timeout = ::std::time::Duration::from_secs($timeout);
        let test = $crate::test_framework::TestCase {
            id,
            name: $name.to_string(),
            suite: $suite.to_string(),
            tags: ::std::collections::HashSet::new(),
            timeout: Some(timeout),
            skip: false,
            skip_reason: None,
            test_fn: ::std::sync::Arc::new(move |ctx: &$crate::test_framework::TestContext| -> $crate::test_framework::TestResult {
                $body(ctx)
            }),
        };
        $crate::test_framework::TestRegistry::global().register(test);
    }};
}

// ===========================================================================
// Test suite builder
// ===========================================================================

/// Builder for creating test suites.
pub struct TestSuiteBuilder {
    suite_name: String,
    tags: HashSet<String>,
}

impl TestSuiteBuilder {
    pub fn new(suite_name: impl Into<String>) -> Self {
        Self {
            suite_name: suite_name.into(),
            tags: HashSet::new(),
        }
    }

    pub fn tag(mut self, tag: impl Into<String>) -> Self {
        self.tags.insert(tag.into());
        self
    }

    pub fn tags(mut self, tags: impl IntoIterator<Item = impl Into<String>>) -> Self {
        for tag in tags {
            self.tags.insert(tag.into());
        }
        self
    }
}

// ===========================================================================
// Helpers
// ===========================================================================

#[inline]
fn now_nanos() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0)
}

// ===========================================================================
// Assertion macros (forwarded from assertions module, plus extra helpers)
// ===========================================================================

#[macro_export]
macro_rules! test_assert {
    ($cond:expr) => {
        if !($cond) {
            return $crate::test_framework::TestResult::failed(
                std::format!("assertion failed: {}", stringify!($cond)),
                "unknown",
                std::time::Duration::ZERO,
                std::format!("{}", stringify!($cond)),
            );
        }
    };
}

#[macro_export]
macro_rules! test_assert_eq {
    ($left:expr, $right:expr) => {
        let left = $left;
        let right = $right;
        if left != right {
            return $crate::test_framework::TestResult::failed(
                "assertion failed: left == right",
                "unknown",
                std::time::Duration::ZERO,
                std::format!("{:?} != {:?}", left, right),
            );
        }
    };
}

#[macro_export]
macro_rules! test_assert_approx {
    ($left:expr, $right:expr, $epsilon:expr) => {
        let left = $left;
        let right = $right;
        let epsilon = $epsilon;
        if (left - right).abs() > epsilon {
            return $crate::test_framework::TestResult::failed(
                "assertion failed: approx",
                "unknown",
                std::time::Duration::ZERO,
                std::format!("{:?} != {:?} (epsilon={:?})", left, right, epsilon),
            );
        }
    };
}

// ===========================================================================
// Module docs
// ===========================================================================

#[cfg(test)]
mod framework_tests {
    use super::*;

    #[test]
    fn registry_registers_and_queries() {
        let registry = TestRegistry::global();
        elysium_test!("unit", "sample", |_ctx| TestResult::passed("sample", "unit", Duration::ZERO));

        assert!(registry.len() > 0);
        let suite_tests = registry.by_suite("unit");
        assert!(suite_tests.iter().any(|t| t.name == "sample"));
    }

    #[test]
    fn parametrized_macro_creates_cases() {
        let registry = TestRegistry::global();
        let before = registry.len();
        elysium_test_param!("param", "add", [(1, 2, 3)], |_p, _ctx| {
            TestResult::passed("add-0", "param", Duration::ZERO)
        });
        assert_eq!(registry.len(), before + 1);
    }
}
