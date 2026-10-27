# Autopilot ledger — #1554 structured-effort-switch

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Spec 1554 — issue #1554 "Structured view: model reasoning effort cannot be changed"; labels: bug, flow:bug (orchestrator added in-progress). Entered as a bug; the bug unit returned NEXT: feature (no effort handling exists in specs 938/943; #943 lists effort out of scope). User asked to confirm the switch.
- **Kind**: feature (entered as a bug, switched)
- **Effort**: default
- **Issue**: #1554
- **Worktree branch**: claude/project-thread-carlvm
- **Started**: 2026-10-27
- **Phase**: design PR
- **Next step**: orchestrator opens the design PR (body in the session scratchpad), waits on CI and merges; then milestone M1 (T001–T010).

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| pending | Design: spec, clarifications, plan, research, contracts, tasks | pushed, PR to be opened by the orchestrator | — |

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1 | T001–T010 | full | Core: levels, support, control lines, `/effort` classification, notes and markers; recordings | — | pending |
| M2 | T011–T017 | full | Daemon: keep the choice, launch with `--effort`, apply to an idle or stopped session | — | pending |
| M3 | T018–T021 | full | Daemon: busy pick deferred to turn end; re-apply on model change | — | pending |
| M4 | T022–T026 | full | Window: effort picker beside the model, the marker; guide | — | pending |
| M5 | T027–T031 | light | `/effort` in the composer; daemon backstop; guide | — | pending |

## Decisions

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|
| D1 | spec | Which levels does the picker offer? | Default plus the session model's supported levels (all five when none reported) | agent-resolved | crates/micold-core/tests/fixtures/stream-json/command-initialize.jsonl (`supportsEffort`, `supportedEffortLevels`) |
| D2 | spec | Choice not supported by the current model? | Keep the stored choice, say so in the view, CLI default applies until a supporting model | agent-resolved | spec.md#Edge Cases |
| D3 | spec | What does `/effort auto` do? | Same as Default; `ultracode` refused (out of scope) | agent-resolved | command-initialize.jsonl argumentHint lists `auto` and `ultracode` |
| D4 | clarify 1 | Can the choice be changed while the model does not support effort? | Yes: picker still opens (Default + five levels), pick is stored and applies on a supporting model | agent-resolved | spec.md#FR-002, #Edge Cases "Session not running", D2 |
| D5 | clarify 1 | Launch with a stored level the model is known not to support? | Launch anyway naming no level (FR-010a); CLI default applies, view says so | agent-resolved | D2, spec.md#FR-011 |
| D6 | plan | Does a running CLI take an effort change, or must the session relaunch? (spec Assumption 1) | Running: `apply_flag_settings {"effortLevel": level|null}`; deferred to turn end when busy, like `set_model`; `--effort` at launch in both kinds | agent-resolved | research.md P-1, P-2 (recorded against the real CLI 2.1.311), R1 |
| D7 | plan | Spec Assumption 3 says the CLI does not report the level in use | Wrong for 2.1.311 (`get_settings` → `applied.effort`); design unchanged, the view shows the choice; follow-up noted | agent-resolved | research.md R10 |
| D8 | tasks | Plan named four milestones; how many? | Five: daemon split along US1 scenario 4 (busy → M3, the 943 model milestone was ~1,470 lines), window split by story (M4 picker, M5 `/effort`) so no UI is half-wired; plan *Delivery* and contracts/wire.md updated (plan fixed) | agent-resolved | tasks.md *Milestones*; milestones.md rules 3, 6 |

## Review rounds

| Review | Round | Snapshot | Verdict |
|---|---|---|---|
| Spec | 1 | aa19795f9a5556b477115b150e8fdcf6847969e1:828bc8e24442d9c9685869c35fe69134934bd233 | CLEAN: 3 MINOR (F1 `/effort auto` = Default, `ultracode` out of scope; F2 two-windows wording; F3 fixture path moved to Assumptions) — fixed |
| Clarify | 1 | — | 2 questions, both agent-resolved (D4, D5); FR-003 amended, FR-010a added, FR-013 extended |
| Clarify | 2 | — | No critical ambiguities |
| Plan | 1 | c3f46ffa915d65a2d4339354633aba57e70384f6:976f758bfa2f4870bff251d80a5ab2f67250317e | CLEAN: 3 MINOR — F1 spec Assumption 3 corrected (`get_settings` reports the level; view still shows the choice); F2 R4 re-apply hooked on `ModelAnswer::Accepted` in state.rs; F3 mid-turn effect and `CLAUDE_EFFORT` marked as assumptions — fixed, prose only. Reviewer ran as a fresh headless `claude -p` (read-only tools): no Agent tool in this unit |
| Analyze | 1 | — | speckit-analyze (fresh headless process): 0 CRITICAL, 2 HIGH (C1 marker text in draw code → `effort_changed_text` tested; D1 terminal-kind pick → spec Edge Case added), 5 MEDIUM, 5 LOW — all fixed except S1 (task size, LOW) |
| Tasks | 1 | b3b75d3032d7a421da8cbd34fcd4c3151089a0b8:1404427c4dbf5c12168db2d65f2d0b86dfeb896f | CHANGES: F1 MAJOR (M4 verified against guide text M5 ships → Part B 7 split 7a/7b), F2–F4 MINOR — all fixed. Fresh headless `claude -p`, read-only tools |
| Tasks | 2 | 46f2f3cf63681dad6afe2f105902d3c5b928d369:1404427c4dbf5c12168db2d65f2d0b86dfeb896f | CLEAN (scoped re-review of the fix diff) |

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|

## Handover

None.

## Open escalation

None.

## Follow-ups not done

- Show Claude Code's resolved default level (`Default (medium)`) by reading `get_settings` → `applied.effort` after each launch and change (research R10); not in the spec.

Note: this unit had no Agent tool; the spec review ran as a fresh headless `claude -p` process (read-only tools) instead of an `autopilot-reviewer` subagent.
