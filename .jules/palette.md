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

## 2024-08-02 - [TUI Irrelevant Keybinds]
**Learning:** Rendering or allowing interaction with keybinds (like pause/resume) when the underlying process state makes those actions impossible (e.g., process exited) causes cognitive dissonance and user confusion.
**Action:** Always conditionally hide and ignore keybinds based on the application's lifecycle state to prevent suggesting impossible actions.

## 2024-08-03 - [TUI Primary Metric Highlighting & Peak Tracking]
**Learning:** In terminal user interfaces where historical data rolls off-screen (e.g., rolling time windows), users easily lose track of the session's overall maximums if peak values are not explicitly tracked and displayed. Furthermore, primary metrics styled as plain text can blend into structural borders, reducing visual hierarchy.
**Action:** Always maintain and display historical peak metrics in the UI state when utilizing rolling time windows. Use distinct visual styling (e.g., bold colors via `Line`/`Span`) to separate primary data points from surrounding structural or boilerplate text.

## 2024-11-20 - [TUI Mouse Scrolling]
**Learning:** For TUI applications, enabling `EnableMouseCapture` intercepts mouse scroll events natively. Without an explicit implementation to handle these events in the event loop, mouse scrolling fails silently, creating broken user expectations for users accustomed to mouse navigation in long lists.
**Action:** Always handle `Event::Mouse` (specifically `ScrollUp` and `ScrollDown`) mapping to pagination bounds to ensure a smoother, native scrolling experience when `EnableMouseCapture` is used.
## 2024-11-20 - [TUI Mouse Scrolling Page Jump Fix]
**Learning:** Using wrap-around selection logic (`next`/`previous` that jumps from end to start) with mouse scroll wheels causes rapid disorientation because scroll wheels fire many events quickly. Wrapping should be disabled for mouse scroll.
**Action:** When handling mouse scroll events in TUI applications, map them to bounded step functions (e.g., clamping at list boundaries) rather than wrapping logic or large page jumps to provide a stable scrolling experience.

## 2024-11-20 - [TUI Cursor Hiding]
**Learning:** Terminal cursors left visible in standard TUI applications (like those rendering dashboards) blink continuously and float above UI elements, creating significant visual noise and distraction.
**Action:** Always call `terminal.hide_cursor()` during application setup to ensure a clean, distraction-free visual experience. (Remember to restore it with `terminal.show_cursor()` on exit).

## 2024-11-20 - [TUI Table Jitter on Selection]
**Learning:** In TUI tables, dynamically appearing highlight symbols (like `>> `) when a selection transitions from `None` to `Some(index)` cause the entire table content to shift horizontally, resulting in a jarring visual jitter.
**Action:** Always configure TUI lists and tables (like `ratatui::widgets::Table`) with `HighlightSpacing::Always` (or equivalent spacing rules) to permanently reserve column width for the highlight symbol, ensuring consistent layout regardless of selection state.

## 2024-11-20 - [TUI Right-Aligned Header Jitter]
**Learning:** In TUI tables, dynamically appending sort indicators (like `" ▼"`) to right-aligned column headers causes the primary header text to shift horizontally when the sort state changes, resulting in a jarring layout jitter.
**Action:** Always pad inactive header states with equivalent spaces (e.g., `"Size  "`) to match the width of the active state with the sort indicator. This ensures the text remains firmly anchored in place regardless of the sort state.

## 2024-11-20 - [TUI Visual Hierarchy & Lifecycle Borders]
**Learning:** Bright, stark white default structural borders in terminal user interfaces command too much visual attention, reducing the scannability of the primary data within. Furthermore, relying solely on text indicators for global application states (like "paused" or "exited") is easily missed.
**Action:** Always dim structural boilerplate (e.g., using `Color::DarkGray` for borders) to elevate the primary content. Additionally, dynamically color these structural borders based on the application lifecycle state (e.g., Red when exited, Yellow when paused) to provide an unmissable, ambient visual cue of the application's status.

## 2024-11-20 - [TUI Backtrace Truncation]
**Learning:** In terminal tables with limited column widths, long string data (like hierarchical call stacks formatted root-to-leaf) gets truncated on the right side. This often hides the most crucial piece of information (the leaf function or actual allocator) from the user.
**Action:** Format hierarchical or path-like string data leaf-first (e.g., `leaf <- node <- root`) in TUI columns, ensuring the most specific and important identifying information survives right-side truncation.

## 2024-11-20 - [TUI Thousands Separators for List Totals]
**Learning:** Applying thousands separators (e.g. `num-format`) only to data columns but missing list totals or positional indicators in titles creates an inconsistent experience and leaves large numbers difficult to read.
**Action:** Always format standard numerical data, including positional indicators and list totals (like "X of Y items"), with thousands separators to ensure consistency and improve scannability across the entire UI.
## 2026-07-28 - [TUI Success Empty States]
**Learning:** When an empty state represents a successful outcome (like zero memory leaks at the end of a profiling session), generic messaging like 'No data collected' creates ambiguity and doubt.
**Action:** Use explicit success messaging (e.g., 'Zero leaks detected') and positive styling (e.g., green text) to reassure the user of the successful outcome.
## 2026-07-29 - TUI 80-Column Layout Optimization
**Learning:** In 80-column TUI terminals, long single-line titles containing both status and keybinds cause horizontal truncation. Empty state messages over 50 chars also truncate.
**Action:** Constrain empty states under 50 chars and move static keybind hints to bottom footers using `title_bottom` to preserve primary content space and scannability.

## 2024-05-18 - Contextual Lifecycle Labels in Real-time TUIs
**Learning:** Using static labels like "Current RSS: " in a real-time TUI can cause cognitive dissonance when the monitored process exits and the value becomes fixed.
**Action:** Always use conditionally rendered lifecycle labels (e.g., "Final RSS: " when `process_exited == true`) to provide clear closure and reduce ambiguity in monitoring tools.
## 2024-11-20 - [TUI Flash Messages Truncation]
**Learning:** In terminal user interfaces with strict width constraints (e.g., 80-column defaults), temporary flash messages (like success toasts) can easily be horizontally truncated if static lower-priority information (like permanent keybind hints) persistently occupies the space.
**Action:** Conditionally hide lower-priority permanent information (like static keybind hints) when temporary flash messages are active to ensure the feedback is prominently visible without truncating.

## 2024-08-05 - [TUI Scrollbars for Long Lists]
**Learning:** In terminal user interfaces, users can get lost in long lists (like active allocations) if there is no visual indicator of the current scroll position relative to the total list length.
**Action:** Always add a `Scrollbar` widget to scrollable components (like `Table` or `List`) to provide immediate context on scroll position and list length, improving spatial awareness and overall UX.

## 2024-08-06 - [Signed Differences Formatting]
**Learning:** When displaying difference values (e.g., memory deltas) in a CLI using `.unsigned_abs()` to enable thousands-separators, failing to explicitly handle the negative sign fallback correctly (e.g., setting it to `""` instead of `"-"`) causes negative differences to look like positive increases.
**Action:** Always verify string sign prefixes for negative differences when stripping standard signs, or encapsulate the sign evaluation and formatting into a helper function to ensure consistent and correct representation.

## 2024-08-07 - Explicit Success Messaging in CLI
**Learning:** Generic and ambiguous empty states (like "  None") in CLI outputs leave users uncertain whether an operation succeeded or just failed to generate data, violating UX guidelines established in TUI interfaces.
**Action:** Always replace generic empty state placeholders with explicitly styled success messages (e.g., using ANSI color codes and explicit wording like "✓ Zero net differences detected") to maintain a consistent, positive user experience across all interfaces.
## 2024-11-23 - [Float Formatting with Thousands Separators]
**Learning:** When formatting floats with thousands separators using the `num-format` crate (which only supports integers), manually splitting the float into integer and fractional parts requires explicitly handling the edge case where the fraction rounds up to the base (e.g., 10 for 1 decimal place). Failing to do so causes "1,023.10" instead of "1,024.0", failing tests.
**Action:** Always handle the rounding rollover condition by incrementing the integer part and resetting the fraction to 0 (e.g., `if frac_part == 10`).
