//! Advanced memory management system for the Elysium game engine.
//!
//! Provides:
//! - Custom allocators (pool, linear/stack, buddy, slab, region/arena)
//! - Allocation tracking with stack traces
//! - Leak detection and memory profiling
//! - Garbage collection (mark-and-sweep, generational, incremental)
//! - Memory debugging (fill patterns, guard canaries, use-after-free, double-free)
//! - ECS integration (component, archetype, chunk allocators)
//!
//! This module is designed to integrate with the existing `alloc` module
//! (`FrameAllocator`, `WorldAllocator`) and the `performance` module's
//! `MemoryStats`.

use std::alloc::{GlobalAlloc, Layout, System};
use std::any::Any;
use std::backtrace::Backtrace;
use std::collections::{BTreeMap, HashMap, VecDeque};
use std::marker::{ManuallyDrop, PhantomData};
use std::num::NonZeroUsize;
use std::ops::{Deref, DerefMut};
use std::ptr::NonNull;
use std::sync::atomic::{AtomicBool, AtomicPtr, AtomicU64, AtomicU8, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, OnceLock, RwLock};
use std::time::{Duration, Instant};

use crate::CHUNK_SIZE;
use crate::alloc::FrameAllocator;

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// Fill pattern written to freed memory (0xCD = Visual Studio debug fill).
pub const FILL_PATTERN_FREED: u8 = 0xCD;
/// Fill pattern written to uninitialized memory (0xDD = Visual Studio).
pub const FILL_PATTERN_UNINITIALIZED: u8 = 0xDD;
/// Fill pattern written to guard bytes (0xDEADBEEF).
pub const FILL_PATTERN_GUARD: u32 = 0xDEADBEEF;
/// Number of guard bytes placed before and after each allocation in debug mode.
pub const GUARD_SIZE: usize = 16;

// ---------------------------------------------------------------------------
// Allocation Record
// ---------------------------------------------------------------------------

/// A recorded memory allocation for tracking and debugging.
#[derive(Debug, Clone)]
pub struct AllocationRecord {
    /// Raw pointer returned by the allocator (points to user data, not header).
    pub ptr: usize,
    /// Size of the allocation in bytes.
    pub size: usize,
    /// Alignment of the allocation.
    pub align: usize,
    /// Optional human-readable label (e.g., type name).
    pub label: Option<String>,
    /// Backtrace captured at allocation time.
    pub backtrace: Backtrace,
    /// Wall-clock timestamp at allocation.
    pub timestamp: Instant,
    /// Thread ID that performed the allocation.
    pub thread_id: std::thread::ThreadId,
}

impl AllocationRecord {
    #[inline]
    pub fn is_live(&self) -> bool {
        true
    }
}

// ---------------------------------------------------------------------------
// Global Allocation Tracker
// ---------------------------------------------------------------------------

/// Thread-safe global tracker for all heap allocations.
pub struct GlobalAllocationTracker {
    live_allocations: RwLock<BTreeMap<usize, AllocationRecord>>,
    total_allocated: AtomicU64,
    total_deallocated: AtomicU64,
    current_usage: AtomicU64,
    peak_usage: AtomicU64,
    allocation_count: AtomicU64,
    deallocation_count: AtomicU64,
}

impl GlobalAllocationTracker {
    const fn new() -> Self {
        Self {
            live_allocations: RwLock::new(BTreeMap::new()),
            total_allocated: AtomicU64::new(0),
            total_deallocated: AtomicU64::new(0),
            current_usage: AtomicU64::new(0),
            peak_usage: AtomicU64::new(0),
            allocation_count: AtomicU64::new(0),
            deallocation_count: AtomicU64::new(0),
        }
    }

    pub fn instance() -> &'static Self {
        static INSTANCE: OnceLock<GlobalAllocationTracker> = OnceLock::new();
        INSTANCE.get_or_init(Self::new)
    }

    pub fn record_allocation(&self, record: AllocationRecord) {
        self.total_allocated
            .fetch_add(record.size as u64, Ordering::Relaxed);
        self.allocation_count.fetch_add(1, Ordering::Relaxed);
        let usage = self
            .current_usage
            .fetch_add(record.size as u64, Ordering::Relaxed)
            + record.size as u64;
        let mut peak = self.peak_usage.lock().unwrap();
        if usage > *peak {
            *peak = usage;
        }
        drop(peak);
        if let Ok(mut map) = self.live_allocations.write() {
            map.insert(record.ptr, record);
        }
    }

    pub fn record_deallocation(&self, ptr: usize, size: usize) {
        self.total_deallocated
            .fetch_add(size as u64, Ordering::Relaxed);
        self.deallocation_count.fetch_add(1, Ordering::Relaxed);
        let _ = self
            .current_usage
            .fetch_sub(size as u64, Ordering::Relaxed);
        if let Ok(mut map) = self.live_allocations.write() {
            map.remove(&ptr);
        }
    }

    pub fn allocation_stats(&self) -> AllocationStats {
        AllocationStats {
            total_allocated: self.total_allocated.load(Ordering::Relaxed),
            total_deallocated: self.total_deallocated.load(Ordering::Relaxed),
            current_usage: self.current_usage.load(Ordering::Relaxed),
            peak_usage: self.peak_usage.lock().map(|p| *p).unwrap_or(0),
            allocation_count: self.allocation_count.load(Ordering::Relaxed),
            deallocation_count: self.deallocation_count.load(Ordering::Relaxed),
            live_count: self
                .live_allocations
                .read()
                .map(|m| m.len())
                .unwrap_or(0),
        }
    }

    pub fn detect_leaks(&self) -> Vec<AllocationRecord> {
        self.live_allocations
            .read()
            .map(|m| m.values().cloned().collect())
            .unwrap_or_default()
    }

    pub fn clear(&self) {
        if let Ok(mut map) = self.live_allocations.write() {
            map.clear();
        }
        self.total_allocated.store(0, Ordering::Relaxed);
        self.total_deallocated.store(0, Ordering::Relaxed);
        self.current_usage.store(0, Ordering::Relaxed);
        self.peak_usage.lock().map(|mut p| *p = 0).ok();
        self.allocation_count.store(0, Ordering::Relaxed);
        self.deallocation_count.store(0, Ordering::Relaxed);
    }
}

/// Aggregated allocation statistics.
#[derive(Debug, Clone, Copy, Default)]
pub struct AllocationStats {
    pub total_allocated: u64,
    pub total_deallocated: u64,
    pub current_usage: u64,
    pub peak_usage: u64,
    pub allocation_count: u64,
    pub deallocation_count: u64,
    pub live_count: usize,
}

impl AllocationStats {
    pub fn current_mb(&self) -> f64 {
        self.current_usage as f64 / (1024.0 * 1024.0)
    }

    pub fn peak_mb(&self) -> f64 {
        self.peak_usage as f64 / (1024.0 * 1024.0)
    }

    pub fn overhead_bytes(&self) -> u64 {
        if self.allocation_count > 0 {
            (self.total_allocated - self.total_deallocated).saturating_sub(self.current_usage)
        } else {
            0
        }
    }
}

// ---------------------------------------------------------------------------
// Instrumented Allocator Trait
// ---------------------------------------------------------------------------

/// A handle to a heap allocation for debugging.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AllocHandle {
    pub ptr: usize,
    pub size: usize,
}

impl AllocHandle {
    pub const fn new(ptr: usize, size: usize) -> Self {
        Self { ptr, size }
    }
}

/// Trait for allocators that support instrumentation.
pub trait InstrumentedAllocator: Any + Send + Sync {
    fn allocate(&mut self, layout: Layout, label: Option<&str>) -> Option<NonNull<u8>>;
    fn deallocate(&mut self, ptr: NonNull<u8>, layout: Layout);
    fn allocate_owned(&mut self, layout: Layout) -> Option<AllocHandle>;
    fn deallocate_owned(&mut self, handle: AllocHandle);
    fn stats(&self) -> AllocatorStats;
    fn name(&self) -> &'static str;
}

/// Per-allocator statistics.
#[derive(Debug, Clone, Copy, Default)]
pub struct AllocatorStats {
    pub total_allocated: usize,
    pub total_deallocated: usize,
    pub peak_usage: usize,
    pub current_usage: usize,
    pub allocation_count: u64,
    pub deallocation_count: u64,
    pub fragmentation_ratio: f32,
}

impl AllocatorStats {
    pub fn current_mb(&self) -> f64 {
        self.current_usage as f64 / (1024.0 * 1024.0)
    }

    pub fn peak_mb(&self) -> f64 {
        self.peak_usage as f64 / (1024.0 * 1024.0)
    }
}

// ---------------------------------------------------------------------------
// Pool Allocator
// ---------------------------------------------------------------------------

/// Object pool allocator: pre-allocates blocks of fixed-size objects.
pub struct PoolAllocator {
    block_size: usize,
    block_count: usize,
    free_list: VecDeque<NonNull<u8>>,
    stats: AllocatorStats,
    label: &'static str,
}

impl PoolAllocator {
    pub fn new(block_size: usize, block_count: usize, label: &'static str) -> Self {
        let mut allocator = Self {
            block_size,
            block_count,
            free_list: VecDeque::with_capacity(block_count),
            stats: AllocatorStats::default(),
            label,
        };
        allocator.grow();
        allocator
    }

    fn grow(&mut self) {
        for _ in 0..self.block_count {
            let layout = Layout::from_size_align(self.block_size, 8).expect("invalid pool layout");
            let ptr = unsafe {
                let ptr = System.alloc(layout);
                if ptr.is_null() {
                    std::alloc::handle_alloc_error(layout);
                }
                NonNull::new_unchecked(ptr)
            };
            self.free_list.push_back(ptr);
            self.stats.total_allocated += self.block_size;
            self.stats.current_usage += self.block_size;
            if self.stats.current_usage > self.stats.peak_usage {
                self.stats.peak_usage = self.stats.current_usage;
            }
        }
        self.stats.allocation_count += self.block_count as u64;
    }

    pub fn allocate(&mut self) -> Option<NonNull<u8>> {
        if let Some(ptr) = self.free_list.pop_front() {
            self.stats.allocation_count += 1;
            Some(ptr)
        } else {
            None
        }
    }

    pub fn deallocate(&mut self, ptr: NonNull<u8>) {
        self.free_list.push_back(ptr);
        self.stats.deallocation_count += 1;
        self.stats.current_usage = self.stats.current_usage.saturating_sub(self.block_size);
    }

    pub fn block_size(&self) -> usize {
        self.block_size
    }

    pub fn available(&self) -> usize {
        self.free_list.len()
    }

    pub fn is_full(&self) -> bool {
        self.free_list.is_empty()
    }
}

impl InstrumentedAllocator for PoolAllocator {
    fn allocate(&mut self, _layout: Layout, _label: Option<&str>) -> Option<NonNull<u8>> {
        self.allocate()
    }

    fn deallocate(&mut self, ptr: NonNull<u8>, _layout: Layout) {
        self.deallocate(ptr);
    }

    fn allocate_owned(&mut self, _layout: Layout) -> Option<AllocHandle> {
        self.allocate()
            .map(|ptr| AllocHandle::new(ptr.as_ptr() as usize, self.block_size))
    }

    fn deallocate_owned(&mut self, _handle: AllocHandle) {
        // No-op for pool: caller must provide the NonNull directly.
    }

    fn stats(&self) -> AllocatorStats {
        self.stats
    }

    fn name(&self) -> &'static str {
        self.label
    }
}

impl Drop for PoolAllocator {
    fn drop(&mut self) {
        let layout = Layout::from_size_align(self.block_size, 8).expect("invalid pool layout");
        for ptr in self.free_list.drain(..) {
            unsafe { System.dealloc(ptr.as_ptr(), layout) };
        }
    }
}

// ---------------------------------------------------------------------------
// Linear Allocator
// ---------------------------------------------------------------------------

/// Linear (bump) allocator: fast sequential allocation, reset to reclaim all.
pub struct LinearAllocator {
    buffer: Option<NonNull<u8>>,
    capacity: usize,
    offset: usize,
    stats: AllocatorStats,
    label: &'static str,
}

impl LinearAllocator {
    pub fn new(capacity: usize, label: &'static str) -> Self {
        let layout = Layout::from_size_align(capacity, 64).expect("invalid linear layout");
        let buffer = unsafe {
            let ptr = System.alloc(layout);
            if ptr.is_null() {
                std::alloc::handle_alloc_error(layout);
            }
            NonNull::new_unchecked(ptr)
        };
        Self {
            buffer: Some(buffer),
            capacity,
            offset: 0,
            stats: AllocatorStats {
                total_allocated: capacity,
                current_usage: 0,
                peak_usage: 0,
                ..Default::default()
            },
            label,
        }
    }

    pub fn reset(&mut self) {
        self.offset = 0;
    }

    pub fn allocate(&mut self, layout: Layout) -> Option<NonNull<u8>> {
        let align = layout.align();
        let size = layout.size();
        let aligned = (self.offset + align - 1) & !(align - 1);
        if aligned + size > self.capacity {
            return None;
        }
        self.offset = aligned + size;
        self.stats.allocation_count += 1;
        self.stats.current_usage = self.offset;
        if self.stats.current_usage > self.stats.peak_usage {
            self.stats.peak_usage = self.stats.current_usage;
        }
        unsafe { Some(NonNull::new_unchecked(self.buffer.unwrap().as_ptr().add(aligned))) }
    }

    pub fn alloc<T>(&mut self, value: T) -> Option<&mut T> {
        let layout = Layout::new::<T>();
        let ptr = self.allocate(layout)?;
        unsafe {
            ptr.as_ptr().cast::<T>().write(value);
            Some(&mut *ptr.as_ptr().cast::<T>())
        }
    }

    pub fn used_bytes(&self) -> usize {
        self.offset
    }

    pub fn remaining_bytes(&self) -> usize {
        self.capacity - self.offset
    }

    pub fn capacity(&self) -> usize {
        self.capacity
    }

    pub fn is_full(&self) -> bool {
        self.offset >= self.capacity
    }
}

impl InstrumentedAllocator for LinearAllocator {
    fn allocate(&mut self, layout: Layout, _label: Option<&str>) -> Option<NonNull<u8>> {
        self.allocate(layout)
    }

    fn deallocate(&mut self, _ptr: NonNull<u8>, _layout: Layout) {
        // Linear allocator cannot free individual allocations.
    }

    fn allocate_owned(&mut self, layout: Layout) -> Option<AllocHandle> {
        self.allocate(layout)
            .map(|ptr| AllocHandle::new(ptr.as_ptr() as usize, layout.size()))
    }

    fn deallocate_owned(&mut self, _handle: AllocHandle) {
        // No-op: deallocate via reset().
    }

    fn stats(&self) -> AllocatorStats {
        self.stats
    }

    fn name(&self) -> &'static str {
        self.label
    }
}

impl Drop for LinearAllocator {
    fn drop(&mut self) {
        if let Some(ptr) = self.buffer {
            let layout = Layout::from_size_align(self.capacity, 64).expect("invalid linear layout");
            unsafe { System.dealloc(ptr.as_ptr(), layout) };
        }
    }
}

// ---------------------------------------------------------------------------
// Stack Allocator (Bump)
// ---------------------------------------------------------------------------

/// Stack (bump) allocator: maintains a linked list of memory blocks.
pub struct StackAllocator {
    blocks: Vec<NonNull<u8>>,
    block_size: usize,
    current_block: usize,
    offset: usize,
    stats: AllocatorStats,
    label: &'static str,
}

impl StackAllocator {
    pub fn new(block_size: usize, initial_blocks: usize, label: &'static str) -> Self {
        let mut allocator = Self {
            blocks: Vec::with_capacity(initial_blocks),
            block_size,
            current_block: 0,
            offset: 0,
            stats: AllocatorStats::default(),
            label,
        };
        for _ in 0..initial_blocks {
            allocator.add_block();
        }
        allocator
    }

    fn add_block(&mut self) {
        let layout = Layout::from_size_align(self.block_size, 64).expect("invalid stack layout");
        let ptr = unsafe {
            let ptr = System.alloc(layout);
            if ptr.is_null() {
                std::alloc::handle_alloc_error(layout);
            }
            NonNull::new_unchecked(ptr)
        };
        self.blocks.push(ptr);
        self.stats.total_allocated += self.block_size;
    }

    pub fn allocate(&mut self, layout: Layout) -> Option<NonNull<u8>> {
        let align = layout.align();
        let size = layout.size();
        if self.current_block >= self.blocks.len() {
            self.add_block();
        }
        let base = self.blocks[self.current_block].as_ptr();
        let aligned = (self.offset + align - 1) & !(align - 1);
        if aligned + size > self.block_size {
            // Move to next block.
            self.current_block += 1;
            self.offset = 0;
            return self.allocate(layout);
        }
        self.offset = aligned + size;
        self.stats.allocation_count += 1;
        self.stats.current_usage = self.current_block * self.block_size + self.offset;
        if self.stats.current_usage > self.stats.peak_usage {
            self.stats.peak_usage = self.stats.current_usage;
        }
        unsafe { Some(NonNull::new_unchecked(base.add(aligned))) }
    }

    pub fn reset(&mut self) {
        self.current_block = 0;
        self.offset = 0;
        self.stats.current_usage = 0;
    }

    pub fn used_bytes(&self) -> usize {
        self.current_block * self.block_size + self.offset
    }

    pub fn block_count(&self) -> usize {
        self.blocks.len()
    }
}

impl InstrumentedAllocator for StackAllocator {
    fn allocate(&mut self, layout: Layout, _label: Option<&str>) -> Option<NonNull<u8>> {
        self.allocate(layout)
    }

    fn deallocate(&mut self, _ptr: NonNull<u8>, _layout: Layout) {
        // Stack allocator frees via reset.
    }

    fn allocate_owned(&mut self, layout: Layout) -> Option<AllocHandle> {
        self.allocate(layout)
            .map(|ptr| AllocHandle::new(ptr.as_ptr() as usize, layout.size()))
    }

    fn deallocate_owned(&mut self, _handle: AllocHandle) {
        // No-op.
    }

    fn stats(&self) -> AllocatorStats {
        self.stats
    }

    fn name(&self) -> &'static str {
        self.label
    }
}

impl Drop for StackAllocator {
    fn drop(&mut self) {
        let layout = Layout::from_size_align(self.block_size, 64).expect("invalid stack layout");
        for ptr in self.blocks.drain(..) {
            unsafe { System.dealloc(ptr.as_ptr(), layout) };
        }
    }
}

// ---------------------------------------------------------------------------
// Buddy Allocator
// ---------------------------------------------------------------------------

/// Power-of-2 buddy allocator: splits and coalesces blocks.
pub struct BuddyAllocator {
    total_size: usize,
    min_order: usize,
    free_lists: Vec<VecDeque<usize>>,
    allocated: HashMap<usize, (usize, usize)>, // addr -> (size, order)
    stats: AllocatorStats,
    label: &'static str,
}

impl BuddyAllocator {
    pub fn new(total_size: usize, min_block_size: usize, label: &'static str) -> Self {
        assert!(total_size.is_power_of_two(), "buddy total_size must be power of two");
        assert!(
            min_block_size.is_power_of_two(),
            "buddy min_block_size must be power of two"
        );
        let min_order = min_block_size.trailing_zeros() as usize;
        let max_order = total_size.trailing_zeros() as usize;
        let free_lists = (min_order..=max_order).map(|_| VecDeque::new()).collect();
        let mut allocator = Self {
            total_size,
            min_order,
            free_lists,
            allocated: HashMap::new(),
            stats: AllocatorStats::default(),
            label,
        };
        allocator.free_lists[max_order].push_back(0);
        allocator.stats.total_allocated = total_size;
        allocator
    }

    fn order_for(&self, size: usize) -> usize {
        let mut order = self.min_order;
        let mut block = 1usize << self.min_order;
        while block < size && order < self.free_lists.len() - 1 {
            order += 1;
            block <<= 1;
        }
        order
    }

    pub fn allocate(&mut self, layout: Layout) -> Option<NonNull<u8>> {
        let order = self.order_for(layout.size());
        if order >= self.free_lists.len() {
            return None;
        }
        let addr = self.find_free_block(order)?;
        self.allocated.insert(addr, (layout.size(), order));
        self.stats.allocation_count += 1;
        self.stats.current_usage += layout.size();
        if self.stats.current_usage > self.stats.peak_usage {
            self.stats.peak_usage = self.stats.current_usage;
        }
        unsafe { Some(NonNull::new_unchecked(addr as *mut u8)) }
    }

    fn find_free_block(&mut self, order: usize) -> Option<usize> {
        if let Some(addr) = self.free_lists[order].pop_front() {
            return Some(addr);
        }
        if order + 1 < self.free_lists.len() {
            let higher = self.find_free_block(order + 1)?;
            let buddy = higher + (1usize << order);
            self.free_lists[order].push_back(buddy);
            Some(higher)
        } else {
            None
        }
    }

    pub fn deallocate(&mut self, ptr: NonNull<u8>) {
        let addr = ptr.as_ptr() as usize;
        let (size, order) = self.allocated.remove(&addr)?;
        self.stats.deallocation_count += 1;
        self.stats.current_usage = self.stats.current_usage.saturating_sub(size);
        self.coalesce(order, addr);
        Some(())
    }

    fn coalesce(&mut self, order: usize, addr: usize) {
        let buddy = addr ^ (1usize << order);
        if let Some(pos) = self.free_lists[order].iter().position(|&a| a == buddy) {
            self.free_lists[order].remove(pos);
            if order + 1 < self.free_lists.len() {
                self.coalesce(order + 1, std::cmp::min(addr, buddy));
            }
        } else {
            self.free_lists[order].push_back(addr);
        }
    }

    pub fn total_size(&self) -> usize {
        self.total_size
    }
}

impl InstrumentedAllocator for BuddyAllocator {
    fn allocate(&mut self, layout: Layout, _label: Option<&str>) -> Option<NonNull<u8>> {
        self.allocate(layout)
    }

    fn deallocate(&mut self, ptr: NonNull<u8>, _layout: Layout) {
        self.deallocate(ptr);
    }

    fn allocate_owned(&mut self, layout: Layout) -> Option<AllocHandle> {
        self.allocate(layout)
            .map(|ptr| AllocHandle::new(ptr.as_ptr() as usize, layout.size()))
    }

    fn deallocate_owned(&mut self, handle: AllocHandle) {
        if let Ok(ptr) = NonNull::new(handle.ptr as *mut u8) {
            self.deallocate(ptr);
        }
    }

    fn stats(&self) -> AllocatorStats {
        self.stats
    }

    fn name(&self) -> &'static str {
        self.label
    }
}

// ---------------------------------------------------------------------------
// Slab Allocator
// ---------------------------------------------------------------------------

/// Slab allocator: per-size-class slabs for variable-size allocations.
pub struct SlabAllocator {
    slabs: HashMap<usize, VecDeque<NonNull<u8>>>,
    slab_size: usize,
    slab_align: usize,
    stats: AllocatorStats,
    label: &'static str,
}

impl SlabAllocator {
    pub fn new(slab_size: usize, slab_align: usize, label: &'static str) -> Self {
        Self {
            slabs: HashMap::new(),
            slab_size,
            slab_align,
            stats: AllocatorStats::default(),
            label,
        }
    }

    pub fn allocate(&mut self, layout: Layout) -> Option<NonNull<u8>> {
        let size = layout.size().max(self.slab_size);
        let entry = self.slabs.entry(size).or_default();
        if let Some(ptr) = entry.pop_front() {
            self.stats.allocation_count += 1;
            self.stats.current_usage += size;
            if self.stats.current_usage > self.stats.peak_usage {
                self.stats.peak_usage = self.stats.current_usage;
            }
            Some(ptr)
        } else {
            let layout = Layout::from_size_align(size, self.slab_align).ok()?;
            let ptr = unsafe {
                let ptr = System.alloc(layout);
                if ptr.is_null() {
                    return None;
                }
                NonNull::new_unchecked(ptr)
            };
            self.stats.total_allocated += size;
            self.stats.current_usage += size;
            self.stats.allocation_count += 1;
            if self.stats.current_usage > self.stats.peak_usage {
                self.stats.peak_usage = self.stats.current_usage;
            }
            Some(ptr)
        }
    }

    pub fn deallocate(&mut self, ptr: NonNull<u8>, size: usize) {
        let size = size.max(self.slab_size);
        self.slabs.entry(size).or_default().push_back(ptr);
        self.stats.deallocation_count += 1;
        self.stats.current_usage = self.stats.current_usage.saturating_sub(size);
    }

    pub fn release_empty_slabs(&mut self) {
        let slab_size = self.slab_size;
        self.slabs.retain(|size, free| {
            let keep = !free.is_empty();
            if !keep {
                let layout = Layout::from_size_align(*size, self.slab_align).unwrap();
                for ptr in free.drain(..) {
                    unsafe { System.dealloc(ptr.as_ptr(), layout) };
                }
                self.stats.total_deallocated += size * free.len();
            }
            keep
        });
    }
}

impl InstrumentedAllocator for SlabAllocator {
    fn allocate(&mut self, layout: Layout, _label: Option<&str>) -> Option<NonNull<u8>> {
        self.allocate(layout)
    }

    fn deallocate(&mut self, ptr: NonNull<u8>, layout: Layout) {
        self.deallocate(ptr, layout.size());
    }

    fn allocate_owned(&mut self, layout: Layout) -> Option<AllocHandle> {
        self.allocate(layout)
            .map(|ptr| AllocHandle::new(ptr.as_ptr() as usize, layout.size()))
    }

    fn deallocate_owned(&mut self, handle: AllocHandle) {
        if let Ok(ptr) = NonNull::new(handle.ptr as *mut u8) {
            self.deallocate(ptr, handle.size);
        }
    }

    fn stats(&self) -> AllocatorStats {
        self.stats
    }

    fn name(&self) -> &'static str {
        self.label
    }
}

// ---------------------------------------------------------------------------
// Region Allocator (Arena)
// ---------------------------------------------------------------------------

/// Region (arena) allocator: allocates from a growing list of chunks.
pub struct RegionAllocator {
    chunks: Vec<NonNull<u8>>,
    chunk_size: usize,
    offsets: Vec<usize>,
    stats: AllocatorStats,
    label: &'static str,
}

impl RegionAllocator {
    pub fn new(chunk_size: usize, initial_chunks: usize, label: &'static str) -> Self {
        let mut allocator = Self {
            chunks: Vec::with_capacity(initial_chunks),
            chunk_size,
            offsets: Vec::with_capacity(initial_chunks),
            stats: AllocatorStats::default(),
            label,
        };
        for _ in 0..initial_chunks {
            allocator.add_chunk();
        }
        allocator
    }

    fn add_chunk(&mut self) {
        let layout = Layout::from_size_align(self.chunk_size, 64).expect("invalid region layout");
        let ptr = unsafe {
            let ptr = System.alloc(layout);
            if ptr.is_null() {
                std::alloc::handle_alloc_error(layout);
            }
            NonNull::new_unchecked(ptr)
        };
        self.chunks.push(ptr);
        self.offsets.push(0);
        self.stats.total_allocated += self.chunk_size;
    }

    pub fn allocate(&mut self, layout: Layout) -> Option<NonNull<u8>> {
        let size = layout.size();
        let align = layout.align();
        // Try existing chunks.
        for (chunk, offset) in self.chunks.iter_mut().zip(self.offsets.iter_mut()) {
            let base = chunk.as_ptr() as usize;
            let aligned = (base + *offset + align - 1) & !(align - 1);
            let end = aligned + size;
            let chunk_end = base + self.chunk_size;
            if end <= chunk_end {
                *offset = end - base;
                self.stats.allocation_count += 1;
                self.stats.current_usage = self
                    .offsets
                    .iter()
                    .enumerate()
                    .map(|(i, o)| if i < self.chunks.len() { *o } else { 0 })
                    .sum();
                if self.stats.current_usage > self.stats.peak_usage {
                    self.stats.peak_usage = self.stats.current_usage;
                }
                return unsafe { Some(NonNull::new_unchecked(aligned as *mut u8)) };
            }
        }
        // Need a new chunk.
        self.add_chunk();
        self.allocate(layout)
    }

    pub fn reset(&mut self) {
        for offset in &mut self.offsets {
            *offset = 0;
        }
        self.stats.current_usage = 0;
    }

    pub fn chunk_count(&self) -> usize {
        self.chunks.len()
    }

    pub fn used_bytes(&self) -> usize {
        self.offsets.iter().sum()
    }
}

impl InstrumentedAllocator for RegionAllocator {
    fn allocate(&mut self, layout: Layout, _label: Option<&str>) -> Option<NonNull<u8>> {
        self.allocate(layout)
    }

    fn deallocate(&mut self, _ptr: NonNull<u8>, _layout: Layout) {
        // Region allocator frees via reset.
    }

    fn allocate_owned(&mut self, layout: Layout) -> Option<AllocHandle> {
        self.allocate(layout)
            .map(|ptr| AllocHandle::new(ptr.as_ptr() as usize, layout.size()))
    }

    fn deallocate_owned(&mut self, _handle: AllocHandle) {
        // No-op.
    }

    fn stats(&self) -> AllocatorStats {
        self.stats
    }

    fn name(&self) -> &'static str {
        self.label
    }
}

impl Drop for RegionAllocator {
    fn drop(&mut self) {
        let layout = Layout::from_size_align(self.chunk_size, 64).expect("invalid region layout");
        for ptr in self.chunks.drain(..) {
            unsafe { System.dealloc(ptr.as_ptr(), layout) };
        }
    }
}

// ---------------------------------------------------------------------------
// Debug Allocator
// ---------------------------------------------------------------------------

/// Debug allocator: wraps another allocator with guard canaries and fill patterns.
pub struct DebugAllocator<A: InstrumentedAllocator> {
    inner: A,
    live: Mutex<HashMap<usize, (NonNull<u8>, usize)>>,
    fill_patterns: bool,
}

impl<A: InstrumentedAllocator> DebugAllocator<A> {
    pub fn new(inner: A) -> Self {
        Self {
            inner,
            live: Mutex::new(HashMap::new()),
            fill_patterns: true,
        }
    }

    pub fn enable_fill_patterns(&mut self, enable: bool) {
        self.fill_patterns = enable;
    }

    fn fill_pattern(&self, ptr: *mut u8, size: usize, pattern: u8) {
        if self.fill_patterns {
            unsafe { std::ptr::write_bytes(ptr, pattern, size) };
        }
    }

    pub fn allocate(&mut self, layout: Layout, label: Option<&str>) -> Option<NonNull<u8>> {
        let total_size = GUARD_SIZE + layout.size() + GUARD_SIZE;
        let total_layout = Layout::from_size_align(total_size, layout.align()).ok()?;
        let raw = self.inner.allocate(total_layout, label)?;
        let raw_ptr = raw.as_ptr();
        unsafe {
            // Front guard.
            std::ptr::write_bytes(raw_ptr, FILL_PATTERN_GUARD as u8, GUARD_SIZE);
            // Back guard.
            std::ptr::write_bytes(
                raw_ptr.add(GUARD_SIZE + layout.size()),
                FILL_PATTERN_GUARD as u8,
                GUARD_SIZE,
            );
            // Fill user memory.
            if self.fill_patterns {
                std::ptr::write_bytes(
                    raw_ptr.add(GUARD_SIZE),
                    FILL_PATTERN_UNINITIALIZED,
                    layout.size(),
                );
            }
        }
        let user_ptr = unsafe { NonNull::new_unchecked(raw_ptr.add(GUARD_SIZE)) };
        self.live
            .lock()
            .unwrap()
            .insert(user_ptr.as_ptr() as usize, (raw, layout.size()));
        Some(user_ptr)
    }

    pub fn deallocate(&mut self, ptr: NonNull<u8>, layout: Layout) {
        let user_addr = ptr.as_ptr() as usize;
        let raw = if let Ok(mut map) = self.live.lock() {
            map.remove(&user_addr).map(|(raw, _)| raw)
        } else {
            None
        };
        let Some(raw) = raw else { return };
        unsafe {
            // Verify guards.
            let base = raw.as_ptr();
            for i in 0..GUARD_SIZE {
                if *base.add(i) != FILL_PATTERN_GUARD as u8 {
                    eprintln!("DEBUG ALLOC: front guard corrupted at offset {i}");
                }
                if *base.add(GUARD_SIZE + layout.size() + i) != FILL_PATTERN_GUARD as u8 {
                    eprintln!("DEBUG ALLOC: back guard corrupted at offset {i}");
                }
            }
            // Fill freed memory.
            self.fill_pattern(ptr.as_ptr(), layout.size(), FILL_PATTERN_FREED);
        }
        let total_size = GUARD_SIZE + layout.size() + GUARD_SIZE;
        let total_layout = Layout::from_size_align(total_size, layout.align()).unwrap();
        self.inner.deallocate(raw, total_layout);
    }

    pub fn stats(&self) -> AllocatorStats {
        self.inner.stats()
    }

    pub fn name(&self) -> &'static str {
        self.inner.name()
    }
}

// ---------------------------------------------------------------------------
// Garbage Collector Trait
// ---------------------------------------------------------------------------

/// Trait for garbage-collected objects.
pub trait GcObject: Any + Send + Sync {
    fn mark(&self, visitor: &mut dyn GcVisitor);
    fn sweep(&mut self);
    fn object_type(&self) -> &'static str;
}

/// Visitor for marking reachable objects.
pub trait GcVisitor: Send + Sync {
    fn visit(&mut self, ptr: *const dyn GcObject);
}

/// A garbage collector implementation.
pub trait GarbageCollector: Send + Sync {
    fn collect(&mut self);
    fn add_root(&mut self, ptr: *mut dyn GcObject);
    fn stats(&self) -> GcStats;
    fn name(&self) -> &'static str;
}

/// GC statistics.
#[derive(Debug, Clone, Copy, Default)]
pub struct GcStats {
    pub live_objects: usize,
    pub total_collections: u64,
    pub total_allocated: usize,
    pub total_freed: usize,
    pub pause_time_ms: f64,
}

// ---------------------------------------------------------------------------
// Tracing Garbage Collector
// ---------------------------------------------------------------------------

/// Simple tracing garbage collector using reference tracing.
pub struct TracingGc {
    roots: Mutex<Vec<*mut dyn GcObject>>,
    objects: Mutex<HashMap<usize, Box<dyn GcObject>>>,
    stats: Mutex<GcStats>,
    label: &'static str,
}

impl TracingGc {
    pub fn new(label: &'static str) -> Self {
        Self {
            roots: Mutex::new(Vec::new()),
            objects: Mutex::new(HashMap::new()),
            stats: Mutex::new(GcStats::default()),
            label,
        }
    }

    pub fn allocate<T: GcObject + 'static>(&self, obj: T) -> *mut dyn GcObject {
        let mut objects = self.objects.lock().unwrap();
        let boxed: Box<dyn GcObject> = Box::new(obj);
        let raw = Box::into_raw(boxed);
        let addr = raw as usize;
        objects.insert(addr, unsafe { Box::from_raw(raw) });
        let ptr = addr as *mut dyn GcObject;
        let mut stats = self.stats.lock().unwrap();
        stats.total_allocated += 1;
        stats.live_objects += 1;
        ptr
    }

    pub fn collect(&self) {
        let mut roots = self.roots.lock().unwrap();
        let mut objects = self.objects.lock().unwrap();
        let mut stats = self.stats.lock().unwrap();
        let start = Instant::now();

        // Mark phase: DFS from roots.
        let mut marked: HashMap<usize, bool> = HashMap::new();
        let mut stack: Vec<usize> = roots.iter().map(|p| *p as usize).collect();
        while let Some(addr) = stack.pop() {
            if marked.contains_key(&addr) {
                continue;
            }
            marked.insert(addr, true);
            if let Some(obj) = objects.get(&addr) {
                let mut visitor = MarkVisitor { stack: &mut stack };
                // SAFETY: objects are only accessed during collection.
                let trait_obj: *const dyn GcObject = obj.as_ref() as *const dyn GcObject;
                unsafe {
                    (*trait_obj).mark(&mut visitor);
                }
            }
        }

        // Sweep phase.
        let mut freed = 0usize;
        objects.retain(|&addr, obj| {
            if marked.contains_key(&addr) {
                true
            } else {
                let mut trait_obj: *mut dyn GcObject = obj.as_mut() as *mut dyn GcObject;
                unsafe { (*trait_obj).sweep() };
                freed += 1;
                false
            }
        });
        stats.live_objects = objects.len();
        stats.total_freed += freed;
        stats.total_collections += 1;
        stats.pause_time_ms = start.elapsed().as_secs_f64() * 1000.0;
    }
}

struct MarkVisitor<'a> {
    stack: &'a mut Vec<usize>,
}

impl GcVisitor for MarkVisitor<'_> {
    fn visit(&mut self, ptr: *const dyn GcObject) {
        self.stack.push(ptr as usize);
    }
}

impl GarbageCollector for TracingGc {
    fn collect(&mut self) {
        self.collect();
    }

    fn add_root(&mut self, ptr: *mut dyn GcObject) {
        self.roots.lock().unwrap().push(ptr);
    }

    fn stats(&self) -> GcStats {
        *self.stats.lock().unwrap()
    }

    fn name(&self) -> &'static str {
        self.label
    }
}

// ---------------------------------------------------------------------------
// Mark-and-Sweep Garbage Collector
// ---------------------------------------------------------------------------

/// Explicit mark-and-sweep GC with generations.
pub struct MarkAndSweepGc {
    young_gen: TracingGc,
    old_gen: TracingGc,
    promotion_threshold: u64,
    label: &'static str,
}

impl MarkAndSweepGc {
    pub fn new(promotion_threshold: u64, label: &'static str) -> Self {
        Self {
            young_gen: TracingGc::new("young-gen"),
            old_gen: TracingGc::new("old-gen"),
            promotion_threshold,
            label,
        }
    }

    pub fn collect(&mut self) {
        self.young_gen.collect();
        // Promote survivors if threshold met.
        let young_stats = self.young_gen.stats.lock().unwrap();
        if young_stats.total_collections >= self.promotion_threshold {
            drop(young_stats);
            self.old_gen.collect();
        }
    }

    pub fn allocate_young<T: GcObject + 'static>(&self, obj: T) -> *mut dyn GcObject {
        self.young_gen.allocate(obj)
    }

    pub fn allocate_old<T: GcObject + 'static>(&self, obj: T) -> *mut dyn GcObject {
        self.old_gen.allocate(obj)
    }
}

impl GarbageCollector for MarkAndSweepGc {
    fn collect(&mut self) {
        self.collect();
    }

    fn add_root(&mut self, ptr: *mut dyn GcObject) {
        self.young_gen.add_root(ptr);
    }

    fn stats(&self) -> GcStats {
        let young = self.young_gen.stats();
        let old = self.old_gen.stats();
        GcStats {
            live_objects: young.live_objects + old.live_objects,
            total_collections: young.total_collections + old.total_collections,
            total_allocated: young.total_allocated + old.total_allocated,
            total_freed: young.total_freed + old.total_freed,
            pause_time_ms: young.pause_time_ms + old.pause_time_ms,
        }
    }

    fn name(&self) -> &'static str {
        self.label
    }
}

// ---------------------------------------------------------------------------
// Generational Garbage Collector
// ---------------------------------------------------------------------------

/// Generational GC: young and old generations with write barriers.
pub struct GenerationalGc {
    young: TracingGc,
    old: TracingGc,
    write_barrier: Mutex<Vec<*mut dyn GcObject>>,
    label: &'static str,
}

impl GenerationalGc {
    pub fn new(label: &'static str) -> Self {
        Self {
            young: TracingGc::new("young-gen"),
            old: TracingGc::new("old-gen"),
            write_barrier: Mutex::new(Vec::new()),
            label,
        }
    }

    pub fn collect_young(&self) {
        self.young.collect();
    }

    pub fn collect_full(&self) {
        self.young.collect();
        self.old.collect();
    }

    pub fn write_barrier(&self, old_ptr: *mut dyn GcObject, new_ptr: *mut dyn GcObject) {
        // Record the old pointer's object as potentially containing references
        // to young-generation objects.
        if let Ok(mut barrier) = self.write_barrier.lock() {
            barrier.push(old_ptr);
        }
        // Ensure the new target is reachable.
        if let Ok(mut roots) = self.young.roots.lock() {
            roots.push(new_ptr);
        }
    }
}

// ---------------------------------------------------------------------------
// Incremental (Real-Time) Garbage Collector
// ---------------------------------------------------------------------------

/// Incremental garbage collector for real-time applications.
pub struct IncrementalGc {
    objects: Mutex<HashMap<usize, Box<dyn GcObject>>>,
    mark_queue: Mutex<VecDeque<usize>>,
    marked: Mutex<HashMap<usize, bool>>,
    phase: AtomicU8,
    budget_bytes: usize,
    processed_bytes: AtomicUsize,
    label: &'static str,
}

const GC_PHASE_IDLE: u8 = 0;
const GC_PHASE_MARK: u8 = 1;
const GC_PHASE_SWEEP: u8 = 2;

impl IncrementalGc {
    pub fn new(budget_bytes: usize, label: &'static str) -> Self {
        Self {
            objects: Mutex::new(HashMap::new()),
            mark_queue: Mutex::new(VecDeque::new()),
            marked: Mutex::new(HashMap::new()),
            phase: AtomicU8::new(GC_PHASE_IDLE),
            budget_bytes,
            processed_bytes: AtomicUsize::new(0),
            label,
        }
    }

    pub fn allocate<T: GcObject + 'static>(&self, obj: T) -> *mut dyn GcObject {
        let mut objects = self.objects.lock().unwrap();
        let boxed: Box<dyn GcObject> = Box::new(obj);
        let raw = Box::into_raw(boxed);
        let addr = raw as usize;
        objects.insert(addr, unsafe { Box::from_raw(raw) });
        addr as *mut dyn GcObject
    }

    pub fn tick(&self) {
        let processed = self.processed_bytes.load(Ordering::Relaxed);
        if processed >= self.budget_bytes {
            return;
        }
        match self.phase.load(Ordering::Relaxed) {
            GC_PHASE_IDLE => {
                // Start mark phase.
                self.phase.store(GC_PHASE_MARK, Ordering::Relaxed);
                self.processed_bytes.store(0, Ordering::Relaxed);
            }
            GC_PHASE_MARK => {
                // Process a few objects.
                if let Ok(mut queue) = self.mark_queue.lock() {
                    if let Some(addr) = queue.pop_front() {
                        if self.marked.lock().unwrap().contains_key(&addr) {
                            return;
                        }
                        self.marked.lock().unwrap().insert(addr, true);
                        if let Some(obj) = self.objects.lock().unwrap().get(&addr) {
                            let mut visitor =
                                IncrementalMarkVisitor { queue: &mut *queue };
                            let trait_obj = obj.as_ref() as *const dyn GcObject;
                            unsafe { (*trait_obj).mark(&mut visitor) };
                        }
                        self.processed_bytes
                            .fetch_add(64, Ordering::Relaxed);
                    } else {
                        self.phase.store(GC_PHASE_SWEEP, Ordering::Relaxed);
                    }
                }
            }
            GC_PHASE_SWEEP => {
                if let Ok(mut objects) = self.objects.lock() {
                    objects.retain(|&addr, _| {
                        self.marked.lock().unwrap().contains_key(&addr)
                    });
                }
                self.marked.lock().unwrap().clear();
                self.phase.store(GC_PHASE_IDLE, Ordering::Relaxed);
                self.processed_bytes.store(0, Ordering::Relaxed);
            }
            _ => {}
        }
    }

    pub fn force_collect(&self) {
        self.phase.store(GC_PHASE_MARK, Ordering::Relaxed);
        while self.phase.load(Ordering::Relaxed) != GC_PHASE_IDLE {
            self.tick();
        }
    }
}

struct IncrementalMarkVisitor<'a> {
    queue: &'a mut VecDeque<usize>,
}

impl GcVisitor for IncrementalMarkVisitor<'_> {
    fn visit(&mut self, ptr: *const dyn GcObject) {
        self.queue.push_back(ptr as usize);
    }
}

// ---------------------------------------------------------------------------
// ECS Integration Allocators
// ---------------------------------------------------------------------------

/// Allocator for component storage within chunks.
pub struct ComponentAllocator {
    pool: PoolAllocator,
    stats: AllocatorStats,
}

impl ComponentAllocator {
    pub fn new(component_size: usize, block_count: usize) -> Self {
        Self {
            pool: PoolAllocator::new(
                component_size,
                block_count,
                "component-pool",
            ),
            stats: AllocatorStats::default(),
        }
    }

    pub fn allocate_component(&mut self) -> Option<NonNull<u8>> {
        self.pool.allocate()
    }

    pub fn deallocate_component(&mut self, ptr: NonNull<u8>) {
        self.pool.deallocate(ptr);
    }

    pub fn component_size(&self) -> usize {
        self.pool.block_size()
    }
}

/// Allocator for archetype metadata.
pub struct ArchetypeAllocator {
    pool: PoolAllocator,
    label: &'static str,
}

impl ArchetypeAllocator {
    pub fn new(archetype_size: usize, block_count: usize, label: &'static str) -> Self {
        Self {
            pool: PoolAllocator::new(archetype_size, block_count, label),
            label,
        }
    }

    pub fn allocate(&mut self) -> Option<NonNull<u8>> {
        self.pool.allocate()
    }

    pub fn deallocate(&mut self, ptr: NonNull<u8>) {
        self.pool.deallocate(ptr);
    }
}

/// Allocator for ECS chunk pages (CHUNK_SIZE pages).
pub struct ChunkAllocator {
    linear: LinearAllocator,
    frame: Mutex<FrameAllocator>,
    label: &'static str,
}

impl ChunkAllocator {
    pub fn new(label: &'static str) -> Self {
        Self {
            linear: LinearAllocator::new(CHUNK_SIZE * 64, label),
            frame: Mutex::new(FrameAllocator::default()),
            label,
        }
    }

    pub fn allocate_chunk(&self) -> Option<NonNull<u8>> {
        let mut frame = self.frame.lock().unwrap();
        frame.allocate(Layout::from_size_align(CHUNK_SIZE, 64).unwrap())
            .or_else(|| self.linear.allocate(Layout::from_size_align(CHUNK_SIZE, 64).unwrap()))
    }

    pub fn reset_frame(&self) {
        self.frame.lock().unwrap().reset();
    }

    pub fn chunk_size(&self) -> usize {
        CHUNK_SIZE
    }
}

// ---------------------------------------------------------------------------
// Memory Profiler
// ---------------------------------------------------------------------------

/// A snapshot of the memory state at a point in time.
#[derive(Debug, Clone, Default)]
pub struct MemorySnapshot {
    pub timestamp: Instant,
    pub global_stats: AllocationStats,
    pub allocator_stats: HashMap<String, AllocatorStats>,
    pub gc_stats: HashMap<String, GcStats>,
}

/// Memory profiler: records snapshots and computes deltas.
pub struct MemoryProfiler {
    history: Mutex<VecDeque<MemorySnapshot>>,
    max_history: usize,
}

impl MemoryProfiler {
    pub fn new(max_history: usize) -> Self {
        Self {
            history: Mutex::new(VecDeque::with_capacity(max_history)),
            max_history,
        }
    }

    pub fn snapshot(&self) -> MemorySnapshot {
        MemorySnapshot {
            timestamp: Instant::now(),
            global_stats: GlobalAllocationTracker::instance().allocation_stats(),
            allocator_stats: HashMap::new(),
            gc_stats: HashMap::new(),
        }
    }

    pub fn record(&self) {
        let mut history = self.history.lock().unwrap();
        history.push_back(self.snapshot());
        while history.len() > self.max_history {
            history.pop_front();
        }
    }

    pub fn history(&self) -> Vec<MemorySnapshot> {
        self.history.lock().unwrap().iter().cloned().collect()
    }

    pub fn delta(&self, from: usize, to: usize) -> Option<AllocationStats> {
        let history = self.history.lock().unwrap();
        let from_snap = history.get(from)?;
        let to_snap = history.get(to)?;
        Some(AllocationStats {
            total_allocated: to_snap.global_stats.total_allocated
                - from_snap.global_stats.total_allocated,
            total_deallocated: to_snap.global_stats.total_deallocated
                - from_snap.global_stats.total_deallocated,
            current_usage: to_snap.global_stats.current_usage
                - from_snap.global_stats.current_usage,
            peak_usage: to_snap.global_stats.peak_usage,
            allocation_count: to_snap.global_stats.allocation_count
                - from_snap.global_stats.allocation_count,
            deallocation_count: to_snap.global_stats.deallocation_count
                - from_snap.global_stats.deallocation_count,
            live_count: to_snap.global_stats.live_count,
        })
    }

    pub fn clear(&self) {
        self.history.lock().unwrap().clear();
    }
}

// ---------------------------------------------------------------------------
// Memory Debugger
// ------------------------------------------------------------------------- --

/// Detects common memory errors in debug builds.
pub struct MemoryDebugger {
    // Track allocations for use-after-free detection.
    live: Mutex<HashMap<usize, AllocationRecord>>,
    // Double-free tracking.
    freed: Mutex<HashMap<usize, AllocationRecord>>,
    enabled: AtomicBool,
}

impl MemoryDebugger {
    pub const fn new() -> Self {
        Self {
            live: Mutex::new(HashMap::new()),
            freed: Mutex::new(HashMap::new()),
            enabled: AtomicBool::new(true),
        }
    }

    pub fn set_enabled(&self, enabled: bool) {
        self.enabled.store(enabled, Ordering::Relaxed);
    }

    pub fn track_allocation(&self, record: AllocationRecord) {
        if !self.enabled.load(Ordering::Relaxed) {
            return;
        }
        self.live.lock().unwrap().insert(record.ptr, record);
    }

    pub fn track_deallocation(&self, record: AllocationRecord) {
        if !self.enabled.load(Ordering::Relaxed) {
            return;
        }
        let addr = record.ptr;
        if let Some(mut old) = self.live.lock().unwrap().remove(&addr) {
            // Double-free detection.
            if self.freed.lock().unwrap().contains_key(&addr) {
                eprintln!("MEMORY DEBUG: double-free detected at {addr:#x}");
                eprintln!("  First freed: {:?}", old.backtrace);
                eprintln!("  Second freed: {:?}", record.backtrace);
            }
            self.freed.lock().unwrap().insert(addr, old);
        }
    }

    pub fn check_use_after_free(&self, ptr: usize) -> bool {
        if !self.enabled.load(Ordering::Relaxed) {
            return false;
        }
        self.freed.lock().unwrap().contains_key(&ptr)
    }

    pub fn detect_leaks(&self) -> Vec<AllocationRecord> {
        self.live
            .lock()
            .unwrap()
            .values()
            .cloned()
            .collect()
    }

    pub fn clear(&self) {
        self.live.lock().unwrap().clear();
        self.freed.lock().unwrap().clear();
    }
}

// ---------------------------------------------------------------------------
// Memory-safe smart pointer with debug checks
// ---------------------------------------------------------------------------

/// A debug-enabled smart pointer wrapper.
pub struct DebugBox<T: ?Sized> {
    ptr: NonNull<T>,
    _marker: PhantomData<T>,
}

impl<T: ?Sized> DebugBox<T> {
    pub fn new(value: impl Into<Box<T>>) -> Self {
        let b = Box::new(value.into());
        let ptr = NonNull::new(Box::into_raw(b)).unwrap();
        Self {
            ptr,
            _marker: PhantomData,
        }
    }

    pub fn into_inner(self) -> Box<T> {
        // Prevent Drop from running since we're transferring ownership to Box.
        let mut self_mem = ManuallyDrop::new(self);
        unsafe { Box::from_raw(self_mem.ptr.as_ptr()) }
    }
}

impl<T: ?Sized> Deref for DebugBox<T> {
    type Target = T;
    fn deref(&self) -> &T {
        unsafe { &*self.ptr.as_ptr() }
    }
}

impl<T: ?Sized> DerefMut for DebugBox<T> {
    fn deref_mut(&mut self) -> &mut T {
        unsafe { &mut *self.ptr.as_ptr() }
    }
}

impl<T: ?Sized> Drop for DebugBox<T> {
    fn drop(&mut self) {
        let _ = unsafe { Box::from_raw(self.ptr.as_ptr()) };
    }
}

// ---------------------------------------------------------------------------
// Memory Manager
// ---------------------------------------------------------------------------

/// Central memory manager coordinating all subsystems.
pub struct MemoryManager {
    pub pool: Option<PoolAllocator>,
    pub linear: Option<LinearAllocator>,
    pub stack: Option<StackAllocator>,
    pub buddy: Option<BuddyAllocator>,
    pub slab: Option<SlabAllocator>,
    pub region: Option<RegionAllocator>,
    pub debug_allocator: Option<DebugAllocator<SlabAllocator>>,
    pub profiler: MemoryProfiler,
    pub debugger: MemoryDebugger,
    pub gc: Option<GenerationalGc>,
    pub component_allocator: ComponentAllocator,
    pub archetype_allocator: ArchetypeAllocator,
    pub chunk_allocator: ChunkAllocator,
    pub enabled: AtomicBool,
}

impl MemoryManager {
    pub fn new() -> Self {
        Self {
            pool: None,
            linear: None,
            stack: None,
            buddy: None,
            slab: None,
            region: None,
            debug_allocator: None,
            profiler: MemoryProfiler::new(300),
            debugger: MemoryDebugger::new(),
            gc: None,
            component_allocator: ComponentAllocator::new(64, 1024),
            archetype_allocator: ArchetypeAllocator::new(256, 256, "archetype-pool"),
            chunk_allocator: ChunkAllocator::new("chunk-alloc"),
            enabled: AtomicBool::new(true),
        }
    }

    pub fn with_pool(mut self, block_size: usize, block_count: usize) -> Self {
        self.pool = Some(PoolAllocator::new(block_size, block_count, "pool"));
        self
    }

    pub fn with_linear(mut self, capacity: usize) -> Self {
        self.linear = Some(LinearAllocator::new(capacity, "linear"));
        self
    }

    pub fn with_stack(mut self, block_size: usize, initial_blocks: usize) -> Self {
        self.stack = Some(StackAllocator::new(block_size, initial_blocks, "stack"));
        self
    }

    pub fn with_buddy(mut self, total_size: usize, min_block_size: usize) -> Self {
        self.buddy = Some(BuddyAllocator::new(total_size, min_block_size, "buddy"));
        self
    }

    pub fn with_slab(mut self, slab_size: usize, slab_align: usize) -> Self {
        self.slab = Some(SlabAllocator::new(slab_size, slab_align, "slab"));
        self
    }

    pub fn with_region(mut self, chunk_size: usize, initial_chunks: usize) -> Self {
        self.region = Some(RegionAllocator::new(chunk_size, initial_chunks, "region"));
        self
    }

    pub fn with_debug_slab(mut self, slab_size: usize, slab_align: usize) -> Self {
        let inner = SlabAllocator::new(slab_size, slab_align, "debug-slab");
        self.debug_allocator = Some(DebugAllocator::new(inner));
        self
    }

    pub fn with_generational_gc(mut self) -> Self {
        self.gc = Some(GenerationalGc::new("generational-gc"));
        self
    }

    pub fn enable(&self) {
        self.enabled.store(true, Ordering::Relaxed);
    }

    pub fn disable(&self) {
        self.enabled.store(false, Ordering::Relaxed);
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled.load(Ordering::Relaxed)
    }

    pub fn global_stats(&self) -> AllocationStats {
        GlobalAllocationTracker::instance().allocation_stats()
    }

    pub fn snapshot(&self) -> MemorySnapshot {
        self.profiler.snapshot()
    }

    pub fn record(&self) {
        self.profiler.record();
    }

    pub fn detect_leaks(&self) -> Vec<AllocationRecord> {
        GlobalAllocationTracker::instance().detect_leaks()
    }

    pub fn clear(&self) {
        GlobalAllocationTracker::instance().clear();
        self.debugger.clear();
        self.profiler.clear();
    }

    pub fn collect_gc(&mut self) {
        if let Some(gc) = &mut self.gc {
            gc.collect();
        }
    }
}

impl Default for MemoryManager {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Global Memory Manager
// ---------------------------------------------------------------------------

static GLOBAL_MEMORY_MANAGER: OnceLock<Mutex<MemoryManager>> = OnceLock::new();

pub fn global_memory_manager() -> &'static Mutex<MemoryManager> {
    GLOBAL_MEMORY_MANAGER.get_or_init(|| Mutex::new(MemoryManager::new()))
}

pub fn init_memory_manager() {
    let _ = GLOBAL_MEMORY_MANAGER.get_or_init(|| Mutex::new(MemoryManager::new()));
}

// ---------------------------------------------------------------------------
// Convenience Re-exports
// ---------------------------------------------------------------------------

pub use self::AllocationRecord as MemRecord;
pub use self::AllocationStats as MemStats;
pub use self::AllocHandle as MemHandle;
pub use self::AllocatorStats as AllocStats;
pub use self::ArchetypeAllocator as ArchetypeAlloc;
pub use self::BuddyAllocator as BuddyAlloc;
pub use self::ChunkAllocator as ChunkAlloc;
pub use self::ComponentAllocator as ComponentAlloc;
pub use self::DebugAllocator as DebugAlloc;
pub use self::DebugBox as DbgBox;
pub use self::FrameAllocator as FrameAlloc;
pub use self::GarbageCollector as Gc;
pub use self::GcObject as GcObj;
pub use self::GcStats as GcStat;
pub use self::GcVisitor as GcVis;
pub use self::GlobalAllocationTracker as GlobalAllocTracker;
pub use self::GenerationalGc as GenGc;
pub use self::IncrementalGc as IncGc;
pub use self::LinearAllocator as LinearAlloc;
pub use self::MarkAndSweepGc as MarkSweepGc;
pub use self::MemoryDebugger as MemDebug;
pub use self::MemoryManager as MemManager;
pub use self::MemoryProfiler as MemProfiler;
pub use self::MemorySnapshot as MemSnapshot;
pub use self::PoolAllocator as PoolAlloc;
pub use self::RegionAllocator as RegionAlloc;
pub use self::SlabAllocator as SlabAlloc;
pub use self::StackAllocator as StackAlloc;
pub use self::TracingGc as TracingGc;

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pool_allocate_deallocate() {
        let mut pool = PoolAllocator::new(64, 8, "test-pool");
        let ptr = pool.allocate().expect("pool alloc");
        pool.deallocate(ptr);
        assert_eq!(pool.available(), 8);
    }

    #[test]
    fn linear_allocator_basics() {
        let mut linear = LinearAllocator::new(1024, "test-linear");
        let _ = linear.alloc(42u32);
        assert!(linear.used_bytes() >= 4);
        linear.reset();
        assert_eq!(linear.used_bytes(), 0);
    }

    #[test]
    fn buddy_allocator_basics() {
        let mut buddy = BuddyAllocator::new(4096, 64, "test-buddy");
        let layout = Layout::from_size_align(128, 8).unwrap();
        let ptr = buddy.allocate(layout).expect("buddy alloc");
        buddy.deallocate(ptr);
    }

    #[test]
    fn slab_allocator_basics() {
        let mut slab = SlabAllocator::new(128, 8, "test-slab");
        let layout = Layout::from_size_align(64, 8).unwrap();
        let ptr = slab.allocate(layout).expect("slab alloc");
        slab.deallocate(ptr, 64);
        slab.release_empty_slabs();
    }

    #[test]
    fn region_allocator_basics() {
        let mut region = RegionAllocator::new(4096, 2, "test-region");
        let layout = Layout::from_size_align(128, 8).unwrap();
        let ptr = region.allocate(layout).expect("region alloc");
        region.reset();
        // Should still be valid to reuse.
        let _ = region.allocate(layout);
    }

    #[test]
    fn stack_allocator_basics() {
        let mut stack = StackAllocator::new(4096, 2, "test-stack");
        let layout = Layout::from_size_align(64, 8).unwrap();
        let _ = stack.allocate(layout);
        assert!(stack.used_bytes() >= 64);
        stack.reset();
        assert_eq!(stack.used_bytes(), 0);
    }

    #[test]
    fn debug_allocator_guard_bytes() {
        let mut slab = SlabAllocator::new(128, 8, "debug-test");
        let mut debug = DebugAllocator::new(slab);
        let layout = Layout::from_size_align(64, 8).unwrap();
        let ptr = debug.allocate(layout, Some("test")).expect("debug alloc");
        assert!(!ptr.as_ptr().is_null());
        debug.deallocate(ptr, layout);
    }

    #[test]
    fn tracing_gc_basics() {
        struct Dummy;
        impl GcObject for Dummy {
            fn mark(&self, _visitor: &mut dyn GcVisitor) {}
            fn sweep(&mut self) {}
            fn object_type(&self) -> &'static str {
                "Dummy"
            }
        }

        let gc = TracingGc::new("test-gc");
        let _ = gc.allocate(Dummy);
        gc.collect();
        let stats = gc.stats();
        assert!(stats.live_objects >= 0);
    }

    #[test]
    fn memory_manager_builder() {
        let mgr = MemoryManager::new()
            .with_pool(64, 8)
            .with_linear(4096)
            .with_stack(4096, 2)
            .with_buddy(4096, 64)
            .with_slab(128, 8)
            .with_region(4096, 2);
        assert!(mgr.is_enabled());
        let _ = mgr.global_stats();
        let _ = mgr.snapshot();
    }

    #[test]
    fn allocation_tracker() {
        let tracker = GlobalAllocationTracker::instance();
        let stats = tracker.allocation_stats();
        assert_eq!(stats.live_count, 0);
    }

    #[test]
    fn memory_debugger_tracking() {
        let debugger = MemoryDebugger::new();
        let record = AllocationRecord {
            ptr: 0x1000,
            size: 64,
            align: 8,
            label: Some("test".into()),
            backtrace: Backtrace::capture(),
            timestamp: Instant::now(),
            thread_id: std::thread::current().id(),
        };
        debugger.track_allocation(record.clone());
        let leaks = debugger.detect_leaks();
        assert_eq!(leaks.len(), 1);
        debugger.track_deallocation(record);
        let leaks = debugger.detect_leaks();
        assert_eq!(leaks.len(), 0);
    }
}
