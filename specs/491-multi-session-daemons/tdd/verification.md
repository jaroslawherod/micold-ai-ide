---
feature: 491-multi-session-daemons
verdict: FAIL
standard: .specify/extensions/tdd/templates/tdd-test-quality-rubric.md # rubric graded against
verified_at: 0a15aa9c # short SHA audited (M1-M4 on this branch)
behaviors: 27 # listed in tdd/test-list.md (M1 only); M2-M4 behaviors (T018-T045) were never listed
proven: 0
likely: 22
test_after: 5
no_test: 0
high_smells: 5
criteria_total: 25 # acceptance scenarios US1..US5 (the list's own "spec_criteria: 8" is stale)
criteria_covered: 23
mutation_score: 86 # deliberate mutants, 7 sampled (6 caught, 1 survived); no mutation tool in the profile
mutants_survived: 1 # inside a DONE behavior (A3), not equivalent
suite: 6326 passed, 0 failed, 9 ignored, 498 binaries (cargo test --workspace, MICOLD_SKIP_GH_LAUNCH_TEST=1, exit 0); sandbox-real-runtime targets compile out and were not run
---

# TDD Verification: Support multiple session daemons

**Verdict: FAIL.** A real surviving mutant sits inside a behavior marked DONE (the daemon chosen at worktree creation is never stored when the create completes, and no test notices), and five HIGH smells prove nothing. The suite is green and broad, but the TDD evidence is thin: no recorded red for M1 client, M2 and M3, a test list frozen at planning, and every task ticked.

Independence: this audit was run from a fresh context, not by the session that wrote the tests. Every cited line was opened in the audit.

## Test-first evidence

History caveat: every 491 commit is a `wip(491)` or per-milestone commit that adds tests and source together, so ordering cannot be shown from git for any behavior. Nothing reaches PROVEN.

| Behavior | Class | Evidence |
| --- | --- | --- |
| U1-U22 (core registry, migration, bindings, isolation) | LIKELY | cycle log records the red command and 23 failures against `todo!()` stubs; `3635cb57` adds tests and source in one commit, so order is uncorroborated |
| U23, U24 (save_binding survives stale save; refusal names daemon) | TEST_AFTER | cycle log: "without a separate red run"; tests and source land in `c0271741` / `bff30311` together |
| A1, A2, A3 (upgrade, label, form choice) | TEST_AFTER | same entry; `bff30311` adds all three tests with the source |
| M2 behaviors T018-T022, T048, T049 (not on the list) | TEST_AFTER | log: "no separate recorded red run"; `00b9a577`, `2847d3c4` add tests with source |
| M3 behaviors T030-T032 (not on the list) | TEST_AFTER | log: no red recorded; the geometry red was a defect in the test's own harness, not a recorded failing run of the behavior |
| M4 e2e (T040-T044) | LIKELY | two harness reds recorded with causes; the tests are the deliverable, so there is no production change to order against |

Existing tests changed by the feature (checked commit by commit): no assertion was removed, loosened, skipped, renamed out of a filter, or excluded, and no threshold was lowered. The edits are signature updates (`reconcile_catalog(.., DaemonId(1), ..)`, `daemon: String::new()`), counts bumped for new members (`SettingsSection::ALL` 5 to 6, `Icon::ALL` 49 to 50, `DIALOGS` 18 to 20, root vocabulary 21 to 22), a `DEFERRED` list added and then emptied, and key-list additions. Clean on this check.

Tasks vs list: T003-T045 are all ticked `[x]`, but every row of `tdd/test-list.md` still reads `PENDING` and its `updated_at` is the planning commit. See finding 1.

## Findings

| # | Severity | Finding | Evidence |
| --- | --- | --- | --- |
| 1 | HIGH | Completion claims with no list evidence: 27 behaviors all `PENDING`, list scoped to M1 only, M2-M4 test targets (about 19 files) have no behavior ids, no cycle entries per behavior, and `spec_criteria: 8` is stale (spec has 25 scenarios) | `tdd/test-list.md:5-8,18-68`, `tasks.md:12-95` |
| 2 | HIGH | Surviving mutant: dropping the bind on create completion passes every test. A3's "choice stored as a binding" is asserted by calling `Workspace::bind` and `save_binding` directly, never through the form submit and `PendingOp::WorktreeCreate` completion (`pending_daemon` is asserted only for ops in `two_daemons`, not creates). Doubled subject: the test exercises the store, not the form flow | `crates/micold-client/src/shell/daemon_sync.rs:1143` (mutant), `crates/micold-client/tests/worktree_form_daemon_choice.rs:103-110` |
| 3 | HIGH | Tautological: the "close" step saves only `projects`, never settings, then `settings.load()` is compared with a value loaded earlier, with no write between. The US4 scenario 4 reopen leg (registry survives close and reopen) is not exercised; migration persistence is pinned only in `daemons_migration.rs` | `crates/micold-client/tests/upgrade_single_daemon.rs:108-131` |
| 4 | HIGH | Vacuous: the temp dir is never handed to anything; the registry does no I/O, so the directory is empty whatever the code does. Cannot fail. U20 ("removing an entry writes nothing to disk") should save the settings store, remove, and assert the file bytes are unchanged | `crates/micold-core/tests/daemons_bindings.rs:170-181` |
| 5 | HIGH | No drop is performed: the test reconciles one Running snapshot and asserts Running. The comment admits "Nothing is folded when a daemon drops". Name claims behavior that is never driven; the lost-label behavior is covered elsewhere (`worktree_unavailable_label.rs:125-136`) | `crates/micold-client/tests/catalog_two_daemons.rs:194-211` |
| 6 | HIGH | Tautological: asserts the fixture contains the ids the fixture inserted; the "no runtime field" claim is a type annotation | `crates/micold-client/tests/daemon_rows_uniform.rs:52-62` |
| 7 | MED | Conditional logic: `if let Some(change)` decides whether the id is asserted (the isolation assertion itself is unconditional) | `crates/micold-core/tests/daemon_state.rs:139-146` |
| 8 | MED | The multi-daemon e2e folds states by hand (`states.apply`) from raw `connect_at` connections, not through the client's actors; the host leak check runs only if an event arrives in 500 ms (`_ => {}`), so the real proof is the final `converse`. The log records the leak assertion's mutation check as "not run" | `crates/micold-daemon/tests/sandbox_real_multi_daemon.rs:54-58,142-164` |
| 9 | MED | `LabelFacts` (`paths_mapped`, `runtime_missing`, `lost_sessions`) are hand-built, so the code that derives those facts, including FR-019 mapped paths, has no test | `crates/micold-client/tests/worktree_unavailable_label.rs:86-110` |
| 10 | MED | `a_binding_for_a_project_with_no_state_file_is_refused` and `binding_to_an_unknown_daemon_is_refused` assert `is_err()` only, not which refusal | `crates/micold-core/tests/daemons_bindings.rs:211-213`, `crates/micold-client/tests/daemon_removal.rs:150` |
| 11 | MED | Bypassed test utility / duplicated setup: the container-runtime constructor and the host+box registry are re-built in about a dozen files although `support/daemons_fixture.rs` provides them | `daemons_registry.rs:8`, `daemons_bindings.rs:16`, `daemons_isolation.rs:10`, `per_daemon_actors.rs:15,23`, `daemon_routing.rs:15`, `daemon_version_mismatch.rs:63`, `worktree_unavailable_label.rs:13`, `worktree_daemon_label.rs:25`, `worktree_form_daemon_choice.rs:16`, `daemons_section_geometry.rs:27`, `daemon_adopt_on_restart.rs:19` |
| 12 | MED | Eager / assertion roulette: several behaviors and unlabelled asserts per test | `daemons_bindings.rs:216-239`, `worktree_unavailable_label.rs:83-123`, `catalog_two_daemons.rs:91-117` |
| 13 | MED | Snapshot accepted in the same commit that changed it ("accept the layout snapshot with the daemon chip"); the 1330-line fixture diff mixes other PRs' rows, so review of the 491 part cannot be shown | `crates/micold-client/tests/fixtures/layout_snapshot.txt` (`f29b5eea`, `2c0d7521`) |
| 14 | MED | US1 scenario 3 (terminal in the bound daemon's container) is covered only by `terminal_container()` on the registry, not the `ui/terminal.rs` call site; US5 scenario 4 (no runtime means the normal task does not run them) rests on the `cfg` gate with no assertion | `daemon_routing.rs:84-89`, `sandbox_real_*.rs:7-13` |
| 15 | LOW | Dead or misleading: `let _ = json;` and an unused read, `let _ = host;`, `features_daemons.rs` claims SC-004 isolation but asserts defaults, `a_sessions_survive_bs_snapshot` name | `daemons_bindings.rs:88,97`, `worktree_form_daemon_choice.rs:82`, `features_daemons.rs:1-15`, `catalog_two_daemons.rs:92` |

Properties: the suite is deterministic (no real clock, network or sleep outside the real-runtime targets, which poll with budgets), isolated (tempdirs), and fast (the new core targets run in milliseconds). Failure messages are mostly specific. Refactor sensitivity is moderate: many client tests reach into `State` fields and `links::*` helpers directly.

## Mutation results

No mutation tool in the profile (`mutation: null`). Seven deliberate mutants, one at a time, each restored with `git checkout -- <file>` and re-run green (core: 5 targets, 29 tests; client: bins 522 tests plus targets). "Caught" means the command failed; the mutants are type-correct and the post-restore builds are green, so compile failure is not the cause.

| Mutant | Behavior | Result | Judgment |
| --- | --- | --- | --- |
| `daemons.rs:558` unknown bound id falls back to the legacy default | U17 | Caught | pinned by `a_key_naming_an_unknown_id_is_no_daemon` |
| `daemons.rs:207` case-sensitive duplicate names | U3 | Caught | pinned |
| `daemons.rs:388` debounce `<` to `<=` | M2 state machine | Caught | pinned by the 3-failure test |
| `daemons.rs:521` legacy daemon loses its legacy state dir | U22 | Caught | pinned |
| `daemon_sync.rs:1143` bind on create completion dropped | A3, FR-006 | **Survived** | not equivalent: a newly created worktree would silently run on the legacy default; finding 2 |
| `features/daemons.rs:427` running-session count forced to 0 | US3 s5 | Caught | pinned by `daemon_removal.rs` |
| `links.rs:146` route ignores connection state | FR-004 | Caught | pinned by routing and label tests |

Score: 6 of 7 (86 percent) over a sample of 7 mutants across 5 changed source files. This is a sample, not a measured score.

## Traceability

| Criterion | Tests | End to end |
| --- | --- | --- |
| US1-1 concurrent sessions | `two_daemons` (main_tests), `sandbox_real_multi_daemon` | Yes |
| US1-2 row names daemon | `worktree_daemon_label` A2 | Yes (state projection) |
| US1-3 terminal on bound daemon | `daemon_routing::a_terminal_execs...` | Weak (finding 14) |
| US1-4 choose daemon at create/attach | A3 | Weak (findings 2, 14 mutant) |
| US2-1..2 isolation, unavailable label | `per_daemon_actors`, `worktree_unavailable_label`, e2e | Yes |
| US2-3 version mismatch | `daemon_version_mismatch` | Yes (unit of the refusal fold) |
| US2-4 reconnect | `daemon_state`, `daemons_feature` | Yes |
| US3-1..7 Daemons section | `daemons_feature`, `daemon_removal`, `daemons_section_geometry`, visual pass | Yes |
| US4-1..3 upgrade | A1, `daemons_migration` | Yes (hand-assembled launch) |
| US4-4 close and reopen | `upgrade_single_daemon` | Weak (finding 3) |
| US4-5 three platforms | none | No |
| US5-1..3 real runtime | `sandbox_real_multi_daemon*` | Yes, not re-run here |
| US5-4 no runtime means skipped | `cfg` gate only | No assertion |
| US5-5 uniform rows | `daemon_rows_uniform` | Partial (finding 6) |

Untested criteria: US4-5, US5-4. Tests tracing to nothing: none found; the unlisted M2-M4 tests carry task ids in their headers but no behavior ids. No `traces` value points at a missing test.

## What was not audited

- The `sandbox-real-runtime` e2e targets (T040-T043) were not executed: they need a freshly built `micold-daemon:dev` image and a container runtime, and rebuilding that tag mutates a shared resource. Their results rest on the cycle log (3 passed on podman).
- Mutation was a 7-mutant sample, not a tool run; no coverage tool exists in the profile.
- macOS and Windows behavior (FR-019, US4-5): no test and no runner here.
- Visual-pass evidence (`visual-pass/`) was not re-judged; PNGs are outside the rubric.
- Performance and load: no criterion, no test.
- `docs/daemon.md` and `docs/user-guide/settings.md` show uncommitted edits in the worktree that this audit did not make and did not inspect.
