# Quickstart: Review changes reaches every pull request of the worktree (#1003)

## Part A: automated

```
scripts/gate.sh            # (mise run gate where mise is installed)
```

Expect green. Key tests: `worktree_branch_set` (real git: worktree with three branches, one
deleted, one created elsewhere, a second worktree), `pull_request_list` (multi-branch query,
default branch dropped, checks, ordering, 25-branch chunking), `features_pr_changes` (stepping,
section states, last choice wins, selection kept across refresh), `pr_status_is_read_only_on_named_events`.

## Part B: visual pass (needs `gh` signed in and a GitHub repository with a worktree that has
pull requests from several branches)

1. Settings, GitHub: turn "Show pull request status" on.
2. In a worktree, open a PR from branch A and merge it, delete A, `git switch -c B`, open a PR,
   close it, `git switch -c C`, open a PR. Open Review changes: three entries, newest first, each
   with its state mark and check mark.
3. Press the second entry: header names it, diff shows its files. Press next: the third. Next is
   disabled on the last, previous on the first. Press "Working tree": back.
4. With the second shown, open a fourth PR from the worktree; within 5 minutes it appears at the
   top and the second stays shown.
5. A worktree with no PRs: "No pull requests from this worktree yet". Sign out of `gh`: the muted
   "Pull requests can't be shown for this worktree", no notice.
6. Another worktree of the project: none of the first worktree's PRs listed.

## Results (close unit, 2026-10-19)

**Part A:** `scripts/gate.sh` exited 0 at `f2af8054` (fmt, clippy core and workspace, `cargo test --workspace` with 583 `test result: ok` lines, no failure, then the script tests). Key suites: `features_pr_changes` 42 passed, `pull_request_list` 17, `worktree_branch_set` 7, `icons` 5. 12 deliberate mutants run, 12 killed (`tdd/verification.md`).

**Part B:** steps 1–6 passed at a display on 2026-10-19 against a **stand-in `gh`** (no real GitHub here); evidence and the caveats are in `visual-pass.md` and `visual-pass/`. One gap: the drawn "Reading pull requests…" line was not seen. The page heading and the diff header repeat the base line (cosmetic, specified, left).
