# Implementation Plan: Worktree tooltip shows the full name and its details

**Branch**: `feat/tooltip-of-worktree-should-show-full-name` | **Date**: 2026-09-03 | **Spec**: [spec.md](./spec.md)

**Input**: Feature specification from `/specs/029-worktree-tooltip-details/spec.md`

## Summary

Every piece the feature needs already exists; none of them meet. A worktree row ellipsizes its
name (`ui/material/tree_view.rs:361`, `Ellipsized`), and the row's tooltip carries exactly one
string — the project-relative location built by `features::sidebar::worktree_location_label`
(`features/sidebar.rs:397`, attached at `ui/sidebar.rs:562-565`, feature 010 FR-010). Everything
else the tooltip should say is already in the `Worktree` the row was rendered from: `dir_name`,
`branch`, `status`, `included` (`micold-core/src/worktree.rs:109-129`).

So this is a label change, not a new subsystem — four changes over existing machinery:

1. **One pure builder replaces one pure builder.** `worktree_location_label` becomes
   `worktree_tooltip`, in the same render-free module, returning the whole labelled block as a
   `String` rather than one path. Every decision about *which* lines appear — branch omitted when
   there is none, folder omitted when it repeats the name, status only when unhealthy — lives
   there, under unit test (Principle I). `ui/sidebar.rs` keeps calling one function and passing the
   result to `row_tooltip`, which is the glue the Principle I exception covers.
2. **The status word moves into the core.** `WorktreeStatus::label()` gives the chip
   (`ui/sidebar.rs:332-339`) and the new tooltip line one source, so they cannot drift into saying
   different things about the same row (FR-006).
3. **The shared `Tooltip` learns to be multi-line.** Today it is a single `Text` in a
   `Shrink`-width container, so a long path would be measured at its natural width and a tooltip
   could grow wider than the sidebar it describes. A component-owned `MAX_WIDTH` ceiling plus
   glyph-level wrapping bounds it (FR-009, SC-005). The change is inside `ui/material/mod.rs` and
   `ui/material/text.rs`, so every tooltip in the app gets it and no feature-local variant is
   forked (Principle VIII).
4. **The gallery gains the multi-line pose**, because a component that grew a shape the gallery
   does not show is a gallery that is quietly out of date — a condition
   `tests/showcase_completeness.rs` exists to prevent.

The "Default" entry keeps `DEFAULT_LOCATION_LABEL` untouched (FR-011); session rows are not
touched at all.

## Technical Context

**Language/Version**: Rust, stable (pinned by `rust-toolchain.toml`)

**Primary Dependencies**: iced 0.14 (`iced_widget::tooltip`, `iced_core::text::Wrapping`); no new
dependency

**Storage**: none — every fact rendered is already in the in-memory `Worktree` list (FR-012)

**Testing**: `cargo test --workspace` via `mise run test`; `mise run test-core` for the core half.
New unit tests in `crates/micold-client/tests/features_sidebar.rs` and
`crates/micold-core/tests/worktree.rs`; the render glue is covered by `quickstart.md` §B under the
Principle I GUI exception.

**Target Platform**: Linux, macOS, Windows desktop (parity by construction — the change is pure
string building plus one layout constraint, with no platform branch)

**Project Type**: Desktop GUI application — Rust workspace of three crates (`micold-core`,
`micold-client`, `micold-daemon`)

**Performance Goals**: tooltip content built from memory on render, no I/O on the hover path
(FR-012); no measurable change to sidebar render cost

**Constraints**: tooltip bounded at 320dp wide (Material 3 rich-tooltip ceiling) and snapped inside
the window by the rendering stack's own overlay; the row itself keeps ellipsizing

**Scale/Scope**: one worktree list, tens of rows; ~5 files touched, no new module

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

- [x] **I. Test-First (NON-NEGOTIABLE)**: PASS. All decision logic — which lines appear, in what
  order, with what wording — lands in `features::sidebar::worktree_tooltip` and
  `WorktreeStatus::label`, both render-free and reachable from `tests/`, both written Red first.
  What is left in `ui/sidebar.rs` and `ui/material/mod.rs` is glue with no branch of its own
  (one call, one `.max_width`, one `.wrapping`), which is exactly the exception's scope, and
  `quickstart.md` §B records the manual pass for it.
- [x] **II. Multi-Session Support**: PASS. No session state is read, written, or added; the tooltip
  is derived per render from the worktree list.
- [x] **III. Worktree Integration**: PASS. No worktree is created, moved, or removed; the feature
  only describes worktrees the app already manages. The "Default" location keeps its own fixed
  label and is not restyled as a worktree (FR-011).
- [x] **IV. Local-First Storage (NON-NEGOTIABLE)**: PASS. Nothing is persisted and nothing leaves
  the device; no new file, no network.
- [x] **V. Rust + iced Stack**: PASS. Rust and iced only. `WorktreeStatus::label` returns
  `Option<&'static str>` so "healthy has no word" is a type-level fact rather than an empty string
  the caller must remember to test — which is the bug the current chip code is one line away from.
- [x] **VI. Cross-Platform Parity**: PASS. No `cfg(target_os)`, no path-separator assumption beyond
  `Path::strip_prefix`/`Display`, which already behaves per platform. CI runs all three.
- [x] **VII. Documentation First-Class**: PASS. `docs/user-guide/worktrees-and-sessions.md:147-149`
  currently describes the tooltip as location-only and must be updated in the same change; it is a
  task, not an afterthought.
- [x] **VIII. Reusable UI Component Foundation**: PASS. No new widget. The multi-line capability is
  added to the shared `material::Tooltip` and the shared `material::Text`, through chainable
  builder methods terminating in `.into()` — not forked into the sidebar. The gallery entry is
  extended in the same change.

*Post-design re-check (after Phase 1): unchanged — all eight still PASS. The design added no new
component, no new state, and no new persisted field; see [data-model.md](./data-model.md), which
introduces no stored entity at all.*

## Project Structure

### Documentation (this feature)

```text
specs/029-worktree-tooltip-details/
├── plan.md              # This file
├── research.md          # Phase 0 output
├── data-model.md        # Phase 1 output
├── quickstart.md        # Phase 1 output
├── contracts/
│   └── worktree-tooltip.md
├── checklists/
│   └── requirements.md  # /speckit-specify output
└── tasks.md             # /speckit-tasks output — NOT created here
```

### Source Code (repository root)

```text
crates/micold-core/
├── src/
│   └── worktree.rs            # + WorktreeStatus::label() — the one status word
└── tests/
    └── worktree_model.rs      # Red for label(): a word for each unhealthy state, none for Valid

crates/micold-client/
├── src/
│   ├── features/
│   │   └── sidebar.rs         # worktree_location_label -> worktree_tooltip (the whole block)
│   ├── ui/
│   │   ├── sidebar.rs         # glue: call the builder; tag_chip reads WorktreeStatus::label
│   │   └── material/
│   │       ├── mod.rs         # Tooltip: MAX_WIDTH ceiling + WordOrGlyph wrapping
│   │       └── text.rs        # Text: + .wrapping(..) builder method
│   └── showcase/
│       ├── catalogue.rs       # Tooltip entry: third posed state
│       └── sections/floating.rs # the multi-line instance
└── tests/
    ├── features_sidebar.rs    # Red for worktree_tooltip: order, omissions, wording
    └── sidebar_tree.rs        # the existing location-label assertions, moved onto the new shape

docs/user-guide/
└── worktrees-and-sessions.md  # lines 147-149: the tooltip is no longer location-only
```

**Structure Decision**: The existing workspace layout is used unchanged. The split that matters
here is the one the constitution's Principle I exception is written around: decision logic in
`micold-core` and `micold-client`'s render-free modules (`features/`), glue in `ui/`. This feature
sits entirely inside that split — nothing new is introduced to hold it.

## Complexity Tracking

> No Constitution Check violations. Table intentionally empty.

## Bugfix BUG-001 — the tooltip must not cover the row it describes

- **Where the rule lives: the shared `Tooltip`, not the sidebar.** The sidebar cannot know where its
  last row lands in the window; the tooltip's overlay can, because the rendering stack lays it out
  against the window with the trigger's on-screen bounds in hand. So the fix is in the shared
  `Tooltip` (`crates/micold-client/src/ui/material/mod.rs`, built on a new `ui/cdk/tooltip.rs`, see
  *Mechanism* below), and every tooltip in the app gains it (Principle VIII). `tree_view.rs:552-553` keeps calling `Tooltip::new` with no position.
- **The rule (FR-013).** Place the panel on the requested side (`Bottom` by default). If that
  placement, once kept inside the window, would intersect the trigger's bounds, place it on the
  opposite side instead (`Bottom` ↔ `Top`, `Left` ↔ `Right`). If neither side holds the whole
  panel, take the side with more room; only then may the window clamp overlap the trigger. Keeping
  it inside the window still applies to the result (FR-009, SC-005).
- **How much room there is.** At the smallest supported window (640×480) a worktree row is ≈64px
  tall, so the larger side has at least ≈208px. FR-009 bounds the panel's width, not its height: a
  four-line tooltip is ≈75px, but SC-005's worst case (a 120-character name, a long branch, an
  absolute path, and the status and outside-this-app lines, all wrapping by glyph at 320px) can
  reach ≈12–13 lines and come close to 208px. SC-006 is therefore stated for a tooltip that fits on
  one side of the row; the fallback above covers the rest.
- **Scope: every tooltip.** The rule lives in the shared component, so every tooltip in the app
  gains it, including the ones that already ask for `Left` or `Top` (`ui/terminal.rs`), the sidebar
  header icons and `split_action`. For a tooltip that never meets its trigger, which is all of
  them today except a row at the window's bottom edge, nothing changes.
- **Mechanism: a cdk tooltip with its own overlay.** `iced_widget::tooltip` 0.14 clamps and never
  flips (`tooltip.rs` overlay `layout`, lines 528–546); its `position` is private and set only at
  construction, and `iced_core::overlay::Element` offers no way to move a node from outside
  (`new`, `as_overlay`, `as_overlay_mut`, `map`). So the flip needs an `Overlay` implementation of
  its own, which means `overlay::Element::new(..)`. `tests/one_overlay_implementation.rs` rejects
  that anywhere outside `ui/cdk/` (`no_module_outside_the_cdk_implements_its_own_overlay`), so it
  goes in a new `ui/cdk/tooltip.rs`, and the same diff adds an argued `CDK_OVERLAY_IMPLEMENTORS`
  entry (the same reason `cdk/picker.rs` gives: it anchors to its trigger's own on-screen bounds,
  inside content-sized dialogs too). If `ui/material/mod.rs` then stops calling the stack's
  `tooltip(`, its `SANCTIONED` entry goes stale and that test says so; strike it in the same diff.
  The Material `Tooltip` keeps its public API and builds on the cdk one.
  *Rejected: `.position(Right)` at the row's call site.* The sidebar can be 600px wide
  (`SIDEBAR_MAX_WIDTH`) in a 640px window (`MIN_WINDOW_SIZE`), where a 320px panel to the right is
  clamped back over the row: the same bug, moved sideways.
- **Test layer.** The regression test is a geometry gate beside `gates/context_menu_anchor.rs`: it
  builds a state with enough worktrees that the last row sits within a row's height of the bottom of
  the harness window **without scrolling** (a row below the fold cannot be hovered), dispatches a
  real `CursorMoved` over that row into a retained tree (the tooltip opens at once; its delay is
  zero), and reads the tooltip's overlay record against that row's bounds. The harness lays out
  against a fixed `WINDOW` (1280×800, `tests/support/layout.rs:58`); the 640×480 case needs the
  gate's helper to take a window size. On `origin/main` the
  two intersect. A second case pins the unchanged behaviour: a row with room below still gets its
  tooltip below it. The flip itself is layout glue inside the component, covered by those gates and
  by `quickstart.md` §B7 under the visual-pass skill.
- **Constitution**: I — both gate cases are written and seen to fail (the first) before the
  component changes. VII — the user guide's tooltip paragraph says where the tooltip opens. VIII —
  the change is in the shared component. No other principle affected.

**Bugfix**: 2026-09-27 — BUG-001 Updated from bugfix patch.

## Bugfix BUG-002 — the gates read the minimum window size instead of restating it

- **Where the constant lives: the library.** `MIN_WINDOW_SIZE` moves from the private
  `crates/micold-client/src/shell/startup.rs:64` (module `shell` is compiled only into the binary,
  `src/main.rs:15`) to `pub const MIN_WINDOW_SIZE` in `crates/micold-client/src/app.rs`, beside
  `SIDEBAR_MIN_WIDTH`, with its doc comment and its compile-time floor `assert!`.
  `shell/startup.rs` imports it for `window_settings` and its unit test; the value (640×480) and the
  window's behaviour are unchanged.
- **The gates read it.** `tests/gates/tooltip_clears_its_row.rs` drops its `SMALLEST_WINDOW`
  literal (and the doc comment that explains the restatement) and uses `MIN_WINDOW_SIZE`;
  `tests/known_projects_reflow.rs` takes its minimum-window width from `MIN_WINDOW_SIZE.width`.
  The `(640, 480)` click points in `tests/switcher_forget_menu.rs` and the 640-wide records in
  `tests/layout_record_format.rs` are not the window's minimum and stay literals.
- **Test layer.** No new test: the regression check is compile-level. With the gates naming
  `micold_client::app::MIN_WINDOW_SIZE`, the test crates do not build on `origin/main` (E0425 /
  E0432, no such item), and build once the constant is exported. The existing gate cases then run
  unchanged at the same 640×480.
- **Constitution**: I — the gates' use of the constant is written and seen failing to compile
  before the constant moves. No other principle affected (no user-visible change, no docs change).

**Bugfix**: 2026-09-30 — BUG-002 Updated from bugfix patch.
