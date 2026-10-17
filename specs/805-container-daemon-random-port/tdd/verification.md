---
feature: 805-container-daemon-random-port
verdict: FAIL
standard: .specify/extensions/tdd/templates/tdd-test-quality-rubric.md # no override or preset found; extension copy used
verified_at: 4c617f01 # code under audit: dd47b785 (PR #926), diffed against dd47b785^
behaviors: 12 # no tdd/test-list.md; tasks.md T001-T012 stand in
proven: 0
likely: 0
test_after: 12
no_test: 0
high_smells: 0
criteria_total: 11 # the Traceability table rows
criteria_covered: 9 # SC-001 and FR-008 have no automated test; US2.1, US3.1 partial
mutation_score: 89 # deliberate mutants, scope: 4 changed source files, no tool; 8 of 9 non-equivalent caught
mutants_survived: 2 # M1 equivalent, M10 real gap
suite: scoped run only - daemons_registry 14 passed, sandbox_runtime 16 passed, daemons_feature 28 passed, 0 failed; full workspace suite not run
---

# TDD Verification: Container daemon without a port picks a random free port (805)

**Verdict: FAIL.** There is no recorded red for any behavior: no `tdd/cycle-log.md`, no `tdd/test-list.md`, and git shows tests and source landing together in one commit (`dd47b785`), so every behavior is `TEST_AFTER` under the rubric. The tests themselves are good (no HIGH smells, no existing test weakened, 9 of 10 deliberate mutants caught), so the fail is about unprovable test-first order and two coverage gaps, not about test quality.

This audit was run by a fresh session, not the one that wrote the tests. Every test and source file cited was re-read.

## Test-first evidence

No test list exists, so the unit of evidence is the task in `tasks.md`. `tasks.md` says "run red" for T001, T003, T010 but records no command or output anywhere. The only feature commit is `dd47b785`, which contains both `crates/*/tests/*` and `crates/*/src/*` changes, so history cannot show order either way. Fail closed: no red recorded means test-after.

| Behavior (task) | Class | Evidence |
| --- | --- | --- |
| T001/T002 picker, `os_free_loopback_port`, `container_ports` | TEST_AFTER | tests in `daemons_registry.rs` and source in `core/src/daemons.rs` in one commit, no red recorded |
| T003/T004 empty port picked, explicit stored, 0 and duplicate refused, host-to-container, distinct ports, taken elsewhere, no-free-port | TEST_AFTER | same commit, no red recorded |
| T005/T006 `update_daemons` parameter, shell `taken_elsewhere` | TEST_AFTER | source only; no test exists (see finding 2) |
| T007/T008/T009 fixtures, placeholder, user guide | NOT_APPLICABLE in spirit (no behavior test); counted TEST_AFTER for the placeholder only because no test pins it |
| T010/T011 port kept on cleared or untouched edit, restart | TEST_AFTER | same commit, no red recorded |
| T012 port-in-use message and remedy | TEST_AFTER | test added in the same commit as docs; the production strings pre-date it (804), so it was green on arrival: it is a characterization test, no red is possible |

The `tasks.md` checkboxes cannot be checked against a test list that does not exist. All T001-T012 are `[x]`; each has a matching test or an explicit "no test needed" note, except T006 (finding 2).

### Existing tests touched

Checked the whole `dd47b785^..dd47b785` diff for removed or loosened assertions, skips, renames, and config changes. None found.
- `crates/micold-client/tests/a_field_note_shares_its_fields_column.rs:85`: the call gained a `Vec::new()` argument for the new `update_daemons` parameter. No assertion changed.
- `crates/micold-client/tests/support/daemons_fixture.rs`: additive (`model_with`, two new `Model` fields).
- No coverage or mutation thresholds exist to lower. No `#[ignore]` added.

## Findings

| # | Severity | Finding | Evidence |
| --- | --- | --- | --- |
| 1 | FAIL-blocking | No test-first evidence for any behavior. Tests and source share commit `dd47b785`; no cycle log; "run red" in `tasks.md` T001/T003/T010 is unrecorded. Cannot be repaired after the fact; the remedy is to record this limitation and require a red going forward, or accept the FAIL as informational. | `git show --stat dd47b785`; absence of `tdd/` |
| 2 | MED | Mutant M10 survived. `shell/daemons.rs` computes `taken_elsewhere` from the settings store only for `Msg::Saved`; replacing the condition with `false` passes `daemons_feature` and the client lib tests. The cross-window distinctness promise (spec edge case: "from two windows or sessions at once") has a reducer-level test (`a_port_held_elsewhere_is_never_picked`) but nothing proves the shell feeds it. Full `-p micold-client --all-targets` was not run, so a test elsewhere could catch it. | `crates/micold-client/src/shell/daemons.rs:35-43` |
| 3 | MED | FR-003/SC-002 ("starts on its stored port, no new pick at start") is not tested through the start path. `a_picked_port_survives_a_restart` asserts the port survives a serde round trip of the registry (real), then asserts `bring_up(entry, Starting, false) == Spawn`, which does not depend on the port at all, so its message ("nothing picks at start") is not what it checks. | `crates/micold-client/tests/daemons_feature.rs` (last test, ~line 770) |
| 4 | MED | Quickstart step 5 / US3 / FR-005 / SC-003 are covered only at the message level (see below). No automated test runs a start against a taken port and sees the message surface to the UI; the visual pass for step 5 was inconclusive (`visual/results.md`, row 5: "not run (inconclusive)"). | `crates/micold-core/tests/sandbox_runtime.rs:115-127` |
| 5 | MED | `zero_and_an_explicit_duplicate_are_still_refused` is eager (FR-007 and FR-010 in one test) and its duplicate half asserts only `err.field == "port"`, so it would pass for any port-field rejection, not specifically a duplicate. | `daemons_feature.rs` `zero_and_an_explicit_duplicate_are_still_refused` |
| 6 | MED | `editing_with_the_port_cleared_or_untouched_keeps_the_stored_port` loops over `[true, false]` with an `if clear` that decides setup. Assertions run on every iteration so nothing is skipped, but two behaviors share one test and the second iteration reuses the already-renamed daemon. | `daemons_feature.rs` same-named test |
| 7 | LOW | `assert!(![7001].contains(&port), "not another daemon's port")` hard-codes a fixture port as a one-element array and double-negates; derive it from the fixture registry. | `daemons_feature.rs` `an_empty_port_is_picked_stored_and_shown_on_edit` |
| 8 | LOW | The placeholder copy "A free port is chosen if empty" (US1 hint, quickstart step 1) is pinned by no test; geometry gate only keeps the layout green. | `crates/micold-client/src/ui/settings/daemons.rs:310` |
| 9 | LOW | FR-008 (same behavior on Windows/macOS/Linux) rests on the CI OS matrix running the same tests; none is platform specific. Accepted by `tasks.md` T012; recorded here, not a defect. | `tasks.md` T012 |

No HIGH smells: no assertion-free, tautological, doubled-subject or snapshot tests. The `sequence(...)` source injected into `Model::next_port` is a seam, not a double of the unit under test, and the reducer's own selection logic is what is asserted. Helpers: tests reuse `World`, `add_container` and `model_with` from `tests/support/daemons_fixture.rs` rather than hand-rolling; assertions carry messages in most places (core tests follow suit). The local `sequence` helper exists twice with different signatures (`daemons_feature.rs`, `daemons_registry.rs`); that is near-duplication, not a finding.

### Quickstart step 5: is it covered by an automated test?

Partially, at the text level only. The test is `a_taken_port_names_the_port_and_where_to_change_it` in `crates/micold-core/tests/sandbox_runtime.rs:115`. It builds `RuntimeError::PortUnavailable { port: 7727 }` and asserts `reason()` is "Port 7727 is already in use, so the sandbox has no control channel." and `remedy()` is "Stop whatever holds the port, or choose another in Settings → Daemons." This is a real assertion on production code (mutant M9, dropping "Settings → Daemons" from the remedy, was caught), but it is a characterization of strings that already existed from 804.

The chain "occupied port at start produces that error" is covered by neighbors, not by this feature's test: `sandbox_runtime.rs:85-87` classifies the runtime's "address already in use" stderr into `PortUnavailable { port: 7727 }`, and `crates/micold-core/tests/sandbox_state.rs:225-236` checks a taken port makes bring-up fail with `PortUnavailable`. Missing: anything proving the daemon start path in the client surfaces `reason()` plus `remedy()` to the user for a container daemon. The real-runtime `sandbox_real_*` tests are not a match either (no occupied-port case found by name).

## Mutation results

No mutation tool in the profile. Deliberate mutants, one at a time, each restored and re-verified (post-restore `daemons_registry` 14, `sandbox_runtime` 16, `daemons_feature` 28 all green; `git status` shows only `specs/805-.../autopilot.md` modified, which I did not touch and which is not a mutated file). 10 mutants on the highest-risk behaviors (FR-001, FR-003/006, FR-009, FR-005); not exhaustive.

| Mutant | Behavior | Survived | Judgment |
| --- | --- | --- | --- |
| M1 `features/daemons.rs` drop `.filter(\|p\| *p != 0)` | FR-007 | Yes | Equivalent: the registry rejects port 0 at `core/src/daemons.rs:270` and `invalid_field("port")` produces the same text. Redundant guard. |
| M2 ignore stored port on empty edit | FR-006 | No | caught by `editing_with_the_port_cleared_or_untouched_keeps_the_stored_port` |
| M3 drop `taken.extend_from_slice(taken_elsewhere)` | FR-001 | No | caught by `a_port_held_elsewhere_is_never_picked` |
| M4 ignore registry ports when picking | FR-001 | No | caught by `two_empty_adds_get_different_ports_even_when_the_source_repeats` |
| M5 picker ignores `taken` | FR-001 | No | caught by two picker tests |
| M6 attempts 16 to 17 | FR-009 | No | caught by `the_picker_gives_up_after_sixteen_misses` |
| M7 `os_free_loopback_port` returns 0 | FR-001/008 | No | caught by `the_os_port_is_non_zero_and_binds` |
| M8 `container_ports` returns `port + 1` | FR-001 | No | caught by `container_ports_lists_every_stored_container_port` |
| M9 remedy drops "Settings → Daemons" | FR-005 | No | caught by the step-5 test |
| M10 `shell/daemons.rs` never passes `taken_elsewhere` | FR-001 (multi-window) | Yes | Real gap, finding 2. Only `daemons_feature` and client lib ran. |

Score: 9 caught of 10; M1 is equivalent, so 8 of 9 non-equivalent mutants were caught (89 percent). Scope: the 4 changed source files (`core/src/daemons.rs`, `client/src/features/daemons.rs`, `client/src/shell/daemons.rs`, `core/src/sandbox/runtime.rs`); no tool, no version. Not comparable to a tool-run score.

## Traceability

| Criterion | Tests | End to end |
| --- | --- | --- |
| US1.1 empty port stored, no error (FR-001) | `an_empty_port_is_picked_stored_and_shown_on_edit`, `two_empty_adds_...`, picker tests | Reducer entry (`daemons::save`); GUI path by visual pass B1 only |
| US1.2 edit shows port (FR-002) | `an_empty_port_is_picked_stored_and_shown_on_edit` | Reducer entry |
| US1.3 explicit port stored (FR-004) | `an_explicit_port_is_stored_as_given` | Reducer entry |
| US2.1 same port after restart (FR-003, SC-002) | `a_picked_port_survives_a_restart` | Partial: persistence yes, start path no (finding 3) |
| US2.2 untouched edit keeps port (FR-006) | `editing_with_the_port_cleared_or_untouched_...` | Reducer entry |
| US3.1 taken port at start (FR-005, SC-003) | `a_taken_port_names_the_port_...`, neighbors `sandbox_runtime.rs:85`, `sandbox_state.rs:225` | Message level only (finding 4) |
| FR-007 zero/out of range | `zero_and_an_explicit_duplicate_are_still_refused` (zero half) | Reducer entry; out-of-range (e.g. 70000) has no test |
| FR-008 cross-OS | none specific (CI matrix) | n/a |
| FR-009 no free port | `no_free_port_fails_the_save_and_stores_nothing`, `the_picker_fails_when_the_source_fails`, `..._gives_up_after_sixteen_misses` | Reducer entry |
| FR-010 explicit duplicate | `zero_and_an_explicit_duplicate_...` (weak, finding 5) | Reducer entry |
| SC-001 starts and connects with no port entered | none automated | visual pass only; start not run in an image-less environment |

Untested or weakly tested criteria: SC-001, FR-007 out-of-range, FR-005 end to end. `criteria_covered: 9 # SC-001 and FR-008 have no automated test; US2.1, US3.1 partial

## What was not audited

- Full workspace suite and `mise run gate`: not run; only the three feature test binaries (58 tests, all green). Several other worktrees held the shared build lock for most of the session.
- Mutation was deliberate-mutant sampling (10), not a tool run; no coverage tool exists in the profile.
- The `sandbox-real-runtime` acceptance suite (needs a container runtime) was not run.
- Client tests beyond `daemons_feature` and the client lib were not run, so M10 may be caught by a test outside the scope run.
- Visual pass evidence (`visual/*.png`, `results.md`) was read, not re-run; step 5 there is inconclusive.
- Performance, flakiness over repeated runs, and Windows/macOS behavior: not assessed. `the_os_port_is_non_zero_and_binds` binds a real loopback socket and can race with other processes in principle; not observed to flake.
- Unrelated working-tree change: `specs/805-container-daemon-random-port/autopilot.md` is modified and uncommitted; I did not edit it.
