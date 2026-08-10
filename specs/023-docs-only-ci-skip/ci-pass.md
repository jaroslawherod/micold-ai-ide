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

### B1 — the documentation-only path (T018)

_This very pull request. Results recorded on merge._
