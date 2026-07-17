# Phase 0 Research: Generic Motion Library & Overlay Fade In/Out

All items below were resolved from the existing codebase and the pinned iced 0.13 source; no
open NEEDS CLARIFICATION remain.

## R1 — Generic animation driver shape

**Decision**: A keyed collection `Animator<K: Copy + Eq + Hash>` holding `Track { value,
target, speed }` per key, with `set(key, value)` (snap), `to(key, target, speed)` (declare
target + per-tick step), `get(key) -> f32` (0.0 default), `tick()` (advance all toward
target, clamped), and `animating() -> bool` (any track off-target).

**Rationale**: Collapses today's four hardcoded fields (`menu_anim`, `sidebar_anim`,
`main_anim`, `handle_hover_anim`) + four `approach()` calls + four-way subscription check + the
`Anim` struct into one collection driven by one `tick()` and gated by one `animating()`.
Adding an animated element becomes: pick a key, set a target, read `get()` — satisfying
FR-007/FR-008. Per-track `speed` preserves that fades, slides, and the hover ramp each move at
their own rate (FR-009). The current `approach(current, target, step)` math is reused verbatim
inside `tick()`.

**Alternatives considered**:
- *Keep per-field structs* — rejected: this is exactly the duplicated "static list" the
  feature removes.
- *Full ECS / tween-graph library* — rejected: YAGNI for a handful of scalar tracks; adds a
  dependency and concepts the app does not need.
- *`&'static str` keys* — rejected in favor of a typed key enum on the consumer side for
  compile-time safety; the core stays generic over `K` so it imposes no key scheme.

## R2 — Overlay exit-animation lifecycle (the fade-out problem)

**Decision**: Handle fade-out entirely in the GUI binary via a snapshot. The `update` wrapper
(where `main_key` changes are already detected) captures the open overlay + a clone of its
draft **before** the reducer runs; if the overlay transitioned `Some(X) → None`, it stores a
`ClosingOverlay` snapshot and sets the `Overlay` motion target to 0. `view()` renders the live
overlay while open (fade-in) or the snapshot while dismissing (fade-out); the snapshot is
dropped once progress reaches ~0.

**Rationale**: Closing an overlay in the pure core synchronously sets `overlay = None` **and**
clears the draft (`settings_draft = None`, `rename_draft = None`, `worktree_form = None`,
`selector = None`), so the data needed to keep drawing it is gone the instant it closes. A
pre-reducer snapshot is the only way to keep rendering it without touching the core. This
honors the code's stated convention — "Animation progress is tracked by the binary (gui
runtime), not the pure core" (`src/app.rs`) — keeps FR-012 (core/persistence unchanged), and
handles cancel, Esc, and successful-submit closes through one uniform path.

**Alternatives considered**:
- *Add a "closing" phase to the pure core* — rejected: pushes animation-driven state and
  timing into the render-free core, touches all five close handlers, and risks the tested
  reducer/persistence logic for no user-visible gain.
- *Defer the close reducer call until the fade completes* — rejected: submit actions
  (save/confirm/create) that must apply immediately and may fail validation cannot be deferred
  cleanly; mixing deferred dismissals with immediate submits is more complex than one snapshot
  path.

## R3 — Reveal-beneath fade without per-widget opacity

**Decision**: Compose the overlay transition from two primitives the renderer does expose:
an animated **scrim** (`renderer.fill_quad` with alpha = `progress × SCRIM_ALPHA`) that dims
in and clears out to reveal `base`, plus a subtle **dialog transform** (`scale`
0.96→1.0 about center via `renderer.with_transformation`, analogous to the existing `slide`'s
use of `translate`). The existing `fade` widget may additionally fade the dialog contents
toward the dialog surface color.

The dialog scale/lift is a **committed, required** part of the transition (confirmed
2026-07-17 via the motion-style decision) — not an optional extra: the `scale` primitive
(task T012) MUST be implemented and `Modal` (T013) MUST compose it. The overlay motion is
therefore: scrim fade (reveals `base`) + dialog fade + dialog scale/lift.

**Rationale**: Verified against the pinned sources
(`~/.cargo/registry/.../iced_core-0.13.2/src/renderer.rs`): the `Renderer` trait offers
`fill_quad`, `start_layer`/`with_layer`, and `start_transformation`/`with_transformation`,
but **no general per-widget opacity** — `opacity` exists only on `Image`/`Svg`, and `opaque`
is input-capture only. An alpha-animated scrim is the only way to progressively reveal `base`
during exit; a center scale reads as a Material dialog "lift" and needs only a transform.

**Alternatives considered**:
- *Reuse the existing bg-composite `fade` on the whole overlay* — rejected: it composites the
  background color **over** its child, so it washes the whole window to the background rather
  than revealing `base` beneath — wrong for a modal exit.
- *True alpha fade of the dialog* — impossible in iced 0.13 (no general opacity).
- *Scale-only, no scrim* — rejected: the scrim dim/undim is what makes the transition read as
  modal and reveals `base`.

## R4 — Timing model

**Decision**: Express timing as named `Duration` constants and convert to the per-tick step the
`Animator` consumes: `step(d) = ANIM_TICK.as_secs_f32() / d.as_secs_f32()` (clamped to
`(0,1]`). Overlay enter = 300 ms, exit = 240 ms. Existing animations keep today's feel,
re-expressed as durations: menu/main fade ≈ 90 ms (step 0.18), sidebar slide ≈ 114 ms (0.14),
handle hover ≈ 800 ms (0.02).

**Rationale**: The previous ~90 ms overlay-equivalent fade was reported as imperceptible;
300/240 ms is clearly visible without feeling sluggish (FR-004/FR-005), and durations are the
legible, tunable representation FR-013 requires. `ANIM_TICK` (16 ms ≈ 60 fps) already exists.

**Alternatives considered**: raw per-tick step floats (rejected — opaque, the thing FR-013
forbids); an easing-curve engine (deferred — linear `approach` already matches the app's
current feel; curves can be added later behind the same `Track`).

## R5 — Where the driver lives + extractability

**Decision**: The framework-agnostic core lives in `src/motion.rs` as part of the render-free
lib (`pub mod motion;`), with **zero** dependency on iced or any app-specific type — generic
over `K`, using only `std` + `f32`. It is unit-tested under `cargo test
--no-default-features`. The binary (`App`) owns one `Animator<MotionKey>` instance; the
iced-specific render helpers (`fade`/`slide`/`scale`/`Modal`) stay under `src/ui/material/`
and consume progress from it.

Per the delivery decision (see plan.md), the core ships now as a **self-contained module**
(documented public API, no app coupling) rather than a separately published crate — extraction
into its own crate later is a one-step move (drop the file into a new crate, add a path
dependency), and requires no code change because the module already references nothing
app-specific. This satisfies FR-015/016/017 and SC-008 without converting the repo to a Cargo
workspace in this feature.

**Rationale**: Keeping the core render-free and app-agnostic makes it (a) testable per
Principle I, (b) reusable outside this project per the extraction requirement, and (c) aligned
with the existing pure-core/gui-binary split. Not part of the persisted `State` (which stays
Clone/Eq): the `Animator` is standalone runtime UI state owned by the binary, so persistence
is untouched (FR-012).

**Alternatives considered**:
- *Separate workspace crate now* — viable and genuinely publishable, but rejected for this
  feature per the user's delivery choice: it adds workspace conversion (Cargo.toml, CI test
  invocation, packaging-path review) for no additional behavior; the self-contained module
  achieves the same reuse boundary with far less churn.
- *Put the driver in `src/ui/` (gui-only)* — rejected: it would couple the reusable core to
  the `gui` feature and prevent `--no-default-features` testing and external reuse.

## R6 — Reusable overlay component API (Principle VIII)

**Decision**: Add `material::Modal`, a chainable builder mirroring the existing `MenuOverlay`:
`Modal::new(base, dialog, roles).progress(p).into()`, returning `base` as-is at `p ≤ 0.001`.
It owns the scrim + centered dialog + transform + `opaque` input-capture. The five overlay
render functions build only their dialog body and hand it to `Modal` with a `progress`. A
small `material::scale(content, progress)` transform primitive joins `fade`/`slide` in
`src/ui/material/animation.rs`.

**Rationale**: One shared transition used by every overlay (no per-overlay bespoke code)
satisfies FR-011 and the constitution's builder-API rule (Principle VIII); it matches the
established pattern of `MenuOverlay`/`IconButton`/`TreeView`. `scale` generalizes the transform
technique already proven by `slide`.

**Alternatives considered**: per-overlay ad-hoc `stack![base, fade(...)]` wiring — rejected:
duplicates the transition five times and violates the component-reuse gate.
