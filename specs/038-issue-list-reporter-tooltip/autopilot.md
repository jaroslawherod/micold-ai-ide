# Autopilot ledger — 038-issue-list-reporter-tooltip

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Implement GitHub issue #518 (https://github.com/jaroslawherod/micold-ai-ide/issues/518): in the new-worktree form's issue list, show the reporter and labels on a second, wrapping line, and show a truncated description in a tooltip after the cursor rests on a row for 3 seconds. Read the issue with `gh issue view 518` for the acceptance criteria and code pointers.
- **Kind**: feature
- **Issue**: #518
- **Worktree branch**: feat/issue-list-reporter-labels-tooltip
- **Started**: 2026-10-02
- **Phase**: 3-design
- **Next step**: Answer *Open escalation* (plan review, third counted round found MAJORs). On "accept": rebase with `branch-start.sh 525`, run `mise run test-scripts`, open PR 2 (`docs(038): clarify, plan and cut milestones for the issue list's reporter line and description tooltip`, body ending `Refs #518`). Everything else in phase 3 is done: tdd test list, analyze, tasks review CLEAN, checklists.

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #525 | Spec | merged | 96bcd68ea422d8f1e8a18dc2ea5f55808482eae1 |

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1 | T001–T017 | full | Issue rows show two wrapping lines: number and title, then the reporter and labels; showcase pose; guide | | drafted, tasks not yet reviewed |
| M2 | T018–T023 | full | Up and Down keep the highlighted issue row wholly in view | | drafted |
| M3 | T024–T031 | full | Typing a login narrows the list; reporter emphasised; hint; guide | | drafted |
| M4 | T032–T043 | full | The showcase's Tooltip has a rest-delay instance, at most three lines; existing tooltips unchanged | | drafted |
| M5 | T044–T055 | full | Resting on an issue row for 3 s shows its description; guide | | drafted |
| M6 | T056–T057 | light | Quickstart §B recorded, SC-008 measured | | drafted |

## Decisions

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|
| D1 | spec | Which spec number? | 038: `036-tooltip-follow-cursor-delay` is taken by the flow in worktree `fix-issue-430` (not on `main` yet), 037 is on `main`. | agent-resolved | `ls <worktree>/specs` across `git worktree list`, 2026-10-02 |
| D2 | spec | Does this feature build the tooltip show delay that issue #430 asks for? | No. Spec 036 (another flow) adds it. 036's delay counts from pointer entry; #518 needs "cursor still for 3 s", so the rest-delay is this feature's requirement (FR-015, FR-016) and the plan decides how it builds on 036. Stories 1 and 2 do not depend on 036. | agent-resolved | spec.md#Assumptions; `fix-issue-430/specs/036-tooltip-follow-cursor-delay/spec.md` FR-001 |
| D3 | clarify 1 | Must typing a reporter's login find that reporter's issues beyond the 1,000-issue load cap (FR-013)? | No. The reporter is searched like a label name: among loaded issues and those the existing text search beyond the cap returns. No request by author, the existing request unchanged. | agent-resolved | 034 spec.md FR-005a, FR-025; `crates/micold-core/src/github.rs` `search_args` (query is `repo:… is:issue is:open <text>`, no label filter); issue #518 "as number, title and label already do" |
| D4 | clarify 1 | How does the tooltip turn an issue's Markdown body into the description text (FR-022)? | Option A, "Strip Markdown": readable plain text. Markdown markers removed (heading `#`, emphasis, list and checkbox markers, code fences; a link shows its text), HTML comments dropped, line breaks folded into one flowing paragraph. Heading words stay as words. | decided by user | Escalation of clarify round 1 (category 1), answered 2026-10-02; spec.md#Clarifications, FR-022 |
| D5 | design | Is the tooltip's rest delay built on spec 036's show delay, or here? (D2's open plan decision) | Here, as its own mode of the shared tooltip (`Tooltip::after_rest`). 036 is at phase 1-spec with an open escalation, uncommitted, no PR; its delay counts from pointer entry and cannot restart on movement. Both can coexist; whichever merges second rebases `cdk/tooltip.rs`. | agent-resolved | research.md#R7; `fix-issue-430/specs/036-tooltip-follow-cursor-delay/autopilot.md` (Phase 1-spec), `gh pr list --head fix/issue-430` empty, 2026-10-02 |
| D6 | design | How is the body turned into plain text (FR-022, D4)? | GitHub's own `bodyText` field, whitespace folded and capped at 600 characters in core. No Markdown crate. | agent-resolved | research.md#R2, R3; `gh api graphql` on issue #518: `body` "## Problem\n\nThe issue picker…" vs `bodyText` "Problem\nThe issue picker…" |
| D7 | design | FR-007 calls scroll-into-view of the highlighted row existing behaviour. Is it? | No: no picker scrolls on a highlight move. The requirement stands; this feature builds it for the issue picker (an operation modelled on `ui/focus.rs`). | agent-resolved | research.md#R6; no scroll call in `material/picker.rs`, `material/typeahead.rs`, `cdk/picker.rs` |

## Review rounds

| Review | Round | Snapshot | Verdict |
|---|---|---|---|
| Spec | 1 | 562a5e7f5fec78d4f0c7fc40b756ae28c5f93b5d:91e721245225f5de62861883e63a5ad816a190d7 | CHANGES: 3 MAJOR (FR-016 vs rest tolerance, FR-005 untestable, SC-008 unmeasurable), 3 MINOR; all six fixed |
| Spec | 2 | 8b82c5e2181ac182fb31b8e4e9aad9f2b2e5603a:91e721245225f5de62861883e63a5ad816a190d7 | CLEAN |
| Plan | 1 | 9923485e69511a2c10e2f2e1e22316a07dff8c5b:24a25da1e708257eeac7a9fca0088a73bb210c35 | CHANGES: 2 MAJOR (quickstart B11 cannot observe FR-018 or SC-006; seven FRs in no test layer), 3 MINOR (SC-008 fallback against FR-026, transition frames in rest mode, M-numbers undefined in the plan); all five fixed |
| Plan | 2 | 08de3d166cc2661eb82c085ccd62e810fdc147d0:4dbb2d7617200602dce3057abad6733f53a0bab8 | CLEAN (scoped re-review of the round 1 fixes; it made 4 tool calls, so FR coverage of the plan's two tables was also checked by script: none missing) |
| Plan | 3 (full: stale snapshot after the rebase; covers the part-2 edits; counted, it found MAJORs) | 7bab6b0f738cf24142cc0c9a03b9b655df4a87ec:d9deabff8423a7318f5b20fd470ed9ba34be9513 | CHANGES: 2 MAJOR (the layers named for SC-001 run `Before::Mounted` and never see an issue row, and no covered state opens the list; Principle VII PASS said docs ship with their milestone while `component-showcase.md` was in M6), 3 MINOR (trees omit `layout_snapshot.rs` and `evidence/README.md`; §B steps against the renumbered milestones; `delta_into_view` is private); all five verified and fixed. Third counted round with a MAJOR: no fourth round run, see *Open escalation* |
| Tasks | 1 | ec237aaf86f5485f2f287ddef7288c0655175e11:23edb9a83bbe74aee53bada0dec25aa1fceae630 | CHANGES: 2 MAJOR (T006's narrow-window states cannot be laid out, every pass uses the fixed `WINDOW`; T020 reused `delta_into_view`, whose 16 px margin contradicts T018), 3 MINOR (M1 Verify never opened the showcase; T056 named no baseline and §B10 ran three times; plan omitted the M2 and M5 `component-library.md` edits); all five verified and fixed. `speckit-analyze` before it: 0 CRITICAL, 0 HIGH, 2 MEDIUM, 7 LOW, all nine applied |
| Tasks | 2 | 7d30c44f193dc7230540e9209249191a6d449715:35fc46b8499bcad53a4273a8bd5988bfda712b26 | CLEAN (scoped re-review of the round 1 fixes; 1 MINOR, T020 wording on the focus unit tests, fixed). `checklists/requirements.md`: no unchecked item, no checklist finding in either round |

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|

## Handover

None.

## Open escalation

**Category 5, non-convergence: the plan review found MAJOR findings in its third counted round**, so
no fourth plan round was run and PR 2 is not open (review-rubrics.md, *Round limit*).
- Rounds: 1 CHANGES (fixed), 2 CLEAN, then part 2 of the design unit changed the plan (US1 split,
  milestones renumbered, a new gate) and the rebase made the snapshot stale, so round 3 was a full
  round. It found 2 MAJOR and 3 MINOR (*Review rounds*); all five were checked against the code and
  fixed in 6546cc15.
- Evidence the fixes hold: the Tasks review round 1 (a full round by a fresh reviewer, after the
  fixes) read the fixed plan against tasks.md and the code and confirmed the round-3 fix ("the
  state-opened list itself is fine … `Before::Settled` draws the overlay"). It found two further
  MAJORs on tasks T006 and T020, which also touch plan.md and `contracts/picker-row.md`; fixed in
  35fc46b8, and Tasks round 2 is CLEAN. `speckit-analyze`: 0 CRITICAL, 0 HIGH.
- Question: accept the plan as fixed and open PR 2, or review it further?
  1. (Recommended) Accept: open PR 2. The plan's post-fix text has been read by two later fresh
     reviewers (Tasks rounds 1 and 2) with no open finding.
  2. Run one more full plan review beyond the three-round limit, then PR 2 if it is clean.
  3. Hold: the user reads `plan.md` (Test strategy by layer, Delivery order) before anything merges.

## Token usage

## Follow-ups not done

- Spec 036 (GitHub issue #430, tooltip show delay) is not on `main`, and this feature does not
  build on it (D5). Both change `crates/micold-client/src/ui/cdk/tooltip.rs`; the flow that merges
  second rebases over the other.
- The existing-branch picker and `Select` do not scroll the keyboard highlight into view (features
  021 and 022). The operation this feature adds for the issue picker is generic; wiring the other
  two is outside this spec (FR-029).
