use std::alloc::{GlobalAlloc, Layout, System};
use std::collections::VecDeque;
use std::ptr::NonNull;

use crate::CHUNK_SIZE;

/// Statistics for allocator debugging.
#[derive(Default, Debug, Clone)]
pub struct AllocatorStats {
    pub allocated_chunks: usize,
    pub reclaimed_chunks: usize,
    pub active_pages: usize,
}

/// Page-based pool allocator for ECS chunks (Mimari §1.2).
pub struct WorldAllocator {
    pages: Vec<NonNull<u8>>,
    free_pages: VecDeque<NonNull<u8>>,
    stats: AllocatorStats,
}

impl WorldAllocator {
    pub fn new() -> Self {
        Self {
            pages: Vec::new(),
            free_pages: VecDeque::new(),
            stats: AllocatorStats::default(),
        }
    }

    pub fn allocate_chunk(&mut self) -> NonNull<u8> {
        if let Some(ptr) = self.free_pages.pop_front() {
            self.stats.reclaimed_chunks += 1;
            return ptr;
        }
        let layout = Layout::from_size_align(CHUNK_SIZE, 64).expect("invalid chunk layout");
        let ptr = unsafe {
            let ptr = System.alloc(layout);
            if ptr.is_null() {
                std::alloc::handle_alloc_error(layout);
            }
            NonNull::new_unchecked(ptr)
        };
        self.pages.push(ptr);
        self.stats.allocated_chunks += 1;
        self.stats.active_pages = self.pages.len();
        ptr
    }

    pub fn reclaim_chunk(&mut self, ptr: NonNull<u8>) {
        self.free_pages.push_back(ptr);
    }

    pub fn page_count(&self) -> usize {
        self.pages.len()
    }

    pub fn stats(&self) -> &AllocatorStats {
        &self.stats
    }

    pub fn clear_free_list(&mut self) {
        self.free_pages.clear();
    }
}

impl Default for WorldAllocator {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for WorldAllocator {
    fn drop(&mut self) {
        let layout = Layout::from_size_align(CHUNK_SIZE, 64).expect("invalid chunk layout");
        for ptr in self.pages.drain(..) {
            // SAFETY: pointers were allocated with the same layout via System.
            unsafe { System.dealloc(ptr.as_ptr(), layout) };
        }
    }
}
