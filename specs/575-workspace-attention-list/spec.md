# Feature Specification: Attention Indicator on the Sidebar's Worktree Rows

**Feature Branch**: `claude/project-thread-8dnq8h`

**Created**: 2026-10-05

**Status**: Draft

**Input**: User description: "Implement GitHub issue #575 (Show sessions that need attention at workspace level)." Scope changed by the user after the first spec: "the clue was to add indicator of attention at sidebar with the list of worktrees". In the sidebar's list of worktrees, each worktree row, and the Default (project-root) row, that holds at least one unread session (feature 039's sense) shows an attention indicator with the number of such sessions, so the user sees which worktrees need attention without expanding them. Reuse 039's unread state and the shared unread mark and tree row components; no second source of truth; the indicator updates live, clears when the session is viewed and survives a restart as 039's state does.

## What already exists, and the gap

Feature 039 (`specs/039-session-attention-notifications`) marks each unread **session row** in the sidebar (FR-018), shows each project's unread count on the project switcher's rows (FR-021, FR-022) and the other projects' total on the switcher's button (FR-023). This feature reuses that state and those components.

The gap: a session row is shown only while its worktree row is expanded. With worktree rows collapsed, the sidebar does not say which worktrees hold an unread session; the user has to expand each one to find out. This feature puts an indicator with a count on the **location row** (worktree or Default) itself.

## Terms

- **Unread**, **in view**, **awaiting input**, **session service**: as defined by feature 039 (Terms and Key Entities). A session that "needs attention" in issue #575 is an unread session.
- **Location row**: a row of the sidebar's location list (feature 010) — the Default row (sessions that belong to no worktree, at the project root) or a worktree row.
- **Closed session**: a session the user closed (feature 005 FR-015a, as amended by its bugfix BUG-003). Its record is kept but it is never shown in the sidebar again.
- **Counted session**: a session of a location that is unread, not closed, and not the session the window has in view.
- **Attention indicator**: the shared unread mark followed by the number of counted sessions of a location row (039 contract `unread-mark.md`, the form the switcher's rows use).

## Clarifications

### Session 2026-10-05

- Scope changed by the user: "the clue was to add indicator of attention at sidebar with the list of worktrees". The earlier workspace attention list in the switcher's panel (earlier FR-001 to FR-015, and the clarify answers on its placement and entry order) is dropped.
- Q: Does the indicator show on an expanded location row, whose session rows already carry their own marks? → A: Yes. The indicator shows whether the row is expanded or collapsed; the session rows keep their marks unchanged. _(orchestrator default)_
- Q: Do closed sessions that were unread count? → A: No, not on the location row and not in the switcher. The service does not clear `unread` when a session is closed, so a closed session can still carry it; 039 US1 scenario 9 already says a closed session's project count must no longer include it. This feature makes the switcher's counts skip closed sessions too, so the location rows of a project always add up to its switcher count. _(agent-resolved: specs/039-session-attention-notifications/spec.md US1 scenario 9; crates/micold-core/src/workspace.rs `unread_session_count` filters on `unread` only; crates/micold-core/src/session.rs `archived`)_
- Q: Which hidden worktree rows does "the sidebar's filter" cover in the edge cases and FR-010? → A: Both mechanisms that keep a worktree out of the location list: the tag filters (feature 008 FR-025) and the setting that hides agent-owned worktrees (feature 014). A worktree hidden either way has no row and no indicator, its unread sessions still count on the switcher, and the row re-admitted for the current session (feature 024) shows its indicator. _(agent-resolved: crates/micold-client/src/features/sidebar.rs `filtered_worktree_tree` ("Two mechanisms hide a row") and `visible_worktrees`; crates/micold-core/src/workspace.rs `unread_session_count` counts every session of the project)_

## User Scenarios & Testing *(mandatory)*

### User Story 1 - See which worktrees need me without expanding them (Priority: P1)

The user works in a project with several worktrees, their rows collapsed. Sessions in some of them finished a turn while the user looked elsewhere. Each worktree row holding such a session, and the Default row when it holds one, shows the attention indicator with the number of those sessions. Rows with none show nothing new.

**Why this priority**: This is the gap the user named. Without it the user expands each worktree to find the unread sessions.

**Independent Test**: In one project, make two sessions of worktree `feature-x` unread, one session of the Default row unread, and leave worktree `feature-y` with only read sessions; collapse every row and compare the indicators with the sessions known to be unread.

**Acceptance Scenarios**:

1. **Given** worktree `feature-x` holds two unread sessions, worktree `feature-y` holds none and its rows are collapsed, **When** the user looks at the sidebar, **Then** the `feature-x` row shows the indicator with `2` and the `feature-y` row shows no indicator.
2. **Given** one session of the Default row is unread, **When** the user looks at the sidebar, **Then** the Default row shows the indicator with `1`.
3. **Given** worktree `feature-x` with two unread sessions is expanded, **When** the user looks at the sidebar, **Then** its row still shows the indicator with `2`, and each of the two session rows carries its own unread mark as today (039 FR-018).
4. **Given** a worktree holds one unread session and one session awaiting input that the user has already viewed, **When** the user looks at the sidebar, **Then** its row shows the indicator with `1`.
5. **Given** a session of worktree `feature-x` was unread and the user closed it, **When** the user looks at the sidebar, **Then** the session does not add to `feature-x`'s indicator, and the project's count on the switcher does not include it either.

---

### User Story 2 - The indicator follows the sessions on its own (Priority: P1)

While the user works, sessions become unread and others are viewed. The indicators change without any action, clear when the last unread session of a location is viewed, and show the same after the application is restarted.

**Why this priority**: An indicator that lags or survives the session being viewed sends the user to worktrees that need nothing; with Story 1 it is the minimum that solves the issue.

**Independent Test**: With worktree `feature-x` collapsed, have one of its sessions change to awaiting input while another session is in view; then select that session; then make it unread again and restart the application.

**Acceptance Scenarios**:

1. **Given** worktree `feature-x` shows no indicator, **When** one of its sessions not in view changes to awaiting input, **Then** its row shows the indicator with `1` within 1 second.
2. **Given** `feature-x` shows the indicator with `1`, **When** the user selects that session and it comes into view in a focused window, **Then** the indicator is gone within 1 second (039 FR-019).
3. **Given** `feature-x` shows the indicator with `2`, **When** one of the two sessions comes into view, **Then** the indicator shows `1`.
4. **Given** the same project is shown in another window (039 FR-024), **When** a session comes into view in one window, **Then** the other window's indicator for its location changes within 1 second as well.
5. **Given** sessions were unread when the last window was closed, **When** the application is opened again, also after a restart of the computer, **Then** their location rows show the same indicators (039 FR-008a), and sessions that became unread while no window was open (039 FR-008) are counted as well.

---

### Edge Cases

- **The session in view**: the session the window has in view is never counted, even before the service has cleared its `unread` state (039 FR-019 reads it at once), so its location's indicator drops as soon as it is shown.
- **A location whose only unread sessions were closed**: no indicator.
- **A burst of changes** (Principle II): when several sessions of one location become unread or come into view within the same second, the indicator settles on the right count within 1 second of the last change.
- **A project with no worktrees**: only the Default row exists, and it carries the indicator.
- **A worktree the sidebar hides** (by its tag filters, feature 008, or by hiding agent-owned worktrees, feature 014): it has no row, so there is no indicator to show; its unread sessions still count on the project's switcher row (039 FR-022). A worktree row listed only because it holds the current session (feature 024) shows its indicator like any other.
- **A worktree that is missing or invalid** (feature 010 FR-011): shows its indicator like a valid one when it holds counted sessions.
- **Many unread sessions**: counts of two or more digits are shown in full; the row keeps its height (039 contract `unread-mark.md` U8), and the name is what a narrow row shortens.
- **Hover**: the row actions that fade in on hover (feature 008) do not cover or move the indicator.
- **Collapsed sidebar**: no indicator is shown elsewhere; the switcher's counts are unchanged.
- **Connection to the session service lost**: the indicators show the unread state the application last knew, as the session rows do; they follow the state again once reconnected (039 FR-006).
- **Sessions run in a container** (sandbox): the indicators behave the same as with sessions run directly on the computer.
- **Cross-platform** (Principle VI): the indicators behave the same on Linux, macOS and Windows.

## Requirements *(mandatory)*

### Functional Requirements

#### The indicator

- **FR-001**: Each location row of the sidebar — every worktree row and the Default row — MUST show the attention indicator when its location holds at least one counted session, and MUST show no indicator when it holds none.
- **FR-002**: The indicator's number MUST be the number of counted sessions of that location: sessions of the location that are unread in feature 039's sense, not closed, and not the session the window has in view.
- **FR-003**: The indicator MUST show whether the location row is expanded or collapsed. Expanding or collapsing a row MUST NOT change its indicator, and the session rows MUST keep their own unread marks as they are (039 FR-018).
- **FR-004**: The indicator MUST be the shared unread mark with a count from the component library (Principle VIII; 039 contract `unread-mark.md`), in the form the switcher's rows use, and MUST stay visible when the row's name is shortened. The location row MUST keep its height, and the row actions that fade in on hover MUST NOT cover or shift it.
- **FR-005**: The indicator MUST state its meaning to assistive technology and in the row's tooltip, for example "2 unread sessions".

#### Staying current

- **FR-006**: The indicators MUST be derived from the same per-session unread state that marks the session rows and counts the switcher's rows (039 Key Entities), and nothing about them may be stored or sent beyond what 039 stores (039 FR-025, Principle IV).
- **FR-007**: A session that becomes unread MUST add to its location's indicator, and a session that stops being unread, comes into view, is closed or is removed MUST stop adding to it, within 1 second, without any user action.
- **FR-008**: The indicators MUST agree in every open window that shows the project (039 FR-024), and after the application or the computer is restarted they MUST show the sessions unread then (039 FR-008, FR-008a).
- **FR-009**: Showing, expanding, collapsing or hovering a location row MUST NOT by itself make any session stop being unread; only a session coming into view does (039 FR-019).

#### Consistency with the switcher

- **FR-010**: A closed session MUST NOT be counted in the switcher's per-project counts (039 FR-021) or the button's total (039 FR-023), as 039 US1 scenario 9 requires. For a project none of whose worktrees the sidebar hides (tag filters or hidden agent-owned worktrees), the indicators of its location rows MUST add up to that project's count on the switcher's row in the same window.

#### Unchanged behaviour, components, documentation

FR-011 to FR-014 are checked by the regression tests of 039, the component showcase in both themes (FR-012, FR-013), a review of the user guide (FR-014), and SC-006.

- **FR-011**: The session rows' unread marks, the switcher's rows and button (apart from FR-010), desktop notifications and their click MUST look and behave as they do today (039).
- **FR-012**: The component showcase MUST show a location row with the indicator, collapsed and expanded, beside one without, in the light and the dark theme.
- **FR-013**: The indicator MUST meet a contrast ratio of at least 3:1 for the mark and 4.5:1 for its number against the row's background in every row state (rest, hover, selected) in the light and the dark theme.
- **FR-014**: The user guide chapter that describes unread sessions MUST describe the indicator on worktree and Default rows: what it counts, that it shows on collapsed and expanded rows, and that viewing a session lowers it.
- **FR-015**: Everything above MUST behave the same on Linux, macOS and Windows (Principle VI), and with the session service running directly on the computer or in a container.

### Key Entities

- **Unread session** (existing, 039): a session with the unread state set. The indicator holds no data of its own; it is a count over the sessions of one location.
- **Location attention count**: for one location row, the number of its counted sessions. Derived on display; never stored.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: With unread sessions in two of three worktrees of a project and every row collapsed, in 20 of 20 trials the two rows show the right count and the third shows none.
- **SC-002**: In 5 of 5 trials, a user asked "which worktrees have sessions waiting for you?" answers correctly from the collapsed sidebar alone, without expanding any row.
- **SC-003**: Within 1 second of a counted session coming into view, its location's indicator is one lower (or gone at zero), in 20 of 20 trials.
- **SC-004**: Within 1 second of a session becoming unread, its location's indicator is one higher, in 20 of 20 trials.
- **SC-005**: After closing and reopening the application with N unread sessions spread over several locations, every location row shows the same count as before, in 20 of 20 trials, and the counts of a project's rows add up to its switcher count.
- **SC-006**: Every trial above gives the same result on Linux, macOS and Windows, and with the session service run directly and in a container.

## Out of Scope

- A workspace-wide list of unread sessions, in the switcher or elsewhere (dropped by the user's change of scope).
- Changing what makes a session unread or what clears it (039 FR-016 to FR-019), apart from no longer counting closed sessions (FR-010).
- An indicator on a project's name in the sidebar, or anywhere outside the location rows.
- Marking sessions read without viewing them, or a "mark all read" action.
- Showing why a session waits (turn ended, permission, idle prompt).
- A taskbar or dock badge, a tray icon or sound.

## Assumptions

- "Needs attention" in issue #575 is 039's unread state: a session that changed to awaiting input while not in view and has not been viewed since.
- The issue's switcher marks and button total are already met by 039; this feature keeps them and adds the indicator on location rows.
- A closed session is not shown in the sidebar and cannot be viewed, so counting it would leave an indicator the user can never clear.
- **Dependencies**: 039 unread state, unread mark and counts; 010 sidebar location list (Default and worktree rows); 024 filter and current-session rows; 005 FR-015a closed sessions; the shared component library and showcase (Principle VIII).
