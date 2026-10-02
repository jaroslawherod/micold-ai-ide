# Autopilot ledger — 038-issue-list-reporter-tooltip

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Implement GitHub issue #518 (https://github.com/jaroslawherod/micold-ai-ide/issues/518): in the new-worktree form's issue list, show the reporter and labels on a second, wrapping line, and show a truncated description in a tooltip after the cursor rests on a row for 3 seconds. Read the issue with `gh issue view 518` for the acceptance criteria and code pointers.
- **Kind**: feature
- **Issue**: #518
- **Worktree branch**: feat/issue-list-reporter-labels-tooltip
- **Started**: 2026-10-02
- **Phase**: 3-design
- **Next step**: Continue the design unit from *Handover*: write plan.md from research.md, then data-model, contracts, quickstart; plan review; tasks and milestones.

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #525 | Spec | merged | 96bcd68ea422d8f1e8a18dc2ea5f55808482eae1 |

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|

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

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|

## Handover

Design unit, handed over at the context cap after the research step. No PR is open.

- **Done**: `branch-start.sh 525` (3 clarify commits rebased on `origin/main`); the codebase and
  spec 036's state investigated; `research.md` written (R1–R15, with a table of where the code is
  today). Decisions D5–D7 recorded above.
- **Next step**: `speckit-plan` from the top, but do not redo the research: run
  `.specify/scripts/bash/setup-plan.sh --json` (it copies the plan template; the copy made earlier
  was removed so no raw template is committed), fill `plan.md` from `research.md` (use
  `specs/034-github-issue-worktree/plan.md` as the shape: summary, technical context, constitution
  check, requirement → design map for FR-001–FR-031, test strategy by layer, structure, delivery
  order, risks), then `data-model.md`, `contracts/` (suggested: `issue-fields.md` for R1–R4 and
  R13, `picker-row.md` for R5, R6, R10 and R12, `rest-tooltip.md` for R7–R9, R11 and R15),
  `quickstart.md` (§A automated, §B recorded pass, SC-008 measured before and after). Then the plan
  review (round 1 not yet run), `speckit-tasks`, milestones, `speckit-analyze`, tasks review,
  checklists, PR 2.
- **Milestone sketch** (to confirm when tasks exist): M1 US1 two-line rows with the reporter,
  highlight kept in view, showcase, guide; M2 US2 search by reporter, hint, guide; US3 likely split
  in two (the rest-delay tooltip in the component library and showcase; then descriptions on issue
  rows, guide); last, Polish.
- **Open findings**: none. No review has run in this phase.

## Open escalation

None.

## Token usage

## Follow-ups not done

- Spec 036 (GitHub issue #430, tooltip show delay) is not on `main`, and this feature does not
  build on it (D5). Both change `crates/micold-client/src/ui/cdk/tooltip.rs`; the flow that merges
  second rebases over the other.
- The existing-branch picker and `Select` do not scroll the keyboard highlight into view (features
  021 and 022). The operation this feature adds for the issue picker is generic; wiring the other
  two is outside this spec (FR-029).
