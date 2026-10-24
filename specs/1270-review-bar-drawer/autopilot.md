# Autopilot ledger — #1270 review-bar-drawer

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Spec 1270 — issue #1270 "Review UI: review bar and comments drawer"; labels: enhancement, flow:feature
- **Kind**: feature
- **Effort**: default
- **Issue**: #1270
- **Worktree branch**: claude/project-thread-ttad83
- **Started**: 2026-10-24
- **Phase**: implement
- **Next step**: M1 gate and reviews, then push (orchestrator opens the PR)

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #1356 | design: spec, clarify, plan, tasks | merged (rebase) | f15ecc59696cceeef6c4fca16b1b7eb7d8776b45 |

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1 | T001–T007, T017–T018, T023–T026, T044–T045 | full | core rules review::{bar,drawer}, reply::card_preview, anatomy tokens, unit-tested (Layer A) | — | implemented; gate and reviews running |
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
| D12 | tasks | Checklist item "No [NEEDS CLARIFICATION] markers remain" | Ticked: clarify resolved both markers; tasks reviewer r1 found no checklist finding | agent-resolved | grep finds 0 markers in spec.md |
| D11 | tasks | speckit-analyze HIGH F1 (T015 gate needs drawer), C1 (T034/T042 too large), MEDIUM I1, A1, U1–U3, T1 | Fixed: drawer case moved to T041; reducer and layout tasks split; spec edge "Drawer opens over the line" and SC-004 reworded (plan test-strategy row follows); files-browser, 2 s and line-start checks added; T013/T014 split | agent-resolved | scratchpad analyze.md |
| D13 | M1 | The three new anatomy figures reach no client component until M2/M4, and `anatomy_call_sites` fails on an unbound, unrecorded figure | Recorded them as `NOT_BUILT` gaps in `crates/micold-client/tests/anatomy_call_sites.rs`; M2 (`ReviewBar`) and M4 (drawer layout) remove the entries, which `a_recorded_gap_that_became_bound_is_stale` enforces | agent-resolved | plan missed the client gate; anatomy_call_sites.rs RECORDED |

## Review rounds

| Review | Round | Snapshot | Verdict |
|---|---|---|---|
| Spec | 1 | fd089e07fa581ec092d2c314ae1f7f1a1ddc28a9:3a333f7f1ddf23ad4d26f471cfd59f155bed3ff2 | CHANGES: 1 MAJOR (declined), 3 MINOR (fixed) |
| Plan | 1 | 5f5fd06c9c015ce43b1c2b6043648cfdbcdfd036:eac77a56e214e2eccdb0fb472fca37e75f1274fa | CLEAN: 1 MINOR (fixed, prose only) |
| Tasks | 1 | 42a7804ed6fa03cda55289e1422ccf12057a0998:41be4d60561ee8c3d899e38224a6e17ac52eafd3 | CLEAN: 3 MINOR (F2 fixed: focus/dialog cleanup named in T037; F1, F3 task size left, D10) |
| Review A M1 (code-review high) | 1 | f9a89ea1e47eb2cbbd913fbc60a375822d8a5c1c:0150d727dfd87b94372093b4ab921f5a7101e7c8 | CLEAN: 7 MINOR (2 fixed: card_preview reuses `shortened`, `Drawer::apply` no longer clones per event; 5 declined) |

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|
| — | Spec r1 | F1 MAJOR: Clear sent / Discard drafts act on the whole entry today, not the shown diff | Wrong: today they act on what is shown (reviewing-changes.md "Commenting on a pull request": "act only on the comments of what is shown"; features/changes.rs discard_prompt and status_counts use in_scope(view_scope)). The older "Clearing and discarding" wording is stale; added Story 2 scenario 5 to make the rule explicit |
| M1 | Review A r1 | MINOR: a report while the Review tab is not shown consumes the watch | Specified: T024 and data-model `Reported` ("`shown == false` consumes the watch"), FR-020 "only while this entry's Review tab is shown" |
| M1 | Review A r1 | MINOR: `SEND_WIDTH = 176` is unmeasured | Provisional by design; T009 (M2) measures the widest label and sets it |
| M1 | Review A r1 | MINOR: `SendButton::primary` duplicates `enabled` | Data model and contract B4 name the field; kept for the component's API |
| M1 | Review A r1 | MINOR: Close/Escape on a closed drawer keep a pending watch | Intended: closing an already closed drawer is not a close; Esc must stay unused (returns false); pinned by `escape_on_a_closed_drawer_keeps_a_pending_watch` |
| M1 | Review A r1 | MINOR: a reply ending in `\n` measures an extra line | A stored reply is trimmed (`Report::reply`), so it never ends in a newline |

## Handover

None.

## Open escalation

None.

## Follow-ups not done

- docs/user-guide/reviewing-changes.md "Clearing and discarding" says Clear sent / Discard drafts act on "the entry"; they act on what is shown (PR or working tree). The R1 docs rewrite (FR-024) should fix it.
