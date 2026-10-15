# Quickstart: PRs in the diff view (#775)

## Part A: automated
```
mise run test-core        # query, list parse, diff split, fakes
mise run gate             # reducer, shell and doc-link checks
```
Expect green. Key tests: `pull_request_list`, `pr_diff_split`, `features_pr_changes`,
`pr_status_is_read_only_on_named_events`.

## Part B: visual pass (needs `gh` signed in, a repo with a PR from a worktree branch)
1. Settings, GitHub: turn "Show pull request status" on.
2. Select a worktree whose branch has an open and a merged PR; open Changes. Expect a "Working tree"
   row and both PRs, newest first, with distinct open/merged marks, above the working-tree files.
3. Select a PR: its files and diff appear in both layouts; select "Working tree": back (FR-005).
4. Select a merged PR whose branch is deleted: diff still shows (US2 s3).
5. Turn the setting off: the section disappears, no notice (FR-014). Go offline: the last list stays,
   marked possibly stale after 10 minutes (FR-012).
6. Merge or close a PR on GitHub: within 5 minutes the state mark changes (SC-004).

## Results (recorded at close, 2026-10-14)

- **Part A**: `mise run gate` green on each milestone PR (#825, #832, #836); the close PR's gate
  result is in its PR body. `tdd/verification.md` ran 21 hand-applied mutants over `split_pr_diff`,
  the 406 mapping and `SourceChosen`; the 10 real survivors (findings 1-9) got tests in the close PR.
- **Part B**: NOT OBSERVED, accepted limitation. The environment has no Xvfb/xdotool and no
  fixture repo with open, merged, deleted-branch and too-large PRs; creating PRs on GitHub was out
  of scope. Steps 1-6 were not run; see `visual/evidence.md`. T015, T026 and T033 stay unticked.
  The behaviours they cover are pinned by reducer and shell tests (`features_pr_changes`,
  `pr_list_read_*`, `pr_diff_read_*`, `pr_list_refresh_*`), not by pixels.
