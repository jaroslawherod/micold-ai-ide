# Autopilot ledger — 034-daemon-mcp-server

Maintained by the `speckit-autopilot` skill. It is the record of what this flow owns and how far it
has got. `resume` finds this file by its **Worktree branch** line and reads it. Keep it true.

- **Input**: "the daemon server should expose mcp server that will be automaticly binded into AI sessions. Should allow to manage the sessions and worktries."
- **Kind**: feature
- **Worktree branch**: feat/daemon-should-expose-mcp-server-for-agent
- **Started**: 2026-09-29
- **Phase**: 4-milestone M1
- **Next step**: Unit: M1 gate, cross-checks, reviews A and B, open PR. All M1 tasks done except T015/T021 (U127 blocked, see Follow-ups) and the T078–T082 acceptance ticks after the gate

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #455 | Spec | merged | 8ff6e0561386ba21219411bc16f828f08b177961 |
| #465 | Design (PR 2) | merged | a5fcc795294b6dcd2972ab5e3d31a814b41d8fb7 |

## Milestones

| ID | Tasks | Deliverable | PR | Status |
|---|---|---|---|---|
| M1 | T001–T022, T078–T082, T100 | Bound sessions (Claude, Copilot) with the read tools | — | in progress |
| M2 | T023–T028, T083, T101 | Settings toggle for the tool server (protocol 17) | — | planned |
| M3 | T029–T042, T072–T073, T084–T088, T102 | create_worktree / create_session with first prompt, audit line | — | planned |
| M4 | T043–T048, T089 | start_session / rename_worktree; policy rows for the destructive tools | — | planned |
| M5 | T049–T059, T090–T094, T103 | Confirmations in app windows; destructive tools (protocol 18) | — | planned |
| M6 | T060–T071, T095–T099, T104 | Cross-session read/send under the FR-016 setting (protocol 19) | — | planned |
| M7 | T074–T077 | Read latency, sandbox placement, final real-CLI pass, user guide | — | planned |

Size note: M1 (28), M3 (22), M5 (17) and M6 (18) exceed the ~15 guideline because each carries its
acceptance-gate and quickstart tasks and no split along an acceptance scenario leaves an observable
deliverable (tasks.md Notes).

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
| D16 | plan | Does Auto-default cross-session read/type conflict with Principle II? | Yes, a conflict (speckit-analyze C1, tie-break review). Keep `Auto`; record a justified violation in plan.md Complexity Tracking; Principle II check marked "justified violation"; keep FR-010/016/018 guards and `mcp_cross_session.rs`; no amendment | decided by user | plan.md#Complexity Tracking; .specify/memory/constitution.md#II, #Governance |
| D17 | design | Resolve the Principle II conflict how: justified violation, MINOR amendment, or default `Off`? | Option 1, justified violation; no constitution amendment | decided by user | Coordinator relay of the user's decision on the escalation |
| D18 | design | Milestones M1–M7 as cut? | Ship as cut, M1 through M7 in order, no changes | decided by user | Coordinator relay of the user's decision |

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|
| spec | rounds 1–3 | none declined | Round 1: 6 MAJOR + 7 MINOR, all fixed. Round 2 (sonnet): 6 MINOR, all fixed. Round 3 (sonnet): CLEAN, 2 MINOR fixed. |
| plan | analyze | C1 CRITICAL (Principle II) | Not declined: resolved by the user as a justified violation (D16, D17); plan.md Complexity Tracking records it |
| tasks | round 1 | none declined | 2 MAJOR (M3 Default refusal untested through `POST /mcp`; M1 gate pointed at M7's T076), 7 MINOR, all fixed. |
| tasks | round 2 (sonnet) | none declined | CLEAN; 104/104 tasks in exactly one milestone. 2 MINOR fixed: protocol numbers shifted to main's `PROTOCOL_VERSION = 16` (M2 17, M5 18, M6 19); `mcp_audit_log` added to M5/M6 Verify. |
| checklists | sign-off | none declined | `checklists/requirements.md`: all 16 items ticked and confirmed satisfied by the round 2 reviewer. |
| plan | rounds 1–2 | none declined | Round 1: 1 BLOCKER (FR-017 readiness), 2 MAJOR (stop path, missing timeout probes), 8 MINOR, all fixed. Round 2 (sonnet): no BLOCKER/MAJOR, 3 MINOR fixed. |

## Open escalation

None.

## Follow-ups not done

- Hook receiver's `--settings` token file is written with the default umask, not owner-only (`crates/micold-daemon/src/hooks.rs` `prepare_settings`); outside this feature (research R7).
- Pi tool-server binding via a `-e` bridge extension (research R4).
- U127 (`list_worktrees` reports `status: locked`): the daemon cannot produce it. `micold_core::worktree::WorktreeStatus` has no locked state and `wire_worktree_status` never yields `WorktreeStatus::Locked`, so T015/T021 stay unticked on that one behavior. Needs worktree discovery to parse porcelain `locked` (core + sidebar), outside feature 034's files.
- T002 deviation: the MCP test fixture is included by `#[path = "support/mcp.rs"]` instead of re-exported from `tests/support/mod.rs`, so the framer helpers do not become dead code (a clippy `-D warnings` failure) in the MCP test binaries.
