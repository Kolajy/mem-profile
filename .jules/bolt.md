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
