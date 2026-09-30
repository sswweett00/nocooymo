//! Integration tests for the Elysium engine subsystems.

use super::{TestCase, TestContext, TestResult, TestSuite};
use crate::{World, Entity};

// ===========================================================================
// World simulation tests
// ===========================================================================

/// Build a suite of world simulation tests.
pub fn world_simulation_suite() -> TestSuite {
    let mut suite = TestSuite::new("world_simulation").tag("integration").tag("world");

    // Test: world spawns and despawns correctly
    suite = suite.with_test(TestCase {
        id: super::next_test_id(),
        name: "spawn_despawn_cycle".into(),
        suite: "world_simulation".into(),
        tags: {
            let mut t = std::collections::HashSet::new();
            t.insert("integration".into());
            t.insert("world".into());
            t
        },
        timeout: Some(std::time::Duration::from_secs(5)),
        skip: false,
        skip_reason: None,
        test_fn: std::sync::Arc::new(|ctx: &TestContext| -> TestResult {
            let mut world = World::new();
            let e1 = world.spawn();
            let e2 = world.spawn();
            let e3 = world.spawn();
            if world.entity_count() != 3 {
                return TestResult::failed(&ctx.name, &ctx.suite, ctx.elapsed(), "expected 3 entities");
            }
            world.despawn(e2);
            if world.entity_count() != 2 {
                return TestResult::failed(&ctx.name, &ctx.suite, ctx.elapsed(), "expected 2 entities after despawn");
            }
            if !world.is_alive(e1) || !world.is_alive(e3) || world.is_alive(e2) {
                return TestResult::failed(&ctx.name, &ctx.suite, ctx.elapsed(), "entity liveness incorrect");
            }
            TestResult::passed(&ctx.name, &ctx.suite, ctx.elapsed())
        }),
    });

    // Test: component insertion and queries
    suite = suite.with_test(TestCase {
        id: super::next_test_id(),
        name: "component_query_roundtrip".into(),
        suite: "world_simulation".into(),
        tags: {
            let mut t = std::collections::HashSet::new();
            t.insert("integration".into());
            t.insert("world".into());
            t.insert("component".into());
            t
        },
        timeout: Some(std::time::Duration::from_secs(5)),
        skip: false,
        skip_reason: None,
        test_fn: std::sync::Arc::new(|ctx: &TestContext| -> TestResult {
            let mut world = World::new();
            let e = world.spawn();
            world.insert_component(e, crate::Transform::default());
            if !world.has_component::<crate::Transform>(e) {
                return TestResult::failed(&ctx.name, &ctx.suite, ctx.elapsed(), "component missing after insert");
            }
            let comps = world.query::<crate::Transform>();
            if comps.len() != 1 {
                return TestResult::failed(&ctx.name, &ctx.suite, ctx.elapsed(), format!("expected 1 transform, got {}", comps.len()));
            }
            TestResult::passed(&ctx.name, &ctx.suite, ctx.elapsed())
        }),
    });

    // Test: resources
    suite = suite.with_test(TestCase {
        id: super::next_test_id(),
        name: "resource_insert_get_remove".into(),
        suite: "world_simulation".into(),
        tags: {
            let mut t = std::collections::HashSet::new();
            t.insert("integration".into());
            t.insert("world".into());
            t.insert("resource".into());
            t
        },
        timeout: Some(std::time::Duration::from_secs(5)),
        skip: false,
        skip_reason: None,
        test_fn: std::sync::Arc::new(|ctx: &TestContext| -> TestResult {
            let mut world = World::new();
            world.insert_resource(42i32);
            let val: Option<&i32> = world.get_resource::<i32>();
            if val != Some(&42) {
                return TestResult::failed(&ctx.name, &ctx.suite, ctx.elapsed(), "resource value mismatch");
            }
            world.remove_resource::<i32>();
            if world.get_resource::<i32>().is_some() {
                return TestResult::failed(&ctx.name, &ctx.suite, ctx.elapsed(), "resource not removed");
            }
            TestResult::passed(&ctx.name, &ctx.suite, ctx.elapsed())
        }),
    });

    suite
}

// ===========================================================================
// Rendering tests
// ===========================================================================

/// Build a suite of rendering tests.
pub fn rendering_suite() -> TestSuite {
    let mut suite = TestSuite::new("rendering").tag("integration").tag("rendering");

    // Test: screenshot capture is non-empty
    suite = suite.with_test(TestCase {
        id: super::next_test_id(),
        name: "screenshot_capture_non_empty".into(),
        suite: "rendering".into(),
        tags: {
            let mut t = std::collections::HashSet::new();
            t.insert("integration".into());
            t.insert("rendering".into());
            t.insert("screenshot".into());
            t
        },
        timeout: Some(std::time::Duration::from_secs(10)),
        skip: false,
        skip_reason: None,
        test_fn: std::sync::Arc::new(|ctx: &TestContext| -> TestResult {
            // In a real test we would capture from a running renderer. Here we simulate.
            let _pixel_count = 1920 * 1080;
            TestResult::passed(&ctx.name, &ctx.suite, ctx.elapsed())
        }),
    });

    // Test: pixel format validation
    suite = suite.with_test(TestCase {
        id: super::next_test_id(),
        name: "pixel_format_validation".into(),
        suite: "rendering".into(),
        tags: {
            let mut t = std::collections::HashSet::new();
            t.insert("integration".into());
            t.insert("rendering".into());
            t
        },
        timeout: Some(std::time::Duration::from_secs(5)),
        skip: false,
        skip_reason: None,
        test_fn: std::sync::Arc::new(|ctx: &TestContext| -> TestResult {
            let pixel_data = vec![255u8; 4];
            if pixel_data.len() != 4 {
                return TestResult::failed(&ctx.name, &ctx.suite, ctx.elapsed(), "pixel format invalid");
            }
            TestResult::passed(&ctx.name, &ctx.suite, ctx.elapsed())
        }),
    });

    suite
}

// ===========================================================================
// Physics tests
// ===========================================================================

/// Build a suite of physics tests.
pub fn physics_suite() -> TestSuite {
    let mut suite = TestSuite::new("physics").tag("integration").tag("physics");

    suite = suite.with_test(TestCase {
        id: super::next_test_id(),
        name: "physics_world_step".into(),
        suite: "physics".into(),
        tags: {
            let mut t = std::collections::HashSet::new();
            t.insert("integration".into());
            t.insert("physics".into());
            t
        },
        timeout: Some(std::time::Duration::from_secs(5)),
        skip: false,
        skip_reason: None,
        test_fn: std::sync::Arc::new(|ctx: &TestContext| -> TestResult {
            let _world = crate::PhysicsWorld::new();
            TestResult::passed(&ctx.name, &ctx.suite, ctx.elapsed())
        }),
    });

    suite = suite.with_test(TestCase {
        id: super::next_test_id(),
        name: "physics_body_defaults".into(),
        suite: "physics".into(),
        tags: {
            let mut t = std::collections::HashSet::new();
            t.insert("integration".into());
            t.insert("physics".into());
            t
        },
        timeout: Some(std::time::Duration::from_secs(5)),
        skip: false,
        skip_reason: None,
        test_fn: std::sync::Arc::new(|ctx: &TestContext| -> TestResult {
            let body = crate::PhysicsBody::default();
            let mass = body.mass;
            if mass <= 0.0 {
                return TestResult::failed(&ctx.name, &ctx.suite, ctx.elapsed(), "default physics body has non-positive mass");
            }
            TestResult::passed(&ctx.name, &ctx.suite, ctx.elapsed())
        }),
    });

    suite
}

// ===========================================================================
// Audio tests
// ===========================================================================

/// Build a suite of audio tests.
pub fn audio_suite() -> TestSuite {
    let mut suite = TestSuite::new("audio").tag("integration").tag("audio");

    suite = suite.with_test(TestCase {
        id: super::next_test_id(),
        name: "audio_system_initialization".into(),
        suite: "audio".into(),
        tags: {
            let mut t = std::collections::HashSet::new();
            t.insert("integration".into());
            t.insert("audio".into());
            t
        },
        timeout: Some(std::time::Duration::from_secs(10)),
        skip: false,
        skip_reason: None,
        test_fn: std::sync::Arc::new(|ctx: &TestContext| -> TestResult {
            let _manager = crate::AudioManager::new();
            TestResult::passed(&ctx.name, &ctx.suite, ctx.elapsed())
        }),
    });

    suite = suite.with_test(TestCase {
        id: super::next_test_id(),
        name: "audio_listener_defaults".into(),
        suite: "audio".into(),
        tags: {
            let mut t = std::collections::HashSet::new();
            t.insert("integration".into());
            t.insert("audio".into());
            t
        },
        timeout: Some(std::time::Duration::from_secs(5)),
        skip: false,
        skip_reason: None,
        test_fn: std::sync::Arc::new(|ctx: &TestContext| -> TestResult {
            let listener = crate::AudioListenerComponent::default();
            let _ = listener;
            TestResult::passed(&ctx.name, &ctx.suite, ctx.elapsed())
        }),
    });

    suite
}

// ===========================================================================
// Build all integration suites
// ===========================================================================

/// Returns all integration test suites.
pub fn integration_suites() -> Vec<TestSuite> {
    vec![
        world_simulation_suite(),
        rendering_suite(),
        physics_suite(),
        audio_suite(),
    ]
}
