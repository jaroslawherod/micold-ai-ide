# Autopilot ledger — #1269 review-tab

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Spec 1269 — issue #1269 "Review UI: Review as a per-worktree tab that keeps its state"; labels: enhancement, flow:feature
- **Kind**: feature
- **Effort**: default
- **Issue**: #1269
- **Worktree branch**: claude/project-thread-6rh2yo
- **Started**: 2026-10-24
- **Phase**: milestone M1
- **Next step**: M1 review B and full gate, then push branch for the M1 PR (title/body in the scratchpad)

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #1361 | design | merged | 75292027eb695dbe44b01869f8f56c47046429b8 |

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1 | T001–T009 | full | `micold_core::review_tabs` rules T1–T8 proven by core unit tests (FR-016) | — | in progress |
| M2 | T010–T025 | full | switching away keeps the review; **Changes** brings it back as left, re-read if changed; only **Close**/removal drops it | — | todo |
| M3 | T026–T031 | full | **Review changes** on another/sessionless location shows it under its own header; last session closed shows its review | — | todo |
| M4 | T032–T041 | full | the strip's Review tab with draft badge, changed mark and × (replaces **Close**); gates, snapshot, docs, visual pass | — | todo |

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
| D9 | plan | Milestones (superseded by D10) | M1 core rules (no client change), M2 kept views in client state (header only, no strip member), M3 strip Review tab, CountBadge, ×, gates, snapshot, docs, visual pass; none depends on #1125/#1127 | agent | plan.md Delivery order |
| D10 | tasks | Milestones | M1 core rules (T001–T009), M2 kept views on the shown location (T010–T025), M3 another and sessionless locations (T026–T031), M4 the strip tab (T032–T041); Polish T042–T043 to the close unit. M1 is a deliberate departure from milestones.md rule 1 and the rubric's "M1 includes the P1 story" (Foundational alone): its deliverable is FR-016 itself, observable by `test-core`, and folding it into M2 would put M2 far over 800 lines; it also lets the core merge while #1125/#1127 contend for the client. M2 kept at 16 tasks: the hidden-read and re-read-on-show behaviour (US4.3–4.6) cannot ship apart from kept views without presenting old reads as current (US4.5), and the `open` → `tabs` migration touches every focus/attention reader at once. M4 at 10 tasks: no half-wired strip member (rule 6). Plan, quickstart B0, research R14 and contract U-headers renumbered (strip M3 → M4) | agent | tasks.md Milestones; plan.md Delivery order |

## Review rounds

| Review | Round | Snapshot | Verdict |
|---|---|---|---|
| Spec | 1 | 06a25d880f08b234d103ea2291fb03b0635afc26:3a333f7f1ddf23ad4d26f471cfd59f155bed3ff2 | CLEAN: 2 MINOR (FR-003/FR-012 provisional note, checklist ticks), fixed, prose only |
| Plan | 1 | f93f61f0638c026a651b066a3a3c09d0f324d277:2a80f46d473c0f15a956be19111e42659504b291 | CLEAN: 3 MINOR (alternatives for R8/R12–R14, M2 header-only in U16, M2 hand check in quickstart B0), fixed, prose only |
| Tasks | 1 | 501c0f33fe1293434c6134b8cfeeb6673a1ec891:6a7e4285a0140f2655e8c1c3f4db8d6f7ac9dc72 | CHANGES: 1 MAJOR ([P] on T032/T035 same file), 3 MINOR (T012 stopgap readers, M1 rule-1 note, Failure edge case test); all fixed |
| Tasks | 2 | 7db6eeab3ffaf9740641fff36e24e6d87b7390ff:5ee8f094664b699079681e5e7772ee08435c3259 | CLEAN (sonnet re-review of the fix diff) |
| Review A M1 (code-review high) | 1 | 9a59b6ccbd7b16a78b0ea5d619d858cc877bd7e3:86598e23f914c53382f8f6b0cd448245a7953faa | 8 findings: F1 `uncovered` on a hidden tab, F2 `show_if_present` on the shown tab (both fixed, tests added), F5 shared show path (fixed), F8 tooltip alloc (fixed); F3, F4, F6, F7 declined |

## Declined review findings

- Review A M1 F3 (`files_changed` on a shown, covered tab answers `Reread`): contract T5 row 1 says a shown tab answers `Reread` and stays unchanged; whether a covered view reads is the client's watch (R13, U11, M2).
- Review A M1 F4 (`close` answers `show` for a hidden tab): contract T3 defines `show` without regard to `was_shown`; the client switches only when `was_shown` (U4).
- Review A M1 F6 (`Press::None` shadows `Option::None`): the name is the contract's (T8, data-model, M4 tasks); always written qualified. MINOR.
- Review A M1 F7 (`Bytes` xorshift in `tests/terminal_history_format.rs`): pre-existing, outside M1. MINOR.

| Milestone | Review | Finding | Why declined |
|---|---|---|---|

## Handover

None.

## Open escalation

None.

## Follow-ups not done

None.
