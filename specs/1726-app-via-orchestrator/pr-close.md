<!-- claude-projects-attribution: {"github_login":"jaroslawherod"} -->
_Requested by **jaro** · [project thread](https://claude.ai/code/project/chan_01RyHdfvmnDGyHasLedRFLeQ?thread=cmsg_01RyHdfvmnDGyHasLedRFLeQENWuEbL1oca3zdQ8MczbXJ)_

Title: #1726: feat: the desktop app talks only to the orchestrator (aggregated lists, routed ops and streams, starts it when absent)

## Close of specs/1726-app-via-orchestrator

Shipped in #1996 (design), #1999 (M1), #2004 (M2), #2007 (M3), #2012 (M4). Spec status set to Closed; ledger finished.

- Drift records: new `drift-M2.md`; `drift-M3.md` and `drift-M4.md` name every file plan-drift flagged. `scripts/autopilot/plan-drift.sh` is clean for M1-M4.
- `speckit-converge`: no unbuilt behaviour.
- `speckit-tdd-verify`: verdict FAIL, see `tdd/verification.md`. Missing red evidence for squashed M1-M3 and the ignored SC-003 benchmark. Remediation tasks T035-T045 (Phase 9, test-only) are listed in tasks.md and not done here. Added the `bench-routing` mise task.
- Untested by decision (M4): status-cell text of a refused link, pod relay teardown order, no direct daemon socket check. Recorded in the ledger follow-ups.

## Agent review
- Close diff is specs plus one mise task; fresh-context review: see ledger.

## Local gate
`scripts/gate.sh` (fmt, clippy) in the cloud; `cargo test --workspace` left to CI (does not fit the disk here).

Refs #1726

🤖 Generated with [Claude Code](https://claude.com/claude-code)
