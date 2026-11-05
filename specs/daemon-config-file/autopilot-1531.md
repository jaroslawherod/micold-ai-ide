# Autopilot ledger — #1531 daemon-config-file (Issue 10)

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: `/speckit-autopilot #1531` (`GH_TOKEN` for `gh` in sessions of every kind); labels: `flow:feature`, `effort:low`. Spec, Clarify and Plan skipped by the same precedent as #1524, #1525, #1526, #1529, #1530, #1534 and #1535: the design of the whole feature is already on main (PR #1509, `specs/daemon-config-file/`: spec.md, research.md, plan.md, `contracts/daemon-config.md` section `[[github.tokens]]`; FR-011, FR-035, SC-003); the tasks unit only cuts milestones for the "Issue 10" group (T055–T058, T095).
- **Kind**: feature
- **Effort**: low
- **Issue**: #1531
- **Worktree branch**: feat/1531_gh-token-for-gh-in-sessions-of-every-kind
- **Started**: 2026-11-05
- **Phase**: tasks (milestones cut, design PR open)
- **Next step**: design PR open (see PR table); orchestrator waits on CI and merges; then M1

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #1696 | Design (milestones for Issue 10) | OPEN | — |

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1 | T055–T057, T095, T058 | full | `gh` shim sets `GH_TOKEN`/`GH_HOST` from the token chosen for the cwd's repository, on every session kind | — | pending |

## Decisions

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|
| D1 | spec | Write a new spec.md for #1531? | No: the design of the whole feature is on main and covers this issue; a second spec would duplicate FR-011/FR-035/SC-003. Skipped as for #1524/#1525/#1526/#1529/#1530/#1534/#1535. | agent-resolved | specs/daemon-config-file/autopilot-1530.md |
| D2 | tasks | Split Issue 10? | No: 5 tasks, one shim (rule 3). Single milestone. | agent-resolved | milestones.md rule 3 |
| D3 | tasks | Tier | full: new PATH-injected shim that reads a chosen credential and sets it in a session's env (security-sensitive surface); not a reuse of an existing pattern | agent-resolved | milestone-format.md |
| D4 | tasks | speckit-analyze findings | 1 MEDIUM + 3 LOW, no CRITICAL/HIGH: fixed U1 (`GH_HOST` left alone with no token, same as `GH_TOKEN`) and U3 (Windows `gh.cmd` forwards exit code, cannot `exec`) by wording edits to T057/T095/Deliverable; U2 and N1 needed no fix (U2 is already covered by the milestone narrative; N1 is expected sequencing) | agent-resolved | speckit-analyze run |

## Review rounds

| Review | Round | Snapshot | Verdict |
|---|---|---|---|
| Tasks 1531 | 1 | 7a198beac8244eaa27e1da4bb57ab949a3097bce:93bc2a7f9a0a8723aa5ec8061cfe36c235c9f6f3 | CHANGES: F1 MAJOR (Depends-on claimed Issue 9 M2, rationale only cites M1 tasks) fixed by narrowing to M1 merged; F2 MINOR (T056 no file path, wrongly tagged `[P]` with T055 in the same file) fixed by adding the path and dropping `[P]`. Both fixes are task-text only, no new round. |

## Declined review findings

None.

## Handover

None.

## Open escalation

None.

## Follow-ups not done

- **Blocked by #1530** (Issue 9, tasks T046–T054, T092–T094). #1530's design PR #1686 is merged on
  main, but its implementation is not: the `TokenChoice` chooser in
  `crates/micold-core/src/daemon_config/matching.rs` and the stateless `git-credential` work
  (`crates/micold-daemon/src/git_credential.rs`) do not exist on main yet — #1530's M1 commit sits
  unmerged on branch `feat/1530_the-daemon-s-credential-helper-chooses-the-token`. Cutting
  milestones and writing task text here is unaffected (the whole of this unit's job); the M1
  implementation milestone below waits for #1530 to merge, which the orchestrator handles, not
  this unit. M1's tasks (T055–T057, T095) name the #1530 API they build on: `gh-token` resolves a
  token the same way `git-credential get` does (T047), through the `TokenChoice` chooser
  (T046) and the per-host `host`/`ssh_to_https` rewrite (T093).
