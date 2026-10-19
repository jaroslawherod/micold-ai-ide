# Implementation Plan: Allow the GitHub issues integration to be disabled

**Branch**: `feat/1102_allow-the-github-issues-integration-to-be-disabled` | **Date**: 2026-10-19 | **Spec**: [spec.md](spec.md)

## Summary

Add one app-wide, persisted, service-owned boolean `github_issues_enabled` (default **on**) that
mirrors `jira_enabled` end to end (settings file, `DaemonSettings`, `ClientMsg::SettingsSet`, the
daemon catalog, every window's `adopt_daemon_settings`, the Settings draft) and gate three places:
the New worktree form (no GitHub provider, source or request), the client's issue-request starters
(`shell/issues.rs`), and the daemon's MCP `create_worktree` with `github_issue` (error, no GitHub
request). The switch is a checkbox in the Settings **GitHub** section beside the pull request status
one. Unlike `jira_enabled` and `pr_status_enabled`, which default off, this one defaults on: a
serde default function (`default_true`, as `default_desktop_notifications`) covers a missing entry
(FR-007); the wire change bumps `PROTOCOL_VERSION` 55 → 56.

## Technical Context

**Language/Version**: Rust (workspace edition), iced GUI.
**Primary Dependencies**: existing only: `serde`, `bincode`-style wire codec in `micold-core::protocol`, `iced`. No new crate.
**Storage**: the daemon's settings file (`micold-core/src/settings.rs` `SettingsFile`), service-owned field written through `Catalog::persist_service_settings`.
**Testing**: `cargo test` via `mise run test-core`, `mise run gate`; core unit and round-trip tests, daemon integration tests (`crates/micold-daemon/tests`), client reducer tests (`crates/micold-client/tests`, `main_tests.rs`).
**Target Platform**: Linux, macOS, Windows (no platform code).
**Project Type**: desktop app (client) plus session daemon, shared `micold-core`.
**Performance Goals**: none; the switch removes requests.
**Constraints**: no GitHub issue request while off (FR-002); older settings files read as on; wire hash moves, so the version moves once.
**Scale/Scope**: one boolean; about 12 source files touched, all by copying the `jira_enabled` path.

## Constitution Check

| Principle | Status |
|---|---|
| I. Test-First | PASS: every change below has its failing test listed in [quickstart.md](quickstart.md) / the test list before code; tasks order tests first. |
| II. Multi-Session | PASS: one service-owned value; every window and daemon adopts the latest write (FR-008, same path as `jira_enabled`); no per-session state. |
| III. Worktree Integration | PASS: worktrees already created from an issue are untouched; creation without an issue is unchanged (FR-004). |
| IV. Local-First | PASS: the setting is a local file entry; the feature only removes network calls; nothing new leaves the device. |
| V. Rust + iced | PASS: a `bool` field plus a form-level derived availability; the form's "GitHub is the provider but off" state is made unrepresentable by `github_repo()` returning `None` while off, not by scattered checks. |
| VI. Cross-Platform | PASS: no platform-specific code; CI covers all three. |
| VII. Documentation | PASS: `docs/user-guide/settings.md`, `docs/user-guide/worktrees-and-sessions.md`, `docs/daemon.md` (settings list) and the MCP tool description updated in the same change. |
| VIII. Reusable UI | PASS: reuses `Checkbox` + `field_note` exactly as the PR status switch; no new widget. |

Complexity Tracking: none.

## Project Structure

### Documentation (this feature)

```text
specs/1102-github-issues-switch/
├── spec.md  plan.md  research.md  data-model.md  quickstart.md
├── contracts/wire.md     # the SettingsSet / DaemonSettings delta and the MCP error
└── tasks.md              # next unit
```

### Source code (all paths exist; "new" marks additions)

```text
crates/micold-core/src/
├── settings.rs              # Settings + SettingsFile: github_issues_enabled (new, default true)
├── protocol/messages.rs     # DaemonSettings::github_issues_enabled; ClientMsg::SettingsSet::github_issues_enabled
├── protocol/version.rs      # PROTOCOL_VERSION 55 -> 56 + doc paragraph
├── kube/fake_host.rs        # DaemonSettings literal gains the field
└── mcp/tools.rs             # create_worktree description mentions the switch
crates/micold-daemon/src/
├── catalog.rs               # set_github_issues_enabled; service-write copy (beside jira_enabled, l.239/399/531)
├── state.rs                 # DaemonState::set_github_issues_enabled, github_issues_enabled() getter
├── server.rs                # SettingsSet arm applies the field
└── mcp/tools.rs             # resolve_naming: refuse github_issue while off, before remote_list/lookup
crates/micold-client/src/
├── features/settings.rs     # State.github_issues_enabled (default true), SettingsDraft/GithubDraft/ValidSettings, Msg::GithubIssuesToggled, baseline seeding
├── features/worktree_form.rs# form flag + github_repo() gate, resolve_provider, FormMsg::GithubIssuesChanged, captions
├── ui/settings/github.rs    # the checkbox; SETTINGS list entry
├── ui/focus/FieldId         # FieldId::SettingsGithubIssues (new variant, beside SettingsPrStatus)
├── shell/persist.rs         # settings_in_force, set!/changed! lists, SettingsSet builder, apply of an unconnected save
├── shell/daemon_sync.rs     # adopt_daemon_settings + refresh_open_settings
├── shell/startup.rs         # loaded settings -> core.settings
└── shell/issues.rs          # sync_github_issues_availability; the starters refuse while off
docs/user-guide/settings.md, worktrees-and-sessions.md; docs/daemon.md
```

**Structure Decision**: no new module. The value rides the existing service-settings path, so the
hand-written lists that name `jira_enabled` (found with `grep -n jira_enabled`) each get a sibling line.

## Design

### D1. Setting and wire (FR-001, FR-007, FR-008)

- `Settings` and `SettingsFile`: `#[serde(default = "default_true")] github_issues_enabled: bool`; `Settings::default()` sets `true`. A missing, unknown or corrupt file yields `true` (spec edge cases; the corrupt-file path already returns `Settings::default()` and does not rewrite).
- `DaemonSettings::github_issues_enabled: bool` (`#[serde(default = "default_true")]` as the neighbouring fields do where applicable) and `ClientMsg::SettingsSet::github_issues_enabled: Option<bool>`, placed after `jira_connection`. Service-owned: `Catalog::persist_service_settings` and the on-disk merge copy it, as for `jira_enabled`, so the latest write wins on every daemon/window (multi-session edge case).
- Wire contract and version bump: [contracts/wire.md](contracts/wire.md). `tests/schema_hash.rs` pins the hash; the bump and the pin move in one commit.

### D2. Settings page (FR-006)

`GithubDraft` gains `issues_enabled`; `Msg::GithubIssuesToggled(bool)` edits it; `ValidSettings` and `settings_in_force`/`changed!` carry it like `pr_status_enabled`. The `github.rs` page adds, above the label mapping and right after the pull request status checkbox, `Checkbox::new("Offer GitHub issues when creating worktrees", …)` with `field_note` text "Reads open issues of the project's GitHub repository to start worktrees from. Off: no GitHub issue is requested or offered." (wording follows the Jira switch's description). `SETTINGS` gains `("github_issues_enabled", "GithubIssuesToggled")`, which `tests/settings_sections.rs` checks. `GITHUB_ISSUES_INTRO` is extended by one clause only if the visual pass shows the intro contradicting the switch.

### D3. Client gate (FR-002 to FR-005, FR-009)

- `State.settings.github_issues_enabled` (default **true**, hand `Default`) is what the service last reported; `adopt_daemon_settings` sets it and calls `sync_github_issues_availability`, next to `sync_jira_availability`; `startup.rs` seeds it; an unconnected save applies it as `jira_enabled` does.
- The form carries `github_issues_enabled: bool` (set in `open_form` from `state.settings`, like `jira`). `github_repo()` returns `None` while it is false. Every GitHub issue starter in `shell/issues.rs` (`start_issue_load`, `start_issue_search`, `start_issue_descriptions`, `start_issue_details`) already resolves its repo through `github_repo()`, so none can fire; `resolve_provider` treats "off" like `GithubAvailability::Unavailable` so a connected Jira becomes the sole provider (AS 2) and no provider choice shows (`offers_issue_provider_choice` needs `github_repo()`), and with Jira off `issue_source_ready()` is false (AS 3). The caption (`source_caption`) says "GitHub issues are turned off in Settings." when GitHub would otherwise be the provider.
- `FormMsg::GithubIssuesChanged(bool)`, handled like `jira_settings_changed`: when the flag changes, re-resolve the provider; if the provider was or becomes GitHub, `reset_issues()`, leave `BranchSource::Issue` for `New`, bump `issue_request_seq` so an in-flight list, search or description/detail result is stale and discarded (AS 4, FR-005). Turning on re-resolves and the form offers GitHub; the existing source-chosen path loads on demand.
- The remembered per-project provider (`issue_sources`) is neither read nor cleared while off (edge case): `resolve_provider` ignores `remembered_provider == Github` implicitly because GitHub is the default branch; only a remembered Jira choice is consulted, and only when `offers_issue_provider_choice`.
- `RemoteList` is still asked when the form opens: it is a remote read, not a GitHub issue request, and PR status and branch listing use it (FR-009). It decides `GithubAvailability`; the switch is a separate flag so a later turn-on needs no second read.

### D4. MCP `create_worktree` (FR-010)

`resolve_naming` (daemon `mcp/tools.rs`) returns `OpError::invalid_input("GitHub issues are turned off in Settings (GitHub). Give branch, or type, ticket and name instead.")` as its first act for `github_issue: Some(_)` when `state.github_issues_enabled()` is false, before `remote_list`, `issue_lookup` or any detail read; feature 837's relationship reads are inside that path and so never run. Requests without `github_issue` take the early `Ok` return unchanged. The tool description in `micold-core/src/mcp/tools.rs` gains one sentence; `tests/mcp_tools_catalog.rs` pins the catalog text and moves with it.

## Test strategy (layer per requirement)

| Requirement | Layer |
|---|---|
| FR-001, FR-007 | core unit: `settings_roundtrip.rs` (default true, missing entry, corrupt file), `protocol_roundtrip.rs`, `schema_hash.rs` |
| FR-008 | daemon integration `github_issues_setting.rs` (modelled on `jira_setting.rs`): two connections, last write wins, restart keeps it |
| FR-002 to FR-005 | client reducer tests on `worktree_form` (provider/source/caption/seq discard) and `shell/issues` tests with a fake `IssueTooling` asserting zero calls |
| FR-006 | `tests/settings_sections.rs`, `tests/features_settings.rs`; visual pass quickstart §B |
| FR-009 | client test: PR status and `RemoteList` still sent while off |
| FR-010 | daemon `tests/mcp_create_worktree.rs`: error text, zero lookup calls on the fake; call without `github_issue` unchanged |
| SC-003 | core: pre-feature settings fixture reads on; client: form with default settings loads issues |
| Geometry gates | none: no layout change beyond one more checkbox row; quickstart §B looks at it |
