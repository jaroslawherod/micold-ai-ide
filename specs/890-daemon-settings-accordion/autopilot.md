# Autopilot ledger — #890 daemon-settings-accordion

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Issue #890 "Group daemon settings sections in an accordion (new material component)"; user at run start: "do UI refinment and unification, split it for proper sections" (read as: unify the daemon form's look and split settings into proper sections, one PR per milestone). labels: enhancement, flow:feature (now also in-progress)
- **Kind**: feature
- **Effort**: default
- **Issue**: #890
- **Worktree branch**: claude/project-thread-rwlog8
- **Started**: 2026-10-18
- **Phase**: tasks
- **Next step**: tasks unit

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|

## Decisions

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|
| D1 | spec | The issue's accordion component and the local container's five sections already shipped in 804 M4 — what is left? | Extend the component (closed-state summary, required marker); split every kind's form (host, SSH, Kubernetes, container's non-container fields) into sections; unify look; one PR per milestone | agent-resolved | specs/804-session-service-in-daemons/spec.md FR-012/FR-013; crates/micold-client/src/ui/material/accordion_section.rs |
| D2 | spec | Same concept, different names per kind | "Connection", "Credentials", "Resources", "Sessions" everywhere; container "Limits" renamed "Resources" | agent-resolved | spec.md FR-003 |
| D3 | clarify | Does a required error shown on leaving an empty field (937 FR-006) force its section open? | Yes; any shown required error counts under FR-004 | agent-resolved | specs/937-required-field-markers/spec.md FR-006; crates/micold-core/src/required.rs FieldRequirement::Missing |
| D4 | clarify | Kubernetes "Image" section renamed "Container"? | No; container section also picks the runtime, keeps 804's name | agent-resolved | specs/804-session-service-in-daemons/spec.md FR-012 |
| D5 | clarify | Exact summary text per section | Fixed in the plan's UI contract | agent-resolved | spec.md FR-014 |

## Review rounds

| Review | Round | Snapshot | Verdict |
|---|---|---|---|
| Spec | 1 | 24cc8d1b:3b6f429d | CHANGES: 1 MAJOR (F1 FR-009 chooser), 3 MINOR — all fixed (F1 SSH select pattern; F2 required errors force sections open; F3 cautions/group notes placement; F4 SSH single section stated) |
| Spec | 2 (scoped, sonnet) | afc2d2d26e2837ad5c3c52dad8b0e4c936921f78:8e6bc4dbb04b88be60c3e57a63faaf2d358cc75b | CLEAN (F1–F4 fixed) |
| Clarify | 1 | spec.md Clarifications 2026-10-18 | 3 Q, all agent-resolved (D3–D5); FR-004 + edge case changed |
| Clarify | 2 | — | no critical ambiguities |
| Plan | 1 | 6a71819950f543013db8bf4fd875133e7b90b673:7d34cc6b80783b6daaee02c86a179f7f5f8d7174 | CLEAN, no findings |

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|

## Handover

None.

## Open escalation

None.

## Follow-ups not done

