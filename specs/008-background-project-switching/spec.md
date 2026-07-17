# Feature Specification: Background Project Switching

**Feature Branch**: `feat/allow-to-switch-projects`

**Created**: 2026-07-17

**Status**: Draft

**Input**: User description: "Allow switching projects once a terminal session has been started, and continue those sessions in the background. The switcher should be located next to the menu button at the top bar."

## Clarifications

### Session 2026-07-17

- Q: When a background session's process exits unexpectedly while its project is not active, what should happen? → A: Auto-restart with the existing bounded crash-loop guard, and notify the user on return that a background restart occurred (no silent state changes).
- Q: How should the new top-bar switcher relate to the existing "Known projects" body list and folder-browser dialog? → A: Complement them — add the switcher; keep the body list and folder browser unchanged.
- Q: Should there be a limit on how many projects can hold running background sessions simultaneously? → A: No cap; limited only by available system resources.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Keep sessions alive across a project switch (Priority: P1)

A user is working in Project A with one or more terminal sessions running (for example, a long-lived agent task producing output). They switch to Project B to check or do something else, then later switch back to Project A. Every session that was running in Project A is still running, has continued producing output while they were away, and is shown again in the same foreground/background arrangement it had before the switch.

**Why this priority**: This is the core value of the feature. Today, switching projects stops and resets the outgoing project's running sessions, so any in-progress work is lost. Preserving running work across a switch is the single change that turns project switching from destructive into safe, and it delivers value even without any new switcher UI.

**Independent Test**: Start a session in Project A that produces continuous output. Switch the active project to Project B (using any existing switching entry point). Wait, then switch back to Project A. Verify the session is still running (not restarted, not idle), its process is the same one, and the output produced during the absence is present.

**Acceptance Scenarios**:

1. **Given** Project A has a running session producing output, **When** the user switches the active project to Project B, **Then** Project A's session keeps running in the background and its process is not stopped or reset to idle.
2. **Given** Project A has a running background session, **When** the user switches the active project back to Project A, **Then** the session is shown as still running with the output it produced while Project A was inactive.
3. **Given** Project A had several sessions with one in the foreground, **When** the user switches away and back, **Then** the session that was in the foreground is restored to the foreground and the others remain in the background.
4. **Given** Project B has no sessions yet, **When** the user switches to Project B while Project A has running background sessions, **Then** Project B opens normally and Project A's sessions are unaffected.

---

### User Story 2 - Quick project switcher in the top bar (Priority: P2)

A user wants to change the active project quickly without opening the folder-browser dialog or scrolling the main-body project list. They open a project switcher located immediately next to the menu button at the top bar, see their known projects, and pick one to make it active in a single interaction.

**Why this priority**: Frequent switching is only comfortable if switching itself is fast and always reachable. The top-bar switcher makes switching a one- or two-step action from anywhere in the app. It builds on Story 1 (which makes switching safe) but is independently demonstrable.

**Independent Test**: Open the switcher from the top bar next to the menu button, confirm it lists the known projects, select a project other than the active one, and verify the active project changes to the selected one.

**Acceptance Scenarios**:

1. **Given** the app is showing any project, **When** the user opens the control immediately next to the top-bar menu button, **Then** a switcher listing the known projects is shown.
2. **Given** the switcher is open, **When** the user selects a project other than the current one, **Then** that project becomes active without the folder-browser dialog being opened.
3. **Given** the switcher is open, **When** the user selects the already-active project (or dismisses the switcher), **Then** the active project does not change.
4. **Given** the switcher is open, **When** the user chooses to add a project that is not yet known, **Then** they are taken to the existing add-a-project flow (folder browser).

---

### User Story 3 - See which projects have work running (Priority: P3)

A user with several known projects wants to tell at a glance, from the switcher, which projects currently have sessions running in the background, so they can decide where to return.

**Why this priority**: Once multiple projects can hold running sessions simultaneously, users need a way to see where their live work is without opening each project. This is a refinement of the switcher (Story 2) rather than a prerequisite for it.

**Independent Test**: Start a session in Project A, switch to Project B, open the switcher, and verify Project A is marked as having a running background session while Project B (with no sessions) is not.

**Acceptance Scenarios**:

1. **Given** Project A has running background sessions and Project B has none, **When** the user opens the switcher, **Then** Project A shows a running-session indicator and Project B does not.
2. **Given** the switcher is open, **When** the user views the active project, **Then** it is visibly marked as the active project.
3. **Given** a known project's folder is missing or unavailable, **When** the user opens the switcher, **Then** that project is shown with an unavailable indication.

---

### Edge Cases

- **Background session crashes while its project is inactive**: A running background session's process exits unexpectedly while its project is not active. The system applies the same unexpected-exit handling used for foreground sessions (automatic restart bounded by the existing crash-loop guard). The resulting session state is reflected when the user returns to that project, and the user is notified on return that a background restart occurred. If restarts are exhausted, the session is shown as failed on return rather than silently disappearing.
- **Switching to an unavailable project**: The user selects a project from the switcher whose folder has been moved or deleted. Switching is prevented or clearly reported, and any background sessions belonging to still-available projects are unaffected.
- **Rapid switching**: The user switches active project several times in quick succession. No project's background sessions are stopped as a side effect, and the final selected project is the one shown.
- **App restart**: Live session processes do not survive an application restart (only session identity and title are persisted, consistent with existing behavior). After a restart, previously running sessions are restored as idle and resumable, not as live background processes. "Background" in this feature means within a single running app session.
- **Many projects running at once**: Several projects each hold running background sessions simultaneously. Sessions remain isolated per project and no project's sessions leak state into another.
- **Output volume while inactive**: A background session produces a very large amount of output while its project is inactive. Output remains visible on return up to the same scrollback limit that applies to foreground sessions.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: Users MUST be able to change the active project at any time, including while one or more terminal sessions are running, without those sessions being stopped or reset.
- **FR-002**: When the active project changes, the previously active project's running sessions MUST continue running in the background — their processes stay alive and their output continues to accumulate.
- **FR-003**: When the user returns to a project that has background sessions, the system MUST reattach to and display those sessions with their live state, restoring the previously foreground session to the foreground and keeping the others in the background.
- **FR-004**: The system MUST provide a project switcher control in the top app bar, positioned immediately adjacent to the existing menu button.
- **FR-005**: The switcher MUST list the known projects and allow the user to make any listed project active in a single selection, without opening the folder-browser dialog.
- **FR-006**: The switcher MUST clearly indicate which project is currently active.
- **FR-007**: The switcher MUST indicate, per listed project, whether that project currently has running background sessions.
- **FR-008**: The switcher MUST indicate projects whose folders are unavailable or missing, and MUST NOT silently activate an unavailable project.
- **FR-009**: The switcher MUST provide access to the existing add-a-project flow (folder browser) for projects that are not yet known, complementing rather than replacing the existing switching entry points.
- **FR-010**: Sessions belonging to different projects MUST remain isolated while running concurrently in the background; no background session may leak filesystem, in-memory, or configuration state into another project's sessions.
- **FR-011**: When a background session's process exits unexpectedly while its project is inactive, the system MUST apply the same unexpected-exit handling as for foreground sessions (bounded automatic restart with the existing crash-loop guard), MUST reflect the resulting session state (running, restarting, or failed) when the user next views that project, and MUST notify the user on return that a background restart occurred rather than changing the session's state silently.
- **FR-012**: Output produced by a background session while its project was inactive MUST be preserved and visible when the user returns, subject to the same scrollback limit that applies to foreground sessions.
- **FR-013**: The system MUST allow multiple projects to hold running background sessions simultaneously, with no fixed cap on the number of such projects beyond available system resources.

### Key Entities *(include if feature involves data)*

- **Project**: A known working location the user can make active. Has an identity (its filesystem location), a display name, and an availability state (available vs. missing/unavailable). Exactly one project is active at a time.
- **Session**: A terminal session bound to one project. Has an identity and title that persist, and a live run state (idle, starting, running, restarting, failed) that does not persist across app restarts. A session is either foreground (currently displayed) or background (running but not displayed) for its project.
- **Active-project selection**: The single pointer to the currently active project, changed via the switcher or the existing switching entry points. Changing it must not stop the previously active project's sessions.
- **Project switcher**: The top-bar control, adjacent to the menu button, that lists known projects with active/running/unavailable indications and lets the user change the active project.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: A user can switch away from a project that has running sessions and later return, and 100% of those sessions are still running with no restart and no lost session identity.
- **SC-002**: Changing the active project from the top-bar switcher takes at most two interactions (open the switcher, select the project).
- **SC-003**: Output produced by a background session while its project was inactive is fully visible on return, with no lines lost within the applicable scrollback limit.
- **SC-004**: From the switcher alone, a user can correctly identify which known projects currently have running background sessions without opening any project.
- **SC-005**: Switching the active project displays the newly selected project within 1 second under normal conditions.
- **SC-006**: In a session where the user switches projects at least once while work is running, zero running sessions are stopped as a side effect of switching.
- **SC-007**: When a background session restarts or fails while its project is inactive, the user is informed of that change on returning to the project; the state never changes silently.

## Assumptions

- **Complements existing entry points**: The top-bar switcher is added alongside the existing "Known projects" body list and the folder-browser dialog; those remain available. The folder browser stays the way to add a not-yet-known project, reachable from the switcher.
- **Switcher contents**: Each switcher entry shows the project display name, an active marker for the current project, a running-background-sessions indicator (including a count) where applicable, and an unavailable indication for missing projects.
- **No artificial resource limits in this feature**: Background sessions of inactive projects run under the same conditions as foreground sessions; this feature introduces no separate throttling, suspension, or cap. Resource pressure from many concurrent sessions is governed by existing session behavior, not by new limits here.
- **Crash handling reuses existing behavior, plus a return notification**: Unexpected exit of a background session reuses the existing foreground unexpected-exit handling, including the existing crash-loop guard; no new restart policy is introduced. The only addition for background sessions is a notification, shown when the user returns to the affected project, that a background restart occurred (per the 2026-07-17 clarification).
- **"Background" is within one app run**: Keeping sessions alive across a switch applies within a single running application session. Live processes are not expected to survive an application restart; existing session persistence (identity and title, restored as resumable/idle) continues to govern restart behavior.
- **Single window**: The application continues to use a single main window; this feature does not introduce multiple windows. Switching changes what the single window shows.
