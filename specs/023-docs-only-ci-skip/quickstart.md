# Quickstart: Documentation-Only Changes Skip the Build

**Feature**: `specs/023-docs-only-ci-skip` | **Date**: 2026-08-09

How to prove this feature works. Part A runs on your machine and covers everything that can be
checked without GitHub. Part B needs real pull requests, because the thing being verified *is*
GitHub's behaviour — no local runner reproduces how a ruleset treats a check run.

> Part B is not a visual pass and the `visual-pass` skill does not apply: there is nothing to look
> at on a display. It is a live-CI pass, and it needs pull requests against this repository.

## Prerequisites

- The repository, on the feature branch, with `git` available.
- For Part B: `gh` authenticated, and permission to open pull requests against `main`.

## Part A — automated, local

### A1. The declaration classifies the paths it should

```bash
git check-attr micold-docs -- \
  docs/user-guide/settings.md \
  specs/023-docs-only-ci-skip/spec.md \
  README.md CLAUDE.md LICENSE dialog-list.png \
  CHANGELOG.md \
  crates/micold-core/src/lib.rs .github/workflows/ci.yml \
  Cargo.toml scripts/build-lock.sh assets/fonts/Roboto-Regular.ttf
```

**Expected**: `set` for the first six. `unset` for `CHANGELOG.md` — it is compiled into the binary,
so it is a build input, not documentation. `unspecified` for the rest.

### A2. The classifier agrees, over real diffs

```bash
scripts/tests/classify-change.test.sh
```

**Expected**: every case in the [classifier contract](./contracts/classify-change.md#test-cases)
passes — documentation-only, mixed, empty, deleted files, a path with a space, an unresolvable base,
and the `full-ci` override. Each case builds its own throwaway repository, so the suite leaves
nothing behind.

Spot-check it by hand against the current branch:

```bash
scripts/classify-change.sh origin/main HEAD
```

**Expected on this branch**: `docs_only=true` while only `specs/` has changed; `docs_only=false`
from the commit that adds `ci.yml` and the script onward — this feature's own implementation is a
code change, and CI must treat it as one.

### A3. Nothing under test reads project prose

```bash
mise run test-core
```

**Expected**: `documentation_is_not_read` passes. It scans every `.rs` file under `crates/` for a
string literal that resolves to a real repository path carrying `micold-docs`, and fails on any hit
that is not in its allowlist.

To see it bite — this is the check earning its keep, and worth doing once:

```bash
cat >> crates/micold-core/tests/documentation_is_not_read.rs <<'EOF'
#[test]
fn temporary_probe() {
    let _ = std::fs::read_to_string("docs/user-guide/settings.md");
}
EOF
mise run test-core   # expect: FAILS, naming docs/user-guide/settings.md
git checkout -- crates/micold-core/tests/documentation_is_not_read.rs
```

**Expected**: the run fails and the message names both the offending literal and the exemption it
would break (SC-008). Revert the probe.

### A4. Every job is covered by the aggregate gate

```bash
mise run test-core
```

**Expected**: `ci_gate_covers_every_job` passes — every top-level job in `ci.yml` except the gate
itself appears in the gate's `needs:` list. To see it bite, add a job to `ci.yml` and leave it out
of `needs:`: the test must fail naming that job. This is the check that stops the gate rotting into
a green light for a pipeline nobody wired up (FR-015).

### A5. The required context matches the gate's name

```bash
gh api repos/{owner}/{repo}/rulesets/19840981 \
  --jq '.rules[] | select(.type=="required_status_checks")
        | .parameters.required_status_checks[].context' | sort
```

**Expected before the switch**: the four legacy contexts (`fmt + clippy`, `build + test (<os>)` ×3).
**Expected after**: exactly `ci complete`, matching the gate job's `name:` in `ci.yml`. A mismatch
in either direction means pull requests are about to become unmergeable — and the ruleset cannot be
fixed by a pull request. See B0 for the switch itself.

### A6. Governance and the pipeline say the same thing

Read `.specify/memory/constitution.md`'s Development Workflow & Quality Gates section against the
job behaviour matrix in [contracts/required-checks.md](./contracts/required-checks.md).

**Expected**: version `1.6.0`, a Sync Impact Report explaining the narrowing, the TDD gate scoped to
changes able to affect what is built/linted/packaged/tested, and the documentation-only exemption
named — no case the text forbids that the pipeline permits (SC-009).

## Part B — live CI, on real pull requests

Record the outcome of each step in `specs/023-docs-only-ci-skip/ci-pass.md` (run identifiers and
timings), the way the repository records its other manual passes.

### B0. The ruleset switch — order matters

This is the one step that touches settings outside the repository, and the one that can wedge the
default branch if taken early. **Do it in this order.**

1. Push this feature's branch and open its pull request. Its own run is code-affecting, so it
   exercises the full pipeline.
2. Confirm the new context exists on that run — it must print `ci complete`:

   ```bash
   gh pr checks --json name --jq '.[].name' | grep -Fx 'ci complete'
   ```

   Until this prints, **do not touch the ruleset**. Requiring a context nothing produces leaves
   every open pull request pending forever (FR-016, research §R1).
3. Save the current ruleset, then swap the four contexts for the one, exactly as in
   [research §R13](./research.md) — including keeping `integration_id: 15368`, so only GitHub
   Actions can satisfy the gate.
4. Verify with A5. **Expected**: exactly `ci complete`.
5. Confirm the feature's own pull request is now mergeable against the new gate, then merge it.

Keep the saved pre-change ruleset until the feature has been live for a few pull requests; the
rollback command is one `PUT` (research §R13).

### B1. The documentation-only path

Open a pull request that changes only prose — one file under `specs/` is enough.

**Expected**:

1. No compile, lint or test step executes anywhere in the run (US1 scenario 1, FR-008).
2. Zero macOS and zero Windows runners appear in the run's job list. Three jobs run, all Linux:
   `classify`, `docs`, `ci complete` (SC-002).
3. `lint`, `test` (all three legs) and `assertions` are shown as **skipped**, not as successful —
   no check claims a pass for work it did not do (US1 scenario 2, FR-019).
4. `ci complete` reports success; the pull request is mergeable with no override
   (US1 scenario 3, SC-003).
5. Wall clock from first job start to last job finish is under 3 minutes (SC-001).
6. The run summary states the classification and reason without opening a job log
   (FR-018, SC-006).
7. The `classify` job's reason is a path count (e.g. `2 documentation paths`), **not**
   `base ref unavailable`. That reason means the base ref was never fetched: the run falls back
   to the full pipeline, everything still looks correct, and the feature silently never fires.

### B1a. The push-to-`main` path

Land a prose-only commit on `main` (merging the B1 pull request does this).

**Expected**: the same three jobs run — `classify`, `docs`, `ci complete` — and nothing compiles
(US1 scenario 4). The push path resolves its base differently from the pull-request path
(`github.event.before` rather than the base branch), so it has two degenerate cases the
pull-request path does not: an all-zero `before` on a brand-new branch, and a `before` that no
longer exists after a force push. Both must classify as **code-affecting**, not documentation-only
(FR-006). Confirm the reason recorded in the run summary matches.

### B2. The code path is untouched

Open a pull request that changes one `.rs` file and several documentation files.

**Expected**: every job that ran before this feature still runs, on every platform it ran on before
— `lint`, three real `build + test` legs on their own operating systems, `assertions`, `docs` — plus
the two new ones, `classify` and `ci complete`. Diff the job list against a pre-feature run: no job
lost, no platform lost (US2 scenario 1, SC-004). Confirm the ordering-independence cases too, by
pushing the `.rs` change and the documentation change as separate commits in each order
(US2 scenario 2, FR-005).

### B2a. The gate summarises rather than papers over

On a scratch branch, break one platform deliberately — a `compile_error!` behind
`#[cfg(target_os = "windows")]` is enough — and open a pull request.

**Expected**: `build + test (windows-latest)` fails, `ci complete` **fails** naming it, and the pull
request is not mergeable (US2 scenario 5, FR-014). This is the check that proves the gate is a gate.
Do not skip it: an aggregate gate that cannot fail is worse than the four checks it replaced.

Then, on the same scratch branch, push again and cancel the run mid-flight (`gh run cancel`).

**Expected**: `ci complete` reports **failure**, not success. This is what `if: always()` is for —
without it the gate is skipped when the run does not complete, and a skipped check reports success,
so anyone could clear the gate by cancelling. Delete the scratch branch afterwards.

### B3. The documentation gate still bites

On a documentation-only branch, delete one of the documents the `docs` job requires and open a pull
request.

**Expected**: the run fails and the pull request is not mergeable (US3 scenario 1, SC-005). Skipping
the build did not skip the one check whose whole job is to notice this. Restore the file.

### B4. The escape hatch

On the B1 pull request, apply the `full-ci` label.

**Expected**: applying the label starts a fresh run (not a re-run), the full pipeline executes on
all three operating systems, and `ci complete` reports that run's real outcome
(US4 scenarios 1–2, FR-021, FR-022).

### B5. The saving, measured once

```bash
gh pr list --state merged --limit 100 --json number,files \
  --jq '.[] | {n: .number, docs: ([.files[].path] | all(test("^(docs|specs)/|^[^/]+\\.md$")))}'
```

Count the share that would classify as documentation-only, excluding any whose only Markdown change
is `CHANGELOG.md`.

**Expected**: a recorded number, so the saving is known rather than claimed (SC-007).

## Troubleshooting

| Symptom | Cause | Fix |
|---------|-------|-----|
| `ci complete` sits at "Expected — waiting for status to be reported" | The ruleset was switched before a run produced the context, or the gate job was renamed | Roll back the ruleset (research §R13), land the gate, then switch again in B0's order |
| `ci complete` is green but a job failed | The failed job is missing from the gate's `needs:`, or the gate lacks `if: always()` (the default `success()` skips it when upstream fails, and a skipped check reports success) | Fix both; `ci_gate_covers_every_job` should have caught the first — check why it did not |
| A documentation-only pull request runs the full build | Something it touched is not in the documentation set. Check `classify`'s summary for the offending paths | Usually correct behaviour. Widen `.gitattributes` only if the path genuinely cannot affect a build |
| `documentation_is_not_read` fails on a literal that reads nothing | A fixture string that happens to name a real documentation path | Add an allowlist entry with a written reason (see [data-model](./data-model.md#entity-honesty-allowlist)) |
| The `full-ci` label does nothing | `labeled` missing from the workflow's `pull_request` types, or the label was added to a run that was merely re-run | Re-runs replay the original event; the label needs a fresh one |
