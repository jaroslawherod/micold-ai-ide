# TDD cycle log — 482 worktree changes review (M1)

No `tdd/test-list.md` was derived at design time; as in 575 and 582, the test-first task pairs of
tasks.md are the test list. Each red was run against the stubs of commit `959df6e6` (they compile
and return the "nothing" answer: empty lists, `None`, zero limits, a nil id) and failed on its
assertion, not on a build error.

| Behaviour | Tasks | Red (stub) | Green |
|---|---|---|---|
| `RelPath`, `LineRange`, `limits` | T003 → T004 | `review::tests`: 5 of 5 FAILED (e.g. `a_line_range_refuses_line_zero_and_a_reversed_range`, `the_limits_are_those_of_research_r8` left `0` right `5000`) | `cargo test -p micold-core --lib review` ok |
| Comment JSON shape, round trip, refused reversed range, v4 ids | T005 → T006 | `review::comment::tests`: 4 of 4 FAILED (stub derives: `"state": "Pending"`, `range` nested, nil id) | ok |
| Default branch, toggles, `DiffRange::for_view` | T011 → T016 | `review::base::tests`: 5 of 5 FAILED (stub `None`, toggles off) | ok |
| numstat / raw parsers, assemble, untracked merge, content, large | T012 → T017 | `review::changes::tests`: 8 of 8 FAILED (stub empty vectors, `large` always false) | ok |
| Real-git lists and base | T013 → T018 | `review_git`: 8 of 8 FAILED at their assertion (e.g. `the_base_is_the_merge_base_with_local_main` left `Unavailable(NoDefaultBranch)`; `committed_uncommitted_and_both_lists_follow_the_toggles` left `[]`) | 8 passed |
| Wire delta v30, `diff_layout` default | T007 → T008 | `protocol_roundtrip`/`settings_roundtrip`: did not compile (no `ReviewEdit`, `ReviewEditOp`, `DiffLayout`); after the wire edit one red left: postcard refused `#[serde(flatten)]` on `ReviewComment` (no `deserialize_any`), fixed by the `StoredComment` repr | `protocol_roundtrip`, `settings_roundtrip`, `schema_hash` ok |
| Daemon stores and pushes `diff_layout`; review messages refused | T009 (test-after) → T010 | none: T010's serving was written before T009 (handover), so the test passed on first run; recorded as test-after | `diff_layout_setting`: 3 passed |
| Changes view list state (open, toggles, seq, again, selection, V2, V3, L1, L3) | T014 → T020 (green: 13/13) | `features_changes` on the stubbed `features::changes` (`update` → `Effect::None`, empty view-model): 13 of 13 FAILED at their assertions | `features_changes` 13 passed |
| `VirtualRows` windowing | T015 → T019 (green: 6/6) | `virtual_rows::tests` on the stubbed `visible_range` (`0..len`) and `spacers` (`(0, 0)`): 4 of 6 FAILED; the empty-list and total-height cases pass on the stub by construction | `virtual_rows` 6 passed |
| Review changes opens the view (V1) | T022 test → T022 | `features_sidebar`/`app_state` `review_changes*`: did not compile (no `sidebar::review_changes`, `Outcome::ChangesRequested`, `SidebarMsg::ReviewChangesRequested`, `State::changes`, `take_changes_effect`); green: `review_changes` 1 passed in `features_sidebar`, 1 in `app_state` |

## M2 — unified diff (T027–T040)

Reds run against stubs that compile and answer "nothing" (`FileDiff::Text(vec![])`, `None`, empty
rows), each failing at its assertion.

| Behaviour | Tasks | Red (stub) | Green |
|---|---|---|---|
| `parse_unified` (headers, both numbers, CRLF, ending-only pair, no-newline marker, binary, not UTF-8, mode only), `added_file`, `unified_rows`, `SideLines` | T027 → T032 | `review::diff::tests`: 13 of 13 FAILED (e.g. `a_binary_notice_gives_binary` left `Text([])`) | `cargo test -p micold-core --lib review::diff`: 13 passed (one fixture fix on the way: `\`-continuation had eaten the context lines' leading space) |
| `GitCli::file_diff`: committed / uncommitted / both, rename with old path, deleted, untracked, binary, LF → CRLF, 6,000 lines `TooLarge` unless forced | T028 → T033 | `review_git`: 6 new of 14 FAILED at their assertions (stub `Text([])`), the 8 M1 cases ok | `review_git`: 14 passed |
| `diff_added`/`diff_removed` tints: listed in the CSS export, text and line numbers at 4.5:1 on both in both schemes | T031 → T034 | css `every_colour_role_is_emitted_as_a_hex_triple` FAILED (roles not emitted); `composition_contrast::diff_line_text_is_legible_on_the_added_and_removed_tints` FAILED, 8 violations (stub tints = `on_surface`: 1.00:1) | `micold-core --lib tokens` 12 passed; `composition_contrast` 5 passed |
| `DiffView` unified: number cells and text aligned across rows, tint roles per kind, 50,000 lines build only the visible rows, D3 messages and the D4 large gate with no rows | T030 → T035 | `diff_view::tests`: 5 of 5 FAILED (stub `todo!()` panics at `body`/`tint`/`line_row`) | `cargo test -p micold-client --lib diff_view`: 5 passed |
| Diff half of the view: select → `ReadDiff` (scope, toggles, old path of a rename), stale answer dropped, `TooLarge` → Show diff → forced read and `shown_large`, D3 answers allow no pick, list re-read keeps the selection and re-reads its diff (old diff kept on screen) or drops it | T029 → T036 | `features_changes`: 7 new of 20 FAILED (stub: select answers `Effect::None`, `diff_body` `NoSelection`) | `cargo test -p micold-client --test features_changes`: 20 passed |
