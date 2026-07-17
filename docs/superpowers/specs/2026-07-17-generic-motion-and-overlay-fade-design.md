# Generic motion library + overlay fade in/out — design

- **Date:** 2026-07-17
- **Status:** Approved (pending spec review)
- **Author:** jaro-herod (with Claude)
- **Constitution:** Principle I (Test-First), Principle IV (Local-First), Principle V (Rust/iced), Principle VIII (Reusable UI Component Foundation)

## Problem

Modal overlays (About, project selector, rename, add-worktree, Settings) appear and
disappear instantly — no enter/exit motion. Every other animation in the app
(overflow-menu fade, sidebar slide, main-view fade, resize-handle hover) is driven by a
**hardcoded per-animation field** repeated across four places:

- `App` (binary) holds four `f32` fields: `menu_anim`, `sidebar_anim`, `main_anim`,
  `handle_hover_anim`.
- `AnimationTick` calls `approach()` once per field.
- `subscription` recomputes each target and OR-checks four "is it still moving" tests.
- `ui::Anim` mirrors the four fields, populated by hand in `view()`.

Adding overlay fades naively would add five more fields × four places — the "static list
of all animations" we want to avoid. And a second problem blocks fade-**out**: closing an
overlay synchronously sets `overlay = None` **and clears the draft** (`settings_draft =
None`, `rename_draft = None`, `worktree_form = None`, `selector = None`) in the render-free
core, so the data needed to keep drawing the overlay is gone the instant it closes.

A third issue surfaced during review: the existing fade runs at `FADE_STEP = 0.18` per
16ms tick, reaching full opacity in ~6 ticks ≈ **~90ms** — too fast to perceive.

## Goals

1. One **generic, reusable animation driver** — no per-animation fields/branches. Adding an
   animated thing costs: declare a key, set a target, read its value.
2. **Migrate** the existing four animations (menu, sidebar, main, handle-hover) onto it.
3. **Overlay fade in AND fade out** for all five modal overlays, reusing a shared modal
   component (Principle VIII), with the pure core left untouched.
4. Timing expressed as **legible, tunable durations (ms)**, not magic step floats. Overlay
   fade is clearly perceptible: **enter ~300ms / exit ~240ms**.

## Non-goals

- A full pluggable transition catalog (slide-up, flip, spring, per-component selectable).
  Out of scope (YAGNI). We add exactly the primitives the overlays need.
- Changing overlay lifecycle, dismissal rules, validation, or persistence in the core.
- Reworking the overflow-menu / sidebar / main-view *visuals* — only their *driver* moves.

## Key decisions

| # | Decision | Rationale |
|---|----------|-----------|
| D1 | One generic keyed `Animator`; migrate all existing animations onto it. | Removes the four-fields-×-four-places duplication; matches "reusable lib cross widgets/components". |
| D2 | Fade-out via a **GUI-side snapshot**; the pure core is untouched. | Honors the code's stated convention ("animation progress is tracked by the binary, not the pure core"); zero risk to tested reducer logic + persistence; one uniform path for both cancel and successful-submit closes. |
| D3 | Modal transition = **scrim-alpha fade + optional dialog scale/lift** (no true opacity). | iced 0.13 exposes no general per-widget opacity (only `Image`/`Svg`). Scrim alpha reveals `base` beneath; scale/translate via `with_transformation` reads as fade+lift. |
| D4 | Timing as **ms durations** converted to per-tick step; overlay enter ~300ms / exit ~240ms. | Fixes the imperceptible ~90ms fade; makes the feel legible and tunable. |

## Architecture

### 1. Pure animation driver — `micold_ai_ide::motion::Animator<K>`

New render-free module `src/motion.rs` (`pub mod motion;` in `lib.rs`). No iced dependency,
so it is unit-tested with `cargo test --no-default-features` (Principle I). It is **not**
part of the persisted `State` (which stays Clone/Eq) — it is a standalone struct the binary
owns.

```rust
/// A single animated scalar moving toward a target at a fixed per-tick step.
pub struct Track { pub value: f32, pub target: f32, pub speed: f32 }

/// A keyed collection of independent animated scalars. `K` identifies each track.
pub struct Animator<K> { tracks: HashMap<K, Track> }

impl<K: Copy + Eq + Hash> Animator<K> {
    /// Snap a track to `value` (initialization; no animation).
    pub fn set(&mut self, key: K, value: f32);
    /// Declare/refresh a track's target and per-tick step (`speed`). Creates it at the
    /// current value (or `target` if new and never set) so it animates from where it is.
    pub fn to(&mut self, key: K, target: f32, speed: f32);
    /// Current value of a track (0.0 if the key was never set — safe default).
    pub fn get(&self, key: K) -> f32;
    /// Advance every track toward its target by its `speed`, clamped so it never overshoots.
    pub fn tick(&mut self);
    /// True while any track has not reached its target — gates the animation subscription.
    pub fn animating(&self) -> bool;
}
```

`tick()` reuses today's `approach` math (move toward target by at most `speed`). Per-track
`speed` preserves that fades, slides, and the hover ramp each move at their own rate.

This single type replaces the four `_anim` fields, the four `approach()` lines, the
four-way subscription check, and the `Anim` struct.

### 2. Motion keys + binary wiring

`ui::MotionKey` (gui) enumerates what animates:

```rust
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub enum MotionKey { Menu, Sidebar, Main, HandleHover, Overlay }
```

- `App` holds one `motion: Animator<MotionKey>` (replacing the four fields) plus the
  fade-out bookkeeping in §4.
- A single helper computes targets from state, used by both the tick and the subscription so
  they never disagree:
  ```rust
  fn apply_motion_targets(app: &mut App) {
      app.motion.to(MotionKey::Menu,     if app.core.help_menu_open  {1.0} else {0.0}, step(MENU_FADE));
      app.motion.to(MotionKey::Sidebar,  if app.core.sidebar_hidden  {0.0} else {1.0}, step(SIDEBAR_SLIDE));
      app.motion.to(MotionKey::Main,     1.0,                                          step(MAIN_FADE));
      app.motion.to(MotionKey::HandleHover, if app.handle_hovered    {1.0} else {0.0}, step(HANDLE_HOVER));
      // MotionKey::Overlay target is driven by the lifecycle in §4, not here.
  }
  ```
- `Message::AnimationTick` becomes: `apply_motion_targets(app); app.motion.tick();` then the
  fade-out cleanup in §4. (Four `approach()` lines → one `tick()`.)
- `subscription` becomes: `apply_motion_targets(app)` (on a scratch copy or via a shared pure
  `targets()` fn) then `if app.motion.animating() { …ANIM_TICK… }`. (Four-way OR → one call.)
- `ui::view(state, terminal, &app.motion)` reads `app.motion.get(MotionKey::Menu)` etc. The
  `Anim` struct is deleted.

> Note: `apply_motion_targets` needs `&mut`. To keep `subscription` (which takes `&App`)
> honest, factor the target computation into a pure `fn motion_targets(app: &App) -> [(MotionKey,
> f32, f32); N]` that both the mutating apply and the subscription's `animating`-preview use.
> Implementation detail; resolved in the plan.

### 3. Reusable modal transition — `material::Modal` (Principle VIII builder)

New `src/ui/material/modal.rs`, exported from `material`. Mirrors the existing `MenuOverlay`
builder exactly:

```rust
Modal::new(base, dialog, roles).progress(p).into()   // returns `base` as-is at p <= 0.001
```

Rendering (reveal-`base` fade, built from primitives since there is no general opacity):

- **Scrim:** a full-window `fill_quad` whose alpha = `p * SCRIM_ALPHA`. As `p → 0` the scrim
  clears and `base` shows through — this is what makes fade-out actually reveal the app.
- **Dialog:** `center(dialog)` wrapped in the existing `material::fade` widget at progress `p`
  (contents fade toward the dialog surface color), then `opaque(...)` for input capture while
  shown so `base` is inert under an open dialog.
- **Optional polish (stretch, validated visually):** a small new `material::scale(content, p)`
  transform primitive — scales the dialog 0.96→1.0 about its center via `with_transformation`,
  analogous to how `slide` uses `translate`. Adds a Material-style "lift". Easy to drop if it
  doesn't earn its keep; the ~300ms scrim fade alone satisfies the requirement.

The five overlay render fns (`about`, `settings_form`, `rename`, `worktree_form`,
`project_selector`) refactor to build **only their dialog body** and hand it to `Modal` with a
`progress` argument. Their existing scrim/`opaque`/`center`/`stack` plumbing moves into
`Modal` (shared once). `about::modal` keeps its no-draft signature; the four draft-backed fns
gain a `progress: f32` parameter.

### 4. Overlay fade-out via GUI snapshot (core untouched)

The binary keeps the just-closed overlay drawable while it fades:

```rust
enum ClosingOverlay {
    About,
    Selector(Selector),
    Rename(RenameDraft),
    Worktree(WorktreeForm, Option<String>),   // draft + worktree_error
    Settings(SettingsDraft),
}
// App gains: dismissing: Option<ClosingOverlay>
```

All variants clone data already `Clone` in the core `State`.

Lifecycle, entirely in the `update` wrapper (the same place `main_key` changes are already
detected) — capture happens **before** `update_inner` may clear the draft:

1. Before `update_inner`: read `core.overlay` and clone the matching draft into a candidate
   `ClosingOverlay` (`None` if no overlay open).
2. After `update_inner`:
   - **Closed** (`before == Some(X)` && `core.overlay == None`): `app.dismissing =
     Some(snapshot)`, `app.motion.to(Overlay, 0.0, step(OVERLAY_EXIT))`.
   - **Opened / switched** (`core.overlay != None` && changed): `app.dismissing = None`,
     `app.motion.set(Overlay, 0.0)` then `app.motion.to(Overlay, 1.0, step(OVERLAY_ENTER))`.
   - **Unchanged**: leave as-is (a re-render mid-fade just continues).
3. `AnimationTick` (after `tick()`): if `motion.get(Overlay) <= 0.001`, `app.dismissing =
   None` (snapshot released once invisible).

`view()` selects the source and progress:

- `core.overlay != None` → render the **live** overlay from core state, `progress =
  motion.get(Overlay)` (fade-in).
- else `dismissing = Some(snap)` → render the **snapshot** via the same modal fns, same
  progress (fade-out).
- else → `base`.

Both paths call the identical `Modal`-based render fns, so there is no duplicate view logic —
only a match over which data source to read. The core reducer, dismissal rules, validation,
and persistence are unchanged.

### 5. Timing model

All durations are named `Duration` constants in the binary; a helper converts to the
per-tick step the `Animator` consumes:

```rust
const ANIM_TICK: Duration = Duration::from_millis(16);
fn step(d: Duration) -> f32 { (ANIM_TICK.as_secs_f32() / d.as_secs_f32()).clamp(0.0, 1.0) }
```

| Animation | Duration | ≈ step | Notes |
|-----------|----------|--------|-------|
| Overlay enter | 300ms | 0.053 | Clearly perceptible (was ~90ms) |
| Overlay exit  | 240ms | 0.067 | ~0.8× enter (Material convention) |
| Menu fade     | ~90ms (unchanged, `MENU_FADE`) | 0.18 | Preserves current feel |
| Main fade     | ~90ms (unchanged) | 0.18 | Preserves current feel |
| Sidebar slide | ~114ms (unchanged) | 0.14 | Preserves current feel |
| Handle hover  | ~800ms (unchanged) | 0.02 | Preserves current feel |

Existing animations keep their current speeds (re-expressed as durations for legibility).
Only the overlay is new and deliberately slower.

## Data flow (overlay open → close)

```
open msg   → core sets overlay=Some(X)  → update wrapper: dismissing=None,
                                           motion.set(Overlay,0), motion.to(Overlay,1,enter)
AnimationTick* → motion.tick() ramps Overlay 0→1 → view renders live overlay fading in
close msg  → wrapper snapshots X (pre-clear) → core clears overlay+draft
             → wrapper: dismissing=Some(snap), motion.to(Overlay,0,exit)
AnimationTick* → motion.tick() ramps Overlay 1→0 → view renders snapshot fading out
             → when Overlay<=0.001: dismissing=None → view renders base only
```

## Error handling / edge cases

- **Open while dismissing:** step 2 "opened" branch drops `dismissing` and restarts the
  fade-in — no stale snapshot, no double-draw.
- **Snapshot vs live mismatch:** the snapshot is frozen and non-interactive; that is correct
  for a leaving dialog.
- **`get` on an unset key:** returns 0.0 (safe default) — a never-opened overlay reads as
  fully faded, i.e., absent.
- **Subscription accuracy:** targets are computed identically for tick and subscription, so
  the clock always runs exactly while something moves and stops when settled (no busy loop).

## Testing strategy

- **`src/motion.rs` unit tests (`cargo test --no-default-features`):**
  - `to` + repeated `tick` converges to target and stops exactly there (no overshoot).
  - Per-track `speed` respected; a slower track lags a faster one.
  - `animating()` is true mid-flight and false once every track settles.
  - Independent keys advance independently.
  - `get` on an unset key returns 0.0.
- **Behavior preservation:** menu/sidebar/main/hover keep current durations and targets;
  verified by inspection + the app running (`/verify` skill) since these are gui-side.
- **Overlay fade:** validated by running the app and observing each of the five overlays fade
  in on open and fade out on cancel and on successful submit; scrim reveals `base` on exit.

## Files

- **New:** `src/motion.rs`; `src/ui/material/modal.rs`; (optional) `material::scale` in
  `src/ui/material/animation.rs`.
- **Edit:** `src/lib.rs` (add `motion`); `src/main.rs` (Animator, `ClosingOverlay`,
  lifecycle, durations, view/subscription rewire); `src/ui/mod.rs` (`MotionKey`, delete
  `Anim`, `view` signature); `src/ui/material/mod.rs` (export `Modal`, maybe `scale`); the
  five overlay render fns (`about`, `settings_form`, `rename`, `worktree_form`,
  `project_selector`) to use `Modal` + accept `progress`.

## Risks / open questions

- **Scale primitive complexity:** a scale-about-center advanced widget needs correct
  layout + inverse-transform event mapping (like `slide`). Mitigation: it is optional; ship
  the scrim+fade first, add scale only if the visual warrants it.
- **Dialog fade fidelity:** without true opacity, the dialog's own card does not alpha-fade;
  the scrim carries the fade and the card fades its contents / lifts. Accept and tune the
  backdrop color (card surface vs window bg) during visual verification.
