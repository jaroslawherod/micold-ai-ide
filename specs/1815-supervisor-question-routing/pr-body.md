## Milestone M3 of 4 — supervisor-question-routing (specs/1815-supervisor-question-routing)

**Deliverable**: `route next --reask` and `route overdue` exist; the skill sends banners before ending the turn on `OVER`, re-asks after restart and drops answered questions.
**Verify**: `bash scripts/tests/supervisor.test.sh` prints no FAIL (267 cases)
**Tasks**: T017–T019 · **Satisfies**: US3 scenarios 1–3, FR-008 (confirm, ceiling), FR-009, FR-010, FR-011, SC-003

## Agent review
Round 2: A CLEAN, B one MAJOR (T018 wording) fixed, plan-drift exit 0. Round 1: A (4 findings) and B (3) CHANGES, all fixed in round 2 (reask loop in fours, OVER check on Restart, overdue noted once, test assertion, T018 names supervisor-route.sh). Light tier: no tdd/cycle-log.md; red-then-green was run in order (T017/T019 cases failed 16 of 267 before the implementation, 0 after).

## Local gate
Cloud thread: no mise; CI is the gate. `supervisor.test.sh` passes. `autopilot.test.sh` shows one failure, "a 50-file diff lists every file", in code M3 does not touch (git push negotiation noise in this sandbox).

Refs #1815

🤖 Generated with [Claude Code](https://claude.ai/code)

https://claude.ai/code/session_01NCdYvMspBdZhdaBtsHZK63
