# Autopilot ledger — #942 structured-permission-prompts

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Issue #942 "Structured session: permission prompts via permission-prompt-tool" (epic #945; depends on #939 transcript view and #941 composer/busy state, neither on main yet; #943 depends on this). labels: enhancement, flow:feature
- **Kind**: feature
- **Effort**: default
- **Issue**: #942
- **Worktree branch**: claude/project-thread-tt60gq
- **Started**: 2026-10-21
- **Phase**: design PR
- **Next step**: orchestrator opens the design PR (title/body in the unit's scratchpad), waits on CI and merges; then milestone M1

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| pending | Design: spec, clarify, plan, research, contracts, tasks | to open (orchestrator, GitHub MCP) | — |

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1 | T001–T023 | full | Core: request kinds, answers, rules, modes, control lines and parse against recorded fixtures; capture script | — | pending |
| M2 | T024–T034 | full | Daemon holds and decides prompt requests (switch off), wire Permission*, stand-in prompt | — | pending |
| M3 | T035–T038 | full | Daemon: waiting activity and withdrawal | — | pending |
| M4 | T039–T042 | full | Daemon: mode persisted, created, launched | — | pending |
| M5 | T043–T048 | full | Daemon: mode change while running, following Claude, plan approval sets mode | — | pending |
| M6 | T049–T055 | full | Client render-free prompt state and create-flow mode | — | pending |
| M7 | T056–T063 | full | Window transcript side (ChoiceList, PromptCard, request/plan/question views); needs #939 M5–M6 on main (I-1) | — | pending |
| M8 | T064–T069 | full | Window composer side, switch on, user guide, Part B; needs #941 on main (I-2) | — | pending |

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
| 10 | tasks | M1 has 23 tasks (over rule 3's ~10) | Kept whole: its only split (Phase 2 vs Phase 3) leaves a Foundational-alone milestone (rule 1); 13 of its tasks are tests, fixtures or capture scripts, and the core code is small pure functions | agent | tasks review r1 F3 |
| 9 | tasks | Milestones by layer, not by story (rule 2 deviation) | Every story spans core/daemon/window and the window needs #939/#941; the plan's six layers kept, the two daemon steps each split in two along acceptance scenarios (US1 vs US5/US6; US2 sc1–3 vs sc5+US3) for the ~800-line budget → M1–M8 | agent | tasks.md *Milestones*; plan *Delivery* |

## Review rounds

| Review | Round | Snapshot | Verdict |
|---|---|---|---|
| Spec review | 1 | e384ed34098b974a962950905d077d5e736d9aec:9ad0504870e12886d671ea3f4124162315386f78 | CHANGES: 2 MAJOR, 3 MINOR (all fixed) |
| Spec review | 2 | 004bb5b1f949c41b2cca2730ff420b7af0b6099f:f13d3e811a532c30a69e9475c01265080385e3f8 | CLEAN: 3 MINOR (fixed, prose only) |
| Plan review | 1 | f2b7b32bfd45c20bf89137e325eab23124ae2413:bdd0ffab5e0b7317d1c13b816299b6c68b23a5bf | CLEAN: 1 MINOR (fixed, prose only) |
| Tasks review | 1 | 606e053a7f9c9c7632da9f594a373a2fbb5a824d:dffdf2b12eb552819450d6edf043665419ab018e | CHANGES: 2 MAJOR ([P] on shared files, fixed), 2 MINOR (F3 ledger decision 10; F4 checklist ticked) |
| Tasks review | 2 | 9798b34af9176881119e5e988c331a2bed97b955:18723871b14cc61aac3015421e5aea161167f6bb | CLEAN |

## Declined review findings

| Phase | Review | Finding | Why declined |
|---|---|---|---|
| tasks | speckit-analyze | C1 MEDIUM: sandboxed-session edge case has no task | a sandboxed session runs the daemon and its CLI in the same container (feature 027), so the prompt call is the same loopback tool-server call with the same credential that T029 covers; reachability in the container is already the sandbox suite's (`sandbox_real_mcp.rs`) |
| tasks | speckit-analyze | C4 LOW: no real `SessionInterrupt` test | #941's; plan I-4: an interrupt ends the turn with a `result`, which T037 covers |

| Milestone | Review | Finding | Why declined |
|---|---|---|---|

## Handover

None.

## Open escalation

None.

## Follow-ups not done

None yet.
