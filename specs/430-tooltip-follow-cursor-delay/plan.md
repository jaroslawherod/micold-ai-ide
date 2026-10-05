# Implementation Plan: Tooltip follows the cursor and waits before showing

**Branch**: `fix/issue-430` | **Date**: 2026-10-05 | **Spec**: [spec.md](./spec.md)

**Input**: Feature specification from `/specs/430-tooltip-follow-cursor-delay/spec.md`

## Summary

The shared tooltip (`ui/cdk/tooltip.rs`, built by spec 029 BUG-001, extended by 038) gains two
opt-in capabilities and no caller adopts them: a pointer-following placement
(`Position::FollowCursor`) and a show delay (`Tooltip::show_delay`). Decisions, each in
[research.md](./research.md):

1. **The wait becomes one three-way mode, not two optional fields** ([R1](./research.md)).
   `Wait::{Hover, Delay(d), Rest(d)}` replaces today's `rest: Option<Duration>`. Setting both
   `after_rest` and `show_delay` is therefore unrepresentable (Constitution V): **the setter called
   last wins**, documented on both setters and held by a test. This is the plan decision clarify
   left open.
2. **The show delay is a pure rule in core** ([R2](./research.md)): `micold_core::tooltip::ShowTimer`
   beside `RestTimer`, same `Rest { open, wake_at }` result. It counts from entering the trigger,
   ignores movement, closes on leave, and (like the rest wait) stays closed after a press until the
   pointer has left.
3. **Pointer placement is a pure function** ([R3](./research.md)): `place_at_pointer` in
   `cdk/tooltip.rs` next to `place`, decided per axis (after the pointer; before it when the first
   side lacks room; the side with more room when neither fits), then kept inside the window. The
   pointer is a point, so "does not sit under it" is a visible-area test against that point.
4. **The pointer reaches the overlay through the widget's `State`** ([R4](./research.md)): `update`
   records `cursor.position_over(bounds)` on mouse events and redraws; `overlay()` (which gets no
   cursor) reads it, adds the overlay `translation`, and hands it to `Panel`. A moved pointer while
   open calls `shell.invalidate_layout()`; a still pointer changes nothing (FR-007, SC-005).
5. **No-delay hover stays the default and is untouched** ([R5](./research.md)): `Wait::Hover` is
   today's `cursor.is_over` branch. A pointer-following tooltip with `Wait::Hover` runs the delay
   rule with a zero delay, so it gets the press rule the spec's edge case asks for while the four
   fixed placements keep today's behaviour exactly (FR-006).
6. **Both are exposed on `material::Tooltip`** (`.show_delay(d)`, `TooltipPosition::FollowCursor`)
   and demonstrated in the component showcase. No screen adopts them.

## Technical Context

**Language/Version**: Rust, edition 2021, workspace MSRV (unchanged)

**Primary Dependencies**: `iced` 0.14 (`Shell::request_redraw_at` via `cdk::motion::wake_at`,
`Overlay`), `micold-core` module `tooltip`. **No new crate.**

**Storage**: none (Principle IV): all state is the widget's in-memory tree `State`.

**Testing**: `mise run test-core` for the new timer; `cargo test -p micold-client` for placement
unit tests (`cdk/tooltip.rs` `placement_tests`), the widget glue (`tests/support/tooltip.rs`
driver) and `tests/idle_requests_no_frames.rs`; `mise run gate` for the workspace; quickstart §B
(`visual-pass` skill) for how it looks in the showcase.

**Target Platform**: Linux, macOS, Windows desktop. No OS-specific code, no `cfg` arm.

**Project Type**: desktop application (three-crate Cargo workspace).

**Performance Goals**: an idle window repaints nothing (FR-007): the delay costs one timed wake at
its deadline, the pointer placement costs a relayout only on a pointer move while open.

**Constraints**: logical units only (Principle VI); existing tooltip tests pass unchanged (SC-004).

**Scale/Scope**: two source files of behaviour (`micold-core/src/tooltip.rs`,
`micold-client/src/ui/cdk/tooltip.rs`), one builder (`ui/material/mod.rs`), one showcase section.

## Constitution Check

- [x] **I. Test-First**: every rule is a pure function or a driven widget with a failing test first
  ([quickstart.md](./quickstart.md) lists the layers; tasks.md orders tests before code).
- [x] **II. Multi-Session**: PASS. The state is per widget instance in the iced tree; no session
  state, one window's tooltip never touches another's.
- [x] **III. Worktree Integration**: PASS, not applicable: no file or VCS operation.
- [x] **IV. Local-First**: PASS. Nothing stored, nothing remote.
- [x] **V. Rust + iced**: PASS. `Wait` makes "both delays set" unrepresentable.
- [x] **VI. Cross-Platform**: PASS. Logical units, the window's own bounds, no `cfg` arm; the
  cursor and the clock come through iced events on all three platforms.
- [x] **VII. Documentation**: PASS with a stated exception: no user-visible change ships (spec
  Assumptions), so the PR carries the `docs-not-needed` label; developer-facing rustdoc on the new
  builder methods and `docs/development` untouched. If a caller lands later, its feature updates
  the guide.
- [x] **VIII. Reusable UI Component Foundation**: PASS. Extends the shared tooltip through its
  chainable builder (`.show_delay(..)`, `.position(..)`), no feature-local fork.

## Project Structure

### Documentation (this feature)

```text
specs/430-tooltip-follow-cursor-delay/
├── plan.md
├── research.md
├── data-model.md
├── quickstart.md
├── contracts/tooltip-api.md
└── tasks.md
```

### Source Code (repository root)

```text
crates/micold-core/src/tooltip.rs               # + ShowTimer (pure delay rule)
crates/micold-core/tests/tooltip_show.rs         # new: ShowTimer table
crates/micold-client/src/ui/cdk/tooltip.rs       # Wait, Position::FollowCursor, show_delay, place_at_pointer
crates/micold-client/src/ui/material/mod.rs      # Tooltip::show_delay, wait field, FollowCursor passthrough
crates/micold-client/src/showcase/sections/floating.rs   # two new poses
crates/micold-client/tests/support/tooltip.rs    # new builder helper (existing helper unchanged)
crates/micold-client/tests/tooltip_show_glue.rs  # new: widget glue for delay + follow
crates/micold-client/tests/idle_requests_no_frames.rs    # + show-delay and follow idle cases
crates/micold-client/tests/material_builder_api.rs       # + show_delay builder step
```

**Structure Decision**: extend the two existing tooltip modules; no new module.

## Requirement map

| Requirement | Where |
|---|---|
| FR-001 | `Position::FollowCursor`, `place_at_pointer`, `State.pointer` (R3, R4) |
| FR-002 | `place_at_pointer` per-axis flip and clamp (R3) |
| FR-003 | `ShowTimer`, `Wait::Delay`, `show_delay` (R1, R2) |
| FR-004 | `Wait::Hover` unchanged branch (R5) |
| FR-005 | `place` unchanged for the four sides; `place_at_pointer` for the pointer; both tested at all four edges |
| FR-006 | `Wait::Hover`/`Wait::Rest` take today's code paths; existing tests untouched (SC-004) |
| FR-007 | wake only while `ShowTimer` waits; relayout only on a moved pointer (R4) |

## Test strategy

| Behaviour | Layer |
|---|---|
| Delay opens at D, never before, not restarted by movement, cancelled by leave, press holds closed | core unit, `tests/tooltip_show.rs` |
| Pointer placement: beside, offset, flips per axis, clamps, never under the pointer, corner, tiny window | client unit, `placement_tests` |
| Fixed placements at all four edges still clear the trigger (SC-003, SC-006) | client unit, `placement_tests` |
| Widget feeds the timer: events observed, wake at deadline, panel re-laid out on pointer move, not on still pointer | client driver, `tests/tooltip_show_glue.rs` |
| No wake and no frame request without a waiting or open tooltip | `tests/idle_requests_no_frames.rs` |
| Follow with no delay: press closes until leave; `subject()` change restarts the delay; a trigger moved from under a still cursor is seen on redraw | client driver, `tests/tooltip_show_glue.rs` |
| No pointer, zero-size trigger or unknown window size: nothing opens (guard in `overlay()`, new) | client unit/driver |
| Both setters: last wins, each way | client unit at `cdk` and at `material` (`Wait` is `pub(crate)`) |
| `.show_delay` is a builder step | `tests/material_builder_api.rs` |
| Existing tooltips unchanged | existing tests, run unchanged |
| How it looks, real pointer, timing | quickstart §B |

## Complexity Tracking

No violations.
