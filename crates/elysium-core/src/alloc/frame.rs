use std::alloc::{GlobalAlloc, Layout, System};
use std::marker::PhantomData;
use std::ptr::NonNull;

const DEFAULT_FRAME_CAPACITY: usize = 4 * 1024 * 1024;

/// Linear bump allocator reset each frame (Mimari §1.2).
pub struct FrameAllocator {
    buffer: NonNull<u8>,
    capacity: usize,
    offset: usize,
}

impl FrameAllocator {
    pub fn new(capacity: usize) -> Self {
        let layout = Layout::from_size_align(capacity, 64).expect("invalid frame layout");
        let ptr = unsafe {
            let ptr = System.alloc(layout);
            if ptr.is_null() {
                std::alloc::handle_alloc_error(layout);
            }
            NonNull::new_unchecked(ptr)
        };
        Self {
            buffer: ptr,
            capacity,
            offset: 0,
        }
    }

    #[inline]
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
        unsafe { Some(NonNull::new_unchecked(self.buffer.as_ptr().add(aligned))) }
    }

    pub fn alloc<T>(&mut self, value: T) -> Option<&mut T> {
        let layout = Layout::new::<T>();
        let ptr = self.allocate(layout)?;
        unsafe {
            ptr.as_ptr().cast::<T>().write(value);
            Some(&mut *ptr.as_ptr().cast::<T>())
        }
    }

    #[inline]
    pub fn used_bytes(&self) -> usize {
        self.offset
    }

    #[inline]
    pub fn capacity(&self) -> usize {
        self.capacity
    }
}

impl Default for FrameAllocator {
    fn default() -> Self {
        Self::new(DEFAULT_FRAME_CAPACITY)
    }
}

impl Drop for FrameAllocator {
    fn drop(&mut self) {
        let layout = Layout::from_size_align(self.capacity, 64).expect("invalid frame layout");
        unsafe { System.dealloc(self.buffer.as_ptr(), layout) };
    }
}

/// Scoped guard that resets the frame allocator when dropped.
#[allow(dead_code)]
pub struct FrameScope<'a> {
    allocator: &'a mut FrameAllocator,
    _marker: PhantomData<&'a ()>,
}

#[allow(dead_code)]
impl<'a> FrameScope<'a> {
    pub fn new(allocator: &'a mut FrameAllocator) -> Self {
        allocator.reset();
        Self {
            allocator,
            _marker: PhantomData,
        }
    }

    pub fn allocator(&mut self) -> &mut FrameAllocator {
        self.allocator
    }
}