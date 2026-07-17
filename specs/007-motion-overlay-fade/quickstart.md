# Quickstart / Validation Guide: Motion Library & Overlay Fade

Proves the feature end-to-end. See [contracts/animation-api.md](./contracts/animation-api.md)
and [data-model.md](./data-model.md) for the interfaces referenced here.

## Prerequisites

- Rust stable toolchain via `mise` (per constitution).
- Build deps for the GUI (iced) present.

## 1. Core library — extractable, render-free (FR-010, FR-015, SC-007, SC-008)

Run the framework-agnostic core's tests without compiling the GUI:

```bash
cargo test --no-default-features motion
```

Expected: the `motion` unit tests pass, covering contracts C1–C7:
- converges to target and stops (C1); faster speed converges sooner (C2);
- `animating()` flips true→false as tracks settle (C3);
- independent keys (C4); `get` default 0.0 (C5); new-key-settled-at-target (C6);
- the module compiles with **no** default features — i.e. no iced and no app-specific code
  in the core, demonstrating extractability (C7 / SC-008).

Full suite (all existing tests still pass — SC-007):

```bash
cargo test
```

## 2. Build the app

```bash
cargo build
cargo run
```

## 3. Overlay fade in/out — all five overlays (US1: FR-001..006, SC-001..003)

For each overlay — About, project selector, rename project, add worktree, Settings — verify:

| Action | Expected |
|--------|----------|
| Open the overlay | It **fades in** (scrim dims in, dialog lifts to full size) over ~0.25 s; not an instant pop (SC-002/SC-003). |
| Close via **Cancel** | It **fades out** over ~0.2 s and the app content behind it progressively reappears (FR-002/FR-003). |
| Close via **Esc** | Same fade-out as Cancel (FR-002). |
| Submit **successfully** (rename / add worktree / Settings) | Overlay **fades out** (same exit), does not blink away (FR-002). |
| Submit with **invalid** input (e.g. Settings scrollback = abc) | Overlay **stays open** with its error; no exit animation (edge case). |

Timing check (SC-002): each open reads as ~0.25 s and each close ~0.2 s — clearly perceptible,
within the 0.15–0.5 s band.

## 4. Existing animations preserved (US2: FR-009, SC-005)

Confirm no regression in the four migrated animations:

| Animation | Check |
|-----------|-------|
| Overflow (help) menu | Toggling the toolbar overflow menu still fades in/out as before. |
| Sidebar | Collapsing/expanding the worktree sidebar still slides as before. |
| Main view | Switching main content (project ⇄ session terminal) still fades as before. |
| Resize-handle hover | Hovering the sidebar resize handle still ramps its highlight (~0.8 s) as before. |

## 5. Reveal-beneath + edge cases (FR-003, edge cases)

- During any overlay fade-out, watch the region **around** the dialog: the app beneath
  (sidebar / terminal / project surface) becomes progressively visible as the scrim clears.
- **Reopen during exit**: open an overlay, close it, and immediately open another before the
  fade finishes — the new overlay fades in cleanly; no leftover/partial dialog remains.
- **Rapid toggle**: quickly open/close the same overlay repeatedly — no dialog gets stuck
  partially visible.
- **Quit mid-animation**: close the window while an overlay is mid-fade — the app exits
  cleanly (no panic).

## 6. Idle cost unchanged (FR-014, SC-006)

With the app open and **nothing** animating (no overlay, menu closed, not hovering the
handle), the app is idle — the animation clock subscription is not running. Verify no
continuous redraw/CPU churn at rest (e.g. observe CPU is ~0 when untouched), matching today's
behavior.

## 7. Adding a new animated element is single-site (SC-004)

Sanity demonstration of FR-008: to animate a new element a developer picks a `MotionKey`
variant, calls `motion.to(key, target, step(duration))` where the target is set, and reads
`motion.get(key)` in `view` — no new `*_anim` field, no new `tick` line, no new subscription
branch. Contrast with the pre-feature pattern that required edits in four places.
