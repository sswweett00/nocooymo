//! Test fixtures — reusable setup/teardown patterns for tests.

use super::{TestContext, TestResult};
use std::any::Any;
use std::sync::{Arc, RwLock};

// ===========================================================================
// Fixture types
// ===========================================================================

/// A fixture value with optional setup and teardown.
pub struct Fixture<T> {
    pub value: T,
    setup_done: bool,
    teardown_done: bool,
}

impl<T> Fixture<T> {
    pub fn new(value: T) -> Self {
        Self { value, setup_done: false, teardown_done: false }
    }

    pub fn setup(&mut self, _ctx: &TestContext) -> Result<(), String> {
        self.setup_done = true;
        Ok(())
    }

    pub fn teardown(&mut self, _ctx: &TestContext) -> Result<(), String> {
        self.teardown_done = true;
        Ok(())
    }
}

// ===========================================================================
// Fixture registry
// ===========================================================================

#[derive(Debug, Default)]
pub struct FixtureRegistry {
    fixtures: RwLock<HashMap<String, Box<dyn Any + Send + Sync>>>,
}

impl FixtureRegistry {
    pub fn global() -> &'static Self {
        static REGISTRY: std::sync::OnceLock<FixtureRegistry> = std::sync::OnceLock::new();
        REGISTRY.get_or_init(FixtureRegistry::default)
    }

    pub fn insert<T: Any + Send + Sync>(&self, name: impl Into<String>, fixture: T) {
        self.fixtures.write().unwrap().insert(name.into(), Box::new(fixture));
    }

    pub fn get<T: Any + Send + Sync>(&self, name: &str) -> Option<&T> {
        self.fixtures.read().unwrap().get(name)?.downcast_ref::<T>()
    }

    pub fn get_mut<T: Any + Send + Sync>(&self, name: &str) -> Option<&mut T> {
        self.fixtures.write().unwrap().get_mut(name)?.downcast_mut::<T>()
    }

    pub fn remove(&self, name: &str) -> Option<Box<dyn Any + Send + Sync>> {
        self.fixtures.write().unwrap().remove(name)
    }

    pub fn clear(&self) {
        self.fixtures.write().unwrap().clear();
    }
}

// ===========================================================================
// Test fixture context
// ===========================================================================

/// Provides access to commonly-used fixtures within a test.
#[derive(Debug, Clone)]
pub struct TestFixtureContext {
    pub world: crate::World,
    pub entities: Vec<crate::Entity>,
}

impl Default for TestFixtureContext {
    fn default() -> Self {
        Self {
            world: crate::World::new(),
            entities: Vec::new(),
        }
    }
}

impl TestFixtureContext {
    pub fn with_world(world: crate::World) -> Self {
        Self { world, entities: Vec::new() }
    }

    pub fn spawn_entity(&mut self) -> crate::Entity {
        let entity = self.world.spawn();
        self.entities.push(entity);
        entity
    }

    pub fn with_entities(mut self, count: usize) -> Self {
        for _ in 0..count {
            self.spawn_entity();
        }
        self
    }
}

// ===========================================================================
// Fixture lifecycle helpers
// ===========================================================================

/// Set up a fresh world fixture for each test.
pub fn world_fixture() -> TestFixtureContext {
    TestFixtureContext {
        world: crate::World::new(),
        entities: Vec::new(),
    }
}

/// Set up a world with a specific capacity.
pub fn world_fixture_with_capacity(capacity: usize) -> TestFixtureContext {
    TestFixtureContext {
        world: crate::World::with_capacity(capacity),
        entities: Vec::new(),
    }
}
