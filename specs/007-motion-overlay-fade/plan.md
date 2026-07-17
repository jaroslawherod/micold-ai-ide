# Implementation Plan: Generic Motion Library & Overlay Fade In/Out

**Branch**: `007-motion-overlay-fade` | **Date**: 2026-07-17 | **Spec**: [spec.md](./spec.md)

**Input**: Feature specification from `/specs/007-motion-overlay-fade/spec.md`

## Summary

Introduce one render-free, generic animation driver (a keyed `Animator`) that every widget
reads from, migrate the four existing ad-hoc animations onto it, and add fade in/out to all
five modal overlays via a single reusable Material `Modal` transition component. Overlay
fade-out reveals the app beneath and is driven entirely in the rendering layer using a
GUI-side snapshot of the just-closed overlay, so the pure core (overlay lifecycle, drafts,
persistence) is untouched. Timing is expressed as durations (overlay enter ≈300 ms / exit
≈240 ms); existing animations keep their current feel. iced 0.13 has no general per-widget
opacity, so the reveal-beneath fade is composed from an animated scrim plus a dialog
transform.

The animation library is delivered as two layers so its core is reusable **outside** this
project (FR-015/016/017, SC-008): a **framework-agnostic core** — `src/motion.rs`,
`Animator<K>` — with zero dependency on iced or any app-specific type (generic over the key
type, pure `f32`/`std` only), and a thin set of **iced render helpers** (`fade`, `slide`,
`scale`, `Modal`) that depend on iced and consume progress values the core produces. Per the
delivery decision, the core ships now as a **self-contained module** (not app-coupled,
documented public API) structured so lifting it into its own crate later is a one-step move;
the repo is not converted to a Cargo workspace in this feature.

## Technical Context

**Language/Version**: Rust, edition 2021, rust-version 1.80 (stable, via `mise`)

**Primary Dependencies**: `iced` 0.13 (existing; `advanced` widget API for the render
wrappers). No new dependencies.

**Storage**: N/A — presentation-only; no persisted state added or changed.

**Testing**: `cargo test --no-default-features` for the render-free driver (`motion`), plus
`cargo test` (default/gui) for the workspace; GUI transitions validated by running the app
(see quickstart.md).

**Target Platform**: Desktop — Linux, macOS, Windows.

**Project Type**: Desktop application (Rust + iced), single project.

**Performance Goals**: 60 fps animation clock (~16 ms tick, existing `ANIM_TICK`); zero
animation work at rest (clock subscription runs only while something is animating).

**Constraints**: iced 0.13 exposes no general per-widget opacity (only `Image`/`Svg`), so the
overlay fade uses an animated dimming scrim (`fill_quad` alpha) + a dialog transform
(`with_transformation`) rather than true alpha on the dialog. The pure core
(`micold_ai_ide::app` and siblings) MUST remain render-free and unchanged; all animation and
exit-lifecycle state lives in the GUI binary.

**Scale/Scope**: 5 modal overlays + 4 existing animations unified; ~2 new modules
(`src/motion.rs`, `src/ui/material/modal.rs`) plus edits to ~8 files.

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

- [x] **I. Test-First (NON-NEGOTIABLE)**: The animation driver's logic (`Animator`: `to`,
  `tick`, `animating`, `get`) is pure and render-free; its tests are written failing-first in
  `src/motion.rs` and run under `cargo test --no-default-features`. The GUI render wrappers
  (`Modal`, `scale`) and the exit-lifecycle wiring are presentation code with no meaningful
  unit surface; they are validated by the quickstart run (rendering cannot be asserted in a
  headless unit test). No pure logic ships without a preceding failing test.
- [x] **II. Multi-Session Support**: N/A — no session state is added. Animation progress is
  ephemeral, app-global UI state (never per-session, never persisted), so no session can leak
  animation state into another.
- [x] **III. Worktree Integration**: N/A — no filesystem or VCS operations.
- [x] **IV. Local-First Storage (NON-NEGOTIABLE)**: PASS — no network, no persistence change;
  fully offline. Persistence behavior is explicitly unchanged (FR-012).
- [x] **V. Rust + iced Stack**: PASS — Rust + iced only, no new framework. Invalid states are
  made unrepresentable: the closing overlay is modeled as a typed `ClosingOverlay` enum whose
  variants carry exactly the data each overlay needs to render its exit.
- [x] **VI. Cross-Platform Parity**: PASS — pure Rust + iced with no OS branching; identical
  behavior on all three platforms; covered by existing cross-platform CI.
- [x] **VII. Documentation First-Class**: PASS — the user-facing motion change ships with a
  user-guide update in the same change (a short "Motion & animations" note in the appearance
  docs). Tracked as a documentation task in tasks.md.
- [x] **VIII. Reusable UI Component Foundation**: PASS — the overlay transition is a single
  shared `material::Modal` component with a chainable builder API terminating in `.into()`
  (mirroring the existing `MenuOverlay`), reused by all five overlays; the `Animator` is the
  shared motion mechanism; any transform primitive (`scale`) is added to the shared
  `material` animation module, not forked per feature.

**Result**: All gates PASS. Complexity Tracking is empty.

**Post-Phase-1 re-check**: The design artifacts (research.md, data-model.md,
contracts/animation-api.md, quickstart.md) introduce no new violations. The extractable core
stays render-free and app-agnostic (I, and the FR-015 boundary), the transition is a single
builder-style shared component (VIII), and the pure core/persistence remain untouched (IV,
FR-012). Gates still PASS.

## Project Structure

### Documentation (this feature)

```text
specs/007-motion-overlay-fade/
├── plan.md              # This file
├── research.md          # Phase 0 output
├── data-model.md        # Phase 1 output
├── quickstart.md        # Phase 1 output
├── contracts/
│   └── animation-api.md # Phase 1 output — shared component/interface contracts
├── checklists/
│   └── requirements.md  # From /speckit-specify
└── tasks.md             # Phase 2 output (/speckit-tasks — NOT created here)
```

### Source Code (repository root)

```text
src/
├── lib.rs                     # + pub mod motion;
├── motion.rs                  # NEW — pure, render-free, framework-agnostic Animator<K>
│                              #   (Principle I; zero app/iced coupling; extractable, FR-015)
├── main.rs                    # Animator<MotionKey>, ClosingOverlay, exit lifecycle,
│                              #   duration constants, view()/subscription() rewire
└── ui/
    ├── mod.rs                 # MotionKey enum; delete Anim; view() takes &Animator<MotionKey>
    └── material/
        ├── mod.rs             # export Modal (+ scale)
        ├── animation.rs       # existing fade/slide; + scale transform primitive
        └── modal.rs           # NEW — reusable Modal transition (builder, Principle VIII)
    ├── about.rs               # use Modal; accept progress
    ├── project_selector.rs    # use Modal; accept progress
    ├── rename.rs              # use Modal; accept progress
    ├── worktree_form.rs       # use Modal; accept progress
    └── settings_form.rs       # use Modal; accept progress

docs/user-guide/
└── appearance-theming.md      # + short "Motion & animations" note (Principle VII)
```

**Structure Decision**: Single project, matching the existing layout. The pure/render-free
core stays in `src/*.rs` (with the new `motion.rs` joining it, no iced dependency); all GUI
rendering stays under `src/ui/` behind the `gui` feature, exactly as today. No new
top-level structure is introduced.

## Complexity Tracking

> No constitution violations. Section intentionally empty.
