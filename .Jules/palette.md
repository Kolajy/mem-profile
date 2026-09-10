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
