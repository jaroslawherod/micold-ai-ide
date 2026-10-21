# Contract: the Jira base URL on the wire and in the settings file

## W1 — `DaemonSettings.jira_base_url: Option<JiraBaseUrl>`

After `github_issues_enabled`. Strict: decoded through `JiraBaseUrl::parse`. The service drops a
stored value that no longer parses when it reads its file, so it never sends one.

## W2 — `ClientMsg::SettingsSet.jira_base_url: Option<Option<JiraBaseUrl>>`

After `github_issues_enabled`. `None`: unchanged. `Some(Some(url))`: set. `Some(None)`: clear. The
service stores it (`Catalog::persist_service_settings`, the on-disk merge copies it as it does
`jira_enabled`) and broadcasts `DaemonSettings` to every connection, so every window adopts it
(FR-012). The service never reads it otherwise.

## W3 — `PROTOCOL_VERSION` 57 → 58

One edit with W1 and W2, a doc paragraph in `protocol/version.rs`, and the `tests/schema_hash.rs`
pin moved in the same commit. `docs/daemon.md` names version 58.

## Settings file

`settings.json` gains `"jira_base_url": "https://example.atlassian.net"` (absent when unset).
A file without it, or with a value that does not parse, reads as unset; the rest of the file loads
(FR-010). `SETTINGS_VERSION` is unchanged (missing-field-defaults contract).

## Tests

- `crates/micold-core/tests/settings_roundtrip.rs`: set, unset, missing, unparsable.
- `crates/micold-core/tests/protocol_roundtrip.rs`: both messages with each of the three values.
- `crates/micold-daemon/tests/jira_base_url_setting.rs` (new, modelled on
  `github_issues_setting.rs`): two connections, a set on one reaches the other; clear; the value
  survives a restart.
