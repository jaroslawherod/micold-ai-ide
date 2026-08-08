# Implementation Plan: Material 3 Visual System

**Branch**: `feat/improve-material-design` | **Date**: 2026-07-26 | **Spec**: [spec.md](./spec.md)

**Input**: Feature specification from `specs/018-material3-visual-system/spec.md`

**Depends on**: [`017-material-component-architecture`](../017-material-component-architecture/plan.md), which must land first. That feature closes the component boundary with zero visual change; this one changes how the application looks.

## Summary

Complete the Material 3 design system that feature 003 started: replace two neutral surface roles
with the full M3 baseline role set derived from Material's own seed, add the elevation, type,
shape, state-layer and motion scales that 003 deferred, and correct the anatomy of every component
that currently only imitates a Material component.

The technical approach is shaped by four findings from Phase 0. First, **the rendering stack
already supports everything the visual system needs except one thing** — `iced::Shadow` exists and
is a field on both `container::Style` and `button::Style`, absolute line heights exist, and font
weights and embedded font registration exist. The spec's premise that "there is not a single shadow
anywhere" described a usage gap, not a capability gap. Second, **the workspace split that landed on
`main` makes feature 017 structural**: `micold-core` declares no rendering dependency, so moving
`tokens.rs` there turns "tokens are render-free" from a test convention into a compile error.
Third, **keyboard focus is a real capability limit** — only text inputs can hold focus in this
stack — so FR-022 was narrowed and recorded as the second accepted fidelity gap. Fourth, **widgets
can own their own state**: the `Widget` trait's per-instance state hooks are available and the
`advanced` feature is already enabled, so components hold their presentation state themselves
rather than the application holding it for them.

Feature 017 has already closed the boundary: no feature module can style anything, every styled
widget comes from the component library, and the library is split into a behavior layer and an
appearance layer. That is what makes this feature tractable — every visual decision below is made
in **one place** instead of at the 119 call sites that styled things before.

## Technical Context

**Language/Version**: Rust, stable, MSRV 1.97 (pinned in workspace `Cargo.toml`)

**Primary Dependencies**: `iced 0.14.0` (features: tokio, canvas, advanced, lazy);
`ttf-parser 0.21` (dev-only, font assertions). **No new runtime dependency is added by this
feature** — the tonal ramps are baked constants (research R7).

**Storage**: N/A — this feature adds no persisted state. The existing follow-system/light/dark
preference in `micold-core::settings` is unchanged.

**Testing**: `cargo test --workspace` (`mise run test`). Verification splits by what can be
asserted without a human judging pixels — **not** by the crate boundary. Token invariants live in
`crates/micold-core/tests/`. Client-level structural gates (source scans over the rendering layer,
and behavioural tests driving a component directly) are ordinary automated tests, as feature 017
established with its six of them; SC-005a and SC-008 are verified this way. Only the thin
token→render conversion and the `view` call sites rely on the recorded `quickstart.md` procedure
under the constitution's Principle I GUI-wiring exception.

**Target Platform**: Linux, macOS, Windows desktop — parity required (Principle VI)

**Project Type**: Desktop application, three-crate Cargo workspace

**Performance Goals**: No regression in frame time, **measured for trend rather than gated**
(FR-039c) against the reference scene of FR-039b, with the pre-change figure captured before any
token value lands (T000z, SC-018). Shadows and state layers are per-widget style values resolved at
view time, not new render passes. The motion primitive already gates frames at rest
(`Progress::animating()` in `ui/cdk/motion.rs`) and must continue to — 017 holds the rendering layer
to exactly one frame-request site, and this feature's four new animations route through it rather
than adding a second (FR-039e).

**Constraints**: No behavior change except the notification surface (FR-036a). Terminal typography
exempt (FR-012). Tokens must remain nameable from a crate that cannot see `iced`. AA contrast is a
build-failing gate (FR-004).

**Scale/Scope**: ~15 type roles + 3 sidebar roles, ~36 color roles × 2 schemes, 6 elevation levels,
7 shape sizes, 7 state layers, 12 motion tokens. Every existing component restyled, three new ones
(`FormField`, `Ripple`, `Snackbar`). Two font binaries added. Four new animations. **No feature module styles
anything** — 017's boundary test fails the build if one does. Feature modules are still *edited*
where a call site must name a type role, pass a density, or migrate a placeholder onto a label
(T017–T021, T047, T053, T055, T061); what they may not do is decide how something looks.

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

- [x] **I. Test-First (NON-NEGOTIABLE)**: PASS. Every token value, contrast invariant, tone
  monotonicity check, type-role table and motion token is pure data in `micold-core` and gets a
  failing test first. The thin GUI conversion in `ui/style.rs` and the `view` call sites fall under
  the Principle I GUI-wiring exception — they invoke already-tested pure values and carry no
  decision logic — and are validated by `quickstart.md`. The two pieces of *new logic* are tested in the two different ways feature 017
  established. The snackbar queue discipline (FR-032a/b) is pure decision logic with no rendering in
  it, so it lives in `micold-core` and is unit-tested with no renderer. The ripple's geometry, phase
  progression and lifetime (FR-024b, FR-024e) live **inside the component instance** and are tested
  by driving that component directly from a client-level test — the pattern 017 shipped in
  `idle_requests_no_frames.rs`. Neither relies on the GUI-wiring exception; only the thin conversion
  in `ui/material/style.rs` and the `view` call sites do.
- [x] **II. Multi-Session Support**: PASS. No new session-scoped state. The snackbar queue is
  global view state, exactly as the notification stack it replaces already is.
- [x] **III. Worktree Integration**: PASS. No file or VCS operation is touched.
- [x] **IV. Local-First Storage (NON-NEGOTIABLE)**: PASS. Nothing is stored, nothing leaves the
  device. Both fonts are vendored in-repo, so no network fetch is introduced at build or run time.
- [x] **V. Rust + iced Stack**: PASS. iced only; no widget is forked. Token types (`Rgb`,
  `TypeRole`, `Elevation`, `StateLayer`) are plain data in core, making an invalid role/tone pair
  unrepresentable at the type level rather than checked at runtime.
- [x] **VI. Cross-Platform Parity**: PASS — and improved. Shipping Roboto removes the platform's
  default UI font as a source of divergence (FR-008), which is a parity *gain*. CI already builds
  and tests all three platforms.
- [x] **VII. Documentation First-Class**: PASS. `docs/` user-guide updates ship in the same change
  (FR-041), plus `assets/fonts/PROVENANCE.md` and `LICENSE` for Roboto (FR-009).
- [x] **VIII. Reusable UI Component Foundation**: PASS — and materially strengthened. The library
  now *wraps* the rendering stack rather than sitting beside it (feature 017): feature modules cannot
  import a styled widget or reach the styling layer, enforced by a build-failing test (feature 017).
  `Button`, `Text`, `TextField`, `Checkbox`, `Scrollable` and `Surface` already exist — feature 017
  built them, and this feature restyles rather than introduces them. The primitives genuinely new
  here are **`FormField`, `Ripple` and `Snackbar`**, and all three expose the chainable builder
  terminating in `.into()`. Pure layout primitives stay unwrapped by explicit carve-out
  (feature 017), since they carry no Material appearance.

**Post-Phase-1 re-check**: PASS. Principle VIII moved from "satisfied by convention" to "enforced
by a test", and components now own their own presentation state rather than the application holding
it (feature 017) — the most significant changes since the first check. Still no new dependency,
no new persisted state, and no platform branch; the ripple draws with the canvas facility already
enabled and already used by the terminal, and holds its state in the widget tree. Decisions stay
pure and tested in core; only storage moved into the components. See Complexity Tracking.

## Project Structure

### Documentation (this feature)

```text
specs/018-material3-visual-system/
├── plan.md              # This file
├── research.md          # Phase 0 output — iced capability findings, spec amendments
├── data-model.md        # Phase 1 output — token entity model
├── quickstart.md        # Phase 1 output — manual validation procedure
├── contracts/
│   ├── design-tokens.md # the revised design system contract (supersedes 003) — every *value*
│   └── component-api.md # API surface of the three new components — every *shape*
├── checklists/
│   └── requirements.md
└── tasks.md             # Phase 2 output (/speckit-tasks — NOT created here)
```

### Source Code (repository root)

```text
crates/micold-core/src/tokens/          # values re-authored here (017 moved the module)
├── palette.rs                          # M3 baseline tonal ramps + tag hues
├── typography.rs                       # 15 type roles + sidebar aliases
├── elevation.rs                        # 6 levels: tonal role + one shadow
├── shape.rs                            # 7 corner sizes
├── state.rs                            # 7 state-layer opacities
└── motion.rs                           # durations + easing curves
crates/micold-core/src/notify.rs        # snackbar queue discipline (pure, tested)

crates/micold-client/src/ui/material/   # THE ONLY PLACE THIS FEATURE EDITS
├── style.rs                            # token -> render conversion; state layers; elevation
├── ripple.rs                           # Material color/opacity over 017's cdk renderer
├── snackbar.rs                         # NEW — replaces the inline notification strip
├── form_field.rs                       # NEW — label/hint/error/adornment parts
├── text_field.rs, select.rs            # filled text-field anatomy
├── toolbar.rs                          # small app bar + elevate-on-scroll
├── tree_view.rs, menu.rs, modal.rs     # density, menu and dialog anatomy
├── tag.rs, toggle_chip.rs              # chip anatomy
└── progress.rs                         # indeterminate linear indicator

assets/fonts/                           # Roboto 400 + 500, licence, provenance
docs/user-guide/                        # updated for the new visual system
```

**Structure Decision**: The three-crate workspace already on `main` is kept as-is. The only
structural move is `tokens.rs` (and the pure half of `motion.rs`) from `micold-client` into
`micold-core`, expanded from a single file into a `tokens/` module directory because it grows from
~216 lines to roughly six scale tables. This move is what makes feature 017 enforceable by the compiler
rather than by convention, and it is friction-free: `tokens.rs` already imports only
`micold_core::naming` and `micold_core::theme` (research R5).

## Phase Ordering

Sequenced so each user story is independently demonstrable, matching the spec's P1–P5 priorities.

| Phase | Delivers | Spec story |
|-------|----------|------------|
| A | Token **values** re-authored in the core: baseline palette, tags, scales — `tasks.md` Phase 0 (T000z, T000a–T000i) | prerequisite |
| B | Surfaces, elevation, shape applied; borders removed | US1 (P1) |
| C | Roboto shipped; type roles assigned | US2 (P2) |
| D | State layers + ripple appearance; text-field focus | US3 (P3) |
| E | Component anatomy; text field; progress; snackbar | US4 (P4) |
| F | Motion tokens applied | US5 (P5) |

Phase A is the only hard prerequisite within this feature — every story reads token values from it.
B through F touch disjoint concerns inside the appearance layer and can be reordered or
parallelised; each ends in a demonstrable state. All of them presuppose 017 is complete.

## Complexity Tracking

> Recorded for review visibility. Neither item is a constitution violation.

| Item | Why needed | Simpler alternative rejected because |
|------|------------|--------------------------------------|
| Three new shared primitives (`FormField`, `Ripple`, `Snackbar`) rather than styling in place | `FormField` wraps whichever control it is given and owns the parts every field shares — container, active indicator, label, supporting text, error state, adornments — on the model of Angular Material's form field, so a change to how fields present a label is one edit rather than one per field type (FR-031c). `Snackbar` replaces an inline layout node with a floating one and owns queue presentation. `Ripple` is the appearance half of 017's behavior-layer renderer. | Reassembling the field chrome at each of the seven input call sites is what the codebase does today, and is the duplication Principle VIII exists to prevent. `Surface` is **not** on this list — it already exists (017); this feature only puts the elevation scale behind it. |
| Snackbar queue/timeout logic in `micold-core`, not in the UI layer | It is decision logic (which notification is visible, when it expires, how dedup interacts with the queue), so Principle I requires it to be tested — and the GUI-wiring exception explicitly does not cover code with branching of its own. | Putting it in `ui/` was rejected: it would be structurally unreachable from tests, which is the precise situation the constitution's exception carve-out refuses to extend to. |
| Ripple state held in the component instance rather than centrally | FR-024e requires it: a call site presses a button and never learns a ripple exists. Feature 017's behavior layer provides the per-instance state hooks that make this possible, which is precisely why the ripple was deferred out of 017 (FR-024f). | Holding it in `micold-core` keyed by an animation key was this plan's original design and is now **rejected**: FR-024e forbids registering an animation key, and central state cannot deliver per-element independence (FR-024d) without the application knowing about every rippling element. Testability was the original argument for it, and 017 removed that argument — a client-level test drives the component and asserts its state directly, as `idle_requests_no_frames.rs` already does for the motion primitive. |

## Risks

| Risk | Mitigation |
|------|------------|
| Transcription error in the baked tonal ramps (research R7) | Test invariants, not digits: the AA gate covers every pair, plus a monotonicity test asserting luminance decreases with tone. A wrong digit that still passes both is not visually material. |
| The purple identity change (clarification Q1) surprises on first run | Deliberate and recorded in FR-005b. `quickstart.md` calls it out as the first thing to verify, so it is confirmed rather than discovered. |
| Sidebar rows grow from ~28dp to the dense figure, showing fewer worktrees | ~~FR-026a caps this: visible-worktree count must not drop materially.~~ **Revised by BUG-005.** The cap was the reason the row height was deleted rather than applied, which cost the component its density entirely. FR-026a is now a floor on *compactness* — the scale's own most-dense step, and no further — and the resulting drop is accepted and measured rather than forbidden. `quickstart.md` §B4 records the before and after count against the same repository, which is now a number to publish rather than a gate to pass. |
| A stated height attached to the wrong node applies to some instances of a component and not others | A height belongs to the **row**, not to a line inside it and not to a spacer that happens to be there. `tree_view.rs` hung §7.2's floor on each row's indent spacer, whose width is the row's depth × 16dp — so at depth 0 the spacer was `Fixed(0)` wide, iced dropped it as void, and the floor silently applied to nested rows only. Nothing separates that from "the component has no height": the specimen you measure decides which answer you get, and the gate held one specimen, at depth 0. Worse, the arithmetic used to *justify removing* the height was done with it still on the wrong node — flooring the name line makes a two-line row 36 + 2 + 16 = 54 where flooring the row leaves it at 41.6 — so a 7.7% cost was reported as ~30% and a contract figure was deleted to avoid it. SC-008d requires the gate to exercise every axis a component varies on, depth included. Found the hard way — BUG-005, and the void-child half of it is the fourth instance of that trap after `FormField`'s slots, the snackbar's minimum height and this same file's first fix. |
| SC-003 ("zero raw sizes") decays as new code is written | Enforced by a test (`type_role_call_sites.rs`) rather than by review, so a regression fails the build. |
| Snackbar timeout makes an error vanish unread | FR-032b gives errors the 10 s long duration and keeps manual dismissal. Flagged during clarification; the user chose full Material semantics with this mitigation. |
| Ripple cost: many simultaneous animations, or a redraw storm | Ripples are short (300 ms + 200 ms), self-removing (FR-024d), and the existing animation clock already idles at rest. The invariant "no ripple state retained once faded" is tested, so a leak fails the build rather than degrading frame time silently. |
| Giving a component a fixed height leaves its content where the framework's defaults put it | Adding a height to a previously content-sized component changes what its padding *means*: zero vertical padding stops saying "size me by my label" and starts saying "pin my label to the top edge". Two separate defaults do this and both point the same way — a button stretches its content node to the fixed height and the text widget then draws its glyphs at the top of that node; a container lays its child out loosely and aligns it top-leading. FR-030a requires the anatomy to state the alignment, and SC-008a is verified by **rasterising**: the layout tree is identical either way, so only the drawn pixels can tell centred from top-aligned. Found the hard way — BUG-001, and again in the app bar during its sweep. |

| A builder call that *also* sets a length silently discards the one that stated the anatomy figure | `Container::center_x(w)` is `self.width(w).align_x(Center)`; `center_y`/`center` likewise. So `.width(Fixed(48)).center_x(Fill)` reads as "48dp wide, centred" and means "as wide as there is room for, centred" — the call stating the contract's figure is dead, overwritten by the one meant only to centre. Nothing at the call site shows it, and the component still *looks* right in isolation: it only misbehaves beside a `Length::Fill` sibling, where it takes an equal share of the free space. SC-008b is verified by laying each component out under **two differently-sized limits** and declaring each axis `Fixed`/`Fill`/`Content`: one measurement cannot separate `Fixed(48)` from `Fill`, since offering exactly 48dp makes them agree. Found the hard way — BUG-002, in the app bar and the terminal's bottom bar at once. |

| A constant that restates another component's dimension, in a component built twice | The overflow menu and the project switcher are two panels hanging off one bar, and each carried its own `const TOP_OFFSET: f32 = 52.0; // approx. toolbar height` — an eyeballed copy of a bar that was content-sized when it was written and has been a fixed 64dp plus a 1dp divider since §7.1 landed. Neither copy moved, because nothing links a copy to its original: the defect is not that 52 is wrong, it is that 52 is *stated* where it should be read. FR-029a requires the offset to be derived from the bar's own anatomy, and the contract gains the row it derives from (§7.1's bottom edge). The duplication is the multiplier: the same panel content was hand-built in two modules, so §7.5's item height reached one of them and left the other 12dp shorter, and every figure in §7.5 has to be applied as many times as there are copies. FR-029b makes the item row one component, on the model FR-031c already sets for form fields. Found the hard way — BUG-003, in both panels at once. |
| Every gate reads one component, so nothing can see two of them collide | This feature's four checks are each scoped to a single component: constants (SC-008), drawn content (SC-008a), laid-out box (SC-008b), and feature 019's containment gate reads a child against its own parent. A panel laid over the app bar is invisible to all four — it is the right size, correctly filled, and inside the window that owns it. SC-008c adds the missing scope: a relationship between two *independent* components, asserted over the covered states. It must assert rather than snapshot, because a fixture adopts a defect older than itself as its expected value; the same reason T093 had to regenerate rather than trust the fixture that was green throughout BUG-002. |

| Unifying a component's **part** leaves the **whole** free to keep diverging | BUG-003 made the menu item row one shared component and, having removed the visible symptom, left the panel around it and the trigger that opens it as two hand-built copies each. Everything the shared components gained afterwards therefore reached one of the two: §7.3's 48dp target and 24dp glyph (the switcher's trigger stayed at 28dp and 14dp), the panel's enter/exit fade (the switcher's panel returns nothing when closed, so it has nothing left to fade), and even §7.5's width (240 against the copy's 260, from the same edge). None of it fails a per-component check, because the copy answers to no contract row — §7.1 describes *the* app-bar action, and nothing said the switcher was one. FR-029c states that the panel and the trigger are shared components too, and the fix is deletion rather than alignment: the copies go, and their call sites build the shared ones. Found the hard way — BUG-007, one bug after the row was unified. |
| Nothing compares two instances of the same kind | The gate classes above each read one component against a figure (SC-008, SC-008a, SC-008b) or one component against another *of a different kind* (SC-008c: a panel against the bar). A fork is invisible to all of them: both copies are internally consistent, and the second answers to no figure. SC-008e adds the last scope — same kind, read against each other — so two app-bar triggers that differ in target or glyph, or two bar-anchored panels that differ in width, fail the build regardless of what any contract row says about either. |

**Bugfix**: 2026-08-08 — BUG-007 Updated from bugfix patch: recorded the part-versus-whole risk above and the missing same-kind gate scope. No change of approach — the fix is subtraction, deleting `project_switcher.rs`'s trigger and overlay in favour of `IconButton` and `MenuOverlay`, which is what FR-029b's model already implied for the row.

**Bugfix**: 2026-08-07 — BUG-005 Updated from bugfix patch: recorded the wrong-attachment-point risk above, and the two gate classes that could not see it — a single-specimen size check, which cannot find a defect that depends on which instance you measure, and a fixture whose sixteen covered states contain no nested row at all. Also a change of *figures*, not of approach: §7.2's base moves from Material 2's 48dp list item to Material 3's 56 / 72, since the base being scaled had never matched the feature's subject.

**Bugfix**: 2026-08-07 — BUG-003 Updated from bugfix patch: recorded the restated-constant risk and the single-component scope shared by every gate this feature built. No change of approach; two additions to it — a panel's offset is derived, not stated (FR-029a), and the item row is one component rather than one per panel (FR-029b).

**Bugfix**: 2026-08-07 — BUG-001 Updated from bugfix patch: recorded that a fixed height reinterprets a component's padding, and the gate class (constants-only anatomy checks) that could not see it.

**Bugfix**: 2026-08-07 — BUG-002 Updated from bugfix patch: recorded the builder-aliasing risk above, and the gate class (a snapshot, which records a defect older than itself as its own baseline) that could not see it. No change of approach — the US4 anatomy pass was structurally correct; one of its components did not apply the figure it named.
