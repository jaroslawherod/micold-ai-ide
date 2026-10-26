# TDD cycle log: 1274 M1

Red runs are `scripts/build-lock.sh cargo test -p micold-core --lib -- <filter>`; each red was
observed before the implementation it names.

| Task | Behaviour | Red evidence | Green |
|---|---|---|---|
| T001 | `review::{keys, navigate, jump}` declared, module docs | scaffolding, no test | builds |
| T002 → T003 | K1 table: 12 rows in order with K1's key, mods, action, repeats, sheet text; no shared `(key, mods)`; only save-comment has no action; only `j k n p ] [` repeat; K4 labels both forms | `-- review::keys` against tests only: `error[E0425]: cannot find value REVIEW_KEYS in this scope` (×7), `E0433 cannot find type KeyName` / `ReviewAction` (compile-fail) | after T003, same command: the 4 table tests pass while `resolve` is a stub (`6 passed; 7 failed`) |
| T004 → T005 | K2/K3 `resolve`: every action row under `Review`; `Other` none; `Sheet` only fresh unmodified `?`; `Composer` only a fresh send chord; Ctrl/Alt/logo refuse a single key, AltGr (Ctrl+Alt, no logo) accepts `[`; Shift ignored for chars, `J` unbound; no text → none; repeat only for steppers; chord needs primary+Shift, refuses Alt and Ctrl+logo, primary+Enter alone → none | `-- review::keys` with `resolve` returning `None`: `test result: FAILED. 6 passed; 7 failed` (e.g. `every_row_with_an_action_resolves_to_it_while_the_review_has_the_keyboard` panicked at keys.rs:352) | same command: `13 passed; 0 failed` |
| T048 → T049 | J2/J3: `detail: None` ranks as 968; a name match sorts before any detail-only match whatever its rank; detail matched only when the name misses; spans index the matched text | `-- review:: quick_switch` against tests only: `E0433 cannot find type Via` (×5), `E0560 Candidate has no field named detail`, `E0609 no field via on Ranked`, `E0599 no variant ReviewFile` (compile-fail) | after T049: every `quick_switch` test green, 968's included (their fixtures gained `detail: None` / `via: Via::Name` only) |
| T050 → T051 | J1 `review_candidates`: file name / full path / `ReviewFile`; comment text joined by spaces / its path / `ReviewComment`; files then comments in comment order; `ReviewEntry` reviewed state or status, path, line, first text line; SC-005 on 200 files: 4 typed characters rank the file in the first 3 rows | compile-fail as above (`E0425 cannot find function review_candidates`, `E0433 ReviewEntry`); then with a `Vec::new()` stub: all 3 `review::jump` tests FAILED | `-- review:: quick_switch`: `311 passed; 0 failed` |
| T006 → T007 | `has_hunks` (text with changes; binary, not UTF-8, pure rename, large not shown → none; large shown → hunks); `step_file` US1 s1, s2, s5, no / unlisted selection, empty | stubs (`false`, `None`): `j_and_k_step_through_the_file_order_and_wrap`, `a_text_file_with_changed_lines_has_hunks_and_the_others_do_not` and 2 more FAILED (run total `285 passed; 26 failed`) | `311 passed; 0 failed` |
| T008 → T009 | `step_hunk` / `resolve_edge` (R5): no current hunk → first / last; inside → ±1; crossing to the next / previous file with hunks with `First` / `Last`, files without hunks passed over, wrapping; wrapping back to the selected file → `Current`; loading / failed → `Nothing`; no selection → first / last file with hunks; none with hunks → `Nothing`; `resolve_edge` 0, n−1, none for 0 | stubs (`HunkMove::Nothing`, `None`): 7 `step_hunk` / edge tests FAILED in the same run | `311 passed; 0 failed` |
| T010 → T011 | `header_rows`: header rows `[0, 6]` unified and `[0, 5]` side by side for TWO_HUNKS with its hidden run; none for no hunks | stub `Vec::new()`: `header_rows_name_each_hunk_headers_row_in_both_layouts ... FAILED` | `311 passed; 0 failed` |
| T022 → T023 | `next_unreviewed`: US2 s3 (2, 4, 2), s4 `AllReviewed`, s5 predicate read at the call, no selection, selected only one left, empty → `Nothing`; `ALL_FILES_REVIEWED` | stub `Unreviewed::Nothing`: 4 `g_*` tests FAILED | `311 passed; 0 failed` |
| T028 → T029 | `comment_order` (file index, line, created, id; unlisted last by path); `step_comment` US3 s1 (select `c.rs` on the third, wrap), s2, s3 drawer, unlisted file → drawer without select, no comments, gone `last` | stubs (input order, `None`): 4 comment tests FAILED | `311 passed; 0 failed` |
| T030 → T031 | `hunk_change_range` (added → new side; remove-only → old side; neither → none); `comment_target` precedence (US3 s4–s6, FR-009); `PICK_OR_HUNK` | stubs (`None`, `Refused`): 2 tests FAILED | `311 passed; 0 failed` |
| T032 | SC-003: `step_file`, `step_hunk`, `comment_order` + `step_comment`, `next_unreviewed` on 200 files / 500 comments under 50 ms | against the stubs it FAILED on its result assertions (`every_rule_together_is_fast_on_a_large_review`), not on time | `311 passed; 0 failed` |

Refactor: clippy's `double_ended_iterator_last` (`next_back`), a `Row` alias for K1's tuple, a
`one_span` helper for span assertions; no behaviour change, suite re-run green.

Review A fix (M1): `step_hunk` with a stale current hunk past a re-read, shorter diff — `p` now
takes the file's last hunk instead of crossing files. Red: `-- shrunken`:
`left: File { path: RelPath("a.rs"), edge: Last }, right: Current(1)` (FAILED); green after
`i.min(count).checked_sub(1)`: `-- review:: quick_switch` `312 passed; 0 failed`.

### M2 — single-key switch, core (T059–T062)

| Tasks | Behaviour | Red | Green |
|---|---|---|---|
| T059 → T060 | `Settings.review_single_keys`: default on; a file without the key loads on; `false` written as `"review_single_keys": false` and loaded back; `settings_issue_mapping` field list | accepted red: a compile failure (tests written before the field; no failing assertion was run): the tests did not compile (no field `review_single_keys` on `Settings`) | `review_single_keys_default_on ... ok`, `review_single_keys_off_round_trips ... ok` (`cargo test -p micold-core --all-targets`) |
| T061 → T062 | `resolve(.., single_keys)`: off → no single row under any holder, chord still `Send` under `Review` and `Composer`; `is_single` for the ten chars; `sheet_rows` on 12 rows / off the two chords; `key_hint` per action and switch; `SINGLE_KEYS_OFF_NOTE` | accepted red: a compile failure (tests written before the code; no failing assertion was run): `resolve` took two arguments and `sheet_rows`, `key_hint`, `is_single`, `SINGLE_KEYS_OFF_NOTE` did not exist (compile errors) | 5 new `review::keys` tests ok, the 20 existing calls pass `true` unchanged |

Refactor after review A (M2): one `ReviewKey::is_on(single_keys)` predicate for `resolve`, `sheet_rows` and `key_hint`; persist test `a_form_save_keeps_a_stored_single_key_switch_off` pins that the T060 placeholder never overwrites a stored `false`.

### M3 — move and mark (T012–T021, T024–T027, T063–T065)

| Tasks | Behaviour | Red | Green |
|---|---|---|---|
| T013 → T014 | K5 `primary_held` (`cfg!` arms), `review_press`: produced char, Enter, `Other` for no text / control char / two chars; every modifier and `repeat` copied | accepted red: written with the code in one step (a compile failure before `review_press` existed; no failing assertion was run) | `--test keymap`: `50 passed; 0 failed` |
| T015 → T016 | holder: `Review` shown and nothing else; `Composer` open + `FieldId::ReviewComposer`; a closed box gives it back; `Other` for the tree filter, any field, Settings, Overview, quick switcher, a dialog; nothing shown | stubbed `review_key` (`return Effect::None`) run: the holder tests passed (holder not stubbed; accepted) | `--test review_keys`: `15 passed; 0 failed` |
| T017 → T018 | K7 `j`/`k` (wrap, diff from top, collapsed folder revealed, row scrolled into view, filter), `n`/`p` (current hunk + header offset, crossing with `pending_edge` resolved only by the current read, `p` to the last), `Nothing`, picks/layout kept, hidden tab untouched, SC-003 200 files / 500 comments < 100 ms; core `tree::file_order` (folds ignored, filter, sort) | `review_key` stubbed: `test result: FAILED. 5 passed; 10 failed`; `file_order` stubbed `Vec::new()`: `file_order_ignores_folds_and_keeps_the_filter_and_sort ... FAILED` | `15 passed; 0 failed`; `-- file_order`: `2 passed` |
| T024 → T025 | `v` = **Mark reviewed**'s edit for the selected file only, unmark; `g` 2, 4, 2, another window's mark skipped, `AllReviewed` note; SC-001 30 files `j`+`v` | same stubbed run (`v_sends…`, `g_selects…`, `j_then_v…` FAILED) | `15 passed; 0 failed` |
| T063 → T064 | switch off: `j n v k p g` presses `Effect::None`, state unchanged, tooltips name no key; on: `j` acts, "Next change (n)", "… (v)"; settings draft/save/cancel; persist keeps other fields; Save applies at once (`main_tests`) | same stubbed run (`with_the_switch_off…` FAILED); settings/persist tests written with the code (accepted red) | in the gate |

### M4 — comments, send chord and shortcut sheet (T033–T046)

Red: one run of `scripts/build-lock.sh cargo test -p micold-client --test review_keys --test
overlay_registry --test keymap --test features_composer --no-fail-fast` against the new tests, with
the reducer arms, `composer_press`, `shortcut_sheet` and the holder's `Sheet` stubbed (`Effect::None`
/ `None`), and `TextArea`'s `.on_press` stored but never consulted and its submit still taking Shift.

| Task | Behaviour | Red evidence | Green |
|---|---|---|---|
| T035 → T036 | `TextArea`: Ctrl/Cmd+Enter saves, with Shift it does not; `.on_press` claims presses before the bindings, unclaimed ones keep the editor's keys and the save (driven through the real widget: click to focus, then key events) | `features_composer`: `primary_enter_saves_and_primary_shift_enter_does_not`, `on_press_sends_its_message_for_each_press_it_claims` FAILED (`29 passed; 2 failed`) | `31 passed; 0 failed` |
| T037 | FR-011: the send chord's labels are no 803 FR-006 reserved chord, no app chord label, and encode to terminal bytes | a guard, not a red: the chord was already free, so it passed on the first run; it fails if a later change moves `Send` onto a reserved or app chord | — |
| T033 → T034 | `]`/`[` in comment order with select, card jump (drawer focus + pending jump), drawer for outdated / unlisted, wrap, `last_comment`; `c` picked / hunk / refused / focus composer | `review_keys`: 5 tests FAILED (e.g. `close_bracket_and_open_bracket_step_through_the_comments_and_wrap` at review_keys.rs:739) | `29 passed; 0 failed` |
| T038 → T039 | `Send` = `SendPressed` (s1, SC-006), nothing without drafts / while sending (s2–s3), composer chord sends saved drafts and keeps the box (s4), 30-file walk + one chord (SC-001), switch off still sends and names it (FR-022) | 4 tests FAILED (`the_send_chord_sends_as_send_review_does` …) | same run green |
| T041 → T042, T043 | sheet: `?` opens/closes, holder `Sheet`, `j` inert, Escape closes, overflow item opens it, switch off: `?` inert, rows = `sheet_rows(false)` + note; registered dialog (`review_shortcut_sheet`, Dialog, Escape / outside click → `ShortcutsClosed`, drawn from the registry) | `review_keys`: 3 sheet tests FAILED; `overlay_registry`: `the_reducer_opens_a_dialog_through_that_mechanism`, `an_agent_prompt_waits_behind_the_open_dialog_rather_than_closing_it` FAILED (the root did not route `ShortcutsClosed`) | `overlay_registry` `26 passed`; `surface_registration_cost` `10 passed` |
| T044 → T045 | SC-004: *Reviewing from the keyboard* names every `REVIEW_KEYS` label (both forms, in backticks) and sheet text | `keymap`: `the_user_guide_names_every_review_key_and_its_text` FAILED (`the section says "Next file"`) | `keymap` `52 passed` after the docs |

Refactor: `platform_label` shared by the tooltips and the sheet; `sheet_model` shared by the dialog
and the showcase. A fixture fix: US3 s1's line 20 became line 5, since the 12-line fixture made a
line-20 quote outdated (a drawer jump, the next test's case).

### M5 — jump to review files and comments (T052–T056)

Red: `cargo test -p micold-core --lib review::navigate` with `comment_move` stubbed to `None`, and
`cargo test -p micold-client --test quick_switch --test review_keys --no-fail-fast` with the API in
place (`Step::Review`, `ReviewJump`, `Msg::Jumped`, `Kind::Review*`) but `candidates` not yet
inserting the review entries and `view_of` not yet moving detail spans.

| Task | Behaviour | Red evidence | Green |
|---|---|---|---|
| T054 (core) | `comment_move` = `]` reaching one id (select, card / drawer), `None` for an id not listed; `step_comment` delegates to it | `a_chosen_comment_moves_as_bracket_reaching_it_does` FAILED (`20 passed; 5 failed` — the stub broke `step_comment` too) | in the gate |
| T053 → T054 | J4 position (after locations and sessions, before projects; hidden tab too), query by name, choosing a file (tab shown, file selected, session / location / project / session count unchanged), choosing a comment = `]`'s view | `quick_switch`: 5 FAILED (`51 passed; 5 failed`), e.g. `review_entries_follow_the_projects_locations_and_sessions_before_other_projects` | `56 passed; 0 failed`; `review_keys` `29 passed` |
| T053 | FR-019 no tab → no entries, others unchanged; a key no longer listed does nothing | a guard: passed on the red run (nothing listed yet); fails if review entries leak to another entry | — |
| T055 | file row `File · <path> · reviewed|not reviewed`, comment row first line + `Comment · <path>:<line> · <status>`, a path-only match emphasised on the second line | `review_rows_show_the_reviewed_state_status_and_line` FAILED | green; a first green run found `reviewed_paths` empty unless sorted Unreviewed first — switched to `file_reviews` (as `g`) |

Refactor: `step_comments` and the comment jump share `reach_comment`; the Review tab fixtures
(`path`, `file`, `id`, `whole`, `open_into`, `marked`, `draft`) moved to
`tests/support/review_fixture.rs`, used by `review_keys` and `quick_switch`.
