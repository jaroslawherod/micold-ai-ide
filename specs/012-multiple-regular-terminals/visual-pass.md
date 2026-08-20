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

---

## 2026-08-19 — §4 and §8, re-run for BUG-005 (T076)

**Ran on**: Xvfb `:77` (1600×1400) + lavapipe, not a physical display. Client and daemon built
together from `fix/terminal-tab-restart-width` and copied out of `target-shared/` before launching;
verified with `strings ~/vp/bin/micold-ai-ide | grep -c shell_instance_menu` → 5, so the binary under
test is the one carrying the fix.

Isolated `XDG_DATA_HOME` in the run's scratchpad and `XDG_RUNTIME_DIR=/tmp/vp77`, with a throwaway
git project at `/tmp/vp77proj` **seeded into a private `projects.json`** rather than opened through
the user's catalogue. That mattered here: the user's own app was running and holding their project,
and the previous passes' shared data home would have meant taking it over. Only processes whose
`XDG_RUNTIME_DIR` read `/tmp/vp77` were ever stopped.

**Scenario**: Default session → Regular Terminal mode → the bar's "+" twice. Three instances,
numbered 1–3; a fourth opened later. Both schemes.

**Method note.** Previous passes compared crops stacked at 2–5× magnification. This one also read
**pixel columns numerically** — locating each glyph's ink run and comparing centres between two
frames captured at identical geometry. That is the difference that found the first defect below: a
4.6dp shift is a pixel and a half at 1× and reads as measurement noise when two crops are stacked,
which is exactly what two earlier passes concluded about it.

### Passed — both themes

- **No tab draws a container.** Every entry is a bare label. The hover state layer draws a pill under
  the pointer, which is a text button's state layer and not a container.
- **Exactly one indicator, on the top edge.** Measured on the frame: a 3px accent rule spanning
  x 1352–1471 above the active tab, and nothing above any other. 120dp is the tab's *content* box —
  136dp less the button's `spacing::SM` either side — which is what "spans the tab's width" means for
  a padded button.
- **The active label takes the accent, and its `×` with it** (FR-011a). Dark: lavender on near-black
  against `on_surface_variant` grey for the rest. Light: deep indigo against near-black.
- **The close control is visible on the active tab.** Not a ghost in either scheme.
- **The squint test (SC-009).** Blurred to where no digit is legible, the bar and the tinted glyphs
  still say which tab is active, in both schemes.
- **A primary press still selects, through the new `ContextArea` wrapper** (§4). Pressing an inactive
  tab activated it; the wrapper intercepts only the right button, as T069 asserts and as this
  confirms end to end.
- **The menu belongs to the tab it was opened on** (FR-010a). Right-clicked instance 1's tab while
  instance 4 was active and pressed **Close**: instance 1 went, instance 4 stayed active. The menu
  addresses its own instance, not the selected one — the property the whole bugfix is about.

### Defect 1 — a tab's content is off its midline, and slides when the tab is activated

Every **inactive** tab drew its label and close control **4.6dp left of centre**; the moment a tab
became active its content slid 4.6dp right, and the tab losing activation slid its content 4.6dp
left. Measured, dark scheme, glyph-ink centres at identical crop geometry:

| glyph | active = 3 | active = 1 | Δ |
|---|---|---|---|
| tab 1 label | 1118.5 | 1122.5 | **+4.0** |
| tab 1 `×` | 1150.0 | 1155.0 | **+5.0** |
| tab 2 label / `×` | 1263.0 / 1294.0 | 1263.0 / 1294.0 | 0.0 |
| tab 3 label | 1411.5 | 1406.5 | **−5.0** |
| tab 3 `×` | 1443.0 | 1438.0 | **−5.0** |

Tab 2, which was inactive in both frames, did not move by a pixel — so this is not drift in the
capture. The light scheme gave the same figures to within half a pixel.

The tabs themselves never move: they are 136dp at a 144dp pitch in every frame. What moves is inside
one. The active tab's `Divider` is `Length::Fill`, so its column measures the tab's whole 120dp
content box and centres the row in it; an inactive tab's indicator placeholder is a `Space` with a
height and **no width**, so a shrinking column measured only the 110.8dp row and pinned it to the
leading edge. Half the 9.2dp slack is the shift.

It is older than this bugfix and was *amplified* by it. The same asymmetry existed at
`TAB_WIDTH = 128`, where the slack was 1.2dp and the offset **0.6** — under any tolerance, invisible
at any magnification. FR-004c's derivation corrected the width to 136 (T068), which had nothing to do
with centring and multiplied the defect by eight into visibility.

Fixed by giving the column `Length::Fill`, so every tab measures its content box and centres its row
whether or not it draws an indicator. **`tests/gates/tab_children_fit.rs` gained
`a_tabs_content_sits_on_its_tabs_midline`**, which fails on the unfixed build naming both inactive
tabs at exactly `-4.6dp`. Asked per tab against its own midline rather than by comparing tabs to each
other: "every tab is wrong in the same way" would pass a difference test and is still a defect.

### Defect 2 — the tab's context menu opened downward, into 27px of window

The menu FR-010b moved the restart affordance into is anchored at the press point, and the tab strip
is in the terminal's **bottom bar**. Measured: the panel's surface began at y ≈ 1371 in a 1400-tall
window and ran to the frame's last row — about 27px of a ~48dp item, its label cut through. With the
instance exited, **Restart** and **Close** would both be offered and the second would be entirely
outside the window.

That is the same defect BUG-005 is about, one layer out: an affordance present in the tree, correctly
conditioned, dispatching the right message, and impossible to press. `Anchor::Point`'s own
documentation says "the caller is responsible for clamping the point so the panel cannot open
off-screen" — and this caller cannot clamp it, because the room below any press inside a bar pinned
to the window's bottom edge is at most that bar's height.

Fixed by adding `cdk::overlay::Anchor::BottomStart { bottom, start }` — the panel's *bottom*-left
corner measured from the window's bottom edge — and `material::ContextMenu::rising_above(bottom)`,
mounted with `anatomy::app_bar::HEIGHT` read rather than restated. Posed in the showcase beside the
cursor-anchored default, so the two anchors are comparable rather than remembered.

### Re-run against the fixed build

Both fixes were verified by a second pass on a rebuilt, re-pinned pair, not left on the argument:

- **Defect 1.** Three instances, activation switched from 3 to 1, glyph-ink centres read at identical
  geometry: **every one of the eight runs moved 0.0px**, against ±4.6dp before. Each tab's label also
  sits on its own midline (tab 1's `×` at 1155.0 in a tab spanning 1056–1192, midline 1124, with the
  close control's own offset accounting for the rest).
- **Defect 2.** Right-clicking a tab now opens the panel **above** the bar, its whole surface and its
  item inside the window, its left edge at the press x.
- **BUG-005 reproduced on the matched pair**, so the report is not an artefact of a stale binary:
  `exit` in instance 1, its pane showing `exit` and no new prompt, and its menu still offering
  **Close** alone twenty seconds later.

### A trap in the recipe, worth carrying forward

The `visual-pass` skill's pinning command builds both binaries in one invocation:

```
cargo build -p micold-client --bin micold-ai-ide -p micold-daemon && cp …
```

`--bin` is a **target filter applied across every selected package**, so `-p micold-daemon`
contributes no matching target and the daemon is not built at all. The `cp` then pins whatever
`target-shared/debug/micold-daemon` another worktree left there. Here that was a pre-v6 build, and
the app opened with the red "the session service is a different version" banner — which is the
*good* case, because the handshake caught it. A stale daemon that happened to agree on the wire would
have been pinned silently, which is the same hazard the skill already documents for the client.

Name both bins in the one invocation — `-p micold-client --bin micold-ai-ide -p micold-daemon --bin
micold-daemon` — and verify with a string only the current wire carries: `strings
~/vp/bin/micold-daemon | grep -c live_shells` was `0` before and `2` after. `--version` is not a
substitute: running the daemon binary to ask starts a daemon. (A parallel worktree hit the same trap
on the same day and its fix to the skill is the one that landed; this pass's own edit was dropped in
favour of it.)

### §4's restart step — blocked, then run, on the merged branch

The first attempt could not reach it. Typing `exit` killed the shell and the client went on calling
the instance **running** indefinitely: the bar said `running` 30+ seconds later and after a forced
catalogue push, and the tab's menu offered **Close** alone. `ShellLifecycle::Exited` was unreachable,
so the item FR-010b creates was never offered to press. That was diagnosed to the daemon never
reaping a shell instance whose process exited on its own, and was about to be filed as a bug.

It was already fixed on `main`, by BUG-003's second commit: `overlay_live_summaries` now filters
`live_shells` by `pty.is_alive()` rather than by presence in the map, with the comment saying exactly
why — "reporting it live would make `exited` unreachable". This branch was two commits behind it, and
the pass was reading a daemon that predated the fix. **A pass on a branch is a pass on that branch's
dependencies too**, and "the state I need never arrives" is a claim to check against `main` before
filing.

Re-run after merging, on a freshly pinned pair:

- Three instances, instance 1 exited while instance 3 stayed active.
- Right-click instance 1's tab: **Restart** and **Close**, both items fully inside the window, opening
  upward from the bar.
- Press **Restart**: instance 1's pane comes back with a fresh prompt, instance 3 keeps the indicator
  and is never selected, and re-opening instance 1's menu now offers **Close** alone — its lifecycle
  is `Running` again. That is FR-010a end to end: a background instance restarted, by id, without
  being selected first.

§4 **passes**.

### Not run — and why

- **Ten or more instances**, for the two-digit and ellipsis cases. Unexercised in all three passes
  now; the fixed width bounds the damage but the label's `max_width` behaviour is still unseen.
- **Mid-flight animation and perceived smoothness** — out of reach of this harness, as before.

### Frames

In the run's scratchpad, not committed: `07strip.png`, `20strip.png` (the strip at 4× in each
scheme), `reflow.png` (dark, before/after activation at identical geometry — defect 1),
`squint.png` (both schemes, blurred), `15menu.png` (the clipped menu — defect 2), `36menu.png` (the
same menu rising from the bar, after), `43menu.png` (Restart + Close on an exited background tab).

### What this says about the previous two entries

Both earlier passes reported "no reflow on activation" and both were right about what they measured:
every tab holds its position and size, which is the SC-008 sentence and the risk the indicator design
was weighed against. The 4.6dp is *inside* a tab, below the resolution of a stacked visual comparison
— and at the time it was 0.6dp, below any resolution at all. Reading columns numerically is cheap and
is now part of this recipe; "I looked and nothing moved" is a claim about magnification, not about
the layout.
