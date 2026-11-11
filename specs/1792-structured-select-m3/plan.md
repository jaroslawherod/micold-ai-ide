# Implementation Plan: Structured view Material 3 select and expand animation

**Branch**: `feat/1792_feat-structured-view-material-3-select-input-and` | **Date**: 2026-11-11 | **Spec**: [spec.md](spec.md)

**Input**: Feature specification from `specs/1792-structured-select-m3/spec.md`

## Summary

The structured view's session options row (Model, Effort, Permission; `ui/composer.rs`
`options_row`) is built from `MenuSelect`, whose trigger is a text `Button` ("Label: Value" plus
chevron), not the filled-field Material 3 trigger the showcase's `Select` has. Two changes:

1. **Trigger look (US1, FR-001..004, FR-009)**: give `MenuSelect` the `Select` trigger anatomy
   (container shape, fill or outline, label, value, trailing arrow, hover/focus/pressed/disabled/open
   state layers) by extracting the trigger drawing both share, so one definition serves both
   controls. The list already uses `material::picker`, the same list `Select` floats (US2); a
   parity test pins it.
2. **Expand animation (US3, FR-005..008, FR-013)**: wrap the options row in the existing
   self-animating `material::expand` (`micold-material/src/animation.rs`), over a motion token
   duration and its standard easing; `MEDIUM_2`, zero duration when
   `settings.reduce_motion`, read in `composer::view`. The row is always built (no longer `if options_open`), so the wrapper
   keeps its track in widget-tree state; per-session independence follows from the track living in
   each composer's tree (see research R3).

No new tokens, no new settings, no new persisted state (`composer_options_open` is unchanged).

## Technical Context

**Language/Version**: Rust (workspace toolchain), iced 0.14

**Primary Dependencies**: `micold-client` (`ui/material`, `ui/composer.rs`), `micold-material` (`animation::expand`), `micold-core` (`tokens::motion`, `tokens::anatomy`, `settings.reduce_motion`)

**Storage**: N/A (existing `composer_options_open` setting only)

**Testing**: `cargo test` via `mise run`: client unit and geometry tests (`tests/gates`, `ui/material/*_anatomy.rs`, `picker_parity.rs`), core unit tests for any pure helper; quickstart Part B visual pass

**Target Platform**: Linux, macOS, Windows (desktop iced app)

**Project Type**: desktop-app (Rust workspace)

**Performance Goals**: animation runs at frame rate; settles within one frame of the token duration (SC-002)

**Constraints**: no hard-coded colour, shape, duration or easing (SC-003); offline

**Scale/Scope**: one row of three controls; one shared trigger extraction; one wrapper

## Constitution Check

- [x] **I. Test-First**: each task is test-first (see Test strategy); tasks unit orders the red tests ahead of code.
- [x] **II. Multi-Session**: animation state is widget-tree state of each session's composer view; `options_open` stays the existing app-wide toggle. Independence is tested (R3).
- [x] **III. Worktree**: no file or VCS operation involved. PASS (n/a).
- [x] **IV. Local-First**: no remote service, no new storage.
- [x] **V. Rust + iced**: Rust and iced only; the open/closed target stays the existing bool.
- [x] **VI. Cross-Platform**: tokens only, no platform branches; CI runs all three.
- [x] **VII. Documentation**: `docs/user-guide` page for the structured view/settings updated if it describes the options row; component catalog docs for the shared trigger; reduce-motion note in `docs/user-guide/settings.md`.
- [x] **VIII. Reusable UI**: the trigger is extracted into the shared library and consumed by `Select` and `MenuSelect` (builder API kept, `.into()`); the animation reuses `material::expand`, no feature-local widget.

Post-design re-check: unchanged, all PASS. Complexity Tracking: none.

## Project Structure

### Documentation (this feature)

```text
specs/1792-structured-select-m3/
├── plan.md
├── research.md
├── data-model.md
├── quickstart.md
├── contracts/ui-contract.md
└── tasks.md             # tasks unit
```

### Source Code

```text
crates/micold-client/src/ui/material/select.rs         # Select: trigger moves to the shared piece (behaviour unchanged)
crates/micold-client/src/ui/material/select_anatomy.rs # trigger anatomy reused (existing)
crates/micold-client/src/ui/material/menu_select.rs    # MenuSelect trigger adopts the shared trigger; list unchanged
crates/micold-client/src/ui/material/picker.rs         # list (existing, unchanged)
crates/micold-client/src/ui/composer.rs                # options_row wrapped in material::expand; instant from reduce_motion
crates/micold-material/src/animation.rs                # expand (existing, unchanged)
crates/micold-client/src/showcase/                     # MenuSelect and options-row entries show the new look
crates/micold-client/tests/gates/                      # geometry: trigger equals Select's per state
docs/user-guide/                                       # structured view and settings pages
```

**Structure Decision**: edit in place; one extraction inside `ui/material`.

## Requirement mapping

| FR | Where |
|---|---|
| FR-001, 002 | shared trigger (R1, contract §1) |
| FR-003, 009 | all three controls built by `MenuSelect` with the shared trigger (contract §1) |
| FR-004 | existing `picker::animated_menu`; parity test (R2) |
| FR-005, 006, 007 | `material::expand` with motion tokens (R3, contract §2) |
| FR-008 | per-composer widget-tree track (R3, data-model) |
| FR-010 | message wiring in `composer.rs` untouched; existing composer tests keep passing |
| FR-011 | empty list, missing value, long text, small window (contract §1, §2; geometry tests) |
| FR-012 | quickstart Part B |
| FR-013 | `instant` when `reduce_motion` (R4) |

## Test strategy

- **Core unit**: none expected; no new core logic. If a pure helper (duration choice) lands in core, it is tested there.
- **Client unit/geometry (`ui/material`, `tests/gates`)**: `MenuSelect` trigger vs `Select` trigger per state (rest, hover, focus, pressed, disabled, open) for FR-001/002/009; menu list parity (FR-004); long value truncated with arrow visible, empty list opens nothing, missing value renders (FR-011).
- **Client unit (`ui/composer.rs`, `drawer_flags_tests.rs` style)**: expand duration is the motion token and zero under reduce motion (FR-006, FR-013); two sessions' rows keep separate tracks (FR-008); effort/permission/model pick messages unchanged (FR-010); `tests/material_boundary.rs` and `one_overlay_implementation.rs` still pass.
- **Visual pass (quickstart Part B)**: look of every state, mid-animation reversal feel (FR-007), SC-001, SC-002, FR-012.
