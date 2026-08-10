# CI pass record — 023-docs-only-ci-skip

Records for the live-CI procedure in [quickstart.md](./quickstart.md) Part B. Not a visual pass;
the `visual-pass` skill does not apply.

## Baseline (pre-feature) — T001

Run `31385201795` on `main`, commit `cacd9ab`, 2026-08-10T11:49:27Z, conclusion **success**.
This is the comparison SC-004 is measured against: after the feature, a code-affecting run must
still contain every job below, on every platform below.

| Job | Conclusion | Duration |
|-----|-----------|----------|
| `fmt + clippy` | success | 38s |
| `build + test (ubuntu-latest)` | success | 202s |
| `build + test (macos-latest)` | success | 76s |
| `build + test (windows-latest)` | success | 162s |
| `assertion freeze (advisory)` | success | 7s |
| `docs check` | success | 4s |

Six jobs. Wall-clock is bounded by the slowest leg (~202s) plus queueing, on **three** operating
systems. A documentation-only run after this feature should be three Linux jobs and well under
SC-001's three minutes.

## Ruleset baseline — T002

Saved to [`ruleset.before.json`](./ruleset.before.json) — ruleset `19840981`, enforcement `active`,
zero bypass actors. Required contexts at capture time:

- `fmt + clippy`
- `build + test (ubuntu-latest)`
- `build + test (macos-latest)`
- `build + test (windows-latest)`

Other rules that must survive the switch untouched: `deletion`, `non_fast_forward`,
`pull_request`, `required_linear_history`, `code_quality`.

This file is the rollback source (research §R13).

## Part A results (local) — 2026-08-10

| Step | Result |
|------|--------|
| A1 — the declaration classifies the paths it should | ✅ `set` for `docs/`, `specs/`, `README.md`, `CLAUDE.md`, `LICENSE`, `dialog-list.png`; `unset` for `CHANGELOG.md`; `unspecified` for source, workflows, manifests, scripts, assets |
| A2 — the classifier agrees over real diffs | ✅ `classify-change.test.sh` 16/16; `documentation-set.test.sh` all assertions |
| A3 — nothing under test reads project prose | ✅ `documentation_is_not_read` 3/3, allowlisting the two known fixture literals |
| A3 — the gate bites (SC-008) | ✅ a probe reading `docs/user-guide/settings.md` failed the gate naming the path; reverted |
| A4 — every job is covered by the aggregate gate | ✅ `ci_gate_covers_every_job` 3/3 (coverage, `if: always()`, no stale exemptions) |
| A5 — the required context matches the gate's name | ⏳ still the four legacy contexts; pending the ruleset switch (T009) |
| A6 — governance and pipeline agree (SC-009) | ✅ all three CI mandates in the constitution are scoped; none forbids what the pipeline does |

Red-first was observed for each unit rather than assumed:

- `documentation-set.test.sh` failed 11 assertions before `.gitattributes` existed.
- `ci_gate_covers_every_job` failed naming the four pre-existing jobs before the gate existed.
- `classify-change.test.sh` failed 16/16 before the script existed.
- `documentation_is_not_read` failed on exactly the two fixture literals research §R7 predicted,
  before they were allowlisted.

One real defect was caught by its own suite during implementation: the first classifier stored the
NUL-separated diff in a shell variable, and `$(...)` silently drops NUL bytes. Every code case still
reported `docs_only=false` — via the "could not classify" fail-safe rather than by classifying —
which is exactly the failure a suite that only checked verdicts would have passed. Fixed by routing
both NUL streams through files.

## Part B results

### B0 — the ruleset switch (T008–T010) — 2026-08-10

Feature pull request [#134](https://github.com/jaroslawherod/micold-ai-ide/pull/134), run
`31394068048`. Being code-affecting, it exercised the full pipeline and produced the new context
alongside the four legacy ones, which is what made the switch safe to apply while it was open.

| Step | Result |
|------|--------|
| T008 — a run produced `ci complete` | ✅ `gh pr checks` listed it before the ruleset was touched |
| T009 — swap four contexts for one | ✅ `PUT /rulesets/19840981` |
| T010 — verify | ✅ required contexts = `ci complete` alone; `deletion`, `non_fast_forward`, `pull_request`, `required_linear_history`, `required_status_checks`, `code_quality` all intact; enforcement `active`; zero bypass actors; PR `MERGEABLE`/`CLEAN` |

`PUT` is the working method — a previously recorded 404 came from `PATCH`, which these endpoints do
not implement. The body was rebuilt from `ruleset.before.json` and diffed against it before sending;
the only difference was the contexts.

Classification on that run: `docs_only=false`, `reason=9 non-documentation path(s) of 23`. The
reason matters as much as the verdict — `base ref unavailable` would also have produced `false`,
and the full pipeline would have run for the wrong reason, with nothing visibly wrong. That was
analysis finding C1, fixed by fetching the base ref explicitly.

### B1 — the documentation-only path (T018) — 2026-08-10

Pull request [#136](https://github.com/jaroslawherod/micold-ai-ide/pull/136), run `31394694174`,
one file changed under `specs/`. Classification: `docs_only=true`, `reason=1 documentation path(s)`.

| Job | Conclusion |
|-----|-----------|
| `classify change` | success |
| `docs check` | success |
| `ci complete` | success |
| `fmt + clippy` | **skipped** |
| `build + test (${{ matrix.os }})` | **skipped** |
| `assertion freeze (advisory)` | **skipped** |

| Criterion | Target | Actual |
|-----------|--------|--------|
| SC-001 wall clock | < 3 min | **25 s** to settle; 11 s of job time (13:47:34 → 13:47:45) |
| SC-002 macOS/Windows runner minutes | zero | zero — no leg started |
| SC-002 compilation | none | none; no lint, build or test step executed |
| SC-003 mergeable without override | yes | `MERGEABLE` / `CLEAN` |
| FR-019 honest reporting | skipped, not success | all three gated jobs report `skipped` |
| SC-006 legible from the check list | yes | six checks, three plainly skipped |

Against the pre-feature baseline: six jobs across three operating systems, bounded by a ~202 s leg,
becomes three Linux jobs in 11 s.

### R3 confirmed — the design was necessary, not just cleaner

The skipped matrix job reports as **one** check named `build + test (${{ matrix.os }})` — the
un-expanded expression — not as three per-leg checks. Research §R3 could only cite community reports
for this and deliberately refused to bet the default branch on it.

It is now observed here: had the four per-job contexts stayed required and been gated with a
job-level `if:`, `build + test (ubuntu-latest)` and its two siblings would never have been created
on a documentation-only run, and every such pull request would have waited for ever on checks that
no run emits. The aggregate gate was the only design that works on this repository.

The fallback design (required jobs always run, steps conditional, matrix collapsed onto Linux)
would also have worked, at the cost of three green `build + test (<os>)` checks that built nothing.
Both facts are worth keeping: the alternative was viable, and the naive version was not.

### B2 — the code path is untouched (T020, T041) — 2026-08-10

Pull request #134's run `31394068048`, compared against the T001 baseline:

| | Baseline | After the feature |
|---|---|---|
| `fmt + clippy` | ✅ | ✅ |
| `build + test (ubuntu-latest)` | ✅ | ✅ |
| `build + test (macos-latest)` | ✅ | ✅ |
| `build + test (windows-latest)` | ✅ | ✅ |
| `assertion freeze (advisory)` | ✅ | ✅ |
| `docs check` | ✅ | ✅ |
| `classify change` | — | ✅ (new) |
| `ci complete` | — | ✅ (new) |

No job lost, no platform lost (SC-004). The same run is T041's evidence: the full workspace suite
green on Linux, macOS and Windows, with both new gates running on all three.

### B2a — the gate summarises rather than papers over (T023, T024) — 2026-08-10

Scratch pull request #139, a `compile_error!` behind `#[cfg(target_os = "windows")]`.

| Job | Result |
|-----|--------|
| `fmt + clippy` | pass |
| `build + test (ubuntu-latest)` | pass |
| `build + test (macos-latest)` | pass |
| `build + test (windows-latest)` | **fail** |
| `ci complete` | **fail** |

The gate's own output, which is what a reviewer reads:

```
ok       classify     success
ok       lint         success
BLOCKED  test         failure
ok       docs         success
advisory assertions   success (not gated)
```

Pull request state `BLOCKED`. One platform failing is enough, and the advisory job is reported
without gating, exactly as designed (FR-014).

**First attempt was wrong, and worth recording.** The `compile_error!` was initially prepended
above `lib.rs`'s `//!` module docs, where inner doc comments are a syntax error — so it broke all
three platforms instead of one. The run still showed `ci complete: fail`, but it did not prove the
precise claim. Corrected by inserting after the doc block, which is when ubuntu and macOS went
green and windows alone went red.

**Cancellation (T024)**: run `31396007887` cancelled mid-flight. Four jobs `cancelled`,
`ci complete` **failure** — not success, not skipped. This is what `if: always()` buys: without it
the gate inherits `success()`, is skipped when upstream does not succeed, and a skipped check
reports success — so anyone could clear the merge gate by cancelling their own run.

### B3 — the documentation gate still bites (T027) — 2026-08-10

Scratch pull request #138 deleted `docs/user-guide/icons.md` and touched nothing else.
Classification `docs_only=true`. `fmt + clippy`, `build + test` and `assertion freeze` all skipped
— and `docs check` **failed**, `ci complete` **failed**, state `BLOCKED` (SC-005). Skipping the
build did not skip the one gate a prose change can break.

### B4 — the escape hatch (T031) — 2026-08-10

Scratch pull request #140, one file under `specs/`.

1. Before the label: three jobs, everything else skipped, `ci complete` pass (run `31395333073`).
2. Applying `full-ci` started a **fresh** run `31395560991` — a new run, not a re-run.
3. That run executed the entire pipeline on all three operating systems and passed, on a change
   that touches nothing but prose (FR-021, FR-022).

**Note for whoever applies the label next**: `gh pr edit --add-label` failed here with a
Projects-classic GraphQL error and applied nothing, silently enough that it looked like the trigger
had not fired. `gh api -X POST repos/{owner}/{repo}/issues/<n>/labels -f 'labels[]=full-ci'` works.

### T021 — ordering independence: covered by the suite, not re-run live

`classify-change.test.sh` builds real repositories and asserts both orders — a documentation commit
followed by a code commit, and the reverse — both classify as code-affecting. The classifier reaches
them through the same `base...head` three-dot diff that every live run uses, and both live verdicts
(`true` on #136, `false` on #134) exercised that path. A live re-run would cost a full three-platform
pipeline to re-test the same code path, so it was not run. Recorded as a decision, not an oversight.

### T022 — fork pull requests: reasoned, not arranged

The `classify` job only ever *reads* the base repository: `actions/checkout@v4` on the merge ref,
then `git fetch origin <base>`. Both are available to the read-only token a fork's `pull_request`
run receives, so classification behaves the same. If the fetch were ever refused, the script reports
`base ref unavailable` and the change is treated as code-affecting — the full pipeline, which is the
safe direction (FR-006).

Arranging a real fork pull request needs a second account, so this is reasoned rather than observed.
Recorded here so it is a stated position rather than an untested assumption.

### T042 — the rollback file stays

[`ruleset.before.json`](./ruleset.before.json) is kept. It is the only record of what the branch
protection looked like before the switch, it costs nothing, and the ruleset lives outside the
repository where nothing else version-controls it.

## Summary

Every success criterion in the spec is met and observed:

| | |
|---|---|
| SC-001 | 25 s to settle (target: under 3 min) |
| SC-002 | zero macOS/Windows runner minutes, zero compilation |
| SC-003 | merged with no override, no re-run, no branch-protection touch |
| SC-003a | one ruleset edit, rollback recorded, no pull request left unmergeable |
| SC-004 | every baseline job and platform still runs on a code-affecting change |
| SC-005 | a deleted required document fails a documentation-only run |
| SC-006 | the two paths are distinguishable from the check list alone |
| SC-007 | 27 of the last 100 merged pull requests qualify |
| SC-008 | a probe reading documentation fails the build, demonstrated |
| SC-009 | all three constitution CI mandates scoped; no case the text forbids and the pipeline permits |
