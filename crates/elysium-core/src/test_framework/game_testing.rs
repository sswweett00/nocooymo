//! Game testing — input playback/recording, screen capture, game state assertions, test scenes.

use super::{TestContext, TestResult};
use crate::{Entity, World, Transform, Vec3};
use serde_json;
use std::collections::VecDeque;
use std::sync::Arc;

// ===========================================================================
// Input recording and playback
// ===========================================================================

/// A single input event in a recording.
#[derive(Debug, Clone, Copy)]
pub enum InputEvent {
    Key { key: KeyCode, pressed: bool, timestamp: f64 },
    MouseMove { x: f32, y: f32, timestamp: f64 },
    MouseButton { button: MouseButton, pressed: bool, timestamp: f64 },
    GamepadAxis { axis: GamepadAxis, value: f32, timestamp: f64 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum KeyCode {
    Space,
    Enter,
    Escape,
    W,
    A,
    S,
    D,
    Left,
    Right,
    Up,
    Down,
    MouseLeft,
    MouseRight,
    MouseMiddle,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GamepadAxis {
    LeftX,
    LeftY,
    RightX,
    RightY,
    TriggerL,
    TriggerR,
}

/// Records input events for later playback.
#[derive(Debug, Clone, Default)]
pub struct InputRecorder {
    pub events: VecDeque<InputEvent>,
    pub recording: bool,
    pub start_time: f64,
}

impl InputRecorder {
    pub fn new() -> Self {
        Self {
            events: VecDeque::new(),
            recording: false,
            start_time: 0.0,
        }
    }

    pub fn start(&mut self) {
        self.recording = true;
        self.start_time = now_secs();
    }

    pub fn stop(&mut self) {
        self.recording = false;
    }

    pub fn record(&mut self, event: InputEvent) {
        if self.recording {
            self.events.push_back(event);
        }
    }

    pub fn save(&self, path: impl AsRef<std::path::Path>) -> std::io::Result<()> {
        let data: Vec<InputEvent> = self.events.iter().copied().collect();
        std::fs::write(path, serde_json::to_vec(&data).unwrap_or_default())
    }

    pub fn load(path: impl AsRef<std::path::Path>) -> std::io::Result<Self> {
        let bytes = std::fs::read(path)?;
        let events: VecDeque<InputEvent> =
            serde_json::from_slice(&bytes).unwrap_or_default().into();
        Ok(Self {
            events,
            recording: false,
            start_time: 0.0,
        })
    }
}

/// Plays back recorded input events.
#[derive(Debug, Clone, Default)]
pub struct InputPlayback {
    pub events: VecDeque<InputEvent>,
    pub current_index: usize,
    pub speed: f64,
    pub start_time: f64,
}

impl InputPlayback {
    pub fn new(events: VecDeque<InputEvent>) -> Self {
        Self {
            events,
            current_index: 0,
            speed: 1.0,
            start_time: now_secs(),
        }
    }

    pub fn with_speed(mut self, speed: f64) -> Self {
        self.speed = speed;
        self
    }

    pub fn next(&mut self) -> Option<InputEvent> {
        if self.current_index < self.events.len() {
            let event = self.events[self.current_index];
            self.current_index += 1;
            Some(event)
        } else {
            None
        }
    }

    pub fn remaining(&self) -> usize {
        self.events.len().saturating_sub(self.current_index)
    }

    pub fn reset(&mut self) {
        self.current_index = 0;
        self.start_time = now_secs();
    }
}

// ===========================================================================
// Screen capture
// ===========================================================================

#[derive(Debug, Clone)]
pub struct ScreenCapture {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<u8>,
    pub timestamp: u64,
}

impl ScreenCapture {
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            pixels: vec![0u8; (width * height * 4) as usize],
            timestamp: super::now_nanos(),
        }
    }

    pub fn len(&self) -> usize {
        self.pixels.len()
    }

    pub fn is_empty(&self) -> bool {
        self.pixels.is_empty()
    }
}

/// Diff between two screen captures.
#[derive(Debug, Clone)]
pub struct PixelDiff {
    pub total_pixels: usize,
    pub differing_pixels: usize,
    pub max_channel_diff: u8,
    pub mean_channel_diff: f64,
}

impl PixelDiff {
    pub fn ratio(&self) -> f64 {
        if self.total_pixels == 0 {
            return 0.0;
        }
        self.differing_pixels as f64 / self.total_pixels as f64
    }
}

pub fn compare_screenshots(left: &ScreenCapture, right: &ScreenCapture) -> PixelDiff {
    let min_w = left.width.min(right.width) as usize;
    let min_h = left.height.min(right.height) as usize;
    let total = min_w * min_h * 4;

    let mut diff_sum: u64 = 0;
    let mut diff_count: usize = 0;
    let mut max_diff: u8 = 0;

    for y in 0..min_h {
        for x in 0..min_w {
            let idx = (y * min_w + x) * 4;
            for c in 0..4 {
                let a = left.pixels.get(idx + c).copied().unwrap_or(0);
                let b = right.pixels.get(idx + c).copied().unwrap_or(0);
                let d = a.abs_diff(b);
                max_diff = max_diff.max(d);
                diff_sum += d as u64;
                if d > 0 {
                    diff_count += 1;
                }
            }
        }
    }

    PixelDiff {
        total_pixels: total / 4,
        differing_pixels: diff_count,
        max_channel_diff: max_diff,
        mean_channel_diff: if diff_count > 0 { diff_sum as f64 / diff_count as f64 } else { 0.0 },
    }
}

// ===========================================================================
// Game state assertion helpers
// ===========================================================================

/// Asserts a property of the game world and returns a result.
#[derive(Debug, Clone)]
pub struct GameStateAssertion {
    pub name: String,
}

impl GameStateAssertion {
    pub fn new(name: impl Into<String>) -> Self {
        Self { name: name.into() }
    }

    pub fn entity_count(self, world: &World, expected: usize) -> GameStateAssertionBuilder {
        GameStateAssertionBuilder::new(self).check(move || {
            let actual = world.entity_count();
            if actual == expected {
                Ok(())
            } else {
                Err(format!("expected {} entities, got {}", expected, actual))
            }
        })
    }

    pub fn entity_alive(self, world: &World, entity: Entity) -> GameStateAssertionBuilder {
        GameStateAssertionBuilder::new(self).check(move || {
            if world.is_alive(entity) {
                Ok(())
            } else {
                Err(format!("entity {:?} is not alive", entity))
            }
        })
    }

    pub fn has_component<T: crate::Component + Clone + Send + Sync>(
        self,
        world: &World,
        entity: Entity,
    ) -> GameStateAssertionBuilder {
        GameStateAssertionBuilder::new(self).check(move || {
            if world.has_component::<T>(entity) {
                Ok(())
            } else {
                Err(format!("entity {:?} missing component {}", entity, std::any::type_name::<T>()))
            }
        })
    }

    pub fn transform_approx(
        self,
        world: &World,
        entity: Entity,
        expected_position: Vec3,
        epsilon: f32,
    ) -> GameStateAssertionBuilder {
        GameStateAssertionBuilder::new(self).check(move || {
            let transform = world.get_component::<Transform>(entity);
            match transform {
                Some(t) => {
                    let diff = (t.translation - expected_position).length();
                    if diff <= epsilon {
                        Ok(())
                    } else {
                        Err(format!("transform position diff {:.4} > epsilon {:.4}", diff, epsilon))
                    }
                }
                None => Err("entity missing Transform".into()),
            }
        })
    }
}

#[derive(Debug, Clone)]
pub struct GameStateAssertionBuilder {
    pub assertion: GameStateAssertion,
    checks: Vec<Box<dyn Fn() -> Result<(), String> + Send + Sync>>,
}

impl GameStateAssertionBuilder {
    pub fn new(assertion: GameStateAssertion) -> Self {
        Self { assertion, checks: Vec::new() }
    }

    pub fn check(mut self, f: impl Fn() -> Result<(), String> + Send + Sync + 'static) -> Self {
        self.checks.push(Box::new(f));
        self
    }

    pub fn run(&self, ctx: &TestContext) -> TestResult {
        for check in &self.checks {
            if let Err(msg) = check() {
                return TestResult::failed(&self.assertion.name, &ctx.suite, ctx.elapsed(), msg);
            }
        }
        TestResult::passed(&self.assertion.name, &ctx.suite, ctx.elapsed())
    }
}

// ===========================================================================
// Test scenes
// ===========================================================================

/// A test scene that can be loaded and asserted against.
#[derive(Debug, Clone)]
pub struct TestScene {
    pub name: String,
    pub setup: Option<Arc<dyn Fn(&mut World) + Send + Sync>>,
    pub assertions: Vec<Arc<dyn Fn(&World, &TestContext) -> TestResult + Send + Sync>>,
}

impl TestScene {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            setup: None,
            assertions: Vec::new(),
        }
    }

    pub fn with_setup(mut self, setup: impl Fn(&mut World) + Send + Sync + 'static) -> Self {
        self.setup = Some(Arc::new(setup));
        self
    }

    pub fn assert(mut self, assertion: impl Fn(&World, &TestContext) -> TestResult + Send + Sync + 'static) -> Self {
        self.assertions.push(Arc::new(assertion));
        self
    }

    pub fn run(&self, ctx: &TestContext) -> TestResult {
        let mut world = World::new();
        if let Some(ref setup) = self.setup {
            setup(&mut world);
        }

        for assertion in &self.assertions {
            let result = assertion(&world, ctx);
            if !result.is_success() {
                return result;
            }
        }

        TestResult::passed(&self.name, &ctx.suite, ctx.elapsed())
    }
}

// ===========================================================================
// Helpers
// ===========================================================================

#[inline]
fn now_secs() -> f64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs_f64())
        .unwrap_or(0.0)
}
