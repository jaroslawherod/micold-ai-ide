# Autopilot ledger — #1524 daemon-config-file (Issue 3)

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: `/speckit-autopilot #1524` (The daemon loads its configuration file and reloads it live); labels: `flow:feature`, `effort:high`. Spec, Clarify, Plan skipped; design on main (PR #1509); tasks unit only cuts milestones for the "Issue 3" group (T014–T020, T019a, T019b, T088, T089).
- **Kind**: feature
- **Effort**: high
- **Issue**: #1524
- **Worktree branch**: feat/1524_the-daemon-loads-its-configuration-file-and
- **Started**: 2026-10-30
- **Phase**: implement M1
- **Next step**: push the branch and open the M1 PR (rebased onto origin/main, gate rerun)

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #1616 | Design (milestones for Issue 3) | merged | b6f32a1a7feabb6a2fedfd5b4c1b0ed5d7dedf2a |

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1 | T014–T020, T015a, T016a, T019a, T019b, T020a, T088, T089 | full | Daemon loads config file, live reload, `config` diagnostic | — | in progress |

## Decisions

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|
| D1 | tasks | Skip spec/clarify/plan? | Yes, design merged on main | agent-resolved | autopilot-1523.md D1; issue body "Plan: specs/daemon-config-file/ ... Implement test-first" |
| D2 | tasks | Split Issue 3? | No: one milestone (shared DaemonSettings and Arc swap; no half-wired daemon) | agent-resolved | milestones.md rule 3 (about 11 tasks, one story) |
| D3 | tasks | User-guide task | Added T020a to the milestone (CI user-guide gate) | agent-resolved | milestones.md rule 4 |
| D4 | pr | Push rejected as stale by the old remote branch; force push? | Allowed; the stale remote branch is gone, plain `git push -u origin HEAD`, explicit-lease force push allowed if needed | user | "allow force push", 2026-10-31 |

## Review rounds

| Review | Round | Snapshot | Verdict |
|---|---|---|---|
| Tasks 1524 | 1 | b37eaff2:02497c0c | CHANGES: 2 MAJOR (no tests for diagnostic and credential_root: added T015a, T016a), 2 MINOR (T019b marked guard test, T089 moved after T020); test-task additions, no further round (round limit not reached; sibling pattern) |
| Tasks 1524 | 2 | adf7e0fe:a4f36020 | CLEAN (sonnet, fix diff); 2 MINOR fixed, prose/markers only (T015a/T016a not [P], milestone prose) |
| Review A M1 | 1 | 82be00e2:27264b75 | CHANGES: 1 MAJOR (unbounded debounce: fixed, test added), 2 MINOR (UTF-8 decode fixed; blocking read in task declined) |
| Review B M1 | 1 | 82be00e2:27264b75 | CHANGES: 2 MAJOR (red evidence: mutation checks recorded; task file lists amended), 1 MINOR (OS-neutral noted in cycle log); Verify run by me: 19 + 17 pass |

## Declined review findings

- Review A F3 (blocking `std::fs::read` on the 2 s poll inside a tokio task): declined, MINOR; the files are a few hundred bytes and a stalled mount only delays the reload task, not sessions. Revisit with `spawn_blocking` if a field report shows it.

## Handover

None.

## Open escalation

None.

## Follow-ups not done

- Without a file and without `--kind`, a sandbox daemon still starts from the old `MICOLD_*` variables (transition, D11); flipping that to fatal belongs to issue 15.

- Previous PR #1614 (close of #1523) is merged on main.
