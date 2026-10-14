# Autopilot ledger — #775 diff-view-session-prs

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Issue #775 "Diff view should show PRs created within sessions and worktrees"; labels: enhancement, flow:feature
- **Kind**: feature
- **Effort**: default
- **Issue**: #775
- **Worktree branch**: feat/775_diff-view-should-show-prs-created-within-sessions
- **Started**: 2026-10-12
- **Phase**: milestone M3
- **Next step**: M3 PR merge, then close; T015/T026/T033 visual pass still owed (see Follow-ups)

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #782 | Design PR (spec, plan, tasks) | merged | 0de0b3ce005d2cb69c99300e52e5cb6ed3da25fa |
| #825 | M1: PR list in the Changes pane | merged | 057d71e68f57170dda7af54bdc254cab626d7d47 |
| #832 | M2: select a PR and read its diff | merged | bd39d5c1c43e92ff1cdd845510a2779ad37eb743 |

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1 | T001–T015 | full | PRs of the entry's branch listed in the Changes pane | #825 | merged (T015 visual pass moved to M2) |
| M2 | T016–T026, T023b (+T015 visual pass) | full | Select a PR, read its base...head diff, return | #832 | merged (visual pass not observed) |
| M3 | T027–T032 (T033 visual pass unobserved) | full | List refreshes on status triggers, stale marked | PR opening | gate green, review A CLEAN |

## Decisions

- Clarify round 1: deleted-head PR diff is shown from GitHub (agent-resolved, FR-004). No escalation.
- Tasks: M1 (15 tasks) and M2 (11) exceed the ~10-task guide but have no split along an acceptance scenario that leaves a user-observable deliverable (state marks, selection and return are one flow); kept whole. M3 follows M2 only to avoid conflicts in `features/changes.rs`.
- Tasks: analyze M-1..M-6 fixed (T034 left to the close unit, rate-limit hold in T012, two-sessions test in T007, off-to-on in T006, too-large edge in spec, Verify commands); plan Constitution II wording made "read per (project, branch), held by the entry's view".

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|

## Review rounds

| Review | Round | Snapshot | Verdict |
|---|---|---|---|
| Spec | 1 | 31e1f276:13165b3a (orig) | CHANGES: 2 MAJOR (US2 s3 untestable; PR status setting unspecified), 3 MINOR; all fixed |
| Spec | 2 | 33713142e3fdd0cdca9affa9cea75fea057f1713:1206eb1f207ab7e71834248b5788b51af061713b | CLEAN (1 MINOR fixed) |
| Plan | 1 | 9542cb17a47a5cde8de0d120e2391d023bbabb49:4055448678d6361c09ed6bf9f33d1e0188781e25 | CHANGES: 4 MAJOR (PrState/BranchPrState mismatch, no test map, single call-site guard test, rate-limit hold), 3 MINOR; all fixed |
| Plan | 2 | 44a2dbea1d4c763aa859c8ae3c982ed7bf415968:4055448678d6361c09ed6bf9f33d1e0188781e25 | CLEAN (2 MINOR fixed, prose only) |
| Tasks | 1 | f299e17b1b36488e83cceece626596063613b3d7:a3576aeee9ba5abbc3ef2b89553f242f5bed2ac7 | CHANGES: 2 MAJOR (PullRequestRow premise, showcase paths), 3 MINOR; all fixed, wording and task split only, checked against the repo |
| M1 review B (sonnet) | 1 | d08b95eb:da6107ad | CHANGES: 2 MAJOR (no red evidence, declined with the deviation recorded; Verify not observed by B, observed by the gate), 3 MINOR (pull-forward noted, T005/T030 text amended) |
| M2 review A (high) | 1 | e91435f8:a7a35a9f | CLEAN (clippy large_enum_variant found by the gate, fixed by boxing) |
| M2 review B (sonnet) | 1 | e91435f8:a7a35a9f | CLEAN (3 MINOR: unused `_started` kept as pass-through, leftover fixed, red evidence known) |
| M1 review A (high) | 1 | a24a773c7a9fa0ff3115024bc9939df6a47f8604:addb7d2a98edc3c8c044fca7705e4b25d0154682 | CHANGES: 2 MAJOR (disconnect leaves list Loading; list read raised a notice with no route), 1 MINOR; MAJORs fixed with test, MINOR declined |
| M3 review A (high) | 1 | da4c3766:9fb42b3b | CLEAN |
| M3 review B (sonnet) | 1 | da4c3766:9fb42b3b | CLEAN |

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|
| M1 | B | tdd/cycle-log.md: no red-before-green evidence for T006-T009 (MAJOR) | Not recreatable after the fact; the deviation is recorded honestly in tdd/cycle-log.md and the tests were shown green. Red proof by mutation is left to the tdd-verify of the close unit. |
| M1 | A | changes.rs:146 stale branch list while a read for another branch is in flight (MINOR) | The window is one read long and the queued read replaces it; the view's branch changes only when the entry's branch does. Not worth a seq-branch field. |

## Handover

None. (M2 finished; the earlier handover text below is superseded.)

<!-- superseded handover:

M2 unit hit the 150k cap with code written but NOT yet compiled or tested (no gate run, no PR, nothing
pushed; the work is one local WIP commit on the branch). Branch was reset to origin/main (M1 merged).

Done (uncommitted-then-WIP, unverified):
- Core: `crates/micold-core/src/review/pr_diff.rs` (+ `pub mod pr_diff`), `pull_request.rs`
  (`PullRequestSource::diff(repo, number, now)`, `DiffReadFailure`, `diff_args`, `diff_failure`,
  `diff_outcome`, Fake `with_diff_answer/with_diff_failure/diff_calls`), `github.rs` GhCli::diff,
  `session.rs` `SessionLocation` derives `PartialOrd, Ord` + inline test; fixtures
  `tests/fixtures/gh/pr_diff_*.txt` + README section; tests `tests/pr_diff_split.rs`,
  `tests/pull_request_diff.rs` (T016-T020). A `cargo test -p micold-core --test pr_diff_split --test
  pull_request_diff` was started in background and never read: re-run it.
- Client reducer `features/changes.rs`: `Source`, `PullRequestView`, `PrDiffLoad`, `State.remembered`,
  `Msg::{SourceChosen, PullRequestDiffRead, PullRequestOpenRequested}`, `Effect::ReadPullRequestDiff`,
  `ListBody::PullRequestUnread`, `selected_file`, PR branches in `list_body/base_line/diff_body/
  can_pick/file_comments`; tests appended to `tests/features_pr_changes.rs` (T021/T022).
- Shell: `shell/pr_status.rs` (`diff_requested`, `on_diff_remotes`, `may_read_pull_requests`, timeout arm),
  `shell/daemon_sync.rs` (`PendingOp::PrDiffRemotes`, 4 arms), `shell/changes.rs` (effect wiring,
  `pull_request_address`, open on GitHub). Guard pins updated in
  `tests/pr_status_is_read_only_on_named_events.rs` (3 factory lines, `.diff(&repo` only in shell).
- UI `ui/changes.rs`: "Working tree" row + PR rows as Buttons, too-large/unread line, `Open on GitHub`.

Next steps (in order):
1. `scripts/build-lock.sh cargo check -p micold-client --tests` and fix compile errors (expect: unused
   imports, `Spans`/`parse_unified` import paths, `DiffReadFailure` Eq/Clone on `ReadingFailure`,
   `Button::with_content` arg types in `choice` closure, `PullRequestRow` selected(false) leftover,
   other `OpenView {..}` / exhaustive `Effect`/`Msg` matches elsewhere, e.g. main.rs / app.rs).
2. Run core tests, `features_pr_changes`, guard test `pr_status_is_read_only_on_named_events`.
3. T023: add `pr_diff_read_*` shell tests in `crates/micold-client/src/main_tests.rs` (model on
   `pr_list_read_*` at ~9714; `BlockingSource` at ~8893 must implement `diff`, which the compiler will
   demand): `ReadPullRequestDiff` runs `diff` on the source from the factory; failure raises no
   notice; timeout and disconnect end the read. Also add `BranchPullRequest`-style no-store coverage
   if needed.
4. T025 user guide `docs/user-guide/reviewing-changes.md` (selecting a PR, returning, merged-and-deleted
   heads, large-PR line); tick T016-T026+T023b in tasks.md; tdd/cycle-log.md M2 rows (core tests were
   written before implementation; reducer tests too, but red run not shown - record honestly).
5. T015 + T026 visual pass (quickstart §B steps 1,2,3,4,5) via `visual-pass` skill/autopilot-worker; evidence
   in specs/775-diff-view-session-prs/.
6. Review A (code-review high) on origin/main...HEAD in shadow of scoped gate, Review B (conformance,
   sonnet), full `mise run gate` (locked; MICOLD_NO_BUILD_LOCK=1 gives false failures), PR title
   `feat(775): select a pull request and read its diff (#775)`, body per tasks/pr.md, `Refs #775`.
7. Known simplification to mention in PR: plan wanted a single factory call; M1 already had two and
   M2 adds a third line (`diff`), guard updated to 3.
8. Note: shell `RemotesTimedOut` for the diff read reuses `Msg::RemotesTimedOut` with `PendingOp::
   PrDiffRemotes`.

-->

## Open escalation

None. <or: the banner as sent, and when>

## Follow-ups not done

- T015 and T026 (visual pass, quickstart Part B 1-5) NOT observed: no Xvfb/xdotool here (headless sway is possible) and no fixture worktree with open/merged/deleted-branch/too-large PRs. Evidence file `visual/evidence.md` says so. Left unticked; run in a session with a display and a fixture repo, or accept at close.
- T033 (M3 visual pass, quickstart §B 5 offline part, 6) NOT observed, same reason as T015/T026; left unticked.
- M3: trigger refresh reuses the view's stored branch; stale mark is `PrList::possibly_stale` (any shown list older than `STALE_AFTER_SECS`), caption "Possibly stale".
- M2 shape: plan wanted one factory call; there are three (list, status, diff), guard test pins 3.
- No red evidence for M2 behaviours (part 1 handover): tdd-verify mutation testing at close should cover `split_pr_diff`, the 406 mapping, `SourceChosen`.
