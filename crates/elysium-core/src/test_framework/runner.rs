//! Test runner — executes registered tests and collects results.

use super::{
    TestCase, TestConfig, TestContext, TestResult, TestStatus, TestSuite,
    discovery::DiscoveryResult,
    suite::run_test_case,
};
use rayon::prelude::*;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

// ===========================================================================
// Execution summary
// ===========================================================================

#[derive(Debug, Clone, Default)]
pub struct RunSummary {
    pub total: usize,
    pub passed: usize,
    pub failed: usize,
    pub panicked: usize,
    pub skipped: usize,
    pub timeout: usize,
    pub duration: Duration,
    pub results: Vec<TestResult>,
    pub by_suite: HashMap<String, Vec<TestResult>>,
}

impl RunSummary {
    pub fn success_rate(&self) -> f64 {
        if self.total == 0 {
            return 0.0;
        }
        self.passed as f64 / self.total as f64
    }

    pub fn print_summary(&self) {
        eprintln!("\nTest Summary\n============");
        eprintln!("  Total:  {}", self.total);
        eprintln!("  Passed: {}", self.passed);
        eprintln!("  Failed: {}", self.failed);
        eprintln!("  Panics: {}", self.panicked);
        eprintln!("  Skip:   {}", self.skipped);
        eprintln!("  Time:   {:.2}s", self.duration.as_secs_f64());
    }
}

// ===========================================================================
// Single test execution
// ===========================================================================

fn execute_test(test: &TestCase, config: Arc<TestConfig>) -> TestResult {
    let ctx = TestContext {
        name: test.name.clone(),
        suite: test.suite.clone(),
        tags: test.tags.clone(),
        start_time: Instant::now(),
        config: config.clone(),
        captured_logs: Vec::new(),
        screenshots: Vec::new(),
        metadata: HashMap::new(),
    };

    // Timeout wrapper
    if let Some(timeout) = test.timeout {
        // Simple single-threaded timeout simulation
        let result = run_test_case(test, &ctx);
        if result.duration > timeout && result.status == TestStatus::Passed {
            TestResult {
                name: result.name,
                suite: result.suite,
                status: TestStatus::Timeout,
                duration: result.duration,
                message: Some(format!("exceeded timeout of {:?}", timeout)),
                backtrace: result.backtrace,
                timestamp: result.timestamp,
            }
        } else {
            result
        }
    } else {
        run_test_case(test, &ctx)
    }
}

// ===========================================================================
// Runner
// ===========================================================================

/// Execute tests and return the summary.
pub fn run_discovery(discovery: DiscoveryResult, config: Arc<TestConfig>) -> RunSummary {
    let start = Instant::now();
    let tests: Vec<TestCase> = discovery.tests.into_iter().collect();

    let results: Arc<Mutex<Vec<TestResult>>> = Arc::new(Mutex::new(Vec::with_capacity(tests.len())));

    if config.parallel {
        let max_parallel = config.max_parallel.unwrap_or_else(|| std::thread::available_parallelism().map(|p| p.get()).unwrap_or(4));
        tests.par_iter().for_each(|test| {
            let result = execute_test(test, config.clone());
            results.lock().unwrap().push(result);
        });
    } else {
        for test in &tests {
            let result = execute_test(test, config.clone());
            results.lock().unwrap().push(result);
        }
    }

    let results = results.lock().unwrap().clone();
    let mut summary = RunSummary {
        total: results.len(),
        duration: start.elapsed(),
        ..Default::default()
    };

    for result in &results {
        match result.status {
            TestStatus::Passed => summary.passed += 1,
            TestStatus::Failed => summary.failed += 1,
            TestStatus::Panicked => summary.panicked += 1,
            TestStatus::Skipped => summary.skipped += 1,
            TestStatus::Timeout => summary.timeout += 1,
        }
        summary.by_suite.entry(result.suite.clone()).or_default().push(result.clone());
    }

    summary.results = results;
    summary
}

/// Execute all registered tests.
pub fn run_all(config: Arc<TestConfig>) -> RunSummary {
    let discovery = super::discovery::discover_all();
    run_discovery(discovery, config)
}

/// Execute tests for a specific suite.
pub fn run_suite(suite: &str, config: Arc<TestConfig>) -> RunSummary {
    let discovery = super::discovery::discover_suite(suite);
    run_discovery(discovery, config)
}
