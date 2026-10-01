//! Parameterized test support — run the same test with multiple input values.

use super::{TestCase, TestContext, TestResult};
use std::collections::HashSet;
use std::sync::Arc;

// ===========================================================================
// Parametrized test runner
// ===========================================================================

/// Wraps a parameterized test body.
pub struct ParametrizedTest<P> {
    pub name: String,
    pub suite: String,
    pub params: Vec<P>,
    pub body: Arc<dyn Fn(&P, &TestContext) -> TestResult + Send + Sync>,
    pub tags: Vec<String>,
    pub timeout: Option<std::time::Duration>,
}

impl<P: Send + Sync + 'static> ParametrizedTest<P> {
    pub fn new(name: impl Into<String>, suite: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            suite: suite.into(),
            params: Vec::new(),
            body: Arc::new(|_p, _ctx| TestResult::skipped("parametrized", "unknown", "no body")),
            tags: Vec::new(),
            timeout: None,
        }
    }

    pub fn with_body(mut self, body: impl Fn(&P, &TestContext) -> TestResult + Send + Sync + 'static) -> Self {
        self.body = Arc::new(body);
        self
    }

    pub fn with_params(mut self, params: Vec<P>) -> Self {
        self.params = params;
        self
    }

    pub fn with_param(mut self, param: P) -> Self {
        self.params.push(param);
        self
    }

    pub fn tag(mut self, tag: impl Into<String>) -> Self {
        self.tags.push(tag.into());
        self
    }

    pub fn timeout(mut self, duration: std::time::Duration) -> Self {
        self.timeout = Some(duration);
        self
    }

    /// Expand this parametrized test into individual test cases.
    pub fn expand(self) -> Vec<TestCase> {
        self.params
            .into_iter()
            .enumerate()
            .map(|(idx, param)| {
                let case_name = format!("{} [{}]", self.name, idx);
                let id = super::next_test_id();
                let body = self.body.clone();
                let mut tags = HashSet::new();
                for tag in &self.tags {
                    tags.insert(tag.clone());
                }
                tags.insert("parametrized".to_string());

                TestCase {
                    id,
                    name: case_name,
                    suite: self.suite.clone(),
                    tags,
                    timeout: self.timeout,
                    skip: false,
                    skip_reason: None,
                    test_fn: Arc::new(move |ctx: &TestContext| -> TestResult { body(&param, ctx) }),
                }
            })
            .collect()
    }
}

// ===========================================================================
// Macro to register parametrized tests inline
// ===========================================================================

#[macro_export]
macro_rules! elysium_parametrize {
    ($suite:expr, $name:expr, $params:expr, |$param:ident, $ctx:ident| $body:expr) => {{
        let id = $crate::test_framework::next_test_id();
        let case_name = $name.to_string();
        let params = $params;
        let test = $crate::test_framework::TestCase {
            id,
            name: case_name.clone(),
            suite: $suite.to_string(),
            tags: {
                let mut t = ::std::collections::HashSet::new();
                t.insert("parametrized".to_string());
                t
            },
            timeout: None,
            skip: false,
            skip_reason: None,
            test_fn: ::std::sync::Arc::new(move |$ctx: &$crate::test_framework::TestContext| -> $crate::test_framework::TestResult {
                // Expand to first param only; full expansion is via ParametrizedTest::expand
                let $param = &params[0];
                $body
            }),
        };
        $crate::test_framework::TestRegistry::global().register(test);
    }};
}
