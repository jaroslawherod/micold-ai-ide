# Feature Specification: Worktree State Icon

**Feature Branch**: `claude/project-thread-h057j5`

**Created**: 2026-10-17

**Status**: Draft

**Input**: User description: "Implement GitHub issue #905: Worktree should show a state icon:
idle/waiting, blocked or running. Each worktree entry should show an icon for its current state:
Idle / waiting (no work in progress, or waiting for user input); Blocked (cannot proceed, e.g.
needs approval, failed, waiting on a lock); Running (the session in the worktree is actively
working). Expected: the icon is visible on the worktree in the list without opening it; state is
derived from the worktree's session and updates live; icons are distinguishable by shape as well
as colour."

## Terms

- **Worktree row**: a worktree's entry in the sidebar list, including a parallel-run row (feature
  483), which is a worktree row inside its group.
- **Location**: a worktree, or the project root shown as **Default**. Each hosts zero or more
  un-archived sessions: one main session and any helpers (feature 776).
- **Worktree state**: one of three values the row shows (or Unknown, when no session gives a
  signal: rule 7), derived from the location's sessions:
  - **Running**: a session is actively working: it is starting, restarting after an unexpected
    exit, or its activity reads *working*.
  - **Blocked**: a session cannot proceed until the user acts: it stopped mid-turn to ask for a
    permission or an answer to a question (one case, as feature 613's *Needs permission* kind), its
    start failed or its restarts were exhausted (*failed*), or it was interrupted by a service
    restart and waits to be resumed (*resumable*).
  - **Idle / waiting**: nothing is in progress: no session, a session whose lifecycle is *idle*
    (never started, or stopped by the user), a *running* session whose activity reads *ended*, or a
    session that finished its turn and waits for the next prompt (the Per-session state rules decide).
- **Per-session state**: one session's state, taken from its lifecycle first and its activity
  second, so every combination has exactly one answer. The first rule that matches wins:
  1. lifecycle *failed* or *resumable* → Blocked (whatever the activity reads);
  2. lifecycle *starting* or *restarting* → Running;
  3. lifecycle *idle* → Idle / waiting (whatever the activity reads);
  4. lifecycle *running*, activity *working* → Running;
  5. lifecycle *running*, activity *awaiting input* after a mid-turn stop to ask for a permission →
     or an answer → Blocked; after the end of a turn → Idle / waiting;
  6. lifecycle *running*, activity *ended* → Idle / waiting;
  7. lifecycle *running*, activity *unknown* → **Unknown**: no state is claimed (see the Edge
     Cases section's *Activity unknown* entry); never Blocked.
- **State icon**: the glyph on the worktree row that shows the worktree state.

## Clarifications

### Session 2026-10-17

- Q: Is a session that stopped mid-turn to ask the user a question (not a permission) Blocked, or
  Idle / waiting? → A: Blocked, the same case as a permission. The app does not tell the two apart:
  feature 613 defines *Needs permission* as a mid-turn stop "to ask for a permission or for an
  answer", and the core's `NotificationKind::NeedsPermission` carries that one meaning; the tooltip
  reads *Blocked: needs permission* for both _(agent-resolved:
  specs/613-notification-kinds/spec.md#Terms; crates/micold-core/src/attention.rs
  `NotificationKind::NeedsPermission`)_.
- Q: Does a worktree with several sessions take its state from all of them or from its main session
  alone? → A: From all of its un-archived sessions, main and helpers, by precedence Blocked over
  Running over Unknown over Idle / waiting. The location row already summarises every session it
  holds (feature 575's attention indicator counts every un-archived session of the location), and a
  helper stuck on a permission is exactly what Story 2 must surface _(agent-resolved:
  specs/575-workspace-attention-list/spec.md#Terms "Location row", "Attention indicator";
  crates/micold-client/tests/sidebar_attention.rs
  `a_worktree_row_counts_its_unread_sessions_and_a_read_only_one_shows_none`)_.
- Q: Does a session with unknown activity show Idle / waiting, or no icon? → A: No icon: the
  fixed-width slot is reserved and left empty, and the tooltip line reads *State unknown*. The app
  never shows a state it has no signal for: an unknown activity draws nothing in a reserved slot
  (feature 010 FR-016c), and a provider without a signal MUST NOT read busy, idle or awaiting input
  (feature 488 FR-009) _(agent-resolved: specs/010-daemon-session-persistence/spec.md#FR-016c;
  specs/488-codex-opencode-providers/spec.md#FR-009)_.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - See at a glance which worktrees are working (Priority: P1)

A developer runs agent sessions in several worktrees at once. Looking down the sidebar, without
opening any worktree, they see which worktrees have a session actively working and which are idle,
so they know where work is still happening and where they can give the next prompt.

**Why this priority**: It is the core of the issue: the state is visible on the row without
opening it. Running versus idle is the most frequent question a user has about a busy list.

**Independent Test**: Start a session in one worktree and give it a prompt that takes a while;
leave a second worktree's session idle. Both rows show a state icon without being selected; the
first shows Running, the second Idle / waiting.

**Acceptance Scenarios**:

1. **Given** a worktree whose session is working on a prompt, **When** the user looks at the
   sidebar, **Then** that worktree's row shows the Running icon, whether or not the worktree is
   selected or expanded.
2. **Given** a worktree whose session finished its turn, **When** the user looks at the sidebar,
   **Then** its row shows the Idle / waiting icon.
3. **Given** a worktree with no session, **When** the user looks at the sidebar, **Then** its row
   shows the Idle / waiting icon.
4. **Given** a worktree whose session is working, **When** the session finishes its turn, **Then**
   the row's icon changes from Running to Idle / waiting without any action from the user.

---

### User Story 2 - Spot worktrees that are stuck (Priority: P1)

A developer leaves agents running and switches to other work. When they come back, the sidebar
shows which worktrees cannot proceed without them: a session waiting for a permission, a session
whose start failed, or one interrupted by a service restart.

**Why this priority**: A blocked worktree wastes time until the user notices it; making it stand
out is the main payoff of the icon, and it is as important as Running versus Idle.

**Independent Test**: Make one worktree's session ask for a tool permission, and make another
worktree's session fail to start. Both rows show the Blocked icon; a third, idle worktree does not.

**Acceptance Scenarios**:

1. **Given** a worktree whose session stopped mid-turn to ask for a permission, **When** the user
   looks at the sidebar, **Then** its row shows the Blocked icon.
2. **Given** a worktree whose session failed (start failed or restarts exhausted), **When** the
   user looks at the sidebar, **Then** its row shows the Blocked icon.
3. **Given** a worktree whose session is resumable after a service restart, **When** the user
   looks at the sidebar, **Then** its row shows the Blocked icon.
4. **Given** a Blocked worktree, **When** the user grants the permission and the session carries
   on working, **Then** the icon changes to Running without any further action.

---

### User Story 3 - Tell the states apart without relying on colour (Priority: P2)

A user who cannot distinguish the theme's colours, or who uses a theme where they are close, still
tells the three states apart, and can learn what an icon means without leaving the sidebar.

**Why this priority**: It is an explicit expectation of the issue and an accessibility guarantee,
but it refines the icons of Stories 1 and 2 rather than adding a new capability.

**Independent Test**: Render the three state icons in greyscale in both light and dark themes;
each is still identifiable by its shape. Hover a row: its tooltip names the state in words.

**Acceptance Scenarios**:

1. **Given** the three state icons shown side by side in greyscale, **When** a viewer compares
   them, **Then** each has a different shape.
2. **Given** any worktree row, **When** the user hovers it, **Then** the row's tooltip names the
   row's state in words.
3. **Given** the light theme and the dark theme, **When** the sidebar is shown, **Then** every
   state icon is visible against the row's background in both.

---

### Edge Cases

- **No session**: a worktree with no session (created without one, or its session deleted) is
  Idle / waiting.
- **Several sessions** (main plus helpers, Principle II): the row shows one state for the whole
  location, taken from all of its un-archived sessions, main and helpers alike, by precedence
  Blocked over Running over Unknown over Idle / waiting: a helper waiting for a permission makes the
  row Blocked, and one unknown session beside idle ones leaves the row without an icon. Archived
  sessions do not count. Each session's own sub-row keeps its existing activity dot, unchanged.
- **Activity unknown**: a running session of a CLI whose activity the app cannot read (no hooks
  configured, a provider that reports none, or Pi with its activity setting off) gives no evidence
  of working or waiting, so its state is Unknown: the row's icon slot stays reserved and empty, and
  its tooltip line reads *State unknown*. It is never shown as Blocked (an unknown activity is never
  dressed up as a call for the user), nor as Idle / waiting.
- **Missing or invalid worktree**: keeps its error-tinted name and status tag; its state icon
  follows the same rules as any other row (with no session it is Idle / waiting).
- **Service restart or disconnection**: while the window has no connection to the session service,
  every row keeps the last state it showed; on reconnect the rows take the service's current state.
  A session the service finds interrupted on restart becomes resumable, so Blocked.
- **Fast changes**: a session that flips between working and waiting several times a second never
  makes the row's height or the position of its name change; only the glyph changes.
- **Several windows**: every window that shows the project shows the same state for the same
  worktree.
- **Collapsed or filtered list**: a worktree hidden by a tag filter shows nothing (it is not
  listed); a listed worktree shows its icon whether it is expanded or collapsed.
- **Narrow sidebar**: the icon keeps its size however narrow the sidebar is; the name shortens
  first, as it does today.
- **Cross-platform (Principle VI)**: each state shows the same glyph on Linux, macOS and Windows;
  no platform shows a different, substituted or missing glyph.
- **Default entry**: the project root's Default entry is a location too and shows the same state
  icon, beside its own house icon, without being styled as a worktree.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: Every worktree row in the sidebar MUST show a state icon for its worktree state
  (Running, Blocked or Idle / waiting), without the worktree being selected, opened or expanded; a
  row whose state is Unknown (FR-005) shows its reserved slot empty instead.
- **FR-002**: The Default entry MUST show the state icon of the project root's sessions by the same
  rules, and MUST NOT otherwise change how it is presented.
- **FR-003**: Each session's state MUST be its per-session state, by the ordered rules of the Terms
  section, for every combination of lifecycle and activity; a location with no session MUST be
  Idle / waiting.
- **FR-004**: When a location has more than one un-archived session, the row MUST show a single
  state taken from all of them, main and helpers, by precedence Blocked over Running over Unknown
  over Idle / waiting; archived sessions MUST NOT count.
- **FR-005**: A session whose activity the app cannot read is Unknown: it MUST NOT make a row
  Blocked or Idle / waiting, and a row whose state is Unknown MUST leave the icon's slot reserved
  and empty, with the tooltip line *State unknown*.
- **FR-006**: The state icon MUST update while the user watches: within 1 second of the service
  reporting a session change that alters the derived state, with no user action, no list refresh
  and no change of selection.
- **FR-007**: The three state icons MUST have three different shapes, so each is identifiable
  without colour. Each MAY also be tinted with a colour role of the theme, and MUST be visible in
  the light and the dark theme.
- **FR-008**: The state icons MUST NOT share a shape with any other mark the same row can show (the
  pull request indicator's glyphs, the unread mark, the session activity dot, the Default house
  icon), so no glyph on a row has two meanings.
- **FR-009**: The row's tooltip MUST name the row's state in words (for example *Running*,
  *Blocked: needs permission*, *Blocked: failed*, *Blocked: resumable*, *Idle*, *State unknown*),
  on a line of its own, alongside the lines it already has.
- **FR-010**: The icon MUST occupy a fixed-width slot on every row, whatever the state, so the
  name's position does not move when the state changes, and the row's height MUST NOT change.
- **FR-011**: A change of state MUST NOT trigger a notification, a sound, a dialog or a change of
  selection; it changes the icon and the tooltip line only. Existing notifications and the unread
  mark are unchanged.
- **FR-012**: The state icon MUST be a shared UI component (Principle VIII), shown in each of its
  three states in the component showcase.
- **FR-013**: The user guide MUST describe the three states, their icons and what makes a worktree
  enter each one (Principle VII).

### Key Entities

- **Worktree state**: Running, Blocked (with its reason: needs permission, failed, resumable),
  Idle / waiting, or Unknown (no icon). Derived, never stored: it is recomputed from the
  location's sessions each time they change.
- **Session signals** it is derived from: a session's lifecycle (idle, starting, running,
  restarting, failed, resumable) and its activity (unknown, working, awaiting input, ended), plus
  whether an awaiting-input stop was mid-turn (a permission or a question to the user) or the end
  of the turn.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: With five worktrees in mixed states, a user names the state of every worktree from
  the sidebar alone, without selecting any, in under 5 seconds.
- **SC-002**: In a greyscale rendering of the sidebar in both themes, 100% of the state icons are
  identified correctly by shape.
- **SC-003**: 100% of state changes reported by the session service appear on the affected row
  within 1 second, in every open window showing the project.
- **SC-004**: In an automated pass over every combination of session lifecycle and activity, the
  row's state matches the ordered per-session rules (Terms) and FR-004 in 100% of cases.
- **SC-005**: Across every state change of a row, the name's horizontal position and the row's
  height change by 0 pixels.

## Out of Scope

- Detecting a session that waits on something outside the app, such as a file or build lock in a
  terminal. The issue's "waiting on a lock" example is covered only where the app can observe it:
  the session's own lifecycle and activity. A session blocked on an outside lock reads as Running
  (or Idle / waiting) according to its activity.
- A state icon on a parallel-run **group row**; each run row inside the group has its own.
- Changing the session sub-rows' activity dots, notifications, the unread mark or the attention
  count.
- Filtering or sorting the list by state.
- A state for the shell (Regular-terminal) processes of a worktree: only AI CLI sessions count.

## Assumptions

- The session service already reports each session's lifecycle and activity to every window as
  they change (feature 010), and already tells a mid-turn stop to ask the user apart from the end
  of a turn when it raises notifications (feature 613); the state is derived from those signals.
  Where that distinction does not yet reach the window, carrying it there is part of this feature.
- A session that ended (its process exited) is Idle / waiting, not Blocked: it can be started again
  and nothing is stuck. A failed session (start failed, or restarts exhausted) is Blocked because
  it needs the user to act before it can run.
- A session in a run whose prompt could not be delivered (feature 483) is shown by its session's
  own state; the run row's existing *failed* tag and tooltip are unchanged.
- The three glyphs exist in the icon set the app already ships; the feature adds no new icon source.
