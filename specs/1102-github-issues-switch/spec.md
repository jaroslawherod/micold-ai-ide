# Feature Specification: Allow the GitHub issues integration to be disabled

**Feature Branch**: `feat/1102_allow-the-github-issues-integration-to-be-disabled`

**Created**: 2026-10-19

**Status**: Closed 2026-10-19 — shipped in PRs #1112, #1114, #1133, #1138

**Input**: Issue #1102: "The Jira integration has an on/off switch (`jira_enabled`), but the GitHub issues integration has none: it is always on. A project that tracks its work only in Jira, or does not want GitHub issues offered at all, still gets GitHub issues in the New worktree form and pays for the GitHub calls that load them. A setting, like `jira_enabled`, that turns the GitHub issues integration off. When it is off, GitHub issues are not loaded or offered as an issue source, and no GitHub issue requests are made."

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Turn GitHub issues off (Priority: P1)

A user who does not use GitHub issues opens Settings and turns the GitHub issues switch off. From then on the New worktree form does not offer GitHub issues and the app makes no GitHub issue request.

**Why this priority**: It is the whole request: without the switch nothing else matters.

**Independent Test**: With the switch off, open the New worktree form on a repository that has a GitHub remote and check that no GitHub issue list is requested and no GitHub issue source is offered.

**Acceptance Scenarios**:

1. **Given** the switch is off, **When** the user opens the New worktree form on a project with a GitHub remote, **Then** no GitHub issue list or issue details are requested and the form offers no GitHub issue source.
2. **Given** the switch is off and Jira is connected for the project, **When** the form opens, **Then** Jira is the only issue source and no GitHub / Jira choice is shown.
3. **Given** the switch is off and Jira is off or not connected, **When** the form opens, **Then** the form has no issue source at all and creating a worktree without an issue still works.
4. **Given** the form is open with a GitHub issue picked or a GitHub load in flight, **When** the user turns the switch off, **Then** the in-flight result is discarded, the GitHub list is dropped and the form falls back as in scenarios 2 and 3.

---

### User Story 2 - Existing behaviour is kept when the user does nothing (Priority: P1)

A user who has never seen the switch keeps getting GitHub issues exactly as today.

**Why this priority**: GitHub issues are on today; an upgrade must not silently remove a feature.

**Independent Test**: Start with a settings file that has no GitHub issues entry and check that GitHub issues are offered.

**Acceptance Scenarios**:

1. **Given** a settings file written before this feature, **When** the app starts, **Then** the switch reads as on and the New worktree form behaves as before.
2. **Given** the switch is on, **When** the form opens on a project with a GitHub remote, **Then** GitHub issues load and are offered as today.

---

### User Story 3 - The switch is remembered and shared like the Jira one (Priority: P2)

The switch is saved with the other settings, reaches every open window and every connected daemon the way `jira_enabled` does, and turning it back on restores GitHub issues without a restart.

**Why this priority**: A switch that forgets itself, or differs between windows, is a defect, but the P1 stories already deliver value without cross-window polish.

**Independent Test**: Turn the switch off in one window, check a second window's open form drops GitHub issues, restart, and check the switch is still off.

**Acceptance Scenarios**:

1. **Given** two windows with the form open, **When** the switch is turned off in one, **Then** the other drops GitHub issues without reopening the form.
2. **Given** the switch was turned off, **When** the app restarts, **Then** it is still off.
3. **Given** the switch is off, **When** the user turns it on with a form open, **Then** the form offers GitHub issues again and loads them.

### Edge Cases

- Switch off and the repository has no GitHub remote: nothing changes (no GitHub source was offered anyway); no request is made.
- Switch off while a GitHub issue detail load is in flight: the result is discarded, not shown.
- A remembered issue-source choice of GitHub (per project) while the switch is off: ignored, and kept so that turning the switch on restores it.
- Several daemons or sessions (multi-session) toggle the switch at the same moment: the most recent write wins and every session ends on that value.
- The settings file is unreadable or corrupt: the app starts with the switch on, as for a missing entry, and does not overwrite the file until the user changes a setting.
- Saving the switch fails: the switch keeps its new value for the running session, the failure is reported like any other settings save failure, and the next save retries.
- A GitHub issue request completes after the switch was turned off: its result is discarded, not shown.
- A worktree already created from a GitHub issue stays as it is; the switch only governs offering and loading issues.
- Settings file from a newer or older build: an unknown or missing entry falls back to on.
- Behaviour is identical on Linux, macOS and Windows.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: The app MUST have a persisted setting that turns the GitHub issues integration on or off, on by default.
- **FR-002**: While it is off, the app MUST make no GitHub issue request: no issue list load, no issue search, no issue details load, and no read of GitHub issue titles for the overview's worktree rows.
- **FR-003**: While it is off, the New worktree form MUST NOT offer GitHub as an issue source, and MUST NOT show the GitHub / Jira choice.
- **FR-004**: While it is off, Jira, when on and connected, MUST remain usable as the sole issue source, and a worktree without an issue MUST remain creatable.
- **FR-005**: Turning the switch off MUST, in every open window, drop the GitHub issue list, discard a GitHub load in flight and clear a picked GitHub issue; turning it on MUST offer GitHub issues again without a restart.
- **FR-006**: The user MUST be able to turn the switch on and off in the Settings **GitHub** section, beside the pull request status switch, with a one-line description of what it does.
- **FR-007**: A settings file without the entry MUST read as on, so existing users see no change.
- **FR-008**: The switch MUST be saved across restarts, and a value set in one window or on one daemon MUST become the value every other window and daemon uses; when two changes race, the most recent one wins on all of them (the behaviour the Jira switch has today).
- **FR-009**: Turning the switch off MUST NOT affect GitHub features that are not issues (pull request status, remotes, branch listing).
- **FR-010**: While it is off, the MCP `create_worktree` with `github_issue` MUST fail without any GitHub request, with an error that names the switch and points to `branch`, or `type`, `ticket` and `name`; feature 837's relationship details are not read.

### Key Entities

- **GitHub issues switch**: one app-wide boolean setting, default on, sibling of the Jira switch.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: With the switch off, opening the New worktree form makes zero GitHub issue requests.
- **SC-002**: With the switch off, 100% of New worktree forms show no GitHub issue source.
- **SC-003**: A settings file written before this feature yields identical New worktree form behaviour (switch on).
- **SC-004**: Turning the switch off or on takes effect in every open form without reopening the form or restarting the app.

## Out of Scope

- A per-project switch (the switch is app-wide).
- GitHub features that are not issues: pull request status, remotes, branch listing (FR-009).
- Changes to how Jira is enabled or connected.

## Assumptions

- The switch is global like `jira_enabled`, not per project.
- Only the issue features of GitHub are covered: listing, searching and reading issues for the New worktree form.
- The Jira switch is the model for wording and for how the value is saved and shared; it is placed in the GitHub section, not the Jira one.

## Clarifications

### Session 2026-10-19

- Q: Do issue references elsewhere (MCP `create_worktree` with `github_issue`, feature 837's relationship metadata) stop making GitHub requests when the switch is off? -> A: Yes. The issue's own rule is "no GitHub issue requests are made", and FR-002 is absolute; the MCP call reads the issue to fill the form's inputs, so it would break it. With the switch off, `create_worktree` with `github_issue` fails with a clear error naming the switch and pointing to `branch`, or `type`, `ticket` and `name`; feature 837's details are never read. Calls without `github_issue` are unchanged. _(agent-resolved: GitHub issue #1102 text; specs/837-issue-relationship-metadata/spec.md FR-001, FR-010)_
- Q: Where does the switch live in Settings? -> A: In the existing **GitHub** section (`SettingsSection::GithubIssues`), above the issue-label mapping, beside the pull request status switch. The Jira switch heads the Jira section because that section holds Jira's connection; the GitHub section is the one that already holds GitHub's switch and issue settings, so a new section is not needed. The one-line description follows the Jira switch's wording. _(agent-resolved: crates/micold-client/src/features/settings.rs `SettingsSection`, `GithubDraft`)_
