# Phase 6: record unit

The last PR of a run. It records the run in the ledger and changes nothing else.

1. Run `scripts/autopilot/branch-start.sh <last-pr>` with the last PR the orchestrator gave you. On
   any refusal, return `FAILED` with its output. Never discard changes.
2. **Ledger already reads `done`** in a commit on this branch (a resumed run): skip to step 4.
3. In the ledger: set **Phase** to `done`, record the last PR's merge SHA, and fill *Token usage*
   from `mise run autopilot-tokens`. It prints one `### <session>` section per session; copy each
   section's heading, Total row and model table. Commit.
4. Run `mise run test-scripts`. Push and open the PR per
   [../references/pr-and-merge.md](../references/pr-and-merge.md) §3–4, title
   `docs(NNN): record the autopilot run` (a bug appends ` (BUG-<k>)`). Body: the ledger path and
   the token table only, not the milestone template.
5. Return `DONE` with the PR number. Do not wait for CI.

Unlike other units:

- **Do not add this PR to the ledger.** The handoff checks it by number.
- **No review.** The diff is the ledger only.
- **Edit only the ledger.** Any other change the tree needs is a `FAILED` return that names it.
