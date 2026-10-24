# Visual pass: #1126 session context header

## M1 — the header above the terminal (T020)

**How it was run.** In the cloud container, with no daemon or GitHub to drive a live window, the
window was drawn by the application's own `ui::view` through its tiny-skia renderer (the renderer
`ICED_BACKEND=tiny-skia` selects), from fixture states built like the covered states: a project with
the worktree `1117_ci-red-on-main` on `fix/1117_ci-red-on-main`, its issue title *CI red on main:
daemon tests* held, pull request status on with #1118 open and its checks failing, one session
shown; the same with the session in the Default entry; and the same with a long issue title and
worktree name. Each at 1280×800 and at 640×480 beside a 300 px sidebar, light and dark. Live
updates (a session starting, a reading arriving) are held by
`session_area::tests::the_header_shows_a_change_on_the_refresh_the_sidebar_shows_it`, not here.

| # | Check | Result | Screenshot |
|---|---|---|---|
| B1 (header part) | Worktree session shown: above the terminal, the issue title, then chips: state with icon and word, `fix/1117`, `PR #1118 · Open · checks failing` led by the sidebar's indicator; the bar under the terminal unchanged | pass. The state reads *Needs attention* with the eye icon on the warning fill: the Overview card's state for failing checks (row 5), the same as the card | `visual-pass/m1-worktree-light-1280.png` |
| B8 | Default entry's session: title *Default*, its state chip, no branch or pull request chip | pass (*Default*, *Idle*, nothing else) | `visual-pass/m1-default-light-1280.png`, `visual-pass/m1-default-dark-640.png` |
| B9 (header part) | 640×480 beside a 300 px sidebar: nothing overlaps or leaves the window; title and chips end in `…` | pass: the long title ends *daemon tes…*, the pull request chip *PR #1118…*; state and branch chips whole | `visual-pass/m1-long-light-640.png`, `visual-pass/m1-worktree-light-640.png` |
| B12 (B1, B9 in dark) | Same in dark, all text legible | pass: the title on the dark surface, the state chip's dark text on its warning fill, the neutral chips' light text on their tint | `visual-pass/m1-worktree-dark-1280.png`, `visual-pass/m1-long-dark-640.png` |

Tooltips (full title with the worktree name, the state's reason, the full branch, the pull request
with the stale note) are not on these frames; their text is held by core tests H1–H4 and their
placement by the shared `Tooltip`.

## M2 — the strip replaces the bar (T038, T042)

**How it was run.** As for M1: `ui::view` drawn through tiny-skia from fixture states, here the
covered states' `strip_with` (a worktree `Short` on `feat/short`, pull request #1118 open with its
checks failing, the main session with two shell instances, one helper), with both sessions'
processes running, at 1280×800 and at 640×480 beside a 300 px sidebar. Presses were applied as the
state they leave (the helper shown, the AI tab marked, a third shell opened, the confirmation
open, a helper appended and shown, a pane split, the AI process idle, nine shells with the ninth
marked). The press routing itself is held by `terminal_tabs::each_strip_member_sends_its_s5_message`
and the S5 unit tests, the menus by `a_members_menu_offers_restart_exactly_when_it_wears_the_stopped_mark`.

| # | Check | Result | Screenshot |
|---|---|---|---|
| B1 | Header, then the strip, then the terminal; nothing under it | pass: `claude · feat/short`, its shells `1` `2` (2 underlined, marked), "+", `claude · helper-1`, then *running*, New session and the two split buttons at the right; the window's status bar is the only thing under the pane | `visual-pass/m2-b1-light-1280.png` |
| B2 | Helper's tab; main's AI tab | pass: the helper shown and underlined, its own "+" after it, the main session's shells gone, header unchanged; with the AI tab marked, `claude · feat/short` is underlined | `visual-pass/m2-b2-helper-light-1280.png`, `visual-pass/m2-b2-main-ai-light-1280.png` |
| B3 | "+", New session's confirmation, confirm | pass: a third shell tab, marked; the confirmation *Start another session?* names the new session a helper; the new helper's tab is last, selected and shown (restored idle in the fixture, so it wears the stopped mark and offers restart) | `visual-pass/m2-b3-plus-light-1280.png`, `visual-pass/m2-b3-confirm-light-1280.png`, `visual-pass/m2-b3-new-helper-light-1280.png` |
| B4 | Split side by side | pass: two panes, each with its own title row; header and strip once above both | `visual-pass/m2-b4-split-light-1280.png` |
| B5 | AI process stopped | pass: the stopped mark on `claude · feat/short`, status *idle* and **restart** beside it | `visual-pass/m2-b5-stopped-light-1280.png` |
| B9 | 640×480 beside a 300 px sidebar | pass: nothing overlaps or leaves the window; status, New session and splits whole; the tabs keep about 160 px and scroll (scroll bar under them) | `visual-pass/m2-b9-light-640.png`, `visual-pass/m2-b9-dark-640.png` |
| B10 | 12 members, the marked one beyond the trailing edge | pass: the strip scrolls, the trailing fade takes the accent, status, New session and splits stay at the right | `visual-pass/m2-b10-twelve-light-1280.png` |
| B11 | Every chord of `terminal-panes.md#keyboard-shortcuts` with the terminal focused | held by tests, not a live window (no daemon or display in the cloud container): `keyboard`, `navigation` and `ui::app_chord_tests` are unchanged and green (T041), and `terminal_focus`'s strip-press guard shows a strip press leaves focus where the bar's control left it. Linux only; CI runs those tests on Linux, macOS and Windows | — |
| B12 | B1, B9 in dark | pass: tab labels, status and controls legible on the dark surface | `visual-pass/m2-b1-dark-1280.png`, `visual-pass/m2-b9-dark-640.png` |
