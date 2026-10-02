# Autopilot ledger — 038-issue-list-reporter-tooltip

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Implement GitHub issue #518 (https://github.com/jaroslawherod/micold-ai-ide/issues/518): in the new-worktree form's issue list, show the reporter and labels on a second, wrapping line, and show a truncated description in a tooltip after the cursor rests on a row for 3 seconds. Read the issue with `gh issue view 518` for the acceptance criteria and code pointers.
- **Kind**: feature
- **Issue**: #518
- **Worktree branch**: feat/issue-list-reporter-labels-tooltip
- **Started**: 2026-10-02
- **Phase**: 3-design
- **Next step**: Continue the design unit from *Handover*: `speckit-tdd-plan` on the drafted tasks.md, `speckit-analyze`, tasks review, checklists, PR 2.

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
| M6 | T056–T058 | light | Quickstart §B recorded, SC-008 measured, showcase doc | | drafted |

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

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|

## Handover

Design unit, part 2, handed over at the context cap after drafting tasks.md. No PR is open; nothing
is pushed.
- **Done**: `branch-start.sh 525` (rebased; the branch is behind `origin/main` again, so run it
  again first). `plan.md`, `data-model.md`, `contracts/` (`issue-fields.md`, `picker-row.md`,
  `rest-tooltip.md`), `quickstart.md`. Plan review: round 1 CHANGES (all five fixed), round 2 CLEAN.
  `tasks.md` drafted by hand to the `speckit-tasks` format (T001–T058, six milestones, `##
  Milestones` section), and the milestones are in the table above.
- **Changed after the plan's round 2 snapshot**, so not yet seen by any reviewer: US1 split in two
  milestones (M1 rows, M2 highlight in view; the plan's Delivery order and the contracts'
  M-numbers renumbered to M1–M6); a new gate `gates/issue_rows_show_all_text.rs` named in the
  plan, `picker-row.md` §6 and quickstart §A; `Row::key` moved to M5 in `data-model.md` §7 and
  `picker-row.md` §4; quickstart evidence path is `evidence/README.md`. The tasks review must
  check these against each other. If it finds the plan itself wrong, that is a plan fix, and a
  further plan round counts as round 3.
- **Next step**: (1) `speckit-tdd-plan` (the `after_tasks` hook; 034 shipped `tdd/test-list.md` in
  its design PR, and `tdd.run` needs it): have an `autopilot-worker` run it and return the
  behaviour count and what it reordered; it adds the `[A#]`/`[U#]` ids to tasks.md. (2)
  `speckit-analyze`, fix what it finds. (3) Tasks and milestone review, round 1. (4) Checklists:
  `checklists/requirements.md` has no unchecked item today; re-check after analyze. (5) PR 2,
  `docs(038): clarify, plan and cut milestones for …`, body ending `Refs #518`; local gate `mise
  run test-scripts`.
- **Known soft spots for the tasks review**: gates under `tests/gates/` are `#[path]` modules of
  `tests/layout_snapshot.rs`, so the Verify commands filter that test binary; T018's "the shell
  chains it" is a source check; M4's PR needs the `docs-not-needed` label; M6 is tier `light`
  (visual pass and measurement, no code expected).
- **Open findings**: none.

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
