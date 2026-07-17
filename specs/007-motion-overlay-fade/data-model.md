# Phase 1 Data Model: Generic Motion Library & Overlay Fade In/Out

This feature adds ephemeral, in-memory UI state only. Nothing here is persisted; the persisted
`State` (Clone/Eq) is unchanged (FR-012).

## Framework-agnostic core (`src/motion.rs`) — extractable

### `Track`

A single animatable scalar.

| Field  | Type  | Meaning                                              |
|--------|-------|-----------------------------------------------------|
| value  | `f32` | Current progress, conventionally in `[0.0, 1.0]`.   |
| target | `f32` | Value it is moving toward.                           |
| speed  | `f32` | Max change applied per `tick()` (per-frame step).   |

Validation / rules:
- `tick()` moves `value` toward `target` by at most `speed`, never overshooting
  (the existing `approach` semantics).
- A track is *settled* when `value == target`.

### `Animator<K>`

A keyed collection of `Track`s. `K: Copy + Eq + Hash`. Zero dependency on iced or any app
type — this is the extractable unit (FR-015, SC-008).

| Field  | Type              | Meaning                          |
|--------|-------------------|----------------------------------|
| tracks | `HashMap<K, Track>` | One track per animated element. |

Operations (public API — see contracts/animation-api.md):
- `new() -> Self`
- `set(&mut self, key: K, value: f32)` — insert/snap a settled track at `value`
  (`value == target`), no animation.
- `to(&mut self, key: K, target: f32, speed: f32)` — set a track's `target` and `speed`;
  if the key is new, it is created **settled at `target`** (so an unset element never
  surprise-animates on first sight — deliberate animation uses `set` then `to`).
- `get(&self, key: K) -> f32` — current `value`, or `0.0` if the key is absent.
- `tick(&mut self)` — advance every track toward its target by its speed.
- `animating(&self) -> bool` — true iff any track is unsettled (drives the clock gate).

State transitions of one track:

```text
absent --set(v)--> settled(value=v,target=v)
absent --to(t,s)--> settled(value=t,target=t)          # no surprise animation
settled --to(t,s)--> moving(value→t at s)               # explicit animation
moving  --tick()*--> settled(value==target)             # converges, then stops
```

## Consumer-side GUI types (`src/ui/`, `src/main.rs`) — app-specific

### `MotionKey` (in `src/ui/mod.rs`)

The application's set of animated elements. This enum is app-specific and lives in the
consumer, never in the extractable core (FR-016).

```text
enum MotionKey { Menu, Sidebar, Main, HandleHover, Overlay }   // Copy + Eq + Hash
```

- `Menu` — overflow-menu fade (was `menu_anim`).
- `Sidebar` — sidebar slide (was `sidebar_anim`).
- `Main` — main-view fade (was `main_anim`).
- `HandleHover` — resize-handle hover highlight (was `handle_hover_anim`).
- `Overlay` — the currently open/closing modal overlay's fade (new).

### `ClosingOverlay` (in `src/main.rs`)

A snapshot of the just-closed overlay, kept alive by the binary so it can be rendered while it
fades out. Each variant carries a clone of exactly the data that overlay's render function
needs. All carried types are already `Clone` in the core `State`.

```text
enum ClosingOverlay {
    About,
    Selector(Selector),
    Rename(RenameDraft),
    Worktree(WorktreeForm, Option<String>),   // form draft + worktree_error
    Settings(SettingsDraft),
}
```

Held on `App` as `dismissing: Option<ClosingOverlay>`.

### `App` motion fields (in `src/main.rs`)

Replaces the four removed `*_anim: f32` fields and the removed `ui::Anim` struct:

| Field      | Type                    | Meaning                                                |
|------------|-------------------------|--------------------------------------------------------|
| motion     | `Animator<MotionKey>`   | The single shared driver for all animations.           |
| dismissing | `Option<ClosingOverlay>`| The overlay currently fading out, if any.              |

### Timing constants (in `src/main.rs`)

Durations (legible, tunable — FR-013), converted to per-tick step via
`step(d) = ANIM_TICK.as_secs_f32() / d.as_secs_f32()`:

| Constant        | Duration | Element                        |
|-----------------|----------|--------------------------------|
| OVERLAY_ENTER   | 250 ms   | `MotionKey::Overlay` fade-in   |
| OVERLAY_EXIT    | 200 ms   | `MotionKey::Overlay` fade-out  |
| MENU_FADE       | ~90 ms   | `MotionKey::Menu`              |
| MAIN_FADE       | ~90 ms   | `MotionKey::Main`              |
| SIDEBAR_SLIDE   | ~114 ms  | `MotionKey::Sidebar`           |
| HANDLE_HOVER    | ~800 ms  | `MotionKey::HandleHover`       |

## Overlay motion lifecycle (consumer orchestration)

```text
overlay opens (None→Some(X)):  dismissing = None;
                               motion.set(Overlay, 0.0); motion.to(Overlay, 1.0, step(ENTER))
tick* :                        value 0→1  → view renders live overlay fading in
overlay closes (Some(X)→None): dismissing = snapshot(X, drafts captured pre-reducer);
                               motion.to(Overlay, 0.0, step(EXIT))
tick* :                        value 1→0  → view renders snapshot fading out
value ≤ 0.001 :                dismissing = None → view renders base only
```
