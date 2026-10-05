# Feature Specification: Workspace List of Sessions That Need Attention

**Feature Branch**: `claude/project-thread-8dnq8h`

**Created**: 2026-10-05

**Status**: Draft

**Input**: User description: "Implement GitHub issue #575 (Show sessions that need attention at workspace level). Feature 039 already tracks which sessions need attention (unread) and shows per-project counts on the switcher's rows and the other-projects total on its button. At workspace level, with several projects open, the user cannot see which projects and worktrees hold such a session without opening each project. Offer one place that lists all sessions needing attention across the workspace, each showing project, worktree and session name; selecting an entry opens that project and session the same way a desktop notification click does; the marks update live, clear when the user views the session, use the existing attention state rather than a second source of truth, and survive a client restart."

## What already exists, and the gap

Feature 039 (`specs/039-session-attention-notifications`) already ships most of what issue #575 asks for. This feature reuses it and changes none of it:

| Issue #575 asks for | Already shipped by 039 |
|---|---|
| Every project in the switcher with an unread session shows a mark with its count | FR-021, FR-022: each switcher row shows its project's unread count, the active project's row included; no count at zero |
| A total visible without opening the switcher | FR-023: the switcher's button shows the total for the other projects |
| Marks update live and clear when the session is viewed | FR-019 (within 1 second), FR-024 (same in every window) |
| One source of truth | The per-session **unread** state (039 Key Entities) |
| Marks survive a client restart | FR-008a |
| Opening a project and session the way a notification click does | FR-011 to FR-014 |

The gap this feature fills: **one place that lists every unread session of the workspace**, naming its project, worktree and session, from which the user opens any of them in one selection.

## Terms

- **Unread**, **in view**, **awaiting input**, **session service**: as defined by feature 039 (Terms and Key Entities). A session that "needs attention" in issue #575 is an unread session.
- **Known project**: a project the project switcher lists (feature 008, FR-005).
- **Attention list**: the list this feature adds.
- **Entry**: one row of the attention list, standing for one unread session.
- **Default entry**: the sidebar's entry for sessions that belong to no worktree, with the name the sidebar gives it.

## Clarifications

### Session 2026-10-05

- Q: Where does the attention list open from (FR-001)? → A: A section at the top of the project switcher's panel, above the project rows; the switcher's button, which already carries the other-projects unread total (039 FR-023), opens it. _(orchestrator default pending the user's answer)_
- Q: In which order must the attention list show its entries (FR-004)? → A: Grouped by project in the switcher's order, then worktree and session in the sidebar's order. Ordering by when a session became unread would need a per-session time 039 does not keep (`attention_seq` counts one session's events and cannot be compared across sessions), would add stored state FR-012 forbids, and would not survive the restart FR-014 requires. _(agent-resolved: specs/039-session-attention-notifications/data-model.md#Session attention fields; spec.md#FR-012)_

## User Scenarios & Testing *(mandatory)*

### User Story 1 - See every session that needs me, across all projects (Priority: P1)

The user works on several projects at once. Sessions in several of them have finished a turn while the user looked elsewhere. Without opening any project, the user opens the attention list and sees every unread session of the workspace, each naming its project, its worktree and the session, and nothing else.

**Why this priority**: This is the gap issue #575 names. The switcher's counts say how many unread sessions a project holds, not which worktrees and sessions; today the user has to open each project and scan its sidebar to find them.

**Independent Test**: Make sessions unread in two projects, one of them in a worktree and one in the Default entry, and leave a third project with none. Open the attention list and compare its entries with the sessions known to be unread.

**Acceptance Scenarios**:

1. **Given** project A has two unread sessions, project B has one and project C has none, **When** the user opens the attention list, **Then** it shows exactly three entries — the two of A and the one of B — and none of C.
2. **Given** an unread session in a worktree named `feature-x` of project A, **When** the attention list is shown, **Then** its entry names project A, worktree `feature-x` and the session by the label its sidebar row shows.
3. **Given** an unread session of project B that belongs to no worktree, **When** the attention list is shown, **Then** its entry names project B, the Default entry's name and the session's label.
4. **Given** the active project has an unread session, **When** the attention list is shown, **Then** that session has an entry too.
5. **Given** no session of the workspace is unread, **When** the user opens the attention list, **Then** it shows no entry and says that no session needs attention.
6. **Given** an unread session of a worktree that the sidebar's filter currently hides, **When** the attention list is shown, **Then** the session has an entry.

---

### User Story 2 - Go straight to a session from the list (Priority: P1)

The user selects an entry. The application makes the entry's project the active project and selects the session so that its terminal is shown, exactly as a click on that session's desktop notification would.

**Why this priority**: A list the user cannot act on still leaves them to navigate by hand; together with Story 1 it is the minimum that solves the issue.

**Independent Test**: With an unread session in a project other than the active one, open the attention list, select its entry, and check the active project, the selected session and the unread mark.

**Acceptance Scenarios**:

1. **Given** an entry for a session of project B while project A is active, **When** the user selects it, **Then** project B becomes active, the session is selected and its terminal is shown.
2. **Given** an entry for a session of the active project, **When** the user selects it, **Then** the active project stays, and the session is selected and shown.
3. **Given** the user selected an entry, **When** the session is shown in a focused window, **Then** the session is no longer unread (039 FR-019), its entry is gone from the attention list and its project's count on the switcher is one lower.
4. **Given** an entry whose session was closed or removed, its worktree deleted, its project forgotten or its folder unavailable, **When** the user selects it, **Then** the active project and selected session stay as they were and an in-app notice says the session is no longer available (as 039 FR-013).
5. **Given** sessions of the project left by the selection are running, **When** the user selects an entry, **Then** they keep running and no session is stopped, restarted or sent input (as 039 FR-014).

---

### User Story 3 - The list stays current on its own (Priority: P2)

While the attention list is open, sessions become unread and others come into view. The list follows without being reopened, and it shows the same sessions after the application is restarted.

**Why this priority**: Live and lasting state makes the list trustworthy; without it, it is a snapshot the user must refresh. It depends on Story 1.

**Independent Test**: Keep the list open while a session in another project changes to awaiting input, then view a listed session in another window; then quit and reopen the application with unread sessions present.

**Acceptance Scenarios**:

1. **Given** the attention list is open, **When** a session not in view changes to awaiting input, **Then** an entry for it appears within 1 second.
2. **Given** the attention list is open in one window, **When** a listed session comes into view in another window, **Then** its entry disappears from the list within 1 second.
3. **Given** sessions were unread when the last window was closed, **When** the application is opened again, also after a restart of the computer, **Then** the attention list shows them (039 FR-008a), and sessions that became unread while no window was open (039 FR-008) as well.

---

### Edge Cases

- **Empty workspace**: with no known project, or no unread session, the list shows no entry and the "no session needs attention" text; it never shows an error.
- **Many entries**: with more entries than fit, the list scrolls; every entry stays reachable by pointer and by keyboard.
- **A session changes while the user is about to select its entry**: if the session came into view or stopped being unread between the list being drawn and the selection, the selection still opens the session when it exists; when it no longer exists, the FR-013 notice of 039 is shown.
- **The active project's sessions**: listed like any other; selecting one only changes the selected session.
- **Several windows** (Principle II): one window at a time holds a project (feature 010). Every window's list shows the same entries (039 FR-024). Selecting an entry whose project another window holds brings that window to the front and shows the session there, as 039 FR-012 does for a notification click; otherwise the window in which the entry was selected switches to the project.
- **Several sessions with the same label**: each has its own entry; entries are told apart by project and worktree, and selecting one opens that session only.
- **Long names**: project, worktree and session names that do not fit are shortened so that the three remain distinguishable, and the full names are available on the entry (for example in its tooltip).
- **Connection to the session service lost**: the list shows the unread state the application last knew, as the sidebar and switcher do; it follows the state again once reconnected (039 FR-006).
- **Sessions run in a container** (sandbox): the list behaves the same as with sessions run directly on the computer.
- **Cross-platform** (Principle VI): the list, its keyboard access and its selection behave the same on Linux, macOS and Windows.

## Requirements *(mandatory)*

### Functional Requirements

#### The list

- **FR-001**: The application MUST offer an attention list as a section at the top of the project switcher's panel, above the project rows, in every window; it lists every unread session of every known project — the active project included — and no other session. Opening the switcher's panel shows the list; no other top-bar control is added. The list scrolls on its own when its entries do not fit (FR-006), so the project rows stay reachable below it.
- **FR-002**: The list's entries MUST include unread sessions of the Default entry and of worktrees the sidebar's filter currently hides, as 039 FR-022 does for the counts.
- **FR-003**: Each entry MUST name, as text, the session's project by the name the switcher gives it, the session's worktree by its name (the Default entry's name for a session in no worktree), and the session by the label its sidebar row shows at that moment, including a placeholder label (032).
- **FR-004**: The entries MUST be shown in a fixed, predictable order: grouped by project in the order the switcher lists the projects, and within a project by worktree, then session, in the order the sidebar shows them (the Default entry where the sidebar places it, worktrees the filter hides in the place they hold when shown). The order MUST NOT depend on when a session became unread.
- **FR-005**: With no unread session in the workspace, the list MUST show no entry and a short text saying that no session needs attention.
- **FR-006**: When the entries do not fit, the list MUST scroll, and every entry MUST stay reachable by pointer and by keyboard; a shortened name MUST be available in full on its entry.
- **FR-007**: The list MUST be operable by keyboard alone — opened, moved through, an entry selected, and closed — the keys moving between entries, Enter (or the platform's equivalent) selecting the focused entry, Escape closing the list, and keyboard focus returning to the control that opened it.

#### Selecting an entry

- **FR-008**: Selecting an entry MUST make the session's project the active project, when it is not already, and select the session so that its terminal is shown, through the same path a desktop notification click takes (039 FR-011), and MUST close the list.
- **FR-009**: When the session's project is held by another window, selecting the entry MUST bring that window to the front with keyboard focus and show the session there (as 039 FR-012). Otherwise the window in which the entry was selected MUST be used.
- **FR-010**: When the entry's session no longer exists or cannot be shown, selecting it MUST leave the active project and selected session as they were and show an in-app notice saying the session is no longer available (as 039 FR-013).
- **FR-011**: Selecting an entry MUST NOT stop, interrupt, restart or send input to any session (as 039 FR-014).

#### Staying current

- **FR-012**: The list MUST show exactly the sessions that are unread in feature 039's sense, and nothing about it may be stored or sent beyond what 039 stores (039 FR-025, Principle IV): no session is listed that the sidebar does not mark unread, and none the sidebar marks unread is missing. At every moment the list's entries, each switcher row's count (039 FR-021) and the button's total (039 FR-023) MUST agree.
- **FR-013**: While the list is shown, a session that becomes unread MUST gain an entry, and a session that stops being unread or is removed MUST lose its entry, within 1 second, without the user reopening the list.
- **FR-014**: The list MUST show the same sessions in every open window (039 FR-024), and after the application or the computer is restarted it MUST show the sessions that are unread then (039 FR-008, FR-008a).
- **FR-015**: Opening, scrolling or closing the list MUST NOT by itself make any session stop being unread; only a session coming into view does (039 FR-019).

#### Unchanged behaviour, components, documentation

FR-016 to FR-020 are checked by the regression tests of 039, the component showcase in both themes (FR-017, FR-018), a review of the user guide (FR-019), and SC-007 (FR-020).

- **FR-016**: The switcher's per-project unread counts, the button's other-projects total, the sidebar's unread mark, desktop notifications and their click MUST look and behave as they do today (039).
- **FR-017**: The attention list and its entries MUST be provided by the shared component library and used from there (Principle VIII); the component showcase MUST show the list with entries from two projects, with a worktree entry and a Default-entry entry, and the empty list.
- **FR-018**: The list's text and its focus and hover states MUST meet a contrast ratio of at least 4.5:1 for text (3:1 for non-text indicators) against their background in the light and the dark theme.
- **FR-019**: The user guide chapter that describes unread sessions MUST describe the attention list: where it opens, what an entry shows, what selecting one does, and that viewing a session removes it.
- **FR-020**: Everything above MUST behave the same on Linux, macOS and Windows (Principle VI), and with the session service running directly on the computer or in a container.

### Key Entities

- **Unread session** (existing, 039): a session with the unread state set. The attention list holds no data of its own; it is a view of the set of unread sessions.
- **Attention entry**: one unread session as the list shows it — project name, worktree name (or the Default entry's name), session label — and what selecting it leads to: that project and session.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: With unread sessions in two of three projects, in 20 of 20 trials the attention list shows exactly the unread sessions and no other, while the switcher shows the right count on the two projects and none on the third.
- **SC-002**: From any project, the user reaches any unread session of the workspace in at most two selections (open the list, select the entry), with its terminal shown within 2 seconds, in 20 of 20 trials — including when the session is in a background project.
- **SC-003**: Within 1 second of a listed session coming into view, its entry is gone and its project's switcher count is one lower, in 20 of 20 trials.
- **SC-004**: Within 1 second of a session becoming unread while the list is open, its entry appears, in 20 of 20 trials.
- **SC-005**: After closing and reopening the application with N unread sessions, the list shows the same N sessions in 20 of 20 trials.
- **SC-006**: A user asked "which sessions need you, in which project and worktree?" answers correctly from the attention list alone, without opening any project.
- **SC-007**: Every trial above gives the same result on Linux, macOS and Windows, and with the session service run directly and in a container.

## Out of Scope

- Changing what makes a session unread or what clears it (039 FR-016 to FR-019).
- Changing the switcher's per-project counts or the button's total (039 FR-021 to FR-023); they already meet the issue's first point.
- Marking sessions read from the list without viewing them, or a "mark all read" action.
- Showing in the list why a session waits (turn ended, permission, idle prompt) or a preview of its output.
- Filtering or searching the list.
- Unread counts in the **Known projects** list of the main window and in the folder browser (as 039).
- A taskbar or dock badge, a tray icon or sound.

## Assumptions

- "Needs attention" in issue #575 is 039's unread state: a session that changed to awaiting input while not in view and has not been viewed since.
- The issue's first point and its first, fourth and fifth acceptance criteria are met by 039 already; this feature keeps them true and adds the list (second and third criteria).
- Selecting an entry behaves like a notification click because the issue says so; the only difference is which window is used when no window holds the project (FR-009): the user is in the window where they selected, so that one is used rather than the most recently focused one.
- The list lists unread sessions only. Sessions awaiting input that the user has already viewed are not listed: they do not need attention in 039's sense.
- **Dependencies**: 039 unread state, reveal path and counts; 008 project switcher; 010 one window per project; 032 session labels; the shared component library and showcase (Principle VIII).
