# Autopilot ledger — #491 multi-session-daemons

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: GitHub issue #491 "Support multiple session daemons: host and container" (host and container runtimes only; SSH #687 and Kubernetes #688 out of scope); labels: enhancement, flow:feature
- **Kind**: feature
- **Effort**: default
- **Issue**: #491
- **Worktree branch**: feat/491_support-multiple-session-daemons-host-container
- **Started**: 2026-10-09
- **Phase**: milestone M2
- **Next step**: continue M2 from Handover

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #695 | Design | merged | 52c691b8fd00540bcf027f60e779003368266214 |
| #752 | M1 registry, bindings, labels | merged | 3787994d7fe1807844ddfa485d51333227e5a66e |

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1 | T001–T017 | full | Upgraded install runs on a one-daemon registry, worktrees bound and labelled, daemon choice persists | #752 | merged |
| M2 | T018–T029, T048–T049 | full | Host and container daemons concurrent and failure-isolated | - | in progress |
| M3 | T030–T039 | full | Settings Daemons section with removal and rebind | - | pending |
| M4 | T040–T045 | full | Real-runtime multi-daemon e2e in CI | - | pending |

## Decisions

- FR-014 (decided by user): removed daemon's worktrees kept, sessions stopped, shown as "no daemon" until rebound; FR-003 is "at most one".
- FR-016 (agent-resolved): names and branches unique across a project's daemons.
- Several container daemons allowed (agent-resolved: issue #491); at most one host daemon (plan Complexity Tracking, R3); spec amended in the tasks unit to match (clarification, FR-001, edge case, assumption).
- Tasks unit: bindings and legacy default keyed by a stable `DaemonId` (analyze F1: name-keyed bindings rebound on remove-and-re-add); contracts, data model, plan updated. Unknown runtime kind is `DaemonRuntime::Unsupported`; `ProjectAdd` sent to the bound daemon on first bind and each connect (research risk resolved).
- M1 is split from the P1 stories along acceptance scenarios (rule 3): M1 carries US1 scenarios 2, 4 and US4, M2 US1 scenarios 1, 3 and US2; the P1 story has no smaller observable deliverable before the registry exists. Polish (T046, T047) is doc-only, left to the close unit.

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|

## Review rounds

| Review | Round | Snapshot | Verdict |
|---|---|---|---|
| Spec | 1 | 1deadc93434025bb863375f513b4bce46ac46929:e9abd92efe2c7d8ad097ff82d4a7073477349524 | CHANGES: 1 MAJOR, 3 MINOR (fixed) |
| Spec | 2 | 26c2ce0a99dd2b7b51f6ae0d78cedecb665d7169:e9abd92efe2c7d8ad097ff82d4a7073477349524 | CLEAN (1 MINOR, fixed, prose only) |
| Plan | 1 | 45443d59c4d375cbe2310511d0aa4b7c45cc4658:fe509bbbdf367dfff39b8a00877b46d761e0bb60 | CHANGES: 3 MAJOR, 3 MINOR (fixed) |
| Plan | 2 | dd98153eb049be30b1f7ded35e6bab4d1b5cee26:fe509bbbdf367dfff39b8a00877b46d761e0bb60 | CLEAN (2 MINOR, fixed, prose only) |
| Tasks | 1 | 553778aa8b75e259e26db8d663adf76b15c0b55a:091a83317c05ccbbf44776757b31d7aa5976014b | CHANGES: 3 MAJOR, 3 MINOR (fixed) |
| Tasks | 2 | 7fe587d53b347a6be1ed064c59646866118a9e28:5c145d4b1457f9917fdd01920fa82aebd179b98e | CLEAN |
| Code A (M1) | 2 | 04d99a7c02f2a93e14989a54da51dd9a81d48350:b30131c92e14b55a61593dbee7c55d3fb3fd14fd | CLEAN (2 MINOR, fixed) |
| Code B (M1) | 1 | 04d99a7c02f2a93e14989a54da51dd9a81d48350:b30131c92e14b55a61593dbee7c55d3fb3fd14fd | CHANGES: 1 MAJOR declined (attach is the same create path), 3 MINOR declined/deferred, see Declined |
| Code A (M1) | 1 | ab3af6818291bc68e24c3e183b22d60a78446c4f:30ccfce93a158c7bda37cfdb0b4a8920fb55396f | CHANGES: 2 MAJOR, 2 MINOR (F1 lock around state read-modify-write, F2 set_single_daemon only on daemon change, F3 daemon taken at submit fixed; F4 binding outliving its worktree left, noted) |
| Code A (M2) | 1 | ef9ab0bb89cdd8fb66bb269b68a797d0de782674:3242febfcc63ac47a42b06e5ca4afeae79012555 | CHANGES: 5 MAJOR (F1 detach binding, F2 on_connected gating, F3 sandbox daemon id, F4 outbox on refused connect fixed; F5 declined), 3 MINOR (fixed) |
| Code B (M2) | 1 | ef9ab0bb89cdd8fb66bb269b68a797d0de782674:3242febfcc63ac47a42b06e5ca4afeae79012555 | CHANGES: 1 MAJOR (cycle-log entry, added), 3 MINOR (fixed); Verify green |

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|
| M1 | A r1 F4 | binding outlives its deleted worktree | Same-name reuse through a path that binds nothing falls back to the stored binding; dropping bindings on delete is registry-editing work (M3); noted as follow-up. |
| M1 | B r1 F1 | attach flow has no daemon choice | "Attach" in US1-4 is the form's existing-branch source, which goes through the same `WorktreeCreate` and binds. `AttachApply` (assistant-made worktrees) is a different flow whose rows take the legacy default until rebound (M3). |
| M1 | B r1 F2 | Default location never bound | The `""` key resolves to the legacy default; an explicit write needs the rebind action (M3, T034). Test A1 covers resolution. |
| M1 | B r1 F4 | T010 production wiring | Per-daemon inputs exist and are tested; callers switch in M2 with the actor (T018+), as T014 keeps the single path. |
| M2 | A r1 F5 | dial failure inserts into `disconnected` before the debounce | The stale-content banner on the first dial failure is the pre-existing single-connection behaviour; the per-daemon state (debounced) drives the row labels, which is what FR-009 asks. |

## Handover

None.

## Open escalation

None.

## Follow-ups not done

SSH (#687) and Kubernetes (#688) split out of #491; extension points only.
