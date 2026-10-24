# Autopilot ledger — #1271 review-reviewed-marks

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: start work on issue #1271 "Review UI: reviewed marks and progress" (blocked by #1269, not merged; spec builds on its design in docs/development/review-redesign.md); labels: enhancement, flow:feature
- **Kind**: feature
- **Effort**: default
- **Issue**: #1271
- **Worktree branch**: claude/project-thread-umhor9
- **Started**: 2026-10-24
- **Phase**: tasks
- **Next step**: tasks review round 1 (orchestrator runs it on the snapshot in Review rounds, rubric tasks.md); then close checklists (all ticked), specs-only gate, design PR text. D4 still provisional; M1 needs A or B

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1 | T001–T017 | full | micold-core: change ids, whole-change read, EntryMarks, states, progress, review file v4 | — | planned (needs D4 A or B) |
| M2 | T018–T024 | full | wire SetReviewed + ReviewChanged.marks, service keeps/pushes/removes marks, client holds them; no view change (docs-not-needed) | — | planned |
| M3 | T025–T034 | full | MVP: row states, Mark reviewed control, n of m reviewed + bar, user guide | — | waits for #1269 |
| M4 | T035–T039 | light | changed-since dot, other windows' marks in the view, guide text | — | planned |
| M5 | T040–T044 | full | pull request marks and progress, guide text | — | planned |

## Decisions

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|
| D1 | spec | Reset marks on every new head commit, or per file's change? | Per file's change (a mark holds while the file's whole change is the same) | agent-resolved | spec.md#Assumptions |
| D2 | spec | Row symbol for a marked file with drafts | Reviewed wins; draft count stays on the row | agent-resolved | spec.md#Assumptions |
| D3 | clarify | Header wording: *n of m reviewed* or *n/m reviewed*? | *n of m reviewed* | agent-resolved | issue #1271 body (spec.md#Input) |
| D4 | clarify | Mark a file while a toggle hides part of its change? | A: no — Mark reviewed unavailable (tooltip) for a file with changes of a hidden kind; unmark stays available. Applied to FR-002, FR-005, Edge Cases > Toggles | provisional, pending user | spec.md US2 rationale, FR-005; Open escalation Q1 |
| D5 | tasks | Milestone cut vs plan's proposal | M1, M2 as planned; `pr_change_ids` moved to M5 (US4); plan's M3 split by story into M3 (US1), M4 (US2 + US3 view), M5 (US4); guide text and snapshot states in each story's milestone; visual pass and redesign note to the close unit. M1 (17 tasks, all of micold-core, likely >800 lines) kept whole: core-only, no observable split along a scenario, and it must land before #1269. Plan fixed: `Icon` lives in `crates/micold-material/src/icons.rs`, not micold-client | agent-resolved | tasks.md#Milestones, milestones.md rules 3, 4 |

## Review rounds

| Review | Round | Snapshot | Verdict |
|---|---|---|---|
| Spec | 1 | 5cce47b02a0a2a5998d722bd75a1292810c331b6:3a333f7f1ddf23ad4d26f471cfd59f155bed3ff2 | CHANGES: 1 MAJOR, 3 MINOR (run by the orchestrator). F1 fixed: FR-016 keeps a mark until unmarked/re-marked/entry removed, edge case and US2 scenario 6 added; F2 fixed (prose): US2 s5 limited to entries with a base; F3 fixed: older-window edge case; F4 fixed: US4 s2, PR marks kept like PR comments |
| Spec | 2 | ebf5c8ada3cf70b15771dbcb49161fbc5e7fd27f:029348b9eeae8148929f5bcf846f762b40db03b6 | CLEAN (model sonnet, scoped re-review of F1, F3, F4) |
| Clarify | 1 | b4b18fc6 | 2 questions: D3 agent-resolved, Q1 escalated, applied provisionally as A (D4) |
| Clarify | 2 | after D4 applied | No critical ambiguities (US1 s7 added to test D4) |
| Plan | 1 | 94d6338cedb4a1484792ddd227b4525777910dda:eff5ce1c9241292547074d7c78439557f5f162c8 | CHANGES: 2 MAJOR, 2 MINOR (run by the orchestrator). F1 fixed: untracked file digested as git's added file (no `U` tag), two G2 tests added (R1, data-model, contract G2); F2 fixed: `PendingOp::ReviewMark` moved to M3 with its sender; F3 fixed (prose): M2's edit of the `Msg::ReviewChanged` arm and the expected trivial conflict stated (plan, R8); F4 fixed (prose): M1 depends on D4 answered (A or B) |
| Plan | 2 | 4898b5511d4f5bfda0a94ab413c3e0ee3b0e07e7:1fffd3bd01fcc90f2a112ade02d6051e1800f7dc | CLEAN (model sonnet, run by the orchestrator, fix diff from round 1) |
| Tasks | 1 | SNAP | pending (orchestrator runs it, rubric tasks.md). speckit-analyze is not invocable here (no Skill entry); FR/SC coverage checked by grep: every FR-001–016 and SC-001–005 named by a task or milestone |

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|

## Handover

None.

## Open escalation

Clarify round 1, category 1 (product decision the repo does not settle). With the user; A applied provisionally (D4). If the user picks B or C, revise FR-002, FR-005, Edge Cases > Toggles and the Clarifications bullet.

- **Q1**: When a Committed or Uncommitted toggle hides part of a file's change, may the user still
  mark that file reviewed? FR-005 judges a mark on the whole file's change, so a mark made while
  only the committed part is shown would also cover uncommitted lines the user never saw — the
  "claims a review that did not happen" that US2's rationale rules out.
  - **A (Recommended)**: The mark control is unavailable, with a tooltip "Turn on Committed and
    Uncommitted to see this file's whole change", for a file that has changes of a hidden kind;
    files with no hidden part can be marked as usual. Evidence: spec.md US2 "Why this priority",
    FR-005, Edge Cases > Toggles.
  - **B**: Marking is always available; the mark covers the whole change, including the part the
    toggle hides.
  - **C**: Marking is always available, but the mark records only the shown part and covers the
    file only while the same toggles are on (changes FR-005).

## Follow-ups not done

None yet.
