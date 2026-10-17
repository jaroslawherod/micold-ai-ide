# Test list: 489 ssh-remote-daemon

Written by hand because the `speckit.tdd.plan` hook could not run in the M1 unit. `tasks.md`
already orders each test task before its implementation task and marks tests `[P]`; this list
names the behaviours those tasks cover, per milestone, traced to the requirements. Milestones
M2..M8 add their rows when their units start.

## M1 outer (acceptance)

| ID | Behaviour | Traces to | Task | Where |
|---|---|---|---|---|
| O1 | Add a "Remote over SSH" daemon, start it, it copies the build and reaches `Connected` with version and `user@host:port`, no new listener | US1.1, FR-001, FR-004, FR-005 | T022 | `daemon/tests/ssh_real_endtoend.rs` (off by default, `mise run test-ssh`) |
| O2 | The Daemons form adds, starts and shows state, progress and endpoint for an SSH runtime; `auto_start` applies | FR-001, FR-020 | T020 | `client/features/daemons.rs` reducers |

## M1 inner (unit)

| ID | Behaviour | Traces to | Task | Where |
|---|---|---|---|---|
| I1 | `SshTarget` validation: host not blank/whitespace/leading `-`; user without `@`; port 1..=65535; `InvalidField{field}` | FR-002, FR-010 | T005 | `core/ssh/target.rs` |
| I2 | argv: `BatchMode=yes`, `StrictHostKeyChecking=yes`, `ClearAllForwardings=yes`, `-T`, `--` before host, `LC_ALL=C`, no `-L/-R/-D/-F`, no `accept-new`, per platform | FR-003, FR-004, FR-010, FR-018 | T006 | `core/ssh/argv.rs` |
| I3 | Failure classification and `is_permanent` for every class and reason string | FR-009 | T007 | `core/ssh/classify.rs` |
| I4 | `DaemonRuntime::Ssh` serde round trip, label, startable, additive load without a version bump, failed start is `Unreachable` at once | FR-001, FR-002 | T008 | `core/tests/daemons_ssh.rs` |
| I5 | Provision sequence: probe, copy only when absent, re-probe, permanent failures stop, artefact choice per (client, remote) pair | FR-005, FR-009 | T009 | `core/ssh/provision.rs` |
| I6 | `remote bridge` relays bytes to the daemon socket, starts it when none runs, reuses a running one; `remote prune` | FR-004, FR-005 | T010 | `daemon/src/remote.rs` |
| I7 | Actor: `Dial::Ssh` goes `Starting` to `Connected` with version and endpoint; a permanent failure is `Unreachable{reason}` with no retry | FR-009, FR-017 | T011 | `client/daemon.rs` |
| I8 | `SshStdio` relays bytes both ways | FR-004 | T003 | `core/ssh/stdio.rs` |

## M5 (unit and outer)

| ID | Behaviour | Traces to | Task | Where |
|---|---|---|---|---|
| I20 | `ssh -G` output parsing and the host list from `~/.ssh/config` (wildcards skipped) | FR-002, FR-016 | T056 | `core/ssh/config_hosts.rs` |
| I21 | `SshDuplicate{other}` over resolved (host, user, port); `InvalidField` names the field | FR-010, FR-016 | T057 | `core/tests/daemons_ssh_duplicates.rs` |
| I22 | Picker fills only the host; refused port and duplicate name their field; recent errors; edit; remove with confirmation | US3.1-4, FR-014 | T058 | `client/tests/daemons_feature.rs` |
| I23 | Picker and error rows paint without overflow | FR-002 | T059 | `client/tests/daemons_section_geometry.rs` |
| O3 | Unknown or changed host key, refused auth, missing key, unreachable host each fail with a named reason within the timeout; no key recorded | FR-009, SC-003 | T060 | `daemon/tests/ssh_real_hostkey_auth.rs` (`mise run test-ssh`) |
