# Feature Specification: Jira Ticket Status

**Feature Branch**: `feat/1101_show-a-jira-ticket-s-current-status`

**Created**: 2026-10-20

**Status**: Closed 2026-10-22 — shipped in PRs #1152, #1155, #1190, #1218

**Input**: User description: "Show a Jira ticket's current status (GitHub issue #1101). A Jira ticket shown in Micold (issue picker rows in the New worktree form, a worktree's linked ticket) does not show its current Jira status, so there is no way to tell whether a ticket is To Do, In Progress or In Review without opening Jira. Read the ticket's status (name and status category) from Jira and show it wherever a Jira ticket appears, refreshed so it reflects the ticket's status now, not the status when it was first loaded. Alternative considered: opening the ticket in Jira."

## Terms

- **Status**: a Jira ticket's workflow state, as Jira reports it: a **status name** (for example *To Do*, *In Progress*, *In Review*; the names are the project's own) and a **status category** (Jira's fixed three: *To Do*, *In Progress*, *Done*).
- **Status chip**: the small label that shows a ticket's status name, tinted by its category.
- **Linked ticket**: the Jira key a worktree carries, shown as the issue tag on its sidebar row (feature 533, FR-015).
- **Jira integration**: feature 533's switch in Settings plus the project's Jira connection. Jira is read only while the switch is on and the open project is connected.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - See a ticket's status in the issue picker (Priority: P1)

A user opens the New worktree form with Jira chosen as the issue source. Each ticket row shows its status chip beside the key, so they can tell a ticket nobody has started from one already in review before picking it.

**Why this priority**: The picker is where a ticket is chosen; the status decides which one. The picker's rows come from the listing Jira is already asked for; they need only the status to be read with it.

**Independent Test**: Connect a project (the fake Jira source is enough), open the form, and check that rows for tickets in *To Do*, *In Progress* and *In Review* show those names.

**Acceptance Scenarios**:

1. **Given** the picker lists tickets whose statuses are *To Do*, *In Progress* and *In Review*, **When** the rows are drawn, **Then** each row shows its own status name.
2. **Given** a search narrows the list, **When** the results are drawn, **Then** every result row shows its status.
3. **Given** a ticket whose status Jira did not report, **When** its row is drawn, **Then** the row shows no status chip and is otherwise unchanged.
4. **Given** a GitHub issue list, **When** rows are drawn, **Then** they are unchanged.

---

### User Story 2 - See the linked ticket's current status on a worktree (Priority: P1)

A user has worktrees named for Jira tickets. Next to each worktree's ticket tag in the sidebar they see the ticket's current status, and it follows the ticket when a teammate moves it in Jira.

**Why this priority**: The issue asks for the status as it is now, not as first loaded; a worktree lives for days and its ticket moves while it does.

**Independent Test**: With a worktree linked to a ticket that the fake source reports as *In Progress*, change the source to *In Review* and trigger a refresh event: the sidebar row shows *In Review* without restarting.

**Acceptance Scenarios**:

1. **Given** a worktree linked to a Jira ticket and the integration on and connected, **When** its status has been read, **Then** the sidebar row shows the status chip beside the key tag.
2. **Given** the ticket moved from *In Progress* to *In Review* in Jira, **When** the next refresh happens, **Then** the row shows *In Review*.
3. **Given** a ticket that moved to a *Done* category status, **When** it is refreshed, **Then** the row shows the done status (the ticket is not hidden or removed).
4. **Given** a refresh fails (offline, token rejected, Jira unavailable), **When** it ends, **Then** the row keeps the last status it showed, marked as possibly out of date, or none when none was ever read; nothing blocks the sidebar and no dialog opens.
5. **Given** Jira answers that a linked ticket is not found or not visible, **When** the refresh ends, **Then** its row shows no chip.

---

### User Story 3 - Status wherever else the ticket appears (Priority: P2)

A ticket that already appears in other places in the window shows its status chip there too, in the same form, so the status never has to be looked up.

**Why this priority**: Consistency, but each place is small and the first two stories carry the value.

**Independent Test**: Open each other surface that names a Jira ticket and check it shows the same chip as the sidebar for the same ticket.

**Acceptance Scenarios**:

1. **Given** a ticket shown in two places, **When** both are drawn, **Then** they show the same status.
2. **Given** the Overview shows a card for a worktree linked to a Jira ticket, **When** it is drawn, **Then** the card shows the ticket's status chip.

---

### Edge Cases

- **Integration off or project not connected**: no Jira request is made to learn a status and rows show no chip (a status read earlier is retired, FR-008).
- **Empty or unknown status**: a ticket with a missing status, a blank name or an unrecognised category shows no chip, or the name in a neutral tint when only the category is missing.
- **Very long status name**: truncated with an ellipsis; never pushes the key or title off the row.
- **Many linked tickets**: tickets are read in few requests regardless of the worktree count, and a refresh never makes Micold slower to draw.
- **Several sessions or windows (Principle II)**: each shows the same status for the same ticket; refreshes triggered at the same moment by several windows or sessions coalesce into one Jira request, and no window can write to Jira.
- **Worktree whose key is not a Jira key, or Jira switched on with no connection for the open project**: no request is made and no chip is shown.
- **Different projects**: only tickets of the connected project are read; a worktree linked to a key of another project shows no status.
- **Cross-platform (Principle VI)**: the chip is drawn the same way on macOS and Linux; its meaning is never carried by tint alone, since the name is shown.
- **Rate limiting or Jira unavailable**: no extra request is made until the next FR-005 event and the last status stays; the token and site are never shown in any message.
- **Status renamed by an admin**: the new name appears at the next refresh.
- **Worktree with a numeric (GitHub) ticket**: unaffected.

## Clarifications

### Session 2026-10-20

- Q: Which events refresh a linked ticket's status? → A: Project open and window focus when the last read is over 5 minutes old; no timer. _(decided by user)_
- Q: Does the Overview card (feature 813) show the status chip? → A: Yes, from the same status as the sidebar. _(decided by user)_

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: Micold MUST read each Jira ticket's status name and status category along with the ticket data it already reads for the picker.
- **FR-002**: Every Jira ticket row in the New worktree form's picker, listing and search results alike, MUST show the ticket's status name.
- **FR-003**: A worktree linked to a Jira ticket MUST show the ticket's status name beside its key tag on its sidebar row.
- **FR-004**: The chip MUST be tinted by status category (To Do, In Progress, Done, each visibly distinct) and MUST show the status name as text.
- **FR-005**: The status shown for a linked ticket MUST be refreshed so it reflects Jira's current value on these events only: the project opening, the window regaining focus when the last read is older than 5 minutes.
- **FR-006**: A status refresh MUST NOT run unless the Jira integration is on and the project connected, and MUST NOT run on any event other than those of FR-005.
- **FR-006b**: This feature amends feature 533 FR-020 and FR-005b: Jira requests are also allowed with the New worktree dialog closed, for a status refresh, and then carry the project key, the linked tickets' keys and the credentials. The Settings and picker disclosure of what is sent MUST say so. No other Jira request rule changes, so feature 533's named-events test is extended to the FR-005 events.
- **FR-007**: A linked ticket may already be done, which the picker's list leaves out, so a refresh MUST read the linked tickets by key, whatever their status, and the tickets of many worktrees MUST be read together, not one request per worktree.
- **FR-008**: When a refresh fails, the last known status MUST stay on screen as a dimmed chip whose accessible text adds "may be out of date"; the failure MUST NOT block, raise a dialog, or change any other part of the window. A status is retired (no chip) when Jira answers that the ticket is not found or not visible, when the integration is turned off or the connection removed, or after 3 consecutive failed refreshes.
- **FR-009**: A ticket with no usable status MUST show no chip and MUST NOT change its row's layout otherwise.
- **FR-010**: The same ticket MUST show the same status on every surface and in every window at the same moment of the app's state.
- **FR-011**: The feature MUST NOT write to Jira, MUST NOT change which tickets the picker lists, and MUST NOT change GitHub issue rows.
- **FR-012**: The API token and the site MUST NOT appear in any status text, tooltip or log line.
- **FR-013**: A status chip MUST have an accessible text equal to the status name (and "may be out of date" when marked so).

### Key Entities

- **Ticket status**: the name and category Jira reports for one ticket, with the moment it was read.
- **Linked ticket**: the Jira key on a worktree, which selects which status the worktree's row shows.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: A user can tell a ticket's status from the picker or the sidebar without opening Jira: 100% of rows for tickets with a status show it.
- **SC-002**: After a ticket changes status in Jira, the sidebar shows the new status within one refresh event, and within 5 minutes of the window being focused again.
- **SC-003**: Opening a project with 50 linked tickets makes 1 Jira request for their statuses, and the window stays interactive throughout.
- **SC-004**: With the integration off, zero Jira requests are made for statuses.
- **SC-005**: A failed refresh leaves the window usable and the last status visible in 100% of tested failure kinds (offline, rejected token, Jira unavailable), and a not-found ticket loses its chip.

## Assumptions

- The Jira integration of feature 533 (switch, connection, token file) is reused and this feature adds no new setting; it widens when requests may be made as FR-006b says.
- The picker's list stays limited to tickets not yet done; a Done status appears only on already-linked worktrees.
- Status names are shown as the project defines them; they are not translated or mapped.
- Linked tickets are the worktrees' Jira keys already parsed for the sidebar tag.
- Showing status on the thread list or any place beyond the picker, the sidebar and the Overview cards is out of scope.

## Out of Scope

- Changing a ticket's status (transitions), comments, assignee, or any Jira write.
- Filtering or sorting the picker by status.
- Jira Data Center and Server.
- Status for GitHub issues or pull requests (the existing pull request status is separate).
- Notifications when a status changes.
