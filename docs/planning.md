# Design & Planning Document: `mem-profile`

This document details the architectural layout, core engineering challenges, design decisions, and data structures for the `mem-profile` Rust library and CLI.

---

## 1. Architectural Goals & Constraints

- **Minimal Overhead**: The allocator hook should add minimal latency to program allocation paths.
- **No Allocation Reentrancy**: Capturing backtraces or storing metadata must not trigger allocations, which would cause infinite recursion (a common pitfall in custom allocators).
- **Thread Safety**: Intercepting allocations from concurrent threads safely with high throughput (low contention lock-free structures or thread-local buffers).
- **Portability**: Support macOS and Linux natively, utilizing standard Rust features where possible.
- **Developer Ergonomics**: Simple drop-in setup with cargo features and cargo subcommand integration.

---

## 2. Technical Challenges & Mitigation Strategies

### Challenge A: Allocator Reentrancy (Infinite Recursion)
When the allocator intercepts a call (e.g., `alloc`), it needs to write to a tracking map. If that map allocates memory, it calls `alloc` again, causing infinite recursion and a stack overflow.

**Mitigation**:
- **Bypassing the Global Allocator**: Inside the tracking code, bypass the `#[global_allocator]` wrapper by directly invoking the system allocator APIs (e.g., calling `std::alloc::alloc` using the underlying `System` allocator directly) or utilizing static/arena pre-allocated buffers.
- **Thread-Local State**: Maintain a thread-local flag `IS_PROFILING: Cell<bool>` to detect when a thread is already executing tracking code. If `IS_PROFILING` is true, the allocation bypasses tracking and goes straight to the underlying allocator.

### Challenge B: Thread Contention & Performance
Multiple threads allocating simultaneously could bottle up on a global mutex protecting the active allocation map.

**Mitigation**:
- **Sharded Maps / Lock-Free Maps**: Use a sharded map structure (e.g., `dashmap` or a custom atomic-swapped lock-free array) to distribute locks across different buckets based on the pointer address.
- **Thread-Local Aggregation**: Accumulate allocation stats in thread-local storage (TLS) and periodically flush or aggregate them to a central collector, reducing the lock frequency on high-frequency allocation paths.

### Challenge C: Fast Backtrace Capture
Capturing and symbolicating backtraces is highly expensive. Symbolicating (converting instruction pointers to file/line numbers) involves parsing debug symbols and disk I/O.

**Mitigation**:
- **Lazy Symbolication**: Only capture raw instruction pointers (`*mut c_void`) during the execution phase. Defer symbolication until the report generation phase (e.g., on program exit or profiling dump trigger).
- **Fast Unwinding**: Use platform-optimized unwinding library calls (via `backtrace::trace_unsymbolized` or frame-pointer unwinding if compile options allow).

---

## 3. Data Structures & Components

### `ProfilingAllocator<A>`
Exposes the `GlobalAlloc` trait. Implements:
```rust
pub struct ProfilingAllocator<A: GlobalAlloc> {
    inner: A,
    // Atomic counters for total/active allocation sizes
    active_bytes: AtomicUsize,
    allocation_count: AtomicUsize,
}
```

### `AllocationMetadata`
Stores info about each active allocation:
```rust
pub struct AllocationMetadata {
    pub size: usize,
    pub timestamp: std::time::Instant,
    pub backtrace: Vec<*mut std::ffi::c_void>, // Raw frame addresses
}
```

### Allocation Registry
A global registry that holds active allocations, mapped from `*mut u8` (allocation pointer) to `AllocationMetadata`.
It will utilize a sharded lock design to scale with CPU cores:
```rust
struct Registry {
    // 64 or 128 shards to minimize lock contention
    shards: [Mutex<HashMap<usize, AllocationMetadata>>; SHARD_COUNT],
}
```

---

## 4. Interface and Output Formats

### Library API
- `init()`: Starts tracking allocations, returns a guard that triggers leak detection on drop.
- `dump_to_file(path: &Path)`: Serializes current heap profile.
- `reset()`: Resets all active allocation counters and maps.

### Export Formats
1. **JSON Summary**: Simple list of active allocations, cumulative sizes, and leak reports.
2. **pprof Protocol Buffer**: Binary file format compatible with Google's `pprof` tool, enabling analysis in existing visualization web engines.
3. **Flamegraph SVG**: Folded stack format for rendering SVG flamegraphs directly.
