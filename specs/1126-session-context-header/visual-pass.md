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
