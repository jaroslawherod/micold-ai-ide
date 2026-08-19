# BUG-003 item 1 — visual pass

**Date**: 2026-08-19
**Run by**: an agent, not a person at a display — Xvfb `:83` at 1600×1400, rendered by Mesa's
lavapipe (software Vulkan), driven with `xdotool`, captured with `import`. Per the repo's
`visual-pass` skill.
**Build**: this branch's own `micold-showcase`, copied out of the shared target directory **inside**
the build lock and launched from that private copy — `target-shared/debug/<bin>` is whichever
worktree built last.
**Isolation**: its own `XDG_RUNTIME_DIR` (`/tmp/vp83`) and `XDG_DATA_HOME`; stopped by PID
afterwards. The showcase needs no daemon, so nothing was spawned beyond the one process.

The showcase rather than the client: the `Typeahead` section poses the exact control the bug is
about, and reaching it is a scroll rather than a project fixture plus a daemon plus a dialog. The
defect is in the shared field, not in the branch picker's use of it.

## Why this needs eyes at all

The geometry gates resolve rectangles and are exact about them — and they were **green while the
icon sat on the label**, because before the fix the icon was not a rectangle they could see. It was
a `text_input::Icon`, drawn inside the input's own paint, so no layout node existed for it and every
gate compared positions that were individually correct. That is the blindness the `visual-pass`
skill's own opening example names, and it is this bug.

The three new assertions in `form_field_anatomy.rs` close it for the future — the icon is a layout
child now — but they were written after the fix. This pass is what says the fix is right on screen.

## What was checked, and what was seen

### 1. Resting label, dark scheme — the reported state — **PASS**

`bug003-item1-before-after.png` stacks the T063 crop (red border, above) against this pass's crop
(blue, below): same control, same scheme, same state, both magnified. Above, the magnifier is drawn
through the "B" of "Branch". Below, the icon sits in its own slot and the label starts at the
content column, clear of it. `bug003-item1-after-dark.png` is the after crop on its own.

The two are at different magnifications — the T063 crop came from the client at a different scale —
so the comparison is of the collision, not of dp-for-dp geometry. The dp are the anatomy tests'
business and they assert the exact column.

### 2. Focused and open, empty query, light scheme — **PASS**

`bug003-item1-after-open.png`. The floated `Branch` label and the `Search branches…` placeholder
share one left edge, both clear of the magnifier, which stays on the container's centre line rather
than following the value down. The active indicator is thickened and the label takes the accent —
the focus treatment is unchanged by this fix.

### 3. Typed query — **PASS**

`bug003-item1-after-typed.png`, query `mat`. The floated label and the value share the same left
edge. This is the half a "does the label clear the icon?" check would not have caught: the label and
the value are one column, not two that happen not to collide.

### 4. Both schemes — **PASS**

Light in checks 2 and 3, dark in check 1, via the showcase's own scheme toggle. Nothing here is
colour-dependent, but the label is drawn by the wrapper and the value by the input, and those take
their tint from different style functions — so seeing both in both schemes is worth the two clicks.

## Found while looking, and **not** fixed

**The trailing clear affordance is squeezed and draws low.** With a query present, the `×` sits
about 11dp below the field's centre line, and its touch target is compressed to 24dp from the 48dp
§7.3 asks for. Measured off `vp83-typed.png` by scanning for ink: the container occupies rows
507–561 (centre ≈ 534) and the glyph's ink runs 539–552 (centre ≈ 545.5).

Traced, so a fix does not have to rediscover it: `FilledField::layout` caps both adornment slots at
`VALUE_LINE` (24dp). An `IconButton` wants 40dp and pads 8dp top and bottom, so inside a 24dp box
its glyph node is laid out 24×**8** and the text overflows downward out of it. Probed directly —
the trailing slot is `48×24` and correctly centred at 28, and the glyph node inside it is at
relative `(8, 8)` with height 8.

This pass **improved** it by 8dp without addressing it, because centring the slots moved it up; it
was lower still before. It is a separate defect from the reported one — the reported one is the
label column — and it is not patched here. The one-line shape of a fix is to limit the adornments by
the container's height rather than by the value line, which would let the icon button lay out at its
natural 40dp; that changes the trailing button in every text field in the application, which is more
than this bug asked for.

## What this pass cannot answer

- **Mid-flight animation.** The label snaps between its two positions by design (accepted fidelity
  gap #4), so there is no transition here to miss — but the picker's list arrival is animated, and a
  screenshot pipeline cannot catch a chosen frame of it. Nothing in this fix touches it.
- **Perceived smoothness.** lavapipe is a software rasteriser; frame pacing here says nothing about
  frame pacing on a real GPU.
- **The client's own branch picker.** Checked in the showcase, which poses the same `Typeahead`
  the dialog builds. The layout snapshot's `add-worktree-dialog-existing-branch` state renders the
  dialog *before* any branch list exists, so it never contained the search field at all — which is a
  second reason no gate saw this, and is recorded here rather than fixed.
