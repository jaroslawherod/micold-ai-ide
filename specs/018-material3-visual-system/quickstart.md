# Quickstart: Validating the Material 3 Visual System

**Feature**: `specs/018-material3-visual-system` | **Date**: 2026-07-26

Two parts. **Part A** is automated and gates the build. **Part B** is the recorded manual procedure
that validates the GUI wiring the render-free core cannot assert — the constitution's Principle I
GUI-wiring exception requires this to exist and to be followed, not improvised.

Run Part B **twice**: once in the light scheme, once in dark.

---

## Prerequisites

```sh
mise trust          # first time in a fresh worktree only
mise run test       # cargo test --workspace
mise run run        # cargo run -p micold-client
```

A test repository with **at least a dozen worktrees** across several conventional-commit types
(`feat/`, `fix/`, `chore/`, `docs/`…) — needed for the tag-color and sidebar-density checks. Build
it with:

```sh
mise run fixture                        # ~/micold-reference-scene
mise run fixture -- /path/to/it --force # …or somewhere else, replacing what is there
```

That is the same 20-row scene §B8 measures against, so the manual checks and the frame-time figures
are looking at identical rows: all ten types, with and without issue keys, one untyped row, one long
enough to force ellipsis, and one orphaned directory carrying a health tag. Hand-building it twice
would produce two different scenes, and the difference would land in the figure without appearing
in it.

---

## Part A — automated gates

```sh
mise run test
```

| Gate | Test | Proves |
|------|------|--------|
| AA contrast, both schemes, every pair | `micold-core/tests/tokens_contrast.rs` | SC-001, FR-004, FR-005 |
| Tone ramps monotonic in luminance | `micold-core/tests/tokens_contrast.rs` | catches ramp transcription error |
| Type/elevation/shape/state/motion invariants | `micold-core/tests/tokens_scales.rs` | FR-007/014/018/020/033, FR-042 |
| Density scale: four steps, −4dp each, no fractional height | `micold-core/tests/tokens_density.rs` | FR-026b |
| Snackbar queue: one visible, dedup, cap, severity duration | `micold-core/tests/notify_queue.rs` | FR-032a, FR-032b |
| Both Roboto faces load at expected weights | `micold-client/tests/roboto_font.rs` | FR-008a, SC-012 |
| No raw text size literal at any call site | `micold-client/tests/type_role_call_sites.rs` | SC-003, FR-010 |
| Ripple origin, clipping and per-element independence | `micold-client/tests/ripple_state.rs` | SC-005a, FR-024b/d/e |
| Component anatomy matches contract §7 | `micold-core/tests/tokens_anatomy.rs` | SC-008, FR-025–FR-032 |

**Expected**: all pass. A failure in the contrast gate must block the change — it is a hard
invariant carried over from feature 003, not a warning.

Verify the render-free boundary is real, not conventional:

```sh
grep -c iced crates/micold-core/Cargo.toml     # expect 0
cargo test -p micold-core                      # tokens exercised with no renderer present
```

---

## Part B — manual walkthrough

### B0. First-run identity check *(do this first)*

`mise run run`.

- [ ] The accent color is **purple**, not the previous blue. This is intended (FR-005b) — the
      palette is now Material's baseline scheme. If it is still blue, the palette did not take.
- [ ] Text renders in **Roboto**, not the platform UI font. Confirm by changing the OS UI font and
      relaunching: the app must be unchanged (FR-008, SC-006).

### B1. Depth and surface hierarchy — US1 (P1)

- [ ] Window background, sidebar and any raised card sit at **visibly different tonal levels**, and
      none uses a border to say so (FR-002, SC-002).
- [ ] Open a dialog: it has a **drop shadow**, a scrim behind it, and a notably larger corner than
      a card (FR-028).
- [ ] Open a context menu and the project switcher popover: each **floats** with its own shadow.
- [ ] Open a context menu **over an open dialog**: the menu renders above, keeping its own shadow;
      neither flattens into the other (FR-017).
- [ ] Every button container is **fully pill-shaped** (FR-019).
- [ ] Scan for leftover decorative outlines. An outline is only legitimate as a divider, an
      outlined control's border, or a focus ring (FR-003).

### B2. Typography — US2 (P2)

- [ ] In a dialog, title / body / caption are each distinguishable from the other two **without
      relying on position** (SC-004).
- [ ] Multi-line body text uses the role's line height, not default line spacing (FR-007).
- [ ] **Terminal output is still monospaced** with its own grid metrics (FR-012). Type a command
      and confirm column alignment is unaffected.
- [ ] Sidebar text is still visibly denser than equivalent text elsewhere (FR-011).
- [ ] Create a worktree whose name contains a character outside Roboto's coverage (e.g. CJK). It
      renders via fallback — **no missing-glyph boxes** (FR-013).

### B3. Interaction states — US3 (P3)

- [ ] Hover in turn: sidebar row, tree item, known-projects row, menu item, context-menu item,
      chip, tag, each button variant. **Every one** changes visibly (FR-021, SC-005).
- [ ] Press and hold each: the change is **stronger** than hover.
- [ ] A selected worktree row and a selected filter chip show a **persistent** treatment, distinct
      from hover (FR-020).
- [ ] Tab into a dialog's text field: a focus indicator is visible **without** the pointer over it
      (FR-022). Hover it while focused — both remain resolvable.
- [ ] Confirm buttons/rows/menu items show **no** focus ring. This is expected — accepted fidelity
      gap #2 (FR-043), not a defect.
- [ ] A disabled control is dimmed, **including** a self-coloring icon glyph (FR-023).

### B4. Component anatomy — US4 (P4)

- [ ] App bar: correct height, title in the app-bar title role, not a cramped strip (FR-025).
- [ ] **Scroll the sidebar** away from the top → app bar takes its elevated appearance; scroll back
      → it returns (FR-025a).
- [ ] Sidebar rows all at the dense density; known-projects rows all at standard (FR-026).
- [ ] **Count worktrees visible without scrolling, against the same repo before the change.** Must
      not have dropped materially (FR-026a) — this is the guard on the density decision.
- [ ] Icon buttons are comfortable to hit; the target extends beyond the visible glyph (FR-027).
- [ ] Dialog actions are grouped at the **trailing** edge with the defined spacing (FR-028).
- [ ] Trigger a notification: it appears as a **floating snackbar**, not an inline strip (FR-032).
- [ ] Trigger several rapidly: **exactly one visible**, others follow in turn (FR-032a).
- [ ] Trigger an error: it stays for the **long** duration and can still be dismissed manually
      (FR-032b).
- [ ] Trigger a snackbar **while a dialog is open**: it renders above the scrim and does not
      permanently block the dialog's actions.

### B5. Motion — US5 (P5)

- [ ] Each existing animation — overlay fade, sidebar slide, menu fade, row hover fade — still
      triggers and ends in the **same visual state** as before (FR-035, SC-010).
- [ ] Fades accelerate and decelerate rather than moving at a constant rate (FR-034).
- [ ] The sidebar slide reads as more expressive than the small fades (emphasized vs standard set).
- [ ] App-bar elevation and snackbar enter/exit are animated, not instant (FR-035a).
- [ ] Leave the app idle: no animation runs at rest (the clock still gates itself).

### B6. No behavior change — FR-036

The critical regression pass. Walk **every** screen, dialog and flow:

- [ ] Create, rename and delete a worktree.
- [ ] Create a worktree from a **new** branch, then from an **existing** branch; exercise both the
      reuse and the overwrite resolutions, and switch the branch-source toggle back and forth
      confirming no residual state. (Feature 016 landed this flow after this spec was written — it
      is inside the no-behavior-change scope and its form is the most heavily restyled in US2.)
- [ ] Watch the worktree-creation progress indicator name each step as it runs.
- [ ] Start, switch and remove a session; confirm a Default (project-root) session still works.
- [ ] Open, filter and switch projects; forget a project.
- [ ] Every keyboard shortcut behaves as before — arrow keys, Tab and PageUp/Down still reach the
      **terminal**, unchanged.
- [ ] Terminal input, output, scrollback and selection unaffected.
- [ ] Quit and relaunch: all persisted state restores identically.

- [ ] **Exactly one** behavioral difference is observed in the whole pass: notifications now show
      one at a time and dismiss on a timeout (FR-036a). Anything else is a defect in this feature.

### B7. Scheme switch under load

- [ ] With a dialog, a menu **and** a snackbar simultaneously on screen, switch light ↔ dark. Every
      visible surface re-resolves — roles, elevation, state layers — with **no restart**.

### B8. Performance — the reference scenes

Reported for trend. **This does not gate the build** (FR-039c): hosted runners render through
software rasterization with no stable frame-time floor, so a threshold there would fail on runner
variance rather than on the change under review. A regression here is a review finding.

**The scenes** (FR-039b). Two of them, because the ripple this feature introduces cannot exist on
the build you measure first.

*Baseline scene* — capturable on **both** builds:

1. A repository with **20 worktrees** in the sidebar. — `mise run fixture` (see Prerequisites).
2. The sidebar **expanded**.
3. **One running terminal session**.
4. A **context menu open over a dialog**.

Step 1 is the only part a script can build; 2–4 are composed in the running application. Compose
them before starting the probe, and do not touch the window while it counts.

*Full scene* — the baseline scene **plus a ripple mid-animation**. Post-change only.

**How to take a figure.** Build the repository with `mise run fixture`, then run:

```sh
MICOLD_FRAME_PROBE=300 MICOLD_FRAME_PROBE_SCENE=baseline mise run run
MICOLD_FRAME_PROBE=300 MICOLD_FRAME_PROBE_SCENE=full     mise run run   # post-change only
```

**The scene composes itself.** Naming a scene makes the application start the session, open the
dialog and open the context menu at a fixed position, then *verify* the result before a single
frame is counted. Nothing is clicked, so nothing differs between runs.

That check is the point, not the convenience. "A context menu open over a dialog" is not
reproducible by hand — opened where, over which dialog? — and a run against 19 worktrees or a
dismissed dialog yields a figure indistinguishable from a good one. There is nothing in
`300 frames — mean 0.84 ms` that says what it was measured against, so the mistake would surface
only as an unexplained gap between slots. A scene that cannot be composed exits non-zero naming
what is missing, and reports no figure at all:

```
frame probe: gave up composing the scene after 3000 frames.
the window is not showing the Baseline reference scene:
  - no terminal session is running; the scene needs one
```

The probe discards the warm-up, counts, prints one line to stderr and exits on its own:

```
frame probe: Baseline scene composed; measuring.
frame probe: 300 frames — mean 0.84 ms, p95 1.07 ms, max 1.40 ms
```

Paste that line whole into the slot below — all three figures must be written to the same
precision or they cannot be compared at a glance, which is the only reason they are recorded
together. `baseline` and `full` are each refused if the *other* one is on screen, so a figure
cannot land in the wrong slot.

**Use the same build profile for all three.** `mise run run` is a debug build, and a release build
is several times faster — a figure from one is meaningless against a figure from the other. The
slots below record which was used.

**What the figure covers, and what it does not.** It is the CPU cost of *composing* a frame —
building the widget tree from state. It is **not** present time: layout, draw and GPU work happen
after the measured span and are not in the number. The alternative — timing the interval between
presented frames — measures the display's refresh rate rather than the scene for anything that
renders faster than one vsync, which is every scene worth comparing here.

So this figure is sensitive to what this feature mostly changes (token lookups, type roles,
elevation and shape resolution, and the extra widgets `FormField` introduces) and blind to ripple
*draw* cost, which lands after composition. Read 2 → 3 with that in mind: it understates the full
scene's true cost.

- [X] **1 — baseline, pre-change** (`tasks.md` T000z). Unobtainable once T000f lands. **Captured.**

      Machine: AMD Ryzen 7 260 (16 threads) / Radeon 780M, Ubuntu 26.04, Wayland/GNOME, debug build
      Date: 2026-07-29
      Frame time: 300 frames — mean 0.84 ms, p95 1.07 ms, max 1.40 ms

      Taken three times to establish the run-to-run spread, so a later delta can be told from
      noise. `p95` is the figure to compare on — it barely moves:

      | run | mean | p95 | max |
      |-----|------|-----|-----|
      | 1 | 0.80 ms | 1.08 ms | 1.42 ms |
      | 2 | 0.87 ms | 1.08 ms | 1.24 ms |
      | 3 | 0.84 ms | 1.07 ms | 1.40 ms |

      **A change in `p95` below ~0.02 ms is noise on this machine.** `mean` varies by ~8% run to
      run and `max` by ~15%, so neither should be read alone.

- [ ] **2 — baseline, post-change** (T076a). Same machine, same profile, same scene as 1.

      Machine: ______________  Date: __________  Frame time: __________

- [ ] **3 — full scene, post-change** (T076a). Same machine, same profile as 1.

      Machine: ______________  Date: __________  Frame time: __________

- [ ] **1 → 2 is the comparison that matters.** Like-for-like: same scene, same machine. A gap here
      is a regression in rendering this feature did not add. **2 → 3** is this feature's own cost —
      expected to be non-zero, and worth knowing rather than fearing.
- [ ] All three figures recorded in the PR so the comparison is reviewable (SC-018).

### B9. Idle quiescence

- [ ] With no operation in flight and no pointer over the window, observe for a sustained window:
      **zero frames requested**, CPU indistinguishable from the pre-change build (SC-017, FR-039a).
- [ ] Press every interactive element in turn, then idle again: no animation state remains held.
- [ ] Start an operation that shows the indeterminate indicator. It animates while the operation
      runs, and **stops within one frame of the operation ending** (FR-039d).

---

## Recording the result

Record the run in the PR: date, platform, scheme(s) exercised, and any unchecked box with its
reason. An unchecked box in **B6** blocks merge — it is a behavior regression, which this feature
defines as a defect.
