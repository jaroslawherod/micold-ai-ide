---
feature: 582-attach-provider-worktrees-sessions
verdict: FAIL
standard: .specify/extensions/tdd/templates/tdd-test-quality-rubric.md # rubric graded against (no override or preset found)
verified_at: 8947038b
range: 1498ad25..8947038b # parent of 0706196c (first M1 commit of PR #587) to HEAD, M1-M5
behaviors: 15 # 12 test/implementation pairs from tasks.md plus 3 untested production units (no tdd/test-list.md exists)
proven: 10
likely: 0
test_after: 2
no_test: 3
high_smells: 0
criteria_total: 13 # the 13 acceptance scenarios of the four stories
criteria_covered: 11
mutation_score: unmeasured # no mutation tool in the profile; 10 deliberate mutants sampled: 7 caught, 3 survived
mutants_survived: 3 # none equivalent
suite: 4948 passed, 0 failed, 9 ignored, 425 binaries, 158s (MICOLD_SKIP_GH_LAUNCH_TEST=1, mise run test); re-run green after the last restore
---

# TDD Verification: Attach a provider's existing worktrees and sessions (#582)

**Verdict: FAIL.** Three deliberate mutants survived inside behaviors the cycle log calls done. The sharpest:
deleting the call that starts a session after a resume (`daemon_sync.rs:942`) fails no test, so acceptance
scenario US2.2 ("the provider resumes that conversation") has no test of its final step.

Independence: this audit was run from cold context (a fresh agent that did not write the tests). Every test
file below was re-read in full. No tests, source or config were edited; all ten mutants were restored and the
full suite re-run green. No fresh-context subagent was available for the smell pass, so the pass was done by
the auditor directly and every cited line was opened.

No `tdd/test-list.md` exists (the design phase did not run `tdd.plan`; `cycle-log.md` says so). The rubric
makes that `BLOCKED` on its face. The audit proceeded anyway against `spec.md`, with the test-first task
pairs of `tasks.md` standing in as the behavior list, and is therefore weaker on ordering evidence than a
planned feature would be. Fail-closed applies: the missing list is itself finding 5.

## Test-first evidence

History: every implementation commit bundles tests and source (`0706196c`, `73f7a1d4`, `2d8e16a2`,
`b2c7f55f`, `5eac4f49`, `6a99e9f6`); no test-only commit exists. So history cannot show order inside a
commit. By the rubric, same-commit plus a recorded red is `PROVEN`; the cycle log is the only red evidence.
No log entry is contradicted by history. Only the T003 entry records the red command; the others give the
failure text only (finding 14).

| Behavior (tasks) | Class | Evidence |
| --- | --- | --- |
| T003/T004 protocol round trip | PROVEN | command and `E0599 no variant AttachDiscover`; test and source in `0706196c` |
| T005/T006 `Catalog::attach_worktrees` | PROVEN | `no method attach_worktrees`; `0706196c` |
| T007/T010 `attachable_worktrees` | PROVEN | `unresolved import attachable_worktrees`; `0706196c` |
| T008/T011 daemon `AttachDiscover`/`AttachApply` | TEST_AFTER | log says only "written with handlers not yet present"; no failure output recorded |
| T009/T012 dialog reducer | TEST_AFTER | log: "written in one pass, no recorded red ... recorded as a gap" |
| T017/T022 `discover_resumable` | PROVEN | `unresolved import discover_resumable`; `2d8e16a2` |
| T018/T021 `store_dirs` | PROVEN | `no method store_dirs`; `2d8e16a2` |
| T019/T023 daemon session apply | PROVEN | `no method attach_session`, then empty `sessions`; `2d8e16a2`, `b2c7f55f` |
| T020/T024 session rows, notes, resume in reducer | PROVEN | 16 compile errors listed; `b2c7f55f` |
| T027/T029 tool catalog and policy | PROVEN | `no variant AttachWorktree`; `5eac4f49` |
| T028/T030 MCP handlers | PROVEN | daemon crate failed to compile (non-exhaustive match); `5eac4f49` |
| T032/T033 start-up offer rules | PROVEN | 21 compile errors listed; `6a99e9f6` |
| T013 `PendingOp::Attach` plumbing, resume-then-start glue (`daemon_sync.rs:924-942`), offer request at project open | NO_TEST | no test selects any of it (finding 1, 5) |
| T014/T034 dialog and banner rendering, `ConnectionBanner::secondary_action` | NO_TEST | only the registry count bumps, the layout snapshot and the visual pass touch it (finding 12) |
| T033 request `AttachDiscover` at project open | NO_TEST | `attach_offer.rs` drives the reducer with a hand-built `OfferListed` |

Existing tests changed by the range (`git diff 1498ad25 HEAD`, test files with deletions): no weakening found.
`overlay_registry.rs` (11 to 12 dialogs, 24 to 26 states, a real `attach_worktrees` entry added),
`popover_displacement.rs` (11 to 12), `root_vocabulary_is_cross_cutting.rs` (17 to 18 variants),
`state_scan.rs` (`as_mut` into MUTATORS, `targets` into READERS), `mcp_tools_catalog.rs` (tool lists gain the
two tools), `mcp_audit_log.rs`, `mcp_binding_spawn.rs` and `mcp_read_latency.rs` (the new tools added to their
enumerations) all tighten or extend. `schema_hash.rs` moves the pinned version 27 to 28 with the protocol
change. `attach_fixture.rs`/`attach_discovery.rs:71-85` only build the outside path from the canonical
fixture base (Windows verbatim prefix); the assertions are unchanged. No test skipped, ignored, renamed out of
a filter, or excluded; no config, coverage or mutation threshold touched. `layout_snapshot.txt` was
regenerated in `73f7a1d4` (finding 12).

`tasks.md` against the evidence: T016 is `[ ]` but its visual evidence is on disk
(`visual/b2-dialog-*.png`, `b2-sidebar-header-*.png`) and commit `8e05dc09` records it (finding 15). No task
is ticked without evidence.

## Findings

Ordered by severity.

| # | Severity | Finding | Evidence |
| --- | --- | --- | --- |
| 1 | HIGH | Surviving mutant M10: replacing `view_and_start(app, ...)` with `let _ = id;` fails nothing in `cargo test -p micold-client`. The step that makes "resume" run the provider is untested; the daemon test attaches an idle entry and only checks `launch_args`. Acceptance US2.2 and FR-008 are not verified end to end. Should assert that a `Msg::Applied` for a lone `AttachItem::Session` that came back `Attached` produces a `SessionStart` to the daemon (the existing fake-daemon harness at `daemon_sync.rs:2663` and `:2847` shows how), and that an `AlreadyRunning` refusal does not. | `crates/micold-client/src/shell/daemon_sync.rs:924-942`; `crates/micold-daemon/tests/attach_apply.rs:709-746,846-861` |
| 2 | HIGH | Surviving mutant M2: dropping `rest.starts_with('/')` from `is_within` fails all 17 `attach_discovery` tests. `claude_rejects_a_matching_name_whose_transcript_cwd_is_elsewhere` uses `/somewhere/else`, never a prefix lookalike such as `<root>/.claude/worktrees-old/x`, so the boundary R3 depends on to keep another directory out is unpinned. The sibling-project tests pass for another reason (the sibling's store directory is never enumerated). | `crates/micold-core/src/attach.rs:253-257`; consumer `crates/micold-core/src/provider.rs:716`; `crates/micold-core/tests/provider_store_dirs.rs:61-79` |
| 3 | HIGH | Surviving mutant M7: removing `&& !w.included` from `attach_worktrees` passes `attach_apply`, `attach_mcp` and all of `-p micold-daemon`. The helper `live()` always sets `included: false`, so "a worktree outside the managed directory is refused" is never exercised at the catalog (the MCP test passes because resolution refuses it earlier). | `crates/micold-daemon/src/catalog.rs:526`; `crates/micold-daemon/tests/attach_apply.rs:47-58` |
| 4 | HIGH | T008/T011 and T009/T012 are `TEST_AFTER`: no failure output for the daemon tests (`attach_apply.rs:373-500`), and the whole reducer test file (`attach_dialog.rs:1-267`) was written with its code. The log admits the second. | `tdd/cycle-log.md:16-21` |
| 5 | HIGH | No `test-list.md`, and acceptance US4.1 ("Micold starts, the user is offered") is only reducer-level: nothing asserts that opening a project sends `AttachDiscover` (`PendingOp::AttachOfferDiscover`) or that the banner is shown from `offer_visible`. `PendingOp::Attach*` plumbing has no test. | `crates/micold-client/tests/attach_offer.rs:39-50`; `crates/micold-client/src/shell/daemon_sync.rs:909-940` |
| 6 | MED | `a_second_resume_while_starting_running_or_restarting_is_refused_as_already_running` claims three states but drives only `Running` through `attach_session`; `Starting` and `Restarting` are checked only as an `is_live()` truth table (`:802-816`). Mutant M1 (`is_live()` to `false`) was caught, but a guard that tests `== Running` would pass. It is also eager (idle, running, text and counts in one test). | `crates/micold-daemon/tests/attach_apply.rs:773-824` |
| 7 | MED | `two_concurrent_attaches_of_one_worktree_leave_exactly_one_record` serializes on a `Mutex` the test itself creates, so it passes for any sequential catalog. It does not exercise the daemon's own lock (FR-016, two windows or an agent and the app). | `crates/micold-daemon/tests/attach_apply.rs:150-179` |
| 8 | MED | Sleepy and time-coupled: fixed 20 ms sleep so an mtime differs (on a 1 or 2 second mtime file system the "nothing written" assertion is vacuous); two fixed 30 ms sleeps to order transcript mtimes; a 10 s wall-clock bound for SC-001. | `attach_apply.rs:90,497`; `attach_mcp.rs:331,333` |
| 9 | MED | Process-global state: `provider_home()` calls `std::env::set_var` for three provider variables once, but only some tests call it. Tests that run discovery without it (`attach_apply.rs:373,427,451,467`) read the developer's real provider home if they run first, and `set_var` races with other threads. | `attach_apply.rs:600-610`; `attach_mcp.rs:32-42` |
| 10 | MED | Vacuous or under-asserting: `assert_eq!(outcomes.len(), 1)` for both the success and the failure notification, never the message or `Level::Error`; `a_refusal_is_reported_as_an_error` asserts only the summary text, not the level its name promises. | `attach_offer.rs:196,206`; `attach_dialog.rs:210-223` |
| 11 | MED | `a_provider_with_no_config_dir_is_a_note_not_an_error` asserts `notes.len() == 1`, not which reason or provider. | `crates/micold-core/tests/attach_discovery.rs:316` |
| 12 | LOW | `layout_snapshot.txt` regenerated in the same commit as the sidebar change (436 lines). Diff read: first toolbar item 148.0 to 122.0 wide, one added 26-wide item, rest shifted; consistent with one new button. Nothing renders `ui/attach_dialog.rs` or the banner except the visual pass, so there is no structural test of either. | `crates/micold-client/tests/fixtures/layout_snapshot.txt`; commit `73f7a1d4` |
| 13 | LOW | Duplicated setup: `provider_home`, `seed_claude`/`seed_claude_as` and the encoding closure are copied between `attach_apply.rs:600-631` and `attach_mcp.rs:32-67`, and again as `AttachFixture::seed_session` in core. `FlakyStore` (`attach_apply.rs:534`) is hand-rolled, justified because `store.rs` has no failing-save fake. | files cited |
| 14 | LOW | Cycle log records the red command only for T003; the other entries give the failure text but no command. | `tdd/cycle-log.md` |
| 15 | LOW | T016 is unticked although its evidence exists (the milder inverse: `/speckit.implement` would redo it). | `tasks.md:50` |
| 16 | LOW | FR-014 end to end (`discover_resumable`, then the daemon) is exercised for Claude only; Copilot and Pi are covered at provider level (`provider_store_dirs.rs:125-203`). | `attach_discovery.rs`, `attach_apply.rs` |

No `HIGH` smell from the catalogue: no assertion-free, tautological, doubled-subject, conditional or skipped
test was found. The one hand-rolled double (`FlakyStore`) wraps the real `JsonFileStore`. Suite properties:
fast (`attach_*` binaries each under 0.2 s except the git-backed ones; the whole suite 158 s), deterministic
except findings 8 and 9.

## Mutation results

Tool: none in the profile (`mutation: null`). Ten hand-made mutants, one at a time, each restored with
`git checkout -- <file>` and verified clean. **Sampled, not exhaustive**: 10 of roughly 40 behaviors, chosen
for data-loss and refusal paths. Final full suite green after the last restore.

| Mutant | Behavior | Survived | Judgment |
| --- | --- | --- | --- |
| M1 `catalog.rs:572` `is_live()` to `false` | T019/FR-016 | No | caught by `a_second_resume_while_...` |
| M2 `attach.rs:256` drop `rest.starts_with('/')` | T018/R3 | Yes | not equivalent: lookalike directory accepted (finding 2) |
| M3 `policy.rs:140` `caller.is_default()` to `false` | T027/FR-015 | No | caught by `mcp_policy` |
| M4 `catalog.rs:545` skip rollback | T005 | No | caught by `a_failed_persist_rolls_the_records_back...` |
| M5 `catalog.rs:540` persist always | T005/FR-004 | No | caught by the mtime test (see finding 8 on its fragility) |
| M6 `catalog.rs:510` unreadable guard off | T005 | No | caught by `an_unreadable_project_refuses_every_target...` |
| M7 `catalog.rs:526` drop `!w.included` | T006 | Yes | survives all daemon tests (finding 3) |
| M8 `attach.rs:358` sort ascending | T017/R4 | No | caught by `thousands_list_newest_first...` |
| M9 `attach.rs` (client) drop provenance clause of `offer_visible` | T032/FR-012 | No | caught by `a_project_with_provenance_records_never_gets_the_offer` |
| M10 `daemon_sync.rs:942` drop `view_and_start` | T024/FR-008 | Yes | survives all of `-p micold-client` (finding 1) |

Score on the sample: 7 of 10 caught (70%), not comparable to a tool run. Coverage tooling is absent
(`coverage: null`), so no corroboration.

## Traceability

Acceptance scenarios (the `test-list.md` `traces` column does not exist; mapping built from the task text and
the tests).

| Criterion | Tests | End to end |
| --- | --- | --- |
| US1.1 three worktrees listed | `attached_worktrees_survive_a_restart...` (protocol discover), `attach_discovery` | Yes |
| US1.2 attach, shown with filter off, files untouched | `attach_apply.rs:373,427` | Yes |
| US1.3 attach again, no duplicate | `attach_apply.rs:451`, `attach_dialog` message tests | Yes |
| US2.1 5 listed, sibling 2 not | `attach_discovery.rs:180`, `attach_apply.rs:671` (Claude only) | Yes |
| US2.2 resume continues the conversation | `attach_apply.rs:671,709`, `attach_dialog.rs:318` | **No**: start step untested (finding 1) |
| US2.3 catalog session not listed twice | `attach_discovery.rs:222`, `attach_apply.rs:702` | Yes |
| US2.4 missing store, reason shown | `attach_discovery.rs:273`, `attach_dialog.rs:431` | Yes (reducer; note text only) |
| US3.1 attach by path or branch | `attach_mcp.rs:143,161` | Yes |
| US3.2 not a worktree: not_found, nothing changes | `attach_mcp.rs:187` | Yes |
| US3.3 repeat: already_attached | `attach_mcp.rs:281` | Yes |
| US3.4 Default caller refused | `attach_mcp.rs:304`, `mcp_policy.rs:476` | Yes |
| US4.1 offered at start | `attach_offer.rs` (reducer only) | **No**: open-time request and banner unverified (finding 5) |
| US4.2 records exist: nothing attached | `attach_offer.rs:76,106` | Yes (reducer) |

Untested criteria: US2.2 final step and US4.1 wiring (partial). Tests tracing to nothing: none
(`features_attach.rs` is the feature-isolation obligation, SC-004 of the app's constitution). Functional
requirements with a sampled-mutant gap: FR-007 boundary (finding 2), FR-016 (findings 6, 7).

## What was not audited

- No mutation tool exists, so the score is a 10-mutant sample, not a measurement. Not mutated: the discovery
  read-only guarantee (FR-010), `store_dirs` Copilot/Pi branches, `attach_discover` sandbox note, the MCP
  resolution order, `list_resumable_sessions` paging.
- No coverage tool, so uncovered branches in `attach.rs`, `state.rs`, `daemon_sync.rs` were not enumerated.
- `crates/micold-client/src/ui/attach_dialog.rs`, the sidebar banner and `connection_banner.rs` rendering: no
  automated test exists; the visual pass evidence in `visual/` was not re-run.
- The `sandbox-real-runtime` acceptance suite (`mise run test-sandbox`) was not run; the one sandbox-facing
  behavior here (`SandboxStoreNotReadable`) is tested with a stand-in auth token only.
- `MICOLD_SKIP_GH_LAUNCH_TEST=1` skipped the GitHub launch test; 9 ignored tests were not examined.
- Windows and macOS: the case-folding and verbatim-prefix tests ran on Linux only.
- Test run used `MICOLD_NO_BUILD_LOCK=1` because the build lock was held by a stale waiter with no cargo
  running; untracked files under `visual/` belong to another session and were not touched.
- Performance beyond the SC-001 bound and the existing `mcp_read_latency.rs` run: no criterion assessed.
