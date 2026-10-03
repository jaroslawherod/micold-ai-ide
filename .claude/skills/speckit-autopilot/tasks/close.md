# Task: close the spec (close unit, feature flow)

When: every milestone in the ledger is merged.

1. **Checks.** `speckit-tdd-verify` always (through an `autopilot-worker`, see
   [../rules/waiting.md](../rules/waiting.md)). `speckit-converge` first, unless *Review rounds*
   holds a `CLEAN` review B for every milestone in the ledger: B already checked each milestone's
   tasks and scenarios against its diff. Do not run `speckit-docguard-guard`: its findings here
   were always repo conventions this repo never adopted.
2. **Unbuilt behaviour** found by either: cut it into a new milestone per
   [milestones.md](milestones.md), add it to the ledger, commit, and return `DONE` with
   `NEW MILESTONES: M<n>…` (do not push; they ship in the first new milestone's PR). The
   orchestrator runs them, then a new close unit.
3. Otherwise fix test-strength and docs findings that add no behaviour, and set the spec's
   `**Status**` to `Closed <date> — shipped in PRs #…`. Test-strength findings go to
   `autopilot-worker` subagents on `"sonnet"`, one per crate: give each its findings from
   `tdd/verification.md` by ID, the test files, and the test command that must pass. Run that
   command yourself afterwards.
4. A fresh subagent reviews the diff ([review.md](review.md), no rubric file: correctness of the
   tests and truth of the docs).
5. Gate ([gate.md](gate.md)): full when the diff touches code or tests, else specs-only or docs.
6. **Finish the ledger** in the PR's last commit: **Phase** `done`, **Next step** `handoff`, the
   close PR in *Pull requests*. The run ends when this PR merges; there is no further PR.
7. Open the close PR per [pr.md](pr.md).

Hands on: return `DONE` with the PR number, or with `NEW MILESTONES: …` and `PR: none`.
