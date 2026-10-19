---
description: "Task list for the GitHub issues switch (#1102)"
---

# Tasks: Allow the GitHub issues integration to be disabled

**Input**: `specs/1102-github-issues-switch/` (spec.md, plan.md, research.md, data-model.md, contracts/wire.md, quickstart.md)
**Tests**: mandatory and first (Constitution I): in every phase the test tasks precede the code they cover and must be seen failing.
**Documentation**: the user-guide task sits in the milestone that ships the behaviour (Constitution VII).
**Pattern**: every code task copies the `jira_enabled` path (`grep -n jira_enabled`), except that this switch defaults **on**.

## Format: `[ID] [P?] [Story] Description`

## Phase 1: Foundational (wire, setting, daemon, client plumbing)

- [ ] T001 [P] [US2] Test: `Settings`/`SettingsFile` default `github_issues_enabled` true, a missing entry and a corrupt file read true (file not rewritten), a false value round-trips, in `crates/micold-core/tests/settings_roundtrip.rs`
- [ ] T002 [P] [US3] Test: `DaemonSettings::github_issues_enabled` and `ClientMsg::SettingsSet::github_issues_enabled: Option<bool>` round-trip in `crates/micold-core/tests/protocol_roundtrip.rs`
- [ ] T003 [US2] Add `github_issues_enabled` (`#[serde(default = "default_true")]`, `Settings::default()` true) to `Settings` and `SettingsFile` in `crates/micold-core/src/settings.rs`
- [ ] T004 [US3] In `crates/micold-core/src/protocol/messages.rs` add the two wire fields (W1, W2); also bump `PROTOCOL_VERSION` 55 -> 56 with its doc paragraph in `crates/micold-core/src/protocol/version.rs`, move the pin in `crates/micold-core/tests/schema_hash.rs`, add the field to the `DaemonSettings` literal in `crates/micold-core/src/kube/fake_host.rs`
- [ ] T005 [P] [US3] Test: daemon integration `crates/micold-daemon/tests/github_issues_setting.rs` (model on `jira_setting.rs`): default on, `SettingsSet` persists and broadcasts, two connections last write wins, restart keeps it
- [ ] T006 [US3] `Catalog::set_github_issues_enabled` and the service-write copy beside `jira_enabled` in `crates/micold-daemon/src/catalog.rs`; `DaemonState::set_github_issues_enabled` and `github_issues_enabled()` in `crates/micold-daemon/src/state.rs`; the `SettingsSet` arm in `crates/micold-daemon/src/server.rs` (same `and_then` chain as `set_jira_enabled`)
- [ ] T007 [US3] (after T004, T006) Add the field to every `DaemonSettings`/`SettingsSet` test literal: `crates/micold-client/src/main_tests.rs`, `crates/micold-client/tests/jira_token_never_leaks.rs` and any literal under `crates/micold-client/tests/support` (a no-op if none)
- [ ] T008 [P] [US3] Test: `State.github_issues_enabled` defaults true; `GithubDraft.issues_enabled` seeded from settings in force; `Msg::GithubIssuesToggled` edits the draft; `ValidSettings` carries it; baseline seeding; a failed save keeps the new value for the session and the next save retries (spec edge case), in `crates/micold-client/tests/features_settings.rs` (and the reducer tests in `crates/micold-client/src/features/settings.rs` where `pr_status_enabled` is covered)
- [ ] T009 [US3] `State.github_issues_enabled` (hand `Default` true), `GithubDraft.issues_enabled`, `ValidSettings`, `Msg::GithubIssuesToggled`, baseline seeding in `crates/micold-client/src/features/settings.rs`
- [ ] T010 [US3] Plumb the value: `settings_in_force`, the `set!`/`changed!` lists, the `SettingsSet` builder and the unconnected-save apply in `crates/micold-client/src/shell/persist.rs`; seed `core.settings` in `crates/micold-client/src/shell/startup.rs`; in `crates/micold-client/src/shell/daemon_sync.rs` set `core.settings.github_issues_enabled` in `adopt_daemon_settings` and let an open Settings draft follow the service value unless the user edited it (`refresh_open_settings`, same untouched rule as `pr_status::enabled_changed`). `shell/pr_status.rs` needs no sibling line: that function is the PR-status switch's own follow-up (changes view, PR task) and the new field has no such consumers

## Phase 2: User Story 1 + 2 - off hides GitHub issues, default unchanged (P1) 🎯 MVP

**Goal**: with the switch off, a form opened on a GitHub-remote project requests and offers no GitHub issue; default on is exactly as before; the user can flip it in Settings.

- [ ] T011 [P] [US1] Test (reducer, `crates/micold-client/tests/features_worktree_form.rs`): form opened with the flag off: `github_repo()` is `None`, no GitHub provider or source, no provider choice with Jira connected (Jira sole provider, AS 2), `issue_source_ready()` false with Jira off (AS 3) with the caption "GitHub issues are turned off in Settings.", creating without an issue still works; with the flag on (default) the form behaves as before (US2 AS 2); `RemoteList` and PR status still sent (FR-009)
- [ ] T012 [P] [US1] Test (`crates/micold-client/tests/issues_are_requested_only_on_named_events.rs` or the `shell/issues` tests): with the flag off, `start_issue_load`, `start_issue_search`, `start_issue_descriptions` and `start_issue_details` make zero calls on a fake `IssueTooling` (FR-002, SC-001)
- [ ] T013 [US1] Form flag `github_issues_enabled` set in `open_form` from `state.settings`, `github_repo()` gated on it, `resolve_provider` treating off like `GithubAvailability::Unavailable`, the `source_caption` text, in `crates/micold-client/src/features/worktree_form.rs`; trace the four starters in `crates/micold-client/src/shell/issues.rs` (`start_issue_load`, `start_issue_search`, `start_issue_descriptions`, `start_issue_details`): any that does not resolve its repo through `github_repo()` gets the off check; T012 must go red to green
- [ ] T014 [P] [US1] Test: `SETTINGS` lists `("github_issues_enabled", "GithubIssuesToggled")` and the checkbox renders under the PR status one, in `crates/micold-client/tests/settings_sections.rs`
- [ ] T015 [US1] Checkbox "Offer GitHub issues when creating worktrees" with its `field_note` right after the PR status checkbox and the `SETTINGS` entry in `crates/micold-client/src/ui/settings/github.rs`; `FieldId::SettingsGithubIssues` in `crates/micold-client/src/features/window.rs`
- [ ] T016 [US1] User guide: the GitHub issues switch in `docs/user-guide/settings.md` and the form's behaviour with it off in `docs/user-guide/worktrees-and-sessions.md`; add the setting to the settings list in `docs/daemon.md`

## Phase 3: User Story 3 + US1 AS4 - live in every window (P2)

**Goal**: flipping the switch, in any window or daemon, changes every open form at once; stale GitHub results are discarded.

- [ ] T017 [P] [US1] Test (reducer, `features_worktree_form.rs`): `FormMsg::GithubIssuesChanged(false)` with GitHub as provider drops the list, clears a picked issue, returns `BranchSource::Issue` to `New`, bumps `issue_request_seq` so an in-flight list, search, description or detail result is discarded (AS 4, edge cases); `true` re-offers GitHub and loads on choosing the chip (US3 AS 3); `workspace.issue_sources` untouched; unchanged value is a no-op
- [ ] T018 [US1] `FormMsg::GithubIssuesChanged(bool)` handled like `jira_settings_changed` in `crates/micold-client/src/features/worktree_form.rs`
- [ ] T019 [P] [US3] Test (`crates/micold-client/src/main_tests.rs` or `tests/`): `adopt_daemon_settings` with a changed value updates `core.settings` and every window's open form; an unconnected save applies it; a second window's open form drops GitHub issues without reopening (US3 AS 1, SC-004)
- [ ] T020 [US3] `sync_github_issues_availability` in `crates/micold-client/src/shell/issues.rs`, called from `adopt_daemon_settings` in `crates/micold-client/src/shell/daemon_sync.rs` and from the unconnected-save apply in `crates/micold-client/src/shell/persist.rs`, beside `sync_jira_availability`
- [ ] T021 [US3] User guide: a sentence in `docs/user-guide/worktrees-and-sessions.md` that an open form follows the switch at once, and that the setting is shared by all windows, in `docs/user-guide/settings.md`

## Phase 4: MCP `create_worktree` refuses `github_issue` while off (FR-010)

**Goal**: an agent cannot make a GitHub issue request through the MCP tool while the switch is off.

- [ ] T022 [P] [US1] Test: `crates/micold-daemon/tests/mcp_create_worktree.rs`: with the switch off, `github_issue` fails `invalid_input` with the contract text (M1), zero `remote_list`/`issue_lookup`/837 detail calls on the fake, no worktree; without `github_issue` unchanged; switch on unchanged. `crates/micold-core/tests/mcp_tools_catalog.rs` pin moves with T024
- [ ] T023 [US1] `resolve_naming` returns the error first for `github_issue: Some(_)` while `state.github_issues_enabled()` is false, in `crates/micold-daemon/src/mcp/tools.rs`
- [ ] T024 [US1] Tool description sentence "Fails while GitHub issues are turned off in Settings." in `crates/micold-core/src/mcp/tools.rs` and the pin in `crates/micold-core/tests/mcp_tools_catalog.rs`
- [ ] T025 [US1] Document the refusal in the MCP section of `docs/daemon.md` and `docs/user-guide/settings.md`

## Dependencies and order

Phase 1 -> Phase 2 -> Phase 3; Phase 4 needs only Phase 1 (T006). Within a phase tests first; `[P]` tasks touch disjoint files. T001 and T002 fail to compile until T003/T004; that is the red step.

## Implementation Strategy

MVP is M1: after it, off hides GitHub issues in forms opened afterwards and the default is unchanged. M2 adds the live path; M3 closes the MCP route. Quickstart Part B (visual pass) and task ticks are left to the close unit (a Polish that changes no code).

## Milestones

Each milestone merges to `main` on its own, through one PR (speckit-autopilot).

### M1 — Switch, wire and form gate 🎯 MVP

- **Tasks**: T001–T016
- **Deliverable**: Settings > GitHub has a persisted "Offer GitHub issues" switch, on by default; with it off, a New worktree form opened on a GitHub-remote project makes no GitHub issue request and offers no GitHub source (Jira stays usable).
- **Satisfies**: US1 acceptance scenarios 1–3; US2 acceptance scenarios 1–2; US3 acceptance scenario 2; FR-001–FR-004, FR-006, FR-007, FR-008 (persistence, daemon sharing), FR-009; SC-001–SC-003 for the New worktree form (the MCP route closes in M3)
- **Verify**: `mise run gate`; tests `github_issues_setting`, `settings_roundtrip`, `schema_hash`, `features_worktree_form`, `settings_sections`
- **Depends on**: —
- **Tier**: full

### M2 — Open forms follow the switch

- **Tasks**: T017–T021
- **Deliverable**: Turning the switch off or on in one window changes every window's open form at once: GitHub list dropped, in-flight results discarded, picked issue cleared; turning on restores GitHub issues without a restart.
- **Satisfies**: US1 acceptance scenario 4; US3 acceptance scenarios 1, 3; FR-005; SC-004
- **Verify**: `cargo test -p micold-client` tests from T017 and T019 (`mise run gate`)
- **Depends on**: M1
- **Tier**: full

### M3 — MCP create_worktree honours the switch

- **Tasks**: T022–T025
- **Deliverable**: With the switch off, MCP `create_worktree` with `github_issue` fails with an error naming the switch and makes no GitHub request; calls without `github_issue` are unchanged.
- **Satisfies**: FR-010; FR-002 for the MCP path
- **Verify**: `cargo test -p micold-daemon --test mcp_create_worktree` and `mise run gate`
- **Depends on**: M1
- **Tier**: full
