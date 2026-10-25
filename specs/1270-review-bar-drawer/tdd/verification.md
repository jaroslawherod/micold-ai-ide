---
feature: 1270-review-bar-drawer
verdict: PASS_WITH_GAPS
standard: .specify/extensions/tdd/templates/tdd-test-quality-rubric.md # rubric graded against
verified_at: 5bf9f251 # M4 tip on `m4-1270` (not yet rebased); M1–M3 merged (#1369, #1382, #1402)
behaviors: 34
proven: 6
likely: 21
test_after: 0
no_test: 0
not_applicable: 7
high_smells: 0
criteria_total: 31 # FR-001–FR-024, SC-001–SC-007
criteria_covered: 31 # FR-024, SC-007 by docs/visual pass; FR-022, SC-005 latency by 946's push path
mutation_score: null # no mutation tool (profile: mutation null); 11 deliberate mutants, 11 caught
mutants_survived: 0
suite: 10987 passed, 0 failed, 18 ignored (scripts/gate.sh on the close tree, green); each mutant target re-run green after its restore
---

# TDD Verification: Review bar and comments drawer

**Verdict: PASS_WITH_GAPS.** No HIGH smell, no weakened existing test, every FR and SC traces to a
test or a recorded docs/visual deliverable, and every deliberate mutant was caught; the gaps are 21
behaviours whose only recorded red is a build error (`unresolved import`, `no variant`) or was
recorded after the fact by this audit's mutants, and no mutation tool beyond the sample.

The audit ran inline in the close unit (cloud run: forked skills run inline), a context that wrote
none of the tests; the smell pass and traceability went to a fresh headless `autopilot-reviewer`,
and every cited line was opened before it reached this report. The feature has no
`tdd/test-list.md`; tasks.md's test tasks and `tdd/cycle-log.md`'s rows are the list (as 1269).

## Test-first evidence

| Behaviour (cycle-log row) | Class | Evidence |
|---|---|---|
| M1 T001 modules, test files | NOT_APPLICABLE | structural |
| M1 T002/T003 anatomy tokens | LIKELY | build error only (`cannot find review_bar in anatomy`) |
| M1 T004/T006 `ScopeCounts`, `counts_text` | LIKELY | build error only (unresolved imports); mutants C1, C2 below |
| M1 T005/T007 `send_button`, `error_text` | LIKELY | same red run; mutant C1 |
| M1 T017/T018 `overflow` | LIKELY | same red run; mutant C2 |
| M1 T023/T025 `DrawerTab`, `tab_counts`, `rows_of`, SC-006 property | LIKELY | build error only |
| M1 T024/T026 `Drawer::apply`, `newly_reported` | LIKELY | build error only; mutants C3, C4, C5 |
| M1 T044/T045 `card_preview` | LIKELY | build error only; mutant C6 |
| M2 T046/T047 `LineClamped::when_cut` | LIKELY | build error only |
| M2 T008/T009 `ReviewBar` | PROVEN | `the_widest_label_fits_the_send_width`: `left: 176.0 right: 169.0` |
| M2 T027/T028 `SideSheet` | LIKELY | build error only |
| M2 T029/T030 row whole-reply mode | LIKELY | build error only |
| M2 T048/T049 card reply at 4 lines | PROVEN | `a_reply_over_four_lines_is_cut_there_and_offers_show_all` failed before the card change |
| M2 T010/T031/T050 showcase poses | NOT_APPLICABLE | structural; showcase gates green |
| M2 T052 layout snapshot unchanged | NOT_APPLICABLE | no covered state changed |
| M3 T011/T012 `review_bar` model, `SendFailed` | LIKELY | build error only (32 errors); mutant K2 |
| M3 T013/T014 `ReviewSend` failure routing | LIKELY | build error only (`PendingOp::ReviewSend` fields) |
| M3 T019/T021 overflow open/close | LIKELY | build error only |
| M3 T020 overflow popover registration | LIKELY | build error only |
| M3 T032/T034 drawer state per view | LIKELY | build error only |
| M3 T033/T035 Esc order | LIKELY | build error only |
| M3 T036/T037 `drawer_rows`, row jump | LIKELY | build error only |
| M3 T038/T039 send watch, auto-open | LIKELY | build error only; mutants C3, C4 |
| M3 T040 user guide | NOT_APPLICABLE | docs |
| M3 review A F1 (no daemon fails in the bar) | LIKELY | written with the fix, no red; red recorded by this audit, mutant K5 |
| M4 T015/T041 `review_bar_fixed` gate | PROVEN | 6 failed against M3's view (`lays out the bar at […]`) |
| M4 T016/T022/T042/T043 layout, overflow hang | PROVEN | same red run; mutant K4 |
| M4 T053/T054 covered states | PROVEN | `the_layout_matches_the_committed_fixture` failed; `containment` flagged the overhang |
| M4 T055 user guide | NOT_APPLICABLE | docs |
| M4 T056 visual pass | NOT_APPLICABLE | `visual-pass-m4.md` |
| M4 review A F1 `composer_in_drawer` | LIKELY | build error only; mutant K1 |
| M4 review B F1 `Effect::ScrollDrawer` | LIKELY | build error only; mutant K3 |
| M4 review A r2 F1 Edit / Answer… jump first | PROVEN | failed: `Edit jumped`, left `Some(RelPath("b.rs"))` |
| M4 T057 gate | NOT_APPLICABLE | gate run |

34 rows: 6 PROVEN, 21 LIKELY, 7 NOT_APPLICABLE.

**Existing tests.** The feature's removed assertions (`git show` of each `(1270)` commit) are the
M3 API swap in `features_changes.rs` and `features_pr_changes.rs` (`send_action`, `tidy_actions`,
`review_toolbar_label`, `panel_rows` → `review_bar`, `drawer_rows`), each with an equal or stricter
replacement; the panel's default flipped from open to closed (FR-011, named in the spec);
`review_toolbar_label == "Review (1)"` became `counts.draft == 1` because the label is gone
(FR-009). `anatomy_call_sites` lost three `NOT_BUILT` gap entries as the tokens got bound (the
exclusion list shrank). Nothing skipped, ignored or filtered.

**tasks.md vs evidence.** T001–T057 ticked, each with a cycle-log row or a docs / visual-pass /
gate deliverable.

## Findings

| # | Severity | Finding | Evidence | State |
|---|---|---|---|---|
| 1 | MED | `primary_follows_enabled_in_every_case` asserted `enabled` with the same rule `send_button` applies | `crates/micold-core/tests/review_bar.rs:174` `draft > 0 && !sending && sendable` | fixed in the close PR: the enabled cases spelled out (`(1 \| 1_000, false, true)`) |
| 2 | MED | The geometry gate checked the drawer's width against `drawer::width(area.width)`, so it agreed with the core function whatever it computed | `crates/micold-client/tests/gates/review_bar_fixed.rs:292` | fixed: the gate states its case — the area under twice `comments_drawer::WIDTH` at this window, the drawer half of it |
| 3 | LOW | The no-daemon send test accepted any snackbar | `crates/micold-client/src/shell/daemon_sync.rs:6622` `snackbar(&app).is_some()` | fixed: the bar's error and the snackbar's text are pinned (`Couldn't send the review: daemon Host unavailable`, and the route's message) |
| 4 | LOW | Several 1270 tests in `features_changes.rs` still cite feature 946's FR ids in their doc comments (the tests they replaced), so `grep FR-0xx` does not reach them | `features_changes.rs` around 2115, 2167, 2190, 2261, 2292 | left: the test names state the behaviour; this report's traceability table maps them |
| 5 | LOW | `the_trailing_group_never_moves` and `the_controls_emit_their_messages` check several reasons in one test | `ui/material/review_bar.rs:353`, `:448` | left: each assertion carries a message naming its case |

Declined smell-pass findings (vetted against the cited lines):

- *Implementation coupled* in the row tests' `.parts()` checks (`review_panel_row.rs`): `parts()`
  is the component's test seam used across `ui/material` (the profile's exemplar pattern), next to
  painted-text asserts.
- *Magic values* in `review_bar.rs:306` (`4.0 * spacing::SM`) and `SEND_WIDTH == widest.ceil()`:
  the token is defined as the widest label's width (research R4), so the equality is the rule.
- *Vacuous* `SEND_WIDTH > 0.0` in `tokens_anatomy.rs:280`: the file's pattern for every token; the
  value is pinned against the font by the client test.
- *Duplicated click helper* (`side_sheet.rs`, `review_bar.rs`), *unclear name*
  `a_reply_is_drawn_on_one_clamped_line` (pre-existing), and the 946 timing test's 100 ms budget
  (pre-existing, not a 1270 test): taste or out of scope.

Properties: the feature's tests are headless and deterministic (no sleeps, no network); the core
targets run in well under a second; `features_changes` (154 tests) in about 0.1 s.

## Mutation results

No `cargo-mutants` (profile: `mutation: null`); deliberate mutants on the behaviours the criteria
depend on, one at a time, each restored and its target re-run.

| Mutant | Behaviour | Caught by | Survived |
|---|---|---|---|
| C1 `bar.rs` 999+ cap `>` → `>=` | FR-003 | `drafts_that_can_be_sent_enable_the_primary_button` | No |
| C2 `bar.rs` Discard drafts… while sending | FR-007 | `discard_drafts_needs_drafts_and_no_send` | No |
| C3 `drawer.rs` auto-open ignores a hidden tab | FR-020 | `a_report_while_the_tab_is_not_shown_consumes_the_watch` | No |
| C4 `drawer.rs` any report hits the watch (another window's send) | FR-020 | `a_report_of_another_send_changes_nothing` | No |
| C5 `drawer.rs` `newly_reported` keeps already-reported comments | FR-020 | `a_report_replacing_a_report_is_not_new` | No |
| C6 `reply.rs` `CARD_LINES` 4 → 5 | FR-021 | `over_four_lines_is_cut`, `a_long_line_counts_the_lines_it_wraps_to` | No |
| K1 `features/changes.rs` `composer_in_drawer` ignores the tab | FR-019, review A M4 F1 | `a_drawer_composer_is_in_its_row_while_shown_and_on_its_card_otherwise` | No |
| K2 `features/changes.rs` `SendFailed` on the shown view, not the sender's | FR-006, R10 | `a_failed_send_shows_its_error_on_its_own_view_until_the_next_send_or_dismissal` | No |
| K3 `features/changes.rs` no `Effect::ScrollDrawer` when the list comes into view | FR-019, review B M4 F1 | `showing_the_drawers_list_scrolls_it_back_to_the_tabs_offset` (+2 1269 tests) | No |
| K4 `ui/changes.rs` diff area `FillPortion(5)` → `(3)` beside the list's 2 | SC-004 | `review_bar_fixed::with_the_drawer_closed_the_diff_spans_the_body_beside_the_file_list` | No |
| K5 `shell/changes.rs` an unsent send does not fail in the bar | FR-006, review A M3 F1 | `review_send::a_send_with_no_daemon_fails_in_the_bar_and_drops_the_watch` | No |

## Traceability

| Criterion | Tests | End to end |
|---|---|---|
| FR-001 bar above the status bar on a Review tab only | `review_bar_fixed` `the_bar_sits_directly_above_the_status_bar…`; `features_changes` `the_review_bar_shows_only_while_a_review_tab_is_shown` | Yes (layout) |
| FR-002 counts of the shown scope, longest fitting form | core `review_bar` counts tests; `ui::material::review_bar` `the_counts_take_the_longest_form_that_fits` | Yes |
| FR-003 Send review (n), 999+, fixed bounds | core `send_button` tests; `the_trailing_group_never_moves`; `review_bar_fixed` `send_review_keeps_its_bounds_*` | Yes |
| FR-004 send rules, PR not in list | core `a_pull_request_not_in_the_list_cannot_send`; `features_pr_changes` send tests | Yes |
| FR-005 Sending… while under way | core `a_send_under_way_reads_sending_and_is_disabled`; `features_changes` | Yes |
| FR-006 error in the bar, × | core `the_error_wording_is_todays`; `features_changes` `a_failed_send_shows_its_error_on_its_own_view…`; `daemon_sync` `review_send` (3) | Yes |
| FR-007 overflow items | core overflow tests; `features_changes` overflow tests; `overlay_registry` | Yes |
| FR-008 Comments toggles the drawer | `features_changes` `drawer_events_act_on_the_shown_views_drawer`; `the_controls_emit_their_messages` | Yes |
| FR-009 header without Review (n) / Send review | the removed APIs; `layout_snapshot` (header subtree of `session-area-review-shown`) | Yes (layout snapshot) |
| FR-010 drawer 360 / half, over the diff | `review_bar_fixed` drawer and SC-002 tests; `side_sheet`; core `width` | Yes |
| FR-011 closed at first | core `a_new_drawer_is_closed…`; `features_changes` | Yes |
| FR-012 × and Esc close | core escape tests; `overlay_dispatch_ordering`; `side_sheet` `close_emits_its_message` | Yes |
| FR-013 tab labels with counts | core `tab_labels_carry_their_count`, `tab_counts_cover_every_scope_of_the_entry` | Yes (core rule; drawn in covered states) |
| FR-014 row order | core `rows_are_ordered_…`; `features_pr_changes` `the_drawer_groups_the_working_tree_first…` | Yes |
| FR-015 draft rows, Edit / Delete | `review_panel_row`; `features_changes` `drawer_rows_split_by_tab_and_describe_their_comments` | Yes |
| FR-016 reply rows, whole reply, Mark applied / Answer… | `review_panel_row` whole-reply tests; `features_changes` | Yes |
| FR-017 row press jumps; line left of the drawer | `a_drawer_row_press_jumps_or_says_why_not`, `edit_or_answer_in_a_drawer_row_selects_the_comments_file`; `review_bar_fixed` drawer left edge | Yes |
| FR-018 empty texts, start hint | core `empty_tabs_say_so`; covered states `drawer-drafts`, `drawer-replies`; visual pass step 4 | Partly (text rule asserted, drawn state by snapshot/visual pass) |
| FR-019 drawer and composer survive; scroll restored | `the_drawer_and_its_composer_text_survive_switching_and_go_with_the_tab`, `showing_the_drawers_list_scrolls_it_back_to_the_tabs_offset`, `a_drawer_composer_is_in_its_row…` | Yes |
| FR-020 auto-open once on the first report | core watch tests (5); `features_changes` report tests (4) | Yes |
| FR-021 card reply at 4 lines, Show all | `reply` `card_preview` tests; `review_comment` (3); `line_clamp` `when_cut` | Yes |
| FR-022 replies replace on push | `features_changes` push tests (`a_push_that_removes_the_focused_comment…`) | Yes (state); latency is 946's push path |
| FR-023 rules in the render-free core | core `review_bar`, `review_drawer` | Yes |
| FR-024 docs, snapshot, showcase | user guide, `layout_snapshot`, showcase gates, visual passes | Docs |
| SC-001 Send review never moves | `review_bar_fixed` `send_review_keeps_its_bounds_*` (2); `the_trailing_group_never_moves` | Yes |
| SC-002 drawer moves no diff line | `review_bar_fixed` `opening_the_drawer_neither_resizes_nor_moves_the_diff` | Yes |
| SC-003 Esc order | `overlay_dispatch_ordering` `escape_closes_what_is_above_the_drawer_then_the_drawer` | Yes |
| SC-004 diff spans the old diff + review column | `review_bar_fixed` `with_the_drawer_closed_the_diff_spans_the_body_beside_the_file_list` | Yes |
| SC-005 a reply shows within 2 s | push replacement as FR-022 | State only; the 2 s path is 946's, not timed here |
| SC-006 every comment in exactly one tab | core `every_comment_is_in_exactly_one_tab`; `drawer_rows_split_by_tab…` | Yes |
| SC-007 recorded visual pass | `visual-pass-m2.md`, `-m3.md`, `-m4.md` | Docs |

Untested criteria: none. Tests tracing to nothing: none among the feature's tests (finding 4 is
about ids in comments, not missing traces).

## What was not audited

- Mutation strength beyond the 11 deliberate mutants: no `cargo-mutants`; coverage unavailable, so
  untested branches in the changed files are unknown.
- The rebased tip: the suite ran on `m4-1270` before its rebase onto `main`; the orchestrator
  re-runs the full gate there.
- `ui/changes.rs` drawing glue beyond the geometry gate: covered by layout states and the visual
  passes, not by assertion-level tests. Quickstart §B steps 5–9 by hand and §C (macOS, Windows):
  not run (see quickstart Results).
- SC-005's 2 s cross-window latency: rides on feature 946's push path; not timed.
