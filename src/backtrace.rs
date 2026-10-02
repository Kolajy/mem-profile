use std::path::PathBuf;

/// Symbol information for a specific call frame.
#[derive(Debug, Clone)]
pub struct SymbolInfo {
    pub name: Option<String>,
    pub filename: Option<PathBuf>,
    pub lineno: Option<u32>,
}

impl std::fmt::Display for SymbolInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // ⚡ Bolt: Zero-allocation optimization: Write directly to the `Formatter` stream
        // using sequential `write!` calls instead of eagerly allocating intermediate `String`s
        // (like `.to_string()` or `.into_owned()`). This completely eliminates up to four
        // heap allocations per stack frame when generating memory profiles and flamegraphs.
        let name = self.name.as_deref().unwrap_or("<unknown>");
        write!(f, "{} at ", name)?;
        if let Some(ref filename) = self.filename {
            write!(f, "{}", filename.display())?;
        } else {
            write!(f, "<unknown>")?;
        }
        write!(f, ":")?;
        if let Some(lineno) = self.lineno {
            write!(f, "{}", lineno)
        } else {
            write!(f, "??")
        }
    }
}

/// Captures the raw instruction pointers of the current thread's backtrace.
#[cfg(feature = "capture-backtrace")]
pub fn capture_raw_backtrace() -> Vec<*mut std::ffi::c_void> {
    // Bolt: Pre-allocate capacity for typical backtrace depths (e.g., 32 frames) to avoid multiple
    // reallocations while capturing stack frames during the ultra-hot GlobalAlloc interception path.
    let mut frames = Vec::with_capacity(32);
    // We skip the first few frames to avoid including mem-profile internal functions
    // (e.g. capture_raw_backtrace, allocator hooking frames).
    backtrace::trace(|frame| {
        frames.push(frame.ip());
        true // Continue unwinding
    });
    frames
}

/// Fallback when backtrace capture is disabled.
#[cfg(not(feature = "capture-backtrace"))]
pub fn capture_raw_backtrace() -> Vec<*mut std::ffi::c_void> {
    Vec::new()
}

/// Symbolicates raw instruction pointers into human-readable SymbolInfo.
#[cfg(feature = "capture-backtrace")]
pub fn symbolicate_frames(frames: &[*mut std::ffi::c_void]) -> Vec<SymbolInfo> {
    let mut symbols = Vec::with_capacity(frames.len());
    for &frame in frames {
        backtrace::resolve(frame, |symbol| {
            symbols.push(SymbolInfo {
                name: symbol.name().map(|n| n.to_string()),
                filename: symbol.filename().map(|f| f.to_path_buf()),
                lineno: symbol.lineno(),
            });
        });
    }
    symbols
}

/// Fallback when backtrace symbolication is disabled.
#[cfg(not(feature = "capture-backtrace"))]
pub fn symbolicate_frames(_frames: &[*mut std::ffi::c_void]) -> Vec<SymbolInfo> {
    Vec::new()
}
