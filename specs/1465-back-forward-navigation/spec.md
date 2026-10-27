# Feature Specification: Back/Forward Navigation (Mouse Side Buttons and Alt+←/→)

**Feature Branch**: `claude/project-thread-e4cve5`

**Created**: 2026-10-27

**Status**: Draft

**Input**: GitHub issue #1465 "Back/forward navigation with mouse side buttons and Alt+←/→". Today the only "go back" is local to the Review tab (the **Back** above a browsed file), and the mouse's back/forward side buttons do nothing.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Mouse back/forward between places (Priority: P1)

A user moves around the app — selects session A, then session B, opens another worktree, opens the Overview — and presses the mouse's **back** side button to return to where they were before, step by step. The **forward** side button goes the other way, as in a browser.

**Why this priority**: It is the core of the feature: a navigation history and the most direct way to walk it, from anywhere in the window.

**Independent Test**: Select session A, then session B; press mouse back: A is shown; press mouse forward: B is shown.

**Acceptance Scenarios**:

1. **Given** the user selected session A and then session B, **When** they press mouse back, **Then** session A is shown, at its location and project.
2. **Given** they then press mouse forward, **When** the press lands, **Then** session B is shown again.
3. **Given** the user went A → B → C and pressed back twice (now on A), **When** they select session D, **Then** forward does nothing (B and C are dropped) and back returns to A.
4. **Given** the pointer is over a terminal pane, **When** the user presses a side button, **Then** the app steps through its history and the terminal program receives no input for that press.
5. **Given** a text field has the keyboard, **When** the user presses a side button, **Then** the app steps through its history.
6. **Given** the user is on the first entry (or last, for forward), **When** they press back (forward), **Then** nothing changes and no error is shown.

---

### User Story 2 - Alt+← / Alt+→ from the keyboard (Priority: P1)

A keyboard user presses **Alt+←** to go back and **Alt+→** to go forward through the same history, without the shortcut stealing the keys from a text field or a terminal that uses them for editing.

**Why this priority**: It is the only way to walk the history without a mouse that has side buttons, and the issue's second acceptance criterion.

**Independent Test**: Select session A, then B, with no text field or terminal holding the keyboard; press Alt+←: A is shown; Alt+→: B is shown.

**Acceptance Scenarios**:

1. **Given** sessions A then B were selected and no text field or terminal has the keyboard, **When** the user presses Alt+←, **Then** A is shown; **When** they press Alt+→, **Then** B is shown.
2. **Given** a text field has the keyboard (a composer, a filter, a comment box, a dialog field), **When** the user presses Alt+← or Alt+→, **Then** the field handles the keys as it does today and the history does not move.
3. **Given** a terminal pane has the keyboard, **When** the user presses Alt+← or Alt+→, **Then** the terminal receives the keys as it does today and the history does not move.
4. **Given** the user holds Alt+←, **When** the key repeats, **Then** each repeat steps back one more entry until there is none.

---

### User Story 3 - A view's own back comes first (Priority: P2)

In the Review tab, a user who followed a link from the diff into a browsed file presses back (mouse or Alt+←) and returns to what they were reading in that tab, exactly as the tab's own **Back** button does. Only when the tab has nothing left to go back to does back leave the tab for the previous place in the history.

**Why this priority**: Without it, back would throw the user out of the review they are reading; but the global history works without it.

**Independent Test**: In a Review tab, open a browsed file from the diff; press mouse back: the diff is shown again in the same tab; press again: the previous place in the history is shown.

**Acceptance Scenarios**:

1. **Given** the Review tab shows a browsed file and its **Back** is available, **When** the user presses mouse back or Alt+←, **Then** the tab returns to what was being read before (same file, scroll position and expanded lines) and the global history does not move.
2. **Given** the Review tab's **Back** is not available, **When** the user presses back, **Then** the previous history entry is shown.
3. **Given** the Review tab is shown, **When** the user presses forward, **Then** the global history moves forward (the tab's own back has no forward counterpart).

---

### User Story 4 - Gone places are skipped (Priority: P2)

A user stops or deletes a session, or removes a worktree, then presses back. The entries for places that no longer exist are passed over silently, and back lands on the nearest earlier place that still exists.

**Why this priority**: History that lands on nothing, or shows an error, would make the feature unreliable in a multi-session app where sessions come and go all the time (Principle II).

**Independent Test**: Select A, then B, then C; delete B; from C press back: A is shown, with no error.

**Acceptance Scenarios**:

1. **Given** history A → B → C with C shown and session B since deleted, **When** the user presses back, **Then** A is shown and no error or notification appears.
2. **Given** history entries in a worktree that was removed, **When** back or forward reaches them, **Then** they are skipped.
3. **Given** every entry behind the current one is gone, **When** the user presses back, **Then** nothing changes.
4. **Given** skipping a gone entry would leave the same place twice in a row, **When** back or forward moves, **Then** it never lands on the place already shown (a press always shows somewhere different, or does nothing).

---

### User Story 5 - Discoverable shortcuts (Priority: P3)

A user looking up the app's shortcuts finds back and forward listed with their mouse buttons and keys, in their platform's form.

**Why this priority**: Discoverability only; the feature works without it.

**Independent Test**: Read the app-chord table in the user guide's terminal panes page and open the Review tab's shortcut sheet; both list Back and Forward.

**Acceptance Scenarios**:

1. **Given** the app-chord table under *Keyboard shortcuts* in the user guide's terminal panes page, **When** the user reads it, **Then** it lists Back (mouse back button, Alt+←) and Forward (mouse forward button, Alt+→) with the platform forms and the rule that a focused text field or terminal keeps the keys.
2. **Given** the Review tab's shortcut sheet, **When** it is open, with single-key shortcuts on or off, **Then** it lists Back and Forward in the platform's form.

---

### Edge Cases

- **Empty history**: at start-up the history holds only the place first shown; back and forward do nothing.
- **Re-selecting the shown place**: selecting the session (or view) already shown adds no entry.
- **Cap**: after more than 50 recorded places, the oldest are dropped; back stops at the oldest kept entry.
- **Moving through history**: back and forward never add entries themselves; selecting somewhere new after going back drops every forward entry.
- **Gone targets**: an entry whose session was stopped, archived or deleted, whose worktree was removed, or whose project was closed, forgotten or is unavailable is skipped by both back and forward, without an error. Gone entries are dropped from the history when skipped.
- **Concurrency (Principle II)**: sessions created or ending in the background (another session finishing, an agent opening a worktree) do not add entries unless the shown place changes; when the app itself moves the shown place (e.g. the shown session ended and another is shown), that move is recorded like any other so the history always ends at what is shown.
- **Several projects**: entries carry their project; going back to an entry in another project switches to that project, as the project switcher does.
- **Surfaces open**: while a dialog, menu, the quick switcher, the Settings panel or the shortcut sheet is open, back and forward (mouse and keys) do nothing, as the navigation chords do today.
- **Review tab entries**: going back to a Review tab entry whose tab was closed shows that location's Review tab again (it is reopened, not skipped) as long as the location exists.
- **Mouse without side buttons / trackpads**: the keys are the path; trackpad swipe gestures are out of scope.
- **Cross-platform (Principle VI)**: each platform reports the mouse's back/forward side buttons in its own way; on Linux (X11 and Wayland), Windows and macOS they mean back and forward. On macOS Alt is the Option key, which macOS text fields use for word movement — those fields keep it under FR-009. [NEEDS CLARIFICATION: macOS chord — keep Option+←/→ on macOS as the issue says, or use macOS's browser convention Cmd+[ / Cmd+] there?]
- **Terminal mouse reporting**: a terminal program that asked for mouse events never receives the side buttons; the app always takes them.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: The app MUST keep one navigation history for its window, holding the places the user has been and a current position in it. The history is not kept across restarts.
- **FR-002**: A place MUST be one of: a session (structured or terminal) of a location of a project; the Review tab of a location; the worktree Overview; the Compare view of a run group. Moving to another place by any means (sidebar click, tab strip, pane focus moving to another session, navigation chords, quick switcher, notification, opening or creating a session) MUST record a new entry after the current position.
- **FR-003**: Recording a place equal to the current entry MUST add nothing.
- **FR-004**: Recording a new entry when the position is not at the end MUST drop every entry after the current position.
- **FR-005**: The history MUST hold at most 50 entries; recording past the cap MUST drop the oldest entry.
- **FR-006**: **Back** MUST move the position to the nearest earlier entry whose target still exists and is not the place shown, and show it, restoring its project, location and session or view. **Forward** MUST do the same towards later entries. With no such entry, the press MUST change nothing and show no error.
- **FR-007**: Moving through the history MUST NOT record entries. Entries skipped because their target is gone MUST be removed from the history.
- **FR-008**: Pressing the mouse's back side button MUST perform Back, and its forward side button Forward, wherever the pointer is in the window, including over terminal panes, text fields and the Review tab. The side buttons MUST NOT be passed to a terminal program.
- **FR-009**: Alt+← MUST perform Back and Alt+→ Forward, except while a text field or a terminal pane has the keyboard: then the keys MUST reach that field or terminal unchanged, as today.
- **FR-010**: While a dialog, menu, the quick switcher, the Settings panel or a shortcut sheet is open, Back and Forward (buttons and keys) MUST do nothing.
- **FR-011**: When the shown place is a Review tab whose own **Back** is available, Back MUST perform that tab's **Back** and leave the global history unchanged. Forward MUST always use the global history.
- **FR-012**: Back and Forward MUST act once per key press and once per key repeat, and once per side-button press.
- **FR-013**: The app-chord table under *Keyboard shortcuts* in the user guide's terminal panes page (`docs/user-guide/terminal-panes.md#keyboard-shortcuts`) MUST list Back and Forward with their mouse buttons and keys in the Linux/Windows and macOS forms, and say that a focused text field or terminal keeps the keys.
- **FR-014**: The Review tab's shortcut sheet MUST list Back and Forward in the platform's form, both with single-key shortcuts on and with them off (they are not single keys). No other in-app shortcut sheet is added.
- **FR-015**: The history rules (record, back, forward, truncation, cap, skipping gone entries, the Review tab's own back first) MUST be covered by reducer tests that need no window; the side buttons and the keys MUST be covered by event-routing tests that show which press becomes Back or Forward, and when a press is left to a focused field or terminal.

### Key Entities

- **Place**: where the main area is: its project, its location (Default or a worktree), and what is shown there — a session, the Review tab, the Overview or the Compare view of a run group. Two places are equal when all of these are.
- **Navigation history**: an ordered list of at most 50 places and the position of the one shown. Transient, one per window.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: After selecting A then B, one back press shows A and one forward press shows B, by mouse button and by the platform's Back/Forward keys, on Linux, Windows and macOS.
- **SC-002**: In 100% of the reducer test cases where entries' targets are gone, back and forward land on an existing place or change nothing, and none shows an error.
- **SC-003**: With a text field or terminal holding the keyboard, Alt+←/→ produce exactly the same text or terminal input as before this feature (no regression in the existing input tests).
- **SC-004**: The history never holds more than 50 entries, whatever the number of moves.
- **SC-005**: Back from a browsed file in the Review tab returns to the tab's previous reading in one press, and leaves the tab only on the next press when nothing is left to go back to.

## Assumptions

- The app has one main window, so "per window" means one history for the app run.
- The issue's "Changes: diff → file list" is the Review tab's own **Back** (browsed file → what was being read): in the Review tab the file list and the diff are side by side, so there is no separate diff → list step today.
- Every change of the shown place is recorded, whether the user or the app made it (FR-002), so the history always ends at what is shown; back over a place the app moved away from because it ended skips it as gone.
- The Settings panel is not a place: it is a panel the user closes, and back/forward do nothing while it is open (FR-010), as the navigation chords do today.
- A focused terminal always keeps Alt+←/→ (shells use them for word movement); there is no per-program exception.
- Pane layout, scroll positions and selections inside a place are not part of the history; showing a place restores it as a sidebar click would.

## Out of Scope

- Trackpad swipe gestures and other back/forward input than the side buttons and Alt+←/→.
- History persisted across restarts, or a visible history list or back/forward toolbar buttons.
- In-view back for views other than the Review tab.
- Rebinding the Back/Forward keys.
- An app-wide shortcut sheet; listing Back/Forward in the user guide's other shortcut tables (Review keys, Overview keys).
