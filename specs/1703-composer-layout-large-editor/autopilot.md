# Autopilot ledger — #1703 composer-layout-large-editor

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: /speckit-autopilot #1703 (Structured view: composer layout — large-editor shortcut, activity icon beside input, settings toggle for model/effort/permission row, unified look); labels: flow:feature
- **Kind**: feature
- **Effort**: default
- **Issue**: #1703
- **Worktree branch**: feat/1703_structured-view-composer-layout-large-editor
- **Started**: 2026-11-05
- **Phase**: milestone M1 PR open
- **Next step**: M1 PR merge, then milestone M2

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #1705 | Design (spec, plan, tasks) | merged | eb1b62d9967f4d7ad7092c0dc75e6d3a1e7af621 |
| #1717 | M1 large editor | open | |

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1 | T001–T013 | full | Large editor: chord/icon opens modal editor, Done/Cancel, shortcut listed | #1717 | PR open |
| M2 | T014–T025 | full | Icon activity indicator + settings-icon options row, persisted, auto-expand | | pending |
| M3 | T026–T031 | full | One selector style (MenuSelect), model filter row | | pending |

Note: M1 (13 tasks) is not split: editor, open button, shortcut entry and docs are one observable behaviour with no earlier deliverable. US2 and US3 are one milestone (M2, 12 tasks): US2 scenario 1 removes the permission selector from the input row and only US3's options row re-homes it (no half-wired UI). T032 (quickstart results) is for the close unit.

## Decisions

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|
| D1 | spec | Shortcut | Ctrl+Shift+M / Cmd+Shift+M | agent-resolved (owner default) | spec.md#decisions |
| D2 | spec | Stop session / refusal note when collapsed | Stay visible, auto-expand on refusal | agent-resolved (owner default) | spec.md#decisions |
| D3 | spec | Model typeahead | In-list first row | agent-resolved (owner default) | spec.md#decisions |
| D4 | spec | Persist expanded state | Yes, per window | agent-resolved (owner default) | spec.md#decisions |
| D5 | spec | Bypass warning marker | On settings icon | agent-resolved (owner default) | spec.md#decisions |
| D6 | spec | Large editor attachments | Text only | agent-resolved (owner default) | spec.md#decisions |

## Review rounds

| Review | Round | Snapshot | Verdict |
|---|---|---|---|
| Spec review | 1 | 23695b79278421607a6d1eeef21cc8220c35f343:3ffb138fbe2f1fbacfb8cbad179ad4a99202e95f | CHANGES: 3 MAJOR (fixed) |
| Spec review | 2 | be3fa230674fce49e8852e215b014e2f60123596:3ffb138fbe2f1fbacfb8cbad179ad4a99202e95f | CLEAN (3 MINOR fixed, prose only) |
| Plan review | 1 | 7e8c799ceb6fabcdecaf0727e82c9a1b7745c6dd:ef7d1da558d16846cd079b27fcc37d93795bc7aa | CHANGES: 1 MAJOR (fixed, prose only: plan scope text), 3 MINOR fixed |
| Tasks review | 1 | 859d3283237831feac5a93d7649f6ab2a1c2483d:f2bd343afdbfe83de2192d7ef0755815054a1519 | CHANGES: 1 MAJOR (Verify commands, fixed), 2 MINOR fixed; plan round-1 fix confirmed |
| Review A (M1) | 1 | a32852da698391233b83a80997d596d7b86e9910:72befd849ecd49e1f2db2f8483ea786350fbe122 | CLEAN (3 MINOR) |
| Review B (M1) | 1 | a32852da698391233b83a80997d596d7b86e9910:72befd849ecd49e1f2db2f8483ea786350fbe122 | CHANGES: 1 MAJOR (no tdd log; declined, see above); Verify 75 passed |

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|
| M1 | A | F1 forgotten session recreates composer on Done | Draft is absent for a never-typed composer too, so skipping would break FR-002 Done; minor, left |
| M1 | A | F2 close_topmost bypasses focus return | Minor; Esc/scrim/Done paths restore focus |
| M1 | B | F1 no tdd/cycle-log.md | The design PR set no tdd plan (no tdd/ dir); tests exist and pass, red-first evidence not retrofittable; left for the close unit's TDD verify |

## Handover

None.

## Open escalation

None.

## Follow-ups not done

None.
