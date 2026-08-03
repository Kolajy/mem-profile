## 2024-05-24 - Sharding Modulo Anti-Pattern for Pointers
**Learning:** In the `ProfilingAllocator`'s thread-safe registry, sharding lock contention by using `ptr % SHARD_COUNT` (where `SHARD_COUNT=16`) caused severe uneven distribution. Due to typical heap allocation alignment (multiples of 8 or 16), the lower bits of heap pointers are mostly zero, causing almost all allocations to funnel directly into shard 0, creating a massive bottleneck instead of distributed load.
**Action:** When mapping pointers to small bounded indices or shards, always ensure you mix bits using bitwise XOR and shifts (e.g., `(ptr >> 3) ^ (ptr >> 7) ...`) before applying modulo to effectively randomize the lower bits based on the higher structural bits of the address space.

## 2024-05-18 - [Resolve Symbolication Bottleneck by Grouping Raw Backtraces]
**Learning:** Generating memory reports, folded stacks, or TUI updates was slow and caused lock contention because the code eagerly symbolicated every single active allocation's raw backtrace `Vec<*mut std::ffi::c_void>`. Many active allocations originate from the exact same call stack.
**Action:** When extracting data from the allocator's sharded `REGISTRY`, first group allocations into a `HashMap<Vec<*mut std::ffi::c_void>, ...>` (e.g. summing size/count) within the lock to minimize lock duration and clone overhead. Then, symbolicate only the unique raw backtraces outside the lock.

## 2024-07-14 - Avoid unconditional cloning in HashMap Entry API
**Learning:** The HashMap `.entry(key.clone()).or_insert(...)` pattern is a known anti-pattern in hot loops when the key is expensive to clone (like a `Vec<*mut c_void>` backtrace). It forces a clone of the key *every single time*, even if the key is already in the map, causing unnecessary memory allocations and degrading performance during profile generation or leak reporting.
**Action:** Always prefer a two-step approach (`get_mut` followed by an `insert` with `.clone()` only if the key doesn't exist) when dealing with keys that are expensive to clone inside hot processing loops.

## 2024-07-15 - Zero-Allocation in Terminal UI Hot Loops
**Learning:** In highly frequent terminal UI render loops (e.g., `ratatui` `draw` cycles), passing owned data using unconditional cloning (like `String::clone()` or `Vec::clone()`) causes severe garbage collection overhead and memory bloat on every frame, even though `ratatui` widgets inherently support borrowed data (e.g., `&str`, `&[T]`).
**Action:** When working with `ratatui` cells, spans, or charts, strictly construct them from references (`.as_str()`, `&[]`) derived from application state rather than cloning the state to prevent continuous heap allocations in the main thread.
## 2024-11-20 - HashMap::entry overhead with conditionally cloned keys
**Learning:** Using `HashMap::entry(cached.clone()).or_insert(...)` when a key must be cloned from a cache to satisfy the entry API forces a string allocation even when the key already exists in the destination map. However, when the string is *already owned* (e.g. freshly created from `join`), passing it to `.entry()` does not cost an allocation.
**Action:** When working with references to strings in caches, avoid the entry API if it requires `.clone()` to pass ownership. Use `.get_mut()` first to update the value if present, and fallback to `.insert(key.clone())` only when a new entry needs to be created.
## 2026-07-16 - Safe TUI Symbolication Memoization
**Learning:** Calling `symbolicate_frames` repeatedly in a TUI loop causes massive CPU spikes. Memoization requires safely wrapping raw pointers to be thread-safe for the App state using `unsafe impl Send` and `Sync` on a dedicated newtype.
**Action:** Always memoize expensive formatting or processing tasks (like stack trace symbolication) outside of the core rendering loop and ensure safe newtypes are used when handling raw pointers in concurrent structures.
## 2024-07-17 - [TUI Clone Optimization]
**Learning:** In hot loops, particularly in terminal UI render loops, `clone()` can become a significant CPU bottleneck. The memory profiling `App` struct uses a stateful symbol cache indexed by `FramePtrs`. By avoiding `clone()` entirely for `frames` and moving it into `FramePtrs` when fetching from the cache, we removed an unnecessary dynamic memory allocation every single render tick.
**Action:** When working with cache map lookups that might take owned types inside highly frequent loops, refactor code to move/take ownership of data structures instead of cloning them.
## 2024-11-21 - Zero-Allocation TUI Cache via Arc<String>
**Learning:** In the hot TUI render loop, grouping backtraces and inserting cached symbolicated strings into a new temporary `HashMap` caused the program to unconditionally call `cached.clone()` (a `String` clone operation) for every unique allocation path on *every tick*. This caused significant heap bloat and CPU overhead from garbage string allocations.
**Action:** When a temporary data structure (like the tick-specific `folded` map) needs ownership or copies of values stored in a persistent cache, store those values wrapped in an `Arc<T>` (e.g., `Arc<String>`). This ensures the cache lookups and inserts only cost an atomic reference count increment (`Arc::clone`) instead of an entire string allocation.
## 2025-01-22 - Zero-Allocation statm Parsing in Polling Loops
**Learning:** Using `std::fs::read_to_string` inside tight polling loops (e.g., reading `/proc/{pid}/statm` every 10ms) causes constant dynamic string allocations and garbage collection overhead. Since the `statm` file content is very small, allocating a heap string every read causes significant memory bloat and CPU usage.
**Action:** Always use a stack-allocated buffer (`[0u8; 128]`) with `std::fs::File::open` and `.read()` when continuously polling small pseudo-files like `/proc/...` to ensure zero heap allocations per tick.
## 2024-11-21 - Avoid clone for cached FramePtrs by implementing Borrow
**Learning:** In the `get_active_allocations` function, although `FramePtrs` had its own type wrapping the vector of raw pointers, we couldn't query the `HashMap` keyed by `FramePtrs` with a slice without allocating due to the missing `Borrow` trait implementation. This caused unnecessary `clone()` operations for backtraces that were already memoized, adding significant CPU overhead during the TUI loop.
**Action:** Always implement `std::borrow::Borrow<[T]>` for wrapper structs containing `Vec<T>` (like `FramePtrs`) to allow zero-allocation queries against caches using `.as_slice()` directly.
## 2025-07-19 - Zero-Allocation Ratatui Table Rows
**Learning:** Using `let cells = vec![...]` directly within `ratatui::widgets::Row::new(...)` inside the frequent TUI render loops causes an `O(N)` dynamic heap allocation issue for every frame (where N is the number of rows), degrading performance and creating garbage collection pressure.
**Action:** Since `ratatui::widgets::Row::new` accepts any `IntoIterator`, always use standard array syntax `let cells = [...]` when passing statically sized collections of table cells to prevent any dynamic `Vec` allocations during rendering.
## 2024-07-21 - O(1) TUI memory sampling history removal
**Learning:** Using `Vec::remove(0)` to manage a sliding window for TUI history graphs caused O(N) memory shifting on every tick, adding unnecessary performance overhead to the background monitoring thread.
**Action:** When implementing sliding windows for historical UI data (like timeline charts in `ratatui`), always use `std::collections::VecDeque`. You can maintain zero-allocation rendering by calling `.make_contiguous()` to provide a flat slice directly to `ratatui` widgets.
## 2025-01-22 - Avoid `format!` in TUI Render Loops
**Learning:** Calling `format!` on every tick of a terminal UI render loop (like formatting a `PID` into a window title or creating a `/proc/{pid}/statm` file path string for polling) causes massive, continuous string allocation overhead and degrades application performance over time by stressing the global allocator.
**Action:** When a formatted string depends entirely on static data that doesn't change during the loop's execution (like a process PID), always pre-allocate the formatted string once during initialization (e.g. in `App::new` or before the polling loop begins) and reuse a reference to it in the hot loop.
## 2024-07-23 - Filter off-screen chart data
**Learning:** Passing the entire history slice (up to 1000 points) to `ratatui`'s `Chart` widget when the X-axis bounds are restricted to a smaller window (e.g., the last 60 seconds) causes the widget to needlessly process off-screen points on every render tick. Furthermore, if `max_bytes` is computed on the entire dataset instead of the visible window, the Y-axis will remain zoomed out due to historical spikes that are no longer on screen.
**Action:** When rendering time-series data in `ratatui` charts with a rolling time window, slice the dataset to include only visible points before passing it to the widget (e.g., using `partition_point` for O(log N) lookup) and before computing max values for dynamic axis scaling. Remember to use `.saturating_sub(1)` when slicing to retain one point just off-screen so the drawing algorithm can render the line entering the chart smoothly.
## 2024-11-21 - Ordering::SeqCst overhead in Global Allocator metrics
**Learning:** Using `Ordering::SeqCst` for statistical atomic counters (`active_bytes`, `allocation_count`, etc.) inside the `GlobalAlloc` trait implementation imposes a full memory barrier on every single heap allocation. This significantly degrades performance in multi-threaded programs by introducing unnecessary hardware-level synchronization overhead.
**Action:** When tracking independent statistical metrics (like counters) that do not act as synchronization primitives for other memory locations, always use `Ordering::Relaxed` to avoid costly memory barriers.
## 2024-05-25 - Avoid Reallocations in Global Allocator Hook
**Learning:** Initializing the backtrace `Vec` with `Vec::new()` in `capture_raw_backtrace` causes multiple dynamic reallocations as frames are pushed. Because this function is called on *every single allocation* intercepted by the `GlobalAlloc` hook, these reallocations add significant overhead and recursively trigger the allocator itself.
**Action:** When capturing data in ultra-hot paths like a global allocator hook, always pre-allocate expected space (e.g., `Vec::with_capacity(32)`) to prevent multiple reallocation cycles.
## 2024-07-28 - Zero-Allocation Iterators for Massive String Parsing
**Learning:** Using `.collect::<Vec<&str>>()` on strings created from massive files (e.g. up to 256MB memory snapshots during diffing) allocates enormous intermediate heap vectors just to iterate through the parts once.
**Action:** Always prefer a direct, lazy streaming iterator loop (e.g., `let iter = string.split(...); for part in iter { ... }`) when scanning over or parsing large string payloads to prevent large dynamic heap spikes and out-of-memory overheads.
## 2024-11-21 - Avoid String Allocations in Custom JSON Parsers
**Learning:** During snapshot diffing in `src/diff.rs`, the simplistic custom JSON parser was eagerly allocating new `String` instances on the heap for every single "stack" key using `.to_string()`. This resulted in significant memory bloat, especially for large snapshot files with millions of elements. Since the keys are substrings of the input string `json`, they can be borrowed directly as string slices (`&str`).
**Action:** Always favor returning borrowed slices `&str` mapped to the lifetime of the input string instead of calling `.to_string()` or `.clone()` inside parsing loops. Change container keys to hold references (e.g., `HashMap<&'a str, AllocationStats>`) wherever feasible to achieve zero-allocation parsing.
## 2025-01-22 - BufWriter optimization for snapshot dumps
**Learning:** Writing massive amounts of data directly to a `File` handler via `writeln!` in a tight loop causes a severe system bottleneck because it executes an unbuffered `write()` syscall for every single line. For operations like serializing memory snapshots containing thousands of allocations, this blocked the process significantly.
**Action:** Always wrap underlying `File` handles in `std::io::BufWriter` when performing multiple sequential writes (like in loops). Ensure `drop(buf_writer)` or `.flush()` is explicitly called to finalize writes before renaming or modifying the file on disk.
## 2025-01-22 - Avoid format! with push_str in Loops
**Learning:** Using `String::push_str` coupled with `format!` in a loop generating large text outputs (like flamegraph folded stacks) causes intermediate `String` allocations for every formatted line.
**Action:** Use `writeln!` with `std::fmt::Write` to directly format text into the target `String` buffer, which prevents intermediate string allocations and significantly improves execution speed.

## 2026-07-20 - Cache backtrace symbolication in snapshot dump
**Learning:** Calling `symbolicate_frames` for every allocation in a tight loop when dumping a snapshot causes major performance degradation and freezes since symbolication is extremely expensive. It was unconditionally called for every allocation rather than being grouped or cached by the raw stack frame pointers.
**Action:** Use a `HashMap` to cache the expensive formatting of `symbolicate_frames`, keyed by the raw backtrace pointers (`&Vec<*mut std::ffi::c_void>`), ensuring that identical backtraces are only symbolicated and formatted once during a snapshot dump.
## 2024-11-20 - Prevent intermediate allocations when joining strings
**Learning:** Collecting elements into an intermediate `Vec<String>` just to `.join()` them causes severe heap churn in hot paths like `ratatui` render loops when formatting call stacks.
**Action:** Use a pre-allocated `String::with_capacity` buffer and `push_str` or `std::fmt::Write` directly in a loop to build the string in place without intermediate collections.
## 2026-07-27 - Avoid Vec<String> and .join() for string building
**Learning:** Using an intermediate `Vec<String>` and calling `.join(";")` to build strings like folded stacks causes unnecessary dynamic memory allocations and heap churn.
**Action:** Use a pre-allocated `String::with_capacity` buffer and directly push characters or mapped characters (using `.chars()`) in-place inside loops instead of collecting into a `Vec`.
## 2024-11-21 - Zero-Allocation TUI Cache via Hoisted Buffers
**Learning:** Instantiating temporary data structures (like `HashMap` and `Vec`) within the TUI event loop on every tick to process allocations causes continuous heap bucket allocations and severe garbage collection pressure, even if we wrap entries in `Arc`.
**Action:** Hoist these temporary buffer collections (`raw_allocs`, `folded`, `items`) to the outer event loop (e.g. `run_app`) and pass them by mutable reference (`&mut`) into the processing functions. Use `.clear()` at the start of each tick to reuse the internal allocated capacity and eliminate O(N) heap allocations per frame.

## 2026-07-31 - Group identical backtraces inside locks before snapshot generation
**Learning:** During snapshot dumping, simply looping over the allocator registry and cloning the backtrace `Vec` into a list for every single allocation caused severe `O(N)` heap memory bloat when there were millions of allocations of the same type.
**Action:** Always group records into a map inside the registry extraction lock to reduce cloning and processing overhead to `O(U)` (where `U` is the number of unique entries).
## 2024-11-23 - File descriptor caching
**Learning:** Repeatedly opening and closing `/proc/{pid}/statm` in a tight loop creates unnecessary syscall overhead.
**Action:** Cache the open `File` descriptor and use `.seek(SeekFrom::Start(0))` before `.read()` to reduce syscall overhead in high-frequency polling loops.
## 2024-05-19 - Remove Dead Code Loops in Hot Paths
**Learning:** Sometimes entire loops are left in codebase from previous iterations. When reviewing string parsing logic (like `parse_json_map` in `diff.rs`), look out for `while` loops that allocate memory inside the loop body but never use the resulting variable, simply discarding it on every iteration.
**Action:** Before optimizing a string allocation loop, double-check if the resulting variable is even used. If it's a dead code loop, removing it entirely offers massive performance gains by avoiding both O(N) iteration and associated allocations.
## 2024-11-23 - File descriptor caching
**Learning:** Repeatedly opening and closing `/proc/{pid}/statm` in a tight loop creates unnecessary syscall overhead.
**Action:** Cache the open `File` descriptor and use `.seek(SeekFrom::Start(0))` before `.read()` to reduce syscall overhead in high-frequency polling loops.
## 2024-05-19 - Pre-allocate Collections to Prevent Reallocations
**Learning:** Initializing `Vec` or `String` buffers using `::new()` inside hot paths for large data mapping—like `symbolicate_frames` (which creates a `Vec` for every captured backtrace) or profiling dumps (which construct strings for thousands of stacks)—causes severe performance degradation due to iterative dynamic heap reallocations when growing.
**Action:** When mapping, formatting, or grouping data into a newly owned collection, always use `.with_capacity()` to pre-allocate memory based on the known size of the input elements or a robust heuristic.
## 2026-08-01 - Pre-allocate Diff Collections
**Learning:** During large memory snapshot diffing, appending thousands of elements to uninitialized collections (`Vec::new()`) triggers massive dynamic heap reallocations and slows down processing.
**Action:** Always pre-allocate output collections (e.g. `Vec::with_capacity(size)`) using the known lengths of input data sets (or their maximum) to completely avoid intermediate heap resizing during the operation.
