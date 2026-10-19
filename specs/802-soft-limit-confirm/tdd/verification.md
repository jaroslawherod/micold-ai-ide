---
feature: 802-soft-limit-confirm
verdict: FAIL
standard: .specify/extensions/tdd/templates/tdd-test-quality-rubric.md
verified_at: 7313f5c05
behaviors: 18
proven: 0
likely: 0
test_after: 15
no_test: 2
high_smells: 3
criteria_total: 16
criteria_covered: 11
mutation_score: 50 # scope: 12 hand-made mutants in the 802 changed source files; no mutation tool (cargo-mutants absent), so a sample, not a tool score
mutants_survived: 6 # 1 equivalent-in-practice but unpinned, 5 judged real gaps
suite: 7523 passed, 1 failed (unrelated flaky daemon test, passes on rerun), 10 ignored, 422s; run stopped at the failing daemon binary, later daemon binaries not run
---

# TDD Verification: Soft session limit, confirm before a parallel session (802)

**Verdict: FAIL.** No behavior has recorded red evidence (the feature has no `tdd/test-list.md` or `cycle-log.md`, and tests and source landed in the same commits), a rewritten 776 test lost its value check, two tests assert the test's own fixture instead of the code, and 5 of 12 hand-made mutants in `DONE` behaviors survived.

This audit is independent: the auditing session did not write the feature.

## Scope

The requested range `1927ab5ca~1..origin/main` holds 429 files from 169 commits, almost all other features. It does not contain the M1 code: that landed in `034d04cfb`, `75db3d75b`, `8e087e878` (parent `b5724fce9`), before the range starts. Audited instead: M1 `b5724fce9..8e087e878` (PR #1048) and M2 `ecced9804` (PR #1055). Source files mutated are the ones those two diffs changed.

## Test-first evidence

No `tdd/test-list.md`, no `tdd/cycle-log.md`. Behaviors are derived from `tasks.md` T002-T012 and the tests. Fail closed: no recorded red means `TEST_AFTER`. History cannot rescue it: `034d04cfb` (a `wip` commit) and `75db3d75b` each change tests and source together; `ecced9804` adds the M2 test and the description change in one commit. `tasks.md` T011 says "observed failing before T012", but no command or output is recorded anywhere.

| Id | Behavior | Test | Class | Evidence |
|---|---|---|---|---|
| B1 | A start that opens the confirmation keeps its CLI (T002) | `additional_session_provider.rs::a_start_that_opens_the_confirmation_keeps_its_cli` | TEST_AFTER | no red; test+source in `034d04cfb` |
| B2 | Declining clears target and CLI | `...::declining_clears_the_target_and_the_cli` | TEST_AFTER | same |
| B3 | Confirming clears target and CLI | `...::confirming_clears_the_target_and_the_cli` | TEST_AFTER | same |
| B4 | Opened without a CLI uses the default | `...::a_confirmation_opened_without_a_cli_uses_the_default` | TEST_AFTER | same |
| B5 | Start on occupied location opens it, sends nothing (FR-001, FR-007) | `main_tests.rs:1542` | TEST_AFTER | same |
| B6 | Refusal naming the occupant opens it, no notice (FR-004) | `main_tests.rs:1561` | TEST_AFTER | same |
| B7 | Refusal after a project switch stays a notice | `main_tests.rs:1581` | TEST_AFTER | same |
| B8 | Other refusals and an automatic refusal stay notices | `main_tests.rs:1594` | TEST_AFTER | same |
| B9 | Confirm sends one flagged create with the chosen CLI | `main_tests.rs:1620` | TEST_AFTER | same |
| B10 | Cancel sends nothing, catalog unchanged | `main_tests.rs:1630` | TEST_AFTER | same |
| B11 | Blocked start and row entry still open it | `main_tests.rs:1644` | NOT_APPLICABLE | regression of 776 behavior |
| B12 | With several sessions the dialog names the main one | `main_tests.rs:1672` | TEST_AFTER | same; see finding 2 |
| B13 | After confirming both sessions are listed, original main (T010) | `main_tests.rs:1687` | TEST_AFTER | same; see finding 1 |
| B14 | Dialog text is generic when occupant unknown (T005/T008) | `confirm_additional_session_generic.rs::the_text_is_generic_...` | TEST_AFTER | same |
| B15 | Dialog text names the occupant | `...::the_text_names_the_occupant_when_known` | TEST_AFTER | same |
| B16 | Tool descriptions state the person/agent rule (T011/T012) | `mcp_tools_catalog.rs::session_start_tools_state_...` | TEST_AFTER | `ecced9804` one commit |
| B17 | `dialog()` looks up and names the main session (T008) | none | NO_TEST | m11 survived |
| B18 | A refusal of an already-confirmed create stays a notice (T007, `additional`) | none | NO_TEST | m1 survived |

## Existing tests changed

- `crates/micold-client/src/main_tests.rs` (was ~1423): `a_refused_session_create_shows_the_refusal_text_and_changes_nothing` was deleted and replaced. Before: `assert_eq!(said, text, "the refusal is shown as the service worded it")` and `assert_eq!(workspace.sessions, before)`. After, in `other_refusals_and_an_automatic_refusal_stay_notices` (1594-1617): `assert!(visible_notice(&app).is_some(), ...)` twice. The notice text is no longer checked, so a wrong or empty notice passes. The replacement is justified by the behavior change (the old scenario now opens the dialog), but the "shown as the service worded it" value check was loosened to a truthiness check and not restored for the cases that still notify. Counts as a weakened existing assertion (finding 3).
- `crates/micold-client/src/ui/confirm_additional_session.rs` and `features/session.rs` `open_in`: the guard `.filter(|location| !state.start_offered(location))` was dropped from the dialog's `open_in`. No test pins the new behavior (dialog stays open, generic text, after the occupant is deleted) and none was removed. Source change, noted.
- `session_start_press.rs`, `root_state_is_shared.rs`: additive (new `automatic: false` field; new allowlist entry with a reason). Not weakened.
- `main_tests.rs` issue_source test: `PathBuf::from` became `std::path::PathBuf::from` to resolve an import change. No assertion change.
- No skips, `#[ignore]`, config exclusions or threshold changes.

## Findings

| # | Severity | Finding | Evidence |
|---|---|---|---|
| 1 | HIGH | Tautological: `after_confirming_both_sessions_are_listed_and_deleting_one_leaves_the_other` pushes the helper into `workspace.sessions` itself, then asserts `len() == 2` and that the original is `main` (its own fixture). It then `retain`s the helper away itself and asserts one session is left. Only `session.active == helper_id` (from `OperationOk`) exercises code. US2 (both run, stop/delete one leaves the other) is therefore not tested. Should drive `CatalogChanged` from the daemon, then a real delete, and assert the reconciled catalog. | `main_tests.rs:1687-1745` |
| 2 | HIGH | Name says "the confirmation names the main one", assertion is `occupancy::main_session(active_sessions, location) == ids[0]` on the test's own fixture. `ui::confirm_additional_session::dialog` (the code that names the session) is never called. Mutant m11 (name replaced by a constant) survived. Should call `dialog`/`body_text` through the state and assert the main session's label is in the text. | `main_tests.rs:1672-1685`; `ui/confirm_additional_session.rs:75-77` |
| 3 | HIGH | Loosened existing assertion: value check on the notice text replaced by `is_some()` (see above). Should assert the notice equals the service's message in both cases. | `main_tests.rs:1594-1617` (was `assert_eq!(said, text)`) |
| 4 | HIGH (survivor in DONE behavior) | m1: dropping `&& !additional` in the refusal arm survives. A refusal of the already-confirmed create would reopen the dialog in a loop; T007/FR-004 say it stays a plain notice. No test refuses a flagged create. | `shell/daemon_sync.rs:1787`; B18 |
| 5 | MED (survivor) | m4/m5: neither `kind == Refused` nor the UUID parse is pinned. A non-`Refused` error carrying a UUID `detail`, and a `Refused` with a non-UUID `detail`, are both untested; T007 says typed fields decide. | `daemon_sync.rs:1782-1785` |
| 6 | MED (survivor) | m3/m7: the `automatic` guard at the start path (`daemon_sync.rs:2593`) and the `automatic: true` flag of the startup auto-start (`main.rs:359`) are unpinned; the automatic case is only tested through the refusal arm on a free location. Judgment: not equivalent, because the flag is the whole FR-004 mechanism, but unreachable today (auto-start location is free). | `daemon_sync.rs:2593`, `main.rs:359` |
| 7 | MED | Eager: `other_refusals_and_an_automatic_refusal_stay_notices` runs two scenarios in one test with a second app built mid-test; a failure does not say which. | `main_tests.rs:1594-1617` |
| 8 | MED | Weak assertion: `the_blocked_start_and_the_row_entry_still_open_the_confirmation` asserts `blocked_start == Some(id)`, the state field, not that the confirmation opens; also two behaviors in one test. | `main_tests.rs:1644-1670` |
| 9 | MED | Redundant: `a_free_location_offers_the_start` tests the existing `start_offered`, not 802 code. | `tests/additional_session_provider.rs:45-49` |
| 10 | MED | No test that an agent create opens no confirmation in any window (US3.1 second clause); rests on the 776 daemon tests, which cannot see window state. | `tasks.md` T014 |
| 11 | LOW | `creates()` returns `(CreateTarget, bool)` with `type CreateTarget = (AiCli,)`, a one-element tuple alias for no reason; `refuse()`/`start()` helpers are defined in `main_tests.rs` rather than the shared support module. | `main_tests.rs:1448-1463` |
| 12 | LOW | Unrelated flaky daemon test: `mcp_audit_log::every_successful_mutating_call_writes_one_info_line` failed once ("terminal does not accept a pasted text") and passed on rerun. No 802 file touched it. | `crates/micold-daemon/tests/mcp_audit_log.rs:213` |

Properties: the new tests are deterministic and fast (593 client bin tests in 11.5s); no sleeps, clocks or network. They are coupled to `PendingOp`/`pending_ops` internals through `pending_create()`, which is acceptable for a shell with no integration seam but will move with any refactor of `PendingOp`. No credentials found.

## Mutation results

No mutation tool; 12 deliberate mutants, each applied alone, test run, restored with `git checkout`, tree confirmed clean and the touched suites re-run green. Client mutants ran `cargo test -p micold-client --bin micold-ai-ide --test additional_session_provider --test confirm_additional_session_generic --test features_session` (662 tests); the M2 mutant ran `cargo test -p micold-core --test mcp_tools_catalog`. A first run of m12 used the client set by mistake and was discarded; the table is the rerun.

| Mutant | Behavior | Survived | Judgment |
|---|---|---|---|
| m1 `daemon_sync.rs:1787` drop `&& !additional` | B18 | Yes | Real gap, finding 4 |
| m2 `:1788` drop `&& !automatic` | B8 | No | caught by `other_refusals_...` |
| m3 `:2593` drop `!automatic &&` at start | B5/FR-004 | Yes | Unpinned, finding 6 |
| m4 `:1782` `kind == Refused` to `true` | B6/B8 | Yes | Real gap, finding 5 |
| m5 `:1785` UUID parse to `is_some()` | B6/B8 | Yes | Real gap, finding 5 |
| m6 drop active-project check | B7 | No | caught by `a_refusal_after_a_project_switch_...` |
| m7 `main.rs:359` `automatic: true` to `false` | B8 | Yes | Unpinned, finding 6 |
| m8 `session.rs` `additional_request` ignores stored CLI | B1/B9 | No | caught by 2 tests |
| m9 `session.rs` `additional_opened` stops storing CLI | B1/B9 | No | caught by 2 tests |
| m10 `additional_confirmed` keeps the CLI | B3 | No | caught by `confirming_clears_...` |
| m11 `confirm_additional_session.rs:77` occupant name replaced by a constant | B17/B12 | Yes | Real gap, finding 2 |
| m12 `tools.rs` start_session description drops "nobody is present to confirm" | B16 | No | caught by the pinning test |

Score 6 of 12 caught (50%), scope: 5 source files from the 802 diffs, sampled by hand (not exhaustive; tool: none). Survivors map to B5, B6, B8, B12, B17, B18, all marked done in `tasks.md`.

## Traceability

| Criterion | Tests | End to end |
|---|---|---|
| US1.1 (opens, names worktree and session, creates nothing) | B5, B12 (naming untested) | Partial: shell handler, not the dialog render |
| US1.2 (confirm creates helper, selected) | B9, B13 | Yes for the flagged create; selection via `OperationOk` |
| US1.3 (decline creates nothing, no error) | B10, B2 | Yes |
| US1.4 (blocked start confirms) | B11 | Yes (regression), weak assertion |
| US2.1, US2.2 (both run; stop/delete one) | B13 | No: test simulates the catalog itself (finding 1) |
| US3.1 (agent refused, no window confirmation) | daemon `mcp_create_session.rs:507` | Refusal yes; "no window confirmation" untested |
| US3.2 (agent with flag creates) | daemon `mcp_create_session.rs:533` | Yes (existing 776 test) |
| US3.3 (docs and descriptions state it) | B16 | Descriptions yes; `docs/user-guide/agent-tools.md` not tested |
| FR-001 | B5, B14, B15 | Partial: naming through `dialog()` untested |
| FR-002 | B9, B1 | Yes |
| FR-003 | B10 | Yes |
| FR-004 | B5, B6, B11, B12 | Partial: B12 and the confirmed-refusal case |
| FR-005 | daemon 507/533 | Yes (existing) |
| FR-006 | B16 | Descriptions yes; guide not pinned |
| FR-007 | B5, B9 | Yes |

Covered fully: 11 of 16 (US1.2, US1.3, US1.4, US3.1 refusal, US3.2, US3.3, FR-002, FR-003, FR-005, FR-006, FR-007; counting US3.1 and US3.3 as covered for their tested clause). Not or only partly covered: US1.1, US2.1, US2.2, FR-001, FR-004. Tests tracing to nothing: `a_free_location_offers_the_start` (existing behavior, finding 9). The suite's real entry points are the shell handlers `on_session_start_requested` and `on_daemon_event` called directly; `update_inner`, the sidebar press and the dialog render are not driven (`tests/session_start_press.rs` only gained the field).

## What was not audited

- Phase 2 against a test list and cycle log: neither exists. All classes are derived.
- Range as requested: the 802 M1 code is outside `1927ab5ca~1..origin/main`; audited by commit range instead. The other ~420 files in the requested range belong to other features and were not read.
- Mutation: hand-picked 12 mutants, not a tool run; `main_tests.rs`, `tests/*.rs` test files not mutated. Coverage tool absent.
- Suite: the full `cargo test --workspace` stopped at the failing daemon binary `mcp_audit_log` (flaky, passes alone). Daemon binaries after it, doc tests, `scripts/tests/*.test.sh`, clippy and fmt were not run; `mcp_create_session.rs` was read but not re-run.
- Windows/macOS behavior, the real GUI (quickstart §B visual pass), and the sandbox acceptance suite.
- Docs content of `worktrees-and-sessions.md` and `agent-tools.md` beyond reading the diff.

## Close-unit resolution

Findings 1-9 were fixed by T015-T021 (finding 7 by splitting the test in T018; mutants for
findings 2, 4, 5, 6 now killed). Accepted gaps: finding 10 (the agent-create clause, no
`additional_target` on an agent create, rests on the daemon's tests, `mcp_create_session.rs`) and
the missing test-first record (no `test-list.md` / `cycle-log.md` was kept for this feature, so
ordering cannot be proven retroactively). Findings 11 (LOW) and 12 (unrelated flaky test) are
untouched.
