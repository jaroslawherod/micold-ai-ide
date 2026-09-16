# Implementation Plan: The settings rail slides when it collapses and expands

**Branch**: `fix/settings-side-bar-should-be-animated` | **Date**: 2026-09-14 | **Spec**: [spec.md](./spec.md)

**Input**: Feature specification from `/specs/030-settings-rail-slide/spec.md`

## Summary

`SectionList` is stateless. Its `Element` conversion reads `collapsed` and returns a container of
one of two fixed widths, so a rebuilt view is a different layout and iced lays it out at once. That
is the snap ([BUG-004](../027-sandboxed-daemon-runtime/bugs/BUG-004.md)).

The fix gives the rail a slide it owns in widget-tree state, split over two small private widgets
in `ui/material/section_list.rs`:

1. **`Rail`** — the width. It holds one `Progress` on the *sidebar slide* row of 018's motion
   contract (`medium_4`, `emphasized`), advances it with `on_layout_frame`, and lays out the rail's
   single rendering at the width that progress gives.
2. **`RowSlide`** — each row and the collapse control. It holds the row's rest forms (labelled,
   labelled with its badge mark on the icon, icon-only), each laid out at its own rest width. It
   reads how far the rail is through its width change from the width it is given, draws exactly one
   form clipped to its own bounds, and offsets it sideways so the icon sits on the straight line
   between its two rest positions. Events, operations and overlays reach only the drawn form, and
   pointer input only while the cursor is inside the row; when the drawn form changes, keyboard
   focus moves with it.

Beside that: `NavigationDrawer`'s track gains `.easing(EMPHASIZED)` so the sidebar panel moves on the
curve its contract row names (FR-003), with the width floor and earlier strip swap that curve needs
(D20, D21), the do-nothing drawer wrapper in `settings_view.rs` goes, and
`docs/user-guide/settings.md` documents the collapse control and its slide (FR-012).

The implementation set aside in WIP commit `14d9c0e4` (D6) is the starting point: the `Rail` widget,
the `Progress` wiring, the event/operation routing, the clip, the wrapper removal and the test
harness in `tests/settings_rail_motion.rs` carry over. What changes is where the form choice lives:
the WIP swapped two whole renderings at the end of the slide, which made icons jump by up to 13dp at
the swap (FR-014) and clipped a badge chip off while its icon was still untinted (FR-015). Moving
the choice into each row is what makes both requirements satisfiable. The WIP's `FocusNth` handoff
is replaced too, since it showed a focus ring after a pointer press (research R5). See
[research.md](./research.md) R1–R5.

No wire-protocol, storage, core or daemon change. The builder API of `SectionList` is unchanged.

## Technical Context

**Language/Version**: Rust, pinned to `stable` by `rust-toolchain.toml`.

**Primary Dependencies**: `iced` (workspace version) through its `advanced` widget API: `Widget`,
`layout::Node`, `Tree`, `Operation`/`Focusable`, `Renderer::with_layer`. The motion primitive
`ui::cdk::motion::Progress` (feature 017) and the tokens `micold_core::tokens::motion::{duration::MEDIUM_4, EMPHASIZED}` (feature 018).

**Storage**: N/A. The rail's collapsed flag stays view state (027 FR-026d); the slide lives in the
widget tree and is never persisted.

**Testing**: `cargo test --workspace` via `mise run test`, and `mise run gate` before pushing.
Integration tests in `crates/micold-client/tests/` mount the real settings surface headlessly and
pump redraw/layout frames with distinct instants (harness from WIP `14d9c0e4`). Pure decision
functions (fraction, form choice, icon offset) get unit tests in `section_list.rs`'s test module.
Draw-only behaviour (the clip) is covered by the recorded visual pass in
[quickstart.md](./quickstart.md) §B.

**Target Platform**: Desktop — Linux, macOS, Windows.

**Project Type**: Desktop application; Cargo workspace (`micold-core`, `micold-client`, `micold-daemon`).

**Performance Goals**: A slide requests frames only while its progress moves (FR-011) and settles
within `medium_4` plus twice the longest frame gap (SC-002). Per frame, the rail lays out at most
`2 · rows + badged rows + 2` button forms (Settings: 4 destinations with 1 badge and the control,
11), against one per row today. At rest, a settled rail requests nothing.

**Constraints**: No new motion assignment: the rail uses the *sidebar slide* row as it stands
(spec Assumptions; 018 `contracts/design-tokens.md` §6.3 — this plan records that use and 018's
contract is not edited). No per-platform branch. Rest layouts identical to today's, rectangle for
rectangle (SC-005). Source-scanning gates stay green: `animated_layout_relayouts.rs` (a widget whose
`layout` reads a track advances it with `on_layout_frame`), `motion_tokens.rs` (durations are named
tokens), `one_overlay_implementation.rs` (the new widgets forward overlays and construct none).

**Scale/Scope**: One component (`SectionList`) and its two screens — Settings and the showcase's
rail example — plus `NavigationDrawer`'s curve, its width floor and its earlier strip swap (D20, D21).
The worktree sidebar's strip itself is out of scope.

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-checked after Phase 1 design — see [Post-Design Re-check](#post-design-re-check).*

- [x] **I. Test-First (NON-NEGOTIABLE)**: PASS. Every decision the slide makes — the fraction a
  width stands for, which form a row draws, how far it is offset, when focus moves, where events
  go — is either a pure function with unit tests or observable from `tests/` by mounting the
  settings surface and reading layout nodes and operations frame by frame. The GUI exception is
  invoked only for the draw-time clip (`with_layer`), which has no branching beyond "narrower than
  rest", and is backed by quickstart §B. Red-Green-Refactor per task through the `tdd` extension.
- [x] **II. Multi-Session Support**: PASS. No session state. The rail's collapsed flag belongs to
  the application instance; the daemon and other attached clients see nothing (spec Edge Cases).
- [x] **III. Worktree Integration**: PASS (not engaged). No file or VCS operation.
- [x] **IV. Local-First Storage (NON-NEGOTIABLE)**: PASS. Nothing stored, nothing sent.
- [x] **V. Rust + iced Stack**: PASS. Rust and iced only. "Which form a row draws" is an enum
  (`RowForm`), not a pair of booleans, so "labelled and icon-only at once" is unrepresentable.
- [x] **VI. Cross-Platform Parity**: PASS. No `cfg` arm; the slide is measured in elapsed time and
  layout units, so refresh rate and display scale do not change it. CI covers all three platforms.
- [x] **VII. Documentation First-Class**: PASS. `docs/user-guide/settings.md` gains the collapse
  control and its slide (FR-012) in the same change. The component's module docs are updated to say
  it owns its slide.
- [x] **VIII. Reusable UI Component Foundation**: PASS. The slide lands in the shared `SectionList`
  component, so every screen built on it gets it (Settings and the showcase example). `Rail` and
  `RowSlide` are private building blocks of that component, not feature-local widgets. The builder
  API (`SectionList::new(..).selected(..).collapsed(..).toggle(..).into()`) does not change. The
  sidebar's curve is corrected inside the shared `NavigationDrawer`, not overridden at a call site.

## Project Structure

### Documentation (this feature)

```text
specs/030-settings-rail-slide/
├── spec.md
├── plan.md              # This file
├── research.md          # Phase 0 output
├── data-model.md        # Phase 1 output
├── quickstart.md        # Phase 1 output
├── contracts/
│   └── rail-slide.md    # UI contract: widths, forms, icon path, focus, input, curve
├── checklists/
│   └── requirements.md
├── autopilot.md         # autopilot ledger
└── tasks.md             # /speckit-tasks output — NOT created here
```

### Source Code (repository root)

```text
crates/micold-client/
├── src/ui/material/
│   ├── section_list.rs        # Rail + RowSlide widgets, TakeFocus/GiveFocus, pure slide functions, unit tests,
│   │                          #   and the in-crate mounts (rail vs drawer, no-icon row): `material` is pub(crate)
│   ├── keyboard_focus.rs      # operate also offers `Focus` via operation.custom (focus handoff keeps `visible`)
│   └── navigation_drawer.rs   # .easing(EMPHASIZED), width floor, swap at the floor; `parked` pub(super)
├── src/ui/settings_view.rs    # drop the pinned-open NavigationDrawer wrapper
├── tests/
│   ├── settings_rail_motion.rs          # NEW: frame-by-frame slide tests (FR-001–FR-015, SC-001–SC-007)
│   ├── fixtures/layout_snapshot.txt     # regenerated: parked forms added, visible rects unchanged
│   ├── gates/rail_icons_align.rs        # gains an inside-the-rail clause; relies on icons_only being the last child (research R10)
│   └── support/
│       ├── covered_states.rs            # `settings.rail` anchor paths follow the new tree
│       └── layout.rs                    # `view_of` made pub for the motion harness
docs/user-guide/settings.md    # FR-012
```

**Structure Decision**: Everything lands in the existing client crate at the paths above. No new
module: the two widgets are private to `section_list.rs` because nothing else composes them, and
`parked` is shared with the drawer rather than copied.

## Design notes

### Motion assignment (018 §6.3)

The rail's slide is an instance of the *sidebar slide* row: trigger "a side panel changes width",
`medium_4`, `emphasized`. No row is added and 018's contract is not edited. `Rail` names both
tokens as constants beside the drawer's, the way the drawer names its duration.

### The sidebar curve (FR-003)

`NavigationDrawer::state` builds `Progress::new(..)` with no curve, which defaults to linear. It
gains `.easing(EMPHASIZED.x1, EMPHASIZED.y1, EMPHASIZED.x2, EMPHASIZED.y2)`. No existing test
asserts the drawer's timing shape (checked: `navigation_drawer.rs` tests assert which child shows,
`animated_layout_relayouts.rs` asserts relayout requests). `ui/mod.rs`, which composes the sidebar,
is untouched. SC-002's "same fraction at the same elapsed time" is asserted by driving a drawer and a
rail with the same frame instants.

The curve's slow tail makes a second change necessary (M1 review round 1, D20). `layout` sized the
revealed panel at `full · p`, which below `p ≈ 0.087` (a 300 px panel) is narrower than the 32 px
strip, so on `emphasized` the content beside the drawer spent ~150 ms left of the strip's edge and
jumped back at the swap. The revealed width is floored at the rail's width less the handle's,
clamped to the panel's (U23). With no handle and a zero-width rail, as in T019's mount and
`settings_view.rs`, the floor is 0 and nothing changes.

The floor alone left the other half of the tail on screen (M1 review round 3, D21): closing, the
width reached the floor at `p ≈ 0.087` (~226 ms) but the rail swapped in at `CLOSED` (~378 ms), so a
still 26 px sliver of panel, the edge of its *Hide sidebar* button, sat there for ~150 ms (17–57 ms
on linear). `layout` now swaps to the rail once a closing panel's `full · p` is no wider than the
floor, or at `CLOSED`, and records that decision in the drawer's state; `update`, `draw`,
`mouse_interaction` and `overlay` read it, so they always agree with the layout they are handed
(U24). Opening never swaps early, and a zero floor keeps the swap at `CLOSED`.

### Risks

- **Layout snapshot churn.** Every row gains a `RowSlide` node and two parked forms, so
  `layout_snapshot.txt` changes for both settings states. The review check is not "the fixture was
  regenerated" but quickstart §A.3: the set of visible rectangles per state is identical to
  `origin/main`'s.
- **The WIP's local gate failure** in `micold-daemon --test exclusivity` is unrelated code; it is
  rechecked on fresh main before the first milestone PR (ledger follow-up).
- **State in parked forms.** A form that parks keeps its tree state (a button's pressed flag, a
  ripple in flight, its focus indicator's `visible` flag), and a button's status is computed only on
  `RedrawRequested`. Parked forms therefore receive redraws and the release, finger-lifted, finger-lost and cursor-left events with the cursor unavailable,
  through a local `Shell` that drops messages and captures but passes redraw requests and
  invalidation on, and focus moves with its `visible` flag, so neither a frozen press, a frozen
  ripple, a disabled look nor a pointer-focus ring appears (research R5). A `Sliding` row keeps three child slots so a badge toggling mid-slide does not
  shift trees between forms (research R3).
- **A destination with no icon** has one form laid out at the current width, so its label may wrap
  mid-slide and its height passes between its two rest heights (research R3a). Plan review round 1
  found the spec as first written asked such a row to keep its height, be cut off and show its badge
  at once, which a badged row with no icon cannot do; FR-013, SC-007 and the edge case were amended
  in this PR (ledger D12). No shipped rail has such a row.

## Post-Design Re-check

Re-evaluated after research, data model, contract and quickstart were written: all eight principles
still PASS. The design added two private widgets and one enum inside an existing shared component,
no dependency, no storage, and no platform branch. Complexity Tracking stays empty.

## Complexity Tracking

No constitution violations.
