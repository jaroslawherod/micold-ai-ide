# Tasks: Review tab fits the smallest window

**Input**: `specs/1474-review-tab-fit/` (spec.md, plan.md, research.md, data-model.md, contracts/, quickstart.md)

**Format**: `[ID] [P?] [Story] Description (file)`. `[P]` = disjoint files, can run in parallel. All
`crates/...` paths are from the repo root. Test tasks come first and are written red (Constitution I).

Stories: US1 review tab fits (P1), US2 sidebar gives way (P1), US3 open the sidebar in a small window
(P2), US4 full screen and sidebar agree (P3).

## Phase 1: Foundational (reducer tests, then the compact state)

- [x] T001 [US2] Write red reducer tests for the contract's table: resize 1000/640/720/719, first report 640 and 0x0, hide at 1000 then resize, toggle at 640, toggle/resize ordering, two independent states; assert `sidebar_hidden()` and that `hidden`/`width` are unchanged (`crates/micold-client/tests/sidebar_state.rs`, `crates/micold-client/tests/features_window.rs`)
- [x] T002 [P] [US4] Write red full-screen tests: full screen at 640 then close gives `sidebar_hidden()` true; at 1000 with the sidebar expanded, close returns expanded, `hidden`/`width` unchanged (`crates/micold-client/tests/overview.rs`)
- [x] T003 [US2] Add `COMPACT_WIDTH = 720` and `window::State::compact()` (D1), with `window::resized` clearing `sidebar.compact_open` when no longer compact (`crates/micold-client/src/features/window.rs`)
- [x] T004 [US2] [US3] Add `sidebar::State.compact_open` and make `sidebar::toggled` flip it at compact width, else `hidden` (D2) (`crates/micold-client/src/features/sidebar.rs`)
- [x] T005 [US2] Add `app::State::sidebar_hidden()` and switch its two readers, the drawer `open` flag and `freed_width` (`crates/micold-client/src/app.rs`, `crates/micold-client/src/ui/mod.rs`); use `findReferences` on `sidebar.hidden` to confirm no other reader; T001 and T002 now pass

## Phase 2: Other suites that set a 640 window (D10)

- [x] T006 [US2] Run `cargo test -p micold-client`, then `mise run gate`; list every suite broken by the strip (candidates in plan D10); fix each by setting the size to 720 or wider where it needs the sidebar, or `sidebar.compact_open = true` where it needs it at 640; the `review_tab_fits` gate failing on stale `#1474` entries is expected here and cleared by T008; record the list in `specs/1474-review-tab-fit/autopilot.md` (files: the failing suites under `crates/micold-client/tests/`)

## Phase 3: US2/US3 gate premise and docs

- [x] T007 [US2] Add the per-state premise to the review gate: at `MIN_WINDOW_SIZE` the sidebar drawer is the 32px strip and the main area (`REVIEW_*` ancestors) is 608px wide; at `WINDOW` the sidebar is expanded (D3, FR-011, SC-004) (`crates/micold-client/tests/gates/review_tab_fits.rs`)
- [x] T008 [US2] With the strip in place, delete each `#1474` entry in `SMALLEST_WINDOW` that no longer fires (the gate fails for a stale entry); re-file the stale-notice source-control entry under a new issue if it still fires (D8, FR-012) (`crates/micold-client/tests/gates/review_tab_fits.rs`)
- [x] T009 [P] [US2] Update the user guide: the narrow-window exception to "the sidebar always returns exactly as you left it", and the bullet in "Resizing and hiding the sidebar" (D9, FR-006) (`docs/user-guide/overview.md`, `docs/user-guide/worktrees-and-sessions.md`)
- [x] T010 [US2] Confirm FR-013: the `layout_snapshot` fixture at 1280x800 is unchanged with `cargo test -p micold-client --test layout_snapshot` (run only; `crates/micold-client/tests/fixtures/layout_snapshot.txt` unchanged)

**Checkpoint**: at 640 the sidebar is the strip, toggling works, 1280 is unchanged.

## Phase 4: US1 the review tab fits (starts from the remaining `#1474` entries)

- [ ] T011 [P] [US1] Write red unit tests for `drawer::capped_with_floor` and `drawer::width`: areas >= 2 x 360 give 360, a 320 area gives 240, an area under the floor gives the area (`crates/micold-core/src/review/drawer.rs`)
- [ ] T012 [US1] Add `capped_with_floor`, make `width` use it, add `anatomy::comments_drawer::MIN_WIDTH` (start 240) to the anatomy ALL table, update the `WIDTH` doc comment (D6a, D6b) (`crates/micold-core/src/review/drawer.rs`, `crates/micold-core/src/tokens/anatomy.rs`)
- [ ] T013 [US1] Add `SideSheet::min_width` (default 0, showcase pose unchanged) used by `layout` via `capped_with_floor`; `comments_drawer` passes `MIN_WIDTH` (`crates/micold-client/src/ui/material/side_sheet.rs`, `crates/micold-client/src/ui/changes.rs`)
- [ ] T014 [US1] Drawer rows: action buttons use `Row::wrap()`; tune `MIN_WIDTH` to the measured natural width of the widest row controls plus padding; delete the drawer header and row `#1474` entries first (red), then fix to green (D6c, FR-009) (`crates/micold-client/src/ui/changes.rs`, `crates/micold-client/tests/gates/review_tab_fits.rs`)
- [ ] T015 [US1] Diff header: remove the entry first (red; it still fires), then give the controls row natural height and, if needed, reorder Whole file and the mark chip onto a second line (no control removed); the gate is green again (D4, FR-007) (`crates/micold-client/src/ui/changes.rs`, `crates/micold-client/tests/gates/review_tab_fits.rs`)
- [ ] T016 [US1] Diff placeholder: remove the entry first (red; it still fires), then centre `StageProgress` in a `Fill` container; the gate is green again (D5, FR-008) (`crates/micold-client/src/ui/changes.rs`, `crates/micold-client/tests/gates/review_tab_fits.rs`)
- [ ] T017 [US1] Pin FR-010 in the gate: at the smallest window the `changes-tree-filter` box and every tree row lie inside the tree pane, `text_overflows` empty (D7) (`crates/micold-client/tests/gates/review_tab_fits.rs`)
- [ ] T018 [US1] Run `cargo test -p micold-client --test layout_snapshot` and confirm zero `#1474` entries remain except a re-filed stale-notice entry (SC-001), and 1280x800 layouts are unchanged (FR-013, SC-005) (`crates/micold-client/tests/gates/review_tab_fits.rs`)

## Phase 5: Polish

- [ ] T019 (close unit) Run quickstart §B (visual pass at 640x480 and 1280x800, light and dark) and record the result in the spec directory (`specs/1474-review-tab-fit/`)

## Dependencies

T001, T002 before T003-T005. T005 before T006. T006 before T007-T010. T011 before T012; T012 before T013, T014.
T008 before T014-T016 (it decides which entries are left). T017 any time after T007.

## Implementation Strategy

MVP is M1: the sidebar collapses at compact width with its tests, the audit of other 640 suites, the gate
premise and the docs. M2 then fixes what the wider main area leaves: the drawer floor, header and
placeholder, driven by the remaining `#1474` gate entries (each deleted entry is the red-to-green signal).

## Milestones

Each milestone merges to `main` on its own, through one PR (speckit-autopilot).

### M1 — Sidebar gives way at the smallest window 🎯 MVP

- **Tasks**: T001–T010
- **Deliverable**: in a window narrower than 720px the sidebar shows as its strip, the main area is 608px wide at 640x480, show/hide and full screen work there, wider windows and 1280x800 layouts are unchanged, and the user guide says so
- **Satisfies**: US2 acceptance scenarios 1–4; US3 scenarios 1–3; US4 scenarios 1–2; FR-001–FR-006, FR-011, FR-013 (SC-002, SC-003, SC-004)
- **Verify**: `cargo test -p micold-client --test sidebar_state --test features_window --test overview`; `mise run gate`; `cargo test -p micold-client --test layout_snapshot review_tab_fits` (green after T008)
- **Depends on**: —
- **Tier**: full

### M2 — The review tab fits at 640x480

- **Tasks**: T011–T018 (T019, the quickstart record, is left to the close unit)
- **Deliverable**: the diff header, loading placeholder, comments drawer and tree lie inside their boxes at 640x480 in every review state, light and dark, with no `#1474` known-defect entry left (bar a re-filed stale notice)
- **Satisfies**: US1 acceptance scenarios 1–5; FR-007–FR-010, FR-012; SC-001, SC-005
- **Verify**: `cargo test -p micold-client --test layout_snapshot review_tab_fits`; `cargo test -p micold-core review::drawer`; quickstart §B
- **Depends on**: M1
- **Tier**: full
