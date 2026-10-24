# Autopilot ledger — #1127 title-bar-breadcrumb

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Issue #1127 "Layout: single 44px title bar with breadcrumb and jump-to field" (body verbatim in spec.md *Input*); the user asked to start the app redesign issues in dependency order. labels: enhancement, flow:feature
- **Kind**: feature
- **Effort**: default
- **Issue**: #1127
- **Worktree branch**: claude/project-thread-iwrht0
- **Started**: 2026-10-24
- **Phase**: clarify
- **Next step**: clarify unit: run speckit-clarify on specs/1127-title-bar-breadcrumb/spec.md (no open markers; spec review CLEAN)

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|

## Decisions

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|
| D1 | spec | Plan usage in the title bar (issue body) or the status bar (#1124)? | Status bar only; the title bar draws none | agent-resolved | specs/1124-bottom-status-bar/spec.md#Clarifications, FR-011; mockup canvas.json note 4 |
| D2 | spec | Can the bar land before #1125's Overview row? | Yes: Stories 1–2 first; Story 3 (Overview button leaves) waits for #1125 M3 to merge; button stays until then | agent-resolved | issue #1127 "Blocked by"; specs/1125-sidebar-state-groups/autopilot.md#Milestones (M3 todo) |
| D3 | spec | What does the breadcrumb's second segment show? | Short branch (else display name); `Overview` / `Settings` when those are shown | agent-resolved | mockup Main.dc.html / Overview.dc.html title bar (`micold-ai-ide / fix/1117`, `/ Overview`) |

## Review rounds

| Review | Round | Snapshot | Verdict |
|---|---|---|---|
| Spec | 1 | 8eb3f915c06f64566ad2e62c6f69da7d28f756f5:08ff39f5c466168d2ad5a9ff688e3b192d4f9461 | CLEAN (3 MINOR, fixed, prose only) |

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|

## Handover

None.

## Open escalation

None.

## Follow-ups not done

None yet.
