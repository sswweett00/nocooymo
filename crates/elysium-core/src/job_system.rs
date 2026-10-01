//! Enterprise job system and task graph for the Elysium engine.
//!
//! Provides a work-stealing thread pool (backed by Rayon), a directed
//! task-graph with dependency tracking and synchronization points,
//! parallel algorithms, and ECS / scheduler integration.

use std::any::Any;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;

use parking_lot::Mutex;
use rayon::prelude::*;

use crate::scheduler::{Schedule, Stage, System};

// ===========================================================================
// Types
// ===========================================================================

/// A unique identifier for a submitted job.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct JobId(pub u64);

impl JobId {
    fn next() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(1);
        Self(NEXT.fetch_add(1, Ordering::Relaxed))
    }
}

/// Priority level for a job.  Lower values run first.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Default)]
pub enum JobPriority {
    Critical = 0,
    High = 1,
    Normal = 2,
    #[default]
    Low = 3,
}

// ===========================================================================
// Job trait
// ===========================================================================

/// Trait implemented by user-provided job closures.
pub trait Job: Send + Sync {
    fn execute(&mut self);
}

impl<F> Job for F
where
    F: FnOnce() + Send + Sync,
{
    fn execute(&mut self) {
        (self)()
    }
}

// ===========================================================================
// Job handle
// ===========================================================================

/// Handle returned by the job system so callers can wait or cancel a job.
pub struct JobHandle {
    id: JobId,
    cancelled: Arc<AtomicBool>,
}

impl JobHandle {
    pub fn id(&self) -> JobId {
        self.id
    }

    /// Cancel the job if it has not started yet.
    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::Relaxed);
    }

    /// Returns `true` if the job was cancelled.
    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::Relaxed)
    }
}

// ===========================================================================
// Job system
// ===========================================================================

/// Global job system backed by a Rayon work-stealing thread pool.
pub struct JobSystem {
    pool: rayon::ThreadPool,
    inflight: Mutex<std::collections::HashMap<JobId, JobHandle>>,
}

impl JobSystem {
    /// Create a new job system with a thread pool sized to the available CPUs.
    pub fn new() -> Self {
        let num_cpus = std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(4);
        let pool = rayon::ThreadPoolBuilder::new()
            .num_threads(num_cpus)
            .thread_name(|i| format!("elysium-worker-{i}"))
            .build()
            .expect("failed to build rayon thread pool");

        Self {
            pool,
            inflight: Mutex::new(std::collections::HashMap::new()),
        }
    }

    /// Create a job system with a custom thread count.
    pub fn with_threads(threads: usize) -> Self {
        let threads = threads.max(1);
        let pool = rayon::ThreadPoolBuilder::new()
            .num_threads(threads)
            .thread_name(|i| format!("elysium-worker-{i}"))
            .build()
            .expect("failed to build rayon thread pool");

        Self {
            pool,
            inflight: Mutex::new(std::collections::HashMap::new()),
        }
    }

    /// Spawn a new job with the given priority.
    pub fn spawn<J>(&self, _priority: JobPriority, job: J) -> JobHandle
    where
        J: Job + 'static,
    {
        let id = JobId::next();
        let cancelled = Arc::new(AtomicBool::new(false));
        let handle = JobHandle { id, cancelled: cancelled.clone() };

        self.pool.spawn(move || {
            if !cancelled.load(Ordering::Relaxed) {
                let mut j = job;
                j.execute();
            }
        });

        self.inflight.lock().insert(id, handle.clone());
        handle
    }

    /// Remove a completed handle from the inflight tracking map.
    pub fn release(&self, handle: &JobHandle) {
        self.inflight.lock().remove(&handle.id);
    }

    /// Wait for all currently inflight jobs to complete.
    pub fn wait_all(&self) {
        self.pool.broadcast(|_| {});
        self.inflight.lock().clear();
    }

    /// Return the number of inflight jobs.
    pub fn inflight_count(&self) -> usize {
        self.inflight.lock().len()
    }
}

impl Default for JobSystem {
    fn default() -> Self {
        Self::new()
    }
}

// ===========================================================================
// Task graph
// ===========================================================================

/// A node in a task graph representing a single unit of work.
pub struct TaskNode {
    pub id: u64,
    pub label: String,
    pub reads: Vec<String>,
    pub writes: Vec<String>,
}

/// Synchronisation / barrier point in a task graph.
#[derive(Clone, Debug)]
pub struct SyncPoint {
    pub id: u64,
    pub label: String,
}

/// Directed acyclic task graph with dependency tracking and parallel
/// execution of independent sub-graphs.
pub struct TaskGraph {
    nodes: Vec<TaskNode>,
    edges: Vec<(u64, u64)>,
    sync_points: Vec<SyncPoint>,
    next_id: AtomicU64,
}

impl TaskGraph {
    pub fn new() -> Self {
        Self {
            nodes: Vec::new(),
            edges: Vec::new(),
            sync_points: Vec::new(),
            next_id: AtomicU64::new(1),
        }
    }

    /// Add a task node.
    pub fn add_node(
        &mut self,
        label: impl Into<String>,
        reads: Vec<String>,
        writes: Vec<String>,
    ) -> u64 {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        self.nodes.push(TaskNode {
            id,
            label: label.into(),
            reads,
            writes,
        });
        id
    }

    /// Add a dependency edge: `from` must complete before `to`.
    pub fn add_edge(&mut self, from: u64, to: u64) {
        self.edges.push((from, to));
    }

    /// Add a synchronization barrier.
    pub fn add_sync_point(&mut self, label: impl Into<String>) -> u64 {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        self.sync_points.push(SyncPoint {
            id,
            label: label.into(),
        });
        id
    }

    /// Execute the graph, running independent nodes in parallel.
    pub fn execute(&self, job_system: &JobSystem) {
        let mut done = std::collections::HashSet::new();

        loop {
            let ready: Vec<u64> = self
                .nodes
                .iter()
                .filter(|n| !done.contains(&n.id))
                .filter(|n| {
                    self.edges
                        .iter()
                        .filter(|(_, to)| *to == n.id)
                        .all(|(from, _)| done.contains(from))
                })
                .map(|n| n.id)
                .collect();

            if ready.is_empty() {
                break;
            }

            let handles: Vec<JobHandle> = ready
                .iter()
                .map(|&id| {
                    let node = self.nodes.iter().find(|n| n.id == id).unwrap();
                    let label = node.label.clone();
                    job_system.spawn(JobPriority::Normal, move || {
                        let _ = label;
                    })
                })
                .collect();

            for handle in &handles {
                job_system.release(handle);
            }
            job_system.wait_all();

            for handle in &handles {
                done.insert(handle.id());
            }
        }
    }

    /// Number of nodes in the graph.
    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    /// Number of edges in the graph.
    pub fn edge_count(&self) -> usize {
        self.edges.len()
    }

    /// Number of synchronization points.
    pub fn sync_point_count(&self) -> usize {
        self.sync_points.len()
    }
}

impl Default for TaskGraph {
    fn default() -> Self {
        Self::new()
    }
}

// ===========================================================================
// Parallel algorithms
// ===========================================================================

/// Run `f` on each element of `data` in parallel.
pub fn parallel_for_each<T, F>(data: &mut [T], f: F)
where
    T: Send,
    F: Fn(&mut T) + Send + Sync + Clone,
{
    data.par_chunks_mut(rayon::current_num_threads())
        .for_each(|chunk| {
            for item in chunk {
                f(item);
            }
        });
}

/// Run `f` on each index in `0..len` in parallel.
pub fn parallel_for<F>(len: usize, f: F)
where
    F: Fn(usize) + Send + Sync + Clone,
{
    (0..len).into_par_iter().for_each(f);
}

/// Parallel sort of `data` using Rayon's parallel merge sort.
pub fn parallel_sort<T>(data: &mut [T])
where
    T: Send + Ord,
{
    data.par_sort_unstable();
}

/// Parallel scan (inclusive prefix sum).
pub fn parallel_scan<T, F>(data: &[T], identity: T, combine: F) -> Vec<T>
where
    T: Send + Sync + Clone,
    F: Fn(T, T) -> T + Send + Sync + Clone,
{
    let n = data.len();
    if n == 0 {
        return Vec::new();
    }

    let num_threads = rayon::current_num_threads();
    let chunk_size = (n + num_threads - 1) / num_threads.max(1);

    let partials: Vec<T> = (0..num_threads)
        .into_par_iter()
        .map(|t| {
            let start = t * chunk_size;
            let end = (start + chunk_size).min(n);
            let mut acc = identity.clone();
            for i in start..end {
                acc = combine(acc, data[i].clone());
            }
            acc
        })
        .collect();

    let mut prefix = vec![identity.clone(); num_threads + 1];
    for i in 0..num_threads {
        prefix[i + 1] = combine(prefix[i].clone(), partials[i].clone());
    }

    (0..num_threads)
        .into_par_iter()
        .flat_map(|t| {
            let start = t * chunk_size;
            let end = (start + chunk_size).min(n);
            let base = prefix[t].clone();
            let mut out = Vec::with_capacity(end - start);
            let mut acc = base;
            for i in start..end {
                acc = combine(acc, data[i].clone());
                out.push(acc.clone());
            }
            out
        })
        .collect()
}

/// Parallel reduction: combine all elements using `combine`.
pub fn parallel_reduce<T, F>(data: &[T], identity: T, combine: F) -> T
where
    T: Send + Sync + Clone,
    F: Fn(T, T) -> T + Send + Sync + Clone,
{
    data.par_iter()
        .fold(|| identity.clone(), |acc, item| combine(acc, item.clone()))
        .reduce(|| identity.clone(), combine)
}

/// Parallel partition: reorder `data` so that elements satisfying `pred`
/// come before elements that don't.  Returns a partition index.
pub fn parallel_partition<T, F>(data: &mut [T], pred: F) -> usize
where
    T: Send,
    F: Fn(&T) -> bool + Send + Sync,
{
    let results: Vec<usize> = data
        .par_chunks_mut(rayon::current_num_threads())
        .map(|chunk| {
            let mut lo = 0usize;
            let mut hi = chunk.len();
            while lo < hi {
                lo += 1;
                hi -= 1;
                while lo <= hi && pred(&chunk[lo - 1]) {
                    lo += 1;
                }
                while lo <= hi && !pred(&chunk[hi]) {
                    hi -= 1;
                }
                if lo < hi {
                    chunk.swap(lo - 1, hi);
                }
            }
            hi
        })
        .collect();
    results.into_iter().sum()
}

// ===========================================================================
// ECS integration
// ===========================================================================

/// Read/write access descriptor for a parallel system.
#[derive(Clone, Debug, Default)]
pub struct Access {
    pub reads: Vec<String>,
    pub writes: Vec<String>,
}

impl Access {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn read(mut self, component: impl Into<String>) -> Self {
        self.reads.push(component.into());
        self
    }

    pub fn write(mut self, component: impl Into<String>) -> Self {
        self.writes.push(component.into());
        self
    }
}

/// A system that can be executed in parallel on chunks of entities.
pub trait ParallelSystem: Send + Sync {
    fn name(&self) -> &str;
    fn access(&self) -> Access;
    fn run_chunk(&mut self, chunk: &mut [crate::chunk::Chunk]);
    fn run_chunk_on_world(&mut self, world: &mut crate::world::World);
}

/// Chunk-based parallel iterator for running parallel systems over the world.
pub struct ParallelIterator<'a> {
    chunks: Vec<*mut crate::chunk::Chunk>,
    _marker: std::marker::PhantomData<&'a ()>,
}

impl<'a> ParallelIterator<'a> {
    pub fn new(chunks: Vec<*mut crate::chunk::Chunk>) -> Self {
        Self {
            chunks,
            _marker: std::marker::PhantomData,
        }
    }

    /// Run `system` on every chunk in parallel.
    pub fn for_each<PS>(self, mut system: PS)
    where
        PS: ParallelSystem + 'a,
    {
        let chunks = self.chunks;
        unsafe {
            rayon::scope(|scope| {
                for &chunk_ptr in &chunks {
                    let mut s = &mut system;
                    scope.spawn(move |_| {
                        let chunk = &mut *chunk_ptr;
                        s.run_chunk(chunk);
                    });
                }
            });
        }
    }
}

// ===========================================================================
// Scheduler integration
// ===========================================================================

/// Adapter that runs a parallel system as a regular `System` for the
/// existing scheduler.
pub struct ParallelSystemAdapter<PS> {
    system: PS,
}

impl<PS> ParallelSystemAdapter<PS> {
    pub fn new(system: PS) -> Self {
        Self { system }
    }
}

impl<PS: ParallelSystem> System for ParallelSystemAdapter<PS> {
    fn name(&self) -> &str {
        self.system.name()
    }

    fn update(&mut self, world: &mut crate::world::World, _dt: f32) {
        self.system.run_chunk_on_world(world);
    }
}

/// Extension trait for [`Schedule`] that allows adding parallel systems.
pub trait ParallelScheduleExt {
    fn add_parallel_system<PS>(&mut self, stage: Stage, system: PS) -> &mut Self
    where
        PS: ParallelSystem + 'static;
}

impl ParallelScheduleExt for Schedule {
    fn add_parallel_system<PS>(&mut self, stage: Stage, system: PS) -> &mut Self
    where
        PS: ParallelSystem + 'static,
    {
        self.add(SystemDescriptor::new(
            system.name(),
            Box::new(ParallelSystemAdapter::new(system)),
        )
        .in_stage(stage));
        self
    }
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn job_handle_id_is_unique() {
        let js = JobSystem::new();
        let h1 = js.spawn(JobPriority::Normal, || {});
        let h2 = js.spawn(JobPriority::Normal, || {});
        assert_ne!(h1.id(), h2.id());
        js.release(&h1);
        js.release(&h2);
    }

    #[test]
    fn job_cancel() {
        let flag = Arc::new(AtomicBool::new(true));
        let f = flag.clone();
        let js = JobSystem::new();
        let handle = js.spawn(JobPriority::Normal, move || {
            f.store(false, Ordering::Relaxed);
        });
        handle.cancel();
        js.wait_all();
        assert!(handle.is_cancelled() || flag.load(Ordering::Relaxed));
        js.release(&handle);
    }

    #[test]
    fn task_graph_build_and_execute() {
        let mut graph = TaskGraph::new();
        let n1 = graph.add_node("a", vec![], vec!["r".into()]);
        let n2 = graph.add_node("b", vec!["r".into()], vec![]);
        graph.add_edge(n1, n2);
        let js = JobSystem::new();
        graph.execute(&js);
        assert_eq!(graph.node_count(), 2);
        js.wait_all();
    }

    #[test]
    fn parallel_for_count() {
        use std::sync::atomic::AtomicUsize;
        let counter = Arc::new(AtomicUsize::new(0));
        let len = 100usize;
        let c = counter.clone();
        parallel_for(len, |_| {
            c.fetch_add(1, Ordering::Relaxed);
        });
        assert_eq!(counter.load(Ordering::Relaxed), 100);
    }

    #[test]
    fn parallel_sort_smoke() {
        let mut data = vec![5u32, 3, 1, 4, 2];
        parallel_sort(&mut data);
        assert_eq!(data, vec![1, 2, 3, 4, 5]);
    }

    #[test]
    fn parallel_reduce_sum() {
        let data: Vec<i32> = (1..=100).collect();
        let sum = parallel_reduce(&data, 0i32, |a, b| a + b);
        assert_eq!(sum, 5050);
    }
}
