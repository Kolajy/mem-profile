## 2024-07-25 - [TUI Empty States]
**Learning:** For TUI widgets (like charts or tables in ratatui), rendering empty vectors natively often results in cluttered visual artifacts (e.g. 0-to-large axis grids) that create poor first impressions.
**Action:** Always conditionally render helpful placeholder messages (like "Waiting for data...") instead of empty charts or grids when state arrays are completely empty.
