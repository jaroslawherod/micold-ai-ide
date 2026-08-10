# Contract: the aggregate gate, and what CI reports under both classifications

The default branch is governed by ruleset `19840981` — enforcement `active`, no bypass actors,
`current_user_can_bypass: never`. Today it requires four status checks by job name. This feature
replaces them with one.

## Before and after

| | Required contexts |
|---|---|
| Today | `fmt + clippy`, `build + test (ubuntu-latest)`, `build + test (macos-latest)`, `build + test (windows-latest)` |
| After | `ci complete` |

The switch is a manual `gh api` call — the ruleset lives outside the repository and no pull request
can carry it. Command, verification and rollback are in [research §R13](../research.md). **It must
be applied only after a run has produced `ci complete`**, or every open pull request blocks on a
context nothing emits.

## The gate

```yaml
ci-complete:
  name: ci complete
  needs: [classify, lint, test, assertions, docs]
  if: always()
  runs-on: ubuntu-latest
```

| Covered job result | Gate verdict |
|--------------------|--------------|
| `success` | satisfied |
| `skipped` | satisfied — a job that did not need to run is not a failure (FR-014) |
| `failure` | **fail**, naming the job |
| `cancelled` | **fail**, naming the job |

**`if: always()` is load-bearing.** Without it the gate inherits the default `success()`, which
means it would be *skipped* whenever an upstream job failed — and a skipped check reports success.
The gate would go green precisely when the run went red.

**`assertions` is advisory** (`continue-on-error: true`), so its result is not allowed to fail the
gate. The gate reads the other four.

**Coverage is checked, not trusted.** `crates/micold-core/tests/ci_gate_covers_every_job.rs` asserts
every top-level job in `ci.yml` except the gate appears in that `needs:` list. A job added later and
forgotten is otherwise a job whose failures silently stop blocking merges (FR-015, research §R12).

## Job behaviour matrix

| Job | Required | Code-affecting | Documentation-only |
|-----|----------|----------------|--------------------|
| `classify` | no | Runs; computes the verdict | Runs; computes the verdict |
| `lint` | no | Runs fully | **Skipped** |
| `test` (×3) | no | Runs fully, each leg on its own OS | **Skipped** — no macOS or Windows runner starts |
| `assertions` | no | Runs (advisory) | **Skipped** |
| `docs` | no | Runs fully | **Runs fully** — must still be able to fail (FR-010) |
| `ci complete` | **yes** | Reports the run's outcome | Reports the run's outcome |

Three jobs run on a documentation-only change: `classify`, `docs`, `ci complete`. All Linux.

## Honesty

The aggregate gate makes FR-019 fall out of the design rather than needing mitigation: on a
documentation-only run the build jobs are *shown as skipped*, because they were. No check reports
success for work it did not do.

What still needs saying explicitly:

- `classify` writes the verdict, the reason, and any offending paths to the run summary, so the run
  says at a glance which path it took (FR-018).
- `docs/development/ci-pipeline.md` explains that `ci complete` green on a documentation-only run
  means "nothing needed building", and where to look to confirm that (FR-020).

## Triggers

```yaml
on:
  push:
    branches: [main]
  pull_request:
    types: [opened, synchronize, reopened, labeled]
```

`labeled` is added so applying the `full-ci` label starts a fresh run — re-running an existing run
replays the original event payload and would not see a label added afterwards (research §R9).

## Change control

Renaming the `ci complete` job breaks the merge gate for every pull request, and no pull request can
fix it. Before changing any job name, check the live ruleset:

```bash
gh api repos/{owner}/{repo}/rulesets/19840981 \
  --jq '.rules[] | select(.type=="required_status_checks")
        | .parameters.required_status_checks[].context'
```

Adding, removing or renaming any *other* job is now safe — that is what the aggregate gate bought —
provided `ci_gate_covers_every_job` stays green.
