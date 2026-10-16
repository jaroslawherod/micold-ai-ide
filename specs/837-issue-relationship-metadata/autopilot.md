# Autopilot ledger — #837 issue-relationship-metadata

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Issue #837 "Show all GitHub issue metadata (blocked by, blocking, parent, sub-issues…) when a worktree is created"; labels: enhancement, effort:high, flow:feature (orchestrator added in-progress)
- **Kind**: feature
- **Effort**: high
- **Issue**: #837
- **Worktree branch**: claude/project-thread-tw6sph
- **Started**: 2026-10-16
- **Phase**: milestone M2
- **Next step**: M2 unit 2: full gate, reviews A and B, visual pass T031, PR files (see Handover)

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #906 | Design: spec, plan, research, contracts, tasks | merged | d24fd3223f0d74d045690100555d88bc28c13900 |
| #907 | M1: tool reports the issue object | merged | 31cc373f6365636e21a56afc9474c247927d6c9d |

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1 | T001–T018, T027, T029 | full | create_worktree with github_issue returns the issue object (relationships, not_readable, details_error); agent-tools guide | #907 | merged |
| M2 | T019–T026, T028, T030, T031 | full | New worktree form shows a picked issue's relationships, blocked warning, failure line; worktrees guide; visual pass | — | implemented, not yet gated or reviewed |

## Decisions

- Clarify: US2 scenario 6, form only, the sidebar row is unchanged (option A). Default chosen by the orchestrator, pending user confirmation. If the user picks B or C, the orchestrator continues the run with that change.
- Clarify: FR-001, the form reads the details when the issue is picked; FR-011, a github_issue tool call waits up to 10 s for them (agent-resolved).
- Clarify round 2: no critical ambiguities.
- Tasks: US3 spans both paths, so its tool-side tasks (T027, T029) ship in M1 and its form-side tasks (T028, T030) in M2; the visual pass T031 runs last in M2, beside the code they test (milestones rule 7 over rule 2); no US3 milestone. M1 cannot be split below ~800 lines: the shared core read (one query) is the first thing anything observable needs, and the tool result is its first observable. Polish T032–T034 change no code: left to the close unit.
- Plan: one core read (`micold_core::issue_details`) shared by tool and form; tool starts it in parallel and awaits after create under a 10 s bound; form reads at pick with seq-tagged state; linked-PR count pages the timeline within the deadline and marks a lower bound otherwise (research R4); no sidebar change, model kept form-independent for a later marker (R10).
| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|

## Review rounds

| Review | Round | Snapshot | Verdict |
|---|---|---|---|
| Spec review | 1 | 939e5694d83a882c00de3c9ff98235ef44a95c5d:8a04f4206330fa7552531e4694e2244b186d345f | CLEAN: 3 MINOR, all fixed (F1, F3 prose only; F2 added FR-011/SC-005 tool-call bound, no new round needed after CLEAN) |
| Plan review | 1 | a8e72b879aa633e947404ec746d63f19b023f49e:43b70bc145326616180860e5edd7b9dc6af331d9 | CLEAN: 3 MINOR, all fixed (F1 layout snapshot row in test strategy, F2 FR map table, F3 u32::try_from in form-details §2; prose/plan text only, no new round) |
| Tasks review | 1 | 260d2c07b1b94129a9328339d2423d85175cff4b:f76fc297161ba35ca11d21a56082b7c2c25ff1af | CLEAN: 3 MINOR, all fixed (F1 [P] dropped on T003; F2 T030 gains a Failed layout snapshot case; F3 stale checklist note; task text only, no new round) |
| Review A M1 (code-review high) | 1 | ab97d95509aaf33fa9330bbbcc3623ec52b7e9a7:50869c2ec6b2d574c4f8ccb2a8231984f4fb8bdc | 10 findings: F1 fixed (500 ms grace past the bound so a read cut at its own deadline keeps what it read), F4 fixed (typed GraphQL error beside a null issue, + test); rest declined or MINOR, see Declined |
| Review A M1 (code-review medium, fix diff) | 2 | ab97d95509aaf33fa9330bbbcc3623ec52b7e9a7:50869c2ec6b2d574c4f8ccb2a8231984f4fb8bdc..5af600b3 | 8 findings on the grace fix: replaced it, the read now ends `DETAILS_MARGIN` (`process::RUN_OVERRUN` + 200 ms) before the bound and the waiter waits exactly the bound (FR-011 holds); doc comment placement fixed; null-issue error prefers the error at `repository.issue` (+ test). No regression test for a read cut at its deadline (needs a real `gh`; MINOR). |

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|
| M1 | A | F2 details read keeps running after a refused call | By design (plan, FR-011): the read starts beside the lookup to hide its latency; it is bounded by the 10 s deadline and its result is dropped. |
| M1 | A | F3 private projects / invisible labels count in `more` | Matches contracts/tool-result.md: `more = total - len(items)`, hidden items included. MINOR. |
| M1 | A | F5 `hasNextPage` with null cursor not a lower bound | GitHub always sends a cursor with `hasNextPage: true`; MINOR edge. |
| M1 | A | F6 open-blocker fallback when the summary is missing | The summary is always selected; fallback only on a partial answer. MINOR. |
| M1 | A | F7 absent `parent`/`milestone` key read as none | GraphQL returns every selected field (null when none); an omission comes with an `errors[]` entry, already mapped to NotReadable. |
| M1 | A | F8 two queries per call | Plan decision: the lookup's query is 550's, unchanged (SC-004); details read separately and in parallel. |
| M1 | A | F9 `IssueSource: IssueDetailsSource` unused | M2's form reads details through `IssueSource` (T020–T022). |
| M1 | A | F10 `VisibleChars` two buffers | Taste, MINOR. |

## Handover

M2 unit 1 stopped at the context cap (about 178k). Done, uncommitted until this commit: T019–T026, T028, T030 ticked in tasks.md; `tdd/cycle-log.md` has the M2 section (tests written and run red first).

- Core: `DetailLine`, `IssueDetails::display_lines` (issue_details.rs); `FakeIssueSource::details_deadlines`.
- Client: `IssueDetailsState`, `DetailsRequest`, `set_picked_issue` (the one writer), `issue_details_request`, `Msg::IssueDetailsLoaded` (features/worktree_form.rs); `shell::issues::on_issue_picked` + `start_issue_details` (main.rs routes `IssuePicked`); view `issue_details` inside `issue_picker` (ui/worktree_form.rs), scrolls past 5 rows because 9 rows pushed the dialog over the app bar (panel_placement); new gate exemption `ISSUE_DETAILS_CONTENT` in tests/gates/containment.rs; three covered states `add-worktree-dialog-issue-details-{loading,loaded,failed}` and the regenerated layout fixture.
- Docs: worktrees-and-sessions.md "The issue's relationships" and a sentence in "What is sent to GitHub".
- Green so far: core `issue_details` 44, client `issue_source` 47, `layout_snapshot` 62. Full client and workspace tests, clippy and the shell tests NOT run yet.

Next, in order: (1) `CARGO_INCREMENTAL=0 scripts/gate.sh` detached (setsid nohup, log in scratchpad), with review A (`code-review` skill, high, on origin/main...HEAD) in its shadow; (2) review B (autopilot-reviewer, model sonnet, conformance rubric, brief M2) and the visual pass T031 (cloud-visual-pass notes: Xvfb, ICED_BACKEND=tiny-skia, python ctypes XTest; save screenshots and notes in specs/837-issue-relationship-metadata/visual-pass/, tick T031); (3) record review rounds with review-snapshot.sh; (4) rerun the full gate on the final tree, commit (feat(837): …), push with `git push --force-with-lease -u origin HEAD`, write SCRATCH/pr-title.txt and pr-body.md (title `#837: Show all GitHub issue metadata (blocked by, blocking, parent, sub-issues…) when a worktree is created (M2)`, body ends `Refs #837`); return `PR: pending (orchestrator opens)`.
Open findings: none yet. Review rounds: none counted.

## Open escalation

None. (Sidebar question: option A applied as orchestrator default, pending user confirmation; see Decisions.)

## Follow-ups not done

None.
