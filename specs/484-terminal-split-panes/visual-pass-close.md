# Visual pass, close (quickstart Part B steps 1-5, 7, 8, 9)

2026-10-09. Real client and daemon (debug build of this branch, built from the working tree and
copied out before launch) on a private Xvfb 1600x1000, lavapipe (software Vulkan), window 1500x900,
two seeded git projects A and B, private `XDG_DATA_HOME` and runtime dir. Driven with xdotool.
Steps 6 and the header/divider looks are in `visual-pass-m3.md` and were not repeated. Not done: dark
scheme for the focus mark (step 3, light only), step 3b (selection, links, scrollback).

| Step | Result | Evidence |
|---|---|---|
| 1 Split (Ctrl+Shift+D) | pass | Two panes; new one is empty, shows "Choose a terminal for this pane" and is focused |
| 2 AI CLI left, shell right | pass | Claude Code left, `bash` of the same session right, both draw and update |
| 3 Typing reaches only the focused pane | pass (light only) | `echo hello-right; stty size` appeared in the focused shell only; Down arrow did not move the unfocused Claude theme list. Focus mark = tinted header, accent strip, outlined body (not colour alone) |
| 4 Click an unfocused pane | pass | Click on Claude pane moved the mark, theme list unchanged (no input); next Down key moved it |
| 5 `stty size` vs divider drag | pass | Panes of 37 and 75 columns reported `40 37` and `40 75`; after the drag both reported `40 50` and measure 50 columns on screen |
| 7 Six panes, seventh refused | partial | Six panes reached (header buttons and chords). The 7th split, and the splits refused at 4 panes for width, changed nothing but showed no reason text, by chord or header button, at 0.15 s to 3 s after |
| 8 Restart restores | pass | See below |
| 9 Idle CPU, 6 vs 1 pane | fail | See below |

## Step 8

- Layout, ratios and focus: project A (six panes, nested splits, ratio 0.665) and project B (two
  panes) both came back with identical geometry. Started directly on A, focus was on pane 2 (the
  persisted value); started directly on B, focus was on the right pane (persisted value).
- Shell terminals are not restarted, so their panes come back as "Empty pane" pickers; the AI CLI
  pane is live. Sessions with no recorded conversation are pruned at attach (existing behaviour), so
  the seeded sessions were given a title to survive the restart.
- Corrupting B's `pane_layout` (`"root": "junk"`, `focused: 99`) in `projects/<id>.json`: B starts
  with one pane showing its session; no error.
- Side observation: switching project (B to A and back) rewrites the persisted focus to the pane
  that shows the foreground terminal (both went to pane 1). A focus resting on an empty pane is lost
  on a project switch. Clicking an empty pane's header or body does not focus it.

## Step 9, idle CPU of the client process

Both runs: shells only, no output, no input, 5 s settle, `utime+stime` from `/proc/<pid>/stat`.

| Layout | Window 1 | Window 2 |
|---|---|---|
| 1 pane (B, one shell) | 313 ticks / 30 s = 10.4 % | 314 ticks / 30 s = 10.5 % |
| 6 panes (A, six shells) | 569 ticks / 30 s = 19.0 % | 568 ticks / 30 s = 18.9 % |

Six panes cost +81 % over one pane, against a limit of +10 %. Idle is not zero in either case
(cursor blink redraws, software rasteriser), so the absolute numbers say nothing about a real GPU;
the ratio is the finding. Not isolated: whether the extra is per-pane redraw cost per blink or
something that runs per pane while idle.

## Found, not fixed

- Step 7: the refusal reason is not visible for a refused split. `on_pane_msg` clears `pane_refusal`
  on every pane message, and showing the reason adds a row above the split view, which resizes the
  panes and may send `PaneMsg::Resized` straight away and clear it again. Unverified.
- Step 9: +81 % idle CPU with six panes.

## M5 re-run (T041), 2026-10-09

Same set-up as above, window 1500x900, HEAD 30568506, driven with xdotool. Crops are in the unit's
scratchpad (not committed).

| Check | Result | Evidence |
|---|---|---|
| Step 3 dark scheme (Settings > Appearance > Dark) and light | pass | Focused pane: tinted header, accent strip, outlined body; unfocused flat; typing reached only the focused shell |
| Step 3b selection, link, scrollback with several panes | pass | Wheel scrollback and drag selection changed only the focused pane (neighbour pixel diff 0); link hover underlines, Ctrl+click reaches the opener. Not driven in the Claude pane |
| Step 7 seventh split | pass | "At most 6 panes fit." visible at 0.2 s and unchanged at 3 s, by chord and header button; no pane resized |
| Step 7 pane too small | pass | "This pane is too small to split." at 0.2 s and 3 s; layout unchanged |
| Empty pane focus (FR-010, FR-016) | pass | Header click and body click focus an empty pane; focus stays on it across a switch to project B and back; persisted `focused` unchanged |
| Step 9 idle CPU | pass | 3.0 % (1 pane) vs 3.1 % (6 panes): +3 %. See quickstart.md |

Observation, not a defect of M5: with an empty pane focused the pane chords do nothing (they are
handled by the terminal widget); the header buttons work.
