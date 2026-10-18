# Feature Specification: Keyboard Navigation of Projects, Worktrees and Sessions

**Feature Branch**: `claude/project-thread-ah7xxz`

**Created**: 2026-10-17

**Status**: Closed 2026-10-18 — shipped in PRs #949, #967, #984, #987, #990

**Input**: User description: "Issue #803: Keyboard shortcuts to navigate worktrees, sessions and
projects. Problem: moving between worktrees, sessions and projects requires the mouse. Proposal:
shortcuts for next/previous project, next/previous worktree within the current project,
next/previous session within the current worktree, or jump to one directly. Acceptance: each level
has next/previous bound to a shortcut; navigation wraps or stops at the ends consistently; default
bindings do not conflict with common system/desktop shortcuts on macOS, Windows, GNOME (Cmd/Alt+Tab,
Cmd+`, Cmd+Space, Win+Arrow, Super+Arrow, Super+Tab, Alt+F4, Ctrl+Alt+Arrow, Cmd+Ctrl+Arrow) nor
existing app bindings; bindings do not steal terminal input (readline/shell keys) in a focused
session; shortcuts listed where the app documents other shortcuts; sidebar focus and selection
follow keyboard navigation. Open questions: per-platform default bindings, user-configurability,
quick-switcher palette follow-up vs in scope."

## Terms

- **Location**: an entry of the sidebar that can hold sessions — the **Default** entry (the project
  root) or a worktree, including a missing or invalid one.
- **Location order**: the order in which the sidebar lists the active project's locations at the
  moment of the key press, after its tag filter: Default first, then the worktrees as shown.
  Locations the filter hides are not in it; the current session's location, which the sidebar keeps
  shown under any filter, is in it.
- **Session order**: the order in which the sidebar lists the open (not closed) sessions of one
  location: the main session first, then its helpers as shown.
- **Project order**: the order of the known projects in the top bar's project switcher, without the
  projects marked **unavailable**.
- **Current**: the session the terminal is showing, its location and the active project, as the
  sidebar already marks them (see the user guide, *Finding the session you are on*).
- **Navigation chord**: one of the six key combinations of FR-001.

## Clarifications

### Session 2026-10-17

- Q: Do the FR-001 defaults collide with a reserved system shortcut, an input-method shortcut or an
  existing app chord? → A: No. GNOME/IBus binds Ctrl+. and Ctrl+; (emoji) and Ctrl+Shift+U
  (Unicode) with exact modifiers, so Ctrl+Shift+. (keysym `greater`) and Ctrl+Shift+, do not match;
  Windows' emoji panel is Win+. / Win+; and its layout switch fires on a bare Ctrl+Shift release,
  not with a third key; macOS reserves none of Cmd+Shift+[ ] , . PageUp PageDown system-wide
  (Cmd+Shift+. shows hidden files only inside Finder and file dialogs, where FR-012 already disables
  the chords; Cmd+Shift+[ ] is the common next/previous-tab convention). None equals an existing
  app chord (E, T, D, H, W, C, V, O, Arrows). _(agent-resolved:
  docs/user-guide/terminal-panes.md#shortcuts; spec.md#FR-006; platform shortcut lists)_
- Q: On macOS laptops PageUp/PageDown need Fn; keep Cmd+Shift+PageUp/PageDown for the project
  chords or pick other keys? → A: Keep Ctrl/Cmd+Shift+PageUp/PageDown on all three platforms (on a
  macOS laptop: Cmd+Shift+Fn+Up/Down). _(CONFIRMED by the user on 2026-10-17)_
- Q: Is "jump directly" to a location, session or project in scope, and in what form? → A: Out of
  scope; a quick-switcher palette is a follow-up issue (#968). _(CONFIRMED by the user on
  2026-10-17)_

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Step between worktrees from the keyboard (Priority: P1)

A developer running several worktrees side by side wants to move to the next or previous worktree
without reaching for the mouse, also while typing in a session's terminal, and land on that
worktree's session ready to type.

**Why this priority**: worktrees are the level a user switches most: a worktree holds one session by
default, so moving between worktrees is moving between pieces of work. This story alone removes the
mouse from the most frequent switch.

**Independent Test**: open a project with Default and three worktrees, each with a session; with the
terminal focused, press the next-worktree chord repeatedly and the previous-worktree chord
repeatedly, and check which session the terminal shows and which row the sidebar highlights after
each press.

**Acceptance Scenarios**:

1. **Given** the current location is the second in location order, **When** the user presses the
   next-location chord, **Then** the third location becomes current, exactly as if its row had been
   clicked: its main session (or the session last looked at there) shows in the terminal, and its
   row is selected in the sidebar.
2. **Given** the current location is the last in location order, **When** the user presses the
   next-location chord, **Then** the first location (Default) becomes current; **and given** the
   first is current, the previous-location chord makes the last one current (wrap at both ends).
3. **Given** a terminal with keyboard focus, **When** the user presses a navigation chord, **Then**
   the terminal receives no bytes for that press, and the newly shown session's terminal has
   keyboard focus so the user can type at once.
4. **Given** keyboard focus in the sidebar (or nowhere), **When** the user presses a navigation
   chord, **Then** the sidebar's keyboard focus moves to the newly selected row.
5. **Given** the target location has no session, **When** it becomes current by the chord, **Then**
   it shows its start-session action as it does when clicked, no session is started, the terminal
   keeps showing the session it showed, and the sidebar's keyboard focus marks the location's row
   (a navigation stop); the next location press steps on from it.
6. **Given** the target row is scrolled out of view, **When** it becomes current by the chord,
   **Then** the sidebar scrolls just far enough to show it.
7. **Given** the sidebar's tag filter hides some worktrees, **When** the user steps through
   locations, **Then** the hidden worktrees are skipped.

---

### User Story 2 - Step between sessions of one worktree (Priority: P2)

A developer who added helper sessions beside a worktree's main session wants to cycle through that
worktree's sessions from the keyboard.

**Why this priority**: helper sessions are opt-in, so fewer users need this, but those who do switch
between them often.

**Independent Test**: on a worktree with a main session and two helpers, press the next-session and
previous-session chords and check which session shows and which sub-row is highlighted.

**Acceptance Scenarios**:

1. **Given** the main session of a worktree with two helpers is current, **When** the user presses
   the next-session chord, **Then** the first helper becomes current; pressing it twice more brings
   back the main session (wrap).
2. **Given** a location with exactly one session, **When** the user presses either session chord,
   **Then** nothing changes and no error is shown.
3. **Given** the worktree's row is collapsed, **When** a helper session becomes current by the
   chord, **Then** the row is opened for the user so the current session's sub-row is on screen, as
   the sidebar already does for the current session.
4. **Given** no session is current (fresh start), **When** the user presses a session chord,
   **Then** nothing changes.

---

### User Story 3 - Step between projects (Priority: P3)

A developer with several known projects wants to move to the next or previous project from the
keyboard, as the top bar's project switcher does by click.

**Why this priority**: switching projects is less frequent than switching worktrees, and the
switcher already makes it one click plus one pick.

**Independent Test**: with three known projects (one of them unavailable), press the next-project
chord repeatedly and check which project becomes active and that the unavailable one is skipped.

**Acceptance Scenarios**:

1. **Given** three available projects with the first active, **When** the user presses the
   next-project chord, **Then** the second becomes active exactly as if it had been picked in the
   switcher: sessions of the project left keep running, and the new project's sidebar and current
   session are restored as the switcher restores them.
2. **Given** the last available project is active, **When** the user presses the next-project
   chord, **Then** the first available project becomes active (wrap), and the previous-project
   chord on the first makes the last active.
3. **Given** a project marked unavailable, **When** the user steps through projects, **Then** it is
   skipped.
4. **Given** only one available project, **When** the user presses either project chord, **Then**
   nothing changes.

---

### User Story 4 - Find the shortcuts (Priority: P3)

A user wants to learn the navigation shortcuts from the documentation.

**Why this priority**: the shortcuts are only useful once known; the app has no in-app shortcut
list, so the user guide is where they are found.

**Independent Test**: read the user guide's keyboard-shortcut lists and find all six chords with
their Linux/Windows and macOS forms.

**Acceptance Scenarios**:

1. **Given** the user guide, **When** the user reads any place that lists the app's keyboard
   shortcuts, **Then** the six navigation chords are listed there with both platform forms, and
   the wrap rule is stated.

---

### Edge Cases

- **Empty lists**: a project with only Default and no worktree: the location chords keep Default
  current. No active project (welcome screen): the location and session chords do nothing; the
  project chords make the first or last available known project active, as the switcher would. No
  known project: nothing happens.
- **No current location** (fresh start, nothing selected): next-location makes the first location
  current, previous-location the last.
- **Missing or invalid worktree** in location order: it is a stop like any other, and becomes
  current exactly as clicking it does (error name, no new session).
- **Lists changing under the user (Principle II)**: a session or worktree created, closed or
  removed in the background, by another window or by an assistant, is taken into account at the
  next press: order is computed at the moment of each press. A press never starts, stops or closes
  a session; background sessions keep running.
- **Key repeat**: holding a chord steps once per repeat; every step is a full selection, and the
  last one wins.
- **Overview open**: a location or session chord closes the Overview and shows the new current
  session, as a sidebar selection does. A project chord switches project and leaves the Overview as
  switching project already does.
- **Settings open**: the navigation chords do nothing while Settings is open (it covers the
  sidebar).
- **Modal dialog or open menu/panel** (create-worktree form, confirmation, context menu, project
  switcher panel): the navigation chords do nothing while one is open; they never move selection
  behind a dialog.
- **Several windows** on the same project: each window navigates its own selection; a press in one
  window changes nothing in another.
- **Terminal programs**: the chords are reserved under the app's existing convention (primary
  modifier plus Shift, taken from the terminal), the same trade-off as Ctrl+Shift+D or C. A terminal
  that drops Shift in its legacy encoding would have sent Ctrl+[ (Escape) or Ctrl+] (readline's
  character search) for the session chords; while the app takes the chord, the program receives
  nothing for that press, and the plain Ctrl+[ and Ctrl+] without Shift still reach it.
- **Keyboard layouts**: a chord is matched by the physical key named in FR-001 (its position on a
  US layout), not by the character Shift produces, so it works the same on layouts where Shift+[
  yields another character or where [ itself needs AltGr (FR-016).
- **Cross-platform (Principle VI)**: each chord uses Cmd on macOS where Linux and Windows use Ctrl;
  behaviour is the same on all three. No chord is one of the reserved system shortcuts listed in
  FR-006 on any of the three platforms.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: The app MUST provide six navigation chords, each with a Linux/Windows form and a macOS
  form (Cmd in place of Ctrl):

  | Action | Linux / Windows | macOS |
  |---|---|---|
  | Previous / next session in the current location | Ctrl+Shift+[ / Ctrl+Shift+] | Cmd+Shift+[ / Cmd+Shift+] |
  | Previous / next location | Ctrl+Shift+, / Ctrl+Shift+. | Cmd+Shift+, / Cmd+Shift+. |
  | Previous / next project | Ctrl+Shift+PageUp / Ctrl+Shift+PageDown | Cmd+Shift+PageUp / Cmd+Shift+PageDown |

  None of these collides with FR-006's lists or a platform input-method shortcut (see
  *Clarifications*). The project chords use PageUp/PageDown on all three platforms; on a macOS
  laptop without those keys they are Cmd+Shift+Fn+Up / Cmd+Shift+Fn+Down.
- **FR-002**: The next/previous location chord MUST make the location after/before the current one
  in location order current, with the same result as clicking that location's row.
- **FR-003**: The next/previous session chord MUST make the session after/before the current one in
  the current location's session order current, with the same result as clicking its sub-row.
- **FR-004**: The next/previous project chord MUST make the project after/before the active one in
  project order active, with the same result as picking it in the project switcher.
- **FR-005**: At every level, stepping past the last entry MUST wrap to the first and stepping
  before the first MUST wrap to the last. A list with one entry MUST stay on it; a press on an empty
  list MUST do nothing.
- **FR-006**: A default chord MUST NOT equal a reserved system shortcut on macOS, Windows or GNOME,
  including Cmd/Alt+Tab, Cmd+`, Cmd+Space, Win/Super+Arrow, Super+Tab, Alt+F4, Ctrl+Alt+Arrow and
  Cmd+Ctrl+Arrow, nor an existing app chord (Ctrl/Cmd+Shift+E, T, D, H, W, C, V, O and Arrows;
  Cmd+C and Cmd+V on macOS).
- **FR-007**: The navigation chords MUST work both while a terminal has keyboard focus and while it
  does not. A chord the app acts on MUST NOT reach the terminal (0 bytes for that press).
- **FR-008**: Every navigation chord MUST carry the platform primary modifier plus Shift; none is a
  plain, Alt-, or Ctrl-only key, so keys typed without Shift (readline's Ctrl and Alt keys, plain
  Ctrl+[ and Ctrl+]) keep reaching the terminal.
- **FR-009**: After a navigation chord changes the current session, the sidebar MUST select the new
  current row, open its location's row when the session is a sub-row, and scroll it into view only
  when it is out of view, as it does for any current session.
- **FR-010**: When a terminal had keyboard focus before the press and the press shows a terminal,
  that terminal MUST have keyboard focus after it. Otherwise (focus was elsewhere, or the new current
  location has no session and shows its start action) the sidebar's keyboard focus MUST move to the
  newly selected row. After a press that left the keyboard on a sidebar row, the next press keeps it
  in the sidebar: the terminal takes the keyboard back only by a click or the focus chords.
- **FR-011**: A navigation chord MUST NOT start, stop, close or remove a session or change which
  session is a location's main session; a location without a session shows its start action.
- **FR-012**: The navigation chords MUST do nothing while Settings, a modal dialog, a context menu,
  the project switcher panel or the sidebar's filter panel is open. A location or session chord MUST
  close an open Overview as a sidebar selection does.
- **FR-013**: The user guide MUST list the six chords, with both platform forms and the wrap rule,
  in every place it lists the app's keyboard shortcuts (Principle VII).
- **FR-014**: Jumping directly to a location, session or project (numbered chords or a picker) is
  out of scope; a quick-switcher palette is a follow-up issue.
- **FR-015**: Each window MUST navigate its own selection only.
- **FR-016**: A chord MUST be recognised by its physical key (the key position FR-001 names on a US
  layout) with the modifiers held, whatever character the active keyboard layout produces for it.

### Key Entities

- **Navigation chord**: an action (level × direction) and its per-platform key combination.
- **Location order / session order / project order**: the lists defined in *Terms*, read at the
  moment of a press.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: A user can move from any worktree to any other worktree of the project without the
  mouse, in at most ⌊N/2⌋ presses for a project of N locations (wrap in both directions).
- **SC-002**: For each of the six chords, pressed with a terminal focused, the terminal receives 0
  bytes.
- **SC-003**: 0 of the six default chords appears in the reserved-shortcut lists of FR-006 on any of
  the three platforms, or among the app's existing chords.
- **SC-004**: After every navigation press, the highlighted sidebar row is the session the terminal
  shows, in 100% of presses. After every press that shows a session, that row is on screen; after a
  press that lands on a navigation stop, the stop's row is on screen (the highlighted row may then
  be scrolled out of a long list).
- **SC-005**: Pressing a navigation chord never changes how many sessions are running.
- **SC-006**: All six chords appear in every user-guide shortcut list, with Linux/Windows and macOS
  forms.

## Assumptions

- The chords are fixed in this feature; user-configurable key bindings are out of scope (the app
  has no key-binding settings today) and a candidate follow-up.
- Jumping directly (FR-014) is out of scope; a quick-switcher (fuzzy-find palette) is a follow-up.
- Wrapping, not stopping, at the ends: it matches tab cycling in common desktop apps and lets a user
  reach the last entry in one press.
- The app's existing reserved-chord convention (platform primary modifier plus Shift, taken from
  the terminal) is reused, so the terminal trade-off is the same one users already have with the
  pane and Overview chords.
- The sidebar keeps its current rules for which session of a location opens when the location is
  selected and for opening and scrolling to the current row; this feature reuses them.

## Out of Scope

- User-configurable key bindings.
- Jumping directly to a location, session or project: numbered chords or a fuzzy-find
  quick-switcher palette (follow-up issue).
- An in-app list of shortcuts (the app has none; the user guide is the list).
- Moving between windows, panes or terminal tabs (panes already have their own chords).
