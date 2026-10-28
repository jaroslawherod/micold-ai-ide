# Visual pass: M6 split-pane copy key and diff anchors (quickstart Part B, steps 1 and 9)

The real client and daemon (release build of the M6 branch, with Review A's F1 fix and the fix this
pass led to) on a private Xvfb display, driven with xdotool (XTest), clipboard read with `xclip`.
One git project with two structured sessions (`left` at the project root, `right` in worktree `b`),
each against the daemon tests' `claude` stand-in on a private `PATH` replaying the recorded
`edit-turn` fixture (Write and Edit cards; the Edit diff swaps `banana`/`pear`/`cherry` for
`kiwi`/`pear`/`plum`/`fig`). The window split side by side with the pane strip's split button.
Linux only; macOS and Windows come from the CI-built clients (Principle VI).

| Step | Check | Result | Evidence |
|---|---|---|---|
| 9 | Split, `right` pane active: drag across the `left` pane's reply, Ctrl+C | pass: clipboard `created fruits.txt, replaced every "a` (M5: nothing) | `m6-9-split-pane-selections.png` |
| 9 | Then drag in the `right` pane, Ctrl+C; then the `left` pane again | pass: `eated fruits.txt, replaced eve`, then `herry lines for kiwi/pear/plu`; each pane keeps its own highlight | same |
| 9 | After a restart of the client (layout restored, `left` active): drag in `right`, Ctrl+C; drag in `left`, Ctrl+C | pass: each copied its own pane's text | — |
| 1 | Fast drag on the Edit card's diff from `banana` to `fig` (press, three moves and the release sent without pauses, so they reach the client as one batch) | pass, 3 of 3: clipboard `banana` / `kiwi` / `pear` / `cherry` / `plum` / `fig`, highlight from `banana`'s first letter (M5: started at `fig`) | `m6-1-fast-drag-banana-to-fig.png` |
| 1 | The same drag with pauses | pass: the same six lines | — |
| 1 | Press in `kiwi`, drag along the line to well right of its end, release | pass: `kiwi` (M5: nothing) | `m6-1-drag-right-of-short-line.png` |
| 1 | Fast drag over two lines of prose | pass: anchors at the press, ends at the release | — |

**Found and fixed in this pass.** The first M6 build (with Review A F1's fix, which learned a
scrollable's shift at every frame's redraw) anchored the fast diff drag at `banana` but ended it
~80 px low, in the tool result under the diff. iced_winit hands a redraw the window's cursor while
moves the widgets have not been sent yet are still queued, so the area learned the distance the
pointer was still to travel as a shift. The area now learns the shift only from a redraw whose
cursor is where the last batch ended. Test: `a_redraw_ahead_of_the_moves_shifts_nothing` (red with
the shift learned at every redraw). The table above is from the build with that fix.

The darkened Edit card in the screenshots is the pre-existing tool-card darkening recorded in
M5's pass (*Follow-ups not done*), not M6.
