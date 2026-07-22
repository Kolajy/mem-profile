## 2024-07-25 - [TUI Empty States]
**Learning:** For TUI widgets (like charts or tables in ratatui), rendering empty vectors natively often results in cluttered visual artifacts (e.g. 0-to-large axis grids) that create poor first impressions.
**Action:** Always conditionally render helpful placeholder messages (like "Waiting for data...") instead of empty charts or grids when state arrays are completely empty.

## 2024-07-26 - [TUI Empty States Highlighting & Toast Feedback]
**Learning:** Highlighting empty state placeholder rows in TUI lists confuses users by indicating interactivity where there is none. Additionally, generic success toasts (like "Snapshot Saved") lack actionable value.
**Action:** Always clear UI list selection states when rendering empty placeholder content. Ensure toast feedback includes context (like filenames) so the user doesn't have to guess or check the filesystem manually.
## 2024-06-25 - Prevent highlighting empty state rows in TUIs
**Learning:** For TUI lists or tables rendering empty placeholder content, users can be confused if the non-interactive fallback rows are highlighted as if they were selectable data.
**Action:** Strictly clear the UI selection state (e.g., calling `table_state.select(None)`) when the data list is empty to prevent highlighting empty fallback rows.
## 2024-05-18 - TUI Information Density & Discoverability
**Learning:** Terminal User Interfaces are severely constrained by 80-column defaults, easily leading to horizontal truncation of empty states. Additionally, implicit features (like vim-style j/k scrolling) are practically non-existent to users unless explicitly surfaced in minimal hint areas.
**Action:** Always constrain empty state copy to under 50 characters for TUI components that share column widths. Prioritize space-efficient key hints (e.g., `[↑/↓/j/k]`) over verbose ones (`[up/down]`) to maximize feature discoverability without breaking layout.
## 2024-07-27 - [TUI List Scroll Position Indicators]
**Learning:** Terminal tables and lists lack native OS scrollbars. When users scroll through a long list using keys (like `j/k`), they easily lose their sense of position within the dataset, leading to poor orientation and UX.
**Action:** Always include an explicit positional indicator (e.g., "Item 5 of 50") in the title or header of scrollable TUI widgets to provide continuous orientation without consuming extra row space.
## 2024-07-28 - [TUI Lifecycle Empty States]
**Learning:** When designing empty states for real-time monitoring TUI components, generic placeholder text (like 'Waiting for data...') can become inaccurate and confusing if the underlying target process has already terminated without generating data.
**Action:** Always conditionally render empty state copy based on the process lifecycle. If the process has exited (`process_exited == true`), explicitly state that no data was collected rather than implying the system is still waiting.
## 2024-07-29 - [TUI Table Numeric Column Alignment]
**Learning:** In terminal user interfaces, rendering tabular data with default left alignment for numeric columns (like memory size or allocation counts) makes it difficult for users to quickly scan and compare magnitudes.
**Action:** Always right-align numeric columns and their corresponding headers in TUI tables to improve scannability and align with standard data presentation practices.
## 2024-07-30 - [TUI Keyboard Accessibility]
**Learning:** TUI lists mapping only single-item step keys (e.g., j/k or up/down) create friction when users need to scroll through large datasets quickly, negatively impacting UX and accessibility.
**Action:** Always map standard pagination and boundary keys (PageUp, PageDown, Home, End) alongside single-step keys to ensure efficient keyboard navigation in TUI scrollable components.

## 2024-07-20 - TUI Table Selection State Management
**Learning:** In real-time updating TUI lists (like tables that frequently redraw based on changing data sizes), users easily lose their place or encounter silent navigation bugs if the selection index outpaces the changing dataset bounds or if the list sorts change dynamically under them.
**Action:** Always auto-select an item on initial data load, strictly clamp the `TableState` selection index within the bounds of `items.len()` on every render cycle, and explicitly reset selection to index `0` whenever a new sort order is applied to maintain context.
## 2024-08-01 - [TUI Thousands Separators]
**Learning:** For TUI tables displaying large numeric data (e.g., memory counts or sizes), users struggle to parse large string values at a glance if they lack standard thousands separators.
**Action:** Always format numerical data with thousands separators (e.g., using `num-format` with `Locale::en`) to significantly enhance readability and scannability of large values.

## 2026-07-22 - [TUI Flash Messages Truncation]
**Learning:** In terminal user interfaces with strict width constraints (e.g., 80-column defaults), temporary flash messages (like success toasts) can easily be horizontally truncated if static lower-priority information (like permanent keybind hints) persistently occupies the space.
**Action:** Conditionally hide lower-priority permanent information (like static keybind hints) when temporary flash messages are active to ensure the feedback is prominently visible without truncating.
