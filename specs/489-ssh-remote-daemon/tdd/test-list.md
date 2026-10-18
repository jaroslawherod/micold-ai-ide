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

## M2-M9 (added by the close unit, T090)

State is `TEST_AFTER` for all rows except where noted: the cycle log records each milestone's
deviation. Rows list the behaviour each ticked test task covers, so every tick has a list item.

| ID | Behaviour | Traces to | Task | Where | State |
|---|---|---|---|---|---|
| I30 | `RemoteLocation` and the namespaced identity path round trip, Windows separators | FR-012, FR-015 | T028 | `core/ssh/remote_path.rs` | TEST_AFTER |
| I31 | `canonicalize_best_effort` keeps a `micold-remote://` identity verbatim | FR-012 | T029 | `core/project.rs` | TEST_AFTER |
| I32 | Add-repository binds to its one daemon; boundary path translation | FR-012, FR-014 | T030 | `client/features/daemons.rs`, `client/shell/daemon_sync.rs` | TEST_AFTER |
| I33 | Remote worktree row shows host path and daemon name | FR-012 | T031 | `client/links.rs`, `client/features/sidebar.rs` | TEST_AFTER |
| O4 | Add repository, worktree, session, input/output and terminal command through a real sshd | US1.2-4 | T032 | `daemon/tests/ssh_real_endtoend.rs` | TEST_AFTER |
| I40 | `Backoff` 1, 2, 4, 8, 15, 15 s and reset | FR-006 | T039 | `core/ssh/reconnect.rs` | TEST_AFTER |
| I41 | State machine: `Reconnecting`, `Attempts`, `GiveUp`, `Reconnect` | FR-006, FR-007, FR-008 | T040 | `core/daemons.rs` | TEST_AFTER |
| I42 | Actor: drop, backoff, explicit reconnect bumps the epoch, no second connect | FR-006, FR-008 | T041 | `client/daemon.rs` | TEST_AFTER |
| I43 | `session_lost_label`: `disconnected` while reconnecting, `lost to daemon <name>` after a restart | FR-019 | T042, T085 | `client/links.rs`, `client/tests/worktree_unavailable_label.rs` | TEST_AFTER (T085 red pass after the fact) |
| O5 | Drop, sessions keep running, reconnect within 30 s, stop trying then Reconnect | FR-006, FR-007, SC-002 | T044 | `daemon/tests/ssh_real_reconnect.rs` | TEST_AFTER |
| I44 | A user Reconnect restarts the actor (epoch changes) | FR-008 | T087 | `core/daemons.rs` tests | red pass after the fact |
| I50 | A remote daemon in every non-connected state leaves a local daemon untouched | FR-013 | T051 | `client/daemon.rs` | characterization |
| I51 | App start never awaits a remote dial | FR-020 | T052 | `core/daemons.rs` (`at_launch`) | characterization |
| O6 | Two bridges on one daemon see the same sessions; local unaffected while remote is cut | FR-013, FR-021, SC-004 | T053 | `daemon/tests/ssh_real_concurrent.rs` | characterization |
| I60 | Mismatch explanation for both directions and `WayOut` | FR-011, SC-005 | T066 | `core/ssh/mismatch.rs` | LIKELY (stubs red) |
| I61 | No session starts on a mismatched daemon; the state text names both versions | FR-011 | T067 | `client/features/daemons.rs` | TEST_AFTER |
| O7 | Mismatch in both directions through a real sshd | FR-011 | T069 | `daemon/tests/ssh_real_mismatch.rs` | TEST_AFTER |
| I70 | `RemoteStatus` parsing; unknown `status_version` is `Unknown` | FR-011 | T073 | `core/ssh/status.rs` | TEST_AFTER |
| I71 | Status writer; `remote stop` refuses unless zero sessions are proven | FR-011 | T074 | `daemon/src/runtime_status.rs`, `daemon/src/remote.rs` | TEST_AFTER (`decide_stop` red pass) |
| I72 | `ReplaceOffer` states drive the Replace action | FR-011 | T075 | `client/features/daemons.rs` | TEST_AFTER |
| O8 | Replace offered with no sessions, withheld with sessions | FR-011 | T076 | `daemon/tests/ssh_real_mismatch.rs` | TEST_AFTER |
| I80 | macOS bundle and Windows installer ship the Linux remote-daemon artefact | FR-018 | T082 | `scripts/tests/remote-daemon-bundle.test.sh` | TEST_AFTER |
