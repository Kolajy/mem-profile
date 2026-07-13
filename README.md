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

## 📖 Usage Guide

`mem-profile` can be used either as a command-line tool (CLI) to profile external binaries or integrated directly into your Rust code as a programmatic library.

### 1. Command-Line Interface (CLI)

The `mem-profile-cli` binary supports running commands under profiling, attaching to existing PIDs, or wrapping Cargo commands.

* **Profile an arbitrary command:**
  ```bash
  cargo run --bin mem-profile-cli -- run <command> [args...]
  ```
  *Example:*
  ```bash
  cargo run --bin mem-profile-cli -- run sleep 1
  ```

* **Attach to an already running process by PID:**
  ```bash
  cargo run --bin mem-profile-cli -- attach <PID>
  ```

* **Run as a Cargo subcommand wrapper:**
  ```bash
  cargo run --bin mem-profile-cli -- cargo build
  ```

### 2. Programmatic Integration (Library)

To use the custom global allocator and export memory reports or flamegraphs directly from your code:

* **Declare the Global Allocator:**
  ```rust
  use mem_profile::ProfilingAllocator;
  use std::alloc::System;

  #[global_allocator]
  static ALLOCATOR: ProfilingAllocator<System> = ProfilingAllocator::new(System);
  ```

* **Initialize and Generate Reports:**
  ```rust
  fn main() {
      // Starts tracking allocations
      let _guard = mem_profile::init();

      // ... your application logic ...

      // Export a flamegraph SVG to visualize memory hot paths
      let _ = mem_profile::report::write_flamegraph("flamegraph.svg");
  }
  ```

* **Run the Example:**
  ```bash
  cargo run --example test_flamegraph
  ```

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