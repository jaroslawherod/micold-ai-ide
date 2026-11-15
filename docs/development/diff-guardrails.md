# Diff guardrails: coverage and mutation testing on changed lines

Two checks look only at the lines a pull request changed (#1931). They answer different questions:

| Check | Question | CI job | Merge-blocking? |
|-------|----------|--------|-----------------|
| Diff coverage | Do the tests run the changed lines? | `diff coverage` | **Yes** |
| Mutants on the diff | Would the tests notice if the changed lines were wrong? | `mutants on the diff` | No, advisory |

Both are Linux-only, pull-request-only, and skipped on a documentation-only change.

## Scope

`scripts/guardrails-scope.conf` is the one place that says which files count: a path-glob list per
language (`**` crosses directories, `*` does not), plus the coverage threshold (80%).

- `[rust]` today: `crates/micold-daemon/src/**` and `crates/micold-core/src/orchestrator/**`.
- `micold-client` and `micold-material` (the desktop app) are being sunset and are left out.
- `[web]` is empty. When the web client exists, list its sources there and point
  `scripts/diff-coverage.py --lang web --lcov <file>` at its lcov file (any tool that writes lcov).
  Nothing else in the scripts changes.

## Diff coverage

The `test` job runs the workspace suite once, through `cargo llvm-cov nextest`, which also writes
`lcov.info`. The doctests and the release frame-budget step stay outside the instrumented run. The
`diff-coverage` job downloads that file and runs `scripts/diff-coverage.py`, which takes the lines
added or changed against the merge base, keeps the in-scope files, and compares the executable ones
(those with a line record in the lcov file) with their hit counts. It passes at the threshold or
above, and when no executable line in scope changed. The parsing lives in
`scripts/autopilot/lcov_diff.py`, shared with `review-facts.py`, which shows the same uncovered
lines to the autopilot reviewers (`review-facts.py --lcov`).

## Mutants on the diff

`scripts/mutants-diff.sh` runs `cargo mutants --in-diff` on the diff, limited to the `[rust]` globs,
testing each mutant with its own crate's tests under nextest. The job is capped at 30 minutes. A
mutant the tests do not catch is listed in the job summary and in `missed.txt`.

## Output

The job summary plus a small artifact (`diff-coverage`: lcov and the report; `mutants-diff`: the
missed and timed-out mutants). Never a pull request comment.

## Blocking or advisory: one setting each

`ci-complete` reads two switches at the top of `.github/workflows/ci.yml`:

```yaml
GUARDRAIL_COVERAGE_BLOCK: "true"   # diff coverage red blocks the merge
GUARDRAIL_MUTANTS_BLOCK: "false"   # mutants red is shown, never blocks
```

Flip the value to change it. Both jobs are in `ci-complete`'s `needs`, so
`crates/micold-core/tests/ci_gate_covers_every_job.rs` stays satisfied either way; an advisory one
prints `advisory` instead of `ok` or `BLOCKED` in the gate step.

## Locally

Not part of `mise run gate`. Both are opt-in and need `cargo install cargo-llvm-cov cargo-mutants
--locked` (and `rustup component add llvm-tools-preview`):

```sh
mise run coverage-diff   # core + daemon suites under llvm-cov, then the changed-line check
mise run mutants-diff    # minutes; BASE=<ref> overrides origin/main for both
```

`.specify/memory/tdd-profile.md` records both commands, so `speckit-tdd-verify` uses them.
