# Research: Documentation-Only Changes Skip the Build

**Feature**: `specs/023-docs-only-ci-skip` | **Date**: 2026-08-09

Everything below was checked against the repository, the live branch ruleset, or GitHub's own
documentation. Where a claim could not be verified, the design avoids depending on it.

## R1. How the default branch's rules actually gate a merge

**Finding**: The default branch is governed by a repository ruleset (id `19840981`, enforcement
`active`, `bypass_actors: []`, `current_user_can_bypass: "never"`). It requires four status checks
by exact context name, all from GitHub Actions (`integration_id: 15368`):

- `fmt + clippy`
- `build + test (ubuntu-latest)`
- `build + test (macos-latest)`
- `build + test (windows-latest)`

**Consequence**: Nobody can merge past a missing check — not even the repository owner. The
ruleset also lives outside the repository, so no pull request can change it. Any design that stops
one of those four names from reporting turns every documentation-only pull request into a
permanently unmergeable one. This is what makes FR-012 – FR-017 the hard part of the feature, not
the skipping.

**Decision (revised after the owner authorised a ruleset edit)**: replace the four per-job contexts
with a single aggregate gate. Requiring job names is the root cause of every awkward constraint
below — it welds the merge gate to the pipeline's internal shape, so the pipeline cannot skip a job
without lying about it. One required context that summarises the run removes the constraint
entirely rather than working around it. See §R4.

**Sequencing (FR-016)**: the switch must be applied only after a run has already produced the
gate's check name. A required context with no producer is a permanently pending check, which is
this very section's failure mode. Order: land the workflow → observe the gate reporting on the
feature's own pull request → swap the ruleset → merge.

## R2. `paths-ignore` at the workflow level is the obvious answer and it is wrong

**Decision**: Do **not** use workflow-level `paths` / `paths-ignore` filters.

**Rationale**: GitHub's own troubleshooting guidance for required status checks states that when a
workflow is skipped by path filtering, "associated checks stay in a 'Pending' state and block
merging", and advises against requiring workflows that can be skipped. A `paths-ignore` on `ci.yml`
would therefore produce exactly the opposite of the feature's goal: documentation-only pull
requests would be *permanently* unmergeable rather than quickly green.

**Alternatives considered**:

- *Twin workflow*: a second workflow with complementary `paths:` filters and jobs carrying the same
  four names, which no-op. This is GitHub's documented workaround and it does work. Rejected
  because it duplicates the four check names across two files, so the names can drift silently, and
  a reader has to hold two workflows in their head to know what runs when.

## R3. Skipped jobs count as success — but skipped *matrix legs* may not appear at all

**Finding (verified)**: GitHub documents that a job skipped by an `if:` conditional reports
`Success` and does not block merging, even when required. This is the standard basis for the
"filter job + `if:` on every job" pattern.

**Finding (was unverified; now CONFIRMED on this repository — see below)**: When a job that carries
a `strategy.matrix` is skipped wholesale by a job-level `if:`, the individual matrix legs do not
appear as check runs at all. An absent required check is pending, which is R1's blocking case. At
design time we could not confirm whether this applied to a *static* matrix — the community reports
involve matrices computed from an upstream job's output — and confirming it would have meant
merging a change that might wedge the default branch to find out.

**Confirmed 2026-08-10** by the first documentation-only run (pull request #136, run
`31394694174`): the skipped matrix reports as a single check named
`build + test (${{ matrix.os }})` — the un-expanded expression — and no per-leg check is created.
Had the four per-job contexts stayed required, `build + test (ubuntu-latest)` and its siblings would
never have existed on that run, and the pull request could never have merged. The caution was
correct, and the aggregate gate turned out to be the only design that works here rather than merely
the tidier one. Recorded in [ci-pass.md](./ci-pass.md).

**Why it stops mattering**: under the aggregate gate (§R4) none of those four names is required any
more, so whether a skipped matrix job creates its per-leg check runs is no longer load-bearing. The
unknown was the *reason* to reject the clean design; removing the requirement removes the unknown
rather than resolving it. What the design still relies on is the documented half — a job skipped by
`if:` reports success — and only for `assertions`, whose result the gate reads.

## R4. The chosen shape: one required gate, everything under it free to skip

**Decision**: `ci.yml` gains a `classify` job at the front and a `ci complete` job at the back.
`ci complete` becomes the sole required status check.

```text
classify ──┬─> lint        (if: not docs-only) ──┐
           ├─> test x3     (if: not docs-only) ──┤
           ├─> assertions  (if: not docs-only) ──┼─> ci complete   [REQUIRED]
           └─> docs        (always)            ──┘
```

- `lint`, `test` and `assertions` take an ordinary job-level `if:` and genuinely skip on a
  documentation-only change. Their steps are untouched — no per-step guards, no conditional
  `runs-on`, no matrix collapsed onto Linux.
- `docs` keeps no condition: it must still be able to fail a documentation-only change (FR-010,
  User Story 3).
- `ci complete` runs with `if: always()` and fails when any covered job's result is `failure` or
  `cancelled`, treating `skipped` as satisfied (FR-014). `always()` is what makes it report on runs
  where upstream jobs skipped *or* failed; without it the gate would itself be skipped and report a
  green that means nothing.

**Rationale**: the merge gate now states the outcome the repository actually cares about — did CI
conclude successfully — instead of enumerating the pipeline's internals. Every awkward constraint in
the previous design was a consequence of that enumeration:

| Required job names (previous) | One aggregate gate (chosen) |
|---|---|
| Required jobs must always run; only their steps may be conditional | Jobs skip outright |
| Three `build + test (<os>)` legs collapse onto Linux and report green having built nothing | Legs are genuinely skipped, and shown as skipped (FR-019) |
| Five Linux jobs start on a documentation-only run | Three: `classify`, `docs`, `ci complete` |
| Rests on §R3's unverified matrix-skip behaviour | Rests on nothing unverified |
| A green check that built nothing needs mitigating prose | Nothing to mitigate |

**Cost**: one short job at the end of every run; the one-time ruleset edit (§R1's sequencing); and
one new failure mode — a job added later and left out of the gate's `needs:` stops blocking merges,
silently. That is the classic way an aggregate gate rots, and it is why FR-015 exists. See §R12.

**Alternative considered**: the previous design — required jobs always run, steps conditional,
matrix collapsed onto Linux — is what this feature would have been had the ruleset been immovable.
Strictly worse on every axis except that it needs no ruleset edit, which is the axis that stopped
mattering when the owner authorised one.

## R5. Declaring the documentation set once, and matching it the same way twice

**Decision**: Declare the set as git *attributes* in a new `.gitattributes` at the repository root,
using a single custom attribute (`micold-docs`). Both the CI classifier and the Rust honesty gate
resolve paths through `git check-attr micold-docs`.

**Rationale**: This gives one declaration, read by one matcher, from two places — which is exactly
what FR-022 asks for and the only thing that stops the two lists drifting apart. It also comes with
gitignore-style glob semantics (`docs/**`, `/*.md`) for free, rather than a hand-rolled matcher
that has to be tested on its own. Verified locally on this repository, including the two cases that
usually break a hand-rolled matcher:

- a path that no longer exists (a deleted file) still classifies correctly, because `check-attr` is
  pure pattern matching and never touches the working tree;
- a later line can *unset* the attribute for a single path, which R6 needs.

Only `set` counts as documentation; both `unset` and `unspecified` mean code, which gives FR-002's
"anything not declared is code" for free.

**Alternatives considered**:

- *`git check-ignore` with `core.excludesFile` pointed at a patterns file*: also works, and was
  tested. Rejected because the repository's real `.gitignore` is consulted at the same time, so an
  ignore rule like `*.pdb` or `*.swp` could silently classify a changed file as documentation.
- *A plain patterns file plus a matcher in shell and a second matcher in Rust*: two matchers is two
  chances to disagree, which is the failure this feature is supposed to make impossible.
- *`dorny/paths-filter` or another marketplace action*: a new third-party dependency on the merge
  path, against the constitution's dependency-vetting constraint, for logic that is four lines of
  git.

## R6. `CHANGELOG.md` is not documentation — the gate found its first real case before it existed

**Finding**: `crates/micold-core/src/metadata.rs:15` contains
`pub const CHANGELOG: &str = include_str!("../../../CHANGELOG.md");` — the changelog is compiled
into the binary so the app can show a "what's new" view offline (Principle IV).

**Consequence**: a change to `CHANGELOG.md` changes the built artifact. Under FR-004 it is a build
input, not documentation, and it must take the full pipeline. The spec's original assumption said
the opposite; it has been corrected.

**Method note**: this was found by scanning every string literal under `crates/` for one that
resolves to a real path in the proposed documentation set — which is precisely what FR-021's
honesty gate does. The gate earned its place before it was written.

## R7. What the honesty gate must tolerate

**Finding**: the same scan turned up two literals that resolve to real documentation-set paths but
read nothing:

| Literal | Where | Why it is benign |
|---------|-------|------------------|
| `"docs/user-guide"` | `crates/micold-core/tests/typeahead_corpus.rs` | A fixture branch name in the typeahead corpus. It looks like a path because branch names do. |
| `"README.md"` | `crates/micold-core/tests/submodule_failure_detail.rs` | A filename created inside a temporary git repository the test builds, not the repository's own README. |

**Decision**: the gate fails on any string literal in scanned sources that resolves to an existing
repository path carrying `micold-docs`, unless the literal's file+text pair appears in a small
in-test allowlist that requires a written reason. Two entries at the outset, both above.

**Rationale**: an existence check alone has false positives (the two above); a "does it look like a
filesystem read" heuristic alone has false negatives (a path built up from parts). Existence plus a
justified allowlist is the combination this repository already uses for its source-scanning gates,
and it fails loud rather than quiet.

**Alternatives considered**: parsing Rust to find real `include_str!` / `fs::` call sites. More
precise, far more machinery, and it still would not catch a path assembled at runtime.

## R8. Determining the changed set

**Decision**: `git diff --name-only -z` against the merge base, computed in the `classify` job from
a full-history checkout.

- pull request: `git diff --name-only -z "origin/${{ github.base_ref }}...HEAD"` (three dots — the
  merge base), which is the pull request's whole changed set and satisfies FR-005 regardless of
  which commit came last.
- push: `github.event.before...github.event.after`, falling back to code-affecting when `before` is
  all zeroes (a new branch) or no longer exists (a force push).
- any failure to determine the set at all: code-affecting (FR-006).

**Rationale**: git sees the whole diff at once, so the spec's "very large pull request" edge case
disappears rather than being handled — there is no page size to exceed. It also needs no token
permissions, which is what makes it work unchanged for pull requests from forks.

`-z` plus `core.quotePath=false` so a path containing a space or a quote is passed through intact.

## R9. The escape hatch

**Decision**: a `full-ci` label on the pull request, with `labeled` added to the workflow's
`pull_request` trigger types so that applying it starts a fresh run.

**Rationale**: re-running an existing run replays the original event payload, so a label added
afterwards would not be seen — the fresh event is the point. Removing the label is deliberately
*not* a trigger: the next push reclassifies anyway, and there is no cost to a full run that was
asked for.

**Cost**: adding any label to any pull request now starts a run, including a full one on a code
pull request. In a repository that does not otherwise use labels, this is close to free.

## R10. Where the honesty gate runs

**Decision**: `crates/micold-core/tests/documentation_is_not_read.rs`, picked up by the existing
`cargo test -p micold-core --all-targets` step, which already runs on all three platforms as the
first test step of every matrix leg.

**Rationale**: no CI edit is needed to run it — unlike the client's showcase gates, which CI names
one by one, the core's step is `--all-targets` and takes new tests automatically. Running it on all
three platforms also exercises the `git check-attr` call on Windows, where path separators are the
usual place this kind of gate breaks.

## R11. Constitution amendment

**Decision**: amend the Development Workflow & Quality Gates section's TDD gate so that the
full-suite requirement binds every change able to affect what is built, linted, packaged or tested,
and names the documentation-only exemption; bump 1.5.0 → **1.6.0** (MINOR) with a Sync Impact
Report, per the project's own precedent.

**Rationale**: the repository has twice treated a narrowly-scoped, explicitly-named exemption as
MINOR rather than PATCH — Principle III's Default-session exception (1.3.0) and Principle I's
showcase-glue path (1.5.0) — with the recorded reasoning that a gate whose reach can be narrowed by
an edit filed as "wording" is a gate that erodes quietly. This narrows a gate the same way.

The 1.5.0 report also set the precedent this feature's FR-021 follows: when the exemption's
precondition is checkable, it ships with a check (`showcase_glue.rs`) rather than resting on
review. `documentation_is_not_read.rs` is that check here.

**Templates**: `.specify/templates/plan-template.md`'s Principle VI line says "CI covers all
three" — still true for every code change, so no template edit is required.

## R12. The aggregate gate's own failure mode, and the check for it

**Finding**: an aggregate gate is only as good as its `needs:` list. Add a job and forget to list
it, and that job's failures quietly stop blocking merges — the gate goes green beside it. Nothing in
GitHub Actions notices; the workflow is perfectly valid.

**Decision**: a source-scanning gate, `crates/micold-core/tests/ci_gate_covers_every_job.rs`, reads
`.github/workflows/ci.yml`, collects the top-level job ids, and asserts every one of them except the
gate itself appears in the gate's `needs:` list. Fails naming the uncovered job (FR-015).

**Rationale**: this is the same shape as the repository's existing source-scanning gates
(`showcase_glue.rs`, `material_boundary.rs`) — read the text, assert a structural property — and it
needs no YAML parser, so no new dependency. Job ids sit at a known indent and `needs:` is a flat
list; a stricter parse would buy nothing here.

**Alternatives considered**: a real YAML parse (`serde_yaml`) — a new dependency on the merge path
for a two-line grep. Review discipline — precisely what FR-015 says is not enough, on the same
reasoning the constitution used when it required `showcase_glue.rs` rather than trusting reviewers
to police a widened exemption.

## R13. Performing the ruleset switch

**Decision**: one `gh api` call, applied by hand after the gate is observed reporting, with the
before-state saved as the rollback (FR-016).

```bash
# 0. Save the current ruleset. This file IS the rollback.
gh api repos/{owner}/{repo}/rulesets/19840981 > ruleset.before.json

# 1. Confirm a run has already produced the new context. Must print "ci complete".
gh pr checks --json name --jq '.[].name' | grep -Fx 'ci complete'

# 2. Swap the four contexts for the one. PUT replaces the ruleset, so send the whole writable body.
jq '{name, target, enforcement, conditions, bypass_actors,
     rules: (.rules | map(if .type == "required_status_checks"
                          then .parameters.required_status_checks =
                               [{"context": "ci complete", "integration_id": 15368}]
                          else . end))}' ruleset.before.json > ruleset.after.json
gh api -X PUT repos/{owner}/{repo}/rulesets/19840981 --input ruleset.after.json

# 3. Verify.
gh api repos/{owner}/{repo}/rulesets/19840981 \
  --jq '.rules[] | select(.type=="required_status_checks")
        | .parameters.required_status_checks[].context'
```

**Rollback**:

```bash
jq '{name, target, enforcement, conditions, bypass_actors, rules}' ruleset.before.json \
  | gh api -X PUT repos/{owner}/{repo}/rulesets/19840981 --input -
```

**Notes**

- `PUT` replaces the ruleset, so the body must carry every rule, not just the changed one — hence
  reconstructing from the saved state rather than hand-writing a body. The other four rules
  (`deletion`, `non_fast_forward`, `pull_request`, `required_linear_history`, `code_quality`) must
  survive untouched.
- `integration_id: 15368` is GitHub Actions. Keeping it means only a check produced by Actions can
  satisfy the context — a status posted by anything else cannot spoof the gate.
- Step 1 is not optional. Requiring a context that no run has produced leaves every open pull
  request pending forever, which is §R1's failure mode arriving by the front door.
