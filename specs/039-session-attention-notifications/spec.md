# Feature Specification: Notify When a Session Needs Attention, and Track Unread Sessions

**Feature Branch**: `feat/notify-session-needs-attention`

**Created**: 2026-10-02

**Status**: Draft

**Input**: User description: "Implement GitHub issue #481 (https://github.com/jaroslawherod/micold-ai-ide/issues/481): Notify when a session needs attention and track unread sessions. When a session moves to awaiting input (turn ended, permission or idle prompt), show an OS desktop notification naming the project, worktree and session; clicking it focuses the window and switches to that session. Mark a session unread when it produces a turn the user has not viewed, and clear the mark when the user opens the session. Show an unread count on the project in the switcher. Suppress the notification when the session is the one currently focused. Settings: on/off for desktop notifications, and optionally per provider. Works in both the host and the sandboxed daemon runtime."

## Terms

- **Awaiting input**: the existing session activity state the sidebar's activity indicator already shows when a session's AI CLI has stopped and waits for the user — its turn ended, it asks for a permission, or it sits at an idle prompt. This feature adds no new way of detecting it.
- **In view**: a session is in view when all three hold: it is the selected session of a window; that window has the operating system's keyboard focus; and the window's main area shows the session — its AI conversation or one of its regular terminal tabs — rather than a screen that takes the main area over, such as Settings. A session is **not in view** when no window of the application is open, when another session or another project is selected, when its window is unfocused, minimised, or on another desktop, or when Settings or another such screen fills the main area.
- **Desktop notification**: a notification shown by the operating system's own notification facility, outside the application's window. The application's existing in-window notices are a different thing and are called **in-app notices** here.
- **Session service**: where sessions run — directly on the user's computer, or in a container (the sandbox).

## Clarifications

### Session 2026-10-02

- Q: FR-008: a session changes to awaiting input while no window is open. What must happen? → A: (b) No desktop notification. The session is shown as unread when the application is next opened. Unread state outlives the window and is stored on the user's computer. _(decided by user)_
- Q: FR-023: must the switcher's button in the top bar show, with its panel closed, that other projects hold unread sessions? → A: (a) Yes. The button carries the total number of unread sessions in projects other than the active one, and nothing when that is zero. _(decided by user)_
- Q: FR-028: is a separate notification switch per AI CLI (Claude Code, GitHub Copilot, Pi Coding Agent) part of this feature? → A: (a) No. One switch covers every AI CLI; a per-CLI choice is left for a later request and is out of scope. _(decided by user)_

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Be told when a session I am not watching needs me (Priority: P1)

A developer runs several agents in parallel: three sessions in one project, two more in another. They are reading one session, or working in their editor with the application's window behind it. One of the other sessions finishes its turn, or stops to ask for a permission. Today nothing tells them; the session waits until they happen to look at the sidebar. They want the operating system to show a notification that says which project, worktree and session is waiting — once, and never for the session they are already looking at.

**Why this priority**: It is the problem the issue states. An agent that waits unnoticed is wasted time, and the cost grows with every parallel session. It stands alone: even with nothing else in this feature, the user learns that a session waits and where it is.

**Independent Test**: Start two sessions in a project. Select the first. Have the second finish a turn. Confirm one desktop notification appears naming the project, the second session's worktree and the second session. Then have the first (selected, window focused) finish a turn and confirm no notification appears.

**Acceptance Scenarios**:

1. **Given** session A is in view and session B of the same project is working, **When** B changes to awaiting input, **Then** exactly one desktop notification appears, and its text names B's project, B's worktree and B by the name its sidebar row shows.
2. **Given** session A is in view, **When** A changes to awaiting input, **Then** no desktop notification appears.
3. **Given** session A is selected but the application's window does not have keyboard focus (another application is in front, or the window is minimised), **When** A changes to awaiting input, **Then** exactly one desktop notification appears for A.
4. **Given** the user is in project P and session C runs in background project Q, **When** C changes to awaiting input, **Then** exactly one desktop notification appears naming Q, C's worktree and C.
5. **Given** session B is already awaiting input and was notified, **When** its AI CLI reports again that it is waiting (an idle reminder, a repeated prompt) without having worked in between, **Then** no further desktop notification appears.
6. **Given** session B was notified, then worked again, **When** B changes to awaiting input a second time while not in view, **Then** one new desktop notification appears.
7. **Given** a session runs without a worktree (the "Default" entry), **When** it changes to awaiting input while not in view, **Then** the notification names the project, the Default entry by the name the sidebar gives it, and the session.
8. **Given** the session service runs in a container, **When** a session not in view changes to awaiting input, **Then** the notification appears on the user's desktop exactly as it does with the session service running directly on the computer.
9. **Given** session A is selected in a focused window and Settings fills the window's main area, **When** A changes to awaiting input, **Then** exactly one desktop notification appears for A.
10. **Given** session A is selected in a focused window that is showing one of A's regular terminal tabs instead of its AI conversation, **When** A changes to awaiting input, **Then** no desktop notification appears.
11. **Given** the application lost its connection to the session service while session B, not in view, was working, **When** it reconnects and finds B awaiting input, **Then** exactly one desktop notification appears for B.
12. **Given** three sessions not in view change to awaiting input within one second, **When** the user looks at their desktop notifications, **Then** there are three, one per session, each naming its own session.
13. **Given** no window of the application is open and session B is working, **When** B changes to awaiting input, **Then** no desktop notification appears — neither at that moment nor when the application is next opened.

---

### User Story 2 - See which sessions have turns I have not looked at (Priority: P2)

The developer comes back from a meeting. Several sessions finished turns while they were away; some notifications have long since faded. In the sidebar they want each session that finished a turn they have not yet looked at to carry an unread mark, and in the project switcher they want each project to show how many such sessions it holds, so that work waiting in a background project is visible. Without opening the switcher they want its button to say how many sessions wait in other projects. Sessions that finished a turn while the application was closed are marked when it is opened again. Opening a session clears its mark.

**Why this priority**: A notification is momentary; the unread mark is what remains when it was missed, dismissed, or turned off. It works without story 1 — unread marks need no desktop notification — and story 1 works without it.

**Independent Test**: With desktop notifications turned off, start two sessions in project P and one in project Q. While viewing the first session of P, have the other two finish a turn. Confirm the second session of P carries the unread mark, the switcher's button shows `1` with its panel closed, the open switcher shows `1` unread on P and `1` on Q, and that selecting each session removes its mark, lowers its project's count to none and removes the number from the button.

**Acceptance Scenarios**:

1. **Given** session B is not in view, **When** B changes to awaiting input, **Then** B's sidebar row carries the unread mark.
2. **Given** session A is in view, **When** A changes to awaiting input, **Then** A's row carries no unread mark.
3. **Given** session B is unread, **When** the user selects B in a focused window, **Then** B's unread mark disappears and its project's unread count falls by one.
4. **Given** project Q holds two unread sessions and project P none, **When** the user opens the project switcher, **Then** Q's row shows an unread count of 2 and P's row shows no unread count.
5. **Given** the active project holds an unread session, **When** the user opens the project switcher, **Then** the active project's row shows its unread count too.
6. **Given** session A is selected but its window is unfocused, **When** A changes to awaiting input, **Then** A is unread; **When** the window regains focus with A still selected, **Then** A's unread mark disappears.
7. **Given** session C was the selected session of background project Q and is unread, **When** the user switches to Q and C is shown, **Then** C's unread mark disappears.
8. **Given** session B is unread, **When** B starts working again (another agent or a queued prompt drives it) without the user having brought it into view, **Then** B stays unread.
9. **Given** session B is unread, **When** B is closed or removed, or its worktree is deleted, **Then** its mark is gone and its project's unread count no longer includes it.
10. **Given** desktop notifications are turned off, **When** a session not in view changes to awaiting input, **Then** it becomes unread exactly as with notifications on.
11. **Given** session A is selected in a focused window, Settings fills the main area, and A changed to awaiting input and is unread, **When** the user leaves Settings so that A is shown, **Then** A's unread mark disappears.
12. **Given** session A is selected in a focused window that is showing one of A's regular terminal tabs, **When** A changes to awaiting input, **Then** A does not become unread.
13. **Given** the application lost its connection to the session service while session B, not in view, was working, **When** it reconnects and finds B awaiting input, **Then** B is unread.
14. **Given** an unread session's worktree is hidden by the sidebar's tag filter, **When** the user opens the project switcher, **Then** the project's unread count includes that session.
15. **Given** project P is active, background project Q holds two unread sessions and background project R one, and the switcher's panel is closed, **When** the user looks at the switcher's button in the top bar, **Then** it shows an unread count of 3.
16. **Given** the active project P holds one unread session and no other project holds any, **When** the user looks at the switcher's button with its panel closed, **Then** it shows no unread count.
17. **Given** the switcher's button shows an unread count of 3 for projects Q (two) and R (one), **When** the user switches to Q, **Then** the button shows 1 for as long as R's session stays unread, whether or not Q's sessions were read.
18. **Given** no window of the application is open and session B is working, **When** B changes to awaiting input and the user later opens the application, **Then** B's row carries the unread mark and its project's unread count includes it.
19. **Given** session B was unread when the last window was closed, **When** the user opens the application again, **Then** B is still unread.
20. **Given** session A was awaiting input and not unread when the last window was closed, and its activity did not change while no window was open, **When** the user opens the application again, **Then** A is not unread.
21. **Given** session A was awaiting input and not unread when the last window was closed, then worked and changed to awaiting input again while no window was open, **When** the user opens the application again, **Then** A is unread.
22. **Given** session B changed to awaiting input while no window was open and then started working again, **When** the user opens the application and B is not the session shown, **Then** B is unread.
23. **Given** session B changed to awaiting input while no window was open and is the session the application selects when it opens, **When** its window is shown with keyboard focus and B in view, **Then** B's unread mark disappears as for any session that comes into view.

---

### User Story 3 - Go straight to the session from its notification (Priority: P3)

The developer is in their editor when the notification appears. They click it. The application's window comes to the front, switches to the session's project if another was active, and shows that session, ready to be answered.

**Why this priority**: It turns the notification from a message into a shortcut. Stories 1 and 2 are useful without it — the notification names the session and the unread mark finds it — so it comes third.

**Independent Test**: With project P active and the application's window behind another application, have a session of background project Q finish a turn, click its desktop notification, and confirm the window is in front and focused, Q is the active project, and that session is the selected one with its terminal shown.

**Acceptance Scenarios**:

1. **Given** a desktop notification for session B of the active project, **When** the user clicks it, **Then** the application's window comes to the front with keyboard focus and B is the selected session.
2. **Given** a desktop notification for session C of background project Q, **When** the user clicks it, **Then** the window comes to the front, Q becomes the active project, and C is the selected session.
3. **Given** the user clicked a notification and the session is shown, **When** they look at the sidebar, **Then** that session's unread mark is gone.
4. **Given** a notification for session B, and B has since been closed or removed, **When** the user clicks the notification, **Then** the window comes to the front, the selected project and session stay as they were, and an in-app notice says the session is no longer available.
5. **Given** the user clicked a notification that switched projects, **When** they return to the project they left, **Then** its sessions are still running and its selected session is the one they left (existing background-project behaviour).
6. **Given** two windows are open and session B's project is open in the second, **When** the user clicks B's notification, **Then** the second window comes to the front and shows B; the first window is unchanged.

---

### User Story 4 - Turn desktop notifications off (Priority: P4)

A developer who shares their screen, or who finds the notifications distracting, opens Settings and turns desktop notifications off. From that moment nothing is sent. The unread marks stay, because they disturb nobody.

**Why this priority**: A control over a new interruption is required by the issue, but the feature's value is delivered by the first three stories with notifications on, which is the default.

**Independent Test**: Turn **Desktop notifications** off in Settings, have a session not in view finish a turn, and confirm no desktop notification appears and the session is unread. Turn it on again and confirm the next such turn raises a notification.

**Acceptance Scenarios**:

1. **Given** a fresh installation, **When** the user opens Settings, **Then** a **Desktop notifications** switch is shown and it is on.
2. **Given** desktop notifications are off, **When** any session changes to awaiting input, in view or not, **Then** no desktop notification appears.
3. **Given** the user turns desktop notifications off while sessions are running, **When** one of those sessions next changes to awaiting input, **Then** no desktop notification appears; nothing needs restarting.
4. **Given** the user turned desktop notifications off and restarted the application, **When** they open Settings, **Then** the switch is still off.
5. **Given** desktop notifications are off, **When** the user turns them on, **Then** the next session not in view that changes to awaiting input raises one desktop notification; sessions that changed while it was off raise none after the fact.
6. **Given** sessions of Claude Code, GitHub Copilot and Pi Coding Agent, **When** the user looks at Settings, **Then** there is one **Desktop notifications** switch and no switch per AI CLI; with it on, a session of any of the three that changes to awaiting input while not in view raises a notification, and with it off none does.

---

### Edge Cases

- **No sessions, or one session in view**: nothing is notified and nothing is unread; the switcher shows no unread count anywhere.
- **Activity unknown**: a session whose activity the application does not know — one discovered from outside the application and not yet started here, or a Pi session with **Show activity for Pi sessions** off — never changes to awaiting input, so it is never notified and never unread.
- **Session ended or crashed**: a session that ends is not awaiting input. It raises no desktop notification and does not become unread. Telling the user about ended sessions is out of scope.
- **Repeated "still waiting" signals**: an AI CLI may report that it is waiting more than once for the same wait. Only the change into awaiting input counts (story 1, scenario 5).
- **Very short turns**: a session that goes working → awaiting input → working → awaiting input raises one notification per change into awaiting input while not in view, however quickly they follow. The application adds no rate limit and merges nothing; the operating system may stack or collapse notifications as it sees fit.
- **Many sessions at once** (Principle II): changes in different sessions at the same moment each produce their own notification and their own unread mark. None is lost, and none is attributed to another session.
- **Application start**: sessions the application finds already awaiting input when it starts are not notified — it saw no change (FR-005). A session that changed to awaiting input while no window was open is unread, also when it has worked again since (FR-008), and a session that was unread when the last window was closed is still unread (FR-008a). A session that was awaiting input and read when the last window was closed, and has not changed since, is not unread.
- **First start with this feature**: the application has no record of what the user viewed before. No session is unread on that start; marks begin with the changes that follow.
- **Stored unread state unreadable or lost**: the application starts with no session unread and shows no error for it; marks begin again with the changes that follow.
- **Session removed while no window was open**: a session that was unread and no longer exists when the application is opened is not counted anywhere (FR-020).
- **Connection to the session service lost and restored while a window is open**: the application knows each session's state from before the loss. A session it last saw in another state and finds awaiting input after reconnecting has changed, only observed late: it is notified and becomes unread under the usual conditions (FR-006). A session that was already awaiting input before the loss is not notified again.
- **Session comes into view between the change and the notification**: if the session is in view at the moment its change to awaiting input is observed, nothing is sent and it is not unread.
- **Sessions driven by another agent**: a session that an orchestrating agent drives through the application's agent tools changes to awaiting input after each turn like any other, and is notified and marked unread like any other. Telling the two kinds apart is out of scope; the Settings switch (story 4) is the remedy.
- **Session renamed after the notification was sent**: the notification keeps the name the session had when it was sent. Clicking it still opens that session.
- **Session with a placeholder name**: the notification shows the same placeholder label the sidebar row shows.
- **Project forgotten, or its folder unavailable, after the notification was sent**: clicking brings the window to the front and shows the in-app notice of story 3, scenario 4; nothing else changes.
- **The operating system refuses or cannot show notifications** (notifications disabled for the application in the system's settings, Do Not Disturb, no notification service running on a Linux desktop): no desktop notification appears and nothing else is affected — sessions run, the activity indicator and unread marks work, and no in-app error is shown per event (FR-010).
- **The desktop's notification service cannot report a click** (some Linux notification services show text only): the notification still appears; clicking it does whatever that service does. The unread mark is how the user finds the session.
- **Notification clicked after the window that raised it was closed, while another window is open**: the notification service has no window left to report the click to (FR-015). The open windows do not change; the session is found by its unread mark.
- **Notification clicked after the last window was closed**: notifications stay in the operating system's notification list after the application has quit. Clicking one then is not required to open the session; if the operating system starts the application in response, it opens as on any start (FR-015a). No new notification is raised while no window is open (FR-008). The session is found by its unread mark or its activity indicator.
- **Stale notifications**: a notification for a session the user has since opened is not withdrawn by the application. Clicking it opens the session again, which changes nothing.
- **Several windows**: one change raises one desktop notification, not one per window (FR-006a). The unread state of a session is the same in every window.
- **Screen sharing and lock screens**: the notification's text is limited to the three names (FR-004). A session's name comes from its AI CLI's title for the conversation, so it may hint at the work; the Settings switch turns notifications off.
- **Settings file unreadable**: the application runs on default settings as it does today, so desktop notifications are on.
- **Cross-platform** (Principle VI): the issue asks for Linux and macOS; the constitution requires parity, so Windows behaves the same (FR-029). Where an operating system asks the user to allow notifications from the application, that prompt is the system's own.

## Requirements *(mandatory)*

### Functional Requirements

**Desktop notification**

- **FR-001**: When a session's activity changes to awaiting input from any other state, the session is not in view at that moment, and desktop notifications are on, the application MUST raise exactly one desktop notification for that session.
- **FR-002**: When the session is in view at that moment, the application MUST NOT raise a desktop notification for it.
- **FR-003**: A session that is already awaiting input MUST NOT raise another desktop notification until it has left that state and changed to it again. Repeated waiting signals for the same wait raise nothing.
- **FR-004**: The notification's text MUST name the session's project, its worktree (for a session without a worktree, the name the sidebar gives the Default entry), and the session by the label its sidebar row shows at that moment, including a placeholder label. It MUST contain no other text taken from the conversation.
- **FR-005**: The application MUST NOT raise a desktop notification for a session whose activity is unknown, working or ended, nor for a session it finds already awaiting input when it starts.
- **FR-006**: When the application, with a window open, loses its connection to the session service and reconnects, a session it last saw in a state other than awaiting input and now finds awaiting input MUST be treated as having changed at the moment of reconnection: FR-001, FR-002 and FR-016 apply to it. A session it last saw awaiting input and finds awaiting input MUST raise nothing and keep its unread state.
- **FR-006a**: One change to awaiting input MUST raise one desktop notification however many of the application's windows are open.
- **FR-007**: Notifications MUST work the same whether the session service runs directly on the computer or in a container: the notification appears on the user's desktop, nothing has to be installed or configured inside the container for it, and the container is given no network access or host access it does not have today.
- **FR-008**: While no window of the application is open, a session that changes to awaiting input MUST NOT raise a desktop notification, neither at that moment nor when the application is next opened. When the application is next opened, every session that changed to awaiting input while no window was open MUST be unread, whatever its activity did afterwards (FR-020) — including a session that was awaiting input and read when the last window was closed and has since worked and changed to awaiting input again, and a session that was created while no window was open and reached awaiting input, which counts as a change. FR-008a states the two exceptions.
- **FR-008a**: Unread state MUST outlive the application's windows: a session that was unread when the last window was closed MUST be unread when the application is next opened, also after the application or the computer was restarted. A session that was awaiting input and not unread when the last window was closed, and whose activity has not changed since, MUST NOT be unread. On the first start with this feature, and when the stored unread state cannot be read, no session is unread and no error is shown.
- **FR-009**: Changes in several sessions at the same time MUST each raise their own notification (when FR-001 holds for them) and set their own unread mark; none may be lost, merged by the application, or attributed to another session.
- **FR-010**: When the operating system does not show a notification — it is refused, disabled for the application, or no notification service is available — sessions, the activity indicator and unread marks MUST be unaffected, no in-app error may be shown for the event, and the application MUST record the failure in its log at most once per run.

**Opening the session from the notification**

- **FR-011**: Clicking a desktop notification MUST bring a window of the application to the front with keyboard focus, make the session's project the active project of that window, and select the session so that its terminal is shown.
- **FR-012**: The window MUST be the one in which the session's project is already open. When the project is open in no window, it MUST be the window that most recently had focus.
- **FR-013**: When the session no longer exists or cannot be shown — it was closed or removed, its worktree was deleted, its project was forgotten or its folder is unavailable — clicking MUST bring the window to the front, leave the selected project and session as they were, and show an in-app notice saying the session is no longer available.
- **FR-014**: Clicking MUST NOT stop, interrupt, restart or send input to any session. Sessions of a project left by the click keep running, as with any project switch.
- **FR-015**: Where the desktop's notification service cannot report a click to the application, the notification MUST still be shown. FR-011 to FR-013 apply wherever the service does report it: on macOS and Windows with the system's own notification facility, and on Linux with any notification service that reports that a notification was activated.
- **FR-015a**: FR-011 to FR-013 apply while the application is running. A notification clicked after the application has quit is not required to open the session, and MUST NOT leave the application in a state different from an ordinary start if the operating system starts it in response. (No notification is raised while no window is open: FR-008.)

**Unread sessions**

- **FR-016**: A session MUST become unread when its activity changes to awaiting input while it is not in view. A session in view at that moment MUST NOT become unread. With no window open no session is in view (FR-008).
- **FR-017**: Becoming unread MUST NOT depend on the desktop-notification setting, nor on whether the operating system showed a notification.
- **FR-018**: An unread session's sidebar row MUST carry an unread mark that is distinct from the activity indicator, is told apart by more than colour, and is visible in the light and the dark theme.
- **FR-019**: A session MUST stop being unread when it comes into view — by being selected in a focused window, by a notification click, by a switch to its project that shows it, by its window regaining focus while it is selected, by the application opening with it selected in a focused window, or by a screen such as Settings closing so that the session is shown. The mark MUST be gone within 1 second of that.
- **FR-020**: An unread session MUST stay unread while it is not in view, whatever its activity does in the meantime. A session that is closed or removed, or whose worktree is deleted, MUST no longer be unread or counted.
- **FR-021**: In the project switcher, each project's row MUST show the number of its unread sessions when that number is one or more, and no unread count when it is zero. The count MUST be shown for the active project as for the others, beside the existing running count, and MUST be told apart from it by a word or a symbol, not by position or colour alone.
- **FR-022**: A project's unread count MUST include every unread session of the project: sessions of the Default entry, and sessions of worktrees the sidebar's filter currently hides.
- **FR-023**: The switcher's button in the top bar MUST show, with its panel closed as well as open, the total number of unread sessions in all projects other than the active one when that total is one or more, and no unread count when it is zero. Unread sessions of the active project MUST NOT be counted in it. The number MUST be told apart from the project's name by a word or a symbol, not by colour alone, MUST be visible in the light and the dark theme, and MUST follow FR-022 and the 1-second limit of FR-019.
- **FR-024**: A session's unread state MUST be the same in every open window, and MUST be kept while the user switches projects. It outlives the last window (FR-008a).
- **FR-025**: Unread state MUST be held and stored on the user's computer only and MUST NOT be sent anywhere (Principle IV).

**Settings**

- **FR-026**: Settings MUST offer one **Desktop notifications** switch, on by default, stored with the application's other settings and kept across restarts.
- **FR-027**: While the switch is off, the application MUST NOT raise any desktop notification this feature defines. Turning it off or on MUST take effect for the next change to awaiting input, for sessions already running, without a restart. Changes that happened while it was off MUST NOT be notified afterwards.
- **FR-028**: The **Desktop notifications** switch MUST apply alike to sessions of every AI CLI (Claude Code, GitHub Copilot, Pi Coding Agent). The application MUST NOT offer a separate notification switch per AI CLI; that choice is out of scope.

**Platforms, consistency and documentation**

- **FR-029**: Everything above MUST behave the same on Linux, macOS and Windows (Principle VI), using each system's own notification facility.
- **FR-030**: The unread mark and the unread count MUST be provided by the shared component library and used from there by the sidebar and the switcher (Principle VIII); the component showcase MUST show a session row with the unread mark, a switcher row with an unread count, and the switcher's button with an unread count.
- **FR-031**: The user guide MUST describe the desktop notification, what clicking it does, the unread mark, the counts on the switcher's rows and button, that sessions which finished a turn while the application was closed are unread when it is opened, and the Settings switch — including that the operating system may ask for, or withhold, permission to show notifications — in the same change that ships each (Principle VII).
- **FR-032**: The existing activity indicator, the switcher's running count and the existing in-app notices MUST look and behave as they do today.

### Key Entities

- **Session** (existing): gains an **unread** state — yes or no — set when it changes to awaiting input while not in view, or while no window is open, and cleared when it comes into view. It is stored on the user's computer and kept across restarts of the application.
- **Attention event**: one change of one session's activity into awaiting input. It is what a desktop notification and an unread mark are raised for; a session produces a new one only after leaving awaiting input.
- **Desktop notification**: the operating system's message for one attention event, carrying the project, worktree and session names and leading back to that session when clicked.
- **Project unread count**: the number of unread sessions a project holds, shown on the project's row in the switcher.
- **Other-projects unread total**: the sum of the project unread counts of every project except the active one, shown on the switcher's button.
- **Desktop notifications setting**: the user's one on/off choice for sessions of every AI CLI, stored with the application's settings.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: In 20 of 20 trials on each of Linux, macOS and Windows, a session that changes to awaiting input while not in view produces exactly one desktop notification, and it appears within 2 seconds of the sidebar's activity indicator showing the change.
- **SC-002**: In 20 of 20 trials, a session that changes to awaiting input while in view produces 0 desktop notifications and no unread mark.
- **SC-003**: With desktop notifications off, 0 desktop notifications appear over 20 changes to awaiting input, while all 20 sessions concerned that were not in view become unread.
- **SC-004**: Clicking a notification shows the named session, in a focused window, within 2 seconds, in 20 of 20 trials — including when the session is in a background project.
- **SC-005**: With 10 sessions across 3 projects changing to awaiting input within 1 second, none in view, the user gets 10 notifications naming 10 different sessions, 10 unread marks, and switcher counts that sum to 10.
- **SC-006**: An unread mark, and its project's count, are updated within 1 second of the session coming into view, in 20 of 20 trials.
- **SC-007**: Every trial above gives the same result with the session service running directly on the computer and with it running in a container.
- **SC-008**: A user asked "which of your sessions finished a turn you have not looked at, and in which projects?" answers correctly from the sidebar and the switcher alone, without opening any session.
- **SC-009**: In 20 of 20 trials, a session that changes to awaiting input while no window is open produces 0 desktop notifications and is unread when the application is next opened; a session that was awaiting input and read at closing, and did not change, is not unread.
- **SC-010**: With the switcher's panel closed, the number on its button equals the number of unread sessions in projects other than the active one in 20 of 20 trials, and is absent when that number is zero.

## Out of Scope

- Telling the user about sessions that ended or crashed, sessions that have worked for a long time, or sessions whose activity is unknown.
- Saying in the notification why the session waits (turn ended, permission, idle prompt).
- Telling sessions driven by another agent apart from sessions the user drives.
- Sound, a taskbar or dock badge, and a tray icon.
- Limiting the rate of notifications, grouping them, and withdrawing a notification once its session was opened.
- Unread counts in the **Known projects** list of the main window and in the folder browser.
- Opening the session from a notification clicked after the application has quit (FR-015a).
- A count of unread turns per session: unread is yes or no.
- Desktop notifications for changes that happen while no window is open, at the time or after the fact (FR-008).
- A separate notification switch per AI CLI (FR-028).

## Assumptions

- "Needs attention" means the existing awaiting-input activity state and nothing else.
- The notification does not say why the session waits: the activity state the application has today does not tell a finished turn, a permission prompt and an idle prompt apart, and the issue does not ask for it.
- "The session currently focused" in the issue is the session in view as defined under Terms. A regular terminal tab of the session counts as showing the session: its activity indicator is beside it in the sidebar and on its tab strip. A screen that takes the main area over, such as Settings, does not: the user cannot see the session there.
- "Produces a turn the user has not viewed" is read as: the session changed to awaiting input while not in view. Output of a session that is still working does not make it unread.
- The unread count appears in the top bar's project switcher only, as the issue says: on the rows of its panel and, as a total for the other projects, on its button.
- Desktop notifications are on by default: the feature exists to interrupt at the right moment, and one switch turns it off.
- A notification sounds or not as the operating system decides.
- Sessions keep running in the session service after the last window is closed (existing behaviour). Whether the application itself is then still running differs by operating system; FR-015a is worded for both cases.
- The issue's acceptance criteria name Linux and macOS. Windows is included because the constitution's Principle VI makes parity a condition of done.
- The application shows the notification itself, on the user's desktop. With the session service in a container, the container takes no part in showing it.
- **Dependencies**: the session activity states and their detection for Claude Code, GitHub Copilot and Pi Coding Agent; the project switcher and background projects (008-background-project-switching); session row labels (032-untitled-session-labels); the Settings screen and its stored file.
