# Autopilot ledger — #1269 review-tab

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Spec 1269 — issue #1269 "Review UI: Review as a per-worktree tab that keeps its state"; labels: enhancement, flow:feature
- **Kind**: feature
- **Effort**: default
- **Issue**: #1269
- **Worktree branch**: claude/project-thread-6rh2yo
- **Started**: 2026-10-24
- **Phase**: spec
- **Next step**: clarify unit: resolve the two `[NEEDS CLARIFICATION]` markers in spec.md User Story 2 (scenarios 4 and 5)

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|

## Decisions

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|
| D1 | spec | What brings a hidden Review tab back as the shown tab? | Only the Review tab itself, the Changes button or **Review changes**; any session selection (sidebar, session tab, chord, quick switcher) shows that session and keeps the Review tab | agent-resolved | spec.md US1 sc. 4, US5 sc. 2, FR-005; review-redesign.md §0 |
| D2 | spec | Are kept views persisted across restart? | No: window memory only, like the remembered PR | agent-resolved | review-redesign.md open question 0; feature 040 FR-032; spec.md Assumptions |
| D3 | spec | How does a hidden tab learn of changes without watching every location? | The shown location is already watched (1126 R6 header counts); other locations' kept views re-read when next shown | agent-resolved | shell/subscriptions.rs header_watch_entry; spec.md FR-010, FR-011 |
| D4 | spec | Where in the strip does the Review tab sit? | Last scrolling member, after session and shell tabs, before **New session**; scrolled into view when shown | agent-resolved | mockup TabStrip.dc.html; 1126 FR-010 |

## Review rounds

| Review | Round | Snapshot | Verdict |
|---|---|---|---|
| Spec | 1 | 06a25d880f08b234d103ea2291fb03b0635afc26:3a333f7f1ddf23ad4d26f471cfd59f155bed3ff2 | pending |

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|

## Handover

None.

## Open escalation

None.

## Follow-ups not done

None.
