# Feature Specification: Notify Only When a Session Needs Attention, With Per-Kind Settings and Icons

**Feature Branch**: `claude/project-thread-8kdqkn`

**Created**: 2026-10-06

**Status**: Draft

**Input**: User description: "Implement GitHub issue #613: Notifications: only notify when a session needs attention; add per-kind settings and icons. Problem: notifications fire for every session state change, so the ones that matter (a session that needs real attention) are buried. There is no setting to choose which kinds of notifications to receive. Notifications carry no icon, so important ones can't be told apart from routine ones at a glance. Expected: notify by default only when a session needs real attention (e.g. waiting for input or permission, errored, finished a long task). Let the user choose which notification kinds they want (per kind on/off). Give each kind a distinct icon, so urgency is visible without reading. Context: this builds on specs/039-session-attention-notifications (and 575-workspace-attention-list)."

## What already exists, and the gap

Feature 039 (`specs/039-session-attention-notifications`) raises one desktop notification each time a session not in view changes to **awaiting input** (039 FR-001), with one **Desktop notifications** switch for all of them (039 FR-026). Awaiting input covers every way an AI CLI stops for the user: the end of every turn, however short, and a stop in the middle of a turn to ask for a permission. So a user who drives several sessions gets a notification at the end of every turn of every session they are not looking at, and the few that need them — a permission that blocks the work, a long task that is done — look exactly like the many that do not. A session that ends with an error raises nothing at all (039 Out of Scope).

Feature 575 (`specs/575-workspace-attention-list`) counts unread sessions on the sidebar's location rows. Neither feature is changed by this one in what makes a session unread, with one exception: a helper agent finishing inside a turn no longer counts as the end of the turn (FR-024).

This feature sorts attention events into **kinds**, notifies by default only for the kinds that need the user, lets the user choose per kind, and gives each kind its own icon.

## Clarifications

### Session 2026-10-06

- Q: Is the long-task threshold fixed at 60 seconds, or user-configurable in Settings? → A: User-adjustable in Settings, in whole seconds, 60 by default, shown next to the **Long task finished** switch. _(decided by the user, 2026-10-06, D4 = B; replaces the earlier provisional answer "fixed at 60 seconds")_

## Terms

- **Awaiting input**, **in view**, **desktop notification**, **in-app notice**, **session service**, **unread**: as defined by feature 039 (Terms and Key Entities).
- **Turn**: one stretch of work a session does for the user, from the moment it starts working on what the user (or an agent driving it) gave it until it hands control back by finishing. A stop to ask for a permission or an answer in the middle of the work does not end the turn; the turn goes on when the user answers.
- **Turn duration**: the time from the start of a turn to its finish, including any time the session spent waiting for a permission or an answer in between.
- **Notification kind**: the reason a session needs the user, as one of the four kinds below. Each attention event and each error ending has exactly one kind.
  - **Needs permission**: the session stopped in the middle of a turn to ask for a permission or for an answer, and cannot continue until the user responds.
  - **Long task finished**: the session finished a turn whose turn duration was at least the long-task threshold.
  - **Turn finished**: the session finished a turn whose turn duration was below the long-task threshold.
  - **Session error**: the session ended because of an error: its AI CLI reported an error and stopped, or its process exited abnormally and the session service did not bring it back (it gave up restarting it after repeated crashes, or could not restart it). An abnormal exit that the session service recovers from by restarting the session is not an ending.
- **Long-task threshold**: the turn duration from which a finished turn counts as a long task. A setting, in whole seconds from 10 to 3600, 60 by default (FR-025).
- **Kind icon**: the icon that stands for one notification kind, distinct in shape from the other three.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Be notified only when a session really needs me (Priority: P1)

A developer runs five sessions across two projects. Today every turn of every session they are not looking at raises a desktop notification, dozens an hour, and they have learned to ignore them all — including the one that says a session has been waiting twenty minutes for a permission. With this feature, out of the box, a session not in view notifies them only when it asks for a permission or an answer, when it finishes a task that took a while, or when it stops with an error. Quick back-and-forth turns raise nothing; the session's unread mark still shows they finished.

**Why this priority**: It is the problem the issue states: the notifications that matter are buried. It works on its own, with the defaults, without the user opening Settings.

**Independent Test**: On a fresh installation, with session A in view, have session B (not in view) finish a 5-second turn, then a 2-minute turn, then stop mid-turn for a permission, then have session C end with an error. Confirm exactly three desktop notifications, for the 2-minute turn, the permission and the error, and that B is unread after the 5-second turn.

**Acceptance Scenarios**:

1. **Given** default settings and session B not in view, **When** B finishes a turn whose turn duration is below the long-task threshold, **Then** no desktop notification appears, and B becomes unread as today (039 FR-016).
2. **Given** default settings and session B not in view, **When** B finishes a turn whose turn duration is at or above the long-task threshold, **Then** exactly one desktop notification of kind **Long task finished** appears for B.
3. **Given** default settings and session B not in view and working, **When** B stops in the middle of its turn to ask for a permission or an answer, **Then** exactly one desktop notification of kind **Needs permission** appears for B.
4. **Given** default settings and session B not in view, **When** B ends because of an error, **Then** exactly one desktop notification of kind **Session error** appears for B.
5. **Given** default settings and session B not in view, **When** B ends without an error — the user closed or stopped it, or its AI CLI exited normally — **Then** no desktop notification appears.
6. **Given** session B is in view, **When** any of the four kinds happens to B, **Then** no desktop notification appears (039 FR-002).
7. **Given** session B asked for a permission, was notified, and the user answered so that B worked on, **When** B finishes the turn, **Then** the turn's kind is decided by its whole turn duration, the time spent waiting for the answer included, and B is notified for it only if that kind is on.
8. **Given** session B finished a turn and was notified, **When** its AI CLI reports again that it is waiting without having worked in between, **Then** no further notification appears (039 FR-003).
9. **Given** sessions of Claude Code, GitHub Copilot and Pi Coding Agent, **When** each finishes a long turn while not in view, **Then** each raises one **Long task finished** notification.
10. **Given** no window of the application is open, **When** a session asks for a permission, finishes a turn or ends with an error, **Then** no desktop notification appears, neither then nor when the application is next opened (039 FR-008).
11. **Given** a Claude Code session B not in view and working, **When** a helper agent it runs finishes inside the turn, **Then** no desktop notification appears, B does not become unread, and when B later finishes the turn its kind is decided by the whole turn's duration (FR-024).

---

### User Story 2 - Choose which kinds of notifications I get (Priority: P2)

A developer wants to hear about every finished turn while they pair with an agent on one task; another only cares about permissions and errors; a third finds a minute too short to call a task long. In Settings, under **Desktop notifications**, each kind has its own switch with its icon, its name and one line saying when it fires, and next to **Long task finished** a field sets, in seconds, how long a turn must last to count as a long task. The master switch still turns everything off at once.

**Why this priority**: The issue asks for it, and it is what lets users who want the old behaviour have it back. Story 1 delivers the value with the defaults; this story makes it adjustable.

**Independent Test**: Turn **Turn finished** on and **Long task finished** off, have a session not in view finish a short turn and then a long one, and confirm one notification for the short turn and none for the long one. Turn **Long task finished** back on, set the long-task threshold to 20 seconds, and confirm a 30-second turn raises **Long task finished**. Restart and confirm the switches and the threshold kept their values.

**Acceptance Scenarios**:

1. **Given** a fresh installation, **When** the user opens Settings, **Then** below the **Desktop notifications** switch there is one switch per kind, in the order **Needs permission**, **Session error**, **Long task finished**, **Turn finished**, each with its kind icon, its name and a one-line description; the first three are on and **Turn finished** is off.
2. **Given** **Turn finished** is on, **When** a session not in view finishes a short turn, **Then** one **Turn finished** notification appears.
3. **Given** a kind's switch is off, **When** an event of that kind happens to a session not in view, **Then** no notification appears for it, and the session's unread state changes exactly as with the switch on.
4. **Given** the user changes a kind's switch while sessions are running, **When** the next event of that kind happens, **Then** the new choice applies; nothing needs restarting. Events that happened while the kind was off are not notified afterwards.
5. **Given** the **Desktop notifications** switch is off, **When** the user looks at the kind switches, **Then** they are shown with the positions they had and cannot be changed, and no notification of any kind appears.
6. **Given** **Desktop notifications** was off and the user turns it on, **When** an event happens, **Then** the kind switches decide as they were before it was turned off.
7. **Given** the user changed kind switches and restarted the application (or the computer), **When** they open Settings, **Then** every switch is as they left it.
8. **Given** a settings file written before this feature, with **Desktop notifications** on or off, **When** the application starts, **Then** **Desktop notifications** keeps its value and the kind switches take their defaults.
9. **Given** the kind switches, **When** the user looks at Settings, **Then** there is no switch per AI CLI; each kind switch applies alike to sessions of every AI CLI (039 FR-028).
10. **Given** a fresh installation, **When** the user opens Settings, **Then** next to the **Long task finished** switch there is a long-task threshold field showing 60, in seconds, with its allowed range stated.
11. **Given** the user sets the long-task threshold to 20 seconds and saves, **When** a session not in view finishes a 30-second turn, **Then** one **Long task finished** notification appears; a 15-second turn is **Turn finished**. Nothing needs restarting, and a turn already running when the value was saved is classified by the new value when it finishes.
12. **Given** the user types a value below 10, above 3600, or not a whole number into the threshold field, **When** they save, **Then** the save is refused with a message naming the allowed range, the field is marked, and the stored threshold is unchanged.
13. **Given** a settings file written before this feature, or one whose threshold is outside 10–3600 seconds, **When** the application starts, **Then** the threshold is 60 seconds for the first, and the nearest bound (10 or 3600) for the second.

---

### User Story 3 - Tell kinds apart at a glance by their icon (Priority: P3)

A developer glances at a notification in the corner of the screen. Without reading it, the icon tells them whether a session is blocked on a permission, has crashed, or has merely finished its work.

**Why this priority**: The issue asks for it. Stories 1 and 2 already cut the noise; the icon makes the remaining notifications faster to triage.

**Independent Test**: Trigger one notification of each kind and confirm each carries its own icon, that the four icons differ in shape, and that each matches the icon beside its switch in Settings.

**Acceptance Scenarios**:

1. **Given** one notification of each of the four kinds, **When** the user looks at them, **Then** each shows its kind's icon, and no two kinds share an icon.
2. **Given** a notification of a kind, **When** the user compares its icon with Settings, **Then** it shows the same glyph as the icon beside that kind's switch.
3. **Given** a notification of each kind, **When** the user reads it, **Then** its title says the kind in words — for example "<session> needs permission", "<session> stopped with an error", "<session> finished a long task", "<session> finished its turn" — so the kind is clear where an icon is not shown.
4. **Given** an operating system whose notification facility does not show an icon supplied by the application, **When** a notification appears, **Then** it still appears, and its title alone tells the kind.
5. **Given** the light and the dark theme of the desktop, **When** a notification of each kind appears, **Then** its icon is recognisable in both.

---

### Edge Cases

- **A permission asked at the end of a turn**: a session that has already finished its turn and then reports a permission or idle prompt does not change state, so it raises nothing new (039 FR-003). Only a stop in the middle of a turn is **Needs permission**.
- **A helper agent finishing inside a turn** (for example Claude Code's subagent stop): the session goes on with its turn and is not waiting for the user, so this is not a change to awaiting input. It raises no notification, does not make the session unread, and neither ends nor pauses the turn (FR-024).
- **Several permission requests in one turn**: each stop in the middle of the turn, after the session worked again, is a new **Needs permission** event and is notified (when the kind is on and the session not in view).
- **Turn duration exactly at the threshold**: counts as a long task.
- **Threshold changed while a turn is running**: the turn is classified by the threshold in force when it finishes. Turns that already finished keep their kind; nothing is notified again or withdrawn.
- **A turn whose start the application did not see** (the application or the session service started while the session was already working, or the connection was lost before the turn began and was restored after it ended): the turn duration is measured from the earliest moment the application knew the session was working; a turn whose start is unknown is **Turn finished** unless the known part already reaches the threshold.
- **A session that ends while waiting for a permission**: if the ending is an error, it is **Session error**; otherwise nothing.
- **A permission refused, and the turn ends there**: the session was already awaiting input since it asked, so the end of the turn is no new change: no notification and no new unread mark (039 FR-003).
- **An error ending after an earlier error ending was notified**: a session that is restarted and ends with an error again raises a new **Session error** notification; repeated crashes the session service handles by restarting the session raise nothing until it gives up, which raises one.
- **A session the user stops or closes**: not an error; no notification.
- **AI CLIs that cannot report a stop in the middle of a turn**: a session whose AI CLI does not report permission requests never raises **Needs permission**; its turns are still classified by their duration.
- **Activity unknown**: a session whose activity the application does not know raises no notification of any kind (039 FR-005), except **Session error**, which depends only on how the session ended.
- **Unread is unchanged**: every change to awaiting input while not in view still makes the session unread, whatever its kind and whatever the switches say (039 FR-016, FR-017). A **Session error** ending does not make a session unread (039 Edge Cases, "Session ended or crashed").
- **Many sessions at once** (Principle II): events of different kinds in different sessions at the same moment each produce their own notification, of their own kind, with their own icon; none is lost or attributed to another session.
- **Several windows**: one event raises one notification (039 FR-006a), of the same kind whichever window shows it.
- **Reconnection**: a session found awaiting input after a reconnection is treated as a change at the moment of reconnection (039 FR-006); its kind is the one the session service decided when the session changed, and is **Turn finished** when the application cannot tell. An event the session service counted before it was itself restarted is not notified afterwards, whatever its kind (039 H3: activity does not survive a restart of the session service).
- **Clicking a notification**: does what 039 FR-011 to FR-015a say, for every kind. Clicking a **Session error** notification shows the ended session as the sidebar shows any ended session.
- **Settings file unreadable**: the application runs on default settings, so the three default kinds are on, **Turn finished** is off and the long-task threshold is 60 seconds.
- **Operating system refuses notifications**: as 039 FR-010, for every kind.
- **Sessions in a container** (sandbox): behave the same as sessions run directly on the computer (039 FR-007).
- **Cross-platform** (Principle VI): the kinds, defaults and switches behave the same on Linux, macOS and Windows; only whether the icon is shown depends on the system's notification facility (FR-016).

## Requirements *(mandatory)*

### Functional Requirements

**Kinds**

- **FR-001**: The application MUST assign every change of a session into awaiting input, and every error ending, exactly one notification kind, whether or not that kind is on: **Needs permission**, **Long task finished**, **Turn finished** or **Session error**, as defined under Terms.
- **FR-002**: A change to awaiting input in the middle of a turn — the session stopped to ask for a permission or an answer — MUST be **Needs permission**. A change to awaiting input because the session finished its turn MUST be **Long task finished** when the turn duration is at least the long-task threshold, and **Turn finished** otherwise.
- **FR-003**: The turn duration MUST include time the session spent waiting for a permission or an answer during the turn, and MUST be measured from the start of the turn as the application observed it (Edge Cases, "A turn whose start the application did not see").
- **FR-004**: A session that ends because of an error — its AI CLI reported an error and stopped, or its process exited abnormally and the session service did not bring it back (it gave up restarting it, or could not restart it) — MUST be **Session error**. An abnormal exit followed by a successful restart MUST raise no notification. A session that ends because the user closed or stopped it, or whose AI CLI exited normally, MUST raise no notification.

**When to notify**

- **FR-024**: A helper agent that an AI CLI runs inside a turn finishing its work MUST NOT count as a change to awaiting input, and MUST NOT end or pause the turn. This replaces 010/039's treatment of Claude Code's subagent stop as the end of a turn, which marked a working session as waiting and unread, and could end a long task's turn early.

- **FR-005**: The application MUST raise exactly one desktop notification for an event when all of these hold: desktop notifications are on, the event's kind is on, the session is not in view at that moment, and a window of the application is open. In every other case it MUST raise none.
- **FR-006**: For **Needs permission**, **Long task finished** and **Turn finished**, the rules of 039 for awaiting input apply unchanged: one notification per change into awaiting input (039 FR-003), none for a session found already awaiting input at start (039 FR-005), the reconnection rule (039 FR-006), one per event across windows (039 FR-006a), none while no window is open (039 FR-008), each event its own notification (039 FR-009).
- **FR-007**: For **Session error**, the application MUST raise at most one notification per ending, the same across windows, and none for a session it finds already ended when it starts or reconnects, nor for an ending that happened while no window was open.
- **FR-008**: The notification's text MUST name the session's project, worktree and session as 039 FR-004 requires, and its title MUST state the kind in words. It MUST contain no other text from the conversation and no error message from the AI CLI.

**Settings**

- **FR-009**: Settings MUST keep the **Desktop notifications** switch (039 FR-026) and MUST offer, below it, one switch per notification kind, in the order **Needs permission**, **Session error**, **Long task finished**, **Turn finished**, each with its kind icon, its name and a one-line description of when it fires.
- **FR-010**: On a fresh installation, with a settings file written before this feature, and when the settings file cannot be read, **Needs permission**, **Session error** and **Long task finished** MUST be on and **Turn finished** MUST be off. A settings file written before this feature MUST keep its **Desktop notifications** value.
- **FR-011**: The kind switches MUST be stored with the application's other settings, on the user's computer only (Principle IV), and kept across restarts.
- **FR-012**: While **Desktop notifications** is off, the kind switches and the long-task threshold MUST be shown with their stored values and MUST NOT be changeable, and no notification of any kind may be raised. Turning it on again MUST restore the kinds and the threshold as they were.
- **FR-013**: A change to any switch, or to the long-task threshold, MUST take effect for the next event, for sessions already running, without a restart; a turn running when the threshold changes is classified by the threshold in force when it finishes. Events that happened while their kind or the master switch was off MUST NOT be notified afterwards.
- **FR-014**: Each kind switch, and the long-task threshold, MUST apply alike to sessions of every AI CLI. The application MUST NOT offer switches or thresholds per AI CLI (039 FR-028).
- **FR-025**: Settings MUST offer the long-task threshold next to the **Long task finished** switch, as a whole number of seconds from 10 to 3600, 60 by default, stated in seconds with its allowed range. It MUST be stored with the application's other settings, on the user's computer only, and kept across restarts. A settings file written before this feature, or that cannot be read, MUST give 60. It MUST stay changeable while **Long task finished** is off (it still divides **Turn finished** from **Long task finished**).
- **FR-026**: A threshold typed into Settings that is not a whole number, or lies outside 10–3600, MUST be refused on save with a message naming the allowed range, and the stored value MUST stay unchanged. A threshold outside 10–3600 found in the settings file, or sent to the session service, MUST be clamped to the nearest bound.

**Icons**

- **FR-015**: Each notification kind MUST have its own kind icon, distinct from the other three in shape, not by colour alone. The same glyph MUST be shown beside the kind's switch in Settings and in the kind's desktop notifications (in a notification it may sit on a coloured tile, FR-017).
- **FR-016**: The application MUST supply the kind icon to the operating system's notification facility on every system where that facility shows an icon supplied by the application. Where it does not, the notification MUST still be shown, and the title (FR-008) carries the kind.
- **FR-017**: The four kind icons MUST stay distinct from one another when drawn in a single colour at 16 by 16 pixels, MUST be supplied to the notification facility in a form with a contrast ratio of at least 3:1 against both a white and a black background, and MUST show beside an enabled switch in Settings, in the application's light and dark theme, with a contrast ratio of at least 3:1 against their background (beside a disabled switch they take the disabled colour, as its label does).

**Unchanged behaviour, components, documentation**

- **FR-018**: What makes a session unread, what clears it, the unread marks, the switcher's counts and the location rows' indicators MUST behave as they do today (039 FR-016 to FR-025; 575), except that a helper agent finishing inside a turn no longer makes a session unread (FR-024). No notification kind or switch changes them.
- **FR-019**: Clicking a notification of any kind MUST do what 039 FR-011 to FR-015a say.
- **FR-020**: When the operating system does not show a notification, 039 FR-010 applies, for every kind.
- **FR-021**: Everything above MUST behave the same on Linux, macOS and Windows (Principle VI), and with the session service running directly on the computer or in a container (039 FR-007).
- **FR-022**: The kind icons, the Settings row that pairs a kind icon with its switch, and the long-task threshold field MUST come from the shared component library (Principle VIII); the component showcase MUST show the four kind icons, the kind switches and the threshold field (with a value, and refused with its message), in the light and the dark theme, with **Desktop notifications** on and off.
- **FR-023**: The user guide MUST describe the four kinds, when each fires, their defaults, their icons, the switches and how they combine with **Desktop notifications**, the long-task threshold (its default, range and when a change applies), and that unread marks do not depend on them, in the same change that ships each (Principle VII). The user guide's icon reference MUST list the four kind icons.

### Key Entities

- **Attention event** (existing, 039): one change of one session into awaiting input. Gains a kind: **Needs permission**, **Long task finished** or **Turn finished**.
- **Error ending** (new): one session ending because of an error. Its kind is **Session error**. It is notified but does not make the session unread.
- **Notification kind**: one of four reasons to notify, each with a name, a one-line description, a kind icon and a default (on for three, off for **Turn finished**).
- **Turn**: a stretch of a session's work from its start to its finish, with a duration; known to the application only while it observes the session.
- **Notification kind settings**: one on/off choice per kind and the long-task threshold in seconds, stored with the application's settings beside the existing **Desktop notifications** switch, which stays the master switch.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: With default settings, a session not in view that goes through 20 short turns without a permission request, 5 long turns without one, 5 short turns that each stop once for a permission (each turn's whole duration, the wait included, below the long-task threshold) and then 1 error ending produces exactly 11 desktop notifications — 5 **Long task finished**, 5 **Needs permission**, 1 **Session error** — and 0 for the 25 short turn ends, in 5 of 5 trials.
- **SC-002**: For each kind, with only that kind on, 20 of 20 events of that kind on a session not in view produce a notification and 0 events of the other kinds do, on Linux, macOS and Windows.
- **SC-003**: The unread marks and counts after the trials of SC-001 and SC-002 are the same as they would be with every switch on, in 20 of 20 trials.
- **SC-004**: Shown one notification of each kind in random order, a user names its kind from the icon alone, without reading the text, in at least 9 of 10 trials, on every system that shows the application's icon.
- **SC-005**: A user who wants to stop **Long task finished** notifications finds and turns off its switch in Settings within 30 seconds, in 5 of 5 trials.
- **SC-006**: After changing kind switches and the long-task threshold and restarting the application, every switch keeps its position and the threshold its value in 20 of 20 trials.
- **SC-008**: With the long-task threshold set to T seconds, for T of 10, 60 and 3600, a turn of T seconds or more is **Long task finished** and a turn of less than T is **Turn finished**, in 20 of 20 trials each.
- **SC-007**: Every trial above gives the same result with the session service running directly on the computer and in a container.

## Out of Scope

- Changing what makes a session unread or what clears it (FR-024 aside), or limiting unread marks to the enabled kinds.
- Notifications for sessions that end without an error, sessions whose activity is unknown, or sessions that work for a long time without finishing.
- Switches per AI CLI, per project or per session.
- Sound per kind, a taskbar or dock badge, a tray icon.
- Limiting the rate of notifications, grouping them, and withdrawing a notification once its session was opened.
- Showing the AI CLI's error message or the permission being asked in the notification.
- Notifications for events while no window is open (039 FR-008).

## Assumptions

- "Waiting for input or permission" in the issue is the session stopping in the middle of a turn for the user (**Needs permission**). A session at an idle prompt after its turn has finished has already been classified by that turn; idle reminders raise nothing (039 FR-003).
- "Every session state change" in the issue is 039's notification at the end of every turn. Short turn ends are what bury the important notifications, so **Turn finished** is off by default; users who want 039's behaviour back turn it on.
- "Errored" means an error ending as defined under **Session error**. Closing or stopping a session yourself is not an error.
- AI CLIs differ in whether they report an error before stopping: today only GitHub Copilot does (`session.error`). For the others, an error is seen only as an abnormal exit that the session service cannot recover from. This is accepted, not worked around.
- The long-task threshold defaults to 60 seconds: long enough that a quick exchange stays quiet, short enough that a user who switched away is told when a real task is done. The user adjusts it in Settings (D4). The range 10–3600 seconds follows the clamp-on-read, refuse-on-save rule of the existing environment-include timeout: below 10 seconds nearly every turn would be "long", and above an hour **Long task finished** would hardly ever fire, which turning the kind off already expresses.
- AI CLIs differ in what they report: a CLI that does not report a stop in the middle of a turn cannot raise **Needs permission**; its stops are classified by turn duration. This is accepted, not worked around.
- The four kinds are fixed. Adding further kinds is a later request.
- The **Desktop notifications** switch stays as the master switch so that 039's one-switch behaviour and its stored value carry over unchanged.
- Whether a notification shows an icon supplied by the application, and how large, is the operating system's decision; the title states the kind wherever it is not shown.
- **Dependencies**: 039 attention events, desktop notification, unread state and Settings switch; 575 location-row indicators (unchanged); the session activity states and their detection for Claude Code, GitHub Copilot and Pi Coding Agent; session supervision and its crash-loop give-up; the shared icon set and component showcase (Principle VIII); the user guide's settings and icon chapters (Principle VII).
