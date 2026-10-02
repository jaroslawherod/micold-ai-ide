# Cycle Log: Reporter, Labels and a Description Tooltip in the Issue List

Append only. Newest last. Every entry's `red` block is the evidence that the test existed and
failed before the implementation.

## Baseline

- suite: `scripts/build-lock.sh cargo test --workspace` -> 2629 passed, 0 failed, 2 ignored (profile, observed at `cdc473ab`; not re-run for this design change)
- commit: `d9deabff` (test list planned against it)
- recorded: cycle 0, before any change. The first M1 cycle re-measures on its own base.
