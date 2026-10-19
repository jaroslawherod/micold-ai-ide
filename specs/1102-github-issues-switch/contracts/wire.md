# Wire and MCP contract: GitHub issues switch (#1102)

## W1. `DaemonSettings` (daemon -> client, in `SettingsChanged` and the settings reply)
Add `pub github_issues_enabled: bool` after `jira_connections`. Doc: "Whether the GitHub issues
integration is on (feature 1102, FR-001). Service-owned so every window follows one switch
(FR-008)." Serde default `true` for a file or frame without it.

## W2. `ClientMsg::SettingsSet` (client -> daemon)
Add `github_issues_enabled: Option<bool>` after `jira_connection`; `None` leaves the value
unchanged. The daemon's `SettingsSet` arm runs `state.set_github_issues_enabled(on)` in the same
`and_then` chain as `set_jira_enabled`, one atomic persist.

## W3. Version
`PROTOCOL_VERSION` 55 -> 56, one edit with W1 and W2, doc paragraph "And 55 -> 56 for feature
1102's GitHub issues switch (contracts/wire.md)". `tests/schema_hash.rs` pin updated in the same
commit.

## W4. Settings file
`github_issues_enabled` key, boolean. Absent -> `true`. No `settings_version` change.

## M1. MCP `create_worktree` with `github_issue` while the switch is off
- Result: tool error, `invalid_input`, text: "GitHub issues are turned off in Settings (GitHub). Give a branch, or type, ticket and name instead."
- Side effects: none. No remote read, no `gh` call, no feature 837 detail read, no worktree.
- Without `github_issue`: unchanged.
- Tool description gains: "Fails while GitHub issues are turned off in Settings."
