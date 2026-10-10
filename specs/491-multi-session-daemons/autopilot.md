# Autopilot ledger — #491 multi-session-daemons

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: GitHub issue #491 "Support multiple session daemons: host and container" (host and container runtimes only; SSH #687 and Kubernetes #688 out of scope); labels: enhancement, flow:feature
- **Kind**: feature
- **Effort**: default
- **Issue**: #491
- **Worktree branch**: feat/491_support-multiple-session-daemons-host-container
- **Started**: 2026-10-09
- **Phase**: milestone M1
- **Next step**: continue M1 from Handover (verify core suite, client T011-T017)

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #695 | Design | merged | 52c691b8fd00540bcf027f60e779003368266214 |

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1 | T001–T017 | full | Upgraded install runs on a one-daemon registry, worktrees bound and labelled, daemon choice persists | - | pending |
| M2 | T018–T029, T048–T049 | full | Host and container daemons concurrent and failure-isolated | - | pending |
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

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|

## Handover

M1 (T001-T017) in progress on branch feat/491_...; first unit handed over at 150k.

Done (uncommitted work is committed as `wip(491): M1 core registry, migration, bindings`, not pushed, no PR yet):
- tdd/test-list.md + cycle-log.md written (behaviors U1-U22 core, A1-A3 client; tasks.md carries [Un]/[An] markers on T003-T016).
- Core: `crates/micold-core/src/daemons.rs` (DaemonId/Name/Runtime/Entry/Registry/Binding, `migrated_entry`, `DaemonEntry::{container_name,state_dir,token_path}`), `Placement::from_runtime`, `Settings.{daemons,next_daemon_id,legacy_default_daemon}` + migration + `Settings::{registry,set_registry,set_single_daemon}`, `Workspace.bindings` persisted in store.rs project state. Tests `daemons_registry|migration|bindings|isolation` + fixtures in `crates/micold-core/tests/` are GREEN (red evidence: todo!() panics, scratchpad red1.log).
- Pending check: whole-core regression `mise run test-core` was started detached but waited on another worktree's build lock; re-run it, fix any breakage from the new `Settings`/`Workspace` fields (struct literals in other crates/tests), `cargo check --workspace` (client/daemon construct Settings/Workspace).

Next steps:
1. Run `mise run test-core` and a workspace `cargo check`/clippy; fix; tick T001-T010 in tasks.md; append cycle-log entries (batched per task group, not one-per-behavior: say so; red = todo!() panics).
2. Client T011-T016 (tests first: `upgrade_single_daemon.rs`, `worktree_daemon_label.rs`, `worktree_form_daemon_choice.rs` in crates/micold-client/tests): load registry into client state, resolve binding via `Binding::resolve(workspace.bindings[project], dir_name or "", registry, legacy_default)`; `WorktreeNode` gains a daemon label (`features/sidebar.rs::worktree_tree`, "no daemon" when NoDaemon); worktree form picker + binding write (`features/worktree_form.rs`, `shell/persist.rs`), holder's daemon in refusal text (`core/worktree.rs`). Where client assigns `settings.daemon`, use `Settings::set_single_daemon` so registry entry stays in step. NB `Workspace` is daemon-owned in store.rs (client save would clobber): check how bindings get written by the client (existing ClientMsg or a client-side project-state write) before T016.
3. T017 user guide docs (worktrees-and-sessions.md, settings.md).
4. Then scoped gate + review A (high) + review B + full gate + PR per tasks/gate.md, review.md, pr.md.

## Open escalation

None.

## Follow-ups not done

SSH (#687) and Kubernetes (#688) split out of #491; extension points only.
