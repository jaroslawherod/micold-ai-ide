# Visual pass, milestone M4 (quickstart §B 8 — the Overview button leaves the bar)

Run 2026-10-25 on Xvfb (tiny-skia), a pinned build of M4 (client and daemon from one build), 1280 × 800,
fixture project `micold-ai-ide` with a `fix/1117-x` worktree, light scheme. Frames d0–d5 are 40% scale.

| Check | Result | Evidence |
|---|---|---|
| §B1/§B8 The bar holds the mark, `micold-ai-ide ▾` breadcrumb, the field, Settings, ⋮ — no Overview button or badge; the sidebar's first row is **Overview** | pass | `b1-bar-no-overview-1280.png` |
| §B8 Press the sidebar row: the Overview opens (place reads `Overview`); pressing it while open changes nothing (US2.3) | pass | `d0-start.png` (open), `d1-row.png` (still open after a press) |
| §B8 Ctrl+Shift+O closes, again opens it | pass | `d2-chord-closes.png`, `d3-chord-opens.png` |
| §B8 Full screen (Ctrl+Shift+F), then Ctrl+Shift+O: closes, sidebar returns | pass | `d4-full.png`, `d5-chord-closes-from-full.png` (sidebar mid-slide) |
| Opening from the closed state by the row | pass, seen in an earlier run (the row press opened it); not kept as a frame | — |
