# Phase 5: close unit

1. **Checks.** `speckit-tdd-verify` always. `speckit-converge` first, unless *Review rounds* holds a
   `CLEAN` review B for every milestone in the ledger: B already checked each milestone's tasks and
   scenarios against its diff. Do not run `speckit-docguard-guard`: its findings here were always
   repo conventions this repo never adopted.
2. **Unbuilt behaviour** found by either: cut it into a new milestone per
   [../references/milestones.md](../references/milestones.md), add it to the ledger, commit, and
   return `DONE` with `NEW MILESTONES: M<n>…` (do not push; they ship in the first new milestone's
   PR). The orchestrator runs them, then a new close unit.
3. Otherwise fix test-strength and docs findings that add no behaviour, and set the spec's
   `**Status**` to `Closed <date> — shipped in PRs #…`. Test-strength findings go to
   `autopilot-worker` subagents on `"sonnet"`, one per crate: give each its findings from
   `tdd/verification.md` by ID, the test files, and the test command that must pass. Run that
   command yourself afterwards. A fresh subagent reviews the diff.
4. Gate: `mise run gate`, detached as in [../references/pr-and-merge.md](../references/pr-and-merge.md)
   §2, when the diff touches code or tests; otherwise *Specs-only PRs* there.
5. **Finish the ledger** in the PR's last commit: **Phase** `done`, **Next step** `handoff`, the
   close PR in *Pull requests*. The run ends when this PR merges; there is no further PR.
6. Open the close PR (`docs(NNN): close the spec`).
