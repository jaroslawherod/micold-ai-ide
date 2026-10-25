---
feature: 1116-container-name-default
verdict: FAIL
standard: .specify/extensions/tdd/templates/tdd-test-quality-rubric.md # rubric graded against
verified_at: d02a98f8a # HEAD; code range 2ecf9e9ea..74a1b4b28 (PR #1362), feature code commit 33419f1a7
behaviors: 12 # no test-list.md; derived from tasks.md T001, T003-T005, T008, T010 and the spec's acceptance scenarios
proven: 0
likely: 0
test_after: 12
no_test: 0
high_smells: 0
criteria_total: 25 # 10 acceptance scenarios + 11 FRs + 4 SCs
criteria_covered: 25 # none untested; 3 only weakly (findings 2, 3)
mutation_score: unmeasured # no mutation tool; 5 deliberate mutants, 5 caught
mutants_survived: 0
suite: "feature test binaries: 18 + 85 + 12 + 76 passed, 0 failed (wall 70 s after a 32 min wait for the shared build lock); full workspace suite not run"
---

# TDD Verification: Container daemon name defaults to the daemon name

**Verdict: FAIL.** No behavior has any recorded red: there is no `tdd/test-list.md`
and no `tdd/cycle-log.md`, and the history puts tests and source in one `wip`
commit (`33419f1a7`), so test-first cannot be shown for any of the 12 behaviors
(rubric: no red recorded means `TEST_AFTER`). The tests themselves are good:
no `HIGH` smell, no weakened existing test, and 5 of 5 deliberate mutants caught.

The auditor did not write these tests; every file was re-read cold.

## Test-first evidence

The feature used a different flow, not the TDD extension: `tasks.md` states "each
test task precedes its code task and must fail first" and orders T001 before T002
and so on, and `autopilot.md` is the run ledger. Neither records a red command or
its output (`grep` for red/failing in `autopilot.md` finds only an unrelated
gate note). Facts from git:

- `33419f1a7 wip(1116): M1 partial work` adds, in one commit, `default_container_name`
  and its tests (`crates/micold-core/src/daemons.rs`, `tests/daemons_registry.rs`),
  `Draft::resolved_container_name`, `container_name_hint`, the `form_error` and
  `runtime_of` changes and all new client tests (`tests/features_daemons.rs`), the
  view change and the docs. Order within the commit is unknowable.
- `533ffa1e0` (and the net change `33419f1a7..74a1b4b28`) only touches three existing
  test files (7 insertions, 3 deletions) to follow the label and required-field change.
- The tasks T010 text says some cases are "regression guards, expected green at once"
  and others "red-first"; no evidence for the red-first ones exists.

| Behavior (task) | Class | Evidence |
| --- | --- | --- |
| B1 `default_container_name` sanitizing, trim, collapse (T001) | TEST_AFTER | no red recorded; test and source in `33419f1a7` |
| B2 truncation to 63 and trailing-separator strip (T001) | TEST_AFTER | same |
| B3 unusable name gives `None` (T001) | TEST_AFTER | same |
| B4 container name not required (T003) | TEST_AFTER | existing tests edited in the same commit as the source change |
| B5 empty/whitespace container name stores sanitized daemon name (T004) | TEST_AFTER | same commit, no red |
| B6 typed name kept (T004) | TEST_AFTER | same; would be green on old code (guard), not marked as such |
| B7 `container_name_hint` (T005) | TEST_AFTER | same |
| B8 defaulted clash lands on `container_name`, port clash on `port` (T008) | TEST_AFTER | same |
| B9 sanitize-alike and truncation collisions refused (T008) | TEST_AFTER | same |
| B10 rename keeps the name; edit shows value (T010) | TEST_AFTER | same (task text says green at once, i.e. a guard) |
| B11 clearing a stored name re-derives; unusable name refused with message (T010) | TEST_AFTER | same, no red |
| B12 empty daemon name / other runtimes unchanged (T010) | TEST_AFTER | same |

### Existing tests changed

All edits follow a requirement change (FR-007, container name no longer required),
not a loosening to hide a failure. Each was reviewed:

- `crates/micold-client/tests/features_daemons.rs:57-72` and `~1115`: expected
  required-field lists lose `DaemonContainerName`, counts 3 to 2, `Missing` to
  `Optional`. Legitimate, spec-driven.
- `crates/micold-client/tests/features_daemon_sections.rs:411-417`
  (`the_marker_follows_the_required_fields`): the container-name half of the
  assertion (marked when empty, unmarked when typed) is removed; what remains is
  "never marked". Legitimate for FR-007, but the new "not required" behavior is
  now pinned only by B4.
- `crates/micold-client/tests/daemons_section_geometry.rs:873`: label `"Container name*"` to `"Container name"`. Legitimate.
- `crates/micold-client/tests/daemons_feature.rs:819-826`
  (`a_required_field_left_empty_keeps_connection_open`): adds `if runtime == Container`
  inside a loop to set daemon name `!!!`. Keeps the assertion alive for the container
  case; adds conditional setup to an existing test (see finding 4).

No test was skipped, renamed out of a filter, or excluded; no threshold changed.

## Findings

| # | Severity | Finding | Evidence |
| --- | --- | --- | --- |
| 1 | HIGH (verdict) | No red evidence for any behavior; no test list, no cycle log; tests and source land in one commit. All 12 behaviors are `TEST_AFTER` by the rubric. Cannot be repaired after the fact; record as process finding | `git show 33419f1a7 --stat`; `specs/1116-container-name-default/` has no `tdd/` before this report |
| 2 | MED | The placeholder (FR-007, US1 scenario 2) is covered only through the pure helper `container_name_hint`; no test renders the form and checks the field shows the hint or is unmarked. The change in `ui/settings/daemons.rs` (`TextField::new(container_name_hint(draft), ...)`, `.required(...)` removed) is untested except by the quickstart visual pass. Reverting the hint wiring in the view would pass the suite | `crates/micold-client/src/ui/settings/daemons.rs:411-413`; tests at `features_daemons.rs:1397-1405` |
| 3 | MED | `an_empty_daemon_name_keeps_its_own_message` does not assert the message its name promises: it checks `form_error.is_none()` and `draft.is_some()`, so changing "A daemon needs a name." would not fail it. Spec edge case "the existing message wins" is unpinned | `crates/micold-client/tests/features_daemons.rs:1530-1536` |
| 4 | MED | Conditional logic added to an existing loop test (`if runtime == Container`), so the same test now runs different setup per runtime | `crates/micold-client/tests/daemons_feature.rs:822-826` |
| 5 | MED | `the_registry_still_refuses_a_duplicate_resolved_name` only calls `registry.add` with a literal container name; it never resolves a name, so it tests unchanged registry behavior, duplicates `daemons_feature.rs:2209`'s registry check, and its name overstates | `crates/micold-client/tests/features_daemons.rs:1475-1493` |
| 6 | MED | `a_typed_container_name_is_kept` (B6) and the rename case are guards that pass on the old code too; they are not distinguished as such in the tasks or the test names. Acceptable as regression guards, but they cannot show the new code works; B6 is caught by the existing `daemons_feature` tests only through mutant C, not by itself being tested for the new path | `features_daemons.rs:1400-1406` |
| 7 | LOW | Magic literals: `"x".repeat(70)`, `"a".repeat(62)`, `7001`, `7999` (the fixture's port) are unexplained outside comments; `7001` relies on the fixture's first daemon | `features_daemons.rs:1458-1463, 1411-1417`; `tests/daemons_registry.rs` `a_long_name_is_cut...` |
| 8 | LOW | Several tests assert multiple behaviors in one body (`unusable_characters_become_one_dash_and_the_edges_are_trimmed` has 5 asserts without labels; `a_defaulted_clash_lands...` covers typed and defaulted clash). Failure output names the input only via the actual/expected values | `tests/daemons_registry.rs:203-211`; `features_daemons.rs:1418-1433` |

Smell-catalogue pass: no assertion-free, tautological, doubled-subject, over-mocked,
vacuous (other than finding 3's near miss), self-approving snapshot, or skipped test.
The `for sep in [...]` loop in `a_long_name_is_cut_and_loses_a_trailing_separator`
asserts on every iteration with a labelled message, so it is not conditional-logic.
Style and helpers: the new core tests sit in a `mod default_container_name` in the
existing integration file and the client tests reuse `daemons_fixture::World` and its
`container()` factory; no hand-rolled doubles. Matches the profile's `unit` exemplar
(`tests/*.rs` with a support module). Isolation: in-memory `World`, no clock/IO,
deterministic; speed: the four binaries run in under 0.3 s each of test time.

## Mutation results

No mutation tool in the profile (`mutation: null`). Five deliberate mutants, one at a
time, restored with `git checkout --` and the suite re-run green afterwards
(18 + 85 + 12 + 76 passed). Sample: sanitizer length/trim rules, typed-name
precedence, hint, and the uniqueness field routing. Not exhaustive; the placeholder
view wiring and the `required_fields` removal were not mutated (finding 2).

| Mutant | Behavior | Survived | Judgment |
| --- | --- | --- | --- |
| `core/src/daemons.rs:13` max 63 to 64 | B2 | No | caught by `a_long_name_is_cut...` |
| `core/src/daemons.rs:34` trailing strip drops `.` | B1/B2 | No | 2 core tests fail |
| `client/src/features/daemons.rs:183` typed name ignored (`if true`) | B6 | No | caught by 2 failures in `daemons_feature` (not by the new `a_typed_container_name_is_kept`, which runs after the first failing binary stops cargo; not individually confirmed) |
| `client/src/features/daemons.rs:986` hint returns empty | B7 | No | caught in `features_daemons` |
| `client/src/features/daemons.rs:1013` duplicate routing reverted to `draft.container_name` | B8 (FR-010) | No | 3 failures in `features_daemons` |

Cargo stops after the first failing test binary, so which individual tests caught
mutants C and A beyond the named ones is not recorded.

## Traceability

Criteria enumerated from `spec.md`: US1 scenarios 1-3, US2 1-2, US3 1-3, US4 1-2
(10 scenarios) plus the 11 FRs and 4 SCs. No `traces` column exists; mapping is by
test content. End-to-end here means through `daemons::update(Msg::Saved)` /
`begin_edit` (the real feature entry point short of the iced view).

| Criterion | Tests | End to end |
| --- | --- | --- |
| US1-1 / FR-001 | `an_empty_container_name_stores_the_sanitized_daemon_name`, `a_whitespace_only...defaults` | Yes (`Msg::Saved`) |
| US1-2 / FR-007 | `a_container_daemon_marks_its_name_but_not_the_container_name`, `..._requires_its_name...`, `the_hint_is_the_sanitized_name_or_nothing` | Partly: state and helper only, not the rendered field (finding 2) |
| US1-3 / FR-004 | `a_typed_container_name_is_kept` | Yes |
| US2-1 / FR-006, SC-003 | `a_rename_keeps_the_container_name_and_edit_shows_it_as_a_value` | Yes |
| US2-2 / FR-008 | same | Yes (draft value; not the rendered field) |
| US3-1 / FR-002, FR-011 | `default_container_name::*` (4 core tests); client `build box` | Yes at core and client |
| US3-2 / FR-003 | `an_unusable_daemon_name_with_an_empty_field_asks_for_a_container_name` | Yes |
| US3-3 | `unusable_characters_...` (`  web  `) | Core only; the client path is the same function |
| US4-1 / FR-005, FR-010, SC-004 | `a_defaulted_clash_lands_on_the_container_name_and_creates_nothing`, `names_that_sanitize_alike...`, `a_truncation_collision_is_refused` | Yes |
| US4-2 | `names_that_sanitize_alike_are_refused_and_a_different_typed_name_saves` | Yes |
| FR-009 | `other_runtimes_do_not_resolve_a_container_name` (SSH only; host and Kubernetes not exercised) | Yes, partial |
| Edge: empty daemon name | `an_empty_daemon_name_keeps_its_own_message` | Weak (finding 3) |
| Edge: concurrency (commit-time check) | none new; relies on registry check | not tested |

Untested criteria: none outright; weakly tested: FR-007 view rendering, the empty-name
message, host/Kubernetes unchanged. Tests tracing to nothing: none.

## What was not audited

- Test-first order for every behavior: no evidence exists (finding 1).
- The full workspace suite, `mise run gate`, clippy and the layout snapshot: only the
  four feature-related test binaries were run (`ledger` notes the host
  `layout_snapshot status-bar-warnings` failure unrelated to this feature).
- Coverage: no tool in the profile.
- Mutation testing across the diff: only 5 deliberate mutants; no tool-based score.
- The rendered GUI (quickstart section B visual pass) and `docs/user-guide` accuracy.
- Concurrency of two sessions saving at once, and the host and Kubernetes drafts.
- The other features in the `2ecf9e9ea..74a1b4b28` range (the range spans ~800 files
  of other features); only files of feature 1116 were assessed.
