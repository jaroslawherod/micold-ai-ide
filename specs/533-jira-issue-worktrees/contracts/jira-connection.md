# Contract: the switch, the connection and the token

**Feature**: [spec.md](../spec.md) | Research [R3](../research.md#r3--where-the-switch-and-the-connection-live), [R4](../research.md#r4--the-token-an-owner-only-file-beside-settings-never-on-the-wire)

## 1. Settings fields

`Settings.jira_enabled: bool` (`#[serde(default)]`, false) and
`Settings.jira_connections: BTreeMap<String, JiraConnection>` (`#[serde(default)]`; an entry that
no longer validates is dropped on read, the rest kept). Both in `StoredSettings` and its
`from_settings`/`into_settings`, the `DaemonSettings`-style wire struct and the client `persist.rs`
`set!` list and its exhaustive destructure (the compiler lists every place). `SETTINGS_VERSION`
unchanged. Wire: `SettingsSet { jira_enabled: Option<bool>, jira_connection: Option<JiraConnectionChange> }`
with `JiraConnectionChange { project: String, connection: Option<JiraConnection> }` (`None`
removes), so two windows saving different projects never overwrite each other; the daemon applies
it to its map and merges it into the on-disk map inside the same `update_reporting` closure
(the Jira setter only; `persist_service_settings` does not copy `jira_connections`), and
`SettingsChanged.settings` carries `jira_enabled` and the whole `jira_connections`;
`PROTOCOL_VERSION` 42. Daemon test: a save of another field keeps both fields.

## 2. Validation (`JiraConnection::parse`)

| Field | Accepted | Refused with |
|---|---|---|
| site | `https://host[:port][/ignored]` | "Enter the https address of your Jira site, e.g. https://acme.atlassian.net." |
| project key | `[A-Z][A-Z0-9_]{1,9}` (input upper-cased and trimmed) | "Enter the project key, e.g. ABC." |
| email | one `@`, host with a dot | "Enter the email of your Atlassian account." |
| token | non-empty (or already stored) | "Enter an API token." |

## 3. Token file

`<data_dir>/jira/<store::project_id(path)>.token`; `owner_only::write` (dir 0700, file 0600, DACL on
Windows, temp-then-rename). Read: `std::fs::read_to_string` into `ApiToken` at the moment of a
request, by `shell/jira.rs` only. Remove: `remove_file`, ignoring not-found. The token never
enters `Settings`, any protocol message, `App` state beyond the in-flight `Task`, a log line
(`log_line` helpers take no token), or `Debug` output.

## 4. Operations (client shell, `shell/jira.rs`)

| Operation | Steps (order matters) |
|---|---|
| Save connection | validate; write token file (when a new token was typed); send `SettingsSet{jira_connection: Some(change)}`; on echo the Settings page and open dialogs re-derive availability. |
| Remove connection | delete token file; send `SettingsSet{jira_connection: Some(removal)}`; bump `issue_request_seq`. |
| Switch off, keep | `SettingsSet{jira_enabled:false}` only. |
| Switch off, remove | remove connection of the open project, then the switch (the prompt names the project). |
| Probe token | `Path::exists` on Settings open, `SettingsChanged`, dialog open. |

## 5. Gate tests

`jira_is_requested_only_on_named_events.rs`: the Jira source is called from exactly the load,
retry and search-due arms and only after `jira_visible` and a connection are checked in the same
function (the pattern of 034's gate). `jira_token_never_leaks.rs`: with a sentinel token, run
save, load, each failure kind and remove through fakes and scan the settings file, the wire
frames, `Debug` of every type reachable from `App`, and the log buffer.
