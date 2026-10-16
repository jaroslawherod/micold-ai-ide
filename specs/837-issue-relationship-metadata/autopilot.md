# Autopilot ledger — #837 issue-relationship-metadata

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Issue #837 "Show all GitHub issue metadata (blocked by, blocking, parent, sub-issues…) when a worktree is created"; labels: enhancement, effort:high, flow:feature (orchestrator added in-progress)
- **Kind**: feature
- **Effort**: high
- **Issue**: #837
- **Worktree branch**: claude/project-thread-tw6sph
- **Started**: 2026-10-16
- **Phase**: milestone M1
- **Next step**: M1 implement continues from Handover (T002 finish, red run, T007+), then verify and PR

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #906 | Design: spec, plan, research, contracts, tasks | merged | d24fd3223f0d74d045690100555d88bc28c13900 |

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1 | T001–T018, T027, T029 | full | create_worktree with github_issue returns the issue object (relationships, not_readable, details_error); agent-tools guide | — | in progress |
| M2 | T019–T026, T028, T030, T031 | full | New worktree form shows a picked issue's relationships, blocked warning, failure line; worktrees guide; visual pass | — | planned |

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

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|

## Handover

M1 unit 1 hit the 150k cap during implement. `branch-start.sh` was skipped: `gh` is not signed in here, and HEAD already equalled origin/main = #906's merge d24fd322.

**Done**: T001 (fixtures in `crates/micold-core/tests/fixtures/issue_details/`, field names confirmed against GitHub's published schema, recorded in research.md R3). T002 partly done: `pub mod issue_details;` added and the module doc written; the test file exists. The T003–T006 and T011 tests are written in `crates/micold-core/tests/issue_details.rs`, plus `read_details_past_its_deadline_runs_no_gh` in `tests/github_gh_cli.rs`. They do not compile yet, because the module has no items. That is the red step. No red run has been recorded yet.

**Next**: for TDD evidence, add the API as `todo!()` stubs, run `cargo test -p micold-core --test issue_details`, and record the panics in `tdd/cycle-log.md`, keyed by task as specs/804 does. Then implement T007–T010 and T014, then the daemon tasks T012/T015/T016/T027/T029, then T013/T017, then T018.

**API the tests expect** (`micold_core::issue_details`):
- `PlainText::new/as_str/is_empty`, `Display`.
- `Reference::{Here(u64), Other{owner,name,number}}`, with `Reference::of(name_with_owner, number, &repo)` that ignores ASCII case, and `Display`.
- `RelatedIssue{pub reference, pub title, pub open}` and `RelatedPr{reference, title, state}`.
- `PrListState::from_fields(state, is_draft, merged)`.
- `pub const LIST_CAP = 20`.
- `RelatedList::new(items, total, hidden)`: truncates to 20 and sets total = max(total, shown+hidden). Accessors `shown() total() hidden() more() is_empty() total_is_lower_bound()`.
- `Kind<T>{Read, NotReadable(String)}` (PartialEq, Debug), with `.reason() -> Option<&str>`.
- `Blockers{pub list, pub open}` and `SubIssues{pub list, pub closed}`.
- `labels`, `assignees` and `projects` are `Kind<RelatedList<..>>`, not the `Vec` data-model shows, because they need total and more.
- `ProjectMembership{pub name, pub fields: Vec<(PlainText,PlainText)>}`.
- `IssueDetails` has pub fields `number`, `blocked_by`, `blocking`, `parent`, `sub_issues`, `labels`, `assignees`, `milestone`, `projects`, `closing_prs`, `linked_prs`. It has `is_blocked()`, `to_json()` and a hand-written `Debug` that prints counts only.
- `DETAILS_BOUND`, `details_args(repo,u32)`, `timeline_args(repo,u32,after)`, `parse_details(&[u8],&repo)` and `details_error_json(u64,&err,&repo)`.
- `read_details_with(repo, number, deadline, FnMut(Vec<String>, Duration) -> RunOutcome)` is the paging driver. It returns TimedOut before a first run past the deadline, runs at most 10 runs in all, and a failed or late page sets the lower bound.
- The `IssueDetailsSource` trait.
- `FakeIssueSource::{with_details (a queue), with_details_delay, details_calls}`.

**Design notes**:
- Extract `GhCli::run`'s outcome handling into `pub(crate) fn answer_of(outcome, parse: impl Fn)`. Add a `GhCli` runner with an explicit timeout of min(left, self.timeout). The GhCli impl is `read_details_with(.., |a,l| self.outcome_within(a,l))`.
- T007: one `pub(crate) fn visible_chars(&str) -> impl Iterator<Item=char>` in github.rs holds the joiner logic. `description_from` (its per-word loop and the skip check) and `PlainText::new` both use it.
- Error path: `["repository","issue",<field>]` maps a field to its kind. For `issueDependenciesSummary`, a subfield containing "locking" goes to blocking and "lockedBy" to blocked_by.
- Reason for `INSUFFICIENT_SCOPES`: "your GitHub sign-in can't read projects (it lacks the read:project scope)". Another kind gets "can't read <noun>" without the scope clause.
- Field values: the value comes from the first of text, number, date, name or title. A whole number is formatted without ".0".
- A timeline source without `number` is an issue, and is skipped.
- Daemon: in `resolve_naming`, a first blocking call gets the lookup Arc and the mapping. Then, if there is a lookup, `spawn_blocking` the details read with deadline = now + `details_bound`. Then a blocking `read_issue`. The function returns `(naming, Option<PendingDetails{handle, started, number, repo}>)`. In `create_derived_worktree`, after `worktree_row`, await `timeout(bound - elapsed)`. On elapsed, or on a join error, report `details_error_json` with `TimedOut` or `Other`.
- Daemon tests: build `IssueDetails` with `parse_details(include_bytes!("../../micold-core/tests/fixtures/issue_details/every_kind.json"), acme/widgets)`. Existing rows of `github_issue` calls now carry `issue.details_error` "no details scripted". Check whether any existing test compares a whole row.

## Open escalation

None. (Sidebar question: option A applied as orchestrator default, pending user confirmation; see Decisions.)

## Follow-ups not done

None.
