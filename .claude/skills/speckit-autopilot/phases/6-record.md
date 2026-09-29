# Phase 6: record unit

The last PR of a run. It records the run in the ledger and changes nothing else.

1. `scripts/autopilot/branch-start.sh <previous-pr>`, passing the last merged PR.
2. In the ledger: set **Phase** to `done`, record the last merge SHA the orchestrator gave you, and
   paste the Total row and model table of `mise run autopilot-tokens` into *Token usage*.
3. Run `scripts/tests/*.test.sh`. Commit as `docs(NNN): record the autopilot run` (a bug appends
   ` (BUG-<k>)`), push, and open the PR with the same title per [../references/pr-and-merge.md](../references/pr-and-merge.md) §3–4. Body: the
   ledger path and the token table.
4. Return `DONE` with the PR number. Do not wait for CI.

Edit only the ledger. Any other change the tree needs is a `FAILED` return that names it.
