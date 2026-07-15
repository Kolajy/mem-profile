## 2024-07-25 - [TUI Empty States]
**Learning:** For TUI widgets (like charts or tables in ratatui), rendering empty vectors natively often results in cluttered visual artifacts (e.g. 0-to-large axis grids) that create poor first impressions.
**Action:** Always conditionally render helpful placeholder messages (like "Waiting for data...") instead of empty charts or grids when state arrays are completely empty.

## 2024-07-26 - [TUI Empty States Highlighting & Toast Feedback]
**Learning:** Highlighting empty state placeholder rows in TUI lists confuses users by indicating interactivity where there is none. Additionally, generic success toasts (like "Snapshot Saved") lack actionable value.
**Action:** Always clear UI list selection states when rendering empty placeholder content. Ensure toast feedback includes context (like filenames) so the user doesn't have to guess or check the filesystem manually.
