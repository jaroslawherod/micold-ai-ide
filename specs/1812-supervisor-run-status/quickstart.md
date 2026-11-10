# Quickstart: validating supervisor run status

Prerequisites: `jq`, `git`, `gh` authenticated (steps 1-2 only for the live check).

1. Tests: `scripts/tests/run-status.test.sh` then
   `scripts/tests/autopilot.test.sh`. Expect every case `ok`; seven states each in text and JSON.
2. Live: in this worktree, `scripts/autopilot/run-status.sh --json | jq .` — expect `state` of
   `running` or `waiting-ci`, `supervised` false, `prs` filled from GitHub. Time it (`time`): under
   5 s (SC-005).
3. Read-only: `git status --porcelain; sha256sum specs/*/autopilot.md` before and after the call:
   identical.
4. Escalation: in a scratch clone set `AUTOPILOT_SUPERVISED=1`, write an *Open escalation* in the
   contract's format, run status: `state` is `asking` with question, options, recommendation
   separate. Run `answer.sh <ledger> "<label>"`: *Decisions* has the row, section reads `None.`.
5. Full gate before pushing: `mise run gate`.

No visual pass: there is no UI.
