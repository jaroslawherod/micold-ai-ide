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
| Code A (M1) | 1 | ab3af6818291bc68e24c3e183b22d60a78446c4f:30ccfce93a158c7bda37cfdb0b4a8920fb55396f | CHANGES: 2 MAJOR, 2 MINOR (F1 lock around state read-modify-write, F2 set_single_daemon only on daemon change, F3 daemon taken at submit fixed; F4 binding outliving its worktree left, noted) |

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|
| M1 | A r1 F4 | binding outlives its deleted worktree | Same-name reuse through a path that binds nothing falls back to the stored binding; dropping bindings on delete is registry-editing work (M3); noted as follow-up. |
| M1 | B r1 F1 | attach flow has no daemon choice | "Attach" in US1-4 is the form's existing-branch source, which goes through the same `WorktreeCreate` and binds. `AttachApply` (assistant-made worktrees) is a different flow whose rows take the legacy default until rebound (M3). |
| M1 | B r1 F2 | Default location never bound | The `""` key resolves to the legacy default; an explicit write needs the rebind action (M3, T034). Test A1 covers resolution. |
| M1 | B r1 F4 | T010 production wiring | Per-daemon inputs exist and are tested; callers switch in M2 with the actor (T018+), as T014 keeps the single path. |

## Handover

Second unit handed over at ~110k context, early, because every probe re-read a large context. Branch is rebased onto origin/main (1e3be384); commits are local, not pushed, no PR yet.
Done (committed as `wip(491): M1 core ...`): core half as in the first handover (daemons.rs, settings migration, Workspace.bindings, Placement::from_runtime, tests daemons_registry|migration|bindings|isolation green), plus in this unit, UNCOMMITTED-then-committed and NOT YET COMPILED OR RUN:
- `store.rs`: `ProjectStore::save_binding(project, key, DaemonId)` (default Ok; JsonFileStore does a read-modify-write of the project state file's `bindings`, refuses a Missing/Corrupt file; FakeProjectStore sets `workspace.bindings`); `JsonFileStore::save` overlays on-disk bindings over the daemon's snapshot (disk wins) so the daemon never clobbers a client-written binding.
- `worktree.rs`: `BlockReason::explain_on(branch, daemon: Option<&str>)` and `explain_directory_taken_on(dir, daemon)` append " It runs on the daemon 'X'." (FR-016, T016).
- Tests U23/U24 appended to `crates/micold-core/tests/daemons_bindings.rs`, listed in tdd/test-list.md.
- `mise run test-core` was started detached (log: the session scratchpad `core.log` of the previous unit; it waits for another worktree's build lock) -- ignore it, re-run.
Next steps:
1. `mise run test-core`; fix. Then `scripts/build-lock.sh cargo check --workspace --all-targets`: new `Settings` fields break struct literals (known: `ValidSettings::into_settings` in `crates/micold-client/src/features/settings.rs:497` -> add `..Settings::default()`-style or explicit `daemons/next_daemon_id/legacy_default_daemon`; `set_changed` in `shell/persist.rs:~331` destructures `Settings` exhaustively -> add the three fields as `_`, and at its end call `target.set_single_daemon(target.daemon.clone())` so a placement/profile save moves the registry entry; `startup.rs:~631` assigns `sandboxed.daemon.placement` -> use `set_single_daemon`). Tick T001-T010 in tasks.md; append batched cycle-log entry (red = todo!() panics).
2. Client design decided (do not redo the analysis): the client never writes projects.json (daemon is the Catalog's single writer), so bindings go through `app.caps.projects().save_binding(..)` (shell side, `caps.projects()` is `Option<&dyn ProjectStore>`) and the in-memory `core.workspace.bindings` is updated in the same step; `workspace.bindings` writes by a feature need an entry in `tests/feature_write_isolation.rs` `OWNERS` (`("workspace.bindings","worktree")`) or go through a `Workspace` method listed in `CORE_MEDIATED`; prefer adding `Workspace::bind(project, key, id)` in core + a CORE_MEDIATED line.
   - settings feature State (`features/settings.rs`) gains `daemons: DaemonRegistry` + `legacy_default_daemon: Option<DaemonId>`; set at boot in `shell/startup.rs` from `loaded.settings` via a new `Msg::DaemonsLoaded{registry, legacy_default}` (callers other than app.rs must go through `Message::Settings(..)`, see feature_registration_cost.rs) and re-sent after a save in `shell/persist.rs::apply_save`.
   - `State::worktree_daemon_label(dir_name)` / binding resolve helper in `features/sidebar.rs` impl State via `Binding::resolve(workspace.bindings[active].get(key)...)`; `WorktreeNode` gains `daemon: String` (name, or const `NO_DAEMON_LABEL = "no daemon"`); update every `WorktreeNode {..}` literal (both in sidebar.rs, tests/features_sidebar.rs and others: grep). Render the label on the row in `ui/` sidebar.
   - worktree form: `WorktreeForm` gains `daemons: Vec<DaemonChoice{id,name}>` (filled in `opened(state)` from the registry), `daemon: Option<DaemonId>` (default legacy default else first), `holder_daemons: BTreeMap<dir_name,label>` for the refusal text; `Msg::DaemonChosen`; Select in `ui/worktree_form.rs::modal` (omit when empty registry); refusal text in `ui/worktree_form.rs:80,563,590` uses `explain_on`/`explain_directory_taken_on` with `holder_daemons[folder_name(path)]`. Binding write on `created()` (worktree_form.rs:1261, emits `Outcome::WorktreeCreated`): in shell where that outcome is interpreted / `shell/daemon_sync.rs`, call `save_binding`.
   - Tests first: T011 `upgrade_single_daemon.rs` (load fixtures from `crates/micold-core/tests/fixtures/daemons/` through `JsonFileSettingsStore`/`JsonFileStore`, build client `State`, assert one daemon, every worktree and Default `Bound(id 1)`, close/reopen), T012 `worktree_daemon_label.rs`, T013 `worktree_form_daemon_choice.rs`. Name new test files in tdd/test-list.md A1-A3 rows.
3. T017 docs (`docs/user-guide/worktrees-and-sessions.md`, `settings.md`); check `scripts/tests/*` doc gates and CI's user-guide gate (feat).
4. Scoped gate + review A (high) + review B + `mise run gate` + PR (`feat(491): ...(#491)`), per tasks/gate.md, review.md, pr.md. Build lock is contended by other worktrees: always start gates detached.

## Open escalation

None.

## Follow-ups not done

SSH (#687) and Kubernetes (#688) split out of #491; extension points only.
