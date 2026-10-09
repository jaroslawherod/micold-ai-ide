# Feature Specification: Split the Terminal Area into Panes

**Feature Branch**: `484-terminal-split-panes`

**Created**: 2026-10-09

**Status**: Draft

**Input**: GitHub issue #484, "Split the terminal area into panes to watch several sessions at once":
only one terminal is visible at a time and switching is tabs-only; the user wants to split the
terminal area horizontally and vertically into panes, each showing any session or regular terminal,
resize by dragging, close and move between panes, keyboard shortcuts, input only to the focused
(clearly marked) pane, and a layout remembered per project across restarts.

## Terms

- **Terminal area**: the region of the main window that today shows exactly one terminal (an AI
  session's CLI or one of its regular terminals), with the tab strip that switches between them.
- **Pane**: one rectangle of the terminal area showing one terminal. A terminal is a session's AI
  CLI or one regular-terminal instance (features 012, 026, 027).
- **Layout**: the arrangement of panes, as a tree of horizontal and vertical splits, with each
  divider's position and which terminal each pane shows.
- **Focused pane**: the one pane that receives keyboard input. There is exactly one while the
  terminal area is displayed.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Watch two terminals side by side (Priority: P1)

A developer runs an AI agent in one session and wants a regular terminal next to it, or wants to
watch two agents work at once. They split the terminal area and choose what each pane shows.

**Why this priority**: It is the whole value of the issue. Without it nothing else matters.

**Independent Test**: Open two sessions, split the terminal area once, show a different session in
each pane, and confirm both render live output at the same time.

**Acceptance Scenarios**:

1. **Given** one terminal shown in the terminal area, **When** the user splits it vertically,
   **Then** two panes appear side by side, the original terminal stays in the first, and the new
   pane shows a terminal not already shown, or the empty-pane state with a picker (FR-004).
2. **Given** two panes, **When** the user splits one of them horizontally, **Then** that pane is
   replaced by two stacked panes and the other pane is untouched.
3. **Given** two panes showing two different sessions, **When** both produce output, **Then** both
   panes update live without the user switching anything.
4. **Given** a pane, **When** the user picks any session or regular terminal of the project for it
   (from the tab strip, the sidebar or a pane's own picker), **Then** that pane shows it.

---

### User Story 2 - Keyboard goes to exactly one pane (Priority: P1)

The user types into one pane and is never surprised by keystrokes landing in another. The focused
pane is visibly marked.

**Why this priority**: Wrong-pane input is destructive (an agent receives a prompt meant for a
shell). It must hold from the first release of the feature.

**Independent Test**: With two panes, type text and press keys; only the focused pane's terminal
receives them. Move focus and repeat.

**Acceptance Scenarios**:

1. **Given** two panes, **When** the user presses a key, **Then** only the focused pane's terminal
   receives it and the other pane's terminal receives nothing.
2. **Given** two panes, **When** the user presses inside the unfocused pane, **Then** that pane
   becomes focused (one press, as feature 023 FR-008b) and the press itself is not delivered as
   input to either terminal.
3. **Given** a focused pane, **Then** it carries a marking that is visible without hovering and
   distinguishes it from unfocused panes in both themes; exactly one pane carries it.
4. **Given** the window loses and regains input focus, **When** it regains it, **Then** the keyboard
   returns to the pane that held it (feature 023 FR-013).
5. **Given** a dialog, menu or text field takes the keyboard from the focused pane, **When** it
   closes, **Then** the keyboard returns to the same pane (feature 023 FR-010).

---

### User Story 3 - Keyboard shortcuts for panes (Priority: P2)

The user splits, closes and moves between panes without the mouse.

**Why this priority**: The issue asks for it, but the mouse path works without it.

**Independent Test**: Using only the keyboard, split twice, move focus to each pane, and close one.

**Acceptance Scenarios**:

1. **Given** a focused pane, **When** the user presses the split-vertical or split-horizontal
   shortcut, **Then** the pane splits and the new pane takes focus.
2. **Given** several panes, **When** the user presses a move-focus shortcut for a direction,
   **Then** focus moves to the neighbouring pane in that direction, or stays when there is none.
3. **Given** several panes, **When** the user presses the close-pane shortcut, **Then** the focused
   pane closes and focus moves to a surviving neighbour.
4. **Given** a pane shortcut, **Then** it is never forwarded to the terminal, and no chord a
   terminal program needs (feature 006's key map) is taken for it.

---

### User Story 4 - Resize, close and rearrange panes (Priority: P2)

The user drags a divider to give one pane more room, closes panes they no longer need, and moves a
terminal from one pane to another.

**Why this priority**: Fixed 50/50 splits are usable but cramped; closing is needed to go back to one
terminal.

**Independent Test**: Split, drag the divider, close one pane, and confirm the survivor fills the
area and its session still runs.

**Acceptance Scenarios**:

1. **Given** two panes, **When** the user drags the divider, **Then** the panes resize live and
   neither shrinks below a minimum usable size.
2. **Given** a pane, **When** the user closes it, **Then** its sibling takes the space, and the
   session or terminal that was shown in it keeps running and stays available in the tab strip and
   sidebar.
3. **Given** the last remaining pane, **When** the user closes it, **Then** it is not closed (the
   terminal area always has at least one pane).
4. **Given** two panes, **When** the user moves the terminal of one pane into the other pane
   (drag the pane's header onto the target), **Then** the target pane shows
   that terminal and the source pane shows the target's previous terminal (a swap).
5. **Given** a divider, **When** the user double-presses it, **Then** the split returns to equal
   sizes.

---

### User Story 5 - Each pane is a real terminal of its own size (Priority: P1)

Each pane resizes its own PTY to its own size, so full-screen programs and wrapped output look right
in every pane.

**Why this priority**: A pane that renders at another pane's size is broken output, not a cosmetic
problem; it is an acceptance criterion of the issue.

**Independent Test**: Show two terminals in panes of different sizes, run `stty size` in each, and
resize a divider; each reports its own pane's rows and columns.

**Acceptance Scenarios**:

1. **Given** two panes of different sizes, **When** `stty size` runs in each terminal, **Then** each
   reports the rows and columns of its own pane.
2. **Given** a divider drag, **When** it ends, **Then** both affected terminals report the new
   sizes; mid-drag resizes are coalesced so the daemon is not flooded.
3. **Given** a terminal shown in a pane, **When** the pane is closed or the terminal replaced,
   **Then** the terminal keeps its last size until it is shown again, then resizes to the pane that
   shows it.

---

### User Story 6 - The layout survives a restart (Priority: P2)

The user quits and reopens the app and finds each project's panes as they left them.

**Why this priority**: Rebuilding a layout after every launch would make the feature a chore.

**Independent Test**: Build a layout in project A and a different one in project B, restart, and
confirm each project shows its own.

**Acceptance Scenarios**:

1. **Given** a project with a layout of several panes, **When** the app restarts, **Then** the
   project shows the same split structure, divider positions, pane contents and focused pane.
2. **Given** two projects with different layouts, **When** the user switches between them, **Then**
   each shows its own layout.
3. **Given** a saved layout that names a session or terminal that no longer exists, **When** the
   project loads, **Then** that pane falls back to an empty-pane state (FR-011) and the rest of the
   layout loads.
4. **Given** a saved layout file that is unreadable or from a newer version, **When** the project
   loads, **Then** the project opens with a single pane and the app does not fail to start.

---

### Edge Cases

- **Empty**: a pane whose terminal is gone shows an empty-pane state with a picker, not a blank or
  a crash. A project with no sessions shows one empty pane.
- **Failure**: a terminal whose process exited shows its exit state in its pane (features 010, 012)
  and does not affect other panes. A failed PTY resize in one pane does not block the other pane.
- **Same terminal in two panes**: a terminal is shown in at most one pane. Choosing a terminal that
  another pane already shows moves focus to that pane instead of duplicating it
  (two views of one PTY would have two sizes).
- **Concurrency (Principle II)**: several sessions of one project, and sessions of different
  worktrees, may be shown at once; output from one never appears in another's pane. Two panes
  may receive output simultaneously; the plan decides how redraws are shared.
- **Minimum size**: a split is refused (with a visible reason) when either resulting pane would fall
  below the minimum size. The window shrinking below the layout's total minimum scales dividers
  down instead of clipping a pane.
- **Session ends or is removed while shown**: the pane stays and shows the empty-pane state; if the
  removed session was the last shown anywhere, the layout is kept, not collapsed;
  empty panes persist until the user closes them.
- **Tab strip with several panes**: pressing a tab shows that terminal in the focused pane (and, by
  the previous edge case, focuses the pane already showing it).
- **Cross-platform (Principle VI)**: the shortcuts avoid chords reserved by macOS, Windows or common
  Linux desktops, use `Cmd` on macOS and `Ctrl+Shift` elsewhere, and are rebindable where other
  shortcuts are. Chords: split vertical `Ctrl/Cmd+Shift+D`, split horizontal `Ctrl/Cmd+Shift+H`,
  close pane `Ctrl/Cmd+Shift+W`, move focus `Ctrl/Cmd+Shift+Arrow` (not `Ctrl+Alt+Arrow`, which
  GNOME/KDE use for workspace switching); none is forwarded to a terminal and none collides with the
  existing `Ctrl/Cmd+Shift+E`/`T`/`C`/`V`.
- **Very many panes**: the layout caps at 6 panes per project; splitting at the cap is refused with
  a visible reason.
- **Idle**: unfocused windows and background panes do not poll or redraw more than today's single
  terminal does (the focus-gated terminal and theme polls recorded in CHANGELOG, `perf: gate terminal/OS-theme polls on window focus`).

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: The user MUST be able to split any pane vertically (side by side) or horizontally
  (stacked), repeatedly, up to 6 panes per project.
- **FR-002**: Each pane MUST show exactly one terminal: any session's AI CLI or any regular-terminal
  instance of the project, chosen by the user.
- **FR-003**: Every pane MUST render its terminal live while visible, with output, scrollback,
  selection and links (features 006, 031, 041) working per pane.
- **FR-004**: A new pane MUST open showing a terminal not already shown in another pane when one
  exists, otherwise the empty-pane state; it MUST NOT duplicate a terminal shown elsewhere.
- **FR-005**: The user MUST be able to resize two adjacent panes by dragging the divider between
  them; panes MUST NOT shrink below a minimum usable size, and a double press on the divider MUST
  restore equal sizes.
- **FR-006**: The user MUST be able to close any pane except the last; the sibling takes the freed
  space.
- **FR-007**: Closing a pane or replacing the terminal it shows MUST NOT stop, restart or detach the
  session or terminal's process; it stays reachable from the tab strip and the sidebar.
- **FR-008**: The user MUST be able to move a terminal from one pane to another; the two panes swap
  their terminals.
- **FR-009**: Keyboard shortcuts MUST exist to split vertically, split horizontally, close the
  focused pane and move focus left, right, up and down. They MUST be handled by the app and never
  forwarded to a terminal, and MUST NOT shadow a chord feature 006's key map forwards.
- **FR-010**: Keyboard input MUST be delivered only to the focused pane's terminal. Exactly one pane
  is focused whenever the terminal area is displayed, and focus-gated key routing (features 006, 023)
  MUST hold per pane: a press in an unfocused pane focuses it without delivering the press as input.
- **FR-011**: The focused pane MUST be clearly marked, visible without hovering, in both themes, and
  distinguishable by more than colour alone (feature 003 accessibility). A pane whose terminal is
  gone MUST show an empty-pane state with a picker.
- **FR-012**: Each pane MUST resize its own PTY to its own size in rows and columns. Resizes during a
  divider drag MUST be coalesced; the final size MUST always be sent.
- **FR-013**: The layout (split tree, divider ratios, each pane's terminal, the focused pane) MUST be
  stored per project and restored on restart, in the same local, versioned store as other
  per-project state (Principle IV); forgetting a project deletes its layout (feature 014).
- **FR-014**: A layout that is missing, unreadable, from a newer version, or names terminals that no
  longer exist MUST degrade to a single pane or empty panes, never block startup or the project.
- **FR-015**: Idle CPU with the maximum number of panes open and no output MUST stay within the
  existing idle budget (a single open terminal's idle CPU today): no per-pane timers or polls beyond the existing focus-gated ones, and
  redraw only on output or interaction.
- **FR-016**: A project's panes MUST be independent of other projects': switching projects switches
  layouts and never shows one project's terminal in another's area.
- **FR-017**: Existing single-terminal behaviour MUST be unchanged for a project with one pane,
  including the tab strip, focus flow (feature 023) and the empty-terminal messages.

### Key Entities

- **Layout**: a project's terminal-area arrangement: a tree whose leaves are panes and whose inner
  nodes are horizontal or vertical splits with a divider ratio; plus the focused pane.
- **Pane**: a leaf with an identity stable across restarts and a reference to the terminal it shows
  (a session and which of its terminals), or nothing.
- **Pane terminal reference**: names a session's AI CLI or one regular-terminal instance; resolves to
  a live terminal or to "gone".

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: A user can go from one terminal to two side-by-side live terminals in at most 2
  actions (one split, one choice), by mouse or keyboard.
- **SC-002**: In 100% of automated trials across all panes of 2- to 6-pane layouts, a key press
  reaches exactly one terminal, the focused pane's.
- **SC-003**: In every pane, the rows and columns the terminal program reports equal the pane's own, within one
  cell, after any split, drag, close or window resize.
- **SC-004**: Idle CPU with 6 open panes and no output is no more than 10% above the single-pane
  figure on the same machine, and a window without input focus shows no added polls.
- **SC-005**: Closing a pane leaves 100% of the sessions it showed running.
- **SC-006**: After a restart, 100% of projects reopen with the same split structure, ratios and
  pane contents, apart from terminals that no longer exist.
- **SC-007**: A corrupt or newer-version layout never prevents the app from starting (0 start
  failures in the fault-injection tests).

## Clarifications

### Session 2026-10-09

- Q: May one terminal be shown in two panes (mirroring)? → A: No, at most one pane per terminal; choosing it focuses the pane that shows it. _(agent-resolved: specs/484-terminal-split-panes/spec.md#Assumptions — terminals are never mirrored, one PTY has one size)_
- Q: Do empty panes persist or auto-close? → A: They persist until the user closes them, so a layout survives a session ending. _(agent-resolved: specs/484-terminal-split-panes/spec.md#Edge Cases — layout kept when the last shown session is removed; FR-011 empty-pane state)_
- Q: Which split/close/focus chords? → A: `Ctrl/Cmd+Shift+D`/`H`/`W` and `Ctrl/Cmd+Shift+Arrow`. _(agent-resolved: crates/micold-client/src/keymap.rs — existing chords are `Ctrl/Cmd+Shift+E`/`T`/`C`/`V`; the new ones follow that family and avoid desktop-reserved `Ctrl+Alt+Arrow`)_

## Assumptions

- The terminal area is the existing region and its tab strip stays; tabs choose what the focused
  pane shows.
- Layout is per project and spans all of that project's worktree sessions; it is not per session.
- Terminals are never mirrored: one PTY has one size (see the first clarification marker).
- Pane splitting applies to the desktop client; the daemon's PTY, session and attach protocol need no
  per-pane notion beyond resizing each attached terminal (the plan confirms whether the wire changes).
- Floating or detached panes, tabs inside a pane, broadcast input to several panes, and synchronised
  scrolling are out of scope.
- Layout is not shared between machines; it lives in the local store like other per-project state.
