# Phase 5: close unit

1. Run in this order: `speckit-converge` → `speckit-tdd-verify` → `speckit-docguard-guard`.
2. **Unbuilt behaviour** found by any of the three: cut it into a new milestone per
   [../references/milestones.md](../references/milestones.md), add it to the ledger, commit, and
   return `DONE` with `NEW MILESTONES: M<n>…` (do not push; they ship in the first new milestone's
   PR). The orchestrator runs them, then a new close unit.
3. Otherwise fix test-strength and docs findings that add no behaviour, and set the spec's
   `**Status**` to `Closed <date> — shipped in PRs #…`. A fresh subagent reviews the diff.
4. Gate: `mise run gate`, detached as in [../references/pr-and-merge.md](../references/pr-and-merge.md)
   §2, when the diff touches code or tests; otherwise `mise run test-scripts`.
5. Open the close PR (`docs(NNN): close the spec`).
