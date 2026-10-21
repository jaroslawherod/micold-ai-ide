# Autopilot ledger — #942 structured-permission-prompts

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Issue #942 "Structured session: permission prompts via permission-prompt-tool" (epic #945; depends on #939 transcript view and #941 composer/busy state, neither on main yet; #943 depends on this). labels: enhancement, flow:feature
- **Kind**: feature
- **Effort**: default
- **Issue**: #942
- **Worktree branch**: claude/project-thread-tt60gq
- **Started**: 2026-10-21
- **Phase**: tasks
- **Next step**: tasks unit (plan review round 1 CLEAN; design PR ships spec + clarify + plan)

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1 | tasks unit | full | Core: PermissionMode, permission::{request,decision,rules}, control, Status/ControlResponse events, recorded fixtures | — | pending |
| M2 | tasks unit | full | Daemon: prompt tool, PromptRegistry, hold/decide/withdraw, activity, wire Permission* (switch off) | — | pending |
| M3 | tasks unit | full | Daemon: mode persisted/launched, SetPermissionMode via control request, follow reported mode, plan approval sets mode | — | pending |
| M4 | tasks unit | full | Client render-free prompt state and create-flow mode | — | pending |
| M5 | tasks unit | full | Window transcript side (ChoiceList, PromptCard, request/plan/question views); needs #939 UI on main (I-1) | — | pending |
| M6 | tasks unit | full | Window composer side, switch on, user guide, Part B; needs #941 on main (I-2) | — | pending |

## Decisions

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|
| 1 | clarify r1 | Can a running session's mode change? | Any time, via stream-json control request, effective next tool request, no restart; Bypass keeps confirmation | decided by user | spec Clarifications; FR-017 |
| 2 | clarify r1 | Always-allow scope | This session only (`session` destination); no Claude settings file written | decided by user | spec Clarifications; FR-010 |
| 3 | clarify r2 | Mode change vs a pending request; while not running | Pending unaffected; saved for next launch | agent | spec FR-017, FR-003, FR-015 |
| 4 | clarify r2 | Always-allow rule with no CLI suggestions | Deferred to plan research (fixtures, FR-020) | agent | deferral |
| 5 | plan | Always-allow rule with no CLI suggestions (item 4) | Daemon holds session-scoped exact-input rules (Bash command, file path, WebFetch host, else tool name) and answers matches itself; dropped at process end | agent | research R9; probe P-11 (MCP path sends no suggestions, ignores updatedPermissions) |
| 6 | plan | Transport: MCP prompt tool vs stdio control channel | MCP prompt tool (issue acceptance); control channel only for set_permission_mode | agent | research R1; P-1, P-7 |
| 7 | plan | CLI tool-call timeout vs FR-003 | Structured binding gets per-server timeout 2147483647 ms | agent | research R3; P-5, P-12, P-13 |
| 8 | plan | Runtime switch to Bypass | Every structured launch adds --allow-dangerously-skip-permissions | agent | research R6; P-9, P-10 |

## Review rounds

| Review | Round | Snapshot | Verdict |
|---|---|---|---|
| Spec review | 1 | e384ed34098b974a962950905d077d5e736d9aec:9ad0504870e12886d671ea3f4124162315386f78 | CHANGES: 2 MAJOR, 3 MINOR (all fixed) |
| Spec review | 2 | 004bb5b1f949c41b2cca2730ff420b7af0b6099f:f13d3e811a532c30a69e9475c01265080385e3f8 | CLEAN: 3 MINOR (fixed, prose only) |
| Plan review | 1 | f2b7b32bfd45c20bf89137e325eab23124ae2413:bdd0ffab5e0b7317d1c13b816299b6c68b23a5bf | CLEAN: 1 MINOR (fixed, prose only) |

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|

## Handover

None.

## Open escalation

None.

## Follow-ups not done

None yet.
