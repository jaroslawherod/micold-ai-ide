# Autopilot ledger — #1269 review-tab

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Spec 1269 — issue #1269 "Review UI: Review as a per-worktree tab that keeps its state"; labels: enhancement, flow:feature
- **Kind**: feature
- **Effort**: default
- **Issue**: #1269
- **Worktree branch**: claude/project-thread-6rh2yo
- **Started**: 2026-10-24
- **Phase**: tasks
- **Next step**: tasks unit: write tasks.md from plan.md (milestones M1 core `review_tabs`, M2 kept views in client state, M3 strip tab + docs + snapshot + visual pass)

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
| D5 | clarify | What does the Changes button do while the Review tab is shown? | Nothing changes: the tab stays shown; the button only opens or shows it (replaces 1126 FR-007's close-on-second-press); tooltip says the review is shown | agent-resolved | issue #1269 body "open or focus the tab"; review-redesign.md fit table row #1126, §0 |
| D6 | clarify | Does **Review changes** on another location make it the shown location; where is a sessionless worktree's review drawn? | Yes, it becomes the shown location (last-shown else main session current) with its Review tab shown; a sessionless location shows its own header and a strip of only Review + **New session** | agent-resolved | review-redesign.md §0 "focuses the existing tab or creates it"; spec.md FR-001 |
| D7 | plan | Where do the tab rules live? | Generic `micold_core::review_tabs::ReviewTabs<V>` (map by `(project, location)`, shown key, freshness, last-shown session, faces); client instantiates with `OpenView` | agent | research.md R1; orchestrator scope |
| D8 | plan | How are a hidden view's in-flight reads handled? | All view messages act on the shown view; hidden results are dropped by the existing seq checks; the view is `Unwatched` and settles its loads then re-reads once when shown | agent | research.md R6; spec edge case *A read in flight* |
| D9 | plan | Milestones | M1 core rules (no client change), M2 kept views in client state (header only, no strip member), M3 strip Review tab, CountBadge, ×, gates, snapshot, docs, visual pass; none depends on #1125/#1127 | agent | plan.md Delivery order |

## Review rounds

| Review | Round | Snapshot | Verdict |
|---|---|---|---|
| Spec | 1 | 06a25d880f08b234d103ea2291fb03b0635afc26:3a333f7f1ddf23ad4d26f471cfd59f155bed3ff2 | CLEAN: 2 MINOR (FR-003/FR-012 provisional note, checklist ticks), fixed, prose only |
| Plan | 1 | f93f61f0638c026a651b066a3a3c09d0f324d277:2a80f46d473c0f15a956be19111e42659504b291 | CLEAN: 3 MINOR (alternatives for R8/R12–R14, M2 header-only in U16, M2 hand check in quickstart B0), fixed, prose only |

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|

## Handover

None.

## Open escalation

None.

## Follow-ups not done

None.
