# Tasks: Compact session header

**Input**: `specs/1463-compact-session-header/` (spec.md, plan.md, research.md, contracts/header-strip.md, quickstart.md)
**Tests**: required (Constitution I): each test task precedes the code it covers and is seen red first.

Paths: `C` = `crates/micold-client`. Gates: `C/tests/gates/session_area_fits.rs` (a module of the `layout_snapshot` test target), `C/tests/session_area_stability.rs`, `C/tests/layout_snapshot.rs`.

## Phase 1: Setup

- [x] T001 Read the current gates and note the helpers to extend (`header_parts`, `strip_parts`, `assert_button_unsqueezed`, `builders()`) in `C/tests/gates/session_area_fits.rs` and `C/tests/session_area_stability.rs`; no edit.

## Phase 2: Foundational

- [x] T002 Make `terminal::active_primary` `pub(crate)` in `C/src/ui/terminal.rs` (needed by the strip slot in US2; no behaviour change).

## Phase 3: User Story 1 - Header on one line (P1) 🎯 MVP

**Independent test**: `session_area_fits` header gates (in the `layout_snapshot` target) at 640x480+300 sidebar and 1280x800, light and dark.

- [x] T003 [US1] Write failing gate in `C/tests/gates/session_area_fits.rs`: header children are title, chips, Changes on one line in that order (FR-001); header height at most about 40 px and Changes no taller than the title line, header at most 40 px (FR-003, SC-003).
- [x] T004 [US1] Write failing gate in `C/tests/gates/session_area_fits.rs`: at 640x480 the title shortens first, chips stay at `CHIP_MAX_WIDTH`, Changes keeps its width for `Changes · 12 files · +345 −67` and four-digit lines cut with ellipsis after title minimum; at 1280x800 five-digit files and six-digit lines whole (FR-002, H10'). The cut itself already exists in `changes_button`.
- [x] T005 [US1] Compact `changes_button` via new `material::Button::density(DENSE)` in `C/src/ui/session_area.rs` (R2, FR-003).
- [x] T006 [US1] Rebuild `header()` as `row![container(title).width(Fill), chips_row, changes]`, spacing `MD`, `align_y(Center)`, vertical padding `XS`, child list never varying, in `C/src/ui/session_area.rs` (FR-001, FR-002).
- [x] T007 [US1] Re-record layout fixtures for the new header after T003-T006 are green: `C/tests/layout_snapshot.rs` and its fixtures (FR-010).
- [x] T008 [US1] Update header description in `docs/user-guide/terminal-panes.md` and note H9/H10 amendment in `specs/1126-session-context-header/contracts/context-header.md` (FR-010).

**Checkpoint**: header is one line on main.

## Phase 4: User Story 2 - Switch control in the strip (P1)

**Independent test**: Claude terminal single pane shows 2 rows above the terminal; switch slot before restart.

- [x] T009 [US2] Write failing unit test for the shared eligibility helper in `C/src/ui/terminal.rs` (primary terminal AI tab, terminal kind, entry exists, not transcript, single pane, Review not shown) (FR-004, FR-006).
- [x] T010 [US2] Write failing gates in `C/tests/gates/session_area_fits.rs`: strip order tabs, status words, switch slot, restart last (FR-004, FR-005); caption cut with ellipsis before switch/restart clipped at 640x480 with longest `held_queue_line` caption (FR-012, SC-006); rows above pane area is 2 for every session and pane top offset about 90 px, including the outlined switch button height (FR-009, SC-001, SC-002); name session kinds (Claude terminal, plain terminal, non-terminal) and a stopped/restarting case (switch button enabled rules as before); add a Review-shown-then-closed case asserting restart keeps its x offset from the right edge (FR-008). Numeric bounds at scale 1.0: header at most 40 px, two rows at most 90 px; if the outlined switch button breaks 90 px, give it smaller padding in T013.
- [x] T011 [US2] Write test that a tiled pane still builds its switch row and its row size/position relative to the pane-area origin equal pre-change values, in `C/tests/gates/session_area_fits.rs` (FR-007, SC-004).
- [x] T012 [US2] Add the shared eligibility helper in `C/src/ui/terminal.rs`, used by both the tiled row and the strip (D2).
- [x] T013 [US2] Change `status_cluster` to `status_cluster(state, model, switch: Option<SwitchSlot>, r)`, add `switch_slot(state, area, model, r)`: always one child, empty `container(Space)` or `row![caption (Ellipsized, max_width cap, tooltip), switch_button]`; update `strip()` in `C/src/ui/session_area.rs` (FR-004, FR-005, FR-008, R5, R6).
- [x] T014 [US2] Stop stacking `terminal_switch_strip` above the body for a single pane in `terminal::pane` in `C/src/ui/terminal.rs`; tiled call in `C/src/ui/panes.rs` unchanged (FR-007, R4).
- [x] T015 [US2] Update `C/tests/session_area_stability.rs`: new `"fn status_cluster<'a>("` marker, edit `the_pane_area_draws_no_bar_under_the_terminal` (drop `terminal_switch_strip(` end marker, assert single-pane `pane` no longer calls it, fix doc comment); child list stable across idle, running, stopped, held queue, Review shown, and a press lands first try (FR-008, SC-005).
- [x] T016 [US2] Re-record tiled and single-pane layout fixtures in `C/tests/layout_snapshot.rs` (FR-010).
- [x] T017 [US2] Update the strip section of `docs/user-guide/terminal-panes.md` and S7'/rows note in `specs/1126-session-context-header/contracts/context-header.md` (FR-010).

## Phase 5: User Story 3 - Steady tabs and controls (P2)

**Independent test**: existing stability and fit gates pass at the smallest window.

- [x] T018 [US3] Add the 640x480+300 sidebar no-overlap/inside check for header plus strip with switch slot filled, light and dark, in `C/tests/gates/session_area_fits.rs` (US3 scenario 2); confirm `C/src/ui/tab.rs` untouched (FR-011).

## Phase 6: Polish

- [ ] T019 Run quickstart Part A and the Part B visual pass (`visual-pass` skill), save screenshots under `specs/1463-compact-session-header/visual-pass/` (done by the close unit in the close PR; changes no code, so it is in no milestone).

## Dependencies

T002 before T013. US1 (T003-T008) independent of US2. US2 tests T009, T010, T011 before T012-T014. T018 after T013.

## Parallel

T003 and T004 touch one file (not [P]). None; kept sequential.

## Implementation Strategy

MVP is US1 (M1). US2 and US3 ship as M2: US3 is one guard task (T018) that needs the strip from T013, so it has no deliverable of its own; M2 is 11 tasks with a small diff (two files plus fixtures), kept whole.

## Milestones

Each milestone merges to `main` on its own, through one PR (speckit-autopilot).

### M1 — One-line session header 🎯 MVP

- **Tasks**: T001, T003–T008
- **Deliverable**: The session header shows title, chips and a compact Changes button on one line of about 40 px, title shortening first.
- **Satisfies**: US1 acceptance scenarios 1–3; FR-001, FR-002, FR-003, FR-010 (header part)
- **Verify**: `cargo test -p micold-client --test layout_snapshot` (through `mise run gate`)
- **Depends on**: —
- **Tier**: full

### M2 — Switch control in the tab strip

- **Tasks**: T002, T009–T018
- **Deliverable**: A Claude terminal session in a single pane has only header and strip above the terminal, the switch control and held-queue caption sit in the strip before restart; tiled panes unchanged.
- **Satisfies**: US2 acceptance scenarios 1–6; US3 scenarios 1–2; FR-004–FR-009, FR-011, FR-012, FR-010 (strip part)
- **Verify**: `cargo test -p micold-client --test layout_snapshot --test session_area_stability`
- **Depends on**: M1
- **Tier**: full

### M3 — Test-strength remediation

- **Tasks**: T021–T028
- **Deliverable**: The fit gates and unit tests kill the five surviving mutants of `tdd/verification.md` (exact button width, tiled switch row numbers, caption cap, held-queue caption, GiveWayRow/Button::density, rank tie), with no behaviour change.
- **Satisfies**: FR-007, FR-011, FR-012; SC-004
- **Verify**: `cargo test -p micold-client --test layout_snapshot` and `cargo test -p micold-material`; each task's mutant fails
- **Depends on**: M2
- **Tier**: full

## Phase 7: TDD remediation

Test-strength findings of `tdd/verification.md` (verdict FAIL). The missing red phase (F1) cannot be redone and is a recorded limitation, not a task.

- [X] T021 Finding 2: restore an exact-width check for `assert_button_unsqueezed` in `C/tests/gates/session_area_fits.rs` wherever the title is at or above `TITLE_MINIMUM`, and add a case that drives the title below it and asserts the cut width equals the width left by the line (not just `<= wanted`); or record in the spec a user-approved amendment with the number. Proof: `cargo test -p micold-client --test layout_snapshot` green, and a mutant that always returns `wanted` for the cut case fails.
- [X] T022 Finding 4: rewrite `a_tiled_pane_still_has_its_switch_row` in `C/tests/gates/session_area_fits.rs:1434` to locate the tile's switch row by path, and assert its size and its offset from the pane-area origin equal the recorded pre-change values (FR-007, SC-004). Proof: mutants dropping the tiled switch row or changing its padding (`[XS, SM]` -> `[SM, SM]` in `terminal_switch_strip`) fail.
- [X] T023 Finding 5: pin the caption cap independently (a literal in the test, or caption width equals 120 at 1280x800 for the longest caption). Proof: mutant `SWITCH_CAPTION_MAX_WIDTH = 400.0` fails `a_long_caption_is_cut_before_the_switch_or_restart_is_clipped`.
- [X] T024 Findings 3, 6, 7, 13: name the zero-width exemption of `"strip tab region"` at `session_area_fits.rs:269` with a comment citing FR-012, replace the `80.0`/`36.0`/`12.0` literals with named constants, drop the redundant restart conjunct, tighten the switch-row heuristic and the natural-width check of New session/split buttons. Proof: `cargo test -p micold-client --test layout_snapshot` green.
- [X] T025 Finding 10: add a unit test for `switch_offer_of` with a held queue (caption is `Some(line)`) and without (`None`). Proof: mutant `caption: None` fails.
- [X] T026 Finding 11: add micold-material unit tests for `GiveWayRow` (`shorten` yields before `keep`, ranks in order) and `Button::density` (height at DENSE vs STANDARD). Proof: mutants `Shorten(rank)` -> `Keep` and density ignored fail `cargo test -p micold-material`.
- [X] T027 Finding 12: add a layout case where words and the switch slot both overflow and assert which shortens first. Proof: mutant `.shorten(1, words)` -> `.shorten(0, words)` fails.
- [X] T028 Finding 13: add a direct test that `tab::SESSION_WIDTH` still equals its recorded value (FR-011), or note it is guarded only by the tab snapshots.
