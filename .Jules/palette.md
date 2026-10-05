## 2025-03-02 - Conditionally disabling ANSI coloring based on TTY presence
**Learning:** Hardcoded ANSI escape codes in CLI output pollute redirected files (e.g. piping to `> out.txt`) resulting in garbled text. Using `std::io::IsTerminal` to dynamically check if `stdout` or `stderr` is connected to a TTY ensures a cleaner experience for automated workflows while preserving color UX for interactive CLI users.
**Action:** When implementing colored CLI output, always query `is_terminal()` and fallback to plain text if false. Cache this check outside of tight loops to avoid performance overhead from repeated syscalls.
## 2025-03-03 - Contextual Keybinds in TUIs
**Learning:** In TUI applications (like ratatui), presenting all keybinds together globally can lead to visual clutter and suggest actions that are impossible (e.g. scrolling an empty table). Separating them into global controls and widget-specific context controls provides a much cleaner micro-UX.
**Action:** Place global commands (quit, pause) in the main layout block and widget-specific commands (up/down/j/k for tables) in the widget's block. Conditionally hide widget controls when the widget is empty or inapplicable.
## 2025-03-04 - ANSI Color Coding in CLI Output
**Learning:** In CLI reporting tools, applying ANSI color codes to headers, bolding key labels (e.g., 'Command:', 'PID:'), and highlighting primary metrics with distinct colors significantly enhances visual hierarchy and scannability for interactive users.
**Action:** Implement color enhancements for terminal output while strictly adhering to previous learnings by always wrapping them in a `std::io::IsTerminal` check to ensure graceful fallback to plain text in non-TTY environments.
## 2025-03-05 - Styling ASCII Charts in CLI
**Learning:** ASCII charts printed to stdout can look like dense walls of text, making it hard to distinguish axes, labels, and data series at a glance.
**Action:** Apply targeted ANSI color codes to ASCII charts (e.g. coloring data series, axes, and bounds differently) to improve data scannability for interactive users, ensuring plain-text fallback for non-TTY environments.

## 2024-05-13 - [Color-code memory diff report]
**Learning:** In CLI diffing reports (such as memory diffs), improve scannability by color-coding changes: use red for increases (e.g., leaks or new paths), green for decreases (e.g., freed memory), and neutral styling like grey for zero differences. However, avoid hardcoding raw ANSI escape sequences (e.g., `\x1b[31m`) directly into formatting logic, as it can pollute redirected file outputs in non-TTY environments.
**Action:** When formatting signed differences (e.g., memory deltas) in CLI outputs using `num-format` and `.unsigned_abs()`, encapsulate the sign logic (explicitly adding `+` or `-`) and string formatting into a dedicated helper function to ensure correctness and prevent automated review systems from misinterpreting inline sign concatenation.
## 2024-09-13 - Inactive Header Contrast
**Learning:** Using `DarkGray` for inactive table headers causes accessibility and readability regressions on standard dark terminal backgrounds. While `DarkGray` is suitable for dimming structural boilerplate, interactive structural elements like sortable headers need higher baseline contrast.
**Action:** Use `Color::Gray` instead of `Color::DarkGray` for inactive but interactive TUI table headers to ensure sufficient contrast while maintaining the visual hierarchy against the active header column.
## 2024-09-14 - Improve TUI Table Header Contrast
**Learning:** Using `Color::DarkGray` backgrounds or text for inactive table headers in TUIs can cause severe readability and accessibility regressions on standard dark terminal themes, rendering them nearly invisible.
**Action:** Use `Modifier::UNDERLINED` for structural separation and rely on inherited terminal default colors (e.g. `Style::default()`) for inactive states to maintain high contrast.
## 2025-03-06 - Dynamic Structural Colors in TUIs
**Learning:** In TUI applications, ensure cohesive visual state indication by aligning the semantic color of structural elements (like dynamic borders) with their corresponding state text (e.g., using `Color::Green` for both the border and `[RUNNING]` text), avoiding mismatched combinations with dim boilerplate colors.
**Action:** When a structural element (like a block border) is used to convey application state dynamically, always coordinate its color exactly with the state text. Never hardcode these state-driven structural colors to neutral boilerplate colors like `DarkGray`.
## 2025-03-08 - Explicit Verification Before Planning
**Learning:** When terminal output from exploration commands (like `cat` or `sed`) is truncated, guessing the unseen code to build SEARCH/REPLACE blocks will fail the Groundedness and Exploration rules during plan review.
**Action:** When terminal output is truncated, never guess the unseen code. Instead, use targeted searches (e.g., `grep -n -A 15 "<specific_code_signature>"`) to explicitly read and verify the exact file contents before proposing an execution plan.
## 2025-03-08 - Styling ASCII Charts in CLI (Data Series and Axes)
**Learning:** ASCII charts printed to stdout can look like dense walls of text, making it hard to distinguish axes, labels, and data series at a glance.
**Action:** Apply targeted ANSI color codes to ASCII charts (e.g. coloring data series, axes, and bounds differently) to improve data scannability for interactive users, ensuring plain-text fallback for non-TTY environments.
## 2025-03-09 - Cross-Target Visual Consistency
**Learning:** Ratatui Charts with unstyled (Span::raw) labels can look like dense walls of text, similar to unstyled ASCII charts. When an application supports multiple rendering targets (e.g. interactive CLI and rich TUI), maintaining consistent semantic color mapping across them (e.g. green for time, magenta for bytes) reduces cognitive load as users switch between modes.
**Action:** Always apply explicit, targeted styles (like `Span::styled`) to TUI chart labels and axes, ensuring the chosen colors match the semantic colors used in the application's interactive CLI equivalents.
## 2025-03-09 - Color-code memory chart axes
**Learning:** In TUI applications built with `ratatui` that also feature CLI equivalents, maintain consistent semantic color mapping across rendering targets to reduce cognitive load. Always apply explicit styles (e.g., `Span::styled`) to TUI chart labels and axes, ensuring their colors match the semantic colors used in the CLI output (e.g., green for time, magenta for bytes) rather than leaving them unstyled or default gray.
**Action:** Always verify color mapping matches standard palette.
## 2025-03-09 - Explicit Check Before Patching
**Learning:** When testing code modifications directly via shell commands (like using `sed`) prior to finalizing an execution plan, ensure you revert those changes (e.g., using `git restore <file>`) before executing `replace_with_git_merge_diff`. Failing to do so causes the `SEARCH` block to fail because the target file already contains the modified state.
**Action:** Before executing `replace_with_git_merge_diff`, run `git status` or `git diff` to confirm the target file is in the expected initial state. If modified, use `git restore <file>` to revert it so the SEARCH block matches perfectly.
## 2025-03-09 - Improve TUI Boilerplate Contrast
**Learning:** Using `Color::DarkGray` for dim structural boilerplate text (like inactive keybinds or static labels) in TUIs can cause severe readability and accessibility regressions on standard dark terminal themes, rendering them nearly invisible.
**Action:** Use `Color::Gray` instead of `Color::DarkGray` for dim boilerplate text to ensure sufficient contrast and legibility while maintaining the visual hierarchy against active content.
## 2025-03-09 - Route CLI Warnings to Stderr
**Learning:** Diagnostic messages and usage errors printed to `stdout` can corrupt piped output streams in CLI applications, leading to broken downstream automation workflows. Checking `std::io::stderr().is_terminal()` allows for correct conditional ANSI coloring on the standard error stream.
**Action:** Always route warnings and error messages to `stderr` via `eprintln!` and verify terminal capabilities using the standard error handle rather than standard output.
## 2024-11-20 - TUI Accessibility & State Cohesion
**Learning:** In TUI applications built with ratatui, ensuring cohesive visual state indication is critical. Aligning the semantic color of structural elements (like dynamic borders) with their corresponding state text (e.g., using `Color::Green` for both the border and `[RUNNING]` text) provides crucial at-a-glance status cues, avoiding mismatched combinations with dim boilerplate colors.
**Action:** When updating TUI state indicators, ensure the border style explicitly matches the color of the state text being rendered.
## 2025-03-09 - Cohesive Error Guidance
**Learning:** Generic fallback states (like "No memory data collected") can feel like error messages to users when they are actually expected outcomes for edge cases (e.g., short-lived processes). Matching fallback text to interactive CLI equivalents and adding subtle iconography (like an info icon) provides reassuring, actionable guidance and reduces cognitive friction.
**Action:** Always explain *why* an empty state occurred if it could be confused with a failure, and coordinate the language between CLI outputs and rich TUI outputs to ensure consistent user onboarding.
## 2025-03-09 - Empty state icons consistency
**Learning:** Empty states in TUIs without icons can be overlooked. In TUI empty states ("No allocations tracked"), prepending a subtle `ℹ` icon improves visual scannability and provides actionable guidance, keeping it consistent with the success empty states (which have a `✓`).
**Action:** Add `ℹ` to empty states without an icon.
## 2025-03-09 - TUI Status Indicators Scannability
**Learning:** In TUI applications, textual state indicators like `[RUNNING]` or `[PAUSED]` can lack immediate visual distinction, even with colors. Adding universally understood Unicode symbols (e.g., `▶`, `⏸`, `■`) alongside the text significantly improves at-a-glance scannability and provides actionable guidance, reducing cognitive friction for interactive users.
**Action:** When designing TUI status badges or interactive key hints, prepend the text with appropriate Unicode icons (e.g., `[▶ RUNNING]`, `⏸ pause`) to enhance visual hierarchy and scannability.
## 2025-03-09 - Remove Duplicate Keybinds
**Learning:** In TUI applications, redundantly duplicating keybind hints across multiple layout blocks causes visual clutter.
**Action:** Contextually separate them by placing global application commands in the primary block's footer and widget-specific controls in the relevant widget's footer. Conditionally hide widget-specific controls when inapplicable.
## 2025-03-09 - Keybind Hints Scannability
**Learning:** In TUI applications, blended bracketed keybind hints like `[s]napshot` look like a typo or are hard to quickly scan.
**Action:** Use distinct spacing and universally understood Unicode icons (e.g., `[s] 📸 snapshot`) to make them more scannable and intuitive.
