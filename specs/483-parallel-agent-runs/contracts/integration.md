# Contract: integrating the picked run into the base branch

The pure half lives in `micold_core::runs::integrate`; the I/O half is `GitCli` methods over the
`Git` trait. Local only; nothing is pushed and no remote is contacted (Principle IV).

## I1 — Inputs observed once, under the project's gate

`base_tip = git rev-parse refs/heads/<base>`, `run_tip = git rev-parse refs/heads/<run branch>`,
`checkout = ` the worktree that has `<base>` checked out, from `git worktree list --porcelain`
(`worktree::parse_worktrees`), or `None`.

## I2 — Conflict pre-check (mutates nothing)

`git merge-tree --write-tree <base_tip> <run_tip>`:

- exit 0 → stdout's first line is the merged tree id.
- exit 1 → the conflicted paths are parsed from the "Conflicts" section into
  `Conflicts { files }`; **nothing is written** and the pick refuses (FR-013, SC-006).
- unknown option / any other failure → `GitTooOld` when stderr names `--write-tree`, else
  `Git(stderr)`.

`parse_merge_tree_conflicts(stdout) -> Vec<RelPath>` is a pure, tested function.

## I3 — Fast-forward

When `git merge-base --is-ancestor <base_tip> <run_tip>` is true, the new tip is `run_tip` and the
result is `Integration::FastForward { base_tip: run_tip }`. No commit is created and the run's
branch is untouched (FR-012).

## I4 — Merge commit

Otherwise the new tip is
`git commit-tree <tree> -p <base_tip> -p <run_tip> -m "Merge run <n> of <group name> into <base>"`,
giving `Integration::MergeCommit { commit }`. The message is the only text this feature writes into
the repository; it names the group and the run so the history says where the change came from. The
run's branch is not moved.

## I5 — Moving the base branch

- `checkout == None`: `git update-ref refs/heads/<base> <new tip> <base_tip>`. The expected old value
  makes it a compare-and-swap: a failure means the base branch moved since I1 and the pick refuses
  `BaseMoved`, having changed nothing.
- `checkout == Some(path)`: `git -C <path> merge --no-edit --no-stat <run branch>`. git fast-forwards
  or writes the merge commit itself, and refuses — changing nothing — when local changes or untracked
  files are in the way; its stderr becomes `BaseBusy(message)`. If a merge is left in progress
  (`.git/MERGE_HEAD` present) after a failure, `git -C <path> merge --abort` restores the checkout
  before the refusal is returned.

I2 has already ruled out conflicts, so the checked-out merge cannot stop in a conflicted state in the
ordinary case; the abort is the belt for the race in which the base branch changed between I2 and I5.

## I6 — What never happens

- The run's branch is never moved, rebased, reset or deleted by a pick (FR-012).
- No branch other than `<base>` is written.
- The winner's worktree is kept (Assumptions); the pick deletes nothing. Loser removal is the
  separate, confirmed cleanup, carried out by the existing `WorktreeDelete` (research R13).
- Nothing is pushed, fetched or otherwise sent off-device.

## I7 — Reported outcome

`Integration::FastForward` → "run `<n>` fast-forwarded `<base>`"; `MergeCommit` → "run `<n>` was
merged into `<base>`". Both name the base branch, because the base branch is what changed and the
user may be looking at a different worktree.
