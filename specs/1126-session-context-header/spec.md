# Feature Specification: Worktree context header and session tabs above the terminal

**Feature Branch**: `claude/project-thread-y28cqn`

**Created**: 2026-10-24

**Status**: Closed 2026-10-24 — shipped in PRs #1318, #1328, #1334, #1337 and the close PR. Clarification 1 is still the orchestrator's provisional pick (option A); the user has not answered.

**Input**: User description: "Implement GitHub issue #1126: Layout: worktree context header and
session tabs above the terminal. Blocked by: #1120 (title and branch logic). Above the terminal:
header with title, state chip, branch chip, PR chip and a Changes button (files, +/-). Under it a
session tab strip (main/helper, New session) with the split buttons at its right. Removes the bar
under the terminal. Acceptance: pane chords and focus behaviour unchanged; Changes button opens the
Changes view; no overlap at the minimum window width; docs (terminal-panes, reviewing-changes),
layout snapshot and visual pass updated. Mockup: Layout Redesign, board 'Proposal: session view'."
Labels: enhancement, flow:feature. Builds on feature 1120 (`specs/1120-overview-card-readability`,
title and short branch), feature 484 (terminal panes) and feature 027 (the bar under the terminal).

## Terms

- **Location**, **card state**, **short branch**, **title**: as defined by features 813 and 1120
  (`specs/1120-overview-card-readability/spec.md#Terms`). The five card states are Needs action,
  Needs attention, Working, Done and Idle. A location is the Default entry or one worktree.
- **Session area**: the main column right of the sidebar that today shows the terminal panes and,
  under them, the bar.
- **Bottom bar**: today's bar under the terminal (feature 027): the session's name and process
  status, a restart control when the process is stopped, the tab strip of terminal instances, the
  split buttons, the "+" (new terminal instance) and the pinned AI tab.
- **Context header**: the new band at the top of the session area that names the active location.
- **Tab strip**: the new row directly under the context header that holds, in order, one session tab
  per session of the shown location (main first, then helpers), the shown session's shell tabs and
  "+" right after its own session tab, the **New session** control, and at its right end the split
  buttons.
- **Session tab**: a strip tab for one session of the location. The shown session's session tab
  stands for its AI conversation (today's AI tab); pressing another session's tab shows that
  session as selecting it in the sidebar does. Session tabs follow the sidebar's order.
- **Shell tab**: a strip tab for one shell instance of the shown session (today's numbered tab).
- **Chip**: a small labelled shape in the context header carrying one fact (state, branch, pull
  request).
- **Changes button**: the control in the context header that opens the Changes view
  (`docs/user-guide/reviewing-changes.md`) and shows how many files changed and how many lines were
  added and removed.
- **Minimum window width**: the narrowest window the app allows (640 px today), with the sidebar at
  its default width.

## Clarifications

### Session 2026-10-24

- Q: Which tabs does the strip hold, and what does "New session" open? → A: One session tab per
  session of the location (main first, then helpers); the shown session's tab is its AI
  conversation, followed by that session's shell tabs and "+"; **New session** runs the existing
  **Start another session...** action, confirmation included. _(orchestrator's provisional pick of
  the recommended option, user decision pending)_
- Q: In what order are session tabs drawn, and what does pressing another session's tab do? → A:
  In the sidebar's order for the location (main first, then helpers, agent-started sessions where
  the sidebar lists them), and pressing one shows that session exactly as selecting it in the
  sidebar does. _(agent-resolved: docs/user-guide/worktrees-and-sessions.md#starting-switching-and-closing-sessions)_

## User Scenarios & Testing *(mandatory)*

### User Story 1 - See which worktree I am in, above its terminal (Priority: P1)

A developer switches between several worktrees. Above the terminal they read the worktree's title
(its issue title, else its branch, else its name), its state, its short branch and its pull
request, without looking back at the sidebar.

**Why this priority**: the identity of the work is the reason for the redesign: today the terminal
area says nothing about which worktree it belongs to, apart from a session name in the bar below it.

**Independent Test**: open a worktree that has an issue title, a branch and an open pull request
and check the header shows each; switch to another worktree and check the header follows.

**Acceptance Scenarios**:

1. **Given** a worktree created from an issue titled "CI red on main: daemon tests" on branch
   `fix/1117_ci-red-on-main` with an open pull request #1118 whose checks fail, **When** one of its
   sessions is shown, **Then** the context header reads the title "CI red on main: daemon tests", a
   state chip with the location's card state (here Needs action), a branch chip `fix/1117` and a
   pull request chip naming #1118, that it is open and that its checks are failing.
2. **Given** a worktree with no issue title, **When** it is shown, **Then** the title is the
   worktree's branch, and with no branch either, its display name — the same title its Overview
   card shows.
3. **Given** a worktree with no pull request, or with pull request status switched off in
   Settings, **When** it is shown, **Then** the header has no pull request chip and nothing in its
   place.
4. **Given** the Default entry (the project root), **When** one of its sessions is shown, **Then**
   the title follows the Overview card's rule for the project root (*Default*, or the root's branch
   when it is known), the state chip shows the root's card state, the branch chip shows the root's
   short branch when it has one, and nothing else is invented for it.
5. **Given** the header is shown, **When** the location's state, branch or pull request changes
   (a session starts running, a pull request is opened or merged), **Then** the header shows the new
   value on the same refresh that updates the sidebar, and never shows a value different from the
   sidebar's for the same location.
6. **Given** a title or chip text too long for the space, **When** it is drawn, **Then** it is cut
   with an ellipsis and its full text is in a tooltip.

---

### User Story 2 - Switch, add and split from a tab strip above the terminal (Priority: P1)

The developer moves between the location's tabs, opens a new one and splits panes from one row just
above the terminal; the bar under the terminal is gone and nothing it offered is lost.

**Why this priority**: the bar is removed by this feature, so its controls have to land in the strip
in the same change, or a session is left with no way to switch, add or split.

**Independent Test**: with two tabs, switch between them, open a new one and split side by side and
stacked, all from the strip; check no bar is drawn under the terminal.

**Acceptance Scenarios**:

1. **Given** a session is shown, **When** the session area is drawn, **Then** the context header is
   at its top, the tab strip directly under it, the panes under that, and nothing under the panes.
2. **Given** the tab strip, **When** the developer presses a tab, **Then** that tab's terminal is
   shown (or the pane already showing it is focused), exactly as pressing the same tab in the bottom
   bar did.
3. **Given** the tab strip, **When** the developer presses its **New session** control, **Then**
   the sidebar's **Start another session...** action runs for the shown location, confirmation
   included, and the new helper session's tab is added after the location's other session tabs,
   selected and shown; cancelling the confirmation changes nothing. The "+" after the shown
   session's shell tabs opens a new shell instance exactly as the bottom bar's "+" did.
4. **Given** the strip, **When** the developer presses the split side by side or split stacked
   button at its right end, **Then** the focused pane splits as the same buttons in the bottom bar
   did, and the split buttons in each pane's own header stay.
5. **Given** the shown session's process has stopped, **When** the strip is drawn, **Then** the
   stopped state and the restart control are shown above the terminal, and the restart control
   restarts the process as the bottom bar's did.
6. **Given** a session is shown with one pane, **When** the strip is drawn, **Then** the shown
   session's name and its process status (for example running, starting, stopped) are readable above
   the terminal, on its selected tab or beside it, as the bottom bar's left end showed them.
7. **Given** a session whose AI tab the bottom bar pinned at its right corner, **When** the strip is
   drawn, **Then** that AI conversation is the shown session's own session tab, and it is reachable
   in one press at any tab count; another session's shell tabs are not drawn while it is not shown.
8. **Given** more tabs than fit, **When** the strip is drawn, **Then** it scrolls rather than
   shrinking, cutting or dropping a tab, the **New session** control and the split buttons keep their place and
   size, and the faded edge points at a marked tab beyond it, as the bottom bar's strip did
   (feature 027).

---

### User Story 3 - Open the worktree's changes from the header (Priority: P2)

The developer sees in the header how many files the worktree changed and how many lines were added
and removed, and opens the Changes view from there.

**Why this priority**: it gives the review flow an entry point next to the terminal; the right-click
**Review changes** route still works, so the feature is usable without it.

**Independent Test**: in a worktree with three changed files, read "3 files · +42 −7" on the button,
press it and check the Changes view opens for that worktree.

**Acceptance Scenarios**:

1. **Given** a worktree with changed files, **When** the header is drawn, **Then** the Changes
   button shows the number of changed files and the added and removed line counts, counted the way
   the Changes view's list counts them.
2. **Given** the Changes button, **When** the developer presses it, **Then** the Changes view opens
   for the shown location, as **Review changes** from the sidebar's right-click menu does.
3. **Given** a location with no changes, **When** the header is drawn, **Then** the button still
   opens the Changes view and says there are no changes instead of showing zero counts.
4. **Given** the counts have been read before and are being read again or the re-read fails,
   **When** the header is drawn, **Then** the button keeps the last known figure (its text does not
   change until a new figure arrives, #1342); **Given** no read has finished yet or the first read
   failed, **Then** the button shows no counts (no made-up figure). Either way it opens the view.
5. **Given** the Changes view is open, **When** the developer looks at the header, **Then** the
   button shows that the view is open, and pressing it again returns to the terminal as **Close**
   does.

---

### User Story 4 - Keyboard and focus work as before (Priority: P1)

A developer who drives panes from the keyboard notices no difference.

**Why this priority**: the acceptance criteria require pane chords and focus behaviour unchanged.

**Independent Test**: run every pane chord of `docs/user-guide/terminal-panes.md#keyboard-shortcuts`
with the terminal focused and check each acts as before; press header and strip controls and check
focus.

**Acceptance Scenarios**:

1. **Given** a focused terminal, **When** the developer uses any pane chord (split, close, move
   focus, previous or next session, location or project, Overview, quick-switcher), **Then** it acts
   exactly as before this feature.
2. **Given** a focused terminal, **When** the developer presses a control in the header or the
   strip, **Then** the press acts on its first try, and focus ends where pressing the same control in
   the bottom bar left it.
3. **Given** the header and strip, **When** their contents change (a chip appears, a tab is opened,
   a process stops), **Then** a press in flight on a header or strip control is not lost or given to
   its neighbour.

### Edge Cases

- **Empty**: no session is shown — the session area keeps today's "Select or start a session" message;
  no header or strip is drawn for a location that is not shown.
- **No branch** (detached head) or **unknown state**: the branch chip is left out; the state chip
  shows the state the Overview card would show.
- **Failure**: reading the pull request or the change counts fails — the chip or the counts are
  left out, the last value is not shown as current, and no dialog or notice appears. The Changes view
  opened from the header fails to read — it shows its own error as it does when opened from the
  sidebar. A restart pressed in the strip fails — the tab keeps its stopped state and the restart
  control, as in the bottom bar.
- **A worktree removed from another window** while shown: the header and strip go with it, as the
  terminal does today.
- **Several sessions in one location (Principle II)**: switching between them keeps the header (it
  names the location, not the session) and changes only the selected tab; two sessions never share a
  tab, and one session's stopped state is never shown on another's tab.
- **Several windows on one project**: each window's header shows its own shown location; one window
  opening the Changes view does not open it in another.
- **Sidebar hidden or resized**: header and strip use the width the session area gets.
- **Minimum window width**: at 640 px with the sidebar at its default width, no text or control
  overlaps another or leaves the window; the title and chips cut with an ellipsis, and the Changes
  button and split buttons stay whole and pressable.
- **Several panes**: the header and strip are drawn once above the whole pane area, not per pane;
  each pane keeps its own header with its name, state, split buttons and close.
- **Platforms (Principle VI)**: tooltips name Ctrl on Linux and Windows and Cmd on macOS; layout and
  behaviour are otherwise the same on all three, and the no-overlap rule (FR-013, SC-004) holds with
  each platform's fonts.
- **Overview open**: the Overview replaces the session area as today; the header and strip return
  when it is closed.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: The session area MUST show, from top to bottom, the context header, the tab strip and
  the panes, and MUST NOT show the bottom bar or any other bar under the panes.
- **FR-002**: The context header MUST show the shown location's title, chosen by the same rule as
  its Overview card's title (issue title, else branch, else display name).
- **FR-003**: The context header MUST show a state chip with the location's card state word, the
  same state its Overview card shows, distinguished by more than colour.
- **FR-004**: The context header MUST show a branch chip with the location's short branch when it
  has a branch, and no branch chip when it has none.
- **FR-005**: The context header MUST show a pull request chip naming the location's current pull
  request, its state and its checks state when that is known and pull request status is on, and no
  chip otherwise.
- **FR-006**: The context header MUST show a Changes button whose label carries the number of changed
  files and the added and removed line counts as the Changes view counts them; with no changes it
  MUST say so, and while the counts are unknown it MUST show no counts.
- **FR-007**: Pressing the Changes button MUST open the Changes view for the shown location, as the
  sidebar's **Review changes** does; while the view is open the button MUST show it is open and
  pressing it MUST close the view.
- **FR-008**: The tab strip MUST hold one session tab per session of the shown location (main first,
  then helpers), the shown session's shell tabs and a "+" directly after its session tab, and a
  **New session** control that runs **Start another session...** for the location, confirmation
  included; the split side by side and split stacked buttons sit at its right end and act on the
  focused pane.
- **FR-009**: Everything the bottom bar offered MUST stay above the panes with the same result: the
  shown session's name and process status, switching between its AI conversation and its shell
  instances, opening a new shell instance with "+", splitting and restarting a stopped process
  (US2 scenarios 2 to 7).
- **FR-010**: The tab strip MUST scroll when its tabs do not fit, keeping every session and shell tab at its own
  width and the **New session** control and split buttons at their place and size, and MUST keep the bottom bar's
  marked-tab edge cue (feature 027).
- **FR-011**: A press on a header or strip control MUST act on its first try and on that control
  only, also while focus, a process state, a chip or the tab count changes (the outcome feature 023
  FR-008a asks of the bottom bar).
- **FR-012**: Every pane chord and the focus rules of `docs/user-guide/terminal-panes.md` MUST behave
  as before this feature.
- **FR-013**: Text in the header that does not fit MUST be cut with an ellipsis and offered in full in
  a tooltip; at the minimum window width no element of the header or strip MUST overlap another or
  leave the window.
- **FR-014**: The header MUST follow the shown location: switching location or session, or a change
  of its title, state, branch, pull request or counts, MUST be shown on the same refresh that updates
  the sidebar or, for the counts, on the same refresh that updates the Changes view's own counts.
- **FR-015**: The user guide (`terminal-panes`, `reviewing-changes`), the layout snapshot's covered
  states and a visual pass MUST be updated to the new layout.

### Key Entities

- **Context header content**: title, card state, short branch (optional), current pull request with
  its state and checks state (optional), change counts (files, added, removed; or unknown) — all for
  one location, derived from what the Overview card and the Changes view already read.
- **Tab**: one entry of the strip, either a session tab (one session of the location: its name,
  main or helper, its process status) or a shell tab (one shell instance of the shown session), with
  whether it is shown, whether its process is stopped, and whether it is marked.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: For a location in each of the five card states, the header's title, state, branch and
  pull request text equals what its Overview card shows, with the sidebar hidden.
- **SC-002**: Every pane chord listed in `terminal-panes.md` gives the same result as before, in a
  check of each chord on each platform.
- **SC-003**: Opening the Changes view takes one press from the session area, against a right-click
  and a menu choice before.
- **SC-004**: At the minimum window width (640 px) and at 1280×800, in light and dark, no element of
  the header or strip overlaps another or overflows the window, with 1 tab (one session, no shell), 6 tabs and
  12 tabs (counting session and shell tabs together), on
  each of Linux, macOS and Windows.
- **SC-005**: Nothing is drawn under the panes in any covered layout state.
- **SC-006**: Every press on a header or strip control acts on the first try, including while the
  tab count or a process state changes.

## Out of scope

- The 44 px title bar and breadcrumb (#1127), the sidebar grouping by state (#1125) and the status
  bar (#1124): other issues of the same redesign.
- Opening the Changes view as a right inspector beside the terminal (an open question of the
  mockup): it keeps replacing the terminal.
- New pull request actions from the header (open in browser, merge); the chip only shows.
- The issue type chip ("Bug") drawn in the mockup: not in the issue's list.

## Assumptions

- Feature 1120 (merged) provides the title rule, the short branch and the card state; this feature
  reuses them and does not change them.
- The pull request chip uses what the worktree's pull request status already reads (feature on
  **Show pull request status on worktrees**); this feature reads nothing new from GitHub.
- The change counts come from the same reading the Changes view lists; when the header needs them
  before the view was opened, they are read in the background for the shown location only.
- The visual design follows the "Proposal: session view" mockup: a title line with chips under it,
  the Changes button at the header's right, a strip with an underline on the selected tab.
- The pane headers (feature 484) are unchanged.
