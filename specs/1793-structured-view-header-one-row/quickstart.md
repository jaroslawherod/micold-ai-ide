# Quickstart: feature 1793

## Part A: automated
`scripts/build-lock.sh cargo test -p micold-client features_header_bar`; gates `header_row_fits`, `top_bar_collapse`; then `mise run gate`.

## Part B: visual pass (`visual-pass` skill)
1. Wide window, session selected with 3 hidden events: one 40 px row, `3 hidden` chip, no title row; top bar order ring, status, Changes, `⋮`, settings, menu.
2. Hover ring: tooltip with tokens, cost, turns.
3. `+`: menu with two entries; terminal opens a new tab each press; Escape closes.
4. Worktree `⋮`: raw-events toggle and view control react.
5. Narrow window: ring text hides, then `Changes` label; tabs scroll; no overlap.
6. Overview: none of the four items.

## Part B results (close unit, 2026-11-14, release build of `fa0e41a73`, Xvfb, evidence in `visual-pass/`)

The client's session here is a PTY `claude` (no CLI login), so the structured transcript, its `N hidden` chip and a ring with usage data could not be shown; the strip, top bar, menus and narrow behaviour are real. Windows 1400, 1024, 800 and 640 px wide.

| Step | Result | Evidence |
|---|---|---|
| 1 | Partial: one 40 px strip, no title row, top bar order ring, status, Changes, `⋮`, settings, menu. The `3 hidden` chip not seen (no structured transcript); chip is covered by `header_row_fits` and `session_area_stability` press test | `B1-wide-strip.png`, `B5-wide-1400.png` |
| 2 | Partial: ring tooltip shows, but with `No usage yet` (no session usage) | `B2-ring-hover.png` |
| 3 | Pass: `+` opens a two-entry menu (`Another session`, `Terminal`); `Terminal` opens a tab. Escape closing not confirmed (key went to the terminal); the Escape path is unit tested. MINOR: with the pointer still on `+` its tooltip overlaps the first entry | `B3-plus-menu.png`, `B3-terminal-opened.png` |
| 4 | Pass: worktree `⋮` holds `Show raw events` (Off then On) and the Structured/Terminal control, both react; the menu stays open after the view control. MINOR: its first row looks dimmed after the swap | `B4-worktree-menu.png`, `B4-raw-events-on.png`, `B4-after-settle.png` |
| 5 | Pass: at 800 px the Changes label drops to its icon, the Jump field to its icon at 640 px, tabs scroll under a fade, no overlap. Tabs scroll rather than shrink first (FR-011, recorded follow-up) | `B5-narrow-800.png`, `B5-narrower-640.png` |
| 6 | Pass: Overview shows no ring, status, Changes or `⋮` | `B6-overview.png` |
