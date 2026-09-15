/// profiler.rs — Enterprise Profiling & Metrics System
///
/// Features:
/// - CPU profiler (named scopes, hierarchical timing)
/// - Frame statistics (FPS, frame time, percentiles)
/// - Memory tracker (allocations, usage by category)
/// - Performance counters (custom metrics)
/// - Performance alert system (threshold-based warnings)
/// - Ring buffer history
/// - Stats aggregation

use std::collections::HashMap;
use std::time::{Instant, Duration};
use serde::{Serialize, Deserialize};

// ═══════════════════════════════════════════════════════════ CPU Profiler

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProfileScope {
    pub name: String,
    pub start_us: u64,
    pub end_us: u64,
    pub depth: u32,
    pub thread_id: u64,
}

impl ProfileScope {
    pub fn duration_us(&self) -> u64 { self.end_us.saturating_sub(self.start_us) }
    pub fn duration_ms(&self) -> f32 { self.duration_us() as f32 / 1000.0 }
}

/// Hierarchical CPU profiler with ring-buffer storage
pub struct CpuProfiler {
    pub scopes: Vec<ProfileScope>,
    pub frame_scopes: Vec<Vec<ProfileScope>>,
    max_frames: usize,
    current_depth: u32,
    active_stack: Vec<String>,
    base_time: Instant,
    total_time_us: u64,
}

impl Default for CpuProfiler {
    fn default() -> Self {
        Self {
            scopes: Vec::new(),
            frame_scopes: Vec::new(),
            max_frames: 300,
            current_depth: 0,
            active_stack: Vec::new(),
            base_time: Instant::now(),
            total_time_us: 0,
        }
    }
}

impl CpuProfiler {
    pub fn new() -> Self { Self::default() }

    pub fn begin_frame(&mut self) {
        self.scopes.clear();
        self.current_depth = 0;
        self.active_stack.clear();
    }

    pub fn end_frame(&mut self) {
        let frame = std::mem::take(&mut self.scopes);
        self.frame_scopes.push(frame);
        if self.frame_scopes.len() > self.max_frames {
            self.frame_scopes.remove(0);
        }
    }

    pub fn begin_scope(&mut self, name: &str) {
        let start_us = self.elapsed_us();
        self.active_stack.push(name.to_string());
        self.current_depth += 1;
        // We'll update end_us later
        self.scopes.push(ProfileScope {
            name: name.to_string(),
            start_us,
            end_us: start_us,
            depth: self.current_depth,
            thread_id: 0,
        });
    }

    pub fn end_scope(&mut self) {
        let end_us = self.elapsed_us();
        if let Some(last) = self.scopes.last_mut() {
            last.end_us = end_us;
        }
        self.active_stack.pop();
        self.current_depth = self.current_depth.saturating_sub(1);
    }

    pub fn elapsed_us(&self) -> u64 {
        self.base_time.elapsed().as_micros() as u64
    }

    /// Get all scopes for a specific frame
    pub fn frame(&self, index: usize) -> Option<&[ProfileScope]> {
        self.frame_scopes.get(index).map(|f| f.as_slice())
    }

    pub fn frame_count(&self) -> usize { self.frame_scopes.len() }

    /// Get scope statistics across all recorded frames
    pub fn scope_stats(&self, name: &str) -> ScopeStats {
        let mut durations: Vec<u64> = self.frame_scopes.iter()
            .flat_map(|f| f.iter())
            .filter(|s| s.name == name)
            .map(|s| s.duration_us())
            .collect();

        if durations.is_empty() {
            return ScopeStats::default();
        }

        durations.sort();
        let total: u64 = durations.iter().sum();
        let count = durations.len() as u64;
        let avg = total / count;
        let min = durations[0];
        let max = durations[durations.len() - 1];
        let median = durations[durations.len() / 2];
        let p95_idx = ((durations.len() as f64) * 0.95) as usize;
        let p99_idx = ((durations.len() as f64) * 0.99) as usize;
        let p95 = durations[p95_idx.min(durations.len() - 1)];
        let p99 = durations[p99_idx.min(durations.len() - 1)];

        ScopeStats { name: name.to_string(), count, avg_us: avg, min_us: min, max_us: max, median_us: median, p95_us: p95, p99_us: p99 }
    }

    /// Get all unique scope names
    pub fn scope_names(&self) -> Vec<String> {
        let mut names: Vec<String> = self.frame_scopes.iter()
            .flat_map(|f| f.iter())
            .map(|s| s.name.clone())
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect();
        names.sort();
        names
    }

    pub fn clear(&mut self) {
        self.scopes.clear();
        self.frame_scopes.clear();
        self.current_depth = 0;
        self.active_stack.clear();
    }
}

#[derive(Clone, Debug, Default)]
pub struct ScopeStats {
    pub name: String,
    pub count: u64,
    pub avg_us: u64,
    pub min_us: u64,
    pub max_us: u64,
    pub median_us: u64,
    pub p95_us: u64,
    pub p99_us: u64,
}

// ═══════════════════════════════════════════════════════════ Frame Statistics

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct FrameStats {
    pub frame_number: u64,
    pub delta_time_ms: f32,
    pub fps: f32,
    pub frame_time_ms: f32,
    pub cpu_time_ms: f32,
    pub render_time_ms: f32,
    pub present_time_ms: f32,
    pub draw_calls: u32,
    pub triangles: u64,
    pub vertices: u64,
    pub textures_bound: u32,
    pub shader_switches: u32,
}

pub struct FrameStatistics {
    pub frames: Vec<FrameStats>,
    max_frames: usize,
    current_frame: u64,
    last_frame_time: Instant,
    fps_ema: f32,
    frame_time_ema: f32,
}

impl Default for FrameStatistics {
    fn default() -> Self {
        Self {
            frames: Vec::new(),
            max_frames: 600,
            current_frame: 0,
            last_frame_time: Instant::now(),
            fps_ema: 60.0,
            frame_time_ema: 16.67,
        }
    }
}

impl FrameStatistics {
    pub fn new() -> Self { Self::default() }

    pub fn begin_frame(&mut self) {
        let now = Instant::now();
        let dt = now.duration_since(self.last_frame_time).as_secs_f32();
        self.last_frame_time = now;

        if dt > 0.0001 {
            self.fps_ema = self.fps_ema * 0.95 + (1.0 / dt) * 0.05;
            self.frame_time_ema = self.frame_time_ema * 0.95 + (dt * 1000.0) * 0.05;
        }

        self.current_frame += 1;

        self.frames.push(FrameStats {
            frame_number: self.current_frame,
            delta_time_ms: dt * 1000.0,
            fps: 1.0 / dt.max(0.0001),
            ..Default::default()
        });

        if self.frames.len() > self.max_frames {
            self.frames.remove(0);
        }
    }

    pub fn end_frame(&mut self, draw_calls: u32, triangles: u64, render_ms: f32) {
        if let Some(frame) = self.frames.last_mut() {
            frame.draw_calls = draw_calls;
            frame.triangles = triangles;
            frame.render_time_ms = render_ms;
        }
    }

    pub fn current_fps(&self) -> f32 { self.fps_ema }
    pub fn current_frame_time(&self) -> f32 { self.frame_time_ema }
    pub fn total_frames(&self) -> u64 { self.current_frame }

    pub fn fps_history(&self) -> Vec<f32> {
        self.frames.iter().map(|f| f.fps).collect()
    }

    pub fn frame_time_history(&self) -> Vec<f32> {
        self.frames.iter().map(|f| f.delta_time_ms).collect()
    }

    pub fn avg_fps(&self) -> f32 {
        if self.frames.is_empty() { return 0.0; }
        let sum: f32 = self.frames.iter().map(|f| f.fps).sum();
        sum / self.frames.len() as f32
    }

    pub fn min_fps(&self) -> f32 {
        self.frames.iter().map(|f| f.fps).fold(f32::INFINITY, f32::min)
    }

    pub fn max_fps(&self) -> f32 {
        self.frames.iter().map(|f| f.fps).fold(0.0f32, f32::max)
    }

    pub fn percentile_fps(&self, p: f32) -> f32 {
        if self.frames.is_empty() { return 0.0; }
        let mut sorted: Vec<f32> = self.frames.iter().map(|f| f.fps).collect();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let idx = ((sorted.len() as f32) * p / 100.0) as usize;
        sorted[idx.min(sorted.len() - 1)]
    }

    pub fn clear(&mut self) {
        self.frames.clear();
        self.current_frame = 0;
    }
}

// ═══════════════════════════════════════════════════════════ Memory Tracker

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct AllocationInfo {
    pub size_bytes: usize,
    pub category: String,
    pub timestamp_us: u64,
}

pub struct MemoryTracker {
    pub total_allocated: usize,
    pub total_freed: usize,
    pub peak_usage: usize,
    pub current_usage: usize,
    pub category_usage: HashMap<String, usize>,
    pub allocation_count: u64,
    pub free_count: u64,
    max_tracked: usize,
    recent_allocations: Vec<AllocationInfo>,
}

impl Default for MemoryTracker {
    fn default() -> Self {
        Self {
            total_allocated: 0,
            total_freed: 0,
            peak_usage: 0,
            current_usage: 0,
            category_usage: HashMap::new(),
            allocation_count: 0,
            free_count: 0,
            max_tracked: 10000,
            recent_allocations: Vec::new(),
        }
    }
}

impl MemoryTracker {
    pub fn new() -> Self { Self::default() }

    pub fn allocate(&mut self, size: usize, category: &str) {
        self.total_allocated += size;
        self.current_usage += size;
        self.allocation_count += 1;
        *self.category_usage.entry(category.to_string()).or_insert(0) += size;

        if self.current_usage > self.peak_usage {
            self.peak_usage = self.current_usage;
        }

        if self.recent_allocations.len() < self.max_tracked {
            self.recent_allocations.push(AllocationInfo {
                size_bytes: size,
                category: category.to_string(),
                timestamp_us: 0,
            });
        }
    }

    pub fn free(&mut self, size: usize, category: &str) {
        self.total_freed += size;
        self.current_usage = self.current_usage.saturating_sub(size);
        self.free_count += 1;
        if let Some(cat_size) = self.category_usage.get_mut(category) {
            *cat_size = cat_size.saturating_sub(size);
        }
    }

    pub fn usage_mb(&self) -> f32 { self.current_usage as f32 / (1024.0 * 1024.0) }
    pub fn peak_mb(&self) -> f32 { self.peak_usage as f32 / (1024.0 * 1024.0) }

    pub fn category_usage_mb(&self) -> Vec<(String, f32)> {
        self.category_usage.iter()
            .map(|(k, v)| (k.clone(), *v as f32 / (1024.0 * 1024.0)))
            .collect()
    }

    pub fn summary(&self) -> MemorySummary {
        MemorySummary {
            current_bytes: self.current_usage,
            peak_bytes: self.peak_usage,
            allocated_bytes: self.total_allocated,
            freed_bytes: self.total_freed,
            alloc_count: self.allocation_count,
            free_count: self.free_count,
            categories: self.category_usage.clone(),
        }
    }

    pub fn clear(&mut self) {
        *self = Self::default();
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct MemorySummary {
    pub current_bytes: usize,
    pub peak_bytes: usize,
    pub allocated_bytes: usize,
    pub freed_bytes: usize,
    pub alloc_count: u64,
    pub free_count: u64,
    pub categories: HashMap<String, usize>,
}

// ═══════════════════════════════════════════════════════════ Performance Counters

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PerfCounter {
    pub name: String,
    pub value: f64,
    pub counter_type: CounterType,
    pub unit: String,
    pub min_value: f64,
    pub max_value: f64,
    pub history: Vec<f64>,
    max_history: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum CounterType {
    Instantaneous,
    Accumulator,
    Average,
    Rate,
}

impl PerfCounter {
    pub fn instantaneous(name: &str, unit: &str) -> Self {
        Self {
            name: name.into(), value: 0.0, counter_type: CounterType::Instantaneous,
            unit: unit.into(), min_value: f64::MAX, max_value: f64::MIN,
            history: Vec::new(), max_history: 300,
        }
    }

    pub fn accumulator(name: &str, unit: &str) -> Self {
        Self { counter_type: CounterType::Accumulator, ..Self::instantaneous(name, unit) }
    }

    pub fn set(&mut self, value: f64) {
        self.value = value;
        self.min_value = self.min_value.min(value);
        self.max_value = self.max_value.max(value);
        self.history.push(value);
        if self.history.len() > self.max_history {
            self.history.remove(0);
        }
    }

    pub fn increment(&mut self, delta: f64) {
        self.value += delta;
        self.set(self.value);
    }

    pub fn avg(&self) -> f64 {
        if self.history.is_empty() { 0.0 }
        else { self.history.iter().sum::<f64>() / self.history.len() as f64 }
    }

    pub fn reset(&mut self) {
        self.value = 0.0;
        self.min_value = f64::MAX;
        self.max_value = f64::MIN;
        self.history.clear();
    }
}

// ═══════════════════════════════════════════════════════════ Performance Alerts

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PerfAlert {
    pub name: String,
    pub counter_name: String,
    pub condition: AlertCondition,
    pub threshold: f64,
    pub severity: AlertSeverity,
    pub enabled: bool,
    pub triggered_count: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum AlertCondition {
    GreaterThan,
    LessThan,
    Equals,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum AlertSeverity {
    Info,
    Warning,
    Critical,
    Fatal,
}

impl AlertSeverity {
    pub fn name(&self) -> &'static str {
        match self {
            Self::Info => "Info",
            Self::Warning => "Warning",
            Self::Critical => "Critical",
            Self::Fatal => "Fatal",
        }
    }
}

// ═══════════════════════════════════════════════════════════ Master Profiler

pub struct Profiler {
    pub cpu: CpuProfiler,
    pub frames: FrameStatistics,
    pub memory: MemoryTracker,
    pub counters: HashMap<String, PerfCounter>,
    pub alerts: Vec<PerfAlert>,
    pub active: bool,
}

impl Default for Profiler {
    fn default() -> Self {
        Self {
            cpu: CpuProfiler::new(),
            frames: FrameStatistics::new(),
            memory: MemoryTracker::new(),
            counters: HashMap::new(),
            alerts: Vec::new(),
            active: true,
        }
    }
}

impl Profiler {
    pub fn new() -> Self { Self::default() }

    pub fn begin_frame(&mut self) {
        if !self.active { return; }
        self.cpu.begin_frame();
        self.frames.begin_frame();
    }

    pub fn end_frame(&mut self) {
        if !self.active { return; }
        self.cpu.end_frame();
        self.frames.end_frame(0, 0, 0.0);
        self.check_alerts();
    }

    pub fn scope(&mut self, name: &str) -> ProfileScopeGuard<'_> {
        let active = self.active;
        if active {
            self.cpu.begin_scope(name);
        }
        ProfileScopeGuard { profiler: self, active }
    }

    pub fn counter(&mut self, name: &str, unit: &str) -> &mut PerfCounter {
        self.counters.entry(name.to_string())
            .or_insert_with(|| PerfCounter::instantaneous(name, unit))
    }

    pub fn add_alert(&mut self, alert: PerfAlert) {
        self.alerts.push(alert);
    }

    fn check_alerts(&mut self) {
        for alert in &mut self.alerts {
            if !alert.enabled { continue; }
            if let Some(counter) = self.counters.get(&alert.counter_name) {
                let triggered = match alert.condition {
                    AlertCondition::GreaterThan => counter.value > alert.threshold,
                    AlertCondition::LessThan => counter.value < alert.threshold,
                    AlertCondition::Equals => (counter.value - alert.threshold).abs() < 0.0001,
                };
                if triggered {
                    alert.triggered_count += 1;
                }
            }
        }
    }

    pub fn triggered_alerts(&self) -> Vec<&PerfAlert> {
        self.alerts.iter().filter(|a| a.enabled && a.triggered_count > 0).collect()
    }

    pub fn report(&self) -> ProfilerReport {
        ProfilerReport {
            fps: self.frames.current_fps(),
            frame_time_ms: self.frames.current_frame_time(),
            frame_count: self.frames.total_frames(),
            memory_usage_bytes: self.memory.current_usage,
            memory_peak_bytes: self.memory.peak_usage,
            scope_stats: self.cpu.scope_names().iter()
                .map(|n| self.cpu.scope_stats(n))
                .collect(),
            counter_values: self.counters.iter()
                .map(|(k, v)| (k.clone(), v.value))
                .collect(),
            alert_count: self.alerts.len(),
            triggered_alert_count: self.alerts.iter().filter(|a| a.triggered_count > 0).count(),
        }
    }

    pub fn clear(&mut self) {
        self.cpu.clear();
        self.frames.clear();
        self.memory.clear();
        for counter in self.counters.values_mut() {
            counter.reset();
        }
        for alert in &mut self.alerts {
            alert.triggered_count = 0;
        }
    }
}

pub struct ProfileScopeGuard<'a> {
    profiler: &'a mut Profiler,
    active: bool,
}

impl<'a> Drop for ProfileScopeGuard<'a> {
    fn drop(&mut self) {
        if self.active {
            self.profiler.cpu.end_scope();
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct ProfilerReport {
    pub fps: f32,
    pub frame_time_ms: f32,
    pub frame_count: u64,
    pub memory_usage_bytes: usize,
    pub memory_peak_bytes: usize,
    pub scope_stats: Vec<ScopeStats>,
    pub counter_values: Vec<(String, f64)>,
    pub alert_count: usize,
    pub triggered_alert_count: usize,
}

// ═══════════════════════════════════════════════════════════ Tests

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cpu_profiler() {
        let mut profiler = CpuProfiler::new();
        profiler.begin_frame();
        profiler.begin_scope("main_loop");
        profiler.begin_scope("update");
        profiler.end_scope();
        profiler.begin_scope("render");
        profiler.end_scope();
        profiler.end_scope();
        profiler.end_frame();

        assert_eq!(profiler.frame_count(), 1);
        let scopes = profiler.frame(0).unwrap();
        assert_eq!(scopes.len(), 3);
        assert_eq!(scopes[0].name, "main_loop");
        assert_eq!(scopes[1].name, "update");
        assert_eq!(scopes[2].name, "render");
    }

    #[test]
    fn test_scope_stats() {
        let mut profiler = CpuProfiler::new();
        for _ in 0..10 {
            profiler.begin_frame();
            profiler.begin_scope("test_scope");
            std::thread::sleep(Duration::from_micros(10));
            profiler.end_scope();
            profiler.end_frame();
        }
        let stats = profiler.scope_stats("test_scope");
        assert_eq!(stats.name, "test_scope");
        assert!(stats.avg_us > 0);
        assert!(stats.min_us > 0);
        assert!(stats.max_us >= stats.min_us);
    }

    #[test]
    fn test_frame_statistics() {
        let mut stats = FrameStatistics::new();
        for _ in 0..60 {
            stats.begin_frame();
            stats.end_frame(100, 10000, 2.0);
        }
        assert!(stats.current_fps() > 0.0);
        assert!(stats.avg_fps() > 0.0);
        assert_eq!(stats.total_frames(), 60);
    }

    #[test]
    fn test_memory_tracker() {
        let mut tracker = MemoryTracker::new();
        tracker.allocate(1024, "Textures");
        tracker.allocate(2048, "Meshes");
        assert_eq!(tracker.current_usage, 3072);
        assert_eq!(tracker.peak_usage, 3072);

        tracker.free(1024, "Textures");
        assert_eq!(tracker.current_usage, 2048);
        assert_eq!(tracker.peak_usage, 3072);

        assert!((tracker.usage_mb() - 2048.0 / (1024.0 * 1024.0)).abs() < 0.001);
    }

    #[test]
    fn test_memory_categories() {
        let mut tracker = MemoryTracker::new();
        tracker.allocate(100, "A");
        tracker.allocate(200, "B");
        tracker.allocate(300, "A");

        let cats = tracker.category_usage_mb();
        assert_eq!(cats.len(), 2);
        assert!(cats.iter().any(|(k, v)| k == "A" && *v > 0.0003));
        assert!(cats.iter().any(|(k, v)| k == "B"));
    }

    #[test]
    fn test_perf_counter() {
        let mut counter = PerfCounter::instantaneous("fps", "fps");
        counter.set(60.0);
        counter.set(55.0);
        counter.set(65.0);
        assert_eq!(counter.value, 65.0);
        assert_eq!(counter.min_value, 55.0);
        assert_eq!(counter.max_value, 65.0);
        assert!((counter.avg() - 60.0).abs() < 0.1);
    }

    #[test]
    fn test_perf_counter_accumulator() {
        let mut counter = PerfCounter::accumulator("draw_calls", "count");
        counter.increment(100.0);
        counter.increment(50.0);
        assert_eq!(counter.value, 150.0);
    }

    #[test]
    fn test_master_profiler() {
        let mut profiler = Profiler::new();
        profiler.begin_frame();
        {
            let _guard = profiler.scope("test_scope");
        }
        profiler.end_frame();

        let report = profiler.report();
        assert!(report.frame_time_ms >= 0.0);
        assert_eq!(report.frame_count, 1);
    }

    #[test]
    fn test_profiler_report() {
        let mut profiler = Profiler::new();
        profiler.counter("fps", "fps").set(60.0);
        profiler.counter("draw_calls", "count").set(100.0);

        let report = profiler.report();
        assert_eq!(report.counter_values.len(), 2);
    }

    #[test]
    fn test_scope_names() {
        let mut profiler = CpuProfiler::new();
        profiler.begin_frame();
        profiler.begin_scope("A");
        profiler.end_scope();
        profiler.begin_scope("B");
        profiler.end_scope();
        profiler.end_frame();

        profiler.begin_frame();
        profiler.begin_scope("A");
        profiler.end_scope();
        profiler.begin_scope("C");
        profiler.end_scope();
        profiler.end_frame();

        let names = profiler.scope_names();
        assert!(names.contains(&"A".to_string()));
        assert!(names.contains(&"B".to_string()));
        assert!(names.contains(&"C".to_string()));
    }

    #[test]
    fn test_profiler_clear() {
        let mut profiler = Profiler::new();
        profiler.begin_frame();
        profiler.end_frame();
        profiler.counter("test", "unit").set(42.0);

        profiler.clear();
        let report = profiler.report();
        assert_eq!(report.frame_count, 0);
    }

    #[test]
    fn test_frame_statistics_percentile() {
        let mut stats = FrameStatistics::new();
        for i in 0..100 {
            stats.begin_frame();
            stats.frames.last_mut().unwrap().fps = i as f32;
            stats.end_frame(0, 0, 0.0);
        }
        let p50 = stats.percentile_fps(50.0);
        let p95 = stats.percentile_fps(95.0);
        assert!(p95 >= p50);
    }

    #[test]
    fn test_frame_stats_min_max() {
        let mut stats = FrameStatistics::new();
        for i in 0..10 {
            stats.begin_frame();
            stats.frames.last_mut().unwrap().fps = (i * 10) as f32;
            stats.end_frame(0, 0, 0.0);
        }
        assert_eq!(stats.min_fps(), 0.0);
        assert_eq!(stats.max_fps(), 90.0);
    }
}
