---
feature: 1857-worktree-parent
verdict: FAIL
standard: .specify/extensions/tdd/templates/tdd-test-quality-rubric.md # rubric graded against
verified_at: c53be1dc9 # short SHA audited (main after #1921)
behaviors: 13 # derived from tasks.md: there is no tdd/test-list.md
proven: 0
likely: 2
test_after: 11
no_test: 0
high_smells: 0
criteria_total: 17 # acceptance scenarios (US1 5, US2 3, US3 3, US4 4, US5 2); FRs are traced in the table too
criteria_covered: 12 # fully; 4 partial (US1 AC3, AC4, US2 AC2, US3 AC1), 1 none (US2 AC3 relocation)
mutation_score: 50 # scope: crates/micold-core/src/worktree_parent.rs only, 8 hand-made mutants, no tool; 4 of 4 survivors judged equivalent or unreachable
mutants_survived: 4 # M1-M4, none shown to matter, none pinned by a test
suite: targeted only. core lib `worktree` 93 passed 0 failed; daemon `mcp worktree_parent` 23 passed; client `sidebar_parent` 9 passed; core `mcp_tools_catalog` parent test 1 passed; `supervisor-ownership.test.sh` 39 cases 0 failures. Full workspace suite not run.
---

# TDD Verification: Worktree parent registration (MCP + sidebar) and supervisor support

**Verdict: FAIL.** 11 of 13 behaviors are `TEST_AFTER`: the cycle log itself says M1, M1b, M2 and M3 were
written test-and-code in one pass, and every milestone merged as one squashed commit, so no behavior has
a red that precedes its code. No test was weakened, no `HIGH` smell was found, and mutation found no
survivor that matters, so the failure is the test-first discipline, not the tests' content.

The audit is independent: the auditor did not write these tests. No fresh-context subagent was available
in this run (no Agent tool), so the smell pass was done by the auditor directly, reading every cited file.

## Test-first evidence

There is no `tdd/test-list.md`. Behaviors are derived from `tasks.md` (docs-only, tracking and visual-pass
tasks T010, T014, T018-T022 docs parts, T026-T029, T032 are `NOT_APPLICABLE`). Git history cannot show
ordering: each of #1873, #1912, #1921, #1917 is one squash commit, and the WIP commits the log names
(`5cfb009ca`, `b733a25c6`, `307e04e11`) are not in this clone.

| Id | Behavior (tasks) | Class | Evidence |
|---|---|---|---|
| B1 | Store round-trip of ids and links (T002/T003) | TEST_AFTER | log "Honest account": not test-first; no red; core unit mutants only |
| B2 | Id minting and reconcile (T004/T005) | TEST_AFTER | same; red substitute is mutants M-a..M-c in the log, after the fact |
| B3 | Tool schemas accept `parent` (T006/T008) | TEST_AFTER | same; test and code in one commit |
| B4 | create_worktree records parent, list filter (T007/T009) | TEST_AFTER | same; first-ever run was against finished code |
| B5 | Visibility by parent, all tools (T030/T031) | TEST_AFTER | log: "tests and the filter landed in one commit"; red recorded after the fact by swapping in origin/main `tools.rs` |
| B6 | Reason text names no hidden worktree (review B F4) | LIKELY | log records two tests red before the fix; squash cannot corroborate order |
| B7 | Rename and restart keep the link (T011/T013) | TEST_AFTER | as B1 |
| B8 | Removed parent, no re-adoption (T012/T013) | TEST_AFTER | as B1; one test failed first for a test bug, not a behavior red |
| B9 | Session rows carry the parent (T020/T021) | TEST_AFTER | as B1; one daemon-wiring mutant red after the fact |
| B10 | Sidebar parent tag, chip, filter (T015-T017) | TEST_AFTER | log: code and tests in `b733a25c6`; red by stubbing, after the fact |
| B11a | `runs-of-parent.sh`, original 25 cases (T023/T024) | TEST_AFTER | log: test and helper in `307e04e11`; red against origin/main after the fact |
| B11b | Review fixes A F1-F6 / B F2 (ledger, `--check`, input errors) | LIKELY | log: rebuilt 39-case test red (20 FAILED) on the old code before the fix; uncorroborated |
| B12 | Supervisor skill ownership and relay text (T025/T033) | TEST_AFTER | as B11a |

### Existing tests changed by the feature

Diffed every test file touched in the four PRs. No assertion was removed in favor of a weaker one, no test
was renamed, skipped, `#[ignore]`d or excluded, and no threshold changed. Reviewed edits:

- `mcp_worktree_parent.rs:223-226` (M1b, #1912): the positive case `create(sid(3), "c2", {"parent": "default"})`
  expecting `("default","known")` became `{"parent": "c1"}` expecting `("c1","known")`. Legitimate under
  FR-016 (a worktree caller may not name `default`), and the refusal is pinned at
  `mcp_worktree_parent_visibility.rs:208`. But it removed the only positive test of an explicit
  `parent: "default"`; see finding 3.
- `mcp_read_tools.rs` (13 sites), `mcp_create_worktree.rs:107`, `mcp_create_session.rs`, `mcp_cross_session.rs`,
  `attach_mcp.rs`, `mcp_audit_log.rs`, `codex_opencode_start.rs`, `mcp_default_prompt.rs`: the caller changed from
  a worktree session (`sid(3)`) to the root session (`sid(1)`) or to a full-view credential, so the
  pre-existing expectations keep their full view under FR-015. Assertion values are unchanged. Not a
  weakening; the worktree-caller view of these tools is now pinned only by the visibility module.
- `features_sidebar.rs:2262-2400`, `logical_state_ownership.rs`, `sidebar_state.rs`, `sidebar_tree.rs:1199`:
  `Copy` to `Clone` mechanical edits (`TagFilter` now holds a `ParentLabel`). Assertions identical.
- `protocol_roundtrip.rs:996`, `schema_hash.rs:351`: protocol version 81 to 82 with the schema change, as
  the one-bump rule requires.
- `mcp_policy.rs`, `mcp_tools_catalog.rs`, `mcp_create_worktree.rs:149-212,412`: expected values extended
  (`ListWorktrees { parent: None }`, two new result keys, `parent`/`parent_state` in the exact-equality
  result). Stronger, not weaker.

### tasks.md against the evidence

Every behavioral task T001-T033 is `[x]`. With no test list there is no `DONE` state to compare, so the
check is against the files: each ticked test task has a test that exists and passes today (runs below). No
`[x]` task lacks a test; the claim "failing test first" in those task descriptions is what the cycle log
contradicts (finding 1).

## Findings

| # | Severity | Finding | Evidence |
|---|---|---|---|
| 1 | HIGH | 11 of 13 behaviors were written test-after, admitted in the cycle log, and the squashed history cannot show otherwise. Constitution I ("mandatory and first") is not met for M1, M1b, M2, M3. | `tdd/cycle-log.md:3-6,43-46,76-79,114-117`; `tasks.md` header "Tests: mandatory and first" |
| 2 | MED | FR-017 / T030 promise "the same message sent to the parent succeeds". The test asserts only that the in-scope send fails with `conflict` (not `not_found`), i.e. "got past the scope check". A wrong scope filter that lets the call fail later for another reason still passes. | `mcp_worktree_parent_visibility.rs:188-205` |
| 3 | MED | No test passes an explicit `parent: "default"` to `create_worktree` and expects success (FR-002). The only one (M1 `c2`) was replaced in #1912 and not re-added elsewhere; `mcp_create_worktree.rs:416` is the implicit root case. | `mcp_worktree_parent.rs:223` (diff of #1912); grep `"parent": "default"` over `crates/micold-daemon/tests` |
| 4 | MED | T012 says "delete the parent through the tool and through the app op". Only `ops::delete_worktree` is exercised for tombstoning; the MCP `delete_worktree` path is tested for scope (`not_found`) but never for tombstoning the children. | `mcp_worktree_parent.rs:85-95,357,380`; `mcp_worktree_parent_visibility.rs:169` |
| 5 | MED | Requirements with no test: FR-005 "relocation of the project" and SC-002's "100% of relocation scenarios" (US2 AC3; accepted as drift in `drift-M1.md`, but the spec still promises it); FR-005 "rename of the child" (only the parent is renamed); FR-003 for a worktree created in the app (the "unparented" fixtures are made with `git worktree add`, not the app op); FR-012 (no other-project test); FR-004 (no test that no tool changes a parent). | `drift-M1.md:7-9`; `mcp_worktree_parent.rs:35-40`; spec FR-003/4/5/12 |
| 6 | MED | No `tdd/test-list.md`: behaviors have no ids, no `traces`, no `DONE` state. This audit had to derive them; the next reader cannot check the mapping. | `specs/1857-worktree-parent/tdd/` holds only `cycle-log.md` |
| 7 | MED | Four mutants in `worktree_parent.rs` survive both the core unit tests and the daemon tests (see Mutation). They are equivalent or unreachable through today's entry points, but each is an unpinned branch: the self-parent guard (`:110`), the `last_name` refresh (`:83-85`, dead code: ids are keyed by directory name and `last_name` is set from that same name), the orphan-link sweep (`:76-77`), and `ParentView::label`'s `default` special case (`:38`, no unit test). `record`'s reuse of an existing parent id (`:112`) is killed only at the daemon level, not by a core unit test. | `crates/micold-core/src/worktree_parent.rs`; `tdd/cycle-log.md:41` (a similar equivalent mutant was already accepted) |
| 8 | MED | `renaming_the_parent_updates_the_label_and_keeps_order` delivers a new `ParentLabel` and asserts the row shows that label: it proves the sidebar renders its input, not that a rename reaches the label. The real rename-to-label path is only in `the_snapshot_labels_a_child...` (daemon). Not `HIGH`: the order assertion and the daemon test carry the behavior. | `crates/micold-client/tests/sidebar_parent.rs:147-169`; `mcp_worktree_parent.rs:327-349` |
| 9 | MED | Skill-text cases assert that words appear in `SKILL.md` (`has "never adopt" "orphan"`, ...). They pin wording, so a reworded but correct rule fails and a wrong rule that keeps the words passes. | `scripts/tests/supervisor-ownership.test.sh:72-81` |
| 10 | LOW | Eager tests and assertion roulette: several behaviors per test, some assertions unlabelled. Odd construct `vec!["fix-c"].into_iter().map(String::from).collect()` where a `vec!["fix-c"]` pattern used two lines above would do. | `mcp_worktree_parent.rs:215-228,353-374`; `sidebar_parent.rs:105-120`; `mcp_worktree_parent_visibility.rs:148-186` |
| 11 | LOW | `core` unit tests carry no assertion messages on `reconcile_mints_once_and_keeps_ids`, `default_links_are_never_removed` against the profile convention ("every assertion carries a message explaining the rule"). | `worktree_parent.rs:168-176,203-213` |

Foreign style, bypassed utilities, framework-under-test, tautologies against doubles, vacuous or
conditional assertions, empty or skipped tests, self-approving snapshots: none found in the files read.
Fixtures reuse `mcp_support` (`init_repo`, `add_worktree`, `call_ok`, `state_over`) and
`support::sidebar_fixture` (`listed`, `deliver`, `state_with_active`); no hand-rolled double duplicates a
helper. The daemon tests use real git and tempdirs (the repo's `mcp` binary convention) and ran in about
1 s, deterministic in the single run made.

## Mutation results

No mutation tool in the profile. Deliberate mutants, 8, all in
`crates/micold-core/src/worktree_parent.rs`, one at a time, restored and re-verified green after each
(`git diff` empty; final re-run: core lib 9 passed, daemon 23 passed). Tests run:
`scripts/build-lock.sh cargo test -p micold-core --lib worktree_parent` (9 tests), and for core
survivors `scripts/build-lock.sh cargo test -p micold-daemon --test mcp worktree_parent` (23 tests).
Sampled the id lifecycle and tombstone behaviors (B2, B7, B8); the visibility filter, catalog, client
and scripts were not mutated.

| Mutant | Behavior | Core unit | Daemon | Judgment |
|---|---|---|---|---|
| M1 `record`: drop the `name == dir` self-parent guard (`:110`) | B4 | survived | survived | Unreachable today: an explicit parent must already be live, and a new worktree cannot share a live name. Defensive guard, unpinned. |
| M2 `reconcile`: never refresh `last_name` (`:83`) | B7/B8 | survived | survived | Equivalent: `last_name` is always set from the same directory name the id is keyed by, so the refresh never fires. Dead branch; T013 says "keep last_name current". |
| M3 `reconcile`: keep links of children with no id (`:76`) | B2 | survived | survived | Equivalent through the app (every path removes the link with the id); only a hand-edited store reaches it. |
| M4 `label`: do not special-case `default` (`:38`) | B10 | survived | survived | Equivalent while `display("default")` is the identity; would matter only if a worktree named `default` carried a display override. Unit-untested. |
| M5 `reconcile`: retired ids not counted as a change (`:70`) | B2/B8 | killed | n/a | `a_vanished_parent_tombstones...` (line 190 asserts `changed`). |
| M6 `reconcile`: mint a new id on every pass (`:64`) | B2 | killed (3) | n/a | `reconcile_mints_once_and_keeps_ids` and two others. |
| M7 `retire`: tombstone the wrong links (`:52`, `==` to `!=`) | B8 | killed | n/a | `forgetting_a_child_drops_its_entries_and_a_parent_tombstones`. |
| M8 `record`: re-mint an existing parent's id (`:112`) | B4/B7 | survived | killed (8) | Caught only at the daemon level (`the_parent_filter_returns_direct_children_only`, ...); no core unit pins it. |

Score: 4 killed of 8 (50%). Of the 4 survivors, 0 are shown to be real bugs and all 4 are judged
equivalent or unreachable by reading, not by test; no survivor sits inside a behavior in a way that
shows a test failing to test what it claims, so the rubric's "survivor inside a `DONE` behavior" `HIGH`
is not raised. Treat the equivalence judgments as unproven (finding 7).

## Traceability

Acceptance scenarios and requirements, mapped by reading the tests (there is no `traces` column).
"End to end" means the real MCP tool entry point (daemon `mcp` binary over a real repo) or the real
client `State::update`/sidebar entry point.

| Criterion | Tests | End to end |
|---|---|---|
| US1 AC1-AC2 (root and worktree callers record parent), FR-001 | `a_root_session_records_default...`, `a_session_in_a_worktree_records_it...`, `mcp_create_worktree.rs:412` | Yes |
| US1 AC3 / FR-002 (explicit override, refuse unknown) | `...explicit_parent_overrides` (named worktree), `an_explicit_parent_that_is_not_live_is_refused...`; explicit `default` positive case missing | Partial (finding 3) |
| US1 AC4 / FR-003 (app-made has none) | `a_root_session_records_default...` (`hand` made by git) | Partial (finding 5) |
| US1 AC5 / FR-008 (filter, direct children, `default` valid) | `the_parent_filter_returns_direct_children_only`, `create_worktree_and_list_worktrees_accept_an_optional_parent` | Yes |
| US2 AC1 / FR-005 rename of parent | `renaming_the_parent_keeps_the_link` | Yes |
| US2 AC2 / FR-005 restart | `a_restart_keeps_the_link` (daemon state reload, reads views directly, not via MCP) | Partial |
| US2 AC3 / FR-005, SC-002 relocation | none; store round-trip only (`store.rs` tests) | No (finding 5) |
| FR-005 rename of the child | none | No (finding 5) |
| US3 AC1 / FR-006 delete parent | `deleting_the_parent_leaves_children_removed...` via `ops`; MCP tool path not tombstone-tested | Partial (finding 4) |
| US3 AC2 / FR-007 same-name re-creation | same test | Yes |
| US3 AC3 `default` link unaffected | `default_links_are_never_removed_and_an_unrelated_delete...` | Yes |
| FR-014 stable id | `the_first_refresh_mints...`, core unit tests | Yes |
| US4 AC1-AC4 / FR-010 | `sidebar_parent.rs` (9), `the_snapshot_labels_a_child...` | Yes (renaming case weak, finding 8) |
| US5 AC1-AC2 / FR-009, FR-011 | `session_rows_carry_the_worktrees_parent_independent_of_origin` | Yes |
| FR-015 | `mcp_worktree_parent_visibility.rs` (8 tests) | Yes |
| FR-016 | `an_explicit_parent_outside_the_callers_set_is_invalid_input` | Yes |
| FR-017 | scope refusal tested; parent reachable only as "not `not_found`" | Partial (finding 2) |
| FR-012 other-project isolation | none | No (finding 5) |
| FR-004 parent fixed at creation | none (absence of a tool) | No (finding 5) |
| FR-013 / SC-005 supervisor and docs | `supervisor-ownership.test.sh` (39 cases); docs not tested | Yes (finding 9) |
| SC-001, SC-003, SC-004 | covered by the rows above | Yes |

Untested criteria: FR-004, FR-005 (child rename, relocation), FR-012; partly FR-002 (explicit `default`),
FR-003 (app op), FR-006 (tool path), FR-017 (success path). Tests tracing to nothing: none found.

## What was not audited

- The full workspace suite (`mise run gate`, 5071 tests, 343 s baseline): not run, a gate was building in
  parallel and no mise here. Only the targeted runs in the frontmatter. No failure inside the feature
  was seen; failures elsewhere are unmeasured.
- Mutation covered one file (`worktree_parent.rs`) with 8 hand-made mutants. The visibility filter and
  `Scope::explain` (`crates/micold-daemon/src/mcp/tools.rs`), `catalog.rs`, `ops.rs`, `state.rs`,
  `store.rs`, the client sidebar and `runs-of-parent.sh` were not mutated, so the strength of B1, B3,
  B5, B9-B12 is unmeasured beyond what the cycle log claims.
- Coverage: the profile has none. Performance, flakiness over repeated runs, and test isolation under
  parallel load: not assessed.
- The cycle log's reds (stubbed or swapped runs against `origin/main`) were not reproduced.
- The visual pass (T018), docs (T010, T014, T019, T022, T032) and the GitHub issue tracking (T026, T029)
  were not graded.
- Uncommitted changes to `spec.md`, `plan.md`, `tasks.md` and `autopilot.md` in the working tree at
  audit time were not reviewed. The pre-squash WIP commits are absent from this clone.
