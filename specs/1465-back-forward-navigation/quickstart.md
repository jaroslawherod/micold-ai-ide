# Quickstart: Back/Forward Navigation

**Feature**: 1465 | **Plan**: [plan.md](./plan.md) | Contracts: [keys](./contracts/keys.md),
[ui](./contracts/ui.md) | Data model: [data-model.md](./data-model.md)

## Prerequisites

- `mise trust` once in a fresh worktree (in the cloud environment: `scripts/gate.sh`).
- For §B: the `visual-pass` skill's private display, client and daemon built together, a private
  data directory, and `xdotool` (it sends mouse buttons 8 and 9, the X11 side buttons).
- For §C: a macOS bundle from `mise run app`, an installed Windows build, and a mouse with side
  buttons on each.

## §A — Automated

```sh
mise run test-core   # micold-core history: record, step, cap, truncation, gone entries, SC-002 sweep
mise run gate        # keymap H5, app_chord, side_button, terminal pane, tests/history.rs, main_tests, review_keys, docs
```

Expected: both green; each test in [plan.md](./plan.md#test-strategy-by-layer) fails without the
change it covers.

## §B — Recorded pass on Linux (visual-pass)

Project P (active) with Default (session A), worktree `feat-1` (sessions B and B2), worktree
`fix-2` (session C) and a run group G of two runs; project Q (available) with session Q1.

| # | Steps | Expected |
|---|---|---|
| B1 | Click A, then B. Mouse back (`xdotool click 8`). Mouse forward (`click 9`). | A, then B, each with its location row marked in the sidebar. |
| B2 | Click A, B, C. Back twice (A shown). Click B2. Forward, then Back. | Forward does nothing; Back shows A (US1 AS3). |
| B3 | Focus B's terminal (pointer over it, running `cat -v`). Mouse back. | A is shown; `cat -v` printed nothing for the press (FR-008). |
| B4 | In a terminal running `cat -v`, press Alt+← with the terminal focused. | `^[[1;3D` printed; the history did not move (FR-009). |
| B5 | Release the terminal (Ctrl+Shift+E), press Alt+←, then Alt+→. | Back, then Forward. Hold Alt+←: steps back once per repeat until the oldest entry. |
| B6 | Open the Review tab of `feat-1`, follow a link into a browsed file. Alt+← | The diff as it was (same file, scroll, expansion); press again: the place before the Review tab (US3). |
| B7 | Type in the review's comment box; press Alt+← / Alt+→. | The caret moves by word; the history does not move. Mouse back: the history moves (FR-008). |
| B8 | Show A, B, C; close session B. From C, Back. Then show C, then B2, remove worktree `fix-2` from its menu, and from B2 press Back. | First: A, with no notification (US4 AS1). Then: the entry before C is shown, C's entry passed over and gone (AS2). |
| B9 | Open the Overview, Back. Open Compare of G, Back. | The previous session, Overview closed; the place before Compare. |
| B10 | Show Q1 (project switcher), Back. Then in P open Compare of G, show Q1, Back, Forward. | P with the place shown before, without an intermediate P place recorded: Forward returns to Q1. Then P with Compare of G open (its runs' counts read), and Forward returns to Q1. |
| B11 | Open the quick switcher (Ctrl+Shift+P); mouse back and Alt+←. Same with Settings open and with the Review sheet (`?`) open. | Nothing moves (FR-010). |
| B12 | Open the Review sheet with single-key shortcuts on, then off. | Both list *Back* (Alt+←) and *Forward* (Alt+→) (FR-014). |
| B13 | Read `docs/user-guide/terminal-panes.md` §Keyboard shortcuts. | Back and Forward rows with mouse buttons, Alt+←/→ and Cmd+[ / Cmd+], and the focused-field/terminal rule (FR-013). |

## §C — Platforms (Principle VI)

On macOS and on Windows, with a mouse that has side buttons:

| # | Steps | Expected |
|---|---|---|
| C1 | B1 with the mouse's own side buttons. | Same as B1 on both. |
| C2 | macOS: Cmd+[ / Cmd+] with no field focused; then in the review's comment box; then Option+← in that box. | Back / Forward; in the box nothing moves the history and Option+← moves the caret by word. |
| C3 | Windows: Alt+← / Alt+→ with no field focused, then in a terminal running `cat -v`. | Back / Forward; in the terminal `^[[1;3D` and no move. |
| C4 | B3 on each platform. | The terminal program receives nothing for a side-button press. |
