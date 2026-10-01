//! Test suites — logical grouping of tests with shared setup/teardown.

use super::{TestCase, TestContext, TestResult};
use std::collections::HashSet;

// ===========================================================================
// Test suite definition
// ===========================================================================

#[derive(Debug, Clone)]
pub struct TestSuite {
    pub name: String,
    pub tests: Vec<TestCase>,
    pub tags: HashSet<String>,
    pub setup: Option<Arc<dyn Fn(&TestContext) -> Result<(), String> + Send + Sync>>,
    pub teardown: Option<Arc<dyn Fn(&TestContext) -> Result<(), String> + Send + Sync>>,
}

impl TestSuite {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            tests: Vec::new(),
            tags: HashSet::new(),
            setup: None,
            teardown: None,
        }
    }

    pub fn with_test(mut self, test: TestCase) -> Self {
        self.tests.push(test);
        self
    }

    pub fn with_tests(mut self, tests: impl IntoIterator<Item = TestCase>) -> Self {
        self.tests.extend(tests);
        self
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

    pub fn setup(mut self, setup: impl Fn(&TestContext) -> Result<(), String> + Send + Sync + 'static) -> Self {
        self.setup = Some(Arc::new(setup));
        self
    }

    pub fn teardown(mut self, teardown: impl Fn(&TestContext) -> Result<(), String> + Send + Sync + 'static) -> Self {
        self.teardown = Some(Arc::new(teardown));
        self
    }
}

// ===========================================================================
// Suite runner
// ===========================================================================

/// Run a single test case with the given context.
pub fn run_test_case(test: &TestCase, ctx: &TestContext) -> TestResult {
    let start = std::time::Instant::now();

    // Handle skipped tests
    if test.skip {
        let reason = test.skip_reason.clone().unwrap_or_else(|| "skipped".to_string());
        return TestResult::skipped(&test.name, &test.suite, reason);
    }

    // Timeout handling is done by the caller (runner)
    let result = (test.test_fn)(ctx);
    TestResult {
        name: result.name,
        suite: result.suite,
        status: result.status,
        duration: start.elapsed(),
        message: result.message,
        backtrace: result.backtrace,
        timestamp: super::now_nanos(),
    }
}
