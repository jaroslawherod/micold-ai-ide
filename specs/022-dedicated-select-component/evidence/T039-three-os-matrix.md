# T039 — the three-platform matrix

**Date**: 2026-08-19
**Run**: [32291591383](https://github.com/jaroslawherod/micold-ai-ide/actions/runs/32291591383), on
[PR #206](https://github.com/jaroslawherod/micold-ai-ide/pull/206) — a branch whose base is `main`,
which carries all of feature 022.
**Verdict**: **PASS** on all three platforms.

| Job | Result | Time |
|---|---|---|
| `build + test (ubuntu-latest)` | pass | 3m50s |
| `build + test (macos-latest)` | pass | 1m15s |
| `build + test (windows-latest)` | pass | 3m10s |
| `fmt + clippy` | pass | 39s |
| `classify change` | pass (code-affecting) | 8s |
| `docs check` | pass | 5s |
| `assertion freeze (advisory)` | pass | 10s |
| `ci complete` | pass | 2s |

The run was classified code-affecting, so nothing was skipped by feature 023's docs-only path.

## Why this run answers T039

The task asks for confirmation via `.github/workflows/ci.yml`'s matrix, and the matrix only runs on
a pull request. 022 is merged, so there is no 022 branch left to open one from — but every PR
against `main` runs the whole matrix over a tree that *contains* 022. This is such a run, from the
same day, and it exercised `select_anatomy`, `picker_parity`, `picker_motion`, `picker_press` and
`picker_visibility` along with everything else.

The task's own note gave the reason a local run is weaker evidence here: every worktree on this
machine shares one `CARGO_TARGET_DIR`, and cargo gives the same `-C metadata` hash to the same crate
built from different worktrees, so they can overwrite each other's test binaries. CI on a clean
checkout has none of that, which is exactly why the task asked for CI rather than for a local pass.

## What the two non-Linux platforms do **not** run

The same caveat 016's T062 record carries, and it is why macOS finished in a third of Linux's time.
`Test (full workspace)` is guarded `if: runner.os == 'Linux'`, so on macOS and Windows the run was:

- `cargo test -p micold-core --all-targets` — the render-free core
- `cargo build --workspace` — the client and the daemon **compile**
- the eleven named component-library and showcase gates, which read source text and open no window

**This matters more for 022 than for most features**, and the record would be misleading without
saying so: 022's own gates — `select_anatomy`, `picker_parity`, `picker_motion`, `picker_press`,
`picker_visibility` — are in-crate `micold-client` tests, so they are covered by
`cargo test --workspace` on **Linux only**. What the other two platforms establish for this feature
is that it compiles there.

So the honest claim is: **022 builds on all three platforms, and its own test suite passes on
Linux.** That is what `ci.yml` is built to assert; it is not what "passes on Linux, macOS and
Windows" sounds like, which is why this section exists.
