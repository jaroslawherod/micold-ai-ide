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

### Part B result (2026-10-19, close unit)
Run on a private headless sway (lavapipe), private data dir, fixture repo with a GitHub remote. Evidence: `visual/`.
1. PASS (`b1-settings-github-on.png`). 2. PASS (`b2-form-off.png`). 3. PASS, Jira connection seeded in settings (`b3-form-jira-only.png`).
4. PASS, two client processes on one daemon; the other form updated in about 3 s (`b4-two-windows-after-off.png`).
5. PASS for offering (`b5-form-after-on.png`); the issue list itself not driven (no `gh` login or network); choosing the chip started a load (`b5-github-chip-chosen.png`).
