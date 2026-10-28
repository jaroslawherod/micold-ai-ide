# Visual pass: M4b selection in the transcript (quickstart Part B, steps 1–5)

The real client and daemon (release build of the M4b branch) on a private Xvfb display (lavapipe
Vulkan), driven with xdotool (XTest, buttons held ~100 ms), clipboard read with `xclip`. The
structured session ran against the daemon tests' `claude` stand-in on a private `PATH`, replaying the
recorded `markdown-turn`, `tool-turn` and `edit-turn` fixtures, and a `long` turn with no result so
the session stayed busy. Linux only; macOS and Windows come from the CI-built clients (Principle VI).

| Step | Check | Result | Evidence |
|---|---|---|---|
| 1 | Drag from one entry, through a heading and a paragraph, into a list: the highlight follows across entries; Ctrl+C puts the shown text on the clipboard, no `**`, bullets as `•`, a line break between lines and entries | pass (clipboard: `claude-sonnet-5-5 · <CWD>` / `Brewing Tea` / `The Basics` / `Good tea needs only water, leaves …` / `• Choose your leaves` / `• Green: gentle and`) | `m4b-1-drag-across-entries.png` |
| 2 | Drag held below the bottom edge: the transcript scrolls and the selection grows (through code blocks, whose lines highlight) | pass | `m4b-2-edge-scroll.png` |
| 3 | A single click clears the selection; the clipboard still holds step 1's text (neither the step 2 release nor the click wrote it) | pass | — |
| 4 | Double-click a word (`notes`), triple-click a code line (`1 tsp leaves`) and a wrapped line of a raw event (that visual line only); Ctrl+A then Ctrl+C (73 lines, every entry); right-click with a selection: Copy and Select all at the press point, selection unchanged, Copy copies it; right-click with none: Copy disabled; Escape closes the menu | pass | `m4b-4-triple-click.png`, `m4b-4-menu-with-selection.png`, `m4b-4-menu-copy-disabled.png` |
| 5 | Busy turn: select a word, Ctrl+C copies it and the turn keeps **Working**; click into the composer, right-click the transcript → Copy still copies the transcript selection; click to clear, Ctrl+C sends one interrupt (the stand-in's stdin shows exactly one `interrupt` control request across the pass) | pass | `m4b-5-busy-copy.png` |
| M4a follow-up | A selection across a folded Edit card highlights its title and its (cut) summary | pass | `m4b-folded-summary-highlight.png` |

Not covered here: steps 6–9 (copy controls, streaming, light/dark and CJK, panes and windows) are
M5's pass. The M3 note's kerning shift between a selected and an unselected proportional `Text`
was not visible at 1×.
