## Milestone M3 of 4 — autopilot supervisor verify handoff and report (specs/1816-supervisor-verify-report)

**Deliverable**: `scripts/autopilot/close-out-report.sh` writes one markdown report over many runs: Completed, Not done, Escalated.
**Verify**: `bash scripts/tests/close-out.test.sh` (report cases)
**Tasks**: T013–T014, T020 · **Satisfies**: US3.1–3, FR-008, FR-009, FR-010, FR-016, SC-002

Runs are classified through `verify-run.sh` (only VERIFIED is Completed; then Escalated by the supervisor ledger's block; else Not done). `--parent <ref> --rows <file>` keeps only runs `runs-of-parent.sh` calls own and names the parent. The Safe to remove section is a stub (`- none`) until M4.

## Agent review
- Review A (self, `high`, no Agent tool in the cloud harness): no findings.
- Review B (conformance, sonnet): dispatched by the orchestrator.

## Local gate
Shell-only diff; `scripts/gate.sh` did not finish in the container. Ran `close-out.test.sh` (78 cases), `autopilot.test.sh`, `supervisor.test.sh`, `run-status.test.sh` and others green. Local gate: scoped only, full gate left to CI.

Refs #1816
🤖 Generated with [Claude Code](https://claude.com/claude-code)

https://claude.ai/code/session_01NCdYvNiLhPf14V7oCABB28
