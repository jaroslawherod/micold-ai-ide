# Autopilot ledger — #968 quick-switcher

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Issue #968 "Quick-switcher to jump directly to a project, worktree or session" (see
  spec.md *Input*); labels: enhancement, flow:feature
- **Kind**: feature
- **Effort**: default
- **Issue**: #968
- **Worktree branch**: claude/project-thread-ftdy5c
- **Started**: 2026-10-18
- **Phase**: plan
- **Next step**: plan unit, continue from *Handover* (plan drafted; review round 1 not run)

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|

## Decisions

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|
| D1 | spec | Which chord opens the quick-switcher? | Ctrl/Cmd+Shift+P | agent-resolved | spec.md#assumptions; docs/user-guide/terminal-panes.md#keyboard-shortcuts |
| D2 | clarify | Are worktrees hidden by the tag filter listed? | No; #803 Location order (after the filter) | agent-resolved | specs/803-keyboard-navigation/spec.md#terms |
| D3 | clarify | Unavailable projects: listed despite #803 Project order excluding them? | Listed in the switcher's order, badged, not selectable | agent-resolved | spec.md US3 scenario 3, FR-006, FR-008 |
| D4 | clarify | List other projects' locations and sessions? | No, active project only (option A) | default, pending user confirmation | may flip to B (all projects): then US3-5, FR-003, FR-004 change; spec.md#clarifications |
| D5 | clarify | Rank of a session kept by its location's match; highlight past unavailable projects | After its location, location's rank; first selectable entry | agent-resolved | spec.md US2-3, FR-007 |

## Review rounds

| Review | Round | Snapshot | Verdict |
|---|---|---|---|
| Review spec | 1 | 06e0b7cb93e4b1df3ad203f637ff3bcdfeb41cb2:8ee4531564743d6032729cd056a73f655abfb1a0 | CLEAN: 3 MINOR, fixed, prose only |

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|

## Handover

Plan unit, handed over at the context cap. Done: `speckit-plan` drafted plan.md, research.md
(R1–R10), data-model.md, contracts/keys.md, contracts/ui.md, quickstart.md (uncommitted draft now
committed). Next: confirm the shared dialog component's real type name in
`crates/micold-client/src/ui/material/dialog.rs` and fix "`material::dialog`" in research R6,
contracts/ui.md U6 and plan D6 if it differs; then review round 1 of the plan (tasks/review.md with
rubrics/plan.md, model omitted), act on findings (review-rounds.md), commit, set **Next step** to
`tasks unit`, return DONE. No review run yet; no open findings; no PR.

## Open escalation

None. (Q1 answered provisionally as D4; user confirmation pending.)

## Follow-ups not done

None.
