# Contracts: Animation Library Public Interfaces

Two layers. Layer 1 (the core) is the extractable, framework-agnostic contract that consumers
outside this project would use (FR-015/017). Layer 2 (render helpers) is the iced-specific
reuse layer for iced apps. Signatures are the contract; bodies are implementation.

## Layer 1 — Framework-agnostic core (`micold_ai_ide::motion`)

Extractable unit. Depends only on `std`. No iced, no app types.

```rust
/// A single animatable scalar moving toward a target at a fixed per-tick step.
pub struct Track { pub value: f32, pub target: f32, pub speed: f32 }

/// A keyed collection of independent animated scalars.
pub struct Animator<K> { /* private: HashMap<K, Track> */ }

impl<K: Copy + Eq + std::hash::Hash> Animator<K> {
    /// An empty animator.
    pub fn new() -> Self;

    /// Snap `key` to `value`, settled (value == target). No animation. Use to initialize.
    pub fn set(&mut self, key: K, value: f32);

    /// Set `key`'s target and per-tick step. Existing key: animates from its current value.
    /// New key: created settled at `target` (no surprise animation on first use).
    pub fn to(&mut self, key: K, target: f32, speed: f32);

    /// Current value of `key`, or 0.0 if it was never set.
    pub fn get(&self, key: K) -> f32;

    /// Advance every track toward its target by its speed, clamped (never overshoots).
    pub fn tick(&mut self);

    /// True iff any track has not yet reached its target.
    pub fn animating(&self) -> bool;
}

impl<K: Copy + Eq + std::hash::Hash> Default for Animator<K> { /* = new() */ }
```

**Contract guarantees** (each is a unit test — Principle I / FR-010):
- C1 `to` then repeated `tick` converges `value` to `target` and then stops exactly there
  (no overshoot, no oscillation).
- C2 A larger `speed` converges in fewer `tick`s than a smaller one (per-track rate honored).
- C3 `animating()` is `true` while any track is mid-flight and `false` once all are settled.
- C4 Distinct keys animate independently (advancing one does not move another).
- C5 `get` on an absent key returns `0.0`.
- C6 `to` on a brand-new key creates it settled at `target` (`get == target`, `animating`
  stays `false` if nothing else moves).
- C7 The module references no iced type and no application type (compiles under
  `--no-default-features`) — the extractability contract (FR-015 / SC-008).

## Layer 2 — iced render helpers (`crate::ui::material`)

Reusable by any iced app; consume a `progress: f32` produced by Layer 1. Builder API
terminating in `.into()` (Constitution Principle VIII).

### Existing (unchanged signatures)

```rust
pub fn fade<'a, M: 'a>(content: impl Into<Element<'a, M>>, progress: f32, backdrop: Color) -> Element<'a, M>;
pub fn slide<'a, M: 'a>(content: impl Into<Element<'a, M>>, progress: f32) -> Element<'a, M>;
```

### New — `scale` transform primitive

```rust
/// Scale `content` about its center by `progress` mapped onto [MIN_SCALE, 1.0].
/// progress 1.0 = full size; progress 0.0 = MIN_SCALE. A passthrough widget
/// (layout/events/overlay delegated to the child), scaling via renderer transformation.
pub fn scale<'a, M: 'a>(content: impl Into<Element<'a, M>>, progress: f32) -> Element<'a, M>;
```

### New — `Modal` reusable overlay transition (builder)

```rust
/// Stacks a modal `dialog` over `base` with a fade+lift transition driven by `progress`
/// (1.0 = fully shown, 0.0 = fully hidden). Renders `base` as-is at progress <= 0.001.
/// Composes: base + animated scrim (alpha = progress * SCRIM_ALPHA, reveals base as it
/// clears) + centered dialog wrapped in `scale`; input captured via `opaque` while shown.
pub struct Modal<'a, M> { /* base, dialog, roles, progress */ }

impl<'a, M: Clone + 'a> Modal<'a, M> {
    pub fn new(base: impl Into<Element<'a, M>>, dialog: impl Into<Element<'a, M>>, roles: Roles) -> Self;
    pub fn progress(self, progress: f32) -> Self;   // default 1.0
}

impl<'a, M: Clone + 'a> From<Modal<'a, M>> for Element<'a, M> { /* ... */ }
```

**Contract**: each of the five overlay render functions builds only its dialog body and returns
`Modal::new(base, dialog, roles).progress(p).into()`. No overlay implements its own
scrim/stack/opaque wiring (FR-011).

## Consumer wiring contract (`src/main.rs`, `src/ui/`)

- `ui::view(state, terminal, motion: &Animator<MotionKey>) -> Element<Message>` reads
  `motion.get(MotionKey::…)`; the `Anim` struct is removed.
- One `apply_motion_targets(app)` computes every key's target+speed from state; called before
  `motion.tick()` in `AnimationTick` and (via a shared pure `motion_targets(&App)` helper) by
  `subscription` so the clock runs exactly while `motion.animating()`.
- Overlay open/close orchestration follows the lifecycle in data-model.md; drafts are snapshot
  into `ClosingOverlay` **before** the reducer clears them.
- Behavioral invariant (FR-012): the core reducer, `Overlay` enum, drafts, and persistence are
  unchanged; removing all motion code would restore today's instant-overlay behavior with no
  other difference.
