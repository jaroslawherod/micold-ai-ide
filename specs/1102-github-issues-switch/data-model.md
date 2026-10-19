# Data model: GitHub issues switch (#1102)

| Entity | Field | Type | Default | Owner |
|---|---|---|---|---|
| `micold_core::settings::Settings` / `SettingsFile` | `github_issues_enabled` | `bool` | `true` (missing, unknown, corrupt file) | daemon, persisted in the settings file; `settings_version` does not move (additive, defaulted) |
| `protocol::messages::DaemonSettings` | `github_issues_enabled` | `bool` | n/a (always sent) | service; broadcast in `SettingsChanged` |
| `ClientMsg::SettingsSet` | `github_issues_enabled` | `Option<bool>` | `None` = leave unchanged | client request |
| `features::settings::State` | `github_issues_enabled` | `bool` | `true` | client, last value the service reported |
| `SettingsDraft.github` (`GithubDraft`) / `ValidSettings` | `issues_enabled` / `github_issues_enabled` | `bool` | seeded from settings in force | Settings page |
| `worktree_form::Form` | `github_issues_enabled` | `bool` | seeded from `State.settings` at open | form |

## Transitions

- Toggle on or off in a window: `SettingsSet { github_issues_enabled: Some(v) }` -> daemon persists atomically -> `SettingsChanged` to all windows -> `adopt_daemon_settings` -> `FormMsg::GithubIssuesChanged(v)`.
- Form, on change `v: bool`: if equal to the form's flag, nothing. Else set flag; `resolve_provider()`; if the provider was or becomes GitHub: `reset_issues()`, `source = New` when it was `Issue`, `issue_request_seq += 1`.
- Invariant: while `github_issues_enabled == false`, `github_repo()` is `None`, hence no GitHub issue request is built and `provider` is never GitHub with a source of `Issue`.
- Unchanged: `workspace.issue_sources` (remembered provider per project) is not written by the switch.
