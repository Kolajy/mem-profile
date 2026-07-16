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
