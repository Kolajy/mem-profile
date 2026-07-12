# mem-profile 🧠📊

A high-performance, developer-friendly terminal-based memory profiling utility and custom global allocator for Rust applications. `mem-profile` helps you track heap allocations, detect memory leaks, identify hot allocation paths, and visualize memory usage with minimal runtime overhead.

Designed with cross-platform compatibility in mind, and optimized for macOS/Linux developer environments.

Our ultimate vision is for `mem-profile` to be packaged as a Homebrew core formula (`brew install mem-profile`), serving as the go-to terminal-based memory visualizer for developers.

---

## 🚀 Key Features

- **`#[global_allocator]` Hooking**: A drop-in custom allocator (`ProfilingAllocator`) wrapper that intercepts all heap allocations and deallocations.
- **Allocation Backtraces**: Capture raw call stacks at allocation time using thread-safe, sharded data structures to identify the exact line of code causing memory leaks.
- **Leak Detection**: Automatic reporting of un-deallocated memory upon program termination, complete with lazy symbolication and size metrics.
- **Signal-triggered Snapshots**: Capture heap snapshots at runtime programmatically or on-demand via Unix signals (`SIGUSR1`/`SIGUSR2`).
- **pprof & Flamegraph Export**: Export profiles in pprof-compatible format or generate interactive SVGs directly.
- **Threshold Alerts**: Trigger runtime callbacks when allocation operations take too long or when memory consumption breaches defined limits.
- **Terminal UI (TUI) & ASCII Graphing**: Run external commands directly under profiling and view interactive, real-time timeline graphs in your console.

---

## 📦 Homebrew Ready Vision

To meet the high standards for inclusion as a core Homebrew package:
1. **Interactive TUI Dashboard**: We are building a real-time, interactive terminal dashboard (utilizing `ratatui`) to view memory growth timelines, active allocation stack trees, and flamegraphs inside the terminal window.
2. **Zero Runtime Dependencies**: The CLI compiles to a statically linked binary with zero external runtime dependencies on both macOS and Linux.
3. **Cross-Platform Release Automation**: Statically linked pre-compiled releases built automatically via GitHub Actions for Apple Silicon/Intel macOS and Linux.

---

## 🛠️ Project Structure

```
mem-profile/
├── Cargo.toml            # Workspace/Package configuration
├── README.md             # Project introduction & user guide
├── src/
│   ├── lib.rs            # Core allocator hook and tracking structures
│   ├── allocator.rs      # ProfilingAllocator implementation
│   ├── backtrace.rs      # Callstack capturing & symbolication
│   ├── report.rs         # Reporting engines (JSON, text summaries)
│   ├── snapshot.rs       # Disk-based snapshots & UNIX signaling
│   ├── pprof.rs          # pprof format serialization
│   ├── flamegraph.rs     # SVG flamegraph generator
│   ├── alert.rs          # Memory threshold alerts & performance callbacks
│   └── bin/
│       └── mem-profile-cli.rs  # CLI command runner wrapper
├── docs/                 # Extended documentation
│   ├── planning.md       # Architectural decisions & implementation details
│   └── roadmap.md        # Feature phases & milestones
└── tests/                # Integration test suites
```

---

## License

This project is licensed under the MIT License - see the LICENSE file for details.