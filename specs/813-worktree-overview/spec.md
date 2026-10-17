# Feature Specification: Worktree Overview

**Feature Branch**: `claude/project-thread-2lmupk`

**Created**: 2026-10-17

**Status**: Draft

**Input**: User description: "Implement GitHub issue #813: feat: worktree overview with
state-aware cards (sessions, PRs, task progress). Add a project Overview: a grid of Material cards,
one per worktree, so the user can see at a glance what every worktree is doing, which ones need
attention, and jump into any of them. Each card shows name / branch and worktree type with a type
icon, the number of sessions (and how many are running), the number of open and merged pull
requests for the worktree's branch, and a task summary with progress. An Overview button in the top
bar next to the project selector opens and closes it and carries an attention badge; the reserved
chord Ctrl+Shift+O (Cmd+Shift+O on macOS) toggles it even while a terminal has focus and never
reaches the terminal. Activating a card opens the worktree's main session. Card colour and icon
derive from one state per worktree (worst wins): needs action (red), needs attention (yellow),
working, done (green), idle (greyed). Task summary from data already at hand (linked issue title,
spec-kit tasks.md progress, phase, autopilot ledger), and from Claude Code hook data (latest todo
list, prompt) where hooks are configured; an LLM summary is optional. Open questions: overlay or
main pane; definition of main session; LLM summary in scope." Labels: enhancement, effort:high,
flow:feature.

## Terms

- **Project**: the project the window has open (the one the project selector shows).
- **Location**: a worktree of the project, an included worktree (one the repository knows about
  outside the app's own worktree folder), an orphan or invalid worktree folder, or the project root
  (shown as **Default** in the sidebar). This is the same set the sidebar lists.
- **Overview**: a view of the project that shows one card per location, opened and closed from the
  top bar or with the Overview shortcut.
- **Card**: the Overview's entry for one location.
- **Location type**: what kind of location a card stands for, one of: *feature* (branch type
  `feat`), *bugfix* (`fix`), *chore* (any other conventional type: `chore`, `docs`, `refactor`,
  `test`, `build`, `ci`, `perf`, `style`), *other* (a branch that follows no type convention),
  *included*, *orphan / invalid* (a folder with no usable worktree: the sidebar's *missing* or
  *invalid* status), and *project root*. Included, orphan / invalid and project root take
  precedence over the branch type.
- **Main session**: the session the sidebar selects for a location (feature 776): the un-archived
  session marked main, else the most recently active un-archived session.
- **Location state** (feature 905): Running, Blocked (needs permission, failed, resumable), Idle /
  waiting or Unknown, derived from the location's un-archived sessions by feature 905's rules.
- **Card state**: the one state a card shows, the worst of its location's signals, from worst to
  best:
  1. **Needs action**: the location state is *Blocked: needs permission*, or a session of the
     location has an agent request waiting for the user's confirmation (feature 034, daemon tool
     server).
  2. **Needs attention**: the location state is *Blocked: failed* or *Blocked: resumable*, or the
     branch's current pull request has failing checks, has changes requested, or cannot merge
     because of a conflict.
  3. **Working**: the location state is Running.
  4. **Done**: none of the above, and the branch's pull request is merged and the branch holds no
     work beyond it (the sidebar's *removable* signal).
  5. **Idle**: every other case (the location state is Idle / waiting or Unknown and no other rule
     applies). Unknown never yields Working or Done by itself.
- **Pull request counts**: the number of open (including draft) and of merged pull requests whose
  head is the location's branch. Closed-unmerged pull requests are not counted.
- **Task summary**: one line saying what the location is working on. **Progress**: how far that
  work is, as a phase and, when known, a done / total count.
- **Attention badge**: a mark on the Overview button that shows the worst card state across all
  cards when it is Needs action or Needs attention.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - See every worktree's state in one place (Priority: P1)

A developer runs agents in several worktrees of one project. They open the Overview and see one
card per location, each with its name, branch, type, session counts, pull request counts and a
state shown by colour, icon and a word. The locations that need them come first.

**Why this priority**: It is the core of the issue: one glance answers "what is every worktree
doing and which one needs me". Every other story builds on the cards.

**Independent Test**: In a project with the root, three worktrees (one with a session waiting for a
permission, one working, one with no session) and one included worktree, open the Overview from
the top-bar button. Five cards show; the permission card is first, red, with an attention icon and
the word *Needs action*; the idle card is greyed.

**Acceptance Scenarios**:

1. **Given** a project with worktrees, included worktrees, an orphan folder and the project root,
   **When** the user opens the Overview, **Then** it shows exactly one card per location the
   sidebar lists, each card marked with its location type by an icon and a word.
2. **Given** a location with three un-archived sessions, two of them running, **When** the user
   looks at its card, **Then** it reads 3 sessions, 2 running; archived sessions are not counted.
3. **Given** a branch with one open and two merged pull requests, **When** the user looks at its
   card, **Then** it reads 1 open and 2 merged.
4. **Given** locations in each of the five card states, **When** the user looks at the Overview,
   **Then** each card shows its state's colour, icon and word, and the cards are ordered Needs
   action, Needs attention, Working, Done, Idle.
5. **Given** a session waiting for a permission and its pull request with failing checks, **When**
   the user looks at its card, **Then** it shows Needs action (the worst state wins).
6. **Given** the Overview open, **When** a session starts working, finishes, or its pull request's
   checks change, **Then** the affected card's state, counts and position update without any action
   from the user.

---

### User Story 2 - Open and close the Overview from anywhere (Priority: P1)

The user reaches the Overview with one click on the top-bar button next to the project selector,
or with Ctrl+Shift+O (Cmd+Shift+O on macOS), even while typing in a terminal. The button shows
whether something needs them while the Overview is closed.

**Why this priority**: Without an entry point the Overview cannot be reached; the badge carries
its value to the moments when the Overview is closed.

**Independent Test**: With focus in a terminal, press Ctrl+Shift+O: the Overview opens and the
terminal receives nothing; press it again: the Overview closes and the previous view returns. Make
a session ask for a permission: the button shows a red badge.

**Acceptance Scenarios**:

1. **Given** the main window, **When** the user looks at the top bar, **Then** an Overview button
   with an icon sits next to the project selector, and its tooltip names the Overview and its
   shortcut for the user's platform.
2. **Given** the Overview closed, **When** the user clicks the button or presses the shortcut,
   **Then** the Overview opens; **When** it is open and they do it again, **Then** the Overview
   closes and the view shown before it returns.
3. **Given** a terminal with keyboard focus, **When** the user presses the shortcut, **Then** the
   Overview toggles and the terminal's program receives no input from the press.
4. **Given** any location in Needs action, **When** the user looks at the button, **Then** it shows
   a red attention badge; given none in Needs action but one in Needs attention, a yellow badge;
   otherwise no badge. The badge has a tooltip line or accessible label naming the count.
5. **Given** the shortcut, **When** the user reads the user guide or the shortcut list in settings,
   **Then** the Overview shortcut is listed there.

---

### User Story 3 - Jump into a worktree from its card (Priority: P1)

From the Overview the user activates a card and lands in that location's main session, ready to
answer the permission or read what the agent did.

**Why this priority**: Seeing that a worktree needs action is only half the job; reaching it in one
step is what saves the time.

**Independent Test**: Open the Overview, click the red card: the Overview closes and the main
session of that worktree is selected and shown.

**Acceptance Scenarios**:

1. **Given** a card whose location has a main session, **When** the user clicks it or presses Enter
   on it, **Then** the Overview closes and that session is selected and shown, as if chosen in the
   sidebar.
2. **Given** a card whose location has no un-archived session, **When** the user activates it,
   **Then** the location is selected and the user is offered to start a session there, with the
   same choices as starting one from the sidebar; nothing starts without the user's confirmation.
3. **Given** a card for an orphan / invalid location, **When** the user activates it, **Then** the
   location is selected in the sidebar and no session is offered (a session cannot start there).
4. **Given** the Overview open, **When** the user presses Tab or the arrow keys, **Then** focus
   moves from card to card, and Enter or Space activates the focused card; **When** they press
   Escape, **Then** the Overview closes and the previous view returns.
5. **Given** a greyed Idle card, **When** the user activates it, **Then** it behaves as any other
   card does.

---

### User Story 4 - Know what each worktree is working on and how far it got (Priority: P2)

Each card says in one line what the location is working on and how far along it is, from what the
app already knows: the linked issue's title, the spec-kit task list and phase in the worktree, and
the autopilot ledger, with no model call.

**Why this priority**: It turns a list of states into a picture of the work, but the cards are
useful without it.

**Independent Test**: In a worktree whose branch names issue #42 and whose feature folder holds a
task list with 6 of 10 tasks ticked, the card shows the issue's title and *Implement · 6/10*. A
worktree with none of those shows its branch name and no progress.

**Acceptance Scenarios**:

1. **Given** a location whose branch or worktree names a ticket whose issue title the app knows,
   **When** the user looks at its card, **Then** the summary is that title; otherwise it is the
   branch name; for the project root it is *Project root*.
2. **Given** a worktree whose feature folder has a task list, **When** the user looks at its card,
   **Then** the progress shows the ratio of ticked tasks to all tasks (for example *6/10*) and a
   bar of the same proportion.
3. **Given** a worktree with a feature folder, **When** the user looks at its card, **Then** the
   phase reads *Spec*, *Plan*, *Tasks* or *Implement*, by the furthest artifact present, or *PR*
   when the branch has an open pull request, or *Done* when the card state is Done; an autopilot
   ledger's recorded phase, when present, wins.
4. **Given** a location with no feature folder, no ledger and no hook data, **When** the user looks
   at its card, **Then** it shows the summary line and an empty-state progress text (*No task
   progress*), never a 0% bar.
5. **Given** a worktree whose task list changes on disk, **When** the user looks at the card while
   the Overview is open, **Then** the progress shows the new ratio within 5 seconds.

---

### User Story 5 - See what the agent is doing now (Priority: P3)

Where the agent CLI reports its own activity through hooks, the card shows the agent's latest todo
list progress and what it is doing now, instead of only the artifact-derived summary.

**Why this priority**: Richer and more current than Story 4, but only for sessions with hooks
configured; Story 4 stays the fallback.

**Independent Test**: In a worktree whose Claude Code session has hooks configured, give a prompt
that makes the agent write a todo list of 5 items and finish 2. The card shows the in-progress item
as the summary and *2/5* as the progress; a session without hooks shows Story 4's summary.

**Acceptance Scenarios**:

1. **Given** a session with hooks configured whose agent writes a todo list, **When** the user looks
   at the location's card, **Then** the progress is that list's done / total and the summary is its
   in-progress item (else the latest prompt the user submitted, shortened to one line).
2. **Given** a location whose main session has hook data and whose worktree has a task list,
   **When** the user looks at the card, **Then** the hook data is shown as the current activity and
   the task list ratio stays visible as the feature's progress.
3. **Given** a session without hook data, or the window reconnecting to the service, **When** the
   user looks at the card, **Then** it falls back to Story 4's summary and progress, without an
   error.

---

### Edge Cases

- **Empty project**: a project with only the root shows one card; the Overview is never blank.
- **No session anywhere**: every card is Idle and greyed; the button shows no badge.
- **Several sessions** (Principle II): counts include every un-archived session, main and helpers;
  the card state takes every one of them into account (worst wins); activation opens the main one.
- **Unknown activity** (a CLI with no readable activity): the session counts as not running, makes
  no card Needs action, and leaves the card Idle unless something else applies.
- **Pull request status off, unreadable or rate-limited**: the counts read *—* (unknown), never 0,
  the PR-derived states (failing, changes requested, conflict, Done) are not claimed, and the card
  state comes from sessions alone. A tooltip says why the counts are missing.
- **No branch** (detached HEAD, orphan folder, project root on the default branch): the pull request
  counts are not shown for that card.
- **Linked issue unreadable or offline**: the summary falls back to the branch name.
- **Malformed or huge task list**: lines that are not checkbox items are ignored; a task list with
  no checkbox items gives no ratio; reading a task list never blocks the window.
- **Several feature folders in one worktree**: the one named by the worktree's current spec-kit
  feature record is used, else the one whose number matches the branch's ticket, else none.
- **A worktree added, removed or renamed while the Overview is open**: its card appears, disappears
  or is renamed within the same refresh as the sidebar.
- **Project switched while the Overview is open**: the Overview shows the new project's locations.
- **Disconnected from the session service**: cards keep their last state and show that the data
  may be stale; on reconnect they take the current state.
- **Several windows** on the same project: each window opens and closes its own Overview; every
  window shows the same card state for the same location.
- **Many locations** (30 or more): the grid scrolls; the attention cards stay first.
- **Narrow window**: the grid reflows to fewer columns, down to one; card text truncates with an
  ellipsis and the full text is in a tooltip; no card overlaps another.
- **Shortcut conflicts**: Ctrl+Shift+O (Cmd+Shift+O) differs from every reserved chord (E, T, D, H,
  W, arrows, C, V); with the Overview's chord reserved, a terminal program that relied on receiving
  it no longer does (same trade-off as the other reserved chords).
- **Cross-platform (Principle VI)**: the shortcut is Cmd+Shift+O on macOS and Ctrl+Shift+O on Linux
  and Windows; icons, colours and layout are the same on all three.
- **Light and dark theme**: every card state is distinguishable in both, and by icon and word
  without colour.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: The Overview MUST show exactly one card for each location the sidebar lists for the
  active project, including included and orphan / invalid worktrees and the project root, and no
  other card.
- **FR-002**: Each card MUST show the location's display name, its branch when it has one, and its
  location type as an icon and a word.
- **FR-003**: Each card MUST show the number of the location's un-archived sessions and how many of
  them are running (location state of that session is Running).
- **FR-004**: Each card with a branch MUST show the number of its open (including draft) and merged
  pull requests, or an unknown mark with an explanatory tooltip when they cannot be read.
- **FR-005**: Each card MUST show one card state, computed by the Terms' ordered rules for every
  combination of session, agent-request and pull request signals, and MUST show it by a colour role
  of the theme, an icon and a word together; no card state may be told apart by colour alone.
- **FR-006**: Idle cards MUST be visibly de-emphasised (greyed) compared with every other state,
  while staying legible and activatable.
- **FR-007**: Cards MUST be ordered by card state (Needs action, Needs attention, Working, Done,
  Idle), and within a state in the sidebar's order.
- **FR-008**: The Overview MUST update while open, within 1 second of the window learning of a
  change to a session, an agent request or a pull request reading, and within 5 seconds of a task
  list or ledger change on disk, with no user action.
- **FR-009**: The top bar MUST show an Overview button next to the project selector, with an icon,
  a tooltip naming the Overview and its shortcut for the platform, and a visible pressed or selected
  appearance while the Overview is open.
- **FR-010**: Activating the button or pressing the shortcut MUST open the Overview when it is
  closed, and close it, returning to the view shown before, when it is open.
- **FR-011**: The Overview shortcut MUST be Ctrl+Shift+O (Cmd+Shift+O on macOS), reserved by the
  app like the focus-out and new-terminal chords: it MUST work while a terminal has focus and MUST
  NOT send any input to the terminal's program.
- **FR-012**: The Overview button MUST show a red attention badge when any card is Needs action,
  else a yellow one when any card is Needs attention, else none, whether or not the Overview is
  open; the badge MUST carry an icon or count, not colour alone, and an accessible label.
- **FR-013**: Activating a card (pointer or keyboard) MUST close the Overview and select and show
  the location's main session; with no un-archived session it MUST select the location and offer to
  start one, and MUST NOT start one without the user's confirmation; for a location where a session
  cannot start it MUST only select the location.
- **FR-014**: Cards MUST be reachable and activatable from the keyboard (focus moves between cards;
  Enter or Space activates), and Escape MUST close the Overview.
- **FR-015**: Each card MUST show a task summary: the linked issue's title when the app knows it,
  else the branch name, else the location's display name.
- **FR-016**: Each worktree card MUST show progress from the worktree's own files when present: the
  phase (Spec, Plan, Tasks, Implement, PR, Done) and the ratio of ticked to all checkbox tasks in
  the feature's task list; an autopilot ledger's recorded phase, when present, MUST win over the
  phase derived from files.
- **FR-017**: A card with no task summary source beyond its name and no progress source MUST show
  an explicit empty state, never a zero-progress bar.
- **FR-018**: Where a session reports its agent's todo list or latest prompt through hooks, the
  card MUST show that list's done / total and its in-progress item (else the latest prompt, on one
  line) as the current activity, and MUST fall back to FR-015 and FR-016 when that data is absent.
- **FR-019**: Building the Overview MUST NOT start any new network polling of its own: pull request
  data MUST come from the readings the app already makes, and their counts from the same source;
  reading the worktrees' files MUST NOT block the window.
- **FR-020**: Prompts, todo items and issue titles shown on cards MUST stay on this machine: they
  MUST NOT be logged or sent to any service by this feature.
- **FR-021**: The Overview, its card and the attention badge MUST be built from the shared UI
  components (Principle VIII), with every colour from the theme's roles, and the card MUST be shown
  in each of its five states in the component showcase.
- **FR-022**: The user guide MUST describe the Overview, its card states and what makes a location
  enter each, the progress sources, and the shortcut; the shortcut MUST also be listed wherever the
  app's shortcuts are documented (Principle VII).
- **FR-023**: The Overview MUST render [NEEDS CLARIFICATION: in place of the main pane (sidebar and
  top bar stay visible), or as an overlay over the whole window?].

### Key Entities

- **Location overview**: per location, derived and never stored: the location, its type, session
  count and running count, open and merged pull request counts (or unknown), card state, task
  summary, phase, task ratio and current activity.
- **Card state**: Needs action, Needs attention, Working, Done or Idle, with the reason that made it
  (for the tooltip: for example *needs permission*, *agent request waiting*, *checks failing*,
  *changes requested*, *merge conflict*, *session failed*, *resumable*, *merged*).
- **Task progress**: phase, optional done / total, and its source (task list, ledger, hooks).
- **Session activity summary**: per session, the agent's latest todo list (items with done and
  in-progress marks) and latest submitted prompt, when hooks report them; held only while the
  session lives.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: With ten locations in mixed states, a user names every location that needs them from
  the Overview alone in under 5 seconds.
- **SC-002**: From any view, including a focused terminal, the user reaches the main session of a
  location that needs action in at most two actions (shortcut or button, then the card).
- **SC-003**: In an automated pass over every combination of session lifecycle and activity, agent
  request and pull request state, the card state matches the Terms' rules in 100% of cases.
- **SC-004**: 100% of session and pull request changes the window learns of appear on the affected
  card within 1 second, and task list changes within 5 seconds.
- **SC-005**: In a greyscale rendering in both themes, 100% of card states and location types are
  identified correctly by icon and word.
- **SC-006**: Pressing the shortcut with a terminal focused delivers 0 bytes to the terminal's
  program.
- **SC-007**: Opening the Overview on a project with 30 locations shows every card within 1 second.

## Out of Scope

- An AI-written summary of a session's transcript (the issue's option 3): it sends transcript text
  to a model and costs tokens; a later feature may add it, opt-in.
- Overviews across several projects at once.
- Acting on a location from its card (starting, stopping, archiving, merging, deleting); the card
  only navigates.
- Filtering, searching, or user-chosen sorting of cards, and remembering the Overview open across
  restarts.
- New kinds of pull request data beyond what the app reads today, except the counts and the
  merge-conflict signal this feature needs.
- Changing the sidebar's state icon, notifications or the unread mark.

## Assumptions

- The sidebar's location list (feature 029, with included worktrees from its BUG-002), feature
  905's location state and feature 776's main session already exist; the Overview reuses them, so a location shows
  the same state on its card as on its sidebar row.
- The app's existing pull request readings (feature 040 status per branch, feature 775 list of a
  branch's pull requests) are the source of counts and check / review / merged signals. Where a
  reading does not yet carry what a card needs (counts for every branch, a merge conflict), carrying
  it on the existing reading is part of this feature; no separate poller is added.
- An agent confirmation request names the session that asked; where the window does not yet know
  that session's location, carrying it is part of this feature.
- The ticket and type come from the worktree's naming convention (`<type>-<ticket>-<name>` folder,
  `<type>/<ticket>-<name>` branch); the issue title is the one the app already looks up for issue
  links, and none is fetched just for the Overview when the lookup is off or offline.
- Hook-fed activity (Story 5) needs the daemon to keep the latest todo list and prompt per session
  from the hook events it already receives, and to send them to the window with the session's other
  state; CLIs or sessions without hooks simply have none.
- "Running" in the session count means the session's feature 905 state is Running.
