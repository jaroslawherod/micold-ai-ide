# Cycle Log: Pull Request and Check Status for Each Worktree

Append only. Newest last. Every entry's `red` block is the evidence that the test
existed and failed before the implementation.

## Baseline

- suite (fast subset): `mise run test-core` (`scripts/build-lock.sh cargo test -p micold-core --all-targets`)
  -> 1510 passed, 0 failed, 7 ignored (135 test-result lines), exit 0
- suite (workspace): not re-run at planning time (slow, shared build lock). Baseline is CI's green
  `main` at `3d52e83e` (merge-base of this branch; `ci.yml` conclusion `success`); the branch adds
  only `specs/040-worktree-pr-ci-status/` documents on top of it.
- commit: `9501106d`
- recorded: cycle 0, before any change
