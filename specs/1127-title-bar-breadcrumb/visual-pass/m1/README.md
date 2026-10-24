# Visual pass, milestone M1 (quickstart §B 1, 5, 7 — M1 parts)

Run 2026-10-24 on Xvfb (tiny-skia), the pinned M1 build, 1280 × 800 and 640 × 480.

| Check | Result | Evidence |
|---|---|---|
| Bar 44 px: divider on row 43, body from 44 | pass (pixel probe) | `bar-light-1280.png` |
| Order: app mark, project switcher, Overview, then Settings and the overflow trigger at the end; no title text | pass | `bar-light-1280.png`, `bar-dark-640.png` |
| App mark tooltip names the application; Settings tooltip; Settings drawn selected while open, Overview inert | pass | `tooltips-and-settings-selected.png` |
| Overflow menu under its trigger, top at y 44, same three items | pass (pixel probe) | `overflow-menu-dark.png` |
| Switcher panel top at y 44, leading edge at the switcher's x 48 | pass (pixel probe) | `switcher-panel-dark.png` |
| Dark scheme | pass | `bar-dark-640.png` |
