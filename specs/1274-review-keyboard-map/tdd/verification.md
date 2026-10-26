---
feature: 1274-review-keyboard-map
verdict: FAIL # TEST_AFTER history in 3 M3 rows, accepted at close (deliberate mutants now catch each); the one HIGH smell (S1) fixed in the close unit
standard: .specify/extensions/tdd/templates/tdd-test-quality-rubric.md # rubric graded against
verified_at: a42fe315f # short SHA audited (main after #1460); close-unit fixes on top, see the last section
behaviors: 32 # rows of the table below; no tdd/test-list.md exists, the cycle log's task rows stand in
proven: 23
likely: 2
test_after: 3
no_test: 0
not_applicable: 4 # scaffolding and three guards
high_smells: 1 # S1, vacuous whole-file assertion; fixed in the close unit
high_findings: 2 # S1, and the test-first breach (3 behaviors)
criteria_total: 28 # FR-001..FR-022, SC-001..SC-006
criteria_covered: 28 # FR-005 and FR-021's visual half by the visual pass only, see Traceability
mutation_score: unmeasured # no mutation tool in the profile; 12 deliberate mutants, 12 caught
mutants_survived: 0
suite: scripts/gate.sh GATE_EXIT=0 at a42fe315f — 11425 passed, 0 failed, 18 ignored (585 test binaries); close tree's full gate GATE_EXIT=0, 11426 passed, 0 failed, 18 ignored
---

# TDD Verification: Review UI — keyboard map for reviewing (#1274)

**Verdict: FAIL.** Three M3 behaviors (`review_press`, the keyboard holder, the switch's settings
and persist wiring) were written test-and-code in one step with no red run (`TEST_AFTER`), and one
client test asserted the whole-file choice against a clone of itself (S1, `HIGH`, fixed in the
close unit). No existing test was weakened, every criterion has a test, and test strength is good:
12 deliberate mutants, the three `TEST_AFTER` behaviors among them, all caught.

The audit was run by hand per `.claude/skills/speckit-tdd-verify/SKILL.md` (the forked skill does
not load in this untrusted workspace). The smell pass ran in a fresh-context headless
`autopilot-reviewer` (read-only tools); every finding it cited was re-opened here before it went in.

## Test-first evidence

History shape: one `feat(1274)` commit per milestone carries its tests and code together
(`469a3f918` M1, `3db9210da` M2, `c85bb320f` M3, `e2c58c918` M4, `b022f8830` M5), plus review-fix
commits. Ordering inside a commit is not visible, so `PROVEN` rests on the cycle log's recorded red
output, as the rubric allows for a test-and-source commit.

| Behavior (cycle-log row) | Class | Evidence |
|---|---|---|
| M1 T001 module scaffolding | NOT_APPLICABLE | no behavior |
| M1 T002→T003 K1 table | PROVEN | compile red, then `6 passed; 7 failed` against `resolve` stub; `469a3f918` |
| M1 T004→T005 K2/K3 `resolve` | PROVEN | `6 passed; 7 failed` (stub `None`) |
| M1 T048→T049 J2/J3 detail ranking | PROVEN | compile red (E0433 `Via`, E0560 `detail`) |
| M1 T050→T051 J1 `review_candidates` | PROVEN | 3 `review::jump` tests FAILED on a `Vec::new()` stub |
| M1 T006→T007 `has_hunks`, `step_file` | PROVEN | stubs: 4 named tests FAILED (`285 passed; 26 failed`) |
| M1 T008→T009 `step_hunk`, `resolve_edge` | PROVEN | 7 tests FAILED on stubs |
| M1 T010→T011 `header_rows` | PROVEN | `header_rows_name_each_hunk_headers_row_in_both_layouts ... FAILED` |
| M1 T022→T023 `next_unreviewed` | PROVEN | 4 `g_*` tests FAILED |
| M1 T028→T029 `comment_order`, `step_comment` | PROVEN | 4 tests FAILED |
| M1 T030→T031 `hunk_change_range`, `comment_target` | PROVEN | 2 tests FAILED |
| M1 T032 SC-003 timing | PROVEN | FAILED on its result assertions against stubs |
| M1 review A fix: stale `p` past a shrunken diff | PROVEN | `-- shrunken` FAILED (`File{..Last}` vs `Current(1)`); `57311237a` |
| M2 T059→T060 `review_single_keys` setting | LIKELY | red was a compile failure only, no failing assertion run |
| M2 T061→T062 `resolve(.., single_keys)`, `sheet_rows`, `key_hint` | LIKELY | compile failure only |
| M3 T013→T014 `primary_held`, `review_press` | TEST_AFTER | "written with the code in one step"; mutant G now caught |
| M3 T015→T016 keyboard holder | TEST_AFTER | holder not stubbed: its tests passed on the red run; mutants H, I now caught |
| M3 T017→T018 `j k n p`, `tree::file_order` | PROVEN | `5 passed; 10 failed`; `file_order ... FAILED` |
| M3 T024→T025 `v`, `g` | PROVEN | same stubbed run, named tests FAILED |
| M3 T063→T064 switch off in the Review tab, tooltips | PROVEN | `with_the_switch_off…` FAILED |
| M3 T063→T064 settings draft/save/cancel, persist | TEST_AFTER | "written with the code (accepted red)"; mutants K, L now caught |
| M4 T035→T036 `TextArea` chord and `.on_press` | PROVEN | `features_composer`: `29 passed; 2 failed` |
| M4 T037 send chord is no reserved chord | NOT_APPLICABLE | a guard, green on first run by design |
| M4 T033→T034 `] [ c` | PROVEN | 5 tests FAILED |
| M4 T038→T039 send chord | PROVEN | 4 tests FAILED |
| M4 T041→T043 shortcut sheet, registry | PROVEN | 3 sheet + 2 registry tests FAILED |
| M4 T044→T045 user guide names every key | PROVEN | `the_user_guide_names_every_review_key_and_its_text` FAILED |
| M5 T054 core `comment_move` | PROVEN | `20 passed; 5 failed` |
| M5 T053→T054 J4 entries and choosing them | PROVEN | `quick_switch`: `51 passed; 5 failed` |
| M5 T053 FR-019 no tab → no entries | NOT_APPLICABLE | a guard, green on the red run |
| M5 T055 row texts | PROVEN | `review_rows_show_the_reviewed_state_status_and_line` FAILED |
| Close: focused terminal keeps every review key (§C) | NOT_APPLICABLE | a guard added at close (`a_focused_terminal_keeps_every_review_key`), green on first run |

**Existing tests.** Every removed line in a test file across the feature's commits was read. None
loosens a check: `DIALOGS 27 → 28` and the dialog-list counts `23 → 24` and `48 → 50` (`dfdd74011`, `e2c58c918`)
follow the new dialog; `settings_sections.rs` dropped `review_single_keys` from `DEFERRED` when its
row shipped (`c85bb320f`); `session_area_fits.rs` widened a loop to a new state; the `quick_switch.rs`
match and the `review_keys.rs` fixtures moved to `tests/support/review_fixture.rs` (`b022f8830`); the
two `resolve` assertions M2 rewrote (`3db9210da`) kept their messages and gained the switch argument.

**tasks.md vs evidence.** No test list; every task is ticked and each behavioral task has its row
above. No ticked task lacks evidence.

## Findings

| # | Severity | Finding | Evidence |
|---|---|---|---|
| 1 | HIGH | Test-first breach: 3 behaviors `TEST_AFTER` (T013/T014, T015/T016, T063/T064 settings and persist) | cycle log M3 rows; mutants G, H, I, K, L each caught afterwards |
| S1 | HIGH | `moving_changes_nothing_but_the_selection_and_the_current_hunk` compared `whole_file` with a clone of the same state after `DiffScrolled`, never with the value before the `n` presses: vacuous | `crates/micold-client/tests/review_keys.rs:390` (before the fix) — **fixed at close**: captured before, compared after |
| S2 | MED | SC-003 timing tests use the real clock with fixed budgets (100 ms per arm, 50 ms in all) | `review_keys.rs::every_arm_is_fast_on_a_large_review`; `crates/micold-core/src/review/navigate.rs:972` — kept: SC-003 names the budget, and the margin is wide (runs take < 1 ms); flake risk recorded |
| S3 | MED | The switch-off sheet test builds its expected rows from `sheet_rows(false)` + `labels()`, the code the model uses | `review_keys.rs::with_the_switch_off_the_sheet_lists_only_the_chords` — mitigated: core pins `sheet_rows(false)` to `["Send review", "Save comment"]` (`keys.rs:787`) and the labels in `labels_give_the_linux_windows_and_the_macos_form`; the client test also pins the count and the note |
| S4 | MED | `on_press_sends_its_message_for_each_press_it_claims` drives `.on_press` with a test closure | `crates/micold-client/tests/features_composer.rs:824` — kept: `.on_press` is the feature's own change to `micold-material`'s `TextArea` (T036), driven through the real widget; the review claim (`composer_press`) has its own tests in `review_keys.rs` |
| S5 | MED | Platform-conditional expectations (`cfg!(target_os = "macos")`) assert one half per platform | `keymap.rs:1109`; `review_keys.rs` `chord()`, the tooltip and sheet tests — kept: the producer is per platform by design (K5); both label forms are pinned on every platform in core; CI runs the macOS half (quickstart §C) |
| S6 | MED | Eager tests / assertion roulette in a few multi-scenario tests | `keymap.rs::a_review_press_carries_the_produced_text_and_every_modifier`, `review_keys.rs` US-scenario tests — taste; not changed |
| S7 | MED | The user-guide test matches each label anywhere in the section, not on its row | `keymap.rs::the_user_guide_names_every_review_key_and_its_text` — not changed |
| S8 | MED | Some tests read view fields (`pending_edge`, `last_comment`, drawer state) a refactor could rename | `review_keys.rs` n/p crossing and `] [` tests — not changed; those fields are the contract K7 state |
| S9 | LOW | Repeated `GutterPressed` literals and 30-file lists | `review_keys.rs` — not changed |

## Mutation results

No mutation tool in the profile (`mutation: null`). Twelve deliberate mutants, one at a time, each
restored and the tree confirmed clean (`git diff --stat -- crates` empty) after the run. Core runs:
`cargo test -p micold-core --lib -- review:: quick_switch`; client runs: `cargo test -p
micold-client --lib --bins --test review_keys --test keymap --test quick_switch`.

| Mutant | Behavior | Survived | Caught by |
|---|---|---|---|
| A `core review/keys.rs` AltGr test → `false` | FR-015 AltGr | No | `ctrl_alt_or_logo_alone_refuses_a_single_key_but_altgr_does_not` |
| B `core review/keys.rs` `is_on` drops `|| !is_single()` | FR-022 chords stay on | No | 3 `review::keys` tests (`with_the_switch_off_the_send_chord_still_sends` …) |
| C `core review/navigate.rs` `g` starts at the selected file | FR-008 | No | `g_visits_the_unreviewed_files_in_order_and_wraps` |
| D `core review/navigate.rs` comment order: time before line | FR-006 | No | `comments_order_by_file_then_line_then_time_then_id_unlisted_last` |
| E `core quick_switch.rs` sort by rank before via | FR-017 name before detail | No | `a_name_match_sorts_before_any_detail_only_match` |
| F `core review/jump.rs` file entry named by its full path | FR-016 | No | `a_file_entry_is_named_by_its_file_and_detailed_by_its_path` |
| G `client keymap.rs` `review_press` drops `repeat` | FR-015 (TEST_AFTER T014) | No | `a_review_press_carries_the_produced_text_and_every_modifier` |
| H `client changes/keys.rs` holder ignores the Overview | FR-002 (TEST_AFTER T016) | No | `a_field_settings_the_overview_or_a_surface_on_top_has_the_keyboard` |
| I `client changes/keys.rs` `Composer` without an open box | US1 s6 (TEST_AFTER T016) | No | `the_open_focused_comment_box_holds_the_keyboard` |
| J `client quick_switch.rs` review entries not added | FR-016/FR-018 | No | 3 `quick_switch` tests |
| K `client shell/persist.rs` switch not written | FR-022 save (TEST_AFTER T064) | No | `saving_the_single_key_switch_keeps_every_other_setting`, `the_single_key_switch_is_applied_by_save_and_discarded_by_cancel` |
| L `client features/settings.rs` draft opens with the switch on | FR-022 form (TEST_AFTER T064) | No | `the_switch_is_on_by_default_and_the_draft_opens_with_the_stored_value` |

## Traceability

Client tests drive the root `app::State::update` / `update_changes` and the shell's `persist`, the
entry points the iced event loop calls; there is no window-driving test harness beyond the layout
snapshot, so the visual pass is the end-to-end check for what is drawn.

| Criterion | Tests | End to end |
|---|---|---|
| FR-001 keys act | core `every_row_with_an_action_resolves_to_it_while_the_review_has_the_keyboard`; `review_keys.rs` per key | Yes (reducer); visual pass B1–B11 |
| FR-002 keys elsewhere | `a_field_settings_the_overview_or_a_surface_on_top_has_the_keyboard`, `the_open_focused_comment_box_holds_the_keyboard`, `a_focused_terminal_keeps_every_review_key` | Yes; B13 |
| FR-003 j/k | `j_and_k_select_the_neighbour_file_and_wrap`, core `step_file`, `file_order_*` | Yes; B1 |
| FR-004 n/p | core `step_hunk`/`resolve_edge`, `review_keys.rs` n/p crossing, `moving_changes_nothing_but_the_selection_and_the_current_hunk` | Yes; B2, B3 |
| FR-005 mark visible | layout snapshot `review-current-hunk` | Visual pass B4 only |
| FR-006 ] [ | core `step_comment`/`comment_order`; `close_bracket_and_open_bracket_step_through_the_comments_and_wrap` | Yes; B7 |
| FR-007 v | `v_sends_the_mark_reviewed_edit_for_the_selected_file_only` | Yes; B5 |
| FR-008 g | core `g_*`; `g_selects_the_next_file_not_reviewed` | Yes; B6 |
| FR-009 c | core `comment_target`, `hunk_change_range`; `review_keys.rs` `c` tests | Yes; B8 |
| FR-010 send chord | `the_send_chord_sends_as_send_review_does`, `…_does_nothing_without_drafts_or_while_sending`, `…_in_the_comment_box_…` | Yes; B9, B10 |
| FR-011 not reserved | `the_send_chord_is_no_reserved_or_app_chord` | n/a (static) |
| FR-012 sheet | `review_keys.rs` sheet tests, `overlay_registry.rs` | Yes; B11 |
| FR-013 tooltips | `with_key_hint` tests, core `key_hint` | Yes; B12 |
| FR-014 shown tab only | `a_press_leaves_a_hidden_tab_alone` | Yes |
| FR-015 modifiers, IME, repeat | core `ctrl_alt_or_logo_…`, `shift_is_part_of_the_character_…`, dead-key/IME case, repeat rows; keymap `review_press` | Yes |
| FR-016 jump-to entries | core `review_candidates`; `quick_switch.rs` J4 position, rows | Yes; B14 |
| FR-017 matching | core J2/J3 ranking; `a_query_finds_a_review_file_by_name` | Yes; B14 |
| FR-018 choosing | `choosing_a_review_file_shows_the_tab_with_it_selected`, `choosing_a_review_comment_does_what_bracket_reaching_it_does` | Yes; B14 |
| FR-019 no tab | `quick_switch.rs` FR-019 guard | Yes |
| FR-020 user guide | `the_user_guide_names_every_review_key_and_its_text`, docs test | n/a (docs) |
| FR-021 snapshot, visual pass | layout snapshot states `review-current-hunk`, `review-shortcut-sheet`, `review-shortcut-sheet-single-keys-off` | Visual pass M3–M5 |
| FR-022 switch | core settings `review_single_keys_*`, `resolve`/`sheet_rows`/`key_hint` off; client switch-off tests; persist and settings tests | Yes; B16 |
| SC-001 30-file walk | `a_thirty_file_review_walked_by_key_is_sent_by_one_chord`, SC-001 `j`+`v` | Yes |
| SC-002 text fields | holder tests; composer `j` typed | Yes; B9, B13 |
| SC-003 ≤ 100 ms | core `every_rule_together_is_fast_on_a_large_review`; client `every_arm_is_fast_on_a_large_review` | Yes (harness) |
| SC-004 sheet lists all | core sheet tests; `the_user_guide_names_every_review_key_and_its_text` | Yes; B11, B16 |
| SC-005 200 files, 4 chars | core `review::jump` SC-005 test | Unit + B14 |
| SC-006 chord = button | `the_send_chord_sends_as_send_review_does` | Yes; B10 |

Untested criteria: none (FR-005 by the snapshot and visual pass only). Tests tracing to nothing:
none found.

## What was not audited

- No mutation tool: 12 deliberate mutants sampled the riskiest rules; not exhaustive.
- The smell pass covered the tests the feature added (core `review::{keys,navigate,jump}`, the
  `tree`/`diff`/`quick_switch`/`settings` additions, client `review_keys.rs`, the review tests of
  `quick_switch.rs`, `keymap.rs`, `features_composer.rs`, `overlay_registry.rs`), not the layout
  snapshot's 2,000 recorded lines, which were reviewed when re-recorded.
- macOS and Windows: graded only through CI's per-OS test runs; no manual pass (quickstart §C).
- Frame timing and animation: no criterion beyond SC-003's harness budget; not assessed.

## Close unit follow-up

- S1 fixed: the test captures `whole_file` before the two `n` presses and compares after, with
  messages on the layout and whole-file checks.
- Quickstart §C: added `a_focused_terminal_keeps_every_review_key` (a guard; no behavior added).
- Findings S2–S9 kept, reasons above. Finding 1 (test-first breach) is history and cannot be
  re-done; the five mutants on those behaviors (G, H, I, K, L) stand in as their retroactive red.
