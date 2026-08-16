# Visual pass — Multiple Regular Terminal Instances per Session

Records of `quickstart.md`'s manual GUI checks, run headlessly with the repo's `visual-pass` skill.

---

## 2026-08-14 — §8, the switcher tab strip (BUG-001)

**Ran on**: Xvfb `:77` (1600×1400) + lavapipe (Mesa's software Vulkan rasteriser), not a physical
display. Client and daemon both `debug`, built from `fix/small-visual-improvements` and copied out of
`target-shared/` before launching (see "Hazard" below). Isolated `XDG_DATA_HOME` and a private
`XDG_RUNTIME_DIR=/tmp/vp77`, with a throwaway git project at `/tmp/vpproj`; the user's own app,
daemon and project catalog were untouched, and only processes whose `XDG_RUNTIME_DIR` read
`/tmp/vp77` were ever stopped.

**Scenario**: one session on the Default worktree, switched to Regular Terminal mode, then a second
instance opened with the bar's "+". Two tabs, numbered 1 and 2.

### Passed — dark theme

- **Every entry is a tab (FR-004a).** Tab 1 (inactive) draws an *outlined* container with a visible
  border; tab 2 (active) draws a *filled* one. Neither is bare text. This is the reported defect
  gone: the row now reads as a tab strip rather than as one pill among loose characters.
- **Uniform size.** Measured off the 5× magnification, the two tabs are the same width to within a
  pixel of measurement noise, and the same height and corner radius.
- **Label centred, close trailing.** In each tab the digit sits on the tab's midline and the `×` is
  pinned at the trailing edge.
- **The close control is legible on the active tab (FR-011a, SC-007).** The `×` on the filled tab is
  dark purple on lavender — `on_primary` over `primary`, the same pair as its own label — and is
  plainly readable at 1×, not only when magnified. This is the headline fix: the reported bug was
  this glyph rendering in `on_surface` and all but vanishing against the fill.
- **No reflow on activation (SC-008).** Pressed tab 1 to make it active and captured at *identical*
  crop geometry. Stacking the two frames, every element — the status text, both tabs, the "+", the
  mode toggle — holds its exact x position; only the fill moves between the two tabs. Nothing shifts
  under the pointer between a press and its release.

### Passed — light theme

Re-run after switching Theme to Light from the overflow menu. Same four results: tab 1 filled deep
purple with a white label and a white `×`; tab 2 outlined with a purple label and a purple `×` on the
light surface; equal geometry; labels centred; both close controls legible. The active tab's `×` is
as readable here as in dark — which matters, because the two schemes swap which of `primary` /
`on_primary` is the light tone, so a fix that only worked by luck in one would show here.

### Also confirmed while set up

- **FR-005**: with a single instance open, no switcher row is drawn at all — the bar is status ·
  "+" · mode toggle. The strip appears only on the second instance.
- **FR-021b** (`023-terminal-focus-flow`): the bar carries no release-focus control, in either mode.

### The defect this pass caught

The first build placed each tab's label **~12dp left of centre**, not centred. Invisible at 1×,
unmistakable at 5×. The leading spacer that balances the trailing close control was sized 24dp from
the close glyph's *visible pill* — but a pressable, non-compact `IconButton` wraps itself in §7.3's
48dp minimum touch target (`icon_button.rs`), so the spacer was balancing a 48dp control with 24.
Predicted error `(48 − 24) / 2 = 12`dp; measured ~12. `TAB_CLOSE_WIDTH` now reads
`anatomy::button::MIN_TOUCH_TARGET` so the two cannot drift apart, and the re-run above is against
the corrected build.

This is exactly the class the geometry gates are blind to — every gate was green with the label
off-centre, because each node *was* where its own layout said it was.

### Not run — and why

- **Ten or more instances**, to confirm a two-digit id does not resize its tab. The fixed-width label
  box is meant to cover it and the arithmetic is straightforward, but it was not exercised.
- **Mid-flight animation and perceived smoothness** — out of reach of a screenshot pipeline on a
  software rasteriser, per the skill.

### Hazard worth carrying forward

The first bar screenshot of this run still showed the removed release-focus button, which read as the
change having failed. It had not — `ui/terminal.rs` contains no `Icon::ReleaseFocus` and its gate
passes. The *binary* was another worktree's: `target-shared/` is shared by every checkout on this
machine (CLAUDE.md), so `target-shared/debug/micold-ai-ide` is whatever branch built last, and four
other worktrees were building throughout this pass. Behind it sits a second trap: the client refuses a
daemon whose protocol **schema hash** differs (`handshake::evaluate`), which the log reports as
"contract or build mismatch" while printing matching v5 / 0.8.0 versions on both sides — so a client
and daemon picked up from `target-shared/` at different moments simply will not connect. Build both
in one invocation, copy them aside, run the copies. The `visual-pass` skill has been updated.

### Frames

In the run's scratchpad, not committed:

- `tabs-dark.png` — both tabs at 5×, dark theme
- `bar-dark.png` — the whole strip in context, dark
- `reflow.png` — before/after activation at identical crop geometry, stacked
- `bar-light.png` — the whole strip, light theme
- `bar-b3.png` — one instance, no switcher
- `term-e1/e2/e3.png` — the §B3 focus cycle (recorded in `023`'s own pass)

---

## 2026-08-16 — §8 re-run for BUG-002, the indicator tabs

**Ran on**: Xvfb `:77` (1600×1400) + lavapipe, not a physical display. Client and daemon built
together and **copied out of `target-shared/` before launching**, per the skill — the hazard that
nearly invalidated the previous pass. Isolated `XDG_DATA_HOME`, `XDG_RUNTIME_DIR=/tmp/vp77`, project
at `/tmp/vpproj`; only processes whose runtime dir read `/tmp/vp77` were ever stopped.

**Subject**: BUG-002 replaced BUG-001's container tabs with Material primary tabs — bare label plus
an accent indicator on the tab's **top** edge, since this bar is anchored to the window's bottom and
the pane a tab selects is above it.

### Passed — both themes

- **No tab draws a container.** Every entry is a bare label. The strip reads as a tab bar, not a row
  of pills.
- **Exactly one indicator, on the top edge.** The accent bar spans the active tab's width along its
  upper edge; no other tab has one. Dark: light lavender on near-black. Light: deep purple on the
  light surface.
- **The active label takes the accent**, and its `×` with it (FR-011a) — the cue is carried twice.
- **No reflow on activation.** Captured before and after switching the active tab at *identical*
  crop geometry and stacked: `starting…`, both labels, both close controls, the "+" and the mode
  toggle all hold their exact x positions. Only the indicator and the colour move.
- **The squint test (SC-009).** Blurred to where no label is legible, the accent bar still says
  which tab is active, in both themes. This was the real risk of dropping containers and it is the
  check worth keeping: without a container the accent *is* the entire cue.

### The defect this pass caught

The first build made **the active tab several times wider than the inactive ones**. `Divider` is
`Length::Fill`, and inside a content-sized button that resolves against the *button's* available
space rather than the label's — so drawing the indicator stretched its own tab, and activation
resized a tab under the pointer. Precisely the SC-008 reflow the design was meant to avoid.

Note what did not catch it. `every_tab_reserves_the_indicators_height` passed throughout, correctly:
both arms *do* use `anatomy::tab::INDICATOR`, and the **height** was always right. The defect was in
the **width**, and every node was exactly where its own layout said it was — the same blind spot as
BUG-001's 12dp centring error, one dimension over. Two visual passes, two width/position defects
invisible to a green suite.

Fixed by giving every tab one fixed width (`TAB_WIDTH`), so the indicator's `Fill` resolves to the
tab rather than to whatever space the bar offers. SC-008 then holds by construction instead of by
arithmetic, and a renamed tab will ellipsise rather than resize the strip — how a browser tab bar
behaves. The figure is set by what must fit: two `MIN_TOUCH_TARGET` widths (the close, and the
spacer balancing it) plus a readable label.

### Not run — and why

- **Ten or more instances**, for the two-digit/ellipsis case. Still unexercised, as in the BUG-001
  pass; the fixed width makes it less interesting than it was, since the tab can no longer grow.
- **Mid-flight animation and perceived smoothness** — out of reach of this harness.

### Frames

In the run's scratchpad, not committed: `tabs-ind-dark.png` (first build, the width defect),
`tabs2-dark.png`, `tabs2-light.png`, `reflow2.png` (before/after at identical geometry),
`squint.png` (both themes, blurred).

### A coverage finding, from the fixture that did not move

This change rebuilt the tab strip — containers replaced by an indicator, a fixed 128dp tab width,
a new row in each tab's column — and `layout_snapshot.txt` did not change by a single byte.

That is not the gate being tolerant. `session-terminal-bottom-bar`, the covered state that renders
this bar, is built with at most one shell instance, so `instance_switcher_row` returns `None` and
**the tab strip is in no covered state at all**. The geometry fixture has never had coverage of this
control.

It reframes both defects these passes caught. The comfortable reading — "geometry gates cannot see
centring or colour" — is true but not what happened here. A 12dp centring error and a tab several
times too wide are both *pure geometry*, exactly what this fixture exists to catch, and it missed
them because the control is never rendered into it. BUG-001's edit did move the fixture, which made
the coverage look real; what moved was the release-focus button's removal from the same bar, not
anything about the tabs.

Registering a covered state with two or more instances is a single step by feature 019's FR-016, and
would put this strip's geometry under the gate for the first time. Recorded here rather than done
here: it belongs to 019's covered set, and adding a state churns the fixture in its own right.
