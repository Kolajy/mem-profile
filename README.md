# mem-profile 🧠📊

A high-performance, developer-friendly memory profiling utility and custom global allocator for Rust applications. `mem-profile` helps you track heap allocations, detect memory leaks, identify hot allocation paths, and visualize memory usage with minimal runtime overhead.

Designed with cross-platform compatibility in mind, and optimized for macOS/Linux developer environments.

---

## Key Features

- **`#[global_allocator]` Hooking**: A drop-in custom allocator (`ProfilingAllocator`) wrapper that intercepts all heap allocations and deallocations.
- **Allocation Backtraces**: Optional capturing of call stacks for active allocations using lock-free data structures to identify the exact line of code causing memory growth.
- **Leak Detection**: Automatic reporting of un-deallocated memory upon program termination, complete with allocation backtraces and size metrics.
- **pprof & Flamegraph Integration**: Export profiles in pprof-compatible format or generate interactive SVGs to visualize memory hogs.
- **Threshold Alerts**: Configure runtime callbacks that trigger when memory consumption exceeds user-defined thresholds (ideal for daemon health monitoring).
- **Zero-Cost compilation**: Toggle memory profiling completely off using cargo features to ensure zero overhead in production.

---

## Quick Start

### 1. Add Dependency

Add `mem-profile` to your `Cargo.toml`:

```toml
[dependencies]
mem-profile = "0.1.0"
```

### 2. Register the Allocator

Wrap your existing allocator (e.g., `std::alloc::System` or `jemallocator`) in your `main.rs` or `lib.rs`:

```rust
use mem_profile::ProfilingAllocator;
use std::alloc::System;

#[global_allocator]
static ALLOCATOR: ProfilingAllocator<System> = ProfilingAllocator::new(System);

fn main() {
    // Initialize profiling session
    let _guard = mem_profile::init();

    // Your application code here
    let mut data = Vec::new();
    for i in 0..10000 {
        data.push(format!("allocation-{}", i));
    }
} // Leak detection reports here if guard goes out of scope and memory remains allocated
```

---

## Project Structure

```
mem-profile/
├── Cargo.toml            # Workspace/Package configuration
├── README.md             # Project introduction & user guide
├── src/
│   ├── lib.rs            # Core allocator hook and tracking structures
│   ├── allocator.rs      # ProfilingAllocator implementation
│   ├── backtrace.rs      # Callstack capturing & symbolication
│   └── report.rs         # Reporting engines (JSON, pprof, text summaries)
├── docs/                 # Extended documentation
│   ├── planning.md       # Architectural decisions & implementation details
│   └── roadmap.md        # Feature phases & milestones
└── examples/             # Code examples showing library usage
```

---

## Documentation & Roadmap

For deep dives into project architecture and development planning, please refer to:
* **[Planning & Architecture Guide](docs/planning.md)**: Design choices, lock-free tracking, and backtracesymbolication.
* **[Development Roadmap](docs/roadmap.md)**: Phased milestones, timeline, and feature goals.

## License

This project is licensed under the MIT License - see the LICENSE file for details.