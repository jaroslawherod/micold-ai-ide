# Visual pass — the pre-fix baseline (T001)

**Date**: 2026-08-10. Run under Xvfb `:78` (1600×1400) with Mesa lavapipe, driving the
**application** at `f546b60` — the last commit before any of this feature's code lands. Recorded
before T012/T013, because the evidence stops existing once they do: SC-002 asks for "exactly one
press, **down from two**", and the "from two" half cannot be reconstructed afterwards.

An isolated instance — its own `XDG_RUNTIME_DIR` (`/tmp/vp78`), `XDG_DATA_HOME` and
`XDG_CONFIG_HOME`, seeded with a single project — so it spawned its own daemon and took nothing over
from the session already running on this machine.

Display `:77` was already occupied by another agent's instance, whose windows composited into the
first capture. Anything recorded here is from `:78`, which was created for this run.

## What was driven

A Default-location session in the worktree, switched to Regular Terminal mode so a real shell was
attached (the AI CLI process stayed at `starting…`; the mode toggle does not depend on it). The
terminal was given focus by pressing inside the pane. The bottom bar then carried three trailing
controls, in this order:

| x, y | Control |
|---|---|
| 1456, 1367 | release focus (keyboard glyph) — **present only while focused** |
| 1512, 1367 | "+" new Regular Terminal instance |
| 1567, 1367 | mode toggle (Regular Terminal ⇄ AI CLI) |

## The finding

Three frames, cropped at identical geometry (`1300x64+300+1338`) and stacked:

1. **Before** — Regular Terminal, focused. Title is the shell's prompt; all three controls present.
2. **After one press on the mode toggle** — still Regular Terminal (the tooltip still reads
   "Regular Terminal — switch to AI CLI"). Only the mode toggle remains: focus was released, so the
   release affordance and the "+" are gone. **The press did not switch the mode.**
3. **After a second press** — AI CLI ("Claude Code", "AI CLI — switch to Regular Terminal").

One press released focus and did nothing else. The mode changed on the second. That is the user's
report, reproduced: `t001-baseline-3frame.png`.

## The part that is not in the spec

**The bug needs a frame between press and release.** The first attempt used `xdotool click 1`, whose
press and release are ~12 ms apart, and the mode switched on that single click — the bug did not
reproduce. Holding the button for 350 ms, which is an ordinary human click, reproduced it every
time.

That is exactly what research R1 predicts and is worth recording, because it explains both why the
defect survived this long and why no test caught it. `iced_widget::button` publishes `on_press` on
**`ButtonReleased`**, gated on an `is_pressed` flag held in its node of the widget tree. The damage
is done by the `view()` that runs *between* the two: the release-focus affordance disappears, the
"+" and the mode toggle each shift one index left, and `Tree::diff_children` — which zips by
position — hands the mode toggle its left neighbour's node, dropping `is_pressed`. If no frame is
rendered in that gap, no diff happens and the press survives.

So the bug is invisible to any driver that clicks instantly, and it is why the second press always
works: with focus already released the bar no longer changes shape mid-click.

## Frames

Kept in the run's scratchpad, not committed (they are 1600×1400 screenshots of a throwaway
instance):

- `t001-baseline-3frame.png` — the three bar states above, the headline evidence
- `t001-slow-before.png` / `t001-slow-after.png` / `t001-slow-after2.png` — the full frames they
  were cropped from
- `t001-compare.png` — the fast-click attempt that did *not* reproduce, kept because the contrast is
  the finding

## What this run could not answer

- Whether the focus ring blinks mid-press (FR-008a, quickstart §B2). A screenshot pipeline cannot
  reliably catch a chosen frame of a transition, and §B2 asks about one frame after the press lands.
  It is recorded as unrun here and belongs to T017 against the fixed build.
- Anything about the AI CLI process, which never left `starting…` on this machine.
