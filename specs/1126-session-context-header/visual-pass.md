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

## M3 — the Changes button (T055)

**How it was run.** As for M1 and M2: `ui::view` drawn through tiny-skia from fixture states
(`strip_with(2, 1)`: the worktree `Short` on `feat/short`, pull request #1118 open with its checks
failing, the main session and a helper, both running), at 1280×800 and at 640×480 beside a 300 px
sidebar, light and dark. Presses and edits were applied as the state they leave: the header's
counts read (3 files, +42 −7); the Changes view opened on the shown worktree (the press's message,
H8, is held by `session_area::tests::the_changes_press_opens_the_shown_location_or_closes_its_view`);
closed again; an edit's wake (`Msg::Changed`) with its read under way; the next answer (4 files,
+51 −7); no changes. The live chain (watch → `Changed` → read → answer) is held by
`features_changes::the_header_reads_the_counts_of_the_location_it_follows` and the reducer's
`HeaderCounts` tests, not by a live window (no daemon or display in the cloud container).

| # | Check | Result | Screenshot |
|---|---|---|---|
| B6 | Press Changes; press it again | pass: closed, the outlined button at the header's right reads `Changes · 3 files · +42 −7` on the title line; pressed, the Changes view of *Short* stands under the header with no strip, the button filled as selected; pressed again, the terminal and the strip are back and the button is outlined | `visual-pass/m3-b6-closed-light-1280.png`, `visual-pass/m3-b6-open-light-1280.png`, `visual-pass/m3-b6-open-dark-1280.png`, `visual-pass/m3-b6-again-light-1280.png` |
| B6 at 640×480 | Same at the smallest window | pass: nothing overlaps or leaves the window; the button whole, the chips row whole under it; the title gives way to `fe…` (FR-013 lets it), the pull request chip ends in `…` | `visual-pass/m3-b6-closed-dark-640.png`, `visual-pass/m3-b6-open-light-640.png` |
| B7 | Edit a file in the worktree | pass: while the reread runs the button reads `Changes` alone (no stale figure, US3 scenario 4), then `Changes · 4 files · +51 −7`; with nothing changed, `Changes · no changes` | `visual-pass/m3-b7-reading-light-1280.png`, `visual-pass/m3-b7-after-light-1280.png`, `visual-pass/m3-b7-after-dark-1280.png`, `visual-pass/m3-nochanges-dark-1280.png` |

In dark the selected button's fill is a low-contrast tonal layer, as on the Overview button whose
pattern it follows; its label stays legible. At 640×480 the button leaves the title little room:
noted in the ledger's follow-ups beside the wide-count case.

## Close — the whole Part B on the final tree (T057)

**How it was run.** On `main` at `9c1f758f` (M3 merged, with #1124's status bar and #1125's sidebar
work beside it), the same way as M1–M3: `ui::view` drawn through tiny-skia from the fixture states
(`strip_with`, pull request #1118 open with its checks failing, both sessions running), every frame
in light and in dark, at 1280×800 and at 640×480 beside a 300 px sidebar. 40 frames were drawn;
the ones below are committed. Live chains (presses, the watch, chords) stay held by the tests named
in the M1–M3 sections; there is still no daemon or display in the cloud container.

| # | Check | Result | Screenshot |
|---|---|---|---|
| B1 | Header, strip, terminal; nothing under it but the window's status bar | pass in both themes: title, `Idle`, `feat/short`, `PR #1118 · Open · checks failing`; Changes at the right; `claude · feat/short`, `1`, `2` (marked), "+", `claude · helper-1`, then *running*, New session and the splits | `visual-pass/close-b1-light-1280.png`, `visual-pass/close-b1-dark-1280.png` |
| B2 | Helper's tab; main's AI tab | pass: the helper underlined with its own "+", main's shells gone, header unchanged; the AI tab underlined when marked | `visual-pass/close-b2-helper-dark-1280.png` |
| B3 | "+", confirmation, confirm | pass: a third shell tab, marked, *starting…*; *Start another session?* names the new one a helper; the new helper last, selected | `visual-pass/close-b3-confirm-light-1280.png` |
| B4 | Split side by side | pass: two panes with their own title rows; header and strip once above both | `visual-pass/close-b4-split-dark-1280.png` |
| B5 | AI process stopped | pass: the stopped mark on the AI tab, *idle* and **restart** beside it | `visual-pass/close-b5-stopped-dark-1280.png` |
| B6 | Changes pressed, pressed again | pass: the Changes view of *Short* under the header with no strip, the button filled; again, the terminal and strip back, the button outlined | `visual-pass/close-b6-closed-light-1280.png`, `visual-pass/close-b6-open-dark-1280.png` |
| B7 | An edit | pass: `Changes` alone while the reread runs, then `Changes · 4 files · +51 −7`; `Changes · no changes` with nothing changed | `visual-pass/close-b7-after-dark-1280.png` |
| B8 | Default entry's session | pass: title *Default*, *Idle*, no branch or pull request chip | `visual-pass/close-b8-default-light-1280.png`, `visual-pass/close-b8-default-dark-640.png` |
| B9 | 640×480 beside a 300 px sidebar | pass: nothing overlaps or leaves the window; the pull request chip ends in `…`, the Changes button and splits whole, the tabs scroll; with counts the title gives way to `fe…` (as recorded at M3) | `visual-pass/close-b9-light-640.png`, `visual-pass/close-b9-dark-640.png` |
| B10 | 12 members, the marked one beyond the edge | pass: the strip scrolls, the trailing fade takes the accent, status, New session and splits stay put | `visual-pass/close-b10-twelve-dark-1280.png` |
| B11 | Chords | held by tests as at M2 (`keyboard`, `navigation`, `ui::app_chord_tests`, `terminal_focus`), green in the close gate | — |
| B12 | Dark | pass: every frame above in dark is legible: title, chips, tab labels, status and the Changes label | the `-dark-` frames above |

Nothing changed from M3's frames except what landed beside this feature (the status bar's key
hints and usage items from #1124). Part B is complete.
