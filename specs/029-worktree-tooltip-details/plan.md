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
