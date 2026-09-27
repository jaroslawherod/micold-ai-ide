# Contract: When the client asks which AI CLIs a directory has

The wire contract is unchanged:
`ClientMsg::AiCliAvailabilityRequest { req, cwd }` →
`DaemonMsg::AiCliAvailability { req, available }` (`crates/micold-core/src/protocol/messages.rs`,
`PROTOCOL_VERSION` 15, answered for `cwd`'s spawn environment by 029 FR-003b). This contract fixes
the **client's** side: which events send a request, for which key, and what happens to the answer.
Research: [R1–R11](../research.md).

## C1 — The closed list of askers (FR-004, SC-003)

| # | Event | Code site | Asks for |
|---|---|---|---|
| A1 | Connected (first connect, and every reconnect) | `shell/daemon_sync.rs` `on_connected` | clear all (FR-011); then `Home`, then every wanted directory |
| A2 | Settings opened | `shell/persist.rs` `on_settings_opened` | `Home` |
| A3 | A row's start list opened (chevron, or primary press with the default missing) | `main.rs` `SessionMsg::StartMenuOpened` arm | that row's directory (refresh) |
| A4 | Project opened / reopened / switched to | `shell/workspace.rs` `open_verified_project`, `on_known_project_reopened` | sync: every wanted directory not held or in flight |
| A5 | Worktree list replaced (created, discovered, deleted, `Missing` ↔ valid) | callers of `State::set_worktrees`: `CatalogChanged` reconcile, `features::worktree::loaded` | sync |
| A6 | Project forgotten | `ProjectMsg::ForgetConfirmed` arm | sync (prunes; asks nothing new) |
| A7 | Environment-include settings changed | `DaemonMsg::SettingsChanged` arm, only when an env-include field differs | `Home` and every wanted directory (refresh, held answers kept until replaced) |

No other site sends the request. Nothing under `ui/` sends it, directly or through a message a view
emits on draw. No `Subscription` and no timer sends it. Enforced by
`crates/micold-client/tests/availability_is_asked_only_on_named_events.rs`.

"Sync" (A4–A6) = `retain(wanted)` then ask each directory in `unasked(wanted)`. It is idempotent, so
running it on an event that did not change the wanted set sends nothing.

## C2 — Filing an answer (FR-002, FR-009)

An answer is filed under the key its request named, and only if that request is still the newest for
the key. An answer to a request from a previous connection, or for a directory pruned since, is
dropped. Answers for different keys never replace one another, whatever order they arrive in.

## C3 — Reading an answer (FR-001, FR-005, FR-008)

| Reader | Key read |
|---|---|
| Sidebar row: chevron presence, primary press | the row's directory, falling back to `Home` while not held |
| Start list items and its missing-default notice | the list's directory, falling back to `Home` |
| Settings default select and its "not installed" sentence | `Home` only |

## C4 — What stays the same

- The affordance's look (split button, chevron, list): unchanged.
- The launch-time check (026 FR-010): unchanged, still the backstop for a stale answer.
- The daemon: no change.
