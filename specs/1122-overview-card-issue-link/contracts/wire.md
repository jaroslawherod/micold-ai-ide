# Contract: the per-project Jira base URL on the wire and in the settings file

Per project (D4, owner's answer), on the route feature 533 uses for each project's Jira connection
(`jira_connections`, `SettingsSet.jira_connection`), keyed the same way
(`jira::connection_key(project)`).

## W1 — `DaemonSettings.jira_base_urls: BTreeMap<String, JiraBaseUrl>`

After `github_issues_enabled`. Every project's value, by key; a project with no value has no entry.
Strict: each value is decoded through `JiraBaseUrl::parse`. The service drops an entry that no
longer parses when it reads its file, so it never sends one.

## W2 — `ClientMsg::SettingsSet.jira_base_url: Option<Box<JiraBaseUrlChange>>`

After `github_issues_enabled`. Boxed so `ClientMsg` stays small (clippy `large_enum_variant`);
postcard encodes a `Box<T>` as `T`, so the layout on the wire is that of
`Option<JiraBaseUrlChange>`. `JiraBaseUrlChange { project: String, url: Option<JiraBaseUrl> }`, in
`micold_core::settings` beside `JiraConnectionChange`, with `apply(&mut BTreeMap)`. `None`: no
project changes. `Some { url: Some(u) }`: set that project's value. `Some { url: None }`: remove it.
The service merges the one change into the map on disk inside its write
(`Catalog::persist_service_settings_with`, as it merges `jira_connection`; the field is not copied
from the service's in-memory snapshot), so two windows saving different projects never overwrite
each other, and broadcasts `DaemonSettings` to every connection, so every window adopts it (FR-012).
The service never reads it otherwise.

## W3 — `PROTOCOL_VERSION` 59 → 60

One edit with W1 and W2, a doc paragraph in `protocol/version.rs`, and the `tests/schema_hash.rs`
pin moved in the same commit. `docs/daemon.md` names version 60. Planned as 57 → 58; `main` took 58
and 59 (feature 942) first.

## Settings file

`settings.json` gains `"jira_base_urls": { "<project key>": "https://example.atlassian.net" }`
(absent when empty). A file without it reads as empty; an entry whose value does not parse is
dropped (that project reads as unset) and the rest of the file loads (FR-010).
`SETTINGS_VERSION` is unchanged (missing-field-defaults contract).

## Tests

- `crates/micold-core/tests/settings_roundtrip.rs`: two projects set, one unset, missing map, one
  unparsable entry beside a valid one; `JiraBaseUrlChange::apply` set and remove.
- `crates/micold-core/tests/protocol_roundtrip.rs`: `DaemonSettings` with an empty and a two-entry
  map; `SettingsSet` with `None`, a set and a clear.
- `crates/micold-daemon/tests/jira_base_url_setting.rs` (new, modelled on `jira_setting.rs`'s
  per-project connection tests): two connections, a set for one project on one reaches the other;
  two clients saving different projects both survive; clear removes only that project's entry; the
  values survive a restart; an unparsable stored entry is dropped and never sent.
