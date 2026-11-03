# Autopilot ledger — #1664 <slug> [BUG-<issue>]

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Spec 1664 (issue #1664: Structured view: raw events are shown inline in the transcript; present them as collapsed summaries); labels: none
- **Kind**: feature
- **Effort**: default
- **Issue**: #1664
- **Worktree branch**: feat/1664_structured-view-raw-events-are-shown-inline-in-the
- **Started**: 2026-11-03
- **Phase**: spec (done)
- **Next step**: clarify unit: answer FR-013 (error footers visible when toggle off?)

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #… | Design | open / merged | … |

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1 | T001–T018 | full | … | #… | pending / in-progress / in-review / ci / merged |

## Decisions

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|
| D1 | clarify | … | … | agent-resolved / user | <path#section, or "AskUserQuestion YYYY-MM-DD"> |

## Review rounds

| Review | Round | Snapshot | Verdict |
|---|---|---|---|
| Spec | 1 | 2479fb2a15da66eb0accabf88bb3df2733cb7cd1:606c5bf85706c584f474b0ce2ff8fd50666a5ca5 | CHANGES: 2 MAJOR (fixed: facts match entry fields; error visibility raised as FR-013 marker); MINORs fixed, prose only |
| Spec | 2 | a7d89dffe96efd9eed2b34554c01e3d784f407d0:2305bb236cec7443b911cbc0c9b526ac9dd8870a | CLEAN (2 MINOR: FR order fixed; error-footer scenario deferred to FR-013 resolution in clarify) |

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|

## Handover

None. <or, when a unit handed over: what is done, the next step, open findings with their review
snapshots. The next unit sets it back to None.>

## Open escalation

None. FR-013 answered by the user (error footers/orphans stay visible); applied to spec.md.

## Follow-ups not done

<Defects found outside this flow's work, scope deliberately cut, etc. Copied into the handoff.>
