# Cycle Log: Create a Worktree from a GitHub Issue

Append only. Newest last. Every entry's `red` block is the evidence that the test existed and
failed before the implementation.

## Baseline

- suite: `scripts/build-lock.sh cargo test --workspace` -> not yet measured
- commit: `d6c2f33e` (test list planned against it)
- recorded: cycle 0. The design PR changes no code, so the full suite is not run for it. M1 starts
  from a fresh `origin/main` after this PR merges; its first entry below runs the suite on that
  base and records the counts before any red.
