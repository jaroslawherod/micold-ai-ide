## Design — daemons dial the orchestrator (specs/1725-daemons-dial-orchestrator)

Spec, clarifications, plan, research, data model, the dial contract, quickstart and tasks for issue #1725 (part of #1722; FR-003, FR-010). No code.

Five milestones, each its own PR: M1 host daemon links (MVP), M2 outage survival and bounded handshake sends, M3 refusal backoff and observable status, M4 reverse forward for container, SSH-host and pod daemons, M5 drop and reconnect.

## Agent review
- Spec review: 2 rounds, 3 MAJOR fixed.
- Plan review: 2 rounds, 4 MAJOR fixed.
- Tasks review: 2 rounds, 5 MAJOR fixed, final CLEAN. speckit-analyze HIGH findings resolved (enabler of `[orchestrator_link]` recorded as an assumption, jitter added to FR-006/FR-007).

## Local gate
Specs-only: `scripts/check-criteria-observables.sh` passed. Docs-only change; no code gate needed.

Refs #1725

🤖 Generated with [Claude Code](https://claude.ai/code)

https://claude.ai/code/session_01NCdYvPQEo5cAr5wHPhDfv5
