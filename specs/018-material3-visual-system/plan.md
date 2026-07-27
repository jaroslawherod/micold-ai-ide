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

**Primary Dependencies**: `iced 0.13.1` (features: tokio, canvas, advanced, lazy);
`ttf-parser 0.21` (dev-only, font assertions). **No new runtime dependency is added by this
feature** — the tonal ramps are baked constants (research R7).

**Storage**: N/A — this feature adds no persisted state. The existing follow-system/light/dark
preference in `micold-core::settings` is unchanged.

**Testing**: `cargo test --workspace` (`mise run test`). Token invariants live in
`crates/micold-core/tests/`; GUI wiring is validated by the recorded `quickstart.md` procedure
under the constitution's Principle I GUI-wiring exception.

**Target Platform**: Linux, macOS, Windows desktop — parity required (Principle VI)

**Project Type**: Desktop application, three-crate Cargo workspace

**Performance Goals**: No regression in frame time. Shadows and state layers are per-widget style
values resolved at view time, not new render passes. The animation clock already gates itself at
rest (`Animator::animating`) and must continue to.

**Constraints**: No behavior change except the notification surface (FR-036a). Terminal typography
exempt (FR-012). Tokens must remain nameable from a crate that cannot see `iced`. AA contrast is a
build-failing gate (FR-004).

**Scale/Scope**: ~15 type roles + 3 sidebar roles, ~36 color roles × 2 schemes, 6 elevation levels,
7 shape sizes, 7 state layers, 12 motion tokens. Every existing component restyled, two new ones
(snackbar, form field). Two font binaries added. Four new animations. **No feature module is edited**
— 017's boundary test fails the build if one is.

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

- [x] **I. Test-First (NON-NEGOTIABLE)**: PASS. Every token value, contrast invariant, tone
  monotonicity check, type-role table and motion token is pure data in `micold-core` and gets a
  failing test first. The thin GUI conversion in `ui/style.rs` and the `view` call sites fall under
  the Principle I GUI-wiring exception — they invoke already-tested pure values and carry no
  decision logic — and are validated by `quickstart.md`. The two pieces of *new logic* — the snackbar queue discipline (FR-032a/b) and the ripple's
  geometry and phase progression (FR-024b) — are expressed as pure functions and pure data in
  `micold-core`, unit-tested with no renderer, while the components hold the transient state
  itself (feature 017).
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
  New primitives (`Button`, `Text`, `TextField`, `Checkbox`, `Scrollable`, `Ripple`, `Surface`,
  `Snackbar`) all expose the chainable builder terminating in `.into()`. Pure layout primitives
  stay unwrapped by explicit carve-out (feature 017), since they carry no Material appearance.

**Post-Phase-1 re-check**: PASS. Principle VIII moved from "satisfied by convention" to "enforced
by a test", and components now own their own presentation state rather than the application holding
it (feature 017–feature 017) — the most significant changes since the first check. Still no new dependency,
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
│   └── design-tokens.md # the revised design system contract (supersedes 003)
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
| A | Token **values** re-authored in the core: baseline palette, tags, scales | prerequisite |
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
| Two new shared primitives (`Surface`, `Snackbar`) rather than styling in place | FR-015 puts elevation on seven different surface kinds; without a shared primitive each would re-derive tonal-role + shadow + corner independently, which is exactly the duplication Principle VIII exists to prevent. `Snackbar` replaces an inline layout node with a floating one and owns queue presentation. | Styling each surface at its call site was rejected: it would spread the elevation table across ~7 modules and make a level change a 7-site edit. |
| Snackbar queue/timeout logic in `micold-core`, not in the UI layer | It is decision logic (which notification is visible, when it expires, how dedup interacts with the queue), so Principle I requires it to be tested — and the GUI-wiring exception explicitly does not cover code with branching of its own. | Putting it in `ui/` was rejected: it would be structurally unreachable from tests, which is the precise situation the constitution's exception carve-out refuses to extend to. |
| Ripple state in `micold-core` too | Same reasoning: which element is rippling, from where, and when it expires is branching logic, not styling. Only the drawing is rendering-specific. | Holding ripple state in the widget was rejected for the same untestability reason, and because per-element independence (FR-024d) is exactly the kind of invariant that needs a test. |

## Risks

| Risk | Mitigation |
|------|------------|
| Transcription error in the baked tonal ramps (research R7) | Test invariants, not digits: the AA gate covers every pair, plus a monotonicity test asserting luminance decreases with tone. A wrong digit that still passes both is not visually material. |
| The purple identity change (clarification Q1) surprises on first run | Deliberate and recorded in FR-005b. `quickstart.md` calls it out as the first thing to verify, so it is confirmed rather than discovered. |
| Sidebar rows grow from ~28dp to 36dp, showing fewer worktrees | FR-026a caps this: visible-worktree count must not drop materially. `quickstart.md` measures it before and after against the same repository. |
| SC-003 ("zero raw sizes") decays as new code is written | Enforced by a test (`type_role_call_sites.rs`) rather than by review, so a regression fails the build. |
| Snackbar timeout makes an error vanish unread | FR-032b gives errors the 10 s long duration and keeps manual dismissal. Flagged during clarification; the user chose full Material semantics with this mitigation. |
| Ripple cost: many simultaneous animations, or a redraw storm | Ripples are short (300 ms + 200 ms), self-removing (FR-024d), and the existing animation clock already idles at rest. The invariant "no ripple state retained once faded" is tested, so a leak fails the build rather than degrading frame time silently. |
