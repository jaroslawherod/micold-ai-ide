# Autopilot ledger — 034-daemon-mcp-server

Maintained by the `speckit-autopilot` skill. It is the record of what this flow owns and how far it
has got. `resume` finds this file by its **Worktree branch** line and reads it. Keep it true.

- **Input**: "the daemon server should expose mcp server that will be automaticly binded into AI sessions. Should allow to manage the sessions and worktries."
- **Kind**: feature
- **Worktree branch**: feat/daemon-should-expose-mcp-server-for-agent
- **Started**: 2026-09-29
- **Phase**: 2-clarify
- **Next step**: Orchestrator: ask clarify round 1's three user questions (Open escalation), send answers to the Clarify round 1 unit; it records them and says whether round 2 is needed

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #455 | Spec | merged | 8ff6e0561386ba21219411bc16f828f08b177961 |

## Milestones

| ID | Tasks | Deliverable | PR | Status |
|---|---|---|---|---|

## Decisions

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|
| D1 | spec | May a Default session's agent create/rename/delete worktrees through the tool server? | No: refused by policy (FR-015a); lifting it needs a constitution amendment | agent-resolved | .specify/memory/constitution.md#III (review round 1, F1) |
| D2 | clarify 1 | May an agent interrupt its own calling session? | No: refused as invalid input (FR-015); the interrupt would abort the turn awaiting the result | agent-resolved | spec.md#FR-013, crates/micold-core/src/protocol/messages.rs#SessionInterrupt |
| D3 | clarify 1 | Cap or rate-limit agent-created worktrees/sessions? | No: same (absent) limits as the user's action (FR-009) | agent-resolved | spec.md#Assumptions; no limit in crates/micold-daemon/src/server.rs |

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|
| spec | rounds 1–3 | none declined | Round 1: 6 MAJOR + 7 MINOR, all fixed. Round 2 (sonnet): 6 MINOR, all fixed. Round 3 (sonnet): CLEAN, 2 MINOR fixed. |

## Open escalation

Clarify round 1 (category 1, product decision), three questions for one AskUserQuestion:

1. FR-010 scope: own project only (Recommended) / all projects read-only, manage own / all projects.
2. FR-014 destructive-op policy: confirm each in a window, 60 s, refuse when none attached (Recommended) / confirm except targets the calling session created / run without asking / leave out.
3. FR-016 cross-session read/type: toggle exists, off by default (Recommended) / on by default / per-send confirmation / leave out.

## Follow-ups not done

None yet.
