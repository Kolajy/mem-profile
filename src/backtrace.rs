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
        let name = self.name.as_deref().unwrap_or("<unknown>");
        let file = self
            .filename
            .as_ref()
            .map(|p| p.to_string_lossy().into_owned())
            .unwrap_or_else(|| "<unknown>".to_string());
        let line = self
            .lineno
            .map(|l| l.to_string())
            .unwrap_or_else(|| "??".to_string());
        write!(f, "{name} at {file}:{line}")
    }
}

/// Captures the raw instruction pointers of the current thread's backtrace.
#[cfg(feature = "capture-backtrace")]
pub fn capture_raw_backtrace() -> Vec<*mut std::ffi::c_void> {
    let mut frames = Vec::new();
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
    let mut symbols = Vec::new();
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
