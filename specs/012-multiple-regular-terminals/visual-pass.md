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
