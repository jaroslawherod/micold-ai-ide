---
feature: 1422-container-name-min-length
verdict: PASS_WITH_GAPS
verified_at: 3caed28b3
standard: .specify/extensions/tdd/templates/tdd-test-quality-rubric.md
scope: tasks T001-T009, PR #1489; feature commits 1ee7e417d (feat+tests) and 78b3c0c0e (fixture)
behaviors: 9 tasks (no test-list.md)
proven: 0
likely: 0
test_after_or_unprovable: 9
high_smells: 0
mutation: deliberate mutants only, 5 sampled, 5 killed
independent: yes (auditor did not write the tests)
---

# TDD verification: container name minimum length

**Verdict: PASS_WITH_GAPS.** The tests are strong (5 of 5 deliberate mutants killed, no weakened existing test), but there is no red-phase evidence: tests and source landed in one commit and no `tdd/test-list.md` or `cycle-log.md` exists.

Note: the supplied diff range `48cda33fe..3caed28b3` covers 121 files of unrelated features (1463, 1439, 1289, 1274, ...). I audited only the feature's own commits, 1ee7e417d and 78b3c0c0e.

## Suite
- `mise`-equivalent via `scripts/build-lock.sh cargo test`: `daemons_registry` 22 passed, `features_daemons` 80 passed, 0 failed. Full workspace suite not run (see not audited).

## Test-first evidence
| Task | Behavior | Class | Why |
|---|---|---|---|
| T001/T002 | `container_name_long_enough` | TEST_AFTER (unprovable) | test and source in the same commit 1ee7e417d; no red output recorded |
| T003 | legacy `a` round-trips | TEST_AFTER (unprovable) | same; this test would pass before the change, so no red was possible anyway |
| T004/T005 | save refusal | TEST_AFTER (unprovable) | same commit, no red |
| T006 | docs | NOT_APPLICABLE | |
| T007/T008 | hint hidden | TEST_AFTER (unprovable) | same commit |
| T009 | legacy edit refused | TEST_AFTER (unprovable) | same commit |

tasks.md says "written red first" but the history cannot show it (squashed into one commit). By the rubric this is not PROVEN; recorded as unprovable rather than disproven. Tasks T001-T009 are all `[X]`; each has a test that exists and passes, so no unearned ticks.

## Existing tests changed
- `crates/micold-client/tests/overlay_registry.rs:1036`: fixture container name `"c"` changed to `"cc"`. Required, since `"c"` is now refused and the test confirms a Save. Not a weakening (assertions untouched). No skips, exclusions, or loosened thresholds.

## Findings
1. MEDIUM, no red evidence (above). Next time record the failing run in a cycle log or commit tests first.
2. MEDIUM, spec scenario 1.4 untested: daemon name `a!` (sanitizes to `a`, must be refused). Tests use `"a"` only (`features_daemons.rs` ~1553, `a_one_character_resolved_container_name_is_refused`). The sanitize-then-check order, FR-003/US1.4, is unguarded against a reorder that checks the raw name.
3. LOW, scenario 1.3 (daemon `ab`, empty container name, Save creates `ab`) has no Save test; only the hint (`the_hint_is_hidden_when_the_default_is_too_short`) covers `ab`. Existing 1116 default tests cover the sanitized-default Save path generally.
4. LOW, scenario 1.2 (refusal, then type `ab`, Save in the same form) is not run as one sequence; `a_two_character_container_name_is_accepted` uses a fresh form (SC-003).
5. LOW, T004 asked for host/ssh/kube drafts unaffected; only SSH is covered (`other_runtimes_do_not_resolve_a_container_name`, pre-existing). Kube/host not exercised for FR-007.
6. LOW smell, `a_one_character_resolved_container_name_is_refused` loops over three cases in one test; a failure is localised by the `{name:?}/{typed:?}` message, so acceptable.
7. LOW, `a_stored_one_character_name_loads_lists_and_is_refused_on_save` bundles several behaviors and the "lists" claim is only a registry lookup, not the list view. Acceptable.

No tautologies, doubled subjects, or re-implemented expectations found. Tests follow repo conventions (integration files under `tests/`, `World`/`save` helpers, `Fake`-free pure reducer tests).

## Test strength (deliberate mutants; no mutation tool in the profile)
Five mutants, one at a time, each restored via `git checkout` and the suite re-run green (22 and 80 passed); `git status` clean afterwards.
| # | Mutant | Result |
|---|---|---|
| M1 | `core daemons.rs`: `>= MIN` to `> MIN` | killed (3 core + 2 client failures) |
| M2 | drop `.trim()` | killed (1 core; client missed it) |
| M3 | `chars().count()` to `trim().len()` (bytes) | killed (1 core; client missed it) |
| M4 | client `runtime_of`: disable the length guard | killed (3 client failures) |
| M5 | client `container_name_hint`: drop the filter | killed (1 client failure) |

M2/M3 are killed only by core tests; the client typed-name path (` x ` case) covers trim only through the trimmed resolved name. Fine, since the predicate is the single source.

## Traceability
| Criterion | Tests | Real entry point |
|---|---|---|
| US1.1 / FR-001/002/008 | `a_one_character_resolved_container_name_is_refused` (`Msg::Saved`) | yes |
| US1.2 | `a_two_character_container_name_is_accepted` (partial, fresh form) | yes |
| US1.3 | none for Save (hint only) | no |
| US1.4 `a!` | none | no |
| US2 / FR-005 | `the_hint_is_hidden_when_the_default_is_too_short` | calls `container_name_hint` directly |
| US3 (`x`, ` x `, `xy`) | same two tests above | yes |
| US4 / FR-006 | `a_stored_one_character_container_name_round_trips_unchanged`, `a_stored_one_character_name_loads_lists_and_is_refused_on_save` | yes |
| FR-007 | SSH only | partial |
| Duplicate-message precedence | `a_too_short_name_never_shows_the_duplicate_message` | yes |

Tests tracing to nothing: none. All cited tests exist and pass.

## What was not audited
- Full workspace suite, clippy, and the `container_section_fits` geometry gate.
- Mutation tooling (none installed); only 5 hand-picked mutants, none on the docs or the `overlay_registry` fixture.
- Quickstart Part A (manual GUI).
- Determinism and speed beyond one run (suite is sub-second for these two files).
- Git ordering of tests against source: unprovable in a single commit.

## Remediation closed 2026-10-26

The findings above describe the audited commit 3caed28b3. F2-F4 were closed by five tests in
`crates/micold-client/tests/features_daemons.rs`: `a_sanitized_default_of_one_character_is_refused`
(US1.4), `a_two_character_daemon_name_with_an_empty_field_stores_it` (US1.3),
`typing_a_two_character_name_after_a_refusal_saves_in_the_same_form` (US1.2),
`a_one_character_name_is_not_refused_for_ssh_and_host_daemons` and
`a_one_character_daemon_name_saves_for_a_kubernetes_daemon` (FR-007). F1 (test-first evidence)
cannot be fixed retroactively.
