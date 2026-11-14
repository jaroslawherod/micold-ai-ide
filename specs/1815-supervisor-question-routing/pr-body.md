## Milestone M4 of 4 — supervisor-question-routing (specs/1815-supervisor-question-routing)

**Deliverable**: `route target` exists; a supervisor whose session list reports a parent relays the question to it, and a `not_found` refusal falls back to the user.
**Verify**: `bash scripts/tests/supervisor.test.sh` prints no FAIL (282 cases)
**Tasks**: T020–T022 · **Satisfies**: US4 scenarios 1–4, FR-012, FR-013

`route target --sessions <list_sessions JSON> --me <id>` reads only the daemon's `parent` (a worktree name) of the `--me` row and maps it to a session of that worktree: `parent <id>`, else `user` (no parent, `default`, no matching session). SKILL.md gains an *Upward relay* paragraph and a step 3 pointer.

## Agent review
Pending: Review A and Review B are for the orchestrator to dispatch (light tier, no tdd/cycle-log.md; the T020/T022 cases failed 13 of 282 before the implementation, 0 after).

## Local gate
Cloud thread: no mise; CI is the gate. `supervisor.test.sh` passes.

Refs #1815

🤖 Generated with [Claude Code](https://claude.ai/code)

https://claude.ai/code/session_01NCdYvKE22c2oWVNFebVx1H
