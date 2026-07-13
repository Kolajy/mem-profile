pub mod alert;
pub mod allocator;
pub mod backtrace;
pub mod cli;
pub mod diff;
pub mod flamegraph;
pub mod pprof;
pub mod report;
pub mod snapshot;
pub mod tui;

pub use allocator::ProfilingAllocator;

/// A session guard that can be used to control the lifetime of memory tracking.
/// When dropped, this guard triggers automatic leak detection and prints a report.
pub struct ProfilerGuard;

impl Drop for ProfilerGuard {
    fn drop(&mut self) {
        report::print_leak_report();
    }
}

/// Initializes the global profiler session.
/// Returns a `ProfilerGuard` which triggers leak reports on drop.
pub fn init() -> ProfilerGuard {
    ProfilerGuard
}
