# Autopilot ledger — #1533 start-script

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Spec 1533 (An optional start script prepares each daemon); labels: `effort:high`, `flow:feature`, in-progress
- **Kind**: feature
- **Effort**: high
- **Issue**: #1533
- **Worktree branch**: feat/1533_an-optional-start-script-prepares-each-daemon
- **Started**: 2026-11-01
- **Phase**: tasks
- **Next step**: design PR merge, then milestone M1

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #1630 | Design (spec, plan, tasks) | open | |

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1 | T001-T009 | full | Script runs before serving; exports applied; StartupReport + protocol bump | | pending |
| M2 | T010-T017 | full | env_file secrets, precedence, run/failure policies, reload, host guide | | pending |
| M3 | T018-T022 | full | Settings Start script group (hidden per kind until delivery) | | pending |
| M4 | T023-T029 | full | Host+SSH delivery and failure display (after #1526) | | pending |
| M5 | T030-T033 | full | Container delivery (after #1527) | | pending |
| M6 | T034-T036 | full | Pod delivery (after #1528) | | pending |

T037 (quickstart results) is left to the close unit. M1 may need the `docs-not-needed` label.

## Decisions

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|
| D1 | spec | Spec location | `specs/1533-start-script/` narrows parent `specs/daemon-config-file/` (Story 5); tasks T068-T077 stay in the parent tasks.md | agent-resolved | parent spec.md |
| D3 | M1 | T004 says `resolve` moves onto `run_and_diff`; keep it on `run_bounded`? | Keep: `run_and_diff` is the new streaming variant for the start script; `resolve` stays on `run_bounded` per research R4 (no behaviour change to env_include resolution) | agent-resolved | research.md R4 |
| D2 | clarify | Round 1 of speckit-clarify | No critical ambiguities; record location, inline-script storage and warning transport are plan-level | agent-resolved | spec.md |

## Review rounds

| Review | Round | Snapshot | Verdict |
|---|---|---|---|
| Spec 1533 | 1 | 78dc2822:dbb82d13 | CHANGES: 2 MAJOR (PATH contradiction, unmerged deps assumption) fixed; 3 MINOR fixed |
| Spec 1533 | 2 | 2c305bf0:7f105e4f | CLEAN (3 MINOR: per-session variable set left to the contract, script_file required checked against the parent contract table, ledger row added) |
| Plan 1533 | 1 | 9afe4badc3b5:6e4fc30a516c | CHANGES: 3 MAJOR (timeout mechanism, align_container_home order, host spawn failure trigger) and 3 MINOR fixed; re-review next |
| Plan 1533 | 2 | ac54eddf2e07:6e4fc30a516c | CLEAN (3 MINOR fixed, prose only) |
| Tasks 1533 analyze | 1 | 43696b27:c6372cd5 | 0 CRITICAL, 1 HIGH (Windows tests/doc) fixed; MEDIUM/LOW fixed except task splitting (declined) |
| Tasks 1533 | 1 | 43696b27:c6372cd5 | CHANGES: 1 MAJOR (M3 half-wired UI: group hidden per kind until delivery) and 3 MINOR fixed |
| Tasks 1533 | 2 | 80d6c8d2:c6372cd5 | CHANGES: 1 MAJOR (guide task T023 moved to M4) and 2 MINOR fixed; prose/ordering only, no round 3 |

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|
| tasks | analyze C3 | split T008, T015, T024 | each is one cohesive file or change; no rubric item on task size |

## Handover

Unit M1 handed over at 150k, nothing pushed, no PR for M1 yet. Previous PR #1630 merged (SHA 1ede52197f6dc7022ea768f7fb884e64b175e83c); branch was reset to origin/main by branch-start.sh.

Done (uncommitted work committed as one WIP commit, NOT yet compiled or tested, because the shared build lock was held by another worktree's gate for hours):
- `crates/micold-core/src/env_include.rs`: `run_and_diff(script, shell, cwd, timeout, extra_env, on_line) -> ScriptRun{end: ScriptEnd, exports}` (POSIX sh wrapper + PowerShell wrapper, watchdog, LineSplitter) and a `run_and_diff_tests` module (T002, T004). `resolve` deliberately left on its `run_bounded` path (research R4 says so); record as a decision.
- `protocol/messages.rs`: `StartupOutcome`, `StartupReason`, `StartupReport`, `ConfigStatus.startup`; `version.rs` 75 -> 76; `tests/protocol_roundtrip.rs` updated + new test (T003, T005). `config_watch.rs` sets `startup: None` (T006: state.rs needs no change).
- `crates/micold-daemon/src/startup_script.rs` (StartupPlan, Tail, exportable, run, failure_block, read_result, unit tests), `lib.rs` module, `logging::append_line`, `main.rs` runs the plan after `align_container_home` (T001, T008, T009), `tests/startup_script.rs` (T007).
- tasks.md NOT yet ticked (T001-T009).

Next: `mise run test-core` (was queued; log in scratchpad), fix compile errors/test failures (likely: unused `Stream` import in daemon, schema_hash test, exact `tempfile`/`std::path::absolute` use, the posix wrapper behaviour in `/bin/sh`), then `cargo test -p micold-daemon --test startup_script` via `scripts/build-lock.sh`, `cargo test -p micold-daemon --lib startup_script`, tick T001-T009, then verify.md, review A (high), review B, scoped gate, full gate, PR (title `feat(1533): run the start script before the daemon serves (#1533)`; maybe `docs-not-needed` label). Another worktree (#1535) also edits ConfigStatus: expect a rebase conflict on messages.rs/version.rs.

## Open escalation

None.

## Follow-ups not done

- Dependencies #1526, #1527, #1528 are still OPEN on GitHub (no delivery commits on main); #1524 is closed. Script delivery (T075) and per-kind failure surfacing need them: plan/milestones must order or stub accordingly.
