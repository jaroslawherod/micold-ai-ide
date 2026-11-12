# Autopilot ledger — #1786 sidebar-user-groups

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Issue #1786 "feat: sidebar: let the user create groups"; labels: enhancement, flow:feature
- **Kind**: feature
- **Effort**: default
- **Supervised**: no
- **Issue**: #1786
- **Worktree branch**: feat/1786_feat-sidebar-let-the-user-create-groups
- **Started**: 2026-11-11
- **Phase**: tasks
- **Next step**: wait for design PR #1848 to merge, then milestone M1

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #1848 | Design | open | … |

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1 | T001–T017, T044 | full | Layout core, store, create a group | #… | pending |
| M2 | T018–T023 | full | Rename and delete a group | #… | pending |
| M3 | T024–T028 | full | Move to group from the menu | #… | pending |
| M4 | T029–T034 | full | Drag between lists and cancel | #… | pending |
| M5 | T035–T042 | full | Reorder, collapse, no state grouping | #… | pending |
| M6 | T043, T045 | full | Showcase poses and visual pass | #… | pending |

## Decisions

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|
| D1 | clarify | … | … | agent-resolved / user | <path#section, or "AskUserQuestion YYYY-MM-DD"> |

## Review rounds

| Review | Round | Scope | Head | Snapshot | Verdict |
| --- | --- | --- | --- | --- | --- |
| Spec | 2 | spec.md | 58e1734ce000a84d1fef7957a5f76047a693cc51 | 15197570fe64dd3fe6ce468637233ff476ad4e90:58e1734ce000a84d1fef7957a5f76047a693cc51 | CLEAN |
| Plan | 2 | design | 076a350f4e2bebc369741dda330ff1265ccea2cb | fe475105b6c85398bc85de75395b5c8b0f586243:8c6d9c8dfc934231e7defc72aab8e90441e8fa1b | CLEAN |
| Tasks | 1 | tasks.md, milestones | fd659fa9 | fd659fa948c3b865c9e5891240b2f31309838abb:d9fddb2b92550718f4a637e8d771d0cbf244709d | CHANGES (1 MAJOR fixed: gate wiring and snapshot moved to M1) |
| Tasks | 2 | fix diff | 1fc59a5f | 70700fb0bd62ac2308bdaa57a5598ba424ce7882:1fc59a5f9b962824a590f71b1c41069072f626c6 | CLEAN (1 MINOR fixed, prose only) |
| Plan | 1 | design | 33647c442736a049f12d6cc8752134e390fc02f0 | 9c4e947e275c19ed0709a25e96d98d7e43160c6c:c0341c2bd14d57f5c0d8119083fd10ec6f2fb7ac | CHANGES (3 MAJOR fixed) |
| Tasks | 2 | fix diff | 7ad2a0361951595eea2d898d6746b37a1ddb0036 | 70700fb0bd62ac2308bdaa57a5598ba424ce7882:1fc59a5f9b962824a590f71b1c41069072f626c6 | CLEAN |
| Plan | 3 | design | 5d3885de780da42ac43772e0841f49a11dce4d3c | e984a4491e90ea566d4e00fe11bf0e87b4597b0f:5d3885de780da42ac43772e0841f49a11dce4d3c | CHANGES (1 MAJOR fixed: rename_dialog.rs line restored in plan.md) |
| Plan | 4 | design | 5d3885de780da42ac43772e0841f49a11dce4d3c | e984a4491e90ea566d4e00fe11bf0e87b4597b0f:5d3885de780da42ac43772e0841f49a11dce4d3c | CLEAN (1 MINOR left: text.rs not listed in plan, pre-existing) |

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|

## Handover

None. <or, when a unit handed over: what is done, the next step, open findings with their review
snapshots. The next unit sets it back to None.>

## Open escalation

None. <or, one question:>

```
- **Category**: <number and name from rules/escalation.md>
- **Question**: <one sentence>
- **Options**:
  1. <label> (Recommended) — <description>
  2. <label> — <description>
- **Recommended**: <label> — <reason>
- **Evidence**: <paths, commands, results>
- **Banner**: sent <date>
```

<A batch: `### Q1`, `### Q2`... each holding Question, Options, Recommended and Evidence; Category and
Banner stay above them. `scripts/autopilot/run-status.sh` reads these fields.>

## Follow-ups not done

<Defects found outside this flow's work, scope deliberately cut, etc. Copied into the handoff.>
