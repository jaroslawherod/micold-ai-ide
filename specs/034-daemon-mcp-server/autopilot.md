# Autopilot ledger — 034-daemon-mcp-server

Maintained by the `speckit-autopilot` skill. It is the record of what this flow owns and how far it
has got. `resume` finds this file by its **Worktree branch** line and reads it. Keep it true.

- **Input**: "the daemon server should expose mcp server that will be automaticly binded into AI sessions. Should allow to manage the sessions and worktries."
- **Kind**: feature
- **Worktree branch**: feat/daemon-should-expose-mcp-server-for-agent
- **Started**: 2026-09-29
- **Phase**: 3-design
- **Next step**: Design unit: plan, tasks, milestones, checklists, PR 2

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
| D4 | clarify 1 | FR-010: other projects in scope? | Own project only; other projects' targets read as "not found" | decided by user | AskUserQuestion, clarify round 1 |
| D5 | clarify 1 | FR-014: destructive-op policy? | Confirm each in an app window; "needs confirmation" after 60 s or with no window attached | decided by user | AskUserQuestion, clarify round 1 |
| D6 | clarify 1 | FR-016: cross-session read/type? | Three-value Settings option: Auto (default; read and send, no confirmation) / Confirm each send (read allowed, each send confirmed as in FR-014) / Off (both refused) | decided by user | User's words: "use auto mode means send all no confirm, confirm each send and off by default auto"; confirmed: "default auto" |
| D7 | clarify 2 | Does a change to the FR-016 option reach running sessions? | Yes, from the next request (checked per request) | agent-resolved | spec.md#FR-004, #FR-006 |
| D8 | clarify 2 | Which windows show a confirmation; which answer counts? | All connected windows; the first answer wins | agent-resolved | spec.md#Edge Cases, FR-011 |
| D9 | clarify 2 | Reason categories for decline / timeout / vanished target | refused by policy / needs confirmation / not found; "cancelled" removed | agent-resolved | spec.md#FR-013 |
| D10 | plan | FR-007/SC-009 "cannot connect": what can a loopback TCP transport guarantee? | Another account cannot read any credential and gets only a refusal; FR-007/SC-009 sharpened, intent unchanged | agent-resolved | research.md#R7 |
| D11 | plan | Pre-approve the server's tools in the CLI (`--allowedTools mcp__micold`, `--allow-tool micold`)? | Yes: the spec's guards are FR-014/015/015a/016, enforced by the service; a CLI prompt would double-confirm and break SC-001 | agent-resolved | research.md#R2 |
| D12 | plan | Bind Pi via a TS bridge extension? | No: Pi has no MCP; FR-002's condition does not hold, FR-005 applies | agent-resolved | research.md#R4 |
| D13 | plan | rmcp SDK or hand-rolled server? | Hand-rolled stateless Streamable HTTP on the hook receiver's HTTP code | agent-resolved | research.md#R5 |
| D14 | plan | FR-017 waited for a first "awaiting input" that a fresh session never reports | Deliver on a per-CLI ready-for-input signal (Claude SessionStart hook, Pi component event, Copilot output settled); bound counts from the request; spec reworded | agent-resolved | crates/micold-daemon/src/activity.rs; research.md#R12 |
| D15 | plan | "Gracefully" stop, and stop/interrupt have no sidebar action | Stop ends processes, marks Idle, broadcasts, stays resumable; Assumption covers protocol operations | agent-resolved | crates/micold-daemon/src/server.rs SessionStop arm; research.md#R9 |
| D16 | plan | Does Auto-default cross-session read/type conflict with Principle II? | No: explicit, user-gated, scoped, audited I/O over the target's own input stream is not a leak; recorded in plan.md Complexity Tracking | agent-resolved (plan review round 1) | .specify/memory/constitution.md#II |

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|
| spec | rounds 1–3 | none declined | Round 1: 6 MAJOR + 7 MINOR, all fixed. Round 2 (sonnet): 6 MINOR, all fixed. Round 3 (sonnet): CLEAN, 2 MINOR fixed. |
| plan | round 1 | none declined | 1 BLOCKER (FR-017 readiness), 2 MAJOR (stop path, missing timeout probes), 8 MINOR: all fixed |

## Open escalation

None.

## Follow-ups not done

- Hook receiver's `--settings` token file is written with the default umask, not owner-only (`crates/micold-daemon/src/hooks.rs` `prepare_settings`); outside this feature (research R7).
- Pi tool-server binding via a `-e` bridge extension (research R4).
