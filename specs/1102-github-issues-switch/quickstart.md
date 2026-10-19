# Quickstart: GitHub issues switch (#1102)

## Part A: automated
`mise run test-core`, then `mise run gate`. Key tests: `settings_roundtrip` (default true),
`protocol_roundtrip`, `schema_hash`, daemon `github_issues_setting` and `mcp_create_worktree`,
client `settings_sections`, `features_settings`, form and issues reducer tests.

## Part B: visual pass (needs eyes at a display)
1. Settings > GitHub: the checkbox sits under "Show pull request status on worktrees", above the label mapping; its note reads as one clear line; checked on a fresh profile.
2. Turn it off, save, open New worktree on a project with a GitHub remote: no Issue chip (or a disabled chip with the caption "GitHub issues are turned off in Settings."); no GitHub / Jira choice.
3. With Jira connected: Jira is the only source, no choice shown.
4. Two windows: turn off in one, the other's open form drops the list without reopening.
5. Turn on again: the form offers GitHub issues and loads them on choosing the chip.
