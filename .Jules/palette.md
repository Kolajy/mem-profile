## 2025-03-02 - Conditionally disabling ANSI coloring based on TTY presence
**Learning:** Hardcoded ANSI escape codes in CLI output pollute redirected files (e.g. piping to `> out.txt`) resulting in garbled text. Using `std::io::IsTerminal` to dynamically check if `stdout` or `stderr` is connected to a TTY ensures a cleaner experience for automated workflows while preserving color UX for interactive CLI users.
**Action:** When implementing colored CLI output, always query `is_terminal()` and fallback to plain text if false. Cache this check outside of tight loops to avoid performance overhead from repeated syscalls.
## 2025-03-03 - Contextual Keybinds in TUIs
**Learning:** In TUI applications (like ratatui), presenting all keybinds together globally can lead to visual clutter and suggest actions that are impossible (e.g. scrolling an empty table). Separating them into global controls and widget-specific context controls provides a much cleaner micro-UX.
**Action:** Place global commands (quit, pause) in the main layout block and widget-specific commands (up/down/j/k for tables) in the widget's block. Conditionally hide widget controls when the widget is empty or inapplicable.
