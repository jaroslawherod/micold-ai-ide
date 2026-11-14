## Milestone M4 of 4 — supervisor-question-routing (specs/1815-supervisor-question-routing)

**Deliverable**: `route target` exists; a supervisor whose session list reports a parent relays the question to it, and a `not_found` refusal falls back to the user.
**Verify**: `bash scripts/tests/supervisor.test.sh` prints no FAIL (282 cases)
**Tasks**: T020–T022 · **Satisfies**: US4 scenarios 1–4, FR-012, FR-013

`route target --sessions <list_sessions JSON> --me <id>` reads only the daemon's `parent` (a worktree name) of the `--me` row and maps it to a session of that worktree: `parent <id>`, else `user` (no parent, `default`, no matching session). SKILL.md gains an *Upward relay* paragraph and a step 3 pointer.

## Agent review
Round 1: A CHANGES (2 MAJOR, 2 MINOR) and B CHANGES (1 MAJOR, 1 MINOR), all fixed except B F2 (light tier, no cycle log: declined). Fixes: `route target` accepts the daemon's `{"sessions":[...]}` shape, resolves a `default` parent through the worktree lookup, honours `parent_state`, prefers a live session and falls back to the `is_caller` row; T021 names supervisor-route.sh. Round 2 pending.

## Local gate
Cloud thread: no mise; CI is the gate. `supervisor.test.sh` passes.

Refs #1815

🤖 Generated with [Claude Code](https://claude.ai/code)

https://claude.ai/code/session_01NCdYvKE22c2oWVNFebVx1H
