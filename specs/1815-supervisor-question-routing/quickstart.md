# Quickstart: supervisor question routing (#1815)

No GUI: there is no Part B visual pass. Everything runs from fixtures, no MCP.

## Part A: automated

1. `bash scripts/tests/supervisor.test.sh` (route cases, skill grep cases) and
   `bash scripts/tests/autopilot.test.sh` (`answer.sh --by`, ask.md, resume.md).
2. Expect no `FAIL` line.

## Part A2: walk the two-session batch by hand

```bash
L=$(mktemp -d)/supervisor.md
scripts/autopilot/supervisor.sh ledger init --ledger "$L" --owner s-0 --cap 3 --source issues 12 14
scripts/autopilot/supervisor.sh route add --ledger "$L" 12 s-12 fixtures/esc-12.json
scripts/autopilot/supervisor.sh route add --ledger "$L" 14 s-14 fixtures/esc-14.json
scripts/autopilot/supervisor.sh route next --ledger "$L"        # two questions, issue 12 first
scripts/autopilot/supervisor.sh route answer --ledger "$L" R12.Q1 user "Option A"
scripts/autopilot/supervisor.sh route answer --ledger "$L" R14.Q1 user "Option B"
scripts/autopilot/supervisor.sh route compose --ledger "$L" 12  # 1. Option A (only issue 12's answer)
```
(`fixtures/` is the test's generated escalation JSON; the test prints its path on failure.)

## Part B

Not applicable (no visual change).
