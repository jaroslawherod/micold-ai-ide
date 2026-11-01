# Autopilot ledger — #1524 daemon-config-file (Issue 3)

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: `/speckit-autopilot #1524` (The daemon loads its configuration file and reloads it live); labels: `flow:feature`, `effort:high`. Spec, Clarify, Plan skipped; design on main (PR #1509); tasks unit only cuts milestones for the "Issue 3" group (T014–T020, T019a, T019b, T088, T089).
- **Kind**: feature
- **Effort**: high
- **Issue**: #1524
- **Worktree branch**: feat/1524_the-daemon-loads-its-configuration-file-and
- **Started**: 2026-10-30
- **Phase**: done
- **Next step**: handoff

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #1616 | Design (milestones for Issue 3) | merged | b6f32a1a7feabb6a2fedfd5b4c1b0ed5d7dedf2a |
| #1624 | M1 | merged | 8d298a88dbce086d88ae937266f294e07cee7172 |

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1 | T014–T020, T015a, T016a, T019a, T019b, T020a, T088, T089 | full | Daemon loads config file, live reload, `config` diagnostic | #1624 | merged |

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
| Close tdd-verify | 1 | - | FAIL (6 survivors on wiring, watcher, restart keys, no-write guard); test-strength fixes F2-F5 (+F10/F11 partly) by a worker, 8 of 8 mutants caught after; F1, F6-F9 left, recorded in verification.md |
| Close converge | - | - | run by a worker; all 14 tasks built, no unbuilt behaviour (F1 LOW: Windows ACL string only unit-tested) |
| Close review | 1 | 01f53354:367d564a | CHANGES: 3 MAJOR (verification.md claimed F7/F9 fixed and had inconsistent counts: doc corrected, prose only), 3 MINOR (F10/F11 partly, Status wording fixed; notify 300 ms wait noted in F9) |
| Review B M1 | 1 | 82be00e2:27264b75 | CHANGES: 2 MAJOR (red evidence: mutation checks recorded; task file lists amended), 1 MINOR (OS-neutral noted in cycle log); Verify run by me: 19 + 17 pass |
| Close checks | 1 | 01f53354:367d564a | converge: all 14 M1 tasks built, none unbuilt; tdd-verify: FAIL (0 of 6 mutants caught), fixed with tests only: 8 of 8 caught |
| Close diff review | 1 | 01f53354:367d564a | CHANGES: 2 MAJOR (verification.md claimed F9 and F7 fixed: claims removed, listed as left; prose only), 3 MINOR (counts fixed, stale-findings note added, SAFETY comment softened) |

## Declined review findings

- Review A F3 (blocking `std::fs::read` on the 2 s poll inside a tokio task): declined, MINOR; the files are a few hundred bytes and a stalled mount only delays the reload task, not sessions. Revisit with `spawn_blocking` if a field report shows it.

## Handover

None.

## Open escalation

None.

## Follow-ups not done

- Without a file and without `--kind`, a sandbox daemon still starts from the old `MICOLD_*` variables (transition, D11); flipping that to fatal belongs to issue 15.

- Previous PR #1614 (close of #1523) is merged on main.
- `scripts/autopilot/resume.sh` globs only `specs/*/autopilot.md`, so it misses `autopilot-1524.md` and reports main's stale copy; not changed in this flow.
- tdd-verify left open: F6 (no test that a changed token file reaches the next git or `gh` call, FR-006), F7 (hash test rebuilds the framing), F9 (fixed 300 ms sleep, real clock), F1 (no per-behavior red for #1524), F8 (no Windows run). See `specs/daemon-config-file/tdd/verification.md`.
- T019b: no test runs a real git or `gh` credential call across a reload (converge note).
- Review A F3 revisit with `spawn_blocking` if a field report shows it (declined above).
- The parked mutant of an earlier close attempt is on local branch `wip/1524-server-mutant`, not merged.
- tdd/verification.md F6 (no test that a changed token file reaches the next git or gh call, FR-006), F7-F9 and the Windows/macOS ACL path (T089) stay open test gaps; F4's consumer-level test of apply_to_process needs a seam (settings::install is a OnceLock).
