use std::alloc::{GlobalAlloc, Layout};
use std::cell::Cell;
use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Mutex, OnceLock};
use std::time::Instant;

thread_local! {
    // Thread-local variable to prevent recursive allocator interception.
    // Const initialization ensures no dynamic allocation on first access.
    pub(crate) static IN_ALLOCATOR: Cell<bool> = const { Cell::new(false) };
}

pub struct AllocationMetadata {
    pub size: usize,
    pub timestamp: Instant,
    pub backtrace: Vec<*mut std::ffi::c_void>,
}

// Manually implement Send and Sync since raw pointers in backtrace Vec are not Send/Sync.
unsafe impl Send for AllocationMetadata {}
unsafe impl Sync for AllocationMetadata {}

const SHARD_COUNT: usize = 16;

#[inline(always)]
fn get_shard_idx(ptr: usize) -> usize {
    // Allocator alignments often make lower bits 0 (e.g., multiples of 8 or 16).
    // Mix the bits to distribute load evenly across shards and avoid lock contention.
    let hash = (ptr >> 3) ^ (ptr >> 7) ^ (ptr >> 11);
    hash % SHARD_COUNT
}

#[derive(Default)]
pub struct Registry {
    shards: OnceLock<[Mutex<HashMap<usize, AllocationMetadata>>; SHARD_COUNT]>,
}

impl Registry {
    pub const fn new() -> Self {
        Self {
            shards: OnceLock::new(),
        }
    }

    pub fn get_shards(&self) -> &[Mutex<HashMap<usize, AllocationMetadata>>; SHARD_COUNT] {
        self.shards.get_or_init(|| {
            [
                Mutex::new(HashMap::new()),
                Mutex::new(HashMap::new()),
                Mutex::new(HashMap::new()),
                Mutex::new(HashMap::new()),
                Mutex::new(HashMap::new()),
                Mutex::new(HashMap::new()),
                Mutex::new(HashMap::new()),
                Mutex::new(HashMap::new()),
                Mutex::new(HashMap::new()),
                Mutex::new(HashMap::new()),
                Mutex::new(HashMap::new()),
                Mutex::new(HashMap::new()),
                Mutex::new(HashMap::new()),
                Mutex::new(HashMap::new()),
                Mutex::new(HashMap::new()),
                Mutex::new(HashMap::new()),
            ]
        })
    }

    pub fn insert(&self, ptr: usize, size: usize, backtrace: Vec<*mut std::ffi::c_void>) {
        let shard_idx = get_shard_idx(ptr);
        let shards = self.get_shards();
        if let Ok(mut shard) = shards[shard_idx].lock() {
            shard.insert(
                ptr,
                AllocationMetadata {
                    size,
                    timestamp: Instant::now(),
                    backtrace,
                },
            );
        }
    }

    pub fn remove(&self, ptr: usize) -> Option<AllocationMetadata> {
        let shard_idx = get_shard_idx(ptr);
        let shards = self.get_shards();
        if let Ok(mut shard) = shards[shard_idx].lock() {
            shard.remove(&ptr)
        } else {
            None
        }
    }

    pub fn update_size(&self, ptr: usize, new_size: usize) {
        let shard_idx = get_shard_idx(ptr);
        let shards = self.get_shards();
        if let Ok(mut shard) = shards[shard_idx].lock() {
            if let Some(meta) = shard.get_mut(&ptr) {
                meta.size = new_size;
            }
        }
    }
}

pub static REGISTRY: Registry = Registry::new();

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
        let start = if crate::alert::is_audit_enabled() {
            Some(Instant::now())
        } else {
            None
        };
        let ptr = self.inner.alloc(layout);
        if !ptr.is_null() {
            IN_ALLOCATOR.with(|in_alloc| {
                if !in_alloc.get() {
                    in_alloc.set(true);

                    if let Some(start_time) = start {
                        crate::alert::check_performance_audit(start_time.elapsed());
                    }
                    let size = layout.size();
                    let current = self.active_bytes.fetch_add(size, Ordering::SeqCst) + size;
                    crate::alert::check_memory_threshold(current);
                    self.allocation_count.fetch_add(1, Ordering::SeqCst);

                    let frames = crate::backtrace::capture_raw_backtrace();
                    REGISTRY.insert(ptr as usize, size, frames);

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
                let removed = REGISTRY.remove(ptr as usize);
                if removed.is_some() {
                    self.active_bytes.fetch_sub(size, Ordering::SeqCst);
                    self.deallocation_count.fetch_add(1, Ordering::SeqCst);
                }
                in_alloc.set(false);
            }
        });
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        let start = if crate::alert::is_audit_enabled() {
            Some(Instant::now())
        } else {
            None
        };
        let ptr = self.inner.alloc_zeroed(layout);
        if !ptr.is_null() {
            IN_ALLOCATOR.with(|in_alloc| {
                if !in_alloc.get() {
                    in_alloc.set(true);

                    if let Some(start_time) = start {
                        crate::alert::check_performance_audit(start_time.elapsed());
                    }
                    let size = layout.size();
                    let current = self.active_bytes.fetch_add(size, Ordering::SeqCst) + size;
                    crate::alert::check_memory_threshold(current);
                    self.allocation_count.fetch_add(1, Ordering::SeqCst);

                    let frames = crate::backtrace::capture_raw_backtrace();
                    REGISTRY.insert(ptr as usize, size, frames);

                    in_alloc.set(false);
                }
            });
        }
        ptr
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        let start = if crate::alert::is_audit_enabled() {
            Some(Instant::now())
        } else {
            None
        };
        let new_ptr = self.inner.realloc(ptr, layout, new_size);
        if !new_ptr.is_null() {
            IN_ALLOCATOR.with(|in_alloc| {
                if !in_alloc.get() {
                    in_alloc.set(true);

                    if let Some(start_time) = start {
                        crate::alert::check_performance_audit(start_time.elapsed());
                    }
                    let old_size = layout.size();
                    if new_ptr == ptr {
                        // Resized in place
                        if new_size > old_size {
                            let current = self
                                .active_bytes
                                .fetch_add(new_size - old_size, Ordering::SeqCst)
                                + (new_size - old_size);
                            crate::alert::check_memory_threshold(current);
                        } else {
                            self.active_bytes
                                .fetch_sub(old_size - new_size, Ordering::SeqCst);
                        }
                        REGISTRY.update_size(ptr as usize, new_size);
                    } else {
                        // Memory block was moved
                        self.active_bytes.fetch_sub(old_size, Ordering::SeqCst);
                        let current =
                            self.active_bytes.fetch_add(new_size, Ordering::SeqCst) + new_size;
                        crate::alert::check_memory_threshold(current);

                        let old_meta = REGISTRY.remove(ptr as usize);
                        let frames = old_meta
                            .map(|m| m.backtrace)
                            .unwrap_or_else(crate::backtrace::capture_raw_backtrace);
                        REGISTRY.insert(new_ptr as usize, new_size, frames);
                    }
                    in_alloc.set(false);
                }
            });
        }
        new_ptr
    }
}
