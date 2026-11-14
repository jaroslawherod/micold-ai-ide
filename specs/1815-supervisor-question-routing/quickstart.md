# Quickstart: supervisor question routing (#1815)

No GUI: there is no Part B visual pass. Everything runs from fixtures, no MCP.

## Part A: automated

1. `bash scripts/tests/supervisor.test.sh` (route cases, skill grep cases) and
   `bash scripts/tests/autopilot.test.sh` (`answer.sh --by`, ask.md, resume.md).
2. Expect no `FAIL` line.

## Part A2: walk the two-session batch by hand

```bash
d=$(mktemp -d); L=$d/supervisor.md
for n in 12 14; do cat > $d/esc-$n.json <<JSON
{"category":"1 Product or scope decision","banner":"b","questions":[{"number":1,"question":"Q for $n?","options":[{"label":"A","description":"a"},{"label":"B","description":"b"}],"recommendation":"A","evidence":"e"}]}
JSON
done
scripts/autopilot/supervisor.sh ledger init --ledger "$L" --owner s-0 --cap 3 --source issues 12 14
scripts/autopilot/supervisor.sh route add --ledger "$L" 12 s-12 $d/esc-12.json
scripts/autopilot/supervisor.sh route add --ledger "$L" 14 s-14 $d/esc-14.json
scripts/autopilot/supervisor.sh route next --ledger "$L"        # two questions, issue 12 first
scripts/autopilot/supervisor.sh route answer --ledger "$L" R12.Q1 user "Option A"
scripts/autopilot/supervisor.sh route answer --ledger "$L" R14.Q1 user "Option B"
scripts/autopilot/supervisor.sh route compose --ledger "$L" 12  # session s-12, then 1. Option A (only issue 12's answer)
```

## Part B

Not applicable (no visual change).
