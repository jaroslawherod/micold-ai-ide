# Autopilot ledger — #1270 review-bar-drawer

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Spec 1270 — issue #1270 "Review UI: review bar and comments drawer"; labels: enhancement, flow:feature
- **Kind**: feature
- **Effort**: default
- **Issue**: #1270
- **Worktree branch**: claude/project-thread-ttad83
- **Started**: 2026-10-24
- **Phase**: tasks
- **Next step**: tasks review, specs-only gate, design PR

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1 | T001–T007, T017–T018, T023–T026, T044–T045 | full | core rules review::{bar,drawer}, reply::card_preview, anatomy tokens, unit-tested (Layer A) | — | pending |
| M2 | T008–T010, T027–T031, T046–T052 | full | ReviewBar, SideSheet, LineClamped::when_cut, ReviewPanelRow reply mode; cards show 4 reply lines (Layer A) | — | pending |
| M3 | T011–T014, T019–T021, T032–T040 | full | reducer, shell, Esc wired to per-entry view; old panel still drawn (Layer B, **blocked by #1269**) | — | blocked |
| M4 | T015–T016, T022, T041–T043, T053–T057 | full | Review tab bar + drawer layout, geometry gate, snapshot, guide, visual pass (Layer B, **blocked by #1269**) | — | blocked |

## Decisions

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|
| D1 | spec | How does the spec depend on #1269 (not merged)? | Only through its contract: each entry has a Review tab whose view state survives switching; drawer state belongs to that view (FR-019, Assumptions) | agent-resolved | prompt; docs/development/review-redesign.md#tasks (R0 row) |
| D2 | spec | Where do send outcomes show? | Success keeps today's snackbar; a failure also stays in the bar until next send or dismissed (FR-006) | agent-resolved | review-redesign.md item 4 ("also shows Sending… and the send error"); features/changes.rs sent_text/send_error_text (snackbars today) |
| D3 | spec | Bar counts: all statuses or non-zero only? | Non-zero only; "No review comments yet" when none | agent-resolved | spec Edge Cases |
| D4 | spec | Drawer tabs' content | Drafts = Draft; Replies = Sent, Applied, Answered (Mark applied / Answer… on Sent rows) | agent-resolved | review-redesign.md item 5 |
| D5 | spec | Pre-execution step skipped | branch-start.sh and issue.sh skipped: gh unauthenticated, branch already at origin/main 3a333f7f | orchestrator | prompt |
| D6 | clarify | How much of a reply does a card show? | At most the first 4 lines as drawn at the card's width, then Show all; drawer Replies rows keep the whole reply (4,000-char cap) (FR-021, US3 #2) | agent-resolved | review-redesign.md#layout items 5, 7 |
| D7 | clarify | Does the drawer open by itself? | Yes, on Replies at the first report of a send started in this window, once per send, only while the entry's Review tab is shown and not closed since the send; open drawer keeps its tab (FR-020) | agent-resolved | review-redesign.md#layout item 5; FR-019; Edge Cases "Two windows" |
| D8 | clarify | Is the auto-open rule tested without drawing? | Yes, added to FR-023 | agent-resolved | constitution I; FR-023 |
| D9 | plan | How is the work split around #1269 (not merged)? | Layer A (M1 core rules in review::{bar,drawer,reply}, M2 shared components) independent of #1269; Layer B (M3 reducer/shell/Esc wiring, M4 layout, snapshot re-record, docs, visual pass) after #1269 merges; building B on today's State.open is a fallback needing a ledger decision | agent-resolved | prompt; plan.md Delivery order; research R11 |

| D10 | tasks | Milestone sizes: M1, M2 have 15 tasks, M3 16, over rule 3's ~10 | Kept per plan D9 (orchestrator: keep the layer split); M1/M2 are pure core / self-contained components, M3 is one reducer change in features/changes.rs that cannot ship half (Panel → ReviewDrawer) | agent-resolved | prompt; plan.md Delivery order |
| D11 | tasks | speckit-analyze HIGH F1 (T015 gate needs drawer), C1 (T034/T042 too large), MEDIUM I1, A1, U1–U3, T1 | Fixed: drawer case moved to T041; reducer and layout tasks split; spec edge "Drawer opens over the line" and SC-004 reworded (plan test-strategy row follows); files-browser, 2 s and line-start checks added; T013/T014 split | agent-resolved | scratchpad analyze.md |

## Review rounds

| Review | Round | Snapshot | Verdict |
|---|---|---|---|
| Spec | 1 | fd089e07fa581ec092d2c314ae1f7f1a1ddc28a9:3a333f7f1ddf23ad4d26f471cfd59f155bed3ff2 | CHANGES: 1 MAJOR (declined), 3 MINOR (fixed) |
| Plan | 1 | 5f5fd06c9c015ce43b1c2b6043648cfdbcdfd036:eac77a56e214e2eccdb0fb472fca37e75f1274fa | CLEAN: 1 MINOR (fixed, prose only) |

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|
| — | Spec r1 | F1 MAJOR: Clear sent / Discard drafts act on the whole entry today, not the shown diff | Wrong: today they act on what is shown (reviewing-changes.md "Commenting on a pull request": "act only on the comments of what is shown"; features/changes.rs discard_prompt and status_counts use in_scope(view_scope)). The older "Clearing and discarding" wording is stale; added Story 2 scenario 5 to make the rule explicit |

## Handover

None.

## Open escalation

None.

## Follow-ups not done

- docs/user-guide/reviewing-changes.md "Clearing and discarding" says Clear sent / Discard drafts act on "the entry"; they act on what is shown (PR or working tree). The R1 docs rewrite (FR-024) should fix it.
