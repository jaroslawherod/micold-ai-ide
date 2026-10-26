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
