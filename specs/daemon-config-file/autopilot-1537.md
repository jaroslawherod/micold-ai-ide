# Autopilot ledger — #1537 daemon-config-file (Issue 15)

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: `/speckit-autopilot #1537` (Retire the old MICOLD_* start-up variables); labels: `flow:feature`, `effort:high` (switched from chore; the work changes user-visible behaviour). Spec, Clarify, Plan skipped; design on main (PR #1509); tasks unit only cuts milestones for the "Issue 15" group (T084, T084b, T085, T085b, T086a, T086b, T086; the originals T084, T085, T086a, T086 were split).
- **Kind**: feature
- **Effort**: high
- **Issue**: #1537
- **Worktree branch**: chore/1537_retire-the-old-micold-start-up-variables
- **Started**: 2026-11-05
- **Phase**: milestone M2 (gate and review)
- **Next step**: M2 implemented (T084b, T085b, T086a, T086); full gate, reviews A and B, `mise run image` + `mise run test-sandbox`, then open the PR. #1530 M2 (PR #1713) merged; M1 merge SHA 04743add7b0041fe5e6b8e8652071b05eeebab66

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #1704 | Design (milestones for Issue 15) | merged | 5e72b83360ee747bc80ad577a468d2119f47bb48 |
| #1709 | M1 | merged | 04743add7b0041fe5e6b8e8652071b05eeebab66 |

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1 | T084, T085, T086b | full | Daemon release build ignores the six `DaemonSettings` variables, one warning each naming the config key; `test-env` seam; guide updated | #1709 | merged |
| M2 | T084b, T085b, T086a, T086 | full | `MICOLD_DATA_DIR`/`MICOLD_TOKEN_PATH` retired; images, SSH launch, entrypoint stop setting them; container-over-SSH regression test; guide; quickstart | | pending |

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

| Review | Round | Snapshot | Verdict |
|---|---|---|---|
| Tasks 1537 | 1 | 72c2d2e1:3ffb138f | CHANGES: F1 capture lib.sh producer, F2 retired_env module uncreated, F3 entrypoint wording; all fixed (task text), F4-F6 MINOR fixed |

## Declined review findings

None.

## Handover

M2 implemented and committed (2 local commits, NOT pushed, no PR): T084b, T085b, T086a, T086 ticked. Review A (code-review high): 8 findings, all fixed or accepted (the basic image pin stays a post-publish bump; `EnvTarget`/`apply_env_hygiene` made pub for the T086a test, declined). Review B (sonnet): CLEAN. Full gate on the final tree: one failure, `micold-daemon::ai_cli_config a_configured_model_is_claude_codes_default_launch_model` ("the stand-in must have run and recorded its launch"), 12639/12640 passed; looks load-related and not touched by this diff, a solo rerun was queued behind the build lock (log: scratchpad `t1.log`, may be gone). Next: rerun that one test (`scripts/build-lock.sh cargo test -p micold-daemon --test ai_cli_config`); if green, rerun `mise run gate` (or record it flaky), run `mise run image` then `mise run test-sandbox` (podman and docker are installed), run `scripts/build-lock.sh cargo test -p micold-core --test main retired_env`, then push, open the PR (title ends (#1537), body ends `Refs #1537`), record it in the ledger. quickstart.md cannot run end to end here (cluster, SSH host, PATs); the close unit records it. Previous PR #1709 merge SHA 04743add7b0041fe5e6b8e8652071b05eeebab66; #1530 M2 (#1713) merged.

## Open escalation

None.

## Follow-ups not done

- Milestones live in tasks.md under "Issue 15 (#1537)". The quickstart run is recorded by the close unit.
