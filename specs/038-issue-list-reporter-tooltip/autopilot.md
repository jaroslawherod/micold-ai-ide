# Autopilot ledger — 038-issue-list-reporter-tooltip

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Implement GitHub issue #518 (https://github.com/jaroslawherod/micold-ai-ide/issues/518): in the new-worktree form's issue list, show the reporter and labels on a second, wrapping line, and show a truncated description in a tooltip after the cursor rests on a row for 3 seconds. Read the issue with `gh issue view 518` for the acceptance criteria and code pointers.
- **Kind**: feature
- **Issue**: #518
- **Worktree branch**: feat/issue-list-reporter-labels-tooltip
- **Started**: 2026-10-02
- **Phase**: 2-clarify
- **Next step**: Ask the user the open escalation (FR-022), then continue clarify round 1 with the answer: record it in spec.md *Clarifications* as `_(decided by user)_`, replace the FR-022 marker, add its acceptance scenario, tick the three open checklist items, commit. Round 1's FR-013 answer is committed, not pushed (it ships in PR 2).

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

## Review rounds

| Review | Round | Snapshot | Verdict |
|---|---|---|---|
| Spec | 1 | 562a5e7f5fec78d4f0c7fc40b756ae28c5f93b5d:91e721245225f5de62861883e63a5ad816a190d7 | CHANGES: 3 MAJOR (FR-016 vs rest tolerance, FR-005 untestable, SC-008 unmeasurable), 3 MINOR; all six fixed |
| Spec | 2 | 8b82c5e2181ac182fb31b8e4e9aad9f2b2e5603a:91e721245225f5de62861883e63a5ad816a190d7 | CLEAN |

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|

## Handover

None.

## Open escalation

Clarify round 1, category 1 (product decision the repo does not settle). One question, FR-022:

**How should the tooltip turn an issue's Markdown body into the description text?**

- **A (Recommended)** Readable plain text: Markdown markers removed (heading `#`, emphasis, list and
  checkbox markers, code fences; a link shows its text), HTML comments dropped, line breaks folded
  into one flowing paragraph. Heading words stay as words. Issue #518's tooltip would read
  `Problem The issue picker in the new-worktree form shows each issue as one line: …`.
- **B** As written, but line breaks folded: `## Problem The issue picker in the …`. Markers and
  template comments stay visible.
- **C** Exactly as written, line breaks kept: for #518 the three lines are `## Problem`, a blank
  line, and one line of text.

Evidence: nothing in the repo decides it. The workspace has no Markdown renderer (no such crate in
any `Cargo.toml`) and the shared tooltip draws plain text, so "render the Markdown" is not an option
without a new dependency. Issue #518 asks for "a short description ... the start of its body,
truncated" in a tooltip that "stays small" (three lines, FR-021); its own body opens with
`## Problem` and a blank line, so C spends two of three lines on no content. A needs a stripping
rule the plan must define and test; B and C need none.

## Token usage

## Follow-ups not done

- Spec 036 (GitHub issue #430, tooltip show delay) is not on `main`. User story 3 (the description
  tooltip) may build on it; the design unit must check its state before cutting milestones.
