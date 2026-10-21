# Autopilot ledger — #1122 overview-card-issue-link

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Implement GitHub issue #1122: Overview cards: link to the worktree's issue. Blocked by: #1120 (the branch line the link sits on). "Issue #N" beside the branch opens the GitHub issue in the browser; N comes from the worktree name, as the issue-title summary already does. A Jira key (MIC-87) in the name links to `<Jira base URL>/browse/<key>`, which needs a Jira base URL setting and shows no link while it is unset. A press on a link does not activate the card. No link on Default or on a worktree with no issue. Acceptance: both links open the right URL; Settings has the Jira field; card activation is unaffected; docs updated. labels: enhancement, flow:feature, in-progress
- **Kind**: feature
- **Effort**: default
- **Issue**: #1122
- **Worktree branch**: claude/project-thread-7041o6
- **Started**: 2026-10-21
- **Phase**: tasks
- **Next step**: tasks unit: review A of tasks round 2 (sonnet, fix diff) requested; then specs-only gate and design PR text. D3, D4 still defaults awaiting the owner; tasks tag the affected tasks `(D3)` / `(D4)`.

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1 | T001–T009 | full | Link rules proven in core; showcase shows the shared `TextLink` | — | planned |
| M2 | T010–T019 | full | Showcase shows cards with "Issue #1122", click opens only the link | — | planned |
| M3 | T020–T030 | full | GitHub issue links on the Overview; rest on link and arrows as specified | — | planned |
| M4 | T031–T038 | full | Issue links by keyboard (focus counts for the card, Menu key returns to link) | — | planned |
| M5 | T039–T050 | full | Jira base URL setting (wire 58) and Jira issue links | — | planned |

Cut notes: US1 (P1) split in three (M1–M3): the whole story is ~30 tasks and well over 800 lines, and each part has its own deliverable (core + `TextLink`; the card in the showcase; the app). M1 and M2 are showcase-only (`docs-not-needed`). M2 is 10 tasks: `ControlOverlay`, the card and the grid change together (one routing decision, R6–R8) and have no smaller observable split. US3 (M4) goes before US2 (M5), both P2, because it completes the keyboard path of the link M3 puts in the app. Rule 6 (review A tasks r1 F1): the link stays out of the Tab order until M4 (`TextLink::focusable(false)` while the card has no `.on_link_keyboard_focus`), so M3 leaves the keyboard exactly as before; design fixed in contracts/card-link.md U1/U3/U6 and research R5. Polish T051–T052 is code-free: close unit.

## Decisions

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|
| D1 | spec | How is the issue link reached by keyboard, given 1121's focus-expands rule? | A Tab stop right after its card; arrows move between cards only; focus on the link counts as focus on the card | agent-resolved | spec.md#FR-008; specs/1121-overview-card-full-text/spec.md#FR-002 |
| D2 | spec | A collapsed card's link moves when the card expands (lines are added above it); how is a click aimed at it kept off the card? | A pointer resting on a collapsed card's link does not expand the card | agent-resolved | spec.md#FR-007 |
| D3 | clarify | Do the issue links follow the integration switches (FR-001)? | Yes: GitHub link only while the GitHub issues switch (1102) is on, Jira link only while the Jira integration switch (533) is on | default; awaiting owner's card answer | spec.md#FR-001; Story 1 sc. 6, Story 2 sc. 7 |
| D4 | clarify | Where does the Jira base URL come from (FR-010)? | One app-wide Settings field, as the issue asks; no link while unset | default; awaiting owner's card answer | spec.md#FR-010; issue #1122 body |

## Review rounds

| Review | Round | Snapshot | Verdict |
|---|---|---|---|
| Review A spec | 1 | 399604367e0651717c12d766941fef7e892d58e9:48d810a53d17d50a45ab3b3ffaabec63220e6048 | CHANGES: 5 MAJOR, 3 MINOR (F1–F8), all fixed in spec.md, none declined |
| Review A spec | 2 | 5b39717f:af373d3b (fix diff, sonnet) | CHANGES: 1 MAJOR (F1 FR-005/FR-006 press-off-link contradiction, ripple on press), 1 MINOR (F2 SC-007 not measurable); both fixed, prose only, no round 3 |
| Review A plan | 1 | 200e0ca569743faecd1117df47127d69ad60e7c2:29f7e6c71c906f99358fc938b2e83ab638dfb25c | CHANGES: 3 MAJOR (F1 link Menu key origin, F2 focus pairing across the menu, F3 repo not re-read on project switch), 2 MINOR (F4 R6 scrollbar cursor, F5 R11 rejected alternatives); all five fixed in research/data-model/contracts/plan, none declined |
| Review A plan | 2 | dbc05dbcce3d2007986d9a9952f95f8041e77c0f:f8b66178b945888957cd553d9d889aff3a718669 (fix diff 29f7e6c7..f8b66178, sonnet) | CLEAN, 2 MINOR (F1 card-id fallback ends the menu hold, F2 deferred loss under an open menu); both fixed in data-model + K5, prose and test list only, no round 3 |
| Review A tasks | 1 | 1922d8f94766790a9b58f3c179918ec99ab12f8a:989fdbface58399752e833df82bfadf17c1174a6 | CHANGES: 4 MAJOR (F1 M3 Tab stop half-wired, F2 adopt test in a binary-only fn, F3 link-focus hand-offs untasked, F4 [P] tasks share files), 3 MINOR (F5 literals, F6 private daemon helpers, F7 D3/D4 tags); all fixed (F1 by keeping the link out of the Tab order until M4: contract U1 `.focusable`, U3), none declined |
| Review A tasks | 2 | 0368cc294df0c6db78913928089c602e7870219f:69649f215a54be0ba4162c45fea1cf8a177d54f7 (fix diff 989fdbfa..69649f21, sonnet) | pending |

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|

## Handover

None.

## Open escalation

None.

## Follow-ups not done

None yet.
