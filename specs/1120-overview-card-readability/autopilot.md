# Autopilot ledger — #1120 overview-card-readability

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Issue #1120 "Overview cards: readable layout and content" (body verbatim in the issue). labels: enhancement, flow:feature
- **Kind**: feature
- **Effort**: default
- **Issue**: #1120
- **Worktree branch**: claude/project-thread-7041o6
- **Started**: 2026-10-19
- **Phase**: clarify
- **Next step**: clarify round 1: apply the user's answer on FR-012 to spec.md (Clarifications + FR-012), then run round 2 if it changed a requirement; then plan

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|

## Decisions

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|

## Review rounds

| Review | Round | Snapshot | Verdict |
|---|---|---|---|
| Spec | 1 | fdfeae51b28753936f4e21eca650306af1098566:38a9ce97b751560df8c8feaddf1271cbceba0cc2 | CHANGES: 2 MAJOR, 3 MINOR (run by orchestrator); all fixed |
| Spec | 2 | e3d615be650081e55115b2439e1bb15d4948f794:c672c92ff959967fbae970486c6fd8c64a0fa07c | CLEAN (scoped re-review on sonnet, run by orchestrator) |

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|

## Handover

None.

## Open escalation

Clarify round 1 (category 1, product decision the repo does not settle):

- **Q1 (FR-012)**: Should every slash command on the activity line read as one generic sentence
  ("Running *name* on *args*" / "Running *name*"), or should Claude's built-in commands read
  differently or be hidden?
  - **A. One generic sentence for every command (Recommended)**: issue #1120 asks only for "a slash
    command as a sentence"; the hook prompt carries just the typed text, so telling built-ins apart
    needs a hand-kept name list that goes stale; the next prompt replaces a `/clear` line anyway.
  - **B. Generic sentence, but hide built-ins** (`/clear`, `/compact`, `/model`, `/login`, `/usage`,
    …): mirrors feature 032, which treats commands `claude` handles itself as "not a turn"
    (`specs/032-untitled-session-labels/contracts/first-turn-label.md` C3.5a); needs a fixed list.
  - **C. Generic sentence, plus own wording for a few built-ins** ("Clearing the conversation",
    "Compacting the conversation", "Switching model"); also a fixed list.

## Follow-ups not done

None.
