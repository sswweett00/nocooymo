//! Enterprise performance profiling and metrics system.
//!
//! Provides CPU frame timing, GPU timestamp queries (via backend), per-system
//! profiling scopes, ring-buffer frame history, and statistical analysis.

use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, OnceLock, RwLock};
use std::time::{Duration, Instant};

// ---------------------------------------------------------------------------
// Frame timing
// ---------------------------------------------------------------------------

/// A single frame's timing breakdown.
#[derive(Clone, Debug, Default)]
pub struct FrameTiming {
    pub frame_index: u64,
    pub total_time: Duration,
    pub cpu_time: Duration,
    pub gpu_time: Duration,
    pub update_time: Duration,
    pub render_time: Duration,
    pub physics_time: Duration,
    pub audio_time: Duration,
    pub network_time: Duration,
    pub asset_time: Duration,
    pub system_timings: Vec<(String, Duration)>,
}

impl FrameTiming {
    pub fn fps(&self) -> f32 {
        if self.total_time.as_secs_f32() > 0.0 {
            1.0 / self.total_time.as_secs_f32()
        } else {
            0.0
        }
    }

    pub fn frame_time_ms(&self) -> f32 {
        self.total_time.as_secs_f32() * 1000.0
    }
}

// ---------------------------------------------------------------------------
// Profiler
// ---------------------------------------------------------------------------

struct ProfilerInner {
    enabled: AtomicBool,
    frame_index: AtomicU64,
    frame_start: Mutex<Option<Instant>>,
    current_frame: Mutex<FrameTiming>,
    history: Mutex<VecDeque<FrameTiming>>,
    max_history: usize,
    scope_stack: Mutex<Vec<ScopeEntry>>,
    gpu_queries: Mutex<Vec<GpuTimestamp>>,
}

struct ScopeEntry {
    name: String,
    start: Instant,
}

#[derive(Clone, Debug)]
pub struct GpuTimestamp {
    pub label: String,
    pub start: u64,
    pub end: u64,
}

static PROFILER: OnceLock<ProfilerInner> = OnceLock::new();

fn profiler() -> &'static ProfilerInner {
    PROFILER.get_or_init(|| ProfilerInner {
        enabled: AtomicBool::new(true),
        frame_index: AtomicU64::new(0),
        frame_start: Mutex::new(None),
        current_frame: Mutex::new(FrameTiming::default()),
        history: Mutex::new(VecDeque::new()),
        max_history: 300,
        scope_stack: Mutex::new(Vec::new()),
        gpu_queries: Mutex::new(Vec::new()),
    })
}

/// Initialise the profiler with a given history size.
pub fn init(max_history: usize) {
    let _ = PROFILER.set(ProfilerInner {
        enabled: AtomicBool::new(true),
        frame_index: AtomicU64::new(0),
        frame_start: Mutex::new(None),
        current_frame: Mutex::new(FrameTiming::default()),
        history: Mutex::new(VecDeque::new()),
        max_history,
        scope_stack: Mutex::new(Vec::new()),
        gpu_queries: Mutex::new(Vec::new()),
    });
}

/// Enable or disable profiling.
pub fn set_enabled(enabled: bool) {
    profiler().enabled.store(enabled, Ordering::Relaxed);
}

/// Mark the beginning of a new frame.
pub fn begin_frame() {
    let p = profiler();
    if !p.enabled.load(Ordering::Relaxed) {
        return;
    }
    let idx = p.frame_index.fetch_add(1, Ordering::Relaxed);
    let mut frame = p.current_frame.lock().unwrap();
    *frame = FrameTiming {
        frame_index: idx,
        ..Default::default()
    };
    *p.frame_start.lock().unwrap() = Some(Instant::now());
}

/// Mark the end of the current frame and push it to history.
pub fn end_frame() {
    let p = profiler();
    if !p.enabled.load(Ordering::Relaxed) {
        return;
    }
    let frame_start = p.frame_start.lock().unwrap().take();
    let total = frame_start.map(|s| s.elapsed()).unwrap_or_default();
    let mut frame = p.current_frame.lock().unwrap();
    frame.total_time = total;
    let frame_clone = frame.clone();
    drop(frame);

    let mut hist = p.history.lock().unwrap();
    if hist.len() >= p.max_history {
        hist.pop_front();
    }
    hist.push_back(frame_clone);
}

/// RAII scope profiler.  Use the `profile_scope!` macro.
pub struct ProfileScope {
    name: String,
    start: Instant,
    gpu: bool,
}

impl ProfileScope {
    pub fn new(name: impl Into<String>) -> Self {
        let name = name.into();
        let p = profiler();
        if p.enabled.load(Ordering::Relaxed) {
            p.scope_stack.lock().unwrap().push(ScopeEntry {
                name: name.clone(),
                start: Instant::now(),
            });
        }
        Self {
            name,
            start: Instant::now(),
            gpu: false,
        }
    }

    pub fn new_gpu(name: impl Into<String>) -> Self {
        let s = Self::new(name);
        // mark as gpu scope — in a real backend this would insert a timestamp query
        s
    }
}

impl Drop for ProfileScope {
    fn drop(&mut self) {
        let p = profiler();
        if !p.enabled.load(Ordering::Relaxed) {
            return;
        }
        let elapsed = self.start.elapsed();
        let mut frame = p.current_frame.lock().unwrap();
        frame.system_timings.push((self.name.clone(), elapsed));

        // Also update the broad category timers based on name prefix
                match self.name.split("::").next().unwrap_or("") {
            "update" | "ecs" | "script" => frame.update_time += elapsed,
            "render" | "gpu" => frame.render_time += elapsed,
            "physics" => frame.physics_time += elapsed,
            "audio" => frame.audio_time += elapsed,
            "network" | "net" => frame.network_time += elapsed,
            "asset" => frame.asset_time += elapsed,
            _ => {}
        }
        frame.cpu_time += elapsed;
    }
}

/// Record a GPU timestamp range (for backends that support timestamp queries).
pub fn record_gpu_timestamp(label: impl Into<String>, start: u64, end: u64) {
    let p = profiler();
    if !p.enabled.load(Ordering::Relaxed) {
        return;
    }
    p.gpu_queries.lock().unwrap().push(GpuTimestamp {
        label: label.into(),
        start,
        end,
    });
}

// ---------------------------------------------------------------------------
// Statistics
// ---------------------------------------------------------------------------

/// Aggregated statistics over the frame history.
#[derive(Clone, Debug, Default)]
pub struct FrameStats {
    pub avg_fps: f32,
    pub min_fps: f32,
    pub max_fps: f32,
    pub avg_frame_ms: f32,
    pub p99_frame_ms: f32,
    pub avg_cpu_ms: f32,
    pub avg_gpu_ms: f32,
    pub avg_update_ms: f32,
    pub avg_render_ms: f32,
    pub avg_physics_ms: f32,
    pub frame_count: u32,
}

/// Compute statistics over the recorded frame history.
pub fn stats() -> FrameStats {
    let p = profiler();
    let hist = p.history.lock().unwrap();
    if hist.is_empty() {
        return FrameStats::default();
    }

    let mut fps_values: Vec<f32> = hist.iter().map(|f| f.fps()).collect();
    let mut frame_ms: Vec<f32> = hist.iter().map(|f| f.frame_time_ms()).collect();
    let cpu_ms: Vec<f32> = hist.iter().map(|f| f.cpu_time.as_secs_f32() * 1000.0).collect();
    let gpu_ms: Vec<f32> = hist.iter().map(|f| f.gpu_time.as_secs_f32() * 1000.0).collect();
    let update_ms: Vec<f32> = hist.iter().map(|f| f.update_time.as_secs_f32() * 1000.0).collect();
    let render_ms: Vec<f32> = hist.iter().map(|f| f.render_time.as_secs_f32() * 1000.0).collect();
    let physics_ms: Vec<f32> = hist.iter().map(|f| f.physics_time.as_secs_f32() * 1000.0).collect();

    fps_values.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    frame_ms.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

    let n = hist.len() as u32;
    let p99_idx = ((n as f32) * 0.99) as usize;

    FrameStats {
        avg_fps: mean(&fps_values),
        min_fps: fps_values.first().copied().unwrap_or(0.0),
        max_fps: fps_values.last().copied().unwrap_or(0.0),
        avg_frame_ms: mean(&frame_ms),
        p99_frame_ms: frame_ms.get(p99_idx).copied().unwrap_or(0.0),
        avg_cpu_ms: mean(&cpu_ms),
        avg_gpu_ms: mean(&gpu_ms),
        avg_update_ms: mean(&update_ms),
        avg_render_ms: mean(&render_ms),
        avg_physics_ms: mean(&physics_ms),
        frame_count: n,
    }
}

fn mean(values: &[f32]) -> f32 {
    if values.is_empty() {
        return 0.0;
    }
    values.iter().sum::<f32>() / values.len() as f32
}

/// Get the last N frame timings for display.
pub fn recent_frames(n: usize) -> Vec<FrameTiming> {
    let p = profiler();
    let hist = p.history.lock().unwrap();
    hist.iter().rev().take(n).cloned().collect()
}

/// Get the current frame index.
pub fn frame_index() -> u64 {
    profiler().frame_index.load(Ordering::Relaxed)
}

// ---------------------------------------------------------------------------
// Memory tracking
// ---------------------------------------------------------------------------

/// Memory usage statistics.
#[derive(Clone, Debug, Default)]
pub struct MemoryStats {
    pub total_allocated: u64,
    pub total_deallocated: u64,
    pub current_usage: u64,
    pub peak_usage: u64,
    pub allocation_count: u64,
    pub deallocation_count: u64,
}

impl MemoryStats {
    pub fn current_mb(&self) -> f32 {
        self.current_usage as f32 / (1024.0 * 1024.0)
    }

    pub fn peak_mb(&self) -> f32 {
        self.peak_usage as f32 / (1024.0 * 1024.0)
    }
}

static MEM_STATS: OnceLock<RwLock<MemoryStats>> = OnceLock::new();

fn mem_stats() -> &'static RwLock<MemoryStats> {
    MEM_STATS.get_or_init(|| RwLock::new(MemoryStats::default()))
}

/// Record a memory allocation.
pub fn record_allocation(size: u64) {
    let mut stats = mem_stats().write().unwrap();
    stats.total_allocated += size;
    stats.current_usage += size;
    stats.allocation_count += 1;
    if stats.current_usage > stats.peak_usage {
        stats.peak_usage = stats.current_usage;
    }
}

/// Record a memory deallocation.
pub fn record_deallocation(size: u64) {
    let mut stats = mem_stats().write().unwrap();
    stats.total_deallocated += size;
    stats.current_usage = stats.current_usage.saturating_sub(size);
    stats.deallocation_count += 1;
}

/// Get a snapshot of memory statistics.
pub fn memory_stats() -> MemoryStats {
    mem_stats().read().unwrap().clone()
}

// ---------------------------------------------------------------------------
// Counter system
// ---------------------------------------------------------------------------

/// A monotonically increasing counter for tracking engine events.
pub struct Counter {
    name: String,
    value: AtomicU64,
}

impl Counter {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            value: AtomicU64::new(0),
        }
    }

    pub fn inc(&self) -> u64 {
        self.value.fetch_add(1, Ordering::Relaxed) + 1
    }

    pub fn add(&self, n: u64) -> u64 {
        self.value.fetch_add(n, Ordering::Relaxed) + n
    }

    pub fn get(&self) -> u64 {
        self.value.load(Ordering::Relaxed)
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn reset(&self) {
        self.value.store(0, Ordering::Relaxed);
    }
}

// ---------------------------------------------------------------------------
// Macros
// ---------------------------------------------------------------------------

/// Profile the current scope.  Usage: `let _scope = profile_scope!("render::draw");`
#[macro_export]
macro_rules! profile_scope {
    ($name:expr) => {
        let _scope = $crate::performance::ProfileScope::new($name);
    };
}

/// Profile the current scope using the current function name.
#[macro_export]
macro_rules! profile_fn {
    () => {
        let _scope = $crate::performance::ProfileScope::new(concat!(module_path!()));
    };
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frame_timing_fps() {
        let t = FrameTiming {
            total_time: Duration::from_secs_f32(1.0 / 60.0),
            ..Default::default()
        };
        assert!((t.fps() - 60.0).abs() < 0.1);
    }

    #[test]
    fn profiler_records_scope() {
        init(100);
        begin_frame();
        {
            let _s = ProfileScope::new("render::draw");
            std::thread::sleep(Duration::from_millis(1));
        }
        end_frame();
        let stats = stats();
        assert!(stats.frame_count >= 1);
        assert!(stats.avg_render_ms > 0.0);
    }

    #[test]
    fn memory_tracking() {
        record_allocation(1024);
        record_allocation(2048);
        record_deallocation(1024);
        let m = memory_stats();
        assert_eq!(m.current_usage, 2048);
        assert_eq!(m.peak_usage, 3072);
        assert_eq!(m.allocation_count, 2);
    }

    #[test]
    fn counter_basic() {
        let c = Counter::new("test");
        assert_eq!(c.inc(), 1);
        assert_eq!(c.inc(), 2);
        assert_eq!(c.add(10), 12);
        assert_eq!(c.get(), 12);
    }
}