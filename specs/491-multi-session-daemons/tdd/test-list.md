---
feature: 491-multi-session-daemons
loop: inside-out
profile: .specify/memory/tdd-profile.md
spec_criteria: 25
planned_at: 52c691b8
updated_at: 52c691b8
suite_baseline: green
---

# Test List: Support multiple session daemons (milestones M1 to M4)

Scope of this list: M1 (T001-T017) in the tables below; M2 to M4 in the section after them.
Rules live in `micold-core` (inside-out); the client behaviors are render-free reducer/projection tests.

## Outer loop: acceptance behaviors

| id | behavior | traces | kind | state | test |
| --- | --- | --- | --- | --- | --- |
| A1 | Host and container fixtures load into a client state with one daemon, every location bound to it, close and reopen keeps it | US4-1..4, FR-013 | example | DONE | `crates/micold-client/tests/upgrade_single_daemon.rs` |
| A2 | Each worktree row projects its daemon name, "no daemon" when unbound, two daemons of one project both listed | US1-2, FR-005 | example | DONE | `crates/micold-client/tests/worktree_daemon_label.rs` |
| A3 | Create and attach offer every registry daemon, choice stored as binding and survives reload; refusal names holder's daemon | US1-4, FR-006, FR-016 | example | DONE | `crates/micold-client/tests/worktree_form_daemon_choice.rs` |

## Inner loop: unit behaviors

### `crates/micold-core/src/daemons.rs`

| id | behavior | traces | kind | state | test |
| --- | --- | --- | --- | --- | --- |
| U1 | A name is trimmed before it is stored | FR-001 | example | DONE | `daemons_registry.rs` |
| U2 | A blank name is refused with BlankName | FR-001 | example | DONE | `daemons_registry.rs` |
| U3 | A name equal case-insensitively to another is refused with DuplicateName | FR-001 | example | DONE | `daemons_registry.rs` |
| U4 | Ids are unique and never reused after removal (remove X, add X: new id) | FR-001 | example | DONE | `daemons_registry.rs` |
| U5 | Renaming keeps the id | FR-003 | example | DONE | `daemons_registry.rs` |
| U6 | A second container daemon with the same container name or port is refused ContainerInUse{other} | FR-001 | example | DONE | `daemons_registry.rs` |
| U7 | An empty image or an out-of-range port is refused InvalidField{field} | FR-001 | example | DONE | `daemons_registry.rs` |
| U8 | A second host entry is refused HostExists | FR-001 | example | DONE | `daemons_registry.rs` |
| U9 | DaemonRuntime::label() is the plain runtime value | FR-015 | example | DONE | `daemons_registry.rs` |

### `crates/micold-core/src/settings.rs`

| id | behavior | traces | kind | state | test |
| --- | --- | --- | --- | --- | --- |
| U10 | A host-placement document without `daemons` yields one Host entry, id 1, legacy_default_daemon 1, next_daemon_id 2 | FR-013 | example | DONE | `daemons_migration.rs` |
| U11 | A container document yields one container entry carrying micold-sandbox, port 7727 and the profile | FR-013 | example | DONE | `daemons_migration.rs` |
| U12 | A document with `daemons` present is never re-migrated | FR-013 | example | DONE | `daemons_migration.rs` |
| U13 | settings_version is unchanged and the `daemon` key is still written | FR-013 | example | DONE | `daemons_migration.rs` |
| U14 | An unknown runtime kind loads as Unsupported (cannot start, removable) and survives save verbatim | FR-001 | example | DONE | `daemons_migration.rs` |

### `crates/micold-core/src/store.rs`

| id | behavior | traces | kind | state | test |
| --- | --- | --- | --- | --- | --- |
| U15 | bindings (incl. reserved key "" for Default) round-trip through project state | FR-006, FR-017 | example | DONE | `daemons_bindings.rs` |
| U16 | An absent key resolves to legacy_default_daemon while registered, else NoDaemon | FR-003 | example | DONE | `daemons_bindings.rs` |
| U17 | A key naming an unknown id reads NoDaemon | FR-003 | example | DONE | `daemons_bindings.rs` |
| U18 | Removing X then adding a new X does not rebind its worktrees | FR-014 | example | DONE | `daemons_bindings.rs` |
| U19 | Renaming keeps bindings; adding a second daemon does not unbind legacy worktrees | FR-003 | example | DONE | `daemons_bindings.rs` |
| U20 | Removing an entry writes nothing to disk | FR-014 | example | DONE | `daemons_bindings.rs` |
| U23 | A binding the client wrote (`save_binding`) survives the daemon's later save of an older snapshot; a project with no state file refuses it | FR-006, FR-017 | example | DONE | `daemons_bindings.rs` |
| U24 | A branch or name refusal names the holding worktree's daemon | FR-016 | example | DONE | `daemons_bindings.rs` |

### `crates/micold-core/src/sandbox/`

| id | behavior | traces | kind | state | test |
| --- | --- | --- | --- | --- | --- |
| U21 | Two container daemons yield disjoint MountSets, state dirs, token paths and container names | FR-017 | example | DONE | `daemons_isolation.rs` |
| U22 | The migrated daemon keeps the legacy name, port and token path | FR-013 | example | DONE | `daemons_isolation.rs` |

## M2 to M4 behaviors

All DONE. Test-first order is not provable from git history for these milestones (tests and code were
committed together); each deviation is recorded per milestone in `tdd/cycle-log.md`.

| id | behavior | traces | kind | state | test |
| --- | --- | --- | --- | --- | --- |
| B1 | Each daemon has the five states and only the legal transitions; one failing daemon leaves the others' state alone | FR-007, FR-008 | example | DONE | `crates/micold-core/tests/daemon_state.rs` |
| B2 | An operation is routed to the bound daemon only; one link dropping leaves the other's sessions live | FR-002, FR-003 | example | DONE | `per_daemon_actors.rs`, `daemon_routing.rs`, `catalog_two_daemons.rs`, binary test `two_daemons` |
| B3 | A worktree on an unreachable daemon shows "daemon unavailable"; a worktree with no daemon refuses operations | FR-004, FR-009, FR-014 | example | DONE | `worktree_unavailable_label.rs` |
| B4 | A version mismatch is reported on that daemon alone; a restarted daemon is re-adopted | FR-008 | example | DONE | `daemon_version_mismatch.rs`, `daemon_adopt_on_restart.rs` |
| B5 | The Daemons section lists, adds, edits, starts, stops and removes; removal asks with counts and leaves disk untouched | FR-010 to FR-012 | example | DONE | `daemons_feature.rs`, `daemon_removal.rs`, `daemons_section_geometry.rs` |
| B6 | Host and container daemons run together against the real runtime; stopping one leaves the other usable | FR-002, FR-018, SC-001, SC-002, SC-006 | example | DONE | `sandbox_real_multi_daemon*.rs`, `daemon_rows_uniform.rs` |

## Verification commands

- single: `scripts/build-lock.sh cargo test --test {file} {name} -- --exact`
- fast subset: `mise run test-core`
- suite: `mise run gate`
