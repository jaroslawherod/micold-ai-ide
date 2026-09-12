# Feature Specification: Refresh the Worktree List on Demand

**Feature Branch**: `feat/refresh-worktrees-list`

**Created**: 2026-08-31

**Status**: Draft

**Input**: User description: "Add to the buttons to hide sidebar and create new worktree a new button to refresh a list of worktrees on demand"

## Context: What This Feature Is

The sidebar's worktree list is not live. It is rebuilt only at a fixed set of moments: when a
project is opened or activated, and after the application itself creates, deletes, renames,
includes, or excludes a worktree. Nothing watches the repository, so a worktree that appears by any
other route — a `git worktree add` typed in a terminal, a coding agent that makes its own
workspace, a branch pulled and checked out elsewhere, a directory removed by hand — is invisible
until one of those moments comes round again. Today the only reliable way to force one is to switch
projects away and back.

This feature gives the user a way to ask for that rebuild directly: a third control in the sidebar
header, next to the two that are already there ("Add a worktree" and "Hide sidebar").

The scope is the **on-demand** ask. Automatic refresh — polling, watching the repository, or a
timer — is deliberately not part of this feature.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - See a worktree that appeared outside the app (Priority: P1)

A user has the project open in the sidebar. In a terminal, or through an agent, a new worktree is
created for the repository. The sidebar does not show it. The user presses the refresh control in
the sidebar header and the new worktree appears in the list, ready to start a session in.

**Why this priority**: This is the entire reason the control exists. Without it the user's only
remedy is to leave the project and come back, and many users do not know that works.

**Independent Test**: Open a project, create a worktree for its repository by any means outside the
application, press refresh, and confirm the new worktree is listed. Delivers the feature's whole
value on its own.

**Acceptance Scenarios**:

1. **Given** a project is open and a worktree exists on disk that the sidebar is not showing,
   **When** the user activates the refresh control, **Then** that worktree appears in the list
   without the user leaving or reopening the project.
2. **Given** a project is open and a worktree the sidebar is showing has been removed outside the
   application, **When** the user activates the refresh control, **Then** that worktree disappears
   from the list.
3. **Given** a project is open and a listed worktree's branch was changed outside the application,
   **When** the user activates the refresh control, **Then** the row shows the branch the
   repository currently reports.
4. **Given** nothing about the project's worktrees has changed, **When** the user activates the
   refresh control, **Then** the list is unchanged and no error is shown.

---

### User Story 2 - Know that the refresh happened (Priority: P2)

The common case is that a refresh changes nothing visible. A control that appears to do nothing is
indistinguishable from a broken one, so the user needs to see that the request was taken, that it
is running, and that it finished.

**Why this priority**: Without it US1 still works, but a user who presses the control on an
unchanged project cannot tell whether the app checked or ignored them, and will press it again.

**Independent Test**: Press refresh on a project whose worktrees have not changed and confirm the
control visibly reports in-progress and then completion, with the list intact.

**Acceptance Scenarios**:

1. **Given** a project is open, **When** the user activates the refresh control, **Then** the
   control immediately shows that a refresh is in progress.
2. **Given** a refresh is in progress, **When** the refreshed list arrives, **Then** the control
   returns to its idle appearance and the list reflects the new listing.
3. **Given** a refresh is in progress, **When** the user activates the control again, **Then** no
   second refresh is requested and the in-progress state continues uninterrupted.
4. **Given** a refresh cannot complete — the project's directory is gone, the repository cannot be
   read, or the request fails — **When** the failure is known, **Then** the user is told through
   the application's existing error surface and the control returns to idle so it can be retried.
5. **Given** a refresh was requested but no answer ever arrives, **When** a bounded wait elapses,
   **Then** the control returns to idle rather than remaining in-progress indefinitely.

---

### User Story 3 - Refresh without losing my place (Priority: P3)

A user with an expanded tree, an active session, a filter applied, and a scrolled list presses
refresh. Everything that is still valid stays exactly as it was; only what the repository actually
reports differently changes.

**Why this priority**: A refresh that collapses the tree, clears filters, or drops the user's
session selection would be worse than the workaround it replaces — users would avoid the control.

**Independent Test**: Set up an expanded, filtered, scrolled sidebar with an active session, press
refresh on an unchanged project, and confirm nothing about that arrangement moved.

**Acceptance Scenarios**:

1. **Given** worktree rows are expanded, a tag filter is applied, the list is scrolled, and a
   session is active, **When** a refresh returns the same worktrees, **Then** expansion, filters,
   scroll position, selection, and the active session are all unchanged.
2. **Given** a worktree that is expanded and selected no longer exists, **When** the refreshed list
   arrives, **Then** it is removed from the list and the state that referenced it is reconciled the
   same way the application already reconciles a worktree that disappears.
3. **Given** a filter is applied that the newly discovered worktree does not match, **When** the
   refresh completes, **Then** the new worktree is discovered but stays hidden by the filter,
   consistent with how the list already treats filtered worktrees.

---

### Edge Cases

- **No project open**: the sidebar has no worktree list to refresh, so the control must not offer
  an action that cannot do anything.
- **Sidebar hidden**: the header and its controls are not on screen. Refreshing is unavailable
  until the sidebar is shown again; the collapsed strip gains no new control.
- **Project directory missing or not a repository**: the refresh yields an empty listing rather
  than a stale one, and the user is told why.
- **The application is disconnected**: the request cannot be carried out; the user sees the
  application's existing disconnected treatment and the control returns to idle.
- **A worktree operation is already running** (create, delete, rename, include, exclude): that
  operation refreshes the list when it completes, so a refresh requested at the same time must not
  produce a conflicting or duplicated list.
- **Rapid repeated activation**: at most one refresh is in flight for a project at a time.
- **The active session's worktree vanishes**: handled by the application's existing reconciliation
  for a worktree that disappears — this feature introduces no new rule for it.
- **A very large repository**: a refresh that takes noticeable time must not freeze the interface;
  the rest of the application stays usable while it runs.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: The sidebar header MUST present a refresh control alongside the existing "Add a
  worktree" and "Hide sidebar" controls, in the same visual treatment and size as those controls.
- **FR-002**: The refresh control MUST carry an explanatory label on hover, in the same manner as
  the neighbouring controls, stating that it re-reads the project's worktrees.
- **FR-003**: Activating the refresh control MUST cause the active project's worktree listing to be
  re-read from the repository and the filesystem, and the sidebar to be rebuilt from the result.
- **FR-004**: A refreshed listing MUST be reconciled with what is already on screen — selection,
  expansion, open menus, and session references — exactly as an equivalent listing arriving for any
  other reason would be; a user MUST NOT be able to tell from the result which trigger produced it.
- **FR-005**: The refresh control MUST be unavailable (visibly non-actionable) when no project is
  open.
- **FR-006**: While a refresh is in progress the control MUST show that state and MUST NOT start a
  second refresh for the same project.
- **FR-007**: The control MUST return to its idle state when the refreshed listing arrives, when
  the attempt fails, or when a bounded wait elapses without an answer.
- **FR-008**: A refresh that fails MUST report the reason through the application's existing error
  surface, and MUST leave the previously shown listing in place rather than emptying the sidebar.
- **FR-009**: A refresh that finds no change MUST leave the sidebar's arrangement — expansion,
  filters, scroll position, selection, and the active session — untouched.
- **FR-010**: The interface MUST remain responsive while a refresh runs; reading the repository
  MUST NOT block interaction with the rest of the application.
- **FR-011**: The refresh MUST apply to the active project only, and MUST NOT disturb worktree
  listings or sessions belonging to other projects or other open windows.
- **FR-012**: The feature MUST NOT introduce automatic, periodic, or filesystem-triggered refresh;
  the listing is re-read only when the user asks or when an existing trigger already fires.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: A user who creates a worktree outside the application can make it appear in the
  sidebar with a single action, without closing, switching, or reopening the project.
- **SC-002**: On a repository with up to 50 worktrees, the refreshed list is on screen within 2
  seconds of the user's action in the common case, and the application accepts other input
  throughout.
- **SC-003**: In 100% of refreshes that return an unchanged listing, the sidebar's expansion,
  filters, scroll position, selection, and active session are identical before and after.
- **SC-004**: Every activation of the control produces visible feedback within 100 ms — the user is
  never left unsure whether the request was taken.
- **SC-005**: Repeatedly activating the control while a refresh is running never results in more
  than one refresh in flight for that project.
- **SC-006**: A user shown the sidebar header for the first time can identify which control
  refreshes the list from its icon and hover label, without consulting documentation.

## Assumptions

- **Placement and order**: the control joins the existing header cluster to the left of "Add a
  worktree", so the two pre-existing controls keep their positions and muscle memory. It uses the
  same compact icon-button treatment and a neutral tint, matching "Hide sidebar" rather than the
  accented "Add a worktree" — refreshing is a routine action, not the header's primary one.
- **Scope of the re-read**: activating the control performs the same project-level re-read the
  application already performs when a project is opened. Where that existing pass also picks up
  sessions started outside the application, this feature inherits that behaviour rather than
  carving out a worktree-only variant; it is the same operation, asked for at a different moment.
- **No new keyboard shortcut** is introduced. The control is reachable by pointer like its
  neighbours; a shortcut can be considered separately.
- **No new persistent state**: nothing about the refresh is remembered across restarts, and the
  refreshed listing is not cached anywhere the existing listing is not.
- **Existing surfaces are reused** for progress and error reporting; this feature defines no new
  notification, dialog, or banner.
- **The collapsed sidebar strip is unchanged** — it hosts only the "show sidebar" control, and
  gains nothing here.
- **The repository is reachable wherever the listing is read today**; this feature changes when the
  listing is read, not from where.
