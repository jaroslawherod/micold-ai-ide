# Feature Specification: Terminal History That Survives a Session Service Restart

**Feature Branch**: `feat/terminal-scrollback-persistence`

**Created**: 2026-10-02

**Status**: Draft

**Input**: User description: "Implement GitHub issue #485: Keep terminal scrollback across daemon restarts and reboots. Sessions survive the window closing because the daemon owns them, but a daemon restart, an update or a reboot loses all terminal history. The user cannot scroll back to see what an agent did before the restart. Proposal: the daemon periodically writes each session's scrollback (up to the configured scrollback limit) to its data directory, and on the session's exit. After a daemon restart, a restored or restarted session shows the saved history above a clear separator line (\"session restarted at …\"), before any new output. The saved history is deleted when the session is deleted. A setting turns persistence off for users who do not want terminal output written to disk. Acceptance criteria: scrollback written before a daemon restart is visible after it, including colours and styles. Writes are batched so that a busy terminal does not cause constant disk I/O. Files are written with user-only permissions, and work in the sandboxed runtime's mounted data directory. A corrupt or unreadable saved file is skipped and reported, never blocks the session from starting."

## Terms

- **Session service**: the background service that owns sessions and their terminals (the daemon). It
  keeps running while the app's windows are closed.
- **Service restart**: any event after which a new session service process is running: the user
  restarts it, an update replaces it, the user logs out and in, the computer reboots, the service
  stops itself after 30 minutes with no window connected and is started by the next window, or the
  service crashes and is started again. All but a crash and a power loss are *orderly* stops.
- **Terminal**: in this spec, a session's AI CLI terminal. A session's Regular Terminal instances are
  covered only by FR-014.
- **Removing a session**: any action after which the session is never shown again: **Close**,
  **Remove**, deleting its worktree, forgetting its project, and the app discarding a session that
  was never used.
- **Terminal history**: the output of a session's terminal that the user can scroll back to, plus
  what is on the terminal's screen, with its colours and text styles.
- **Saved history**: the copy of a terminal history that the session service keeps on disk.
- **Scrollback limit**: the existing *Scrollback lines* setting (default 10,000 lines).

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Scroll back to what happened before the service restarted (Priority: P1)

A user leaves an AI session working, and the session service restarts: an update is installed, the
computer reboots, or the service is restarted by hand. When the user comes back and starts the
session again, the terminal is not empty. Scrolling up shows the output from before the restart, in
the same colours and styles, then one clear line saying the session restarted and when, then the
new output.

**Why this priority**: It is the whole request. Today the session comes back after a restart but its
terminal is blank, so the user cannot see what the agent did or where it stopped.

**Independent Test**: Run a session, print 200 numbered lines of which some are coloured and bold,
restart the session service, start the session again, and confirm that scrolling up shows all 200
lines with their colours and styles above a "session restarted at …" line, and the new output below
it.

**Acceptance Scenarios**:

1. **Given** a session whose terminal printed 200 lines, some coloured and some bold, **When** the
   session service is restarted in an orderly way and the session is started again, **Then** all 200
   lines can be scrolled back to, in their original order, colours and styles.
2. **Given** a session with saved history, **When** it is started after a service restart, **Then**
   one separator line reading "session restarted at" followed by the local date and time of that
   start is shown below the saved history and above every line of new output.
3. **Given** a session that produced no terminal output before the restart, **When** it is started
   after the restart, **Then** the terminal shows no separator line and no blank history.
4. **Given** a session whose history already holds one separator from an earlier restart, **When**
   the service restarts again and the session is started, **Then** the history shows the earlier
   output, the earlier separator, the output between the two restarts and a second separator, in
   that order.
5. **Given** a busy session whose history is longer than the scrollback limit, **When** the service
   restarts and the session is started, **Then** the most recent lines up to the limit are shown and
   older lines are absent, as they would be without a restart.
6. **Given** two sessions that printed different output, **When** the service restarts and both are
   started, **Then** each terminal shows only its own history.
7. **Given** the session service is killed without warning (crash, power loss) while a session is
   printing, **When** it is started again and the session is started, **Then** the history is
   present up to a point no more than 60 seconds before the kill.
8. **Given** a running session and every window closed, **When** the session service stops itself
   after 30 minutes with no window connected, the user reopens the app and opens the session,
   **Then** the whole history from before the stop is shown above the separator, with nothing
   missing.

---

### User Story 2 - Keep terminal output off the disk (Priority: P2)

A user who works with secrets in the terminal does not want terminal output written to disk. They
open Settings, turn terminal history saving off, and from then on the session service writes no
terminal output to disk. After a restart their sessions come back with an empty terminal, as they
do today.

**Why this priority**: Terminal output can hold tokens, passwords and private code. Saving it is new
behaviour that writes such data to disk, so the user must be able to refuse it before the feature
is acceptable to ship by default.

**Independent Test**: Turn the setting off, run a session that prints a recognisable marker text,
restart the session service, and confirm that the marker is nowhere in the service's data location
and that the session's terminal starts empty with no separator line.

**Acceptance Scenarios**:

1. **Given** a fresh installation, **When** the user opens Settings, **Then** the Terminal section
   shows a control for saving terminal history, turned on, with a sentence saying that terminal
   output is written to this computer's disk while it is on.
2. **Given** the setting is off, **When** a session prints output and the service is restarted,
   **Then** no saved history for that session exists on disk and the session's terminal starts empty
   with no separator line.
3. **Given** the setting is on and a session is running, **When** the user turns it off and saves,
   **Then** no further terminal output of any session is written to disk, without restarting the
   session or the service.
4. **Given** the setting is off and a session is running, **When** the user turns it on and saves,
   **Then** that session's history, including output printed while the setting was off and still in
   the terminal, is saved within 60 seconds, without restarting the session.
5. **Given** the user turns the setting off, **When** saved histories from before already exist,
   **Then** [NEEDS CLARIFICATION: are the already-saved histories deleted at once when the setting is
   turned off, or kept until their sessions are removed? Deleting matches the user's wish to have no
   terminal output on disk but destroys history they may still want; keeping leaves old output on
   disk after the user said no.]

---

### User Story 3 - A damaged saved history never stops a session (Priority: P2)

A saved history is damaged: the disk filled up half-way, the file was edited, or it was written by
an app version this one cannot read. The user starts the session after a restart. The session starts
as it always does. Its terminal shows one line saying the earlier output could not be restored, and
the reason is in the service's log.

**Why this priority**: Saved history is a convenience. A session that cannot start because of it
would make the feature worse than not having it.

**Independent Test**: Save a session's history, restart the service after overwriting the saved
history with random bytes, start the session, and confirm that it starts and runs, that its terminal
shows the "could not be restored" line and no garbage, and that the service's log names the session
and the reason.

**Acceptance Scenarios**:

1. **Given** a session whose saved history is not readable as a saved history, **When** the session
   is started after a service restart, **Then** the session starts and runs exactly as a session with
   no saved history does.
2. **Given** the same session, **When** its terminal is shown, **Then** it shows one line saying that
   the earlier output could not be restored, shows none of the damaged content, and the service's log
   holds one warning naming the session and the reason, which also appears among the recent errors
   of *Session service diagnostics*.
3. **Given** a saved history the service has no permission to read, **When** the session is started,
   **Then** the outcome is the same as for a damaged one.
4. **Given** one session with a damaged saved history and another with an intact one, **When** both
   are started after a restart, **Then** the second shows its full history.
5. **Given** a session whose damaged saved history was skipped, **When** the session keeps running,
   **Then** its new output is saved as for any other session, and the next restart restores it.
6. **Given** the disk is full or the data location is not writable, **When** a save fails, **Then**
   the session keeps running without interruption, the failure is written to the service's log as a
   warning, and the save is tried again 30 seconds later.

---

### User Story 4 - Saved history goes away with its session (Priority: P3)

A user closes or removes a session, deletes its worktree or forgets its project. The session will
never be shown again, and nothing of its terminal output stays on disk afterwards.

**Why this priority**: Without it, sessions that are gone would leave private output behind and the
data location would grow without bound. It only matters once histories are being saved.

**Independent Test**: Run a session that prints a recognisable marker text, wait for the history to
be saved, remove the session, and confirm that the marker is nowhere in the service's data location.
Repeat with Close.

**Acceptance Scenarios**:

1. **Given** a session with saved history, **When** the user chooses **Remove** and confirms,
   **Then** its saved history is removed from disk before the removal is reported as done.
2. **Given** a session with saved history, **When** the user chooses **Close**, **Then** its saved
   history is removed from disk before the session disappears from the sidebar.
3. **Given** a worktree or a project with sessions that have saved history, **When** the worktree is
   deleted or the project is forgotten, **Then** the saved history of each of those sessions is
   removed.
4. **Given** a saved history that belongs to no session that can still be shown (a removal was
   interrupted, or the session was closed), **When** the session service starts, **Then** that saved
   history is removed.
5. **Given** a session is removed while a save of its history is in progress, **When** the removal
   completes, **Then** no saved history for it exists on disk afterwards.
6. **Given** a session with saved history, **When** the user stops it, **Then** its saved history is
   kept and is shown when the session is started again after a service restart.

---

### Edge Cases

- **No history**: a session that never printed anything, or that was created but never started, has
  nothing to restore; no separator is shown and no error is reported.
- **Idle terminal**: a terminal with no new output since its last save causes no write.
- **Several busy sessions at once**: each is saved on its own schedule; a save of one never delays
  the output of that session or of another, and no session ever shows another session's history.
- **Regular Terminal instances**: whether their history is saved is open (FR-014); until it is
  decided, nothing in this spec writes their output to disk.
- **Several windows**: every window showing the same session shows the same restored history and the
  same separator.
- **Service killed in the middle of a save**: the previous complete saved history stays usable; a
  half-written one never replaces it.
- **Scrollback limit changed between save and restore**: the most recent lines up to the limit now
  in force are restored.
- **Terminal size changed between save and restore**: restored lines are laid out as live history is
  when the terminal is resized.
- **Full-screen programs**: output of a program that takes over the whole screen and leaves nothing
  to scroll back to is restored only as the last screen it showed.
- **A session that resumes a conversation**: the AI CLI redraws its own view below the separator; the
  saved history above it is unchanged.
- **Session removed while the setting is off**: saved history left from before is still removed.
- **Where the service runs**: history saved by a service running directly on the computer is restored
  by a service running in a container, and the reverse, because both use the same data location.
- **Platform differences**: "only the user can read it" is enforced by each platform's own means on
  Linux, macOS and Windows; the behaviour the user sees is the same on all three. On Windows the
  saved histories are in the local profile, never in the roaming profile, so they are not
  synchronised to other computers.
- **Clock changes**: the separator shows the local time at the moment of the start; a later change of
  the time zone does not rewrite earlier separators.
- **Saved history from a newer or older app version that cannot be read**: handled as a damaged one
  (User Story 3).

## Requirements *(mandatory)*

### Functional Requirements

**Saving**

- **FR-001**: While saving is on, the session service MUST keep a saved history for the AI CLI
  terminal of every session, holding the terminal history up to the scrollback limit in force for
  that terminal, with the text, colours (the 16 basic, the 256 indexed and full-colour values) and
  text styles (bold, dim, italic, underline, inverse, strikethrough) the terminal showed.
- **FR-002**: A terminal's history MUST be saved when its process exits and when the session service
  stops in an orderly way, so that after an orderly service restart no output is missing.
- **FR-003**: While a terminal's process is running, two saves of that terminal MUST be at least 30
  seconds apart, however much output it produces, and output MUST be saved no later than 60 seconds
  after it was printed. The saves of FR-002 are in addition to these.
- **FR-004**: A terminal with no new output since its last save MUST NOT cause a write.
- **FR-005**: Saving MUST NOT drop terminal output, input or a resize of any session, and MUST NOT
  delay them beyond the bound of SC-005.
- **FR-006**: A save that is interrupted MUST leave the previous complete saved history readable.
- **FR-007**: A save that fails MUST leave the session running, MUST be written to the service's log
  as a warning with the session and the reason, and MUST be tried again at the next save; a repeated
  failure for the same session and reason MUST NOT be logged more than once per service run.

**Restoring**

- **FR-008**: When a session that has saved history is opened or started for the first time after a
  service restart (opening an interrupted session resumes it; a session the user had stopped is
  started by the user), its terminal MUST show the saved history, then one separator line, then the
  new output, and the saved history MUST be reachable by scrolling exactly as live history is.
- **FR-009**: The separator MUST read "session restarted at" followed by the local date and time of
  the start, MUST be visually distinct from program output, MUST occupy one line, and MUST NOT be
  sent to the session's process as input.
- **FR-010**: A terminal with no saved history MUST show no separator.
- **FR-011**: Restored history and separators MUST become part of the terminal's history: they count
  against the scrollback limit, are saved again with later output, and remain after further restarts
  until newer output pushes them past the limit.
- **FR-012**: Restored history MUST NOT exceed the scrollback limit in force when it is restored;
  when it is longer, the most recent lines are kept.
- **FR-013**: Restoring a history of 10,000 lines MUST NOT delay the start of its session by more
  than 1 second.
- **FR-014**: Whether a session's Regular Terminal instances are covered: [NEEDS CLARIFICATION: is
  the history of Regular Terminal (shell) instances saved and restored too, or only the AI CLI
  terminal's? Today the instances are not brought back after a service restart, so covering them
  means restoring the instances as well, and removing an instance's saved history when the instance
  is closed; leaving them out means that shell output is never written to disk and is lost on a
  restart as it is today.]
- **FR-015**: Whether a session that is stopped and started again while the session service keeps
  running shows its earlier output: [NEEDS CLARIFICATION: does the saved history and separator also
  apply when a session is stopped, or its process exits, and it is started again while the same
  session service keeps running, or only after a service restart as the issue describes? Applying
  it everywhere makes every start behave the same; limiting it leaves today's behaviour within one
  service run unchanged.]

**Damaged or unreadable saved history**

- **FR-016**: A saved history that cannot be read, for any reason, MUST be skipped: the session MUST
  start and run as one with no saved history, and none of the unreadable content may be shown.
- **FR-017**: When a saved history is skipped, the session's terminal MUST show one line saying that
  earlier output could not be restored, and the service's log MUST hold one warning naming the
  session and the reason, which MUST also appear among the recent errors of *Session service
  diagnostics*.
- **FR-018**: A skipped saved history MUST NOT prevent saving that terminal's later output, and MUST
  NOT affect any other session.

**Privacy and removal**

- **FR-019**: Saved histories MUST be stored only in the session service's local data location on
  the user's computer, MUST NOT be sent anywhere else, and MUST NOT be placed in a location the
  platform synchronises to other computers (on Windows, the roaming profile).
- **FR-020**: Saved histories, and any directory created to hold them, MUST be readable and writable
  only by the user who runs the session service, from the moment they are created, on Linux, macOS
  and Windows.
- **FR-021**: When the session service runs in a container, saved histories MUST be written to the
  data location mounted from the user's computer, MUST be readable there by the user and by nobody
  else, and MUST be restored after the container is recreated.
- **FR-022**: A saved history written by a service running directly on the computer MUST be restored
  by a service running in a container that uses the same data location, and the reverse.
- **FR-023**: Removing a session (Close, Remove, deleting its worktree, forgetting its project, or
  the app discarding a session that was never used) MUST remove its saved histories before the
  action is reported as done, including while saving is off.
- **FR-024**: On start, the session service MUST remove every saved history that belongs to no
  session that can still be shown: one it does not know, or one that was closed or removed.
- **FR-025**: A session MUST never be shown the saved history of another session.

**The setting**

- **FR-026**: Settings MUST offer, in the Terminal section, one control that turns saving of terminal
  history on or off for all sessions, with a sentence stating that terminal output is written to
  this computer's disk while it is on. It MUST be on by default.
- **FR-027**: A saved change of the setting MUST take effect for running sessions without restarting
  them or the session service: turned off, no terminal output is written from then on; turned on,
  each running terminal's history is saved within the next 60 seconds.
- **FR-028**: While the setting is off, the session service MUST NOT write terminal output to disk,
  and a session started after a service restart MUST show no saved history and no separator.
- **FR-029**: The setting MUST be stored with the other settings on the user's computer and MUST keep
  its value across restarts and upgrades.

**Consistency, platforms and documentation**

- **FR-030**: Every behaviour above MUST be the same on Linux, macOS and Windows.
- **FR-031**: The setting's control MUST be built from the app's shared settings components and look
  and behave like the other controls in the Terminal section.
- **FR-032**: The user guide MUST describe, in the same change that ships each behaviour: that
  terminal history survives a service restart and what the separator means, the setting and what
  turning it off does, where saved histories are stored and who can read them, and what the user
  sees when a saved history could not be restored.

### Key Entities

- **Saved history**: the on-disk copy of one terminal's history. Belongs to exactly one terminal of
  one session; holds the lines up to the scrollback limit with their colours and styles, and the
  separators of earlier restarts; lives until its session is removed.
- **Restart separator**: one line in a terminal's history that marks where a restart happened,
  carrying the local date and time of the start that followed it.
- **History saving setting**: one on/off value, global to the installation, on by default.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: After an orderly service restart, 100% of the lines a terminal held before the restart,
  up to the scrollback limit, can be scrolled back to, with the same text, colours and styles.
- **SC-002**: After the service is killed without warning, a terminal's restored history is missing
  at most the last 60 seconds of output before the kill.
- **SC-003**: A terminal that prints continuously for 10 minutes causes at most 21 writes of its
  history to disk, and a terminal that prints nothing for 10 minutes causes none.
- **SC-004**: A session with 10,000 lines of saved history is ready for input no more than 1 second
  later than the same session with none.
- **SC-005**: With ten sessions printing continuously, the time from a keystroke to its echo in any
  one of them is, at the 95th percentile, at most 20 ms longer than with saving turned off.
- **SC-006**: In 100% of cases, a session whose saved history is damaged, unreadable or from another
  app version starts and runs, and the user sees one line saying so.
- **SC-007**: After a session is closed or removed, a search of the service's data location for text
  that session printed finds nothing.
- **SC-008**: With the setting off, a search of the service's data location for text printed after
  it was turned off finds nothing.
- **SC-009**: No account on the computer other than the user's can read a saved history, on each of
  Linux, macOS and Windows, and when the service runs in a container.
- **SC-010**: A user returning after a restart can tell, from the terminal alone and without
  consulting the guide, which output is from before the restart and when the restart happened.

## Assumptions

- **Saving is on by default.** The issue describes a setting that "turns persistence off", so the
  feature ships on, and User Story 2 gives the opt-out.
- **One global setting.** There is no per-project or per-session choice in this feature.
- **The 30-second spacing between saves is fixed**, not a setting. It bounds disk writes on a busy
  terminal, and with it the output lost when the service is killed without warning is at most the
  last 60 seconds.
- **The AI CLI terminal is what the issue is about** ("see what an agent did before the restart").
  Whether Regular Terminal instances are covered is open (FR-014).
- **What is saved is what the user could scroll back to**, plus the last screen. Output a full-screen
  program drew and then cleared is not recoverable, as it is not today.
- **Clickable links, images and cursor position are not part of the saved history.** Text, colours
  and text styles are.
- **The separator uses the computer's local time and the app's usual date format.**
- **Removing a session** means the actions listed under Terms. Stopping a session does not remove
  its saved history.
- **Reporting means a warning in the service's log plus one line in the affected terminal.** The
  warning also appears among the recent errors of *Session service diagnostics*, as every warning of
  the service does. This feature adds no new notification.
- **Saved histories are not encrypted.** They are protected by file permissions, like the other files
  in the data location. A user who needs more turns saving off.
- **Saved histories live in the session service's local data location**: the per-user directory on
  this computer that already holds the service's log and that a containerised service receives as a
  mount from the user's computer. On Windows this is the local profile, not the roaming one, which
  can be synchronised to other computers.
- **Closing and reopening the app's windows is unchanged while the session service keeps running**:
  it keeps the terminal history in memory then, as it does today. Once it has stopped itself after
  30 minutes with no window connected, reopening the app is a service restart (User Story 1,
  scenario 8).
- **History is shown when the session is opened or started**, not before. Opening a session that was
  interrupted by the service restart resumes it, and that is when its history and separator appear.
  A session the user had stopped before the restart shows its history when the user starts it.

### Out of scope

- Restoring the running processes themselves across a service restart.
- Searching, exporting or viewing saved history outside the session's terminal.
- Encrypting saved history, or redacting secrets from it.
- Copying saved history to another computer or to any online service.
- A per-project or per-session choice, and a configurable save interval.
- Keeping the history of a session that was closed or removed.
