# Autopilot ledger — #1537 daemon-config-file (Issue 15)

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: `/speckit-autopilot #1537` (Retire the old MICOLD_* start-up variables); labels: `flow:feature`, `effort:high` (switched from chore; the work changes user-visible behaviour). Spec, Clarify, Plan skipped; design on main (PR #1509); tasks unit only cuts milestones for the "Issue 15" group (T084, T084b, T085, T085b, T086a, T086b, T086; the originals T084, T085, T086a, T086 were split).
- **Kind**: feature
- **Effort**: high
- **Issue**: #1537
- **Worktree branch**: chore/1537_retire-the-old-micold-start-up-variables
- **Started**: 2026-11-05
- **Phase**: done
- **Next step**: handoff

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #1704 | Design (milestones for Issue 15) | merged | 5e72b83360ee747bc80ad577a468d2119f47bb48 |
| #1709 | M1 | merged | 04743add7b0041fe5e6b8e8652071b05eeebab66 |
| #1771 | M2 | merged | 5fc3a75a7c4192b0fdf09c9dc1e867cc29562e0b |
| #1840 | Close the spec | open | |

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1 | T084, T085, T086b | full | Daemon release build ignores the six `DaemonSettings` variables, one warning each naming the config key; `test-env` seam; guide updated | #1709 | merged |
| M2 | T084b, T085b, T086a, T086 | full | `MICOLD_DATA_DIR`/`MICOLD_TOKEN_PATH` retired; images, SSH launch, entrypoint stop setting them; container-over-SSH regression test; guide; quickstart | #1771 | merged |

## Decisions

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|
| D1 | tasks | Skip spec/clarify/plan? | Yes, design merged on main | user | prompt |
| D2 | tasks | Split Issue 15? | Yes, two milestones: daemon-side settings + seam (M1), core reads + images/SSH launch + regression test (M2); T084/T085/T086 split into a/b tasks | agent-resolved | milestones.md rule 3; ~57 files, images and SSH launch still set the variables |
| D3 | tasks | User-guide task | T086b in M1 (visible behaviour), T086 in M2 | agent-resolved | milestones.md rule 4 |
| D4 | tasks | `MICOLD_SSH_DIR` | No read site exists in the repo; nothing to remove | agent-resolved | grep |
| D5 | tasks | Tier | full for both: process env, token check, sandbox boundary | agent-resolved | milestone-format.md |

## Review rounds

M1: review A (code-review high) snapshot cd610b30:2c612ac0 round 1: F1 MAJOR fixed (capture scene waits on an info-level line; the app overwrites the host daemon.toml), F2 declined (DATA_DIR/TOKEN_PATH stay live until M2 by design), F3-F5 MINOR left. Review B (sonnet): CLEAN, 3 MINOR (F2 fixed). The gate caught the basic image content hash; pin updated.

| Review | Round | Scope | Head | Snapshot | Verdict |
| --- | --- | --- | --- | --- | --- |
| Close 1537 | 1 |  |  | 74c0062f37bc014c7f1a95208eff01db01e768a6:c84209f6efdb6fdb9d18d144f35e3ff30dcb20b0 | CHANGES: F1 MAJOR mutant TEST_ENV=true not run (now run, fails the test, restored), F2-F3 MINOR T155 marked partial; prose/record fixes, no new round |
| Tasks 1537 | 1 |  |  | 72c2d2e1:3ffb138f | CHANGES: F1 capture lib.sh producer, F2 retired_env module uncreated, F3 entrypoint wording; all fixed (task text), F4-F6 MINOR fixed |
| Close 1537 diff | 2 | close | 7b385025813cc24f3224a7b02ffce12e299fcb8e | 0525cce6c899d315c7a1f41bdcedaa3bae9d61a4:e751a68607f108be83495634c31add8aff4e9fc2 | CLEAN 3 MINOR (F1 fixed prose only) |
| Close 1537 PR | 1 | 1840 | 9c2c00a04aa1f211de6dee15b2f8b40e8e655b09 | 485792a1bfe0563b71ecd8ebc82acc19200888a3:b23e24833f8c3136a29854d08a0901672e9fc920 | CLEAN F1 MAJOR fixed (comment and task text only, prose only), F2-F3 MINOR left |
| Close 1537 PR | 2 | 1840 | 9e3ff5df6e81ff113c60274d8515b3657d890dfc | df829da81479f81f1af152457de127f2db9f4d73:9e3ff5df6e81ff113c60274d8515b3657d890dfc | CLEAN rebase onto new main, T148-T155 renumbered T152-T159 after the Issue 10 collision; sonnet re-review no findings; full gate green |

M2: review A (code-review high) 8 findings fixed or accepted; review B (sonnet) CLEAN. Local gate on the rebased tree: 12650/12651, the one failure `micold-daemon::ai_cli_config a_configured_model_is_claude_codes_default_launch_model` fails here every time and passes with `HOME` pointed at an empty directory, so it depends on this machine's real `~/.claude.json` (many live Claude sessions), not on the diff. `mise run image` built; `mise run test-sandbox` cannot run here: the runtime rejects `--tmpfs ... uid=1000` (19 targets fail at container create), and `sandbox_real_scoped_token.rs` on main still uses the retired `GitCredentialGrant` (does not compile with the feature on); CI's sandbox job decides. quickstart.md not run (cluster, SSH host, PATs): the close unit notes it.

Close review (sonnet, snapshot 0525cce6:e751a686 after rebase onto main): CLEAN, 3 MINOR (F1 issue number added, fixed prose only; F2 vacuous-branch test is already recorded as I15-4 closed in part; F3 env-var test flake risk left).

## Declined review findings

None.

## Handover

None.

## Open escalation

None.

## Follow-ups not done

- `daemon_config_render::from_entry_delivering` defaults `client_token_file` to `SANDBOX_TOKEN` (`/run/micold/secrets/client-token`) while `CONTAINER_TOKEN_PATH` is `/run/micold/token`: found by close remediation T155, not fixed (adds behaviour); filed as #1838.
- Milestones live in tasks.md under "Issue 15 (#1537)". The quickstart run is recorded by the close unit.
