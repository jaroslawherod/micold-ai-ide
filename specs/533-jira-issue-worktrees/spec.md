# Feature Specification: Create Worktrees from Jira Issues, Behind an Opt-In Integration Switch

**Feature Branch**: `feat/533_create-worktrees-from-jira-issues`

**Created**: 2026-10-12

**Status**: Draft

**Input**: User description: "Implement GitHub issue #533: Create worktrees from Jira issues. The create-worktree dialog can start from a GitHub issue; a project tracked in Jira has no equivalent. A project can be connected to Jira and the dialog then offers its Jira issues the way it offers GitHub issues. Integrations with outside services are off until the user turns them on, and their parts of the interface appear only while they are on."

## What already exists, and the gap

Feature 034 (`specs/034-github-issue-worktree`) lets the create-worktree dialog list the repository's open GitHub issues, search them, and fill in the ticket, the name and the type from the one picked; feature 038 gave its rows a reporter and a description. The label-to-type mapping lives in **Settings → GitHub → GitHub issues**. `naming.rs` already reads a Jira-style key (`ABC-123`) as an issue tag, so a branch named `feat/ABC-123_add-login` shows the `ABC-123` tag in the sidebar. What is missing is any way to reach Jira itself: a Jira user types the key and the name by hand.

The app already has one opt-in integration, **Show pull request status on worktrees** (off until turned on; nothing is sent while it is off). This feature states that rule for integrations in general and applies it to Jira.

## Terms

- **Integration**: a part of the app that sends requests to an outside service on the user's behalf.
- **Jira integration**: the integration this feature adds. Its **switch** is one app-wide setting, off by default.
- **Connection**: a project's link to Jira: a **site URL**, a **project key**, and the **credentials** (account email and API token) that sign requests to that site.
- **Jira issue**: an issue of the connected Jira project whose status is not in the Done status category (an *open* issue).
- **Issue source**: where the dialog's issue picker lists issues from: GitHub or Jira.
- **Issue key**: `KEY-123`, the Jira project key and the issue's number.
- **Notice**: the inline message in the picker that says a load failed and offers a retry (as 034 FR-007 does for GitHub).

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Start a worktree from a Jira issue (Priority: P1)

A developer whose team tracks work in Jira Cloud turns on the Jira integration in Settings, connects the open project to its site and project key, then opens the create-worktree dialog, searches "login", picks `ABC-123 Add login rate limit`, and gets a worktree named `feat/ABC-123_add-login-rate-limit` with the `ABC-123` tag in the sidebar, without typing the key or name.

**Why this priority**: It is the feature. It needs the switch and a connection to exist, so it carries the minimum of both.

**Independent Test**: Against a fake Jira site returning three issues, enable the switch, save a connection, open the dialog, pick the issue, and read the ticket, name, type, branch preview and resulting worktree name.

**Acceptance Scenarios**:

1. **Given** the integration is enabled and the project has a connection, **When** the user opens the create-worktree dialog, **Then** the picker lists the project's open Jira issues, most recently updated first, each row showing the issue key and summary.
2. **Given** the list is showing, **When** the user types text, **Then** it narrows to issues whose key, summary or any label contains the text, ignoring letter case (the matching rule of 034 FR-005 with key in place of number).
3. **Given** the list is showing, **When** the user picks issue `ABC-123` with summary "Add login rate limit", **Then** the ticket is `ABC-123`, the name is the summary (shortened as 034 FR-010 does for a long title), and the directory and branch preview update as if the user had typed those values.
4. **Given** the picked issue has issue type `Bug` and no mapped label, and the label-to-type list maps `bug` to fix, **Then** the type is `fix`; **given** the issue's labels include a mapped label, **Then** the entry nearest the top of the list wins among the labels and the issue-type name together.
5. **Given** a picked issue, **When** the user creates the worktree, **Then** the worktree is named `${type}/${KEY-123}_${name}` and the sidebar shows the `ABC-123` issue tag.
6. **Given** a picked issue none of whose type name or labels is mapped, **Then** the type is left for the user to choose, as for GitHub.
7. **Given** the user picked an issue and then edits the ticket, name or type by hand, **Then** the edit stands and picking another issue replaces the filled values (034 behaviour, unchanged).

---

### User Story 2 - Nothing about Jira exists until I turn it on (Priority: P1)

A developer who does not use Jira never sees a Jira control, is never asked for credentials, and has no request leave the machine for Jira. A developer who turns the integration on sees it, and turning it off makes it all go away again.

**Why this priority**: It is the issue's stated rule and the guard that makes shipping the integration safe for everyone else.

**Independent Test**: With default settings, open Settings and the dialog and compare against the pre-feature behaviour; with a request recorder on the fake site, confirm no request. Enable, confirm controls and requests; disable, confirm both stop.

**Acceptance Scenarios**:

1. **Given** default settings, **When** the user opens the create-worktree dialog or any Settings page, **Then** no Jira picker, source choice or credential field appears (only the switch, off, in Settings), and the dialog looks and behaves as before this feature.
2. **Given** the integration is disabled, **When** any part of the app runs, **Then** no request goes to any Jira site, whether or not a connection is stored.
3. **Given** the switch is on, **When** the user opens Settings, **Then** the Jira section says the integration is enabled and names the connected site and project, or says no project is connected yet.
4. **Given** the integration is enabled with a connection, **When** the user turns the switch off and saves, **Then** the Jira picker and source choice disappear from every open window, an in-flight Jira request's result is discarded, and no further request is made.
5. **Given** the switch was turned off, **When** the user turns it on again, **Then** the stored connection is still there, unless the user removed it (User Story 3).
6. **Given** a settings file written before this feature, **When** it is read, **Then** the integration is disabled and nothing else in the file changes.

---

### User Story 3 - Connect, change and remove a Jira connection (Priority: P2)

In Settings the user enters the site URL, the project key, the account email and an API token, saves, and sees the connection described. They can change any of them or remove the whole connection, which deletes the stored token.

**Why this priority**: Without it the picker has no source, but the switch and the picker (P1 stories) are testable with a preset connection.

**Independent Test**: Save a connection, restart the app, confirm it persists and the token is in no project file or log; remove it and confirm the token file is gone and the dialog offers no Jira.

**Acceptance Scenarios**:

1. **Given** the switch is on and no connection, **When** the user fills the four fields and saves, **Then** the connection is stored for the open project and the Jira section names the site and project.
2. **Given** a stored connection, **When** the app restarts, **Then** the connection is still there and the token field shows that a token is stored without showing it.
3. **Given** a stored connection, **When** the user chooses **Remove connection**, **Then** the site, project key, email and token are deleted and the picker no longer offers Jira for that project.
4. **Given** a stored connection and the switch turned off, **When** the user is asked whether to keep the connection, **Then** keeping it leaves it stored and removing it deletes it as in scenario 3.
5. **Given** the site URL is not an `https` URL, or the project key is empty or not a valid Jira key, or a credential is blank, **When** the user saves, **Then** save is refused, the field is marked, and nothing is stored.
6. **Given** a saved connection, **When** the user opens the dialog, **Then** the stored token is never written to `settings.json`, a project file, the session history or any log line, in any error path.

---

### User Story 4 - Choose between GitHub and Jira, and survive Jira failing (Priority: P2)

A project that has both a GitHub remote and a Jira connection lets the user choose which issues the picker lists. When Jira cannot be reached, the picker says so and offers a retry, and creating a worktree by hand is unaffected.

**Why this priority**: It completes the acceptance criteria but each part is a refinement of Story 1.

**Independent Test**: With both a GitHub remote and a connection, switch sources and check the lists; take the fake site offline and confirm the notice, the retry and a hand-made worktree.

**Acceptance Scenarios**:

1. **Given** a GitHub remote and an enabled Jira connection, **When** the dialog opens, **Then** it shows a choice of source (GitHub, Jira) and the picker lists the chosen source's issues; choosing the other source replaces the list and clears the search text.
2. **Given** the user chose a source in a project, **When** they open the dialog in that project again, **Then** it opens on that source; with no earlier choice it opens on GitHub.
3. **Given** a project with a Jira connection and no GitHub remote, **Then** the picker lists Jira and no source choice is shown.
4. **Given** a project with a GitHub remote and the integration disabled or no connection, **Then** the dialog is exactly the GitHub dialog of 034, with no source choice.
5. **Given** the Jira site cannot be reached or does not answer within 10 seconds, **When** the picker loads, **Then** it shows a notice saying the site could not be reached, with a **Retry**, and the ticket, name and type inputs still work.
6. **Given** Jira refuses the credentials (rejected or expired token, no permission for the project), **When** the picker loads, **Then** the notice says the credentials were refused and names Settings as where to fix them, with a **Retry**; it never shows the token.
7. **Given** the notice is showing, **When** the user chooses Retry after the site is back, **Then** the list loads without reopening the dialog.

---

### Edge Cases

- **Empty project**: the connected project has no open issues; the picker says there are none, as for GitHub (034).
- **More than 1,000 open issues**: the list loads the 1,000 most recently updated; searching beyond them asks Jira, under the same rule as 034 FR-004 and FR-005.
- **Issue with no labels, or a very long summary, or markup in the summary**: shown and shortened as GitHub titles are; no label means type mapping by issue-type name only.
- **Key that naming cannot read**: a Jira project key the issue-tag rule in `naming.rs` does not accept as a tag yields a ticket the user can see and edit; the worktree is still created.
- **Two windows**: a change to the switch or the connection in one window shows in the other without a restart; two windows open on the same project share one connection.
- **No project open**: the switch can be turned on, the connection fields say a project must be open, and no request is made.
- **Site URL with a path or trailing slash**: normalised to its origin; a site URL that is `http`, has a user-info part, or names no host is refused (FR-006).
- **Two windows saving at once**: the last save wins in both windows; neither window shows a half-written connection.
- **Token removed during a load**: the load's result is discarded and the picker shows no Jira list (as FR-005).
- **Token rotated on the Jira side**: the next load shows the refused-credentials notice; saving a new token fixes it.
- **Proxy and TLS**: a proxy or a certificate failure is a "could not be reached" notice with its reason shortened, never the token.
- **Platform differences (Principle VI)**: the stored token is readable only by the signed-in user on Linux, macOS and Windows alike; the dialog and Settings are the same on all three.
- **Daemon and sessions (Principle II)**: no session's start, state or history depends on Jira; the connection lives with the client's settings, not in a session or a daemon.
- **Jira Data Center and Server**: not supported (FR-023). A site that answers like one shows the could-not-use notice of FR-018, not a wrong list.

## Requirements *(mandatory)*

### Functional Requirements

**The opt-in rule (integrations in general)**

- **FR-001**: An integration with an outside service MUST be off by default, and while it is off the app MUST send it no request, ask for none of its credentials and show none of its controls other than its own switch (with a one-line description of what enabling it does), which is where the user turns it on.
- **FR-002**: The Jira integration MUST be one app-wide switch in Settings, off by default and kept in `settings.json`; a settings file without it MUST read as off.
- **FR-003**: With the switch off, the create-worktree dialog and the rest of the app MUST look and behave exactly as before this feature.
- **FR-004**: With the switch on, Settings MUST show that the integration is enabled and which Jira site and project the open project is connected to (or that it is not connected), and the dialog MUST show the Jira picker for a connected project.
- **FR-005**: Turning the switch off MUST remove every Jira control from every open window, stop every Jira request and discard the result of one in flight, and MUST let the user keep or remove the stored connection.
- **FR-005b**: Beside the switch, and above the Jira picker while it loads, the app MUST say that it contacts the connection's Jira site and what it sends: the project key, the search text and the credentials (as 034 FR-025 does for GitHub).

**Connection**

- **FR-006**: A connection MUST hold a site URL (an `https` URL, normalised to its origin), a project key, an account email and an API token; saving MUST refuse an empty or malformed value of any of them and mark the field.
- **FR-007**: A connection MUST belong to one project and be stored on the user's computer with the app's other per-user data; two projects MAY have different connections, and a project with none offers no Jira.
- **FR-008**: The API token MUST be stored the way the app stores its other secrets: in a file readable only by the signed-in user, separate from the settings file, and never in a project file.
- **FR-009**: The token MUST NOT appear in `settings.json`, a project file, a log line, a notice, an error message, a session's history or a `Debug` rendering of any type holding it, on any error path.
- **FR-010**: Settings MUST let the user change a connection's fields, remove the connection (deleting its stored token), and see whether a token is stored without being shown it.

**The picker**

- **FR-011**: With the integration on and a connection, the dialog MUST list the connected project's open issues (status category not Done), most recently updated first, with at most 1,000 loaded, in rows of the same shape as GitHub's (key in place of number; summary; labels; reporter and a description tooltip where GitHub rows show them, 038).
- **FR-012**: The picker MUST search by issue key, summary and label with the same matching rule and emphasis as the GitHub picker, ignoring letter case, and MUST ask Jira for matches beyond the loaded issues only when the loaded list is incomplete (034 FR-004, FR-005).
- **FR-013**: Picking an issue MUST set the ticket to its key, the name to its summary (shortened to 50 characters as GitHub titles are), and the type from the first entry of the label-to-type list, nearest the top, that matches the issue's type name or any of its labels, compared ignoring case and surrounding spaces; with no match the type is left to the user.
- **FR-014**: The Jira issue-type name and labels MUST use the existing label-to-type list (**Settings → GitHub → GitHub issues**), which applies to both sources; that section's heading or intro MUST say it also applies to Jira issue types and labels; the default entries are unchanged, so `Bug` selects `fix` out of the box.
- **FR-015**: A worktree made from a Jira issue MUST be named `${type}/${KEY-123}_${name}` (the name rule of 034, with the key as ticket) and the sidebar MUST show the key as its issue tag.
- **FR-016**: When the project has both a GitHub remote and an enabled Jira connection the dialog MUST offer the choice of source; the chosen source MUST be remembered per project and default to GitHub; with only one source available no choice is shown.
- **FR-017**: Switching source MUST replace the list, clear the search text and discard the other source's in-flight result.
- **FR-018**: A Jira site that is unreachable, times out (10 seconds), answers with something that is not a Jira Cloud listing, or refuses the credentials MUST show a notice in the picker with a **Retry** and the cause in plain words; the ticket, name and type inputs and the creation of a worktree by hand MUST keep working.
- **FR-019**: The loaded issues MUST be held only in the open dialog and MUST NOT be serialised or persisted (as 034 FR-023); a hand-written `Debug` for a Jira issue MUST redact its reporter and description (as 038 FR-025).
- **FR-020**: Requests to Jira MUST be made only while the integration is on, a connection exists and the dialog's Jira source is showing, MUST go only to the connection's site, and MUST carry only the project key, the search text and the credentials.

**Scope and platform**

- **FR-021**: The Jira integration MUST work without the daemon's involvement in a session, on Linux, macOS and Windows, and MUST NOT change how a session starts or what it stores.
- **FR-022**: A change to the switch or a connection made in one window MUST show in every other open window without a restart.
- **FR-023**: Jira Cloud (REST API v3, email and API-token sign-in) is the supported product. Jira Data Center and Jira Server are out of scope: no sign-in with a personal access token, no v2 API, and a site that is one MUST get the notice of FR-018 rather than a wrong list.
- **FR-024**: The user guide MUST document enabling the integration, connecting a project, creating a worktree from a Jira issue, the source choice, the failure notices and where the token is stored.

### Key Entities

- **Jira integration setting**: the one app-wide on/off switch.
- **Jira connection**: per project; site origin, project key, account email (kept with settings) and API token (kept in a separate owner-only secret).
- **Issue source choice**: per project, GitHub or Jira.
- **Jira issue**: key, summary, issue-type name, labels, reporter, description, last-updated; held in memory only.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: With default settings, zero requests go to any Jira host and zero Jira controls other than the off switch are present, across the whole interface (tested by watching the requests the app makes and counting the controls).
- **SC-002**: A user with a connected project goes from opening the dialog to a created, correctly named worktree with only a search, a click and a confirm, with no field typed by hand.
- **SC-003**: Searching a 1,000-issue list by key, summary or label shows its matches within one frame of the keystroke (the GitHub picker's bar, 034).
- **SC-004**: A scan of `settings.json`, project files, logs and the session history after a full connect, load, failure and remove cycle finds the token zero times.
- **SC-005**: With the site down or refusing, 100 percent of picker loads show a notice with a Retry, and a worktree can still be created by hand from the same dialog.
- **SC-006**: Turning the switch off stops Jira requests within one second in every open window, and no Jira control remains.

## Assumptions

- **GitHub issues stay outside the switch (decided by user, FR-001 stands as the rule).** The existing GitHub issue picker keeps working as it does, because it reads only through the user's own `gh` sign-in, would vanish for every current user on upgrade if gated, and the issue's acceptance criteria do not require it. A follow-up brings it under an integrations switch with a migration.
- **Jira Data Center is out of scope (decided by user, FR-023).** It signs in differently, speaks an older API and cannot be exercised without an instance.
- Credentials are an account email plus an API token (Atlassian's documented sign-in for the Cloud REST API); OAuth is out of scope.
- No operating-system keychain is used: the app has no keychain integration, so "the way the app stores other secrets" is the owner-only file of `owner_only.rs`.
- The connection is edited in Settings for the project open in the window, because a project is where the connection lives; the switch itself is app-wide.
- Reporter, labels and the description tooltip follow whatever #518 settles for GitHub rows; this feature gives Jira rows the same shape and does not redesign them.

## Out of Scope

- Jira Data Center and Jira Server; OAuth and personal access tokens.
- Writing to Jira (transitions, comments, assigning); only listing, searching and reading issues.
- Boards, sprints, epics, JQL entry, and projects other than the one connected.
- Putting the GitHub issue picker behind the switch (follow-up, see Assumptions).
- A shared **Integrations** area in Settings that lists every integration's switch, and moving the existing pull request status switch into it (follow-up with the GitHub gating).
- Showing Jira issue status on worktree rows or in the sidebar.

## Clarifications

### Session 2026-10-12

- Q: Does the GitHub issue picker move under the opt-in integrations switch in this feature? -> A: No, it stays ungated; a follow-up brings it under the switch with a migration. _(decided by user)_
- Q: Is Jira Data Center in scope? -> A: No, Jira Cloud only; Data Center and Server in a later feature. _(decided by user)_
