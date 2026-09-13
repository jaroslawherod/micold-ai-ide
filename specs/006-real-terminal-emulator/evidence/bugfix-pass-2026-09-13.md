# 006 T073 — visual pass for BUG-004, BUG-005 and BUG-006

**Date**: 2026-09-13
**Run by**: an agent, not a person at a display. Xvfb `:94` at 1600×1400 with Mesa lavapipe
(software Vulkan), driven with `xdotool` and captured with `import`, following the repo's
`visual-pass` skill.
**Build**: this branch's own `micold-showcase`, `micold-ai-ide` and `micold-daemon`. All three came
from one locked invocation (`Compiling micold-core`, `micold-client` and `micold-daemon` all
printed) and were copied to a private pin directory inside the lock. The client and daemon
hand-shook: the sandbox daemon log shows `client attached to daemon … client_window=<the pinned
client's PID>`.
**Isolation**: private `XDG_RUNTIME_DIR=/tmp/vp94`, plus a scratch `XDG_DATA_HOME`,
`XDG_CONFIG_HOME` and `XDG_STATE_HOME`. The project was a throwaway git repository, seeded through
`projects.json`. No session of the user's was touched, and only this pass's PIDs were stopped.

## Verdicts

| Check | Verdict | Evidence |
|---|---|---|
| BUG-005: showcase `TerminalPane`, light: focused outlined, unfocused not | **PASS** | `bugfix-005-showcase-focus-ring.png` |
| BUG-005: showcase `TerminalPane`, dark: focused outlined, unfocused not | **PASS** | same |
| BUG-005: real app, light: focus ↔ release (Ctrl+Shift+E) | **PASS** | `bugfix-005-app-focus-ring.png` |
| BUG-005: real app, dark: focus ↔ release | **PASS** | same |
| BUG-005: focus never moves the character area or the bar | **PASS** (measured) | — |
| BUG-004: copy chord with nothing selected leaves the clipboard alone | **PASS** | `bugfix-004-006-copy-paste.png` (steps 1–2) |
| BUG-006: paste chord into bash is one bracketed block | **PASS** | same, step 2 |
| BUG-006: context-menu Paste is bracketed | **PASS** | same, step 3 |
| BUG-006: middle-click paste is bracketed | **PASS** | same, step 4 |
| BUG-006: with bracketed paste off, bytes are sent raw | **PASS** (complement) | same, step 5 |

## BUG-005: the focus ring

**Showcase** (the `TerminalPane` section, which poses one unfocused and one focused pane). The
focused pane's edges are exactly the scheme's `secondary`:

- Light: `#625B71` = (98, 91, 113).
- Dark: `#CCC2DB` = (204, 194, 219).

Both match the `terminal.focus_ring` lines in `style_snapshot`. Column-wise the ring is 3 px wide
(x = 28–30 and 1569–1571). Row-wise it falls on a fractional scroll offset, so it rasterises as two
solid rows with an anti-aliased row either side, about 3 px of coverage.

The unfocused pane has no ring. Crop the two panes at identical geometry, aligned on their first
glyph row. Inside the ring the two crops are **pixel-identical** in dark (0 differing pixels). In
light, every differing pixel lies on the ring's rows and columns. The first glyph (`$`) starts at
x = 32 in both, so focus does not move the text. Magnified 7× during the pass, the ring clears that glyph
rather than covering column 0.

**Real app.** A claude session (dark) and then a bash Regular Terminal (light, after switching the
theme in Settings) were each captured focused, then again after Ctrl+Shift+E released focus, with
the pointer parked over the sidebar. In both schemes the whole-frame difference has bounding box
`(306, 65)–(1600, 1336)`:

- It is only three full-length columns (306–308 and 1597–1599) and three full-length rows (65–67
  and 1333–1335).
- **0 differing pixels off the ring.** The sidebar, every terminal cell and the whole 023 bar below
  the pane are unchanged.

That confirms the ring is draw-only: no layout node or bar child moves or appears with focus, and
the process sees the same size either way. The ring's pixel colour in the app matches the showcase
in each scheme.

The crops in `bugfix-005-app-focus-ring.png` are the pane's right-hand corners plus the end of the
bar. The dark rows come from the claude session and the light rows from the bash terminal, which
is why the bar's tabs differ between schemes but never between focus states.

## BUG-004 and BUG-006: copy and paste

This follows the BUG-006 report's own reproduction, in a bash Regular Terminal. Bash's readline
enables bracketed paste. The prompt was set to `PS1='$ '` so no host details appear in the images.

1. `printf 'echo AAA\necho BBB\n'`, then drag-select both lines. Auto-copy puts them on the
   clipboard.
2. Click once to clear the selection, press **Ctrl+Shift+C** with nothing selected, then press
   **Ctrl+Shift+V**:
   - Both lines land on the prompt **unexecuted**, in readline's paste highlight. That is the
     bracketed block (BUG-006; before the fix, `echo AAA` ran).
   - The paste carrying the two lines at all shows the empty copy chord did not overwrite the
     clipboard (BUG-004; before the fix, it wrote an empty string).
3. Right-click → **Paste** on a fresh prompt: the same bracketed block. This is the
   `shell::clipboard::on_paste_requested` path, which has no unit test of its own.
4. **Middle-click** on a fresh prompt: the same bracketed block.
5. The complement, `bind 'set enable-bracketed-paste off'`, then Ctrl+Shift+V: `echo AAA` runs and
   prints `AAA`, and `echo BBB` stays on the prompt. With the mode clear the bytes go through raw,
   so step 2's result follows the process's mode rather than always wrapping.

Two limits on what this shows. The clipboard was not read back with an external tool (`xclip` and
`xsel` are not installed), so BUG-004 is shown through the in-app round trip above; the unit test
`a_copy_chord_with_nothing_selected_leaves_the_clipboard_untouched` pins it directly. The embedded
end-marker stripping was not exercised at the display, because it needs an `ESC` byte on the
clipboard; `tests/keymap.rs` and `a_pasted_end_marker_cannot_close_the_block_early` cover it.

## Observed, not in scope

A single left click in the terminal draws a one-cell highlight at the click point, which stays until
the next click. It is visible in the pass's frames, not in the committed crops. The copyable text of
that selection is empty: step 2's copy chord ran immediately after such a click and wrote nothing.
So it does not affect BUG-004. This branch does not touch the selection code; it is recorded here
rather than filed, pending a look at whether it predates this branch on `main`.

## Not answerable here

- Focus-ring appearance on a real GPU and display, and any transition: none is specified, and the
  ring is on/off.
- Frame pacing under lavapipe.
