---
feature: 1122-overview-card-issue-link
verdict: PASS_WITH_GAPS
standard: .specify/extensions/tdd/templates/tdd-test-quality-rubric.md
verified_at: ca4364d8
behaviors: 41 # cycle-log rows (tasks.md is the list; no tdd/test-list.md)
proven: 35
likely: 0
test_after: 0
no_test: 0
not_applicable: 6
high_smells: 0
criteria_total: 24 # FR-001..FR-017, SC-001..SC-007 read as one set
criteria_covered: 23 # FR-013 not traced to a test
mutation_score: unmeasured # no cargo-mutants; 21 deliberate mutants, scope below
mutants_survived: 4 # B1, B6 (same guard), B2, B5 in the jira filter; all triaged (B1/B6 caught by a sibling test, B2 and B5 equivalent)
suite: scoped runs green after restore (core lib 14+51+31, settings/protocol roundtrip 41+42, daemon jira_base_url_setting 6, client lib text_link 17 / overview_card 69 / control_overlay 7 / card_grid 55 / settings 26 / keyboard_focus 17, overview 75, features_settings 139, bin 662); full workspace suite not run
---

# TDD Verification: Overview cards, a link to the worktree's issue

**Verdict: PASS_WITH_GAPS.** Every milestone has a recorded red, the history agrees with the log, and every non-equivalent mutant on the sampled high-risk behaviours was caught by some test (17 of 21 by the first filter run; B1/B6 by a sibling test, B2/B5 equivalent); the gaps are the missing `test-list.md`, no mutation tool or coverage, and one JiraBaseUrl-specific refusal row that is pinned only through a sibling test.

The audit was independent: this session did not write the tests. Every file cited was re-read at HEAD `ca4364d8` (the worktree is `origin/main` fd2a64ef plus spec-only close commits).

## Test-first evidence

The feature has no `tdd/test-list.md` (the cycle log says so, as for 1121 and 1120); `tasks.md` (52 tasks, all `[X]`, none unticked) is the list. Each cycle-log row names the task pair it covers. The log's rows are the behaviours counted above.

History: the 68 `(1122)` commits are one feature commit per milestone (`4ce9e05d` M1, `245d86e7` M2, `508d7670` M3, `02699b68` M4, `989c4944` M5) plus fix commits, and each adds tests and source together. Per the rubric that is `PROVEN` when the log carries the red, and it does for every behavioural row (the red is a compile error, a stub's failing assertion, or a mutant of the new code, with the output quoted). Ordering inside a commit cannot be verified from history; the log is the only source for it. No commit lands source before its test.

| Rows | Class | Evidence |
|---|---|---|
| M1 T001-T008 and review A F1-F3 (10 rows) | PROVEN | stub or mutant reds quoted (`jira::tests` 3 failed, `issue_link` 8 failed, `text_link` 11 failed); `4ce9e05d`, `203decba`, `246584b2` |
| M2 T010-T018 and review F1-F2 (8 rows) | PROVEN | `control_overlay` 5 failed, `overview_card` 9 failed, `card_grid` 3 failed; `245d86e7`, `714b6604` |
| M3 T020-T028 (4 rows) | PROVEN | `LinkRested` stub, `overview` 2 failed, bin 4 failed, `step_card` stub 3 failed; `508d7670` |
| M4 T031-T036, review A F1-F3, visual r1 (9 rows) | PROVEN | 14 compile errors then 4 mutant failures; `a_blur_under_the_link_menu...` etc.; `02699b68`, `c8fe44e9` |
| M5 T039-T048, review A F2, visual obs. 1 (7 rows) | PROVEN | compile errors then `jira_base_url_setting` 5 failed with a server-arm mutant; `989c4944`, `5b189fb0` |
| T009 (contrast), T029, T037, T049 (docs), M4 review F4 (refactor), M5 review F3 (wording) | NOT_APPLICABLE | guard by design, docs, refactor under existing tests, wording |

Guards that pass at red are labelled as such in the log (T003/T004 "no link" rows, T032 scroll row, T041 `a_stored_entry_that_does_not_parse_is_never_sent`); each was either mutated here (A3, A4, B3) or rests on a lenient read that the roundtrip tests pin.

Existing tests: the removed assertion lines in feature commits (`git show <c> -U0`, tests and `main_tests.rs`) were each opened. None is a weakening:
- `main_tests.rs` `with_the_switch_off_opening_the_overview_reads_no_title` (`508d7670`): `remote_lists_sent(..).is_empty()` ("no remotes asked") became `len() == 1` ("the repository is asked for") plus a title-free answer. A deliberate requirement change (R2: the repository is read for the link even with the title switch off); the "no title recorded, no gh lookup" assertions below it stay.
- `overview_card.rs` `a_plain_card_does_not_ripple` (`245d86e7`): `assert_ne!(tree.tag, Tag::of::<RippleState>())` became `ripple_tree(&tree).is_none()`. Same strength; the ripple now sits one widget deeper (log: "existing ripple tests now find the ripple's tree wherever the card puts it").
- `ui/mod.rs` arrow tests moved to their own module (`b533ad30`) and extended for `CardStepped`; `protocol_roundtrip.rs` `PROTOCOL_VERSION` 59 became 60 with the schema-hash pin (`989c4944`); `focus_return` assertions rewritten to `FocusReturn { location, to_link }` (`c8fe44e9`, same values asserted). `jira.rs` `SiteOrigin::parse` was refactored onto `WebAddress::split`, and its tests (`jira_connection`, `jira::tests`) are unchanged and still green.
- No `#[ignore]`, skip, filter-exclusion, threshold or snapshot-deletion in the range. Snapshot re-blesses are described in the log with the diff scope (added nodes or states only, M5 widened five `settings-view-jira-on-*` states, which T046 and review F3 explain).

## Findings

| # | Severity | Finding | Evidence |
|---|---|---|---|
| F1 | LOW | No `JiraBaseUrl` refusal row has a fragment after a path (`https://host/jira#frag`). The `#` guard in `WebAddress::split` is shared with `SiteOrigin` and is pinned only by `jira_connection.rs:39` (`https://acme.atlassian.net/#frag`). A change that makes `JiraBaseUrl` stop sharing the guard would pass its own refusal table. | mutant B1 (`rest.contains(['?','#'])` to `['?']`) survives `--lib jira` (31 passed), fails `--test jira_connection` (`a_site_that_is_not_an_https_origin_is_refused`) |
| F2 | LOW | Process: the profile has no mutation or coverage tool, so test strength rests on the sampled mutants below. | `tdd-profile.md` `mutation: null`, `coverage: null` |

No HIGH or MED smell found. `tests/overview.rs:1688` loops three Default/no-ticket/Jira cases under one assertion pair with the case in the message (accepted; D1 shows it pins Default). Read for the catalogue: `issue_link.rs` tests (table-driven, specific values, no conditional logic), `jira.rs` `JiraBaseUrl` tests (exact accepted and refused forms), `text_link.rs` / `control_overlay.rs` / `card_grid.rs` / `overview_card.rs` tests (event-level, assert the published messages in order), `tests/overview.rs` `mod issue_link` and `mod jira_base_url_form`, `jira_base_url_setting.rs` (real service over the support harness, no doubles of the unit). Loops over `cases` carry the case in the assertion message. Time: the `Instant::now()` uses in the diff belong to other features' commits; none is in a 1122 test. No secrets in fixtures (`example.atlassian.net`, `o/r`).

Properties: the scoped suites above run in under 15 s each after build; no sleeps; no shared state beyond the support harness; failure names the rule (`left`/`right` plus a message).

## Mutation results

No `cargo-mutants`: deliberate mutants, one at a time, each restored with `git checkout -- <file>` and checked with `git diff --quiet` (all clean). After the last restore every filter below was re-run green and `git status --short` is empty apart from this report.

| Mutant | Behaviour (spec) | Caught by | Judgment |
|---|---|---|---|
| A1 `issue_link.rs:57` Jira key left as written (not upper-cased) | FR-002 | 4 `issue_link` tests (`a_lower_case_jira_key_is_written_upper_case`...) | caught |
| A2 `issue_link.rs:89` `/browse/` to `browse/` | FR-004, SC-002 | `a_jira_address_has_exactly_one_slash_before_browse`, `a_jira_key_links_to_its_browse_page...` | caught |
| A3 `issue_link.rs:87` Jira switch ignored | FR-001 | `a_jira_key_has_no_link_without_a_base_url_or_with_the_switch_off` | caught |
| A4 `issue_link.rs:76` GitHub switch ignored | FR-001 | `a_github_ticket_has_no_link_without_a_repository_or_with_the_switch_off` | caught |
| A5 `issue_link.rs:78` label `Issue #N` to `Issue N` | FR-002 | 2 tests | caught |
| B1 `jira.rs:102` fragment guard dropped | FR-011 | not by `--lib jira`; by `jira_connection` | survivor in the jira filter, caught by a sibling test (F1) |
| B2 `jira.rs:107` user-info guard dropped | FR-011 | none | equivalent: `user@host` still fails the host-label check (letters, digits, hyphens) |
| B3 `jira.rs:176` `https`-only check dropped | FR-011 | 2 tests | caught |
| B4 `jira.rs:182` trailing `/` kept | FR-011, FR-004 | 3 tests (jira, issue_link) | caught |
| B5 `jira.rs:106` empty-authority guard dropped | FR-011 | none | equivalent: an empty host fails the label check |
| B6 `jira.rs:102` fragment guard to a never-matching char | FR-011 | same as B1 | as B1 |
| C1 `control_overlay.rs:131` captured event still reaches the control | Story 4, FR-005 | `an_event_the_content_captures_never_reaches_the_control` | caught |
| C2 `text_link.rs:367` release off the link still clicks | FR-005, FR-006 | `a_press_on_it_released_off_it_publishes_nothing` | caught |
| C3 `overview_expansion.rs` `LinkRested` also expands | FR-007 | 3 tests | caught |
| C4 `main.rs` `link_focus_after` `was != is` to `==` | FR-008, M4 | `escape_on_the_link_keeps_the_keyboard_on_it`, `shift_tab_onto_the_previous_cards_link...`, `link_focus_moves_to_card_when_link_goes` | caught |
| C5 `ui/overview.rs` base URL taken from any project | FR-010, Story 2 sc. 8 | `two_projects_each_link_to_their_own_site`, `no_jira_link_with_the_switch_off_or_the_projects_base_unset` | caught |
| C6 `daemon_sync.rs` read-only switch no longer reads the repository | FR-001 (read-only window) | `repo_reread_on_a_read_only_switch` | caught |
| C7 `catalog.rs:157` base URL set clears the other projects | FR-010 | `two_clients_setting_different_projects_both_survive`, `a_clear_removes_only_its_project_and_none_changes_nothing` | caught |
| C8 `ui/overview.rs` GitHub switch forced on | FR-001, FR-012 | `no_github_issue_link_with_the_switch_off_or_no_repository_known` | caught |
| C9 `card_grid.rs` a press on a link also reaches the other cards | FR-005, U5 | `a_click_on_a_link_leaves_the_keyboard_with_the_card_that_holds_it` | caught |
| D1 `ui/overview.rs` Default card treated as a worktree `feat-1122_x` | FR-001 | `no_issue_link_on_default_without_a_ticket_or_for_a_jira_key` | caught (a first attempt with an invalid name `1122-x` survived because that name has no ticket; discarded) |

Sampled: 8 behaviours from the brief (ticket from the worktree name and none on Default; Jira address and absence while unset; `JiraBaseUrl::parse` refusals and trailing `/`; press on the link never activates the card and press/release split; resting on a link does not expand; arrows and `link_focus_after`; per-project base URL and switches; the read-only switch repo read), 21 mutants in all. Not exhaustive. Score: 17 caught of 21 mutants run through their listed filter (B1, B6, B2, B5 survive the `--lib jira` filter; B1/B6 are one guard, caught by `jira_connection`; B2/B5 equivalent); no tool, so no comparable score.

## Traceability

Real-entry-point tests: `tests/overview.rs` (state to cards), `tests/features_settings.rs` `mod jira_base_url_form`, `tests/settings_sections.rs`, `jira_base_url_setting.rs` (daemon service over the wire), `main_tests.rs` (root `update`/`link_focus_after`), widget tests that drive events through a mounted tree.

| Criterion | Tests | End to end |
|---|---|---|
| FR-001, FR-002 | `issue_link::tests` (14), `tests/overview.rs` `mod issue_link` | Yes |
| FR-003, SC-004 | `overview_card::tests` 280/380/480 width rows, `layout_snapshot` `overview-issue-link` | Yes |
| FR-004, SC-002 | `a_jira_address_has_exactly_one_slash_before_browse`, `issue_link_activated_opens_the_address` (bin) | Yes |
| FR-005, FR-006, SC-003 | `text_link`, `control_overlay`, `overview_card`, `card_grid` press/release rows | Yes (mounted widgets) |
| FR-007 | `overview_expansion` `LinkRested` rows, `card_grid` rest rows | Yes |
| FR-008, FR-009 | `keyboard_focus`, `overview_expansion` focus part, bin Tab/Escape/Menu rows, `tests/overview.rs` | Yes |
| FR-010, FR-011, SC-006 | `jira::tests`, `settings_roundtrip`, `jira_base_url_setting`, `features_settings::jira_base_url_form` | Yes |
| FR-012, SC-005 | `adopt_daemon_settings` rows in bin and `features_settings`, `jira_base_url_setting` broadcast | Yes |
| FR-013 | no 1122 test found for a failed open; the link goes through `shell::links::perform` | Not verified (see below) |
| FR-014 | no network call in `issue_link` (pure) and `IssueLinkActivated` changes no state | Yes |
| FR-015 | `composition_contrast`, `text_link` role rows, `showcase_completeness` | Yes |
| FR-016 | `issue_link`, `jira`, `overview_expansion` are render-free core | Yes |
| FR-017, SC-007 | `layout_snapshot`, user guide entries (docs-guard tests) | Yes |

Untested criteria: none found by name. FR-013: not traced to a test in this audit (the link activation path reuses `shell::links::perform`; whether feature 031's tests cover the address-and-reason notification was not checked). Treated as a gap, not as covered. Tests tracing to nothing: none seen.

## What was not audited

- The full workspace suite and `mise run gate` (fmt, clippy, other features' tests, `scripts/tests/*.test.sh`) were not run; only the scoped filters listed in `suite:`.
- `micold-core --all-targets` carries three root-only reds unrelated to the feature (`settings_refuses_save_over_failed_read`, `settings_write_is_logged`, `worktree_leftovers`), per the cycle log; not touched or re-run.
- Mutation: 21 hand-made mutants over the feature files, not a tool run; `main.rs`, `shell/issues.rs`, `features/overview.rs`, `features/settings.rs`, `ui/settings/jira.rs`, `state.rs`, `server.rs` were sampled through at most one mutant each or not mutated. Coverage: none available.
- `layout_snapshot` and `settings_sections` fixtures and the re-bless diffs were taken from the log, not re-diffed.
- FR-013 end to end (notification text), the Windows/macOS legs, and the sandbox real-runtime suite.
- The 70-file test diff contains other features' tests (interleaved commits); only files named in the brief's mutation scope were graded.
- Performance, beyond the scoped run times noted above.
