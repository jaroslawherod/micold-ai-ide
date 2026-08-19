# T062 — the three-OS matrix

**Date**: 2026-08-19
**Run**: [32263815467](https://github.com/jaroslawherod/micold-ai-ide/actions/runs/32263815467),
on [PR #200](https://github.com/jaroslawherod/micold-ai-ide/pull/200) — the branch carrying 016's
T063 record and all three BUG-003 fixes, rebased onto `main` so the matrix ran against what would
actually merge.
**Verdict**: **PASS** on all three platforms, and on every other job in the pipeline.

| Job | Result | Time |
|---|---|---|
| `build + test (ubuntu-latest)` | pass | 6m14s |
| `build + test (macos-latest)` | pass | 1m29s |
| `build + test (windows-latest)` | pass | 3m5s |
| `fmt + clippy` | pass | 43s |
| `classify change` | pass (code-affecting) | 4s |
| `docs check` | pass | 5s |
| `assertion freeze (advisory)` | pass | 6s |
| `ci complete` | pass | 4s |

`classify change` reported the run as code-affecting, so nothing was skipped by feature 023's
docs-only path — the figures above are a full pipeline, not a fast one.

## What the two non-Linux platforms did **not** run

The wall-clock times give this away and it should be written down rather than inferred: macOS
finished in a quarter of Linux's time because it ran a quarter of the work. `Test (full workspace)`
is guarded `if: runner.os == 'Linux'`, so on macOS and Windows the run was:

- `cargo test -p micold-core --all-targets` — the render-free core, Principle I's FR-040
- `cargo build --workspace` — the client GUI binary and the daemon **compile**
- the eleven named component-library and showcase gates, which read source text and open no window

Confirmed from the run's own step data: both non-Linux jobs list `Test (full workspace)` as
skipped, and Linux lists nothing skipped.

So the honest claim T062 can carry is **"builds everywhere, and the render-free core plus the
source-reading gates pass everywhere; the client and daemon suites pass on Linux."** That is what
`ci.yml` is designed to assert — the guard is deliberate and its comment says why the gates in
particular are worth running on all three (path handling, `\` vs `/` in the scanners' display
keys). It is not what "tests pass on Linux, macOS and Windows" sounds like at a glance, which is
the reason for this section.

## What this run covers that a Linux-only one would not

The branch's own changes are in the render-free core (`naming.rs`, `worktree.rs`, `anatomy.rs`) and
in the client's material layer. The core changes run their full suite on all three platforms. The
client changes are compiled on all three and their layout assertions run on Linux only — they are
`micold-client` integration and in-crate tests, which the guard excludes.

Nothing on this branch touches path *rendering*, which is the difference the matrix comment names.
`CreateError::DuplicateDir` now carries a `PathBuf` and `explain_directory_taken` formats it through
`folder_name`, which is `Path::file_name` — separator-correct on Windows by construction, and its
fallback is `Path::display`. Its tests are in `micold-core`, so they ran on all three.
