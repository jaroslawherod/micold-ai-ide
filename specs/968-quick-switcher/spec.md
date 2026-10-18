# Feature Specification: Quick-Switcher to Jump to a Project, Location or Session

**Feature Branch**: `claude/project-thread-ftdy5c`

**Created**: 2026-10-18

**Status**: Draft

**Input**: User description: "Issue #968: Quick-switcher to jump directly to a project, worktree or
session. Problem: #803 adds next/previous keyboard navigation for projects, worktrees and sessions.
Jumping straight to a specific one still needs the mouse or several steps. Proposal: a
keyboard-opened, filterable quick-switcher (fuzzy-find palette) to jump directly to any project,
worktree or session. Split out of #803 (decision D5 in `specs/803-keyboard-navigation/spec.md`),
where 'jump directly' was put out of scope."

## Terms

The terms **Location**, **Location order**, **Session order**, **Project order** and **Current**
are those of `specs/803-keyboard-navigation/spec.md#terms`. In addition:

- **Quick-switcher**: the filterable list this feature adds, opened from the keyboard.
- **Entry**: one row of the quick-switcher. It is a project, a location or an open (not closed)
  session.
- **Query**: the text typed into the quick-switcher's filter field.
- **Highlighted entry**: the one entry that Enter chooses.
- **Switcher chord**: the key combination of FR-001.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Jump to a location of the active project (Priority: P1)

A developer with many worktrees in the active project wants to go straight to one of them by typing
part of its name, without the mouse and without stepping through the others one press at a time.

**Why this priority**: worktrees are the level users switch most (see #803, User Story 1), and the
next/previous chords take up to ⌊N/2⌋ presses; typing a few letters of the name takes the same few
keys whatever N is. This story alone is a usable quick-switcher.

**Independent Test**: open a project with Default and ten worktrees, each with a session. With the
terminal focused, press the switcher chord, type three letters of the seventh worktree's name, press
Enter, and check which session the terminal shows and which sidebar row is selected.

**Acceptance Scenarios**:

1. **Given** a terminal with keyboard focus, **When** the user presses the switcher chord, **Then**
   the quick-switcher opens with an empty query and keyboard focus in its filter field, and the
   terminal receives no bytes for that press.
2. **Given** the quick-switcher is open with an empty query, **When** the user looks at it, **Then**
   it starts with every location of the active project in location order (full order: FR-004),
   each showing the name and tags
   the sidebar shows for it, with the current location marked, and the first entry highlighted.
3. **Given** the quick-switcher is open, **When** the user types `logpg`, **Then** only entries whose
   name contains the letters l, o, g, p, g in that order (ignoring case) remain, for example
   **Login page**, and the best match (FR-005) is highlighted.
4. **Given** a location entry is highlighted, **When** the user presses Enter, **Then** the
   quick-switcher closes and that location becomes current exactly as if its sidebar row had been
   clicked: the session the sidebar opens for it shows in the terminal, its row is selected and
   scrolled into view, and the keyboard follows FR-010.
5. **Given** the quick-switcher is open, **When** the user presses Up or Down, **Then** the
   highlight moves to the entry above or below, and it scrolls into view in a list longer than the
   panel.
6. **Given** the quick-switcher is open, **When** the user presses Escape, presses the switcher chord
   again or clicks outside it, **Then** it closes, nothing else changes, and keyboard focus returns
   to where it was before it opened.
7. **Given** the chosen location has no session, **When** it becomes current from the
   quick-switcher, **Then** it shows its start-session action as it does when clicked, no session
   is started, and the sidebar's keyboard focus is on its row.
8. **Given** the quick-switcher is open, **When** the user clicks an entry, **Then** it is chosen as
   Enter would choose it.

---

### User Story 2 - Jump to a session (Priority: P2)

A developer who runs helper sessions beside worktrees' main sessions wants to go straight to one
session by its name, wherever it is in the sidebar.

**Why this priority**: helper sessions are opt-in, so fewer users have many sessions per location,
but a session's name (the conversation's name) is often what the user remembers.

**Independent Test**: in a project where three locations each hold a main session and two helpers,
open the quick-switcher, type part of the name of a helper in the third location, press Enter, and
check that this helper shows and its sub-row is selected under an opened row.

**Acceptance Scenarios**:

1. **Given** the quick-switcher is open with an empty query, **When** the user looks at it, **Then**
   below each location entry it lists that location's open sessions in session order, each showing
   the name its sidebar sub-row shows and its location's name.
2. **Given** a query that matches a session's name, **When** the user chooses that session, **Then**
   it becomes current exactly as if its sidebar sub-row had been clicked: it shows in the terminal,
   its location's row is opened, and its sub-row is selected and scrolled into view.
3. **Given** a query that matches a location's name, **When** the list filters, **Then** that
   location's sessions remain listed as well, so typing a worktree's name and then moving down
   reaches its helpers.
4. **Given** a session closed in the sidebar, **When** the quick-switcher lists entries, **Then**
   that session is not listed.

---

### User Story 3 - Jump to a project (Priority: P3)

A developer with several known projects wants to switch to one by typing part of its name.

**Why this priority**: switching projects is less frequent, and the top bar's project switcher
already does it in two clicks.

**Independent Test**: with five known projects (one unavailable), open the quick-switcher, type part
of the fourth project's name, press Enter, and check that it becomes active; then check that the
unavailable project is listed but cannot be chosen.

**Acceptance Scenarios**:

1. **Given** the quick-switcher is open with an empty query, **When** the user looks at it, **Then**
   after the active project's entries it lists the other known projects in project order, each
   showing its name and the running and unread counts the project switcher shows for it.
2. **Given** a project entry is highlighted, **When** the user presses Enter, **Then** that project
   becomes active exactly as if it had been picked in the project switcher: the sessions of the
   project left keep running, and the new project's sidebar and current session are restored.
3. **Given** a project marked unavailable, **When** the quick-switcher lists it, **Then** it is shown
   with its **unavailable** badge, Up and Down skip it, and clicking it does nothing.
4. **Given** the active project, **When** the quick-switcher lists projects, **Then** the active
   project is not offered as a project entry to switch to (its locations and sessions are listed
   above).
5. **Given** a session or location of a project that is not active, **When** the quick-switcher
   lists entries, **Then** [NEEDS CLARIFICATION: are the locations and sessions of the other known
   projects listed too (choosing one switches project, then selects it), or only those of the
   active project?].

---

### User Story 4 - Find the shortcut (Priority: P3)

A user wants to learn the switcher chord from the documentation.

**Why this priority**: a chord is only useful once known; the user guide is the app's shortcut list.

**Independent Test**: read every user-guide list of keyboard shortcuts and find the switcher chord in
its Linux/Windows and macOS forms.

**Acceptance Scenarios**:

1. **Given** the user guide, **When** the user reads any place that lists the app's keyboard
   shortcuts, **Then** the switcher chord is listed with both platform forms, and the quick-switcher
   (what it lists, how to filter and choose) is described.

---

### Edge Cases

- **Empty lists**: no project open (welcome screen): the quick-switcher lists the known projects
  only. No known project: it opens and says there is nothing to switch to. A query that matches
  nothing: it shows "No matches", and Enter does nothing and keeps it open.
- **Empty query**: lists everything in the order of FR-004; Enter chooses the first entry.
- **Missing or invalid worktree**: listed with its `missing` or `invalid` tag, like any location;
  choosing it does exactly what clicking it does (error shown, no session started).
- **Tag filter**: [NEEDS CLARIFICATION: are worktrees the sidebar's tag filter hides listed in the
  quick-switcher (and does choosing one show it in the sidebar), or skipped as the #803 chords skip
  them?].
- **Lists changing while open (Principle II)**: a session or worktree created, renamed, closed or
  removed while the quick-switcher is open, in the background, by another window or by an
  assistant, appears in or leaves the list without closing it. The highlight stays on the same entry
  while it is still listed, and moves to the first entry when it is not. Choosing never starts,
  stops or closes a session; background sessions keep running.
- **An entry that goes away as it is chosen**: if the chosen entry no longer exists when Enter is
  processed, nothing changes and the quick-switcher stays open with the updated list.
- **Settings, a modal dialog, a context menu, the project switcher panel or the filter panel open**:
  the switcher chord does nothing. While the quick-switcher is open, the #803 navigation chords and
  the other app chords that change the selection do nothing.
- **Overview open**: choosing a location or session closes the Overview and shows it, as a sidebar
  selection does. Choosing a project leaves the Overview as switching project already does.
- **Several windows**: each window has its own quick-switcher; opening or choosing in one window
  changes nothing in another.
- **Long names**: an entry's name is shown on one line, cut with "…" at the panel's width; the full
  name is still matched.
- **Many entries**: with hundreds of locations and sessions, the list scrolls inside the panel and
  filtering keeps pace with typing (SC-003).
- **Text input**: the filter field accepts every character the keyboard and input method produce
  (non-ASCII letters, composed characters, paste); matching ignores case.
- **Terminal programs**: the switcher chord is reserved under the app's convention (primary modifier
  plus Shift, taken from the terminal), the same trade-off as Ctrl+Shift+O; the same key without
  Shift keeps reaching the terminal.
- **Keyboard layouts**: the switcher chord is recognised by its physical key (the P key position on
  a US layout), whatever character the layout produces.
- **Cross-platform (Principle VI)**: Cmd on macOS where Linux and Windows use Ctrl; behaviour is the
  same on all three. The chord is none of the reserved system shortcuts of
  `specs/803-keyboard-navigation/spec.md#FR-006` on any of them.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: The app MUST open the quick-switcher on **Ctrl+Shift+P** on Linux and Windows and
  **Cmd+Shift+P** on macOS, both while a terminal has keyboard focus and while it does not. A press
  the app acts on MUST NOT reach the terminal (0 bytes). The same chord MUST close an open
  quick-switcher.
- **FR-002**: The switcher chord MUST NOT equal a reserved system shortcut on macOS, Windows or
  GNOME (the lists of `specs/803-keyboard-navigation/spec.md#FR-006`), an input-method shortcut, or
  an existing app chord (Ctrl/Cmd+Shift+E, T, D, H, W, C, V, O, Arrows, [ ] , . PageUp PageDown).
- **FR-003**: The quick-switcher MUST list, as entries: the locations of the active project; the
  open sessions of each of those locations; and the known projects other than the active one. A
  closed session MUST NOT be listed.
- **FR-004**: With an empty query, entries MUST be listed in this order: each location of the active
  project in location order, each followed by its open sessions in session order; then the other
  known projects in project order.
- **FR-005**: A non-empty query MUST keep exactly the entries whose name contains the query's
  characters in order, ignoring case (a location's sessions also stay when the location's name
  matches). Remaining entries MUST be ranked: a name that starts with the query first, then a name
  that contains it as one run of characters, then the other matches; within a rank, the FR-004
  order. The first remaining entry MUST be highlighted after each change to the query.
- **FR-006**: Each entry MUST show its kind (project, location or session), the name the sidebar or
  project switcher shows for it, and, for a session, its location's name; a location also shows its
  tags; a project its running and unread counts and its **unavailable** badge. The current location
  and current session MUST be marked.
- **FR-007**: Up and Down MUST move the highlight to the next selectable entry above or below,
  stopping at the first and last entry, and keep it in view. Enter MUST choose the highlighted
  entry; a click MUST choose the clicked entry.
- **FR-008**: Choosing a location, session or project MUST have exactly the result of clicking its
  sidebar row or sub-row, or picking the project in the project switcher, and MUST close the
  quick-switcher. An unavailable project MUST NOT be choosable.
- **FR-009**: Escape, the switcher chord, or a click outside the quick-switcher MUST close it without
  changing the current project, location or session.
- **FR-010**: After choosing, when a terminal had keyboard focus before the quick-switcher opened and
  the choice shows a terminal, that terminal MUST have keyboard focus; otherwise the sidebar's
  keyboard focus MUST be on the newly selected row. After closing without a choice, keyboard focus
  MUST return to where it was before the quick-switcher opened.
- **FR-011**: Opening, filtering, choosing or closing the quick-switcher MUST NOT start, stop, close
  or remove a session or change which session is a location's main session.
- **FR-012**: The switcher chord MUST do nothing while Settings, a modal dialog, a context menu, the
  project switcher panel or the sidebar's filter panel is open. While the quick-switcher is open,
  the #803 navigation chords MUST do nothing.
- **FR-013**: The list MUST follow changes to projects, locations and sessions while the
  quick-switcher is open, without closing it or clearing the query; the highlight MUST stay on the
  same entry while it is listed. Choosing an entry that no longer exists MUST change nothing.
- **FR-014**: A query that matches no entry MUST show "No matches"; with no entries at all, the
  quick-switcher MUST say there is nothing to switch to. Enter then does nothing.
- **FR-015**: Each window MUST have its own quick-switcher and change only its own selection.
- **FR-016**: The switcher chord MUST be recognised by its physical key (the P position on a US
  layout) with the modifiers held, whatever character the active layout produces for it.
- **FR-017**: The user guide MUST list the switcher chord, with both platform forms, in every place
  it lists the app's keyboard shortcuts (today `docs/user-guide/terminal-panes.md#keyboard-shortcuts`
  and the shortcut list in `docs/user-guide/worktrees-and-sessions.md#interacting-with-the-terminal`),
  and describe what the quick-switcher lists and how to
  filter and choose (Principle VII).

### Key Entities

- **Entry**: kind (project, location, session), display name, context (a session's location name;
  a location's tags; a project's counts and availability), whether it is current, and whether it is
  selectable.
- **Query**: the filter text; with the entry list it decides which entries remain and their order.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: A user can make any listed location or session current, from a focused terminal and
  without the mouse, with the switcher chord, at most 4 typed characters of a distinguishing part of
  its name, at most 2 Up/Down presses and Enter, in a project of 50 locations.
- **SC-002**: The switcher chord, pressed with a terminal focused, sends 0 bytes to the terminal.
- **SC-003**: With 500 entries, the list shows the filtered result of each typed character within
  100 ms in the app's test harness.
- **SC-004**: After every choice, the highlighted sidebar row is the chosen location or session (or
  the restored current session of the chosen project), in 100% of choices.
- **SC-005**: Opening, filtering, choosing or closing the quick-switcher never changes how many
  sessions are running.
- **SC-006**: The switcher chord appears in every user-guide shortcut list, with Linux/Windows and
  macOS forms.

## Assumptions

- **Ctrl/Cmd+Shift+P** is the chord: it follows the app's primary-modifier-plus-Shift convention, is
  the well-known command-palette chord of common editors, and is free in the app and on all three
  platforms (no system or input-method shortcut binds it; GNOME/IBus binds only Ctrl+Shift+U and
  Ctrl+. / Ctrl+;).
- The chord is fixed; user-configurable key bindings stay out of scope, as in #803.
- Matching is in-order character matching, ignoring case only; accents are not folded (a query `e`
  does not match `é`).
- An entry is matched on its displayed name only: a worktree's branch or folder name, an issue tag
  or a session's CLI are not matched (the displayed name is what the user reads in the sidebar).
- Closed and stored-but-not-open sessions are not listed: resuming them stays in the sidebar's
  existing flows, and a choice never starts a session.
- Choosing reuses the sidebar's and the project switcher's existing rules for which session a
  location opens, opening its row and scrolling into view.

## Out of Scope

- Commands or actions in the quick-switcher (creating a worktree, opening Settings): it only
  switches.
- Searching terminal output, files or conversation text.
- Listing closed or stored sessions to resume them.
- Recently-used ordering of entries.
- User-configurable key bindings.
- An in-app list of shortcuts.
