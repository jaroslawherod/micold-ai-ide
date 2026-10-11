# Autopilot ledger — #491 multi-session-daemons

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: GitHub issue #491 "Support multiple session daemons: host and container" (host and container runtimes only; SSH #687 and Kubernetes #688 out of scope); labels: enhancement, flow:feature
- **Kind**: feature
- **Effort**: default
- **Issue**: #491
- **Worktree branch**: feat/491_support-multiple-session-daemons-host-container
- **Started**: 2026-10-09
- **Phase**: milestone M4
- **Next step**: wait for CI and merge of the M4 PR, then close

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #695 | Design | merged | 52c691b8fd00540bcf027f60e779003368266214 |
| #752 | M1 registry, bindings, labels | merged | 3787994d7fe1807844ddfa485d51333227e5a66e |
| #758 | M2 concurrent host and container daemons | merged | e0d9dd819951476b5f4a2ad94094401af243e601 |
| #767 | M3 Settings Daemons section | merged | 678441fcc0e56eacf56b3005219257c67469d9d0 |
| #768 | M4 real-runtime multi-daemon e2e | open | - |

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1 | T001–T017 | full | Upgraded install runs on a one-daemon registry, worktrees bound and labelled, daemon choice persists | #752 | merged |
| M2 | T018–T029, T048–T049 | full | Host and container daemons concurrent and failure-isolated | #758 | merged |
| M3 | T030–T039 | full | Settings Daemons section with removal and rebind | #767 | merged |
| M4 | T040–T045 | full | Real-runtime multi-daemon e2e in CI | #768 | PR open |

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
| Code A+B (M2) | 2 | a1af07f954ab05029a0c4d7608e1a36d1fc8e8c3:3f7af08c3e6d3e7147aee13d5dfdfc24d282e9d7 | CLEAN (3 MINOR, see Follow-ups) |
| Code B (M3) | 1 | 2e7b47aa4270cb4b7537cf70ba9575811f48af06:c38fd6f09bd55b99c88095bc24dff0680eb6ed54 | CHANGES: 1 MAJOR (cycle-log M3 entry, added), 1 MINOR declined; Verify green (22 tests) |
| Code A (M3) | 1 | 4b52f6d7682f6f00ab06e2a8e2b23bc0e2ce9893:6731e8bb6222e07008a6821038dbc96d905f3075 | CHANGES: 2 MAJOR (F1 stopped daemon not torn down client-side, F2 edit of unsupported runtime; both fixed), 3 MINOR (F3 draft cleared on removal fixed; F4 inactive-project count, F5 route_for test seam: see Declined) |
| Code A (M4) | 1 | 8840b409bd105f4f2c0f794457959d4cc6a68224:678441fcc0e56eacf56b3005219257c67469d9d0 | CHANGES: 1 BLOCKER (HostDaemon missing field, did not compile), 1 MAJOR (5 s timer started after stop), 3 MINOR (F3, F5 fixed; F4 duplicate fuser helper left, see Follow-ups); fixes re-run green |
| Code B (M4) | 1 | a9d9bb12c978df6b328827498291f057e8aa654f:678441fcc0e56eacf56b3005219257c67469d9d0 | CLEAN (3 MINOR: worktree-level binding is inferred from daemon state; leak guard is the final converse; binding test is type-level; left) |

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|
| M1 | A r1 F4 | binding outlives its deleted worktree | Same-name reuse through a path that binds nothing falls back to the stored binding; dropping bindings on delete is registry-editing work (M3); noted as follow-up. |
| M1 | B r1 F1 | attach flow has no daemon choice | "Attach" in US1-4 is the form's existing-branch source, which goes through the same `WorktreeCreate` and binds. `AttachApply` (assistant-made worktrees) is a different flow whose rows take the legacy default until rebound (M3). |
| M1 | B r1 F2 | Default location never bound | The `""` key resolves to the legacy default; an explicit write needs the rebind action (M3, T034). Test A1 covers resolution. |
| M1 | B r1 F4 | T010 production wiring | Per-daemon inputs exist and are tested; callers switch in M2 with the actor (T018+), as T014 keeps the single path. |
| M2 | A r1 F5 | dial failure inserts into `disconnected` before the debounce | The stale-content banner on the first dial failure is the pre-existing single-connection behaviour; the per-daemon state (debounced) drives the row labels, which is what FR-009 asks. |
| M3 | B r1 F2 | old `settings/daemon.rs` not replaced (T034) | MINOR; it is the Session service section and carries the sandbox settings, which the Daemons list does not duplicate. |
| M3 | A r1 F4 | daemon_facts counts only the active project's worktrees | Recorded as follow-up: the confirmation count covers worktrees known to this window; the cross-project view needs the persisted per-project lists. |
| M3 | A r1 F5 | route_for only called by tests | MINOR; kept as the reducer's documented resolution seam, the production path resolves through links::route. |

## Handover

None. Visual pass M3 (T039) ran headless under sway, see specs/491-multi-session-daemons/visual-pass/visual-pass-m3.md: list, details, add form, removal confirm, empty state, bind picker, no-daemon form notice PASS; quickstart B1 (Starting -> Connected), B2 (two bound worktrees), B3 (live stop) NOT RUN/partial: no container runtime locally (M4's real-runtime e2e covers them). Cosmetic notes (Connected and Stopped chips close in colour, removal text mentions container for a host daemon) left as follow-ups.

## Open escalation

None.

## Follow-ups not done

SSH (#687) and Kubernetes (#688) split out of #491; extension points only.
- M2: switching project to a location on an already-connected other daemon does not re-ask CLI availability or re-adopt settings (on_connected does them only for the active daemon).
- M2: a worktree's binding is dropped on the `WorktreeDeleted` reply only; a connection drop first leaves it until rebound.
- M2: only one sandbox container (legacy or first container entry) is brought up by the app; further container daemons connect when already running (per-daemon bring-up for each is not done).
- M2: ConfirmationAnswer, settings sends and CLI-availability asks go to the active daemon only.

- M3 follow-ups (cosmetic, from the visual pass): Connected and Stopped state chips are both muted, only the text differs; the removal confirmation for a host daemon mentions the container; "Details" toggle shifts Remove by ~30px; connect-at-launch checkbox sits 8px above Save.
- M4: the `fuser`/`kill` host-daemon cleanup in `sandbox_real_support` duplicates `client_restart_input.rs`; hoist when either is next touched.
