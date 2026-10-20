---
feature: 970-overview-fullscreen
verdict: FAIL
standard: .specify/extensions/tdd/templates/tdd-test-quality-rubric.md # resolved: no override or preset exists, the extension copy
verified_at: 5f54853a
behaviors: 32 # U1-U17 listed in test-list.md; U18-U32 exist only in cycle-log.md and tasks.md
proven: 15
likely: 0
test_after: 17 # U9, U14 (green against stubs, no red) and U18-U32 (implementation first, red by after-the-fact mutation)
no_test: 0
high_smells: 1 # re-implemented expectation (finding 3); the four survivors below are counted separately
criteria_total: 20 # FR-001..FR-014, SC-001..SC-006
criteria_covered: 17 # FR-003, FR-013, SC-006 (keyboard half) have no automated test; FR-002/FR-008 only below the real wiring
mutation_score: 3 of 7 caught (43%) # scope: 7 deliberate mutants on changed files, no cargo-mutants installed
mutants_survived: 4 # M1, M2, M3 inside DONE behaviors; M6 a boundary, near-equivalent
suite: workspace 8477 passed, 7 failed, 16 ignored, 5m31s build (all 7 are root-only permission/environment tests outside the feature)
---

# TDD Verification: Overview full-screen mode (#970)

**Verdict: FAIL.** Three deliberate mutants survived inside `DONE` behaviors (the app-level
`.retracted(..)`, `.instant(..)` and `freed_width` wiring in `ui/mod.rs`), and milestones M2-M4
(U18-U32) were implemented before their tests, by the cycle log's own account.

Independence: I did not write the feature's tests, and I re-read every test cited below as it stands.
Other `speckit-tdd-verify` and `autopilot-unit` processes shared the working tree while this audit ran,
so every mutant ran in a detached scratch worktree at `HEAD`; the main checkout was never edited
(`git diff -- crates` is empty). A previous report at the same `HEAD` was overwritten; its findings 1-9
are kept under the same numbers so tasks T044-T051 in `tasks.md` still point at them.

## Test-first evidence

`test-list.md` holds U1-U17 only. U18-U32 (M2-M4) appear in `cycle-log.md` and `tasks.md` with no list row.

| Behavior | Class | Evidence |
|---|---|---|
| U1-U4 | PROVEN | `281ba3e0` is a test-only commit (`features_overview.rs`, T001); cycle 1 red 2 records `features_overview` `0 passed; 4 failed` against stubs. |
| U5-U8, U10-U13, U15-U17 | PROVEN | Cycle 1 records the failing tests by name or count (6 lib, 2 keymap, 1 overview, U6/U16/U17 on the entry assertion); `6943e349` adds tests and source together, which the rubric accepts as PROVEN when the red is recorded. |
| U9 icons, U14 un-retraction | TEST_AFTER | Cycle 1: "Green against the stubs, as expected". No red; U9 is data, U14 pins the drawer's existing open/close geometry. Low risk, but the class is fail-closed. |
| U18-U26 (M2, `card_grid`) | TEST_AFTER | Cycle 2: "the implementation was written before this run"; red is a mutation made afterwards (`9118b48f`). Tests and source landed together in `0d74a08f`. |
| U27-U28 (M3) | TEST_AFTER | Cycle 3: "implementation was written before the tests"; red by mutation. `0914629b`. |
| U29-U32 (M4) | TEST_AFTER | Cycle 4: "written before the tests again"; the persist test's mutation was never run. `dfce3095`. |

A red made by mutation after the code exists shows a test can fail, not that it drove the code. The cycle log
admits this three milestones running. Cycle log and history agree; no discrepancy.

Tests that existed before the feature (audited with `git show -U0` over the `(970)` commits):
- `tests/showcase_completeness.rs` (`0d74a08f`): `assert!(!entry.interactive, ..)` became `assert_eq!(entry.interactive, component == "CardGrid", ..)`. Still a value check. Not a weakening.
- `tests/overlay_registry.rs` (`d4c7a205`): `*m != "Message::Overview(msg))"` became `!m.starts_with(..)`, and the text is cut at 80 characters after whitespace removal. Looser: a longer message sharing the prefix is now exempt. Finding 6.
- `tests/icons.rs`: set size 68 to 70 for the two new icons. Expected.
- Reformatting-only assertion edits (`896b141c`, `760195f8`, `5e0261b6`) keep every predicate. `5e0261b6` swaps a pinned `Instant` for `clock.restart_at(1.0)` after the clock moved to `Progress`; same intent.
- No test skipped, filtered, excluded, `#[ignore]`d, or threshold lowered.

`tasks.md` against the list: T001-T041 are `[x]` and every behavior in the cycle log is `DONE`; T042 (visual pass) and T043 (gate) are open. No ticked task lacks evidence; no behavioral task is unticked with its behavior done.

## Findings

| # | Severity | Finding | Evidence |
|---|---|---|---|
| 1 | HIGH | Surviving mutant M1: `.instant(state.settings.reduce_motion)` replaced by `.instant(false)`. No test links the setting to the drawer; FR-008 and SC-003 are unverified through the real wiring. Unit tests set `harness.instant = true` directly. | `crates/micold-client/src/ui/mod.rs:406`; `navigation_drawer.rs:1134` |
| 2 | HIGH | Surviving mutant M2: `.retracted(state.overview.full_screen())` replaced by `.retracted(false)`. Full screen would not hide the sidebar and every test stays green (FR-002, FR-009, SC-001). | `crates/micold-client/src/ui/mod.rs:404` |
| 3 | HIGH | Re-implemented expectation: the zoom layout tests compare `rig.pieces()` with `ribbon(..)`/`ribbon_at(..)`, the function `Grid::layout` itself calls, so a wrong ribbon passes. `ribbon_pieces_never_overlap_and_stay_in_range` also asserts `w` is between `rest(start)` and `rest(end)`, where `w` is the test's own `lerp`: it checks the test's arithmetic, not the code. Mid-zoom positions are pinned only by invariants (no overlap, in range). | `card_grid.rs:1007-1008`, `1214,1223,1226,1235,1248,1261,1273,1276,1289` |
| 10 | HIGH | Surviving mutant M3: `freed_width` without the resize handle (`f32::from(sidebar_width_px())` alone) passes. The width the zoom is told it gains, FR-002's "full width", is wired by `ui/mod.rs` and tested nowhere; `full_screen_rows_span_the_freed_width` builds its own `freed`. | `crates/micold-client/src/ui/mod.rs:1407-1412`; `card_grid.rs:1316` |
| 4 | MED | Conditional assertions: `if width > 0.0 { assert .. }` and `if let [a, b] = own[..]` skip assertions for some iterations. The rubric calls this HIGH when a run can assert nothing; here other iterations always assert, so MED. A reviewer may overrule. | `navigation_drawer.rs:1082`; `card_grid.rs:1017` |
| 5 | MED | Eager tests: `an_instant_drawer_snaps_its_presence` pins three behaviors, `a_snapped_width_ends_the_zoom_on_the_next_layout` three; `the_first_visible_card_stays_in_view_across_a_zoom` four. A failure does not name the Story 3 scenario. | `navigation_drawer.rs:1134-1173`; `card_grid.rs:1255-1277, 1162-1206` |
| 6 | MED | `the_keyboard_subscription_names_no_surface` loosened (prefix match, 80-character cut, one new exemption). | `tests/overlay_registry.rs:596-625` |
| 7 | MED | `the_mode_writes_no_sidebar_state` (U6) fails only if the reducer touches `sidebar`, which it never does; the retraction is render-only. It is a guard, and redundant with U16/U17. | `tests/overview.rs` (`the_mode_writes_no_sidebar_state`) |
| 8 | MED | `test-list.md` omits U18-U32, so traces for M2-M4 cannot be checked mechanically. | `tdd/test-list.md` |
| 9 | LOW | `every_exit_leaves_full_screen` sets `s.open = false` directly for the `app.rs` exits, which only tests the `full_screen()` accessor. | `tests/features_overview.rs` (last exit closure) |
| 11 | MED | Surviving mutant M6: `END_REACHED` 0.5 to 5.0 passes. `a_zoom_ends_at_an_end_outside_the_range_or_after_the_slide` pins "within 0.5 px" only from the inside (1449.8); no width between 0.5 and 5 px short of the end asserts the zoom is still running. Near-equivalent (a 4.5 px early finish in a 300 px zoom) but the stated tolerance is unpinned. | `card_grid.rs:224-226, 261`; test at `1233-1235` |
| 12 | MED | FR-003 (top bar stays visible and working) and the keyboard half of SC-006 have no automated test: U15 pins the label, tooltip and icon only. Evidence is the visual pass (`visual-pass.md`, Tab reaches the button; Space "NOT CONFIRMED" in M1). | `specs/970-overview-fullscreen/visual-pass.md` |
| 13 | LOW | `the_full_screen_toggle_names_its_action_and_chord` builds its tooltip expectation from `full_screen_label` and the chord label (both pinned separately), so only the `"{} ({})"` shape is new; acceptable, noted. FR-013 (user guide) has no guard test. | `ui/overview.rs:597-612` |

Smell catalogue pass, remaining items: no assertion-free test, tautological double assertion, doubled subject, over-mocking,
self-approving snapshot (the `layout_snapshot.txt` fixture gained six rows with `05b2d568`, reviewed in that commit), sleep,
real clock or real randomness found. New tests use the profile's helpers (`test_support::laid_out`, `Mounted`,
`FakeSettingsStore`, `project_with`, `tests/support`) and the exemplar layout (unit tests in `mod tests`, one
behavior-per-file integration tests). Redundant test: the settings draft behaviors are pinned in both
`features/settings.rs::reduce_motion_tests` and `main_tests.rs::reduce_motion_is_applied_by_save_and_discarded_by_cancel`;
different levels, so acceptable double-loop coverage.

Properties: new tests are deterministic (injected `Progress` clock, no sleeps) and fast (client lib 3 s, 861 tests). The 7 suite
failures are `micold-core` `settings_refuses_save_over_failed_read`, `settings_write_is_logged`, `worktree_leftovers` (2),
`micold-daemon` `settings_service_write_refuses_failed_read`, `mutation_semantics` (1), `attention_error_notice` (1). Six depend on
file permissions that the container's uid 0 ignores; `attention_error_notice::without_focus_the_first_reporting_window_gets_the_notice`
was not diagnosed. None touches a file the feature changed in the daemon; the core settings failures are the unreadable-file
tests, not `reduce_motion`.

## Mutation results

Tool: none (`mutation: null` in the profile; `cargo-mutants` absent). Seven deliberate mutants, one at a time, each in a scratch
worktree, tested with `cargo test -p micold-client --no-fail-fast --lib --bins --test overview --test features_overview --test keymap --test icons --test showcase_completeness`.
After the last one the same command was green: 1649 passed, 0 failed, and each file was verified restored. The sample is not exhaustive.

| Mutant | Behavior | Survived | Judgment |
|---|---|---|---|
| M1 `ui/mod.rs:406` `.instant(false)` | U29 / FR-008 | **Yes** | Real gap, finding 1 |
| M2 `ui/mod.rs:404` `.retracted(false)` | U10 / FR-002 | **Yes** | Real gap, finding 2 |
| M3 `ui/mod.rs:1411` freed width drops the handle | U26 / FR-002 | **Yes** | Real gap, finding 10 |
| M4 `features/overview.rs` toggle ignores `settings_open` | U1 | No | Caught by `full_screen_toggles_only_while_open` |
| M5 `features/overview.rs:350` opening keeps the mode | U2 | No | Caught by `opening_the_overview_clears_full_screen` and `twenty_cycles_return_the_sidebar_as_it_was` |
| M6 `card_grid.rs:261` `END_REACHED` 5.0 | U23 | **Yes** | Near-equivalent boundary, finding 11 |
| M7 `features/settings.rs:714` `into_settings` drops `reduce_motion` | U29 / FR-008 | No | Caught by `the_validated_settings_carry_the_draft_value` and `reduce_motion_is_applied_by_save_and_discarded_by_cancel` |

Score 3 of 7 caught, scope: the changed client files, a sample. A prior run at the same `HEAD` also caught the chord key,
the drawer `gone` flag, the `zoom_fraction` clamp and the icon choice, and found the same two glue survivors (M1, M2).

## Traceability

| Criterion | Tests | End to end |
|---|---|---|
| FR-001, FR-010, SC-005 | U1-U3, U7, U8 | Yes (`app_chord_tests`, `tests/overview.rs` on the app `State`) |
| FR-002, FR-009, SC-001 | U10-U14, U16, U26 | No: drawer and reducer only; `.retracted` and freed-width wiring unpinned (M2, M3) |
| FR-003 | none | No automated test (finding 12) |
| FR-004, FR-006, FR-007, SC-002 | U18-U25 | Partly: widget tests; expectations reuse `ribbon` (finding 3); the touching-edges claim of SC-002 spans drawer and grid and is tested in neither together |
| FR-005 | U12, U20 | Drawer curve and 500 ms pinned; the grid's slide shares `SLIDE` |
| FR-008, SC-003 | U29-U32 | No: setting-to-drawer wiring unpinned (M1) |
| FR-011, SC-004 | U4, U5, U16, U17 | Yes |
| FR-012, SC-006 | U9, U15 | Label, tooltip, icon yes; keyboard reach only by visual pass |
| FR-013 | none | Docs; no guard test |
| FR-014 | U7 | `#[cfg(target_os)]` label: only the Linux branch ran here |

Every named test in `test-list.md` exists (all 17 resolved by `grep`); U18-U32 test names were confirmed in `card_grid.rs`,
`navigation_drawer.rs`, `features/settings.rs`, `main_tests.rs`, `persist.rs` and core `settings.rs`. Tests tracing to nothing: none.

## What was not audited

- Coverage: no tool in the profile.
- `cargo-mutants`: not installed; mutation is a 7-mutant sample, not a score over the changed files. No mutants on the scroll anchor (`on_reflow` shift), the persist path, the appearance page, or `ribbon` row-split arithmetic.
- macOS and Windows chord labels and CI legs.
- The `micold-daemon` and sandbox acceptance suites beyond the one workspace run; `attention_error_notice` failure cause was not diagnosed.
- Visual pass (T042) and the full gate (T043) are open tasks and were not run.
- Performance beyond wall time; docs content (FR-013) was not read.
