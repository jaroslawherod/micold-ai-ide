# Data Model: Documentation-Only Changes Skip the Build

**Feature**: `specs/023-docs-only-ci-skip` | **Date**: 2026-08-09

This feature stores nothing and has no runtime state. What it does have is three pieces of data
that must agree with each other, and the whole design is about where each one lives so that they
cannot drift.

## Entity: Documentation Set

The declaration of which paths cannot affect what is built, linted, packaged or tested.

| Field | Value |
|-------|-------|
| Location | `.gitattributes` at the repository root — one file, one place (FR-003) |
| Representation | Lines of `<gitignore-style pattern> micold-docs`, plus `-micold-docs` lines that carve an exception back out |
| Matcher | `git check-attr micold-docs -- <path>` — the only matcher, used by both consumers (FR-025) |
| Verdicts | `set` → documentation; `unset` and `unspecified` → code |

**Contents at the outset**:

```gitattributes
docs/**       micold-docs
specs/**      micold-docs
/*.md         micold-docs
/LICENSE      micold-docs
/*.png        micold-docs
/CHANGELOG.md -micold-docs
```

**Validation rules**:

- Nothing outside this file decides the question. A consumer that hard-codes a path is a bug.
- `unspecified` is the default, so a path is code until someone declares otherwise (FR-002).
- The `-micold-docs` line for `CHANGELOG.md` must come *after* the `/*.md` line that sets it;
  gitattributes takes the last matching line. It exists because the changelog is compiled into the
  binary (research §R6) and is therefore a build input under FR-004.
- Negated patterns (`!`) are not valid in gitattributes; unsetting with `-attr` is the mechanism.

**Consumers**: `scripts/classify-change.sh` (which paths did this change touch) and
`crates/micold-core/tests/documentation_is_not_read.rs` (which paths may no test read).

## Entity: Change Classification

The verdict for one CI run, and the input to every skip decision in the pipeline.

| Field | Type | Meaning |
|-------|------|---------|
| `docs_only` | `true` \| `false` | `true` only when every changed path is documentation (FR-002) |
| `reason` | text | Why — e.g. `no files changed`, `3 non-documentation paths`, `forced by full-ci label`, `base ref unavailable` |
| `offenders` | list of paths | The non-documentation paths that decided a `false` verdict; truncated for display, and the whole point of FR-018's legibility |

**Derivation**:

1. If the pull request carries the `full-ci` label → `false`, reason `forced by full-ci label`
   (FR-021).
2. Otherwise compute the changed set (see the [classifier contract](./contracts/classify-change.md)).
3. Empty changed set → `true` (FR-007).
4. Every path `set` → `true`. Any other path → `false`.
5. Any failure to compute the set → `false`, reason recorded (FR-006).

**State transitions**: none — the verdict is derived per run and never stored. A rebase or a new
push produces a fresh verdict from the new changed set, which is what makes the moving-base-branch
edge case a non-event.

## Entity: Aggregate Gate

The single status the default branch's ruleset demands before a merge, replacing the four per-job
contexts required today. Not data this feature stores — the contract it must not break.

| Field | Value |
|-------|-------|
| Check name | `ci complete` (the job's `name:`, which is what becomes the context) |
| Coverage | Every top-level job in `ci.yml` except itself, via `needs:` |
| Condition | `if: always()` — so it reports on runs where covered jobs skipped *or* failed |
| Verdict | Fails if any covered job's result is `failure` or `cancelled`; `skipped` is satisfied (FR-014) |

**Invariants**:

- The name is fixed by a ruleset that lives outside the repository and that nobody can bypass
  (research §R1). Renaming the job makes every pull request unmergeable, and no pull request can fix
  it.
- It must report a conclusion on every run, under both classifications (FR-012).
- It must never be satisfiable by a run in which a covered job failed. `if: always()` is what
  guarantees this: the default `success()` would *skip* the gate when an upstream job failed, and a
  skipped check reports success — green exactly when the run went red.
- Its `needs:` list must cover every job, asserted by `ci_gate_covers_every_job` rather than by
  review (FR-015).
- `assertions` is advisory (`continue-on-error: true`) and must not be able to fail the gate.

**Why one gate instead of four names**: requiring job names welds the merge gate to the pipeline's
internal shape — no job can be skipped, renamed or re-platformed without breaking merges. One
context that states the run's outcome removes the constraint instead of working around it, which is
what lets a documentation-only run skip the build jobs honestly rather than collapse them onto Linux
and report a pass they did not earn.

## Entity: Ruleset Switch

A one-time, manual change to repository settings — recorded here because it is the only part of this
feature that does not live in the repository.

| Field | Value |
|-------|-------|
| Target | Ruleset `19840981`, rule `required_status_checks` |
| From | Four contexts: `fmt + clippy`, `build + test (<os>)` ×3 |
| To | One context: `ci complete`, `integration_id: 15368` (GitHub Actions) |
| Precondition | A run has already produced a `ci complete` check (FR-016) |
| Rollback | The saved pre-change ruleset, re-applied by `PUT` (research §R13) |

**Rules**: applied by hand with `gh api`; `PUT` replaces the whole ruleset, so the other rules
(`deletion`, `non_fast_forward`, `pull_request`, `required_linear_history`, `code_quality`) must be
carried through unchanged. Keeping `integration_id` pinned to GitHub Actions means no other actor
can post a status that satisfies the gate.

## Entity: Honesty Allowlist

Entries exempting a string literal that resolves to a documentation path but reads nothing.

| Field | Type | Meaning |
|-------|------|---------|
| `file` | repo-relative path | The scanned source file the literal appears in |
| `literal` | text | The exact literal text |
| `reason` | text | Why it is not a read — required, and reviewed on sight |

**Rules**: an entry matches only that exact file-and-literal pair; a stale entry (one that no longer
matches anything) fails the gate, so the list cannot silently accumulate.

**Contents at the outset** (both found by scanning, research §R7):

| file | literal | reason |
|------|---------|--------|
| `crates/micold-core/tests/typeahead_corpus.rs` | `docs/user-guide` | Fixture branch name in the typeahead corpus; branch names look like paths |
| `crates/micold-core/tests/submodule_failure_detail.rs` | `README.md` | Filename created inside a temporary git repository the test builds, not this repository's README |
