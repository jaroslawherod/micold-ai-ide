# Autopilot ledger — #1504 readable-worktree-titles

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Spec 1504 (issue #1504: Worktrees without a GitHub issue title should still show a readable overview); labels: enhancement
- **Kind**: feature
- **Effort**: default
- **Issue**: #1504
- **Worktree branch**: claude/project-thread-t2a600
- **Started**: 2026-10-27
- **Phase**: tasks written; speckit-analyze and tasks review round 1 pending
- **Next step**: speckit-analyze (forked) and tasks review round 1 (rubric .claude/skills/speckit-autopilot/rubrics/tasks.md; files specs/1504-readable-worktree-titles/{tasks,spec,plan,data-model}.md, contracts/location-title.md, quickstart.md; snapshot below); then specs-only gate, push, design PR title/body to scratchpad.

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1 | T001–T016 | full | Chooser behind card/row/header; GitHub ticket from the branch titles the worktree | — | pending |
| M2 | T017–T024 | full | Jira title ticket shows its summary; branch-only key gets no chip | — | pending |
| M3 | T025–T032 | full | PR and main-session titles; source named in row/header/expanded-card tooltip | — | pending |
| M4 | T033–T041 | full | Spec heading and own-commit titles, read off-thread on list change / Overview open | — | pending |
| M5 | T042–T046 | light | Failed GitHub title reads retried after 5 min; NoIssue never re-asked | — | pending |

## Decisions

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|
| D1 | spec | Spec review F1: widen "linked ticket" to branch names? | No: new term "title ticket" for title lookup only; sidebar tag, Jira chip and Jira status keys unchanged | orchestrator | spec review round 1 F1 |
| D2 | clarify | Session title above PR/spec titles in FR-001? | No, below (as drafted) | agent | specs/813-worktree-overview/spec.md#FR-015; specs/032-untitled-session-labels/spec.md#Clarifications |
| D3 | plan | Branch tickets like `feat/1504-readable-titles` (no `_` boundary)? | Also read the number via `short_branch`'s rule; spec Assumption's `-` form is not what `naming` parses | agent | research.md R2; crates/micold-core/src/naming.rs split_name; overview_text.rs short_branch |
| D4 | plan | Card title has no tooltip since 1121: where does FR-008's source go? | Row tooltip, header tooltip, expanded card line | agent | research.md R6; docs/user-guide/overview.md L131 |
| D5 | plan | Plan review 1 F3: keep Jira summaries on a failed refresh, given FR-004 "a failed or absent reading gives no title"? | Yes: FR-004 read as "a failure never yields a title of its own"; a summary from an earlier successful read stays until the board retires it, like the status | agent | research.md R4; jira_status FAILURES_BEFORE_RETIRING |
| D6 | plan | Plan review 1 F1: when are spec/commit facts read? | On worktree list or active-project change (main.rs fingerprint check, covers set_worktrees, created, included, excluded) and on Overview opening; FR-006 "once per worktree list refresh" read as once per title-refreshing occasion | agent | research.md R5; main.rs:490; features/worktree.rs:266,339,348 |
| D7 | tasks | M1 = Foundational (8 tasks) + US1 GitHub branch ticket (8) = 16 tasks, over ~10: split further? | No: US1 split along scenarios (M1: 1,3; M2: 2,4) and US2 (M3: 1,3,5–7; M4: 2,4); Foundational alone is never a milestone and is a behaviour-preserving refactor, so M1's new behaviour is 8 tasks | agent | milestones.md rules 1, 3 |
| D8 | tasks | Optional after_tasks hooks (tdd plan, docguard score), optional before_tasks docguard review | Skipped (optional; tests already first in every phase) | agent | .specify/extensions.yml |

## Review rounds

| Review | Round | Snapshot | Verdict |
|---|---|---|---|
| Spec review | 1 | 541682421a341a1fafc3450b6b9036cdf9e549c0:828bc8e24442d9c9685869c35fe69134934bd233 | CHANGES: 3 MAJOR, 2 MINOR — all fixed (F1 title ticket term, orchestrator choice; F2 FR-013 amendment; F3 edge cases; F4 FR-003; F5 SC-005) |
| Spec review | 2 | f2abbf7224da5e36c1fdee21ec8fe71ba20af4ab:45a9b105ae6d57805c765b6343f57f6e573a90c7 | CLEAN (3 MINOR folded into clarify: FR-002 wording, FR-003 rewrap, SC-005 measurable) |
| Plan review | 1 | a6363620134ea5fad5070c1cb4dacc71154cfb1e:0716b1a0459687647d249c43c1c5168d5a7f7e19 | CHANGES: 1 MAJOR, 3 MINOR — all fixed (F1 facts read on list/project change + Overview opening, `facts_round`/`facts_asked` replace `listing`, main_tests for created/included; F2 alternatives in R7/R9/R10/R11; F3 interpretation D5; F4 spec read by GitHub title ticket, D2 callers corrected) |
| Plan review | 2 | 4d7a2d63c9bec458651d6712ce7f7909685829a8:157212aa9334caef6d3266340e08ae115b0f6f8f | CLEAN (1 MINOR fixed, prose only: FR-006 now reads facts on each worktree list change and each Overview opening, spec Amendments note) |
| Tasks review | 1 | ce45dfee3920e1244eda9ffc375b938c4debd12a:8e83f798b46c61f3251596ce88fb9ca69c3f94e1 | pending (full round; also speckit-analyze pending; specs-only gate green at 8e83f798b) |

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|

## Handover

None.

## Open escalation

None.

## Follow-ups not done

None yet.
