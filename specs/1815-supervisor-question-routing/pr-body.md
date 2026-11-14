## Milestone M2 of 4 — supervisor-question-routing (specs/1815-supervisor-question-routing)

**Deliverable**: `route gate` and `answer.sh --by supervisor`; the supervisor answers a *Never ask* question from repo evidence with `resume --by supervisor`; categories 2, 3, 4 always reach the user.
**Verify**: `bash scripts/tests/supervisor.test.sh` and `bash scripts/tests/autopilot.test.sh` print no FAIL
**Tasks**: T011–T016 · **Satisfies**: US2 scenarios 1–5, FR-002..FR-005a, SC-002

## Agent review
- Review A (code-review, high): round 1 CHANGES (1 MAJOR fixed: resolved-only issues are now sent; 2 MINOR: step ref fixed, mixed-batch By declined with reason); round 2 CLEAN.
- Review B (conformance, sonnet): round 1 CHANGES (1 MAJOR fixed: T013 names supervisor-route.sh; 1 MINOR fixed); round 2 CLEAN. Verify: supervisor.test.sh 249 cases, autopilot.test.sh 354 cases, no FAIL.

## Local gate
Cloud thread: no mise; `scripts/gate.sh` too slow to wait for. Both script suites pass (supervisor 246 cases, autopilot 354); CI is the gate. Clears the M1 follow-up (`answer.sh` parses `--by`).

Refs #1815
🤖 Generated with [Claude Code](https://claude.ai/code)

https://claude.ai/code/session_01NCdYvPSkcfsd48mQKscE4f
