# Feature Specification: Bottom Status Bar

**Feature Branch**: `claude/project-thread-6jo3w5`

**Created**: 2026-10-24

**Status**: Draft

**Input**: User description: "Issue #1124 — Layout: bottom status bar for connection, sandbox, CLI
and plan usage. Blocked by: none. Replace the connection banner, sandbox banner and sandbox
bring-up row stacked under the app bar with one 28 px status bar at the bottom of the window:
daemon connection (dot + text), sandbox state, session CLI, key hints. Failures that need action
still raise a banner/notification; the bar only carries steady state. Acceptance: no stacked chrome
rows under the title bar in the steady state; the bar renders in every body branch (session,
Overview, Changes, Settings), as the banners do today; layout snapshot, overflow gate and docs
updated. Mockup: https://claude.ai/artifact/QXMeLzfh2oEkC9CAwn1M2e (Layout Redesign, board
"Proposal: session view")"

## Context

Today three rows can stack between the app bar and the window body: the connection banner (only
when not connected), the sandbox banner (only for a failed, out-of-date or bypassed sandbox) and
the sandbox bring-up row (while the sandbox is being checked, downloaded or started). Each pushes
the body down when it appears and disappears again, and the healthy state — connected, sandbox
running — is not shown anywhere. The mockup's "Proposal: session view" board draws one 28 px bar
along the bottom edge of the window instead: a status dot with "Daemon connected · local", then
"Sandbox ready", then "CLI: Claude", and on the right the Overview and Jump key hints. The canvas
note for that board says the bar "collects connection, sandbox, CLI and plan usage" and that these
"now share one slot with the banners".

Other redesign issues reshape the title bar (#1127), the sidebar (#1125) and the worktree header
and session tabs (#1126) in parallel. This feature covers only the status bar and the rows it
replaces.

## Clarifications

### Session 2026-10-24

- Q: Does a lost connection to the session service (no action to take, the app reconnects on its
  own, content may be stale) still raise the banner, or move to the bar only? → A: Bar only. The
  bar's connection item shows the error dot with disconnected/reconnecting text and no banner is
  raised; any connection condition that offers an action button keeps its banner. _(answered by
  the issue text: issue #1124 body, "Failures that need action still raise a banner/notification;
  the bar only carries steady state"; today's `Disconnected` banner has no action:
  crates/micold-client/src/ui/mod.rs#connection_banner)_
- Q: Is plan usage shown in the status bar or left in the title (app) bar? → A: In the status bar.
  The indicator moves from the app bar's trailing actions (`ui/toolbar.rs`) into the status bar,
  keeping its label, warning look, hover details and show/hide rules; the title-bar issue #1127
  builds on the app bar without it. _(answered by the issue text: issue #1124 title, "bottom status
  bar for connection, sandbox, CLI and plan usage", and the board's canvas note)_

## User Scenarios & Testing *(mandatory)*

### User Story 1 - See the window's health at a glance without chrome rows (Priority: P1)

A user working in a session wants to know that the window is connected to its session service and
that the sandbox is running, without a row of chrome appearing and disappearing above the content.
A thin bar along the bottom of the window always says so.

**Why this priority**: It is the core of the issue: the steady state gets a fixed, quiet home and
the body stops shifting when a condition comes and goes.

**Independent Test**: Open a project with a running sandbox and an active session; check that
nothing sits between the app bar and the body, that a 28 px bar runs along the bottom edge, and
that it reads connected with a positive dot and "Sandbox ready".

**Acceptance Scenarios**:

1. **Given** the window is connected and the sandbox is running, **When** the user looks at the
   window, **Then** no row is drawn between the app bar and the body, and the bottom bar shows a
   connected dot with the connection text naming where the service runs, and the sandbox state as
   ready.
2. **Given** the user is in the session view, the Overview, the Changes view or Settings, **When**
   each is shown, **Then** the bar is drawn at the bottom of the window in each of them, at the
   same height and position.
3. **Given** the Overview is in full-screen mode, **When** it is shown, **Then** the bar is still
   drawn at the bottom of the window.
4. **Given** no project is open (the empty start screen), **When** the window is shown, **Then**
   the bar is drawn with the connection and sandbox items.
5. **Given** a menu, dialog, the quick switcher or a notification is open, **When** it is drawn,
   **Then** it is drawn over the bar, and the bar never takes keyboard focus from the terminal.
6. **Given** the user clicks or hovers an item in the bar, **When** the pointer is over it,
   **Then** nothing is activated; at most the item's tooltip is shown.
7. **Given** the window is at the application's minimum width, **When** the bar is drawn,
   **Then** no text in it is painted past its box: the key hints are dropped first, then the
   plan-usage item, then the remaining texts are shortened with an ellipsis,
   the connection item last.

---

### User Story 2 - Follow the sandbox coming up from the bar (Priority: P1)

While the sandbox is being checked, downloaded or started, the user wants to see that it is
happening and how far it got, without a progress row pushing the body down.

**Why this priority**: The bring-up row is one of the three rows the issue removes; without this
story its information would be lost.

**Independent Test**: Start the application with a sandbox that needs its image downloaded; check
that the bar's sandbox item shows the stage and progress, that no row appears above the body, and
that the item turns to ready when the sandbox is up.

**Acceptance Scenarios**:

1. **Given** the sandbox is being checked, downloaded or started, **When** the user looks at the
   bar, **Then** the sandbox item names the stage (checking the runtime, getting the image,
   starting) with a busy mark, and the download's percentage when the runtime reports one.
2. **Given** a bring-up is a retry after a failed attempt, **When** the user points at the
   sandbox item, **Then** the full detail — the runtime's last line and the reason for trying
   again — is shown in a tooltip.
3. **Given** the bring-up finishes, **When** the sandbox is running, **Then** the item reads
   ready and no row was ever inserted above the body.
4. **Given** the sandbox failed, **When** the user points at the sandbox item, **Then** the
   tooltip gives the failure's reason.

---

### User Story 3 - Failures that need me still interrupt me (Priority: P1)

When something needs the user to act — another window took the project, the service is a
different version or build, the sandbox failed or is out of date, a session is running without the
sandbox — the user still gets a banner with the button that ends the condition. The bar shows the
short state as well, but it never replaces the call to action.

**Why this priority**: Moving these into a 28 px bar of small text at the bottom would hide conditions the
product deliberately keeps loud (a sandbox bypassed for weeks unnoticed is the known failure).

**Independent Test**: Put the window in each of the seven action-needing states; check that the banner with its
action button still appears, and that the bar's item for the same subject shows a short warning
state at the same time.

**Acceptance Scenarios**:

1. **Given** another window holds or took over the project, **When** this window shows it,
   **Then** a banner with "Take over" is shown, and the bar's connection item reads read-only
   with a warning dot.
2. **Given** the running service is a different contract version or an older build, **When** the
   window shows it, **Then** a banner with "Restart service" is shown, and the bar's connection
   item reads connected with a warning dot.
3. **Given** the sandbox failed, is out of date, or the user accepted running without it, **When**
   the window shows it, **Then** the banner with its action (run without it / restart the sandbox
   / try the sandbox again) is shown, and the bar's sandbox item names the condition in a warning
   colour.
4. **Given** the condition ends, **When** the window updates, **Then** the banner goes away and
   the bar's item returns to its steady text.
5. **Given** the connection to the session service is lost, **When** the window shows it,
   **Then** the bar's connection item reads disconnected/reconnecting with an error dot, and no
   banner is raised — there is no action to take; the app reconnects on its own.

---

### User Story 4 - Know which CLI the session runs and the key chords (Priority: P2)

A user switching between sessions of different AI CLIs wants to see which CLI the session in
front of them runs, and to be reminded of the chords that open the Overview and the quick
switcher.

**Why this priority**: Useful orientation, but the window works without it.

**Independent Test**: Open two sessions running different CLIs in split panes; focus each and
check the bar names that pane's CLI; check the two key hints show the platform's chord.

**Acceptance Scenarios**:

1. **Given** a session is active, **When** the user looks at the bar, **Then** it shows the
   session's CLI by its display name (for example "CLI: Claude Code").
2. **Given** two panes show sessions of different CLIs, **When** the user moves focus from one
   pane to the other, **Then** the bar's CLI item changes to the newly focused session's CLI.
3. **Given** the visible body is not the session view (Overview, Changes, Compare, Settings, the
   project surface without a session, or the empty screen) — even while a session is still active
   behind it — **When** the bar is drawn, **Then** the CLI item is not shown, and only the items
   after it move.
4. **Given** the application runs on macOS, **When** the key hints are shown, **Then** they read
   with Cmd (for example "Cmd+Shift+O Overview"); on Windows and Linux they read with Ctrl ("Ctrl+Shift+O
   Overview").
5. **Given** the Overview is open over an active session, **When** the user closes the Overview
   and the session view returns, **Then** the CLI item shows that session's CLI again.

---

### User Story 5 - Plan usage in the status bar (Priority: P3)

A user on a Claude plan with limits wants the plan's usage where the other steady-state facts are.

**Why this priority**: The issue title names it, but plan usage already has a working home in the
app bar; moving it into the status bar is a relocation, not new information.

**Independent Test**: With a plan-usage reading present, check where the indicator is drawn, that
it shows the same label, warning look and hover details as today, and that it is absent when no
reading is held or the setting is off.

**Acceptance Scenarios**:

1. **Given** a plan-usage reading is held and the setting is on, **When** the window is shown,
   **Then** the plan usage is shown in the status bar, and no longer in the app bar, with the
   same label, warning look and hover details it has today.
2. **Given** no reading is held, or the setting is off, **When** the window is shown, **Then** no
   plan-usage item is drawn anywhere.

---

### Edge Cases

- **Nothing to report**: with no project open, the bar still draws its connection and sandbox
  items (the sandbox is window-wide, not per project); items with nothing to say (CLI outside the
  session view, plan usage with no reading) are omitted rather than drawn empty.
- **Sandbox turned off by the user**: the sandbox item reads off in a neutral colour — this is a
  choice, not a failure — and no banner is raised for it.
- **Narrow window**: at the application's minimum window width, no text in the bar is painted
  past its box. Items give way in a fixed order: the key hints are dropped first, then the
  plan-usage item, then long texts are shortened with an ellipsis, the
  connection item last.
- **Long texts**: a remote service with a long name, or a long download detail, is shortened with
  an ellipsis in the bar and shown in full in the item's tooltip.
- **Several sessions (Principle II)**: the bar reflects the focused pane's session; background
  sessions, other worktrees' sessions and sessions in other projects do not change it. Two windows
  each show their own connection state (one may be read-only while the other holds the project).
- **State changes while a modal or menu is open**: the bar updates under the overlay and is drawn
  below every floating surface, never above a dialog or menu.
- **Install-location block screen**: the bar is not drawn there, as no other chrome is today.
- **Cross-platform (Principle VI)**: chords render in each platform's notation; the bar sits flush
  with the bottom edge on all three platforms, including inside macOS's rounded window corners; it
  uses the shipped fonts so its height does not vary by platform.
- **Light and dark themes**: dot and warning colours keep their meaning and meet text contrast in
  both schemes.
- **Bring-up failure mid-download**: the sandbox item stops showing progress and the failed-sandbox
  banner appears with its action, as today.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: The window MUST draw one status bar, 28 px high, along the full width of its bottom
  edge, below the body and below anything the body lays out.
- **FR-002**: The bar MUST be drawn in every body the window shows: the session view (single or
  split panes), the project surface without a session, the Overview (including full-screen mode),
  the Changes view, Compare, Settings and the empty no-project screen. Only the install-location
  block screen omits it.
- **FR-003**: In the steady state (connected, sandbox running, off or coming up), the window MUST
  draw nothing between the app bar and the body: the connection banner's row, the sandbox banner's
  row and the sandbox bring-up row are gone in those states. A lost connection also draws no row
  there: it is shown in the bar only (FR-009).
- **FR-004**: The bar's first item MUST show the connection to the session service as a coloured
  dot followed by text: connected (positive dot, text naming where the service runs, for example
  "local" or the remote's name), disconnected/reconnecting (error dot), and read-only because
  another window holds the project (warning dot).
- **FR-005**: The bar MUST show the sandbox state as text: ready, off, the bring-up stage with a
  busy mark and download percentage when known, and a short warning-coloured word for failed, out
  of date, or running without the sandbox.
- **FR-006**: The sandbox item's tooltip MUST give the full detail the bring-up row and banners
  give today: the runtime's last progress line, the reason for a retry, and the failure's reason.
- **FR-007**: The bar MUST show the CLI of the focused pane's session by its display name while the
  session view is the visible body, and omit the item whenever another body is visible (Overview,
  Changes, Compare, Settings, project surface, empty screen), even if a session is active behind
  it.
- **FR-008**: The bar MUST show, at its trailing end, the key hints for opening the Overview and
  the quick switcher, in the running platform's chord notation.
- **FR-009**: Each of the seven conditions that offer an action today MUST still raise its banner
  with the same action button, as today, and the bar shows its short state at the same time:
  (1) another window took over the project, (2) another window holds the project, (3) the service
  speaks a different contract version, (4) the service is an older build, (5) the sandbox did not
  start, (6) the sandbox is out of date, (7) a session runs without the sandbox. The lost
  connection offers no action, so it raises no banner: it is shown in the bar's connection item
  only (FR-004).
- **FR-010**: A banner, when one is raised, MUST appear in the same window position as today's
  banners and in every body branch, and MUST disappear when its condition ends.
- **FR-011**: The plan-usage indicator MUST keep its label, warning look, hover details and
  show/hide rules, and MUST move from the app bar into the status bar, after the CLI item; the
  app bar no longer draws it.
- **FR-012**: No text in the bar MUST be painted past its box at any window width the application
  allows; when space runs out, the key hints are dropped first, then the plan-usage item, then texts are shortened with an ellipsis, the connection item last.
- **FR-013**: The bar MUST be drawn beneath every floating surface (menus, dialogs, the quick
  switcher, notifications) and MUST NOT take keyboard focus from the terminal.
- **FR-014**: The bar's items MUST be informational: none of them is a button in this feature.
- **FR-015**: The user guide MUST describe the bar and where each banner now appears; the layout
  snapshot and text-overflow gate MUST cover the bar in its steady, bring-up and warning states.

### Key Entities

- **Status bar item**: one fact about the window — connection, sandbox, CLI, plan usage, key
  hints — with a short text, an optional dot or busy mark, a tone (neutral, positive, warning,
  error) and an optional full-detail tooltip.
- **Action banner**: a condition that needs the user, with a title, a sentence and one action;
  unchanged by this feature except that it no longer shares its slot with steady-state facts.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: In the steady state, the vertical distance from the bottom of the app bar to the top
  of the body is 0 in every body branch, measured by the layout snapshot.
- **SC-002**: The bar is present, 28 px high and flush with the window's bottom edge in 100 % of the
  covered states except the install-location block screen.
- **SC-003**: Over a full sandbox bring-up (check, download, start), the body's top edge does not
  move at all.
- **SC-004**: Each of the action-needing conditions listed in FR-009 still shows its action
  button — 7 of 7 conditions.
- **SC-005**: No text in the bar is clipped or painted outside its box in any of its steady,
  bring-up and warning states, including at the minimum window width — 0 overflowing texts.
- **SC-006**: Whether the window is connected and whether the sandbox is ready can each be read
  from the bar alone, with no menu, tooltip or click — checked in every covered state.

## Assumptions

- The bar's colours follow the existing design tokens (the bar surface like the mockup's
  surface-container tone, positive/warning/error from the existing roles); no new colour roles.
- "Where the service runs" uses the label the daemon list already shows for the location (local or
  the remote's name).
- Key hints are the Overview chord and the quick-switcher chord, the two the mockup draws; no
  other hints are added.
- The CLI item shows the focused session's CLI only; it does not show the CLI a new session would
  use.
- Items are not clickable in this feature; making them open Settings or the daemon list is a later
  change.
- Notifications (snackbars) are unchanged.

## Out of Scope

- The title bar, breadcrumb and jump-to field (#1127), the sidebar regrouping (#1125), the
  worktree header and session tabs (#1126).
- The review bar that will sit above the status bar in the Changes view (review redesign R1).
- New status facts not named in the issue (git branch, CPU, token counts).
- Making status bar items interactive.
