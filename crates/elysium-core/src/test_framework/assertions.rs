//! Test assertion helpers with helpful failure messages.

use super::{TestContext, TestResult};
use crate::{EngineError, EngineResult};
use std::fmt::Debug;

// ===========================================================================
// Core assertion helpers
// ===========================================================================

/// Panic the current test with a message.
pub fn panic(msg: impl Into<String>) -> ! {
    panic!("{}", msg.into());
}

/// Return a failed test result.
pub fn fail(ctx: &TestContext, msg: impl Into<String>) -> TestResult {
    TestResult::failed(&ctx.name, &ctx.suite, ctx.elapsed(), msg)
}

/// Return a failed test result with a formatted message.
#[macro_export]
macro_rules! fail {
    ($ctx:expr, $($arg:tt)*) => {
        return $crate::test_framework::TestResult::failed(
            &$ctx.name,
            &$ctx.suite,
            $ctx.elapsed(),
            std::format!($($arg)*),
        );
    };
}

// ===========================================================================
// Equality assertions
// ===========================================================================

/// Assert that two values are equal.
pub fn assert_eq<T: PartialEq + Debug>(
    ctx: &TestContext,
    left: T,
    right: T,
) -> Result<(), TestResult> {
    if left != right {
        return Err(TestResult::failed(
            &ctx.name,
            &ctx.suite,
            ctx.elapsed(),
            format!("assertion failed: {:?} == {:?}", left, right),
        ));
    }
    Ok(())
}

/// Assert that two values are approximately equal within epsilon.
pub fn assert_approx<T: PartialOrd + Copy + Debug>(
    ctx: &TestContext,
    left: T,
    right: T,
    epsilon: T,
) -> Result<(), TestResult>
where
    T: std::ops::Sub<Output = T> + Into<f64>,
{
    let diff: f64 = (left - right).into();
    let eps: f64 = epsilon.into();
    let tol = diff.abs();
    if tol > eps {
        return Err(TestResult::failed(
            &ctx.name,
            &ctx.suite,
            ctx.elapsed(),
            format!("assertion failed: {:?} approx {:?} (diff={:?}, epsilon={:?})", left, right, diff, epsilon),
        ));
    }
    Ok(())
}

/// Assert that a condition is true.
pub fn assert_true(
    ctx: &TestContext,
    cond: bool,
    msg: impl Into<String>,
) -> Result<(), TestResult> {
    if !cond {
        return Err(TestResult::failed(
            &ctx.name,
            &ctx.suite,
            ctx.elapsed(),
            msg.into(),
        ));
    }
    Ok(())
}

/// Assert that a condition is false.
pub fn assert_false(
    ctx: &TestContext,
    cond: bool,
    msg: impl Into<String>,
) -> Result<(), TestResult> {
    assert_true(ctx, !cond, msg)
}

// ===========================================================================
// Result / error assertions
// ===========================================================================

/// Assert that an [`EngineResult`] is `Ok`.
pub fn assert_ok<T>(
    ctx: &TestContext,
    result: &EngineResult<T>,
) -> Result<&T, TestResult> {
    match result {
        EngineResult::Ok(value) => Ok(value),
        EngineResult::Err(err) => Err(TestResult::failed(
            &ctx.name,
            &ctx.suite,
            ctx.elapsed(),
            format!("assertion failed: result is Err({:?})", err),
        )),
    }
}

/// Assert that an [`EngineResult`] is `Err`.
pub fn assert_err<E>(
    ctx: &TestContext,
    result: &EngineResult<E>,
) -> Result<(), TestResult> {
    match result {
        EngineResult::Err(_) => Ok(()),
        EngineResult::Ok(_) => Err(TestResult::failed(
            &ctx.name,
            &ctx.suite,
            ctx.elapsed(),
            "assertion failed: result is Ok, expected Err",
        )),
    }
}

/// Assert that an [`EngineResult`] is `Err` with the expected message.
pub fn assert_err_msg<E>(
    ctx: &TestContext,
    result: &EngineResult<E>,
    expected_msg: &str,
) -> Result<(), TestResult> {
    match result {
        EngineResult::Err(err) => {
            let msg = std::format!("{:?}", err);
            if msg.contains(expected_msg) {
                Ok(())
            } else {
                Err(TestResult::failed(
                    &ctx.name,
                    &ctx.suite,
                    ctx.elapsed(),
                    format!("assertion failed: error message {:?} does not contain {:?}", msg, expected_msg),
                ))
            }
        }
        EngineResult::Ok(_) => Err(TestResult::failed(
            &ctx.name,
            &ctx.suite,
            ctx.elapsed(),
            "assertion failed: result is Ok, expected Err",
        )),
    }
}

// ===========================================================================
// Panic / exception assertions
// ===========================================================================

/// Assert that a closure panics.
pub fn assert_panics<F: FnOnce() -> R, R>(ctx: &TestContext, f: F) -> Result<String, TestResult> {
    use std::panic::{self, AssertUnwindSafe, catch_unwind};
    match catch_unwind(AssertUnwindSafe(f)) {
        Ok(_) => Err(TestResult::failed(
            &ctx.name,
            &ctx.suite,
            ctx.elapsed(),
            "assertion failed: closure did not panic",
        )),
        Err(payload) => {
            let msg = if let Some(s) = payload.downcast_ref::<&'static str>() {
                (*s).to_string()
            } else if let Some(s) = payload.downcast_ref::<String>() {
                s.clone()
            } else {
                "unknown panic".to_string()
            };
            Ok(msg)
        }
    }
}

/// Assert that a closure panics with a message containing the expected substring.
pub fn assert_panics_with<F: FnOnce() -> R, R>(
    ctx: &TestContext,
    f: F,
    expected_substr: &str,
) -> Result<(), TestResult> {
    let msg = assert_panics(ctx, f)?;
    if msg.contains(expected_substr) {
        Ok(())
    } else {
        Err(TestResult::failed(
            &ctx.name,
            &ctx.suite,
            ctx.elapsed(),
            format!("assertion failed: panic message {:?} does not contain {:?}", msg, expected_substr),
        ))
    }
}

// ===========================================================================
// Collection assertions
// ===========================================================================

/// Assert that a collection contains a value.
pub fn assert_contains<T: PartialEq + Debug>(
    ctx: &TestContext,
    collection: &[T],
    value: T,
) -> Result<(), TestResult> {
    if !collection.contains(&value) {
        return Err(TestResult::failed(
            &ctx.name,
            &ctx.suite,
            ctx.elapsed(),
            format!("assertion failed: collection does not contain {:?}", value),
        ));
    }
    Ok(())
}

/// Assert that a collection has the expected length.
pub fn assert_len<T>(
    ctx: &TestContext,
    collection: &[T],
    expected: usize,
) -> Result<(), TestResult> {
    if collection.len() != expected {
        return Err(TestResult::failed(
            &ctx.name,
            &ctx.suite,
            ctx.elapsed(),
            format!("assertion failed: length {} != {}", collection.len(), expected),
        ));
    }
    Ok(())
}

/// Assert that a collection is empty.
pub fn assert_empty<T: Debug>(
    ctx: &TestContext,
    collection: &[T],
) -> Result<(), TestResult> {
    assert_len(ctx, collection, 0)
}

// ===========================================================================
// Float-specific assertions
// ===========================================================================

/// Assert that a float is approximately equal.
pub fn assert_float_eq(
    ctx: &TestContext,
    left: f32,
    right: f32,
    epsilon: f32,
) -> Result<(), TestResult> {
    assert_approx(ctx, left, right, epsilon)
}

/// Assert NaN.
pub fn assert_nan(ctx: &TestContext, value: f32) -> Result<(), TestResult> {
    assert_true(ctx, value.is_nan(), format!("expected NaN, got {}", value))
}

/// Assert infinity.
pub fn assert_infinite(ctx: &TestContext, value: f32, positive: bool) -> Result<(), TestResult> {
    let cond = if positive { value.is_infinite() && value.is_sign_positive() } else { value.is_infinite() && value.is_sign_negative() };
    assert_true(ctx, cond, format!("expected {}infinity, got {}", if positive { "+" } else { "-" }, value))
}

// ===========================================================================
// Option / result helpers
// ===========================================================================

/// Assert that an Option is Some.
pub fn assert_some<T: Debug>(
    ctx: &TestContext,
    opt: Option<T>,
    label: &str,
) -> Result<T, TestResult> {
    match opt {
        Some(v) => Ok(v),
        None => Err(TestResult::failed(
            &ctx.name,
            &ctx.suite,
            ctx.elapsed(),
            format!("assertion failed: {} is None", label),
        )),
    }
}

/// Assert that an Option is None.
pub fn assert_none<T: Debug>(
    ctx: &TestContext,
    opt: Option<T>,
    label: &str,
) -> Result<(), TestResult> {
    match opt {
        Some(_) => Err(TestResult::failed(
            &ctx.name,
            &ctx.suite,
            ctx.elapsed(),
            format!("assertion failed: {} is Some, expected None", label),
        )),
        None => Ok(()),
    }
}
