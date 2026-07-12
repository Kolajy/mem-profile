pub mod allocator;

pub use allocator::ProfilingAllocator;

/// A session guard that can be used to control the lifetime of memory tracking.
/// In subsequent phases, dropping this guard will trigger automatic leak detection.
pub struct ProfilerGuard;

impl Drop for ProfilerGuard {
    fn drop(&mut self) {
        // Output shutdown report/leak report here in later phases.
    }
}

/// Initializes the global profiler session.
/// Returns a `ProfilerGuard` which triggers exit reports on drop.
pub fn init() -> ProfilerGuard {
    ProfilerGuard
}
