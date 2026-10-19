# Autopilot ledger — #1003 review-worktree-prs

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Review changes: allow navigating between all PRs of the worktree (only the first PR's diff is reachable) — issue #1003 (body: Problem / Expected / Acceptance criteria as in the issue). labels: enhancement, flow:feature
- **Kind**: feature
- **Effort**: default
- **Issue**: #1003
- **Worktree branch**: claude/project-thread-c3olht
- **Started**: 2026-10-19
- **Phase**: implement
- **Next step**: M1 implement, continue from *Handover*

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #1089 | design | merged | 6fbd754f11c2d0ed387dfb0ae886c0070ac2d573 |

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1 | T001–T014 | full | Every PR of the worktree's branch set listed (number, title, state), each diff reachable | — | in progress |
| M2 | T015–T021 | full | Check mark on every entry, one mapping shared with the sidebar | — | pending |
| M3 | T022–T031 | full | Header + previous/next; selection and header hold across refreshes (US2 + US3) | — | pending |
| M4 | T032–T036 | full | Loading, empty and unavailable lines | — | pending |

## Decisions

- Clarify round 1: FR-002 branch set = current branch + branches created in the worktree; branches only checked out there do not count (agent-resolved from issue #1003 acceptance criteria and the spec's edge case). Records an existing rule, so no further round.
- Tasks: US1 split (21 tasks) along its scenarios: 1, 2, 4 in M1, scenario 3 (check status) in M2. US3 adds no code (tests pinning behaviour M1 and M3 deliver), so it rides in M3 instead of a milestone with no deliverable. Polish (T037, quickstart results) has no code: close unit.
| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|

## Review rounds

| Review | Round | Snapshot | Verdict |
|---|---|---|---|
| Spec | 1 | 42282bddf03c394c74f6c803c06440fd7abe2179:a47882ccc0f123e0a991ca87f46e40c20c50dc73 | CHANGES: 1 MAJOR, 3 MINOR |
| Spec | 2 | fddeb540b934ab3d57ea2fc9d3a6deb18973bed2:a47882ccc0f123e0a991ca87f46e40c20c50dc73 | CLEAN (2 MINOR, fixed prose only) |
| Plan | 1 | c266bc56ee3e8390242f9f9ac5837d2be0fac6a9:074c8404d1ad2213236588c1fda9287d199b7d25 | CHANGES: 1 MAJOR (F1 DWIM remote-branch copy counted as created; fixed: R2 rule 3 drops a same-name remote copy, narrower than suggested so `switch -c x origin/main` still counts), 1 MINOR (F2 source on Err(Unavailable); fixed) |
| Plan | 2 | 10adbf465724a6cc47c51564250b6a409eeadb77:78604c31bcfb0b9e25d7865a5e0193932fee29a6 | CLEAN (1 MINOR wrap, fixed prose only) |
| Tasks | 1 | 2e2d92e5e664b0fd615696e35713ac0ca7c21346:190120bc88951f940720a69dafb552a7213128ef | CLEAN (1 MINOR: T027 visual pass covers §B step 4; fixed prose only) |

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|

## Handover

M1 unit 1 handed over at 150k (commit "test(1003): M1 core tests, fixtures and Git::reflogs (WIP)").

- **Base**: branch reset to origin/main 6fbd754f by hand (`branch-start.sh` needs `gh`, not signed in
  here; #1089 merged). Do not run branch-start again: continue on the branch as it is.
- **Done**: T001 fixtures (`pr_list_branches_{three,reopened,default,fork,tie,empty,unknown_state}.txt`;
  the five single-branch `pr_list_*.txt` removed, superseded). `Git::reflogs(dir, refs)` in
  `git.rs` (trait, `GitCli` via `review/git.rs::reflogs` = bounded `read`, `FakeGit::with_reflog(dir,
  refs, text)`, unscripted call = Err). Tests written, red NOT yet observed/recorded:
  T002 (`src/worktree_branches.rs` inline tests + stubs, `pub mod worktree_branches` in lib.rs),
  T004 (`tests/worktree_branch_set.rs`, real git with `GIT_COMMITTER_DATE`-pinned times), T006
  (`tests/pull_request_list.rs`, rewritten; does not compile yet).
- **API the T006 tests expect** (pull_request.rs): `list_query(n) -> String`, `list_args(repo,
  &[String])`, `parse_list(body, branches)`, `read_list_outcome(outcome, branches, now)`,
  `list_in_chunks(branches, FnMut(&[String]) -> Result<Vec<BranchPullRequest>, ReadingFailure>)`
  (chunks of `LIST_BRANCHES_PER_REQUEST`=25, empty = no call, merged + sorted created_at desc then
  number desc; `GhCli::list_branches` uses it like `read_in_chunks`), `BranchPullRequest.head_branch`,
  `PullRequestSource::list_branches(repo, &[String], now)` replacing `list_branch`,
  `FakePullRequestSource::list_calls() -> Vec<(String, Vec<String>)>`. Default-branch alias dropped
  via `defaultBranchRef.name == branches[i]`; a missing alias `p<i>` is `Passing`. M2 adds checks,
  so the M1 query has `headRefName` but no rollup.
- **Branch-set rules as tested**: `candidates` = current first, then HEAD log names newest first,
  deduped; rename X→Y links both names (HEAD on X at creation matches Y).
- **TDD**: no `tdd/test-list.md` exists (the `tdd.run` hook would stop); follow tasks.md's test-first
  order and log each red/green in `specs/1003-review-worktree-prs/tdd/cycle-log.md`.
- **Next**: stubs in pull_request.rs → run core tests, record reds → T003, T005 (`read_branch_set`),
  T007 (+ github.rs, client `main_tests.rs` BlockingSource, `shell/pr_status.rs` caller) → T008–T014.
- **Build**: a warm `cargo test --workspace --no-run` was started detached with
  `CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0` (keep that env; scratchpad `warm.log`).

## Open escalation

None.

## Follow-ups not done

