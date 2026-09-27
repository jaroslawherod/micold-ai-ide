# Cycle Log: The start affordance answers for its own directory

Append only. Newest last. Every entry's `red` block is the evidence that the test existed and
failed before the implementation.

## Baseline

- suite: `scripts/build-lock.sh cargo test --workspace` -> not yet measured
- commit: `7cca563c` (test list planned against it)
- recorded: cycle 0. The design PR changes no code, and the full suite was queued behind other
  worktrees' builds on the shared lock, so the run was stopped. M1 starts from a fresh
  `origin/main` after this PR merges. Its first entry below runs the suite on that base and
  records the counts before any red.
