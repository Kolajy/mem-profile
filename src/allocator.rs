use std::alloc::{GlobalAlloc, Layout};
use std::cell::Cell;
use std::sync::atomic::{AtomicUsize, Ordering};

thread_local! {
    // Thread-local variable to prevent recursive allocator interception.
    // Const initialization ensures no dynamic allocation on first access.
    static IN_ALLOCATOR: Cell<bool> = const { Cell::new(false) };
}

/// A wrapper allocator that profiles memory usage metrics.
pub struct ProfilingAllocator<A: GlobalAlloc> {
    inner: A,
    active_bytes: AtomicUsize,
    allocation_count: AtomicUsize,
    deallocation_count: AtomicUsize,
}

impl<A: GlobalAlloc> ProfilingAllocator<A> {
    /// Creates a new `ProfilingAllocator` wrapping the provided allocator.
    pub const fn new(inner: A) -> Self {
        Self {
            inner,
            active_bytes: AtomicUsize::new(0),
            allocation_count: AtomicUsize::new(0),
            deallocation_count: AtomicUsize::new(0),
        }
    }

    /// Returns the number of active bytes currently allocated.
    pub fn active_bytes(&self) -> usize {
        self.active_bytes.load(Ordering::Relaxed)
    }

    /// Returns the total number of allocations intercepted.
    pub fn allocation_count(&self) -> usize {
        self.allocation_count.load(Ordering::Relaxed)
    }

    /// Returns the total number of deallocations intercepted.
    pub fn deallocation_count(&self) -> usize {
        self.deallocation_count.load(Ordering::Relaxed)
    }
}

unsafe impl<A: GlobalAlloc> GlobalAlloc for ProfilingAllocator<A> {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let ptr = self.inner.alloc(layout);
        if !ptr.is_null() {
            IN_ALLOCATOR.with(|in_alloc| {
                if !in_alloc.get() {
                    in_alloc.set(true);
                    let size = layout.size();
                    self.active_bytes.fetch_add(size, Ordering::SeqCst);
                    self.allocation_count.fetch_add(1, Ordering::SeqCst);
                    in_alloc.set(false);
                }
            });
        }
        ptr
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        self.inner.dealloc(ptr, layout);
        IN_ALLOCATOR.with(|in_alloc| {
            if !in_alloc.get() {
                in_alloc.set(true);
                let size = layout.size();
                self.active_bytes.fetch_sub(size, Ordering::SeqCst);
                self.deallocation_count.fetch_add(1, Ordering::SeqCst);
                in_alloc.set(false);
            }
        });
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        let ptr = self.inner.alloc_zeroed(layout);
        if !ptr.is_null() {
            IN_ALLOCATOR.with(|in_alloc| {
                if !in_alloc.get() {
                    in_alloc.set(true);
                    let size = layout.size();
                    self.active_bytes.fetch_add(size, Ordering::SeqCst);
                    self.allocation_count.fetch_add(1, Ordering::SeqCst);
                    in_alloc.set(false);
                }
            });
        }
        ptr
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        let new_ptr = self.inner.realloc(ptr, layout, new_size);
        if !new_ptr.is_null() {
            IN_ALLOCATOR.with(|in_alloc| {
                if !in_alloc.get() {
                    in_alloc.set(true);
                    let old_size = layout.size();
                    if new_ptr == ptr {
                        // Resized in place
                        if new_size > old_size {
                            self.active_bytes
                                .fetch_add(new_size - old_size, Ordering::SeqCst);
                        } else {
                            self.active_bytes
                                .fetch_sub(old_size - new_size, Ordering::SeqCst);
                        }
                    } else {
                        // Memory block was moved
                        self.active_bytes.fetch_sub(old_size, Ordering::SeqCst);
                        self.active_bytes.fetch_add(new_size, Ordering::SeqCst);
                    }
                    in_alloc.set(false);
                }
            });
        }
        new_ptr
    }
}
