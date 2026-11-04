# Autopilot ledger — #1533 start-script

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Spec 1533 (An optional start script prepares each daemon); labels: `effort:high`, `flow:feature`, in-progress
- **Kind**: feature
- **Effort**: high
- **Issue**: #1533
- **Worktree branch**: feat/1533_an-optional-start-script-prepares-each-daemon
- **Started**: 2026-11-01
- **Phase**: implement
- **Next step**: M5: review A and B, full gate, PR (container delivery, tasks T030-T033); M6 waits for #1528

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #1630 | Design (spec, plan, tasks) | merged | 1ede52197f6dc7022ea768f7fb884e64b175e83c |
| #1665 | M1 | merged | d0f4739f6fe79a7e9bc2af87729e0f30597105c2 |
| #1669 | M2 | merged | 96a41b38670b43471a568573dd143459189e139a |
| #1673 | M3 | merged | b146a398d5597cc67bbd2e2db4994b1bc2649fcf |
| #1682 | M4 | merged | 3af058506bbe0e95b0b8e3e31801c8388b23604c |

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1 | T001-T009 | full | Script runs before serving; exports applied; StartupReport + protocol bump | #1665 | merged |
| M2 | T010-T017 | full | env_file secrets, precedence, run/failure policies, reload, host guide | #1669 | merged |
| M3 | T018-T022 | full | Settings Start script group (hidden per kind until delivery) | #1673 | merged |
| M4 | T023-T029 | full | Host+SSH delivery and failure display (after #1526) | #1682 | merged |
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
| Code M3 A (code-review high) | 1 | 7200ed14:96a41b38 | CHANGES: 1 MAJOR (inline script written before edit commits) 3 MINOR, all fixed |
| Code M3 A | 2 | fix diff | CLEAN (1 MINOR: confirm keeps form open on script write error, fixed) |
| Code M3 B (conformance) | 1 | HEAD~2 | CHANGES: 1 MAJOR (cycle-log M3 section missing, added), 1 MINOR (RestartKeysReported has no producer until M4, recorded in cycle log) |
| Tasks 1533 | 1 | 43696b27:c6372cd5 | CHANGES: 1 MAJOR (M3 half-wired UI: group hidden per kind until delivery) and 3 MINOR fixed |
| Tasks 1533 | 2 | 80d6c8d2:c6372cd5 | CHANGES: 1 MAJOR (guide task T023 moved to M4) and 2 MINOR fixed; prose/ordering only, no round 3 |
| Code M4 A and B | 1 | 76fd9421:41b0f119 | CHANGES, fixed (A.F1-F3, B.F1); B.F2/F3 declined |
| Code M4 B | 2 | fix diff | CLEAN (1 MINOR: second test module in classify.rs, not fixed) |
| Code M1 A (code-review high) | 1 | 3e019daf9ea5:761a3fb9e52b | CLEAN (3 MINOR; F1 invalid env name fixed with test; F2, F3 declined, see Declined) |
| Code M1 B (conformance) | 1 | 3e019daf9ea5:761a3fb9e52b | CHANGES: 1 MAJOR (no red evidence) answered by tdd/cycle-log.md recording the deviation (evidence only, not re-run); T004 text reworded; B could not run Verify (build lock); unit ran core + daemon startup_script tests green |

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|
| M1 | A | F2 timeout fires after child reaped, run reported TimedOut | Window is microseconds at the exact timeout; failing closed is acceptable; revisit in M2 with the run policies |
| M1 | A | F3 `trap EXIT`/`exec` in the script loses exports silently | Documented limitation for the host guide (M4); the Exited(0) run still starts the daemon |
| M4 | B | F2 no red-before-green history | Accepted deviation: M4 was committed in one commit before the cycle log was kept per behaviour; red cannot be reconstructed. Tests are the acceptance evidence; recorded in the cycle log |
| M4 | B | F3 no test for `connect_or_spawn_reading` | Declined (MINOR): the function spawns a real detached daemon, and the decision it takes (`fresh_refusal`) is unit-tested in `startup_result` (fresh, stale, absent, exit 0) |
| tasks | analyze C3 | split T008, T015, T024 | each is one cohesive file or change; no rubric item on task size |

## Handover

M5 implemented, uncommitted until this handover commit; no PR yet. Done: T030-T033 ticked; deliver_container(.., startup) writes startup.sh to config dir, startup.env to secrets dir (paths /run/micold/config|secrets), removes stale copies; startup threaded through ContainerBringUp, BootPlan (+startup_of, refresh in routing.rs refresh_sandbox_boot, test in service_control.rs), start_attempt; start_script_delivered now true for Container; entrypoint.sh names a non-zero daemon exit (+ test in scripts/tests/basic-entrypoint.test.sh); docs in sandboxed-daemon.md and daemon-configuration.md; sandbox_real_startup.rs (wired in micold-daemon Cargo.toml, not yet compiled/run).
Review A round 1 (snapshot 33749c64:2a88331e): F1 MAJOR fixed (boot plan refresh), F2 MINOR doc added.
Next: (1) scoped gate: 4 client tests were updated for container showing the group (daemons_feature, daemons_section_geometry x2 incl. test name `the_container_form_paints_six_titles...`, features_daemon_sections) - rerun `scripts/autopilot/scoped-gate.sh` (detached, hold.sh) and fix; also compile `cargo test -p micold-daemon --features sandbox-real-runtime --no-run --test sandbox_real_startup`; (2) `mise run image` + `mise run test-sandbox` (podman present) for Verify; (3) Review B (conformance, sonnet) once; (4) visual pass for the container form's Start script group; (5) full gate, PR "feat(1533): deliver the start script to container daemons (#1533)", body Refs #1533, then record PR in ledger. Prior PR #1682 merge SHA 3af058506bbe0e95b0b8e3e31801c8388b23604c.
Follow-up: a content change to the script alone does not change the actor identity; delivered at next start.

## Open escalation

None.

## Follow-ups not done

- Dependencies #1526, #1527, #1528 are still OPEN on GitHub (no delivery commits on main); #1524 is closed. Script delivery (T075) and per-kind failure surfacing need them: plan/milestones must order or stub accordingly.

## M2 notes

Review A (snapshot 43c5ab91:aeebb4cf): CLEAN, 3 MINOR not fixed (exports lost on a Skipped start;
watcher hashes after the run; FileMissing leaves tail empty, reason is the output per S2.3).
Review B (sonnet): CLEAN, 3 MINOR (order test stands in for session.env/ai.cli tiers not yet applied
by the daemon; compile-red only; quickstart Part A not run). Scoped gate green.

## M3 docs decision

PR #1673 carries `docs-not-needed`: the Start script group is hidden for every kind until delivery (`start_script_delivered` is false), so no user-visible change exists. The guide page is T023 in M4.

## M4 notes

#1526 delivery (host `deliver_host`, SSH `ConfigDelivery`) is on main. M4 follows its layout: host `<data_dir>/startup.sh|env`, SSH `~/.micold/remote/secrets/startup.sh|env`; SSH config carries `/__MICOLD_REMOTE_HOME__` which `deliver_config` swaps for the probe's home. Spawners read `startup-result.json` via `micold_core::startup_result` (host: `connect_or_spawn_reading`; SSH: bridge `refused`). Deviations are in the cycle log.
