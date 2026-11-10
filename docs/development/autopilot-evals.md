# Autopilot evals

`scripts/autopilot/eval.py` measures past autopilot runs, so an edit to
`.claude/skills/speckit-autopilot/**`, `.claude/agents/**` or `CLAUDE.md` can be compared with the
runs before it.

```bash
scripts/autopilot/eval.py            # metrics per corpus run, the mean, and the delta to the baseline
scripts/autopilot/eval.py baseline   # write the current mean to evals/autopilot/baseline.json
scripts/autopilot/eval.py changed    # the same, only when an agent-shaping file differs from origin/main
scripts/autopilot/eval.py add <pr> [--ledger FILE] [--tokens N] [--note TEXT]
```

The corpus is `evals/autopilot/corpus.json`. Metrics: `review_rounds` and `first_pass` from the
ledger's *Review rounds* table, `ci_red` (failed workflow runs on the PR's branch) from `gh`, and
`tokens` (`cost_eq` from `scripts/autopilot-tokens.py --json`) as the case recorded it. A missing
source prints `-`. Ledgers are gitignored, so `add --ledger` keeps a snapshot under
`evals/autopilot/ledgers/`. Add every failed or escalated run as a regression case. The command
exits 1 when a mean is worse than the baseline. Replaying runs is a later milestone.
