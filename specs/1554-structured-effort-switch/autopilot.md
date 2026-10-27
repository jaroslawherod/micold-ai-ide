# Autopilot ledger — #1554 structured-effort-switch

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Spec 1554 — issue #1554 "Structured view: model reasoning effort cannot be changed"; labels: bug, flow:bug (orchestrator added in-progress). Entered as a bug; the bug unit returned NEXT: feature (no effort handling exists in specs 938/943; #943 lists effort out of scope). User asked to confirm the switch.
- **Kind**: feature (entered as a bug, switched)
- **Effort**: default
- **Issue**: #1554
- **Worktree branch**: claude/project-thread-carlvm
- **Started**: 2026-10-27
- **Phase**: plan
- **Next step**: plan unit (clarify CLEAN after 2 rounds; the plan settles from a recording how a running CLI takes an effort change and the launch flag, per Assumptions).

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|

## Decisions

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|
| D1 | spec | Which levels does the picker offer? | Default plus the session model's supported levels (all five when none reported) | agent-resolved | crates/micold-core/tests/fixtures/stream-json/command-initialize.jsonl (`supportsEffort`, `supportedEffortLevels`) |
| D2 | spec | Choice not supported by the current model? | Keep the stored choice, say so in the view, CLI default applies until a supporting model | agent-resolved | spec.md#Edge Cases |
| D3 | spec | What does `/effort auto` do? | Same as Default; `ultracode` refused (out of scope) | agent-resolved | command-initialize.jsonl argumentHint lists `auto` and `ultracode` |
| D4 | clarify 1 | Can the choice be changed while the model does not support effort? | Yes: picker still opens (Default + five levels), pick is stored and applies on a supporting model | agent-resolved | spec.md#FR-002, #Edge Cases "Session not running", D2 |
| D5 | clarify 1 | Launch with a stored level the model is known not to support? | Launch anyway naming no level (FR-010a); CLI default applies, view says so | agent-resolved | D2, spec.md#FR-011 |

## Review rounds

| Review | Round | Snapshot | Verdict |
|---|---|---|---|
| Spec | 1 | aa19795f9a5556b477115b150e8fdcf6847969e1:828bc8e24442d9c9685869c35fe69134934bd233 | CLEAN: 3 MINOR (F1 `/effort auto` = Default, `ultracode` out of scope; F2 two-windows wording; F3 fixture path moved to Assumptions) — fixed |
| Clarify | 1 | — | 2 questions, both agent-resolved (D4, D5); FR-003 amended, FR-010a added, FR-013 extended |
| Clarify | 2 | — | No critical ambiguities |

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|

## Handover

None.

## Open escalation

None.

## Follow-ups not done

None yet.

Note: this unit had no Agent tool; the spec review ran as a fresh headless `claude -p` process (read-only tools) instead of an `autopilot-reviewer` subagent.
