# Autopilot ledger — #1150 diff-full-file-view

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Issue #1150 "Show the full file and browse the repo's code while reviewing a diff in the Changes view" (full text via gh issue view 1150); labels: none
- **Kind**: feature
- **Effort**: default
- **Issue**: #1150
- **Worktree branch**: feat/1150_show-the-full-file-while-reviewing-a-diff-in-the
- **Started**: 2026-10-20
- **Phase**: implement M3
- **Next step**: finish M3 implement per Handover, then verify.md

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #1151 | Design PR (spec, plan, tasks) | merged | df019910c4b525b25df8b08f3fd1073a11b28f72 |
| #1156 | M1 expand hidden lines | merged | 25e95c33ff20a6a07f72abaa4d0aad4aee683cd2 |
| #1162 | M2 whole-file toggle | merged | 8109df50677a9a0635067277aa0df3137e3c3c48 |

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1 | T001–T017 | full | Expand hidden lines in a diff (MVP) | #1156 | merged |
| M2 | T018–T022 | light | Whole-file toggle | #1162 | merged |
| M3 | T023–T031 | full | Pull request head version | | implementing |
| M4 | T032–T035 | light | Pick revealed lines | | pending |
| M5 | T036–T046 | full | Browse repository files | | pending |
| M6 | T047–T053 | full | Browse at the PR head | | pending |
| M7 | T054–T062 | full | Follow references from the diff | | pending |

## Decisions

- M1 has 17 tasks (Setup + Foundational + US1): Foundational alone is no milestone, and no acceptance-scenario split exists below the gap marker. M5 has 11 tasks, kept whole: its tree, picker and back share one reducer state and no acceptance scenario splits them. Polish (T063-T064) changes no code: left to the close unit.
- analyze C1/C2: browsed-file pick pin moved to M5 (T038); M4 mirrors the existing line-pick handling.

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|

## Review rounds

| Review | Round | Snapshot | Verdict |
|---|---|---|---|
| Spec | 1 | 61e32c1ce6afda347a1774dc7b2c2ebd349b7abd:cf7c64d7d7ee21a78fe03d60d64165dd811a1c92 | round 1 CHANGES: 2 MAJOR (F1 reference undefined, F2 picking in untouched files); all fixed, 3 MINOR fixed |
| Spec | 2 | ebdf38fc38d5021afe13cc798bd796d24dabf88b:cf7c64d7d7ee21a78fe03d60d64165dd811a1c92 | CLEAN (1 MINOR fixed, prose only) |
| Plan | 1 | a062beaa985a9a6340ed2e978a69d8ea0fe6978c:6179267cd162fbbfa13ebaa171f85c55810ee0be | CHANGES: 3 MAJOR (PR trailing gap count, hunk-header conventions, test-layer map), 4 MINOR; all fixed |
| Tasks | 1 | 5266c05cc605449114f9bfbfc5ad2ee6617854f2:111e9d2846815d3132eb6a1358ea0f22e03e285a | CHANGES: 2 MAJOR (M5 deps, read_file tests placement), 3 MINOR; all fixed |
| Tasks | 2 | b4c0e78be90cc023ce70435de12ba29d12acbdfd:5c79030bf66c3416cdc958865266352c156ac6c0 | CLEAN (1 MINOR fixed) |
| M1 A (code-review high) | 1 | 14c45c7f7a26f35387a4d0b109d4b47ec909feaa:035ae6f1a54d7e33ee5b7c8c839cc1486b737d9c | CHANGES: 1 MAJOR (pick kept after run hidden) fixed + test; 1 MINOR (whole new side highlighted for non-revealable files) declined |
| M1 B (conformance) | 1 | same | CHANGES: 1 MAJOR (reshape before jump_offset on DiffRead) fixed; Verify green |
| M2 A+B (autopilot-reviewer, combined) | 1 | d46fca691da0d60a56a4503dcb86b060016d02d9:cd601b580040cabf5a3eb1eb5f57baac4bb3fe57 | CLEAN; Verify green; 1 MINOR fixed (toggle hidden for PR source until M3), 2 MINOR no action |
| Plan | 2 | 3213adb2c03edaa4431e840ab07b5a36c88de031:6179267cd162fbbfa13ebaa171f85c55810ee0be | CLEAN (3 MINOR fixed, prose only) |

## Declined review findings

- M1 A F2 (MINOR): the whole new side is highlighted for added/deleted/PR files too; a per-kind bound needs the change kind threaded into the highlight path, and the highlighter already has a time budget.

| Milestone | Review | Finding | Why declined |
|---|---|---|---|

## Handover

Unit M3 (T023-T031) stopped at 150k, UNCOMMITTED-then-WIP-committed, nothing compiled yet beyond core. Branch was reset to origin/main (old M2 commits backed up on local branch `backup/1150-m2-old`, delete after M3 PR). No M3 task is ticked in tasks.md yet.

Done (written, NOT yet built/tested):
- core: `review/browse.rs` (FileSource, Confined::new(&str), FileText, FileReadFailure+message(), `read_file(source, &Confined, force_large, gh: &dyn PullRequestSource, now)`), registered in review/mod.rs; `git.rs` read_bytes/object_sizes/worktree_bytes now pub(super); `pull_request.rs`: trait method `file_at`, `file_args`, `file_outcome`, fake `with_file_answer/with_file_failure/file_calls`, unit tests `file_at_tests`; `github.rs` GhCli::file_at; `tests/review_browse.rs` (T024).
- client reducer `features/changes.rs`: `HeadText`, `PullRequestView.head_text`, `READING_FILE`, `head_note`, `selected_key/file_kind/new_side`, generalised `whole_file/reshape/revealable/shown_hunks` to PR source, `Msg::FileRead{seq,source,path,result}`, `Effect::ReadFile{seq,project,entry,source,commit,address,path,force_large}`, `request_head` called from FileSelected/ShowLarge for PR; tests `mod head_version` appended in `tests/features_pr_changes.rs` (replaced test `selecting_a_file_of_a_pull_request_reads_nothing`).
- shell `pr_status.rs`: new `pull_request_source(app,&project)` helper replacing 3 duplicated gh-lookup blocks (needs compile check; `Arc`, `PullRequestSource` imports added).

Next:
1. Apply the edit that did NOT land: in `crates/micold-client/src/main_tests.rs` `impl PullRequestSource for BlockingSource` add `fn file_at(&self,_:&GithubRepo,_:&str,_:&RelPath,_:u64)->Result<Vec<u8>,FileReadFailure>{Ok(Vec::new())}`.
2. `shell/changes.rs`: handle `Effect::ReadFile` in `run()` (T029): `local(app,&entry)` for dir + shared_git gate; repo = `GithubRepo::from_remote_url` of `https://github.com/<owner>/<name>` taken from the first two path segments of `address`; build source via `crate::shell::pr_status::pull_request_source(app,&project)`; on a blocking thread (`off_thread`) call `browse::read_file(&FileSource::PrHead{dir,repo,commit}, &Confined::new(path.as_str())?, force_large, &*gh, now)`; answer `Msg::FileRead{seq,source,path,result}`. Errors (not local, no gh, bad address) answer `Err(FileReadFailure::HeadUnavailable)`. Also add FileRead to the stale log if wanted.
3. `ui/changes.rs` (T030): in `ui/changes.rs` around line 563 `if view.source == Source::Changes { diff.shaped(..).on_gap(..) }` drop the condition so PR diffs get gaps; show `head_note(view)` (muted Text, error tint for failures) between the layout row and the diff.
4. Build/test: `mise run test-core`(log in scratchpad core-1.log may still be waiting on the build lock; rerun), then client tests `features_pr_changes`, `features_changes`; fix compile errors (e.g. unused `changes_kind` refs, rustfmt). Update contracts/changes-view.md effect shape note (Effect::ReadFile carries source/commit/address, not FileSource).
5. T031: docs/user-guide/reviewing-changes.md - pull-request files (head version, offline limits). Tick T023-T031 in tasks.md.
6. Then verify.md flow: scoped gate + review A (high) + review B + visual pass (quickstart B steps 4, 9) + full gate with `GIT_CONFIG_GLOBAL` pointing at a clean config + PR (title ends `(#1150)`, body `Refs #1150`).

## Open escalation

None.

## Follow-ups not done

None yet.
