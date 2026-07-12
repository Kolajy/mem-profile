# Design & Planning Document: `mem-profile`

This document details the architectural layout, core engineering challenges, design decisions, and data structures for the `mem-profile` Rust library and CLI.

---

## 1. Architectural Goals & Constraints

- **Minimal Overhead**: The allocator hook should add minimal latency to program allocation paths.
- **No Allocation Reentrancy**: Capturing backtraces or storing metadata must not trigger allocations, which would cause infinite recursion (a common pitfall in custom allocators).
- **Thread Safety**: Intercepting allocations from concurrent threads safely with high throughput (low contention lock-free structures or thread-local buffers).
- **Portability**: Support macOS and Linux natively, utilizing standard Rust features where possible.
- **Developer Ergonomics**: Simple drop-in setup with cargo features and cargo subcommand integration.
- **Homebrew Ready**: Standalone binary packaging suitable for Homebrew core formula requirements (`brew install mem-profile`).

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
- **Sharded Maps**: Use a sharded map structure (a const array of Mutex/HashMap protected by a `OnceLock` for safe initialization) to distribute locks across different buckets based on pointer addresses.
- **Thread-Local Aggregation**: Accumulate allocation stats in thread-local storage (TLS) and periodically flush or aggregate them to a central collector, reducing the lock frequency on high-frequency allocation paths.

### Challenge C: Fast Backtrace Capture
Capturing and symbolicating backtraces is highly expensive. Symbolicating (converting instruction pointers to file/line numbers) involves parsing debug symbols and disk I/O.

**Mitigation**:
- **Lazy Symbolication**: Only capture raw instruction pointers (`*mut c_void`) during the execution phase. Defer symbolication until the report generation phase (e.g., on program exit or profiling dump trigger).
- **Fast Unwinding**: Use platform-optimized unwinding library calls (via `backtrace::trace` or frame-pointer unwinding if compile options allow).

---

## 3. Data Structures & Components

### `ProfilingAllocator<A>`
Exposes the `GlobalAlloc` trait. Implements:
```rust
pub struct ProfilingAllocator<A: GlobalAlloc> {
    inner: A,
    active_bytes: AtomicUsize,
    allocation_count: AtomicUsize,
    deallocation_count: AtomicUsize,
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
It utilizes a sharded lock design to scale with CPU cores:
```rust
pub struct Registry {
    shards: OnceLock<[Mutex<HashMap<usize, AllocationMetadata>>; SHARD_COUNT]>,
}
```

---

## 4. Homebrew & Terminal UI (TUI) Vision

To prepare the project to become a core Homebrew package (`brew install mem-profile`), we must build a world-class Terminal User Interface (TUI) and release pipeline.

### Terminal UI (TUI) Engine
Instead of just printing a static graph at termination, `mem-profile` will feature a real-time, interactive dashboard inside the terminal (using `ratatui` or similar TUI layouts):
- **Live RSS Timeline**: A real-time terminal chart graphing the process's physical memory footprint.
- **Allocations Table**: An interactive table listing top memory-consuming call stacks sorted by size or allocation count.
- **Flamegraph Viewer**: An interactive ASCII/block flamegraph rendering directly in the terminal window.
- **Control panel**: Shortcuts to manually trigger snapshots, dump reports, or pause profiling.

### Release & Packaging Strategy
1. **Zero External Runtime Dependencies**: Ensure the CLI relies only on standard system libraries (`libc`) and contains statically linked Rust dependencies, meeting the strict requirements of Homebrew Formula packaging.
2. **Pre-compiled Binary Releases**: Setup GitHub Actions to build statically linked release binaries for:
   - `x86_64-apple-darwin` (macOS Intel)
   - `aarch64-apple-darwin` (macOS Apple Silicon)
   - `x86_64-unknown-linux-gnu` (Linux)
3. **Formula Tap**: Maintain a custom Homebrew Tap `Kolajy/homebrew-mem-profile` as a staging ground before proposing the formula to Homebrew core.
