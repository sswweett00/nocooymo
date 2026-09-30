//! Performance tests and benchmarks.

use super::{TestContext, TestResult, TestSuite};
use std::time::{Duration, Instant};

// ===========================================================================
// Benchmark result
// ===========================================================================

#[derive(Debug, Clone)]
pub struct BenchmarkResult {
    pub name: String,
    pub suite: String,
    pub iterations: usize,
    pub total_duration: Duration,
    pub min_duration: Duration,
    pub max_duration: Duration,
    pub mean_duration: Duration,
    pub samples: Vec<Duration>,
}

impl BenchmarkResult {
    pub fn throughput(&self, unit: &str) -> f64 {
        let total_secs = self.total_duration.as_secs_f64();
        if total_secs > 0.0 {
            self.iterations as f64 / total_secs
        } else {
            0.0
        }
    }

    pub fn mean_ms(&self) -> f64 {
        self.mean_duration.as_secs_f64() * 1000.0
    }
}

// ===========================================================================
// Benchmark macros
// ===========================================================================

/// Simple benchmark: run a closure N times and return the result.
pub fn benchmark(name: impl Into<String>, iterations: usize, f: impl Fn() -> ()) -> BenchmarkResult {
    let name = name.into();
    let mut samples = Vec::with_capacity(iterations);
    let start = Instant::now();

    for _ in 0..iterations {
        let t0 = Instant::now();
        f();
        samples.push(t0.elapsed());
    }

    let total = start.elapsed();
    let min = *samples.iter().min().unwrap_or(&Duration::ZERO);
    let max = *samples.iter().max().unwrap_or(&Duration::ZERO);
    let sum: Duration = samples.iter().sum();
    let mean = if iterations > 0 { sum / iterations as u32 } else { Duration::ZERO };

    BenchmarkResult {
        name: name.clone(),
        suite: "benchmark".into(),
        iterations,
        total_duration: total,
        min_duration: min,
        max_duration: max,
        mean_duration: mean,
        samples,
    }
}

// ===========================================================================
// Frame time measurement
// ===========================================================================

/// Measure frame time over a number of frames.
#[derive(Debug, Clone)]
pub struct FrameTimeProfiler {
    pub frames: usize,
    pub frame_times: VecDeque<Duration>,
    pub start: Instant,
}

impl FrameTimeProfiler {
    pub fn new(frames: usize) -> Self {
        Self {
            frames,
            frame_times: VecDeque::with_capacity(frames),
            start: Instant::now(),
        }
    }

    pub fn begin_frame(&mut self) {
        self.start = Instant::now();
    }

    pub fn end_frame(&mut self) {
        let elapsed = self.start.elapsed();
        self.frame_times.push_back(elapsed);
        if self.frame_times.len() > self.frames {
            self.frame_times.pop_front();
        }
    }

    pub fn mean_frame_time(&self) -> Duration {
        if self.frame_times.is_empty() {
            return Duration::ZERO;
        }
        let sum: Duration = self.frame_times.iter().sum();
        sum / self.frame_times.len() as u32
    }

    pub fn fps(&self) -> f64 {
        let mean = self.mean_frame_time();
        let secs = mean.as_secs_f64();
        if secs > 0.0 { 1.0 / secs } else { 0.0 }
    }

    pub fn min_frame_time(&self) -> Duration {
        *self.frame_times.iter().min().unwrap_or(&Duration::ZERO)
    }

    pub fn max_frame_time(&self) -> Duration {
        *self.frame_times.iter().max().unwrap_or(&Duration::ZERO)
    }
}

// ===========================================================================
// Memory usage measurement
// ===========================================================================

/// Snapshot of memory usage.
#[derive(Debug, Clone, Default)]
pub struct MemorySnapshot {
    pub current_usage: u64,
    pub peak_usage: u64,
    pub allocations: u64,
    pub deallocations: u64,
    pub heap_usage: u64,
    pub timestamp: u64,
}

impl MemorySnapshot {
    pub fn new() -> Self {
        Self {
            timestamp: super::now_nanos(),
            ..Default::default()
        }
    }
}

/// Measure memory usage before and after an operation.
#[derive(Debug, Clone)]
pub struct MemoryProbe {
    before: MemorySnapshot,
    after: MemorySnapshot,
}

impl MemoryProbe {
    pub fn measure<F: FnOnce() -> R, R>(f: F) -> (R, Self) {
        let before = MemorySnapshot::new();
        let result = f();
        let after = MemorySnapshot::new();
        (result, Self { before, after })
    }

    pub fn delta(&self) -> MemorySnapshot {
        MemorySnapshot {
            current_usage: self.after.current_usage.saturating_sub(self.before.current_usage),
            peak_usage: self.after.peak_usage.saturating_sub(self.before.peak_usage),
            allocations: self.after.allocations.saturating_sub(self.before.allocations),
            deallocations: self.after.deallocations.saturating_sub(self.before.deallocations),
            heap_usage: self.after.heap_usage.saturating_sub(self.before.heap_usage),
            timestamp: self.after.timestamp,
        }
    }
}

// ===========================================================================
// Throughput measurement
// ===========================================================================

#[derive(Debug, Clone)]
pub struct ThroughputMeter {
    pub items_processed: usize,
    pub bytes_processed: usize,
    pub start: Instant,
    pub end: Option<Instant>,
}

impl ThroughputMeter {
    pub fn new() -> Self {
        Self {
            items_processed: 0,
            bytes_processed: 0,
            start: Instant::now(),
            end: None,
        }
    }

    pub fn start(&mut self) {
        self.start = Instant::now();
    }

    pub fn finish(&mut self) {
        self.end = Some(Instant::now());
    }

    pub fn record(&mut self, bytes: usize) {
        self.items_processed += 1;
        self.bytes_processed += bytes;
    }

    pub fn elapsed(&self) -> Duration {
        self.end.unwrap_or_else(Instant::now) - self.start
    }

    pub fn items_per_sec(&self) -> f64 {
        let secs = self.elapsed().as_secs_f64();
        if secs > 0.0 { self.items_processed as f64 / secs } else { 0.0 }
    }

    pub fn mb_per_sec(&self) -> f64 {
        let secs = self.elapsed().as_secs_f64();
        let bytes = self.bytes_processed as f64;
        if secs > 0.0 { (bytes / (1024.0 * 1024.0)) / secs } else { 0.0 }
    }
}

// ===========================================================================
// Performance test suite
// ===========================================================================

pub fn performance_suite() -> TestSuite {
    let mut suite = TestSuite::new("performance").tag("performance").tag("benchmark");

    // Benchmark: world spawn/despawn
    suite = suite.with_test(TestCase {
        id: super::next_test_id(),
        name: "benchmark_world_spawn_despawn".into(),
        suite: "performance".into(),
        tags: {
            let mut t = std::collections::HashSet::new();
            t.insert("performance".into());
            t.insert("world".into());
            t
        },
        timeout: Some(std::time::Duration::from_secs(30)),
        skip: false,
        skip_reason: None,
        test_fn: std::sync::Arc::new(|ctx: &TestContext| -> TestResult {
            let iterations = 1000;
            let result = benchmark("world_spawn_despawn", iterations, || {
                let mut w = World::new();
                let e = w.spawn();
                w.despawn(e);
            });

            // We consider it passing if mean frame time is reasonable (< 1ms per iteration)
            let mean_us = result.mean_duration.as_secs_f64() * 1_000_000.0;
            if mean_us > 1000.0 {
                TestResult::failed(&ctx.name, &ctx.suite, ctx.elapsed(), format!("mean iteration too slow: {:.1}us", mean_us))
            } else {
                TestResult::passed(&ctx.name, &ctx.suite, ctx.elapsed())
            }
        }),
    });

    // Frame time profiling
    suite = suite.with_test(TestCase {
        id: super::next_test_id(),
        name: "benchmark_frame_time_profiling".into(),
        suite: "performance".into(),
        tags: {
            let mut t = std::collections::HashSet::new();
            t.insert("performance".into());
            t.insert("frame_time".into());
            t
        },
        timeout: Some(std::time::Duration::from_secs(30)),
        skip: false,
        skip_reason: None,
        test_fn: std::sync::Arc::new(|ctx: &TestContext| -> TestResult {
            let mut profiler = FrameTimeProfiler::new(120);
            for _ in 0..120 {
                profiler.begin_frame();
                std::thread::sleep(Duration::from_millis(1));
                profiler.end_frame();
            }
            let fps = profiler.fps();
            if fps < 100.0 {
                TestResult::failed(&ctx.name, &ctx.suite, ctx.elapsed(), format!("fps too low: {:.1}", fps))
            } else {
                TestResult::passed(&ctx.name, &ctx.suite, ctx.elapsed())
            }
        }),
    });

    // Throughput measurement
    suite = suite.with_test(TestCase {
        id: super::next_test_id(),
        name: "benchmark_throughput".into(),
        suite: "performance".into(),
        tags: {
            let mut t = std::collections::HashSet::new();
            t.insert("performance".into());
            t.insert("throughput".into());
            t
        },
        timeout: Some(std::time::Duration::from_secs(30)),
        skip: false,
        skip_reason: None,
        test_fn: std::sync::Arc::new(|ctx: &TestContext| -> TestResult {
            let mut meter = ThroughputMeter::new();
            for _ in 0..1000 {
                meter.record(256);
            }
            meter.finish();
            let items = meter.items_per_sec();
            if items < 1000.0 {
                TestResult::failed(&ctx.name, &ctx.suite, ctx.elapsed(), format!("throughput too low: {:.1} items/s", items))
            } else {
                TestResult::passed(&ctx.name, &ctx.suite, ctx.elapsed())
            }
        }),
    });

    suite
}
