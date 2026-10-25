# Visual pass, whole feature (T051 — quickstart Part B, every body branch)

Run 2026-10-25 by the close unit on Xvfb (tiny-skia), a pinned **release** build of M4's head 13beef6d
(client and daemon from one build), 1280 × 800, 900 × 600 and 640 × 480, light and dark. Fixture:
project `micold-ai-ide` with managed worktrees on `fix/1117-x` and on
`feature/a-really-long-branch-name-for-the-title-bar-give-way-check` (a Claude Code session in each),
plus `other-project`; the no-project state from a second launch with an empty catalog. The bar
strips are the window's top 45 rows (the bar, its divider and one body row), one state per row.

| Check | Result | Evidence |
|---|---|---|
| §B1/§B3 One bar in every body branch: session (`fix/1117`), Changes (`fix/1117`), Overview, full-screen Overview (`Overview`), Settings (`Settings`, field disabled, gear selected), no project (`Select project` alone); mark, breadcrumb, centred field, Settings, ⋮; no Overview button | pass | `bars-light-1280.png`, `bars-dark-1280.png`, `bars-light-640.png`, `bars-dark-640.png` |
| FR-001/SC-001 Height: divider on row 43, body from row 44, in session, full screen, Settings and no project, both schemes, both widths (pixel probe at x 1000: rows 42/44 surface, 43 outline) | pass | the strips above |
| Nothing between the bar and the body; the full-screen Overview and Settings keep the bar | pass | `frames-1280.png`, `frames-640.png` |
| §B6 Give-way order on the long branch: 1280 full field; 900 the field drops its words (glyph and hint); 640 glyph only and the place shortens with `…`; the project keeps its name; nothing overlaps or clips | pass | `give-way-640-900-1280.png` |
| §B7 Dark scheme at 640 and 1280 | pass | `bars-dark-*.png`, `frames-*.png` |
| §B2 Switcher hangs from the project segment, top at the divider | pass | `switcher-open-dark.png` |
| §B8 Overview by the chord, in and out of full screen (sidebar row: M4 pass) | pass | `bars-*` Overview rows; `visual-pass/m4/` |

Observation, not a defect of this feature: with the pointer still resting on the project segment
after the press, its `Switch project` tooltip draws over the open panel's first row
(`switcher-open-dark.png`). Every tooltip-carrying menu trigger in the app does the same (the
sidebar's ⋯ shows `More actions` over its open menu); recorded in the ledger's follow-ups.

Not driven here: §B5 tooltips and the §B2 keyboard open (M1–M3 passes and `title_bar_keyboard`,
`switcher_anchor`); the macOS hint `Cmd+Shift+P` (no macOS; `keymap` tests).
