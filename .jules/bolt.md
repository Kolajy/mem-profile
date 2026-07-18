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
