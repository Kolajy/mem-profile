use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::RwLock;
use std::time::Duration;

static MEMORY_THRESHOLD: AtomicUsize = AtomicUsize::new(usize::MAX);
static THRESHOLD_CALLBACK: RwLock<Option<Box<dyn Fn(usize) + Send + Sync>>> = RwLock::new(None);

static AUDIT_THRESHOLD_NS: AtomicU64 = AtomicU64::new(u64::MAX);
static AUDIT_CALLBACK: RwLock<Option<Box<dyn Fn(Duration) + Send + Sync>>> = RwLock::new(None);

/// Sets a memory threshold alert callback.
/// The callback is triggered when active allocations exceed the threshold.
pub fn set_memory_threshold<F>(threshold_bytes: usize, callback: F)
where
    F: Fn(usize) + Send + Sync + 'static,
{
    let boxed_cb = Box::new(callback) as Box<dyn Fn(usize) + Send + Sync>;
    if let Ok(mut cb) = THRESHOLD_CALLBACK.write() {
        *cb = Some(boxed_cb);
        MEMORY_THRESHOLD.store(threshold_bytes, Ordering::Release);
    }
}

/// Checks if the memory threshold has been exceeded and invokes the callback.
pub fn check_memory_threshold(current_bytes: usize) {
    let threshold = MEMORY_THRESHOLD.load(Ordering::Relaxed);
    if current_bytes > threshold {
        if let Ok(cb) = THRESHOLD_CALLBACK.read() {
            if let Some(f) = cb.as_ref() {
                f(current_bytes);
            }
        }
    }
}

/// Sets a performance audit callback.
/// The callback is triggered when an allocation operation takes longer than the threshold.
pub fn set_performance_audit<F>(threshold: Duration, callback: F)
where
    F: Fn(Duration) + Send + Sync + 'static,
{
    let boxed_cb = Box::new(callback) as Box<dyn Fn(Duration) + Send + Sync>;
    if let Ok(mut cb) = AUDIT_CALLBACK.write() {
        *cb = Some(boxed_cb);
        AUDIT_THRESHOLD_NS.store(threshold.as_nanos() as u64, Ordering::Release);
    }
}

/// Checks if the allocation duration exceeds the audit threshold and invokes the callback.
pub fn check_performance_audit(duration: Duration) {
    let threshold_ns = AUDIT_THRESHOLD_NS.load(Ordering::Relaxed);
    if (duration.as_nanos() as u64) > threshold_ns {
        if let Ok(cb) = AUDIT_CALLBACK.read() {
            if let Some(f) = cb.as_ref() {
                f(duration);
            }
        }
    }
}

/// Fast check to see if performance auditing is enabled.
#[inline(always)]
pub fn is_audit_enabled() -> bool {
    AUDIT_THRESHOLD_NS.load(Ordering::Relaxed) != u64::MAX
}
