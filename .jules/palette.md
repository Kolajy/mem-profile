## 2026-07-13 - Added TUI Key Hints and Sort Indicators
**Learning:** Adding visual hints and bold colors to shortcuts drastically improves terminal UX, and dynamic sorting arrows make interactive tables far more intuitive.
**Action:** Always add visual indicators for sortable elements and distinguish keyboard shortcuts from generic text in TUIs.
## 2026-07-13 - Dynamic Pause/Resume TUI Text and Empty State
**Learning:** Adding dynamic wording to toggle actions ("pause"/"resume" instead of a static action name) provides immediate, clear feedback on current application state. Adding empty states to TUI tables prevents confusion when waiting for data to populate.
**Action:** Always prefer dynamically labeling toggle shortcuts based on the state they *will* activate or the current status. Always include empty states for lists or tables that may start empty.
