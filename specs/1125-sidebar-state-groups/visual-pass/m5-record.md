# M5 visual pass — quickstart Part B 6 and 7

- **Date**: 2026-10-25
- **Where**: cloud container, Xvfb `1600x1400x24`, `ICED_BACKEND=tiny-skia`, driven with xdotool;
  client and daemon pinned from one debug build of this branch at 11749fec (daemon log: `project
  attached`). A scratch git project with *Default* and three managed worktrees (`feat/alpha`,
  `fix/beta`, `docs/gamma`). Light theme.
- **A live session without an agent**: `claude` on the session's `PATH` was a stub that prints a
  prompt and, on cue, posts `UserPromptSubmit` (then `Stop`) to the hook URL and token of the
  `--settings` file the daemon wrote. The daemon's real hook receiver turned those into the
  session's activity, so the row's state changed exactly as with a real `claude`.
- **Harness caveat**: as in M1–M4, frames were judged after a window resize forced a full repaint.

| Step | Check | Verdict | Evidence |
|---|---|---|---|
| B6 | With the terminal released (Ctrl+Shift+E), Tab visits the Overview row, **All**, the type chip, the chip bar's three actions and the *Idle · 4* header, top to bottom; Shift+Tab walks back the same way | PASS | `m5-app-tab-order-light.png` |
| B6 | Enter on the focused header collapses Idle, Space expands it; the ring stays on the header | PASS | `m5-app-header-enter-space-light.png` |
| B6 | Location chords (Ctrl+Shift+. then Ctrl+Shift+,) put the ring on `fix/beta`, then back on `feat/alpha` | PASS | `m5-app-keyboard-moves-with-row-light.png` (left) |
| B6 | A chord into a collapsed Done | not seen live: no row can be Done without a merged pull request (no GitHub here); `tests/sidebar_state.rs` reveal tests (M1) | — |
| B7 | `feat/alpha` has the keyboard; its session starts working: the row moves to a new *Working · 1* group at the top, the ring stays on it, *Idle* counts 3 | PASS | `m5-app-keyboard-moves-with-row-light.png` (right) |
| B7 | Pointer resting on `feat/alpha` in Working (its delete action shown); the session stops and the row moves back into Idle: the ring follows it, it no longer shows its hover action, and *Default*, now under the still pointer, shows none either | PASS | `m5-app-hover-drops-on-move-light.png` |

Found and fixed: the user guide said Tab visits each row. Tab visits the Overview row, the chip
bar and the headers; a row joins the Tab order once the location chords put the keyboard on it
(feature 803). The *Browsing worktrees* bullet now says so.

Observed, not a defect of this milestone: two frames taken while the header had the keyboard showed
a grey layer over the sidebar, gone after the next key press and absent from later frames with the
same focus (a transient of the damage-only repaint).
