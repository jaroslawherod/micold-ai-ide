# Feature Specification: Reopen on the session I was last using

**Feature Branch**: `feat/025-last-session-memory`

**Created**: 2026-08-11

**Status**: Draft

**Bugfix**: 2026-08-14 — [BUG-001](./bugs/BUG-001.md) added FR-014 and SC-008 (the terminal area must
not say a restored session is starting), a fourth US1 acceptance scenario, and a correction to the
assumption that output survives a restart — it does not.

**Input**: User description: "Remember which session I was on in each project across application restarts. Today the foreground session per project is remembered only while the app is running; nothing is persisted, so at launch no session is current and I land on the project overview instead of where I left off."

## Clarifications

### Session 2026-08-11

- Q: Should restoring at launch withhold keyboard focus from the session's terminal? → A: No — reversed during implementation. The spec's first answer was "no focus", by analogy with arriving somewhere you did not ask to be. Feature 023 has since made focus *derived* — a terminal holds the keyboard because a session is displayed and the user has not given it away — so withholding it at launch would require either a third writer of the released flag (a test exists to prevent exactly that) or recording that the user gave the keyboard away when they did not. It would be the single special case in a model built to remove them, and the behaviour it buys is worse: you reopen on your session and cannot type into it.
- Q: Should a session whose worktree was deleted outside the application be restored? → A: Yes, and shown as any missing-worktree session is shown. Two reasons, found while implementing: the application already lists such a session and lets the user select it, so declining to *return* them to it would repeat the inconsistency feature 008's BUG-001 was about; and declining would require the project's worktree list at resolve time, which on a project switch is discovered asynchronously and is not there yet — so the same rule would break switching to restore a case the user can see for themselves.
- Q: When should the memory be written to disk? → A: Whenever it changes value, and only then. Reports that name the session already remembered — an attach re-sending the current id, a session start for the session already in front of the user — write nothing. A force-kill therefore loses at most the single most recent change, never the whole memory, which writing only at shutdown would risk in exactly the case the feature exists for.
- Q: When the current session goes away (closed, or nothing current) the client already reports "no session" — should that clear the project's remembered session? → A: No. A "no session" report is ignored; the memory keeps naming the last session actually in front of the user. Clearing on every incidental loss of the pointer would erase the memory for reasons the user never took (a close, an internal cleanup), and the restore already refuses a session that is closed or gone (FR-005), so a stale memory is harmless where a lost one is not.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Pick up where I left off (Priority: P1)

I close the application at the end of the day with a session in front of me. When I open it again,
I want that session in front of me again — not the project overview, and not a different session
that happens to sort first.

Today the application knows which session I was on for exactly as long as it is running. Closing it
forgets, so every launch starts me at the overview of a project I have been using for weeks, and I
have to find my way back to the same row by hand.

**Why this priority**: It is the whole feature. Everything else here is about behaving sensibly when
the remembered session cannot be honoured.

**Independent Test**: Open a project, select a session, quit the application, and start it again.
The same session is in front of you, with no clicks.

**Acceptance Scenarios**:

1. **Given** I quit with a session in front of me in the project the app reopens,
   **When** the application starts,
   **Then** that session is the current one, its location is listed open in the side panel, and its
   terminal is ready to type in.
2. **Given** I quit with no session in front of me,
   **When** the application starts,
   **Then** no session is current and the project overview is shown — the application does not pick
   one for me.
3. **Given** the remembered session's process is no longer running (it stopped, or the machine was
   rebooted),
   **When** the application starts,
   **Then** that session is still the one in front of me, showing its state and whatever output was
   preserved — the same as selecting it by hand.
4. **Given** the remembered session is not running and no output survived the restart,
   **When** the application starts,
   **Then** the terminal area tells me the session is not running and how to run it — it does not
   tell me it is starting, because nothing is (BUG-001).

---

### User Story 2 - The memory is per project, and survives switching (Priority: P2)

I work across several projects. Each one should reopen on its own last session, whichever project
the application happens to start in — and switching to another project after launch should still
take me to *that* project's last session.

**Why this priority**: It is what makes the memory worth having for more than one project, but US1
is already useful alone for the project the application reopens.

**Independent Test**: Use two projects, leaving each on a different session. Quit, restart, then
switch to the other project. Each lands on its own remembered session.

**Acceptance Scenarios**:

1. **Given** two projects, each last used on a different session,
   **When** I restart and switch between them,
   **Then** each project lands on its own remembered session.
2. **Given** I switch projects several times in one run and then quit,
   **When** I restart,
   **Then** each project still remembers the session I last had in front of me *in that project*,
   not the one I was on when I quit.

---

### User Story 3 - It behaves sensibly when the session is gone (Priority: P2)

Between one run and the next, things change: I delete a worktree, close a session, or clear out a
project on disk. When the remembered session cannot be shown, I want the application to fall back
quietly rather than fail, show something misleading, or lose the rest of the project.

**Why this priority**: These are the paths where a naive memory does damage — pointing at something
that no longer exists is worse than not remembering at all.

**Acceptance Scenarios**:

1. **Given** the remembered session was closed before I quit,
   **When** the application starts,
   **Then** it is not restored (a closed session is not listed at all), and the application falls
   back exactly as it does when there is no memory.
2. **Given** the remembered session's worktree was deleted outside the application,
   **When** the application starts,
   **Then** that session is still restored, shown as any session with a missing worktree is shown —
   the same as selecting it by hand — and nothing else about the project is disturbed.
3. **Given** the remembered session's record is gone entirely,
   **When** the application starts,
   **Then** nothing is restored from it, and the next session I make current in that project
   replaces the memory.

---

### Edge Cases

- **The session that was current is closed, and nothing replaces it.** The memory still names it.
  Nothing is restored from it on the next launch (it is closed), but the memory is not erased by the
  closing itself — only by another session becoming current (FR-005a).
- **The remembered session belongs to a project that is no longer open.** The memory for a forgotten
  project goes with the project; it must not resurface if the same folder is opened again later.
- **The application is force-killed rather than closed cleanly.** The memory is only as fresh as the
  last time it was written; the application must start on whatever was last recorded rather than
  refuse to start or show an error.
- **Two windows of the application are open at once.** Whichever wrote last wins; the memory must
  not be corrupted by both writing, and neither window may be prevented from starting by the other.
- **The remembered session exists but its project folder is unavailable** (an unmounted drive):
  nothing is restored for it, consistent with the application already refusing to activate an
  unavailable project. Distinct from a missing *worktree*, which is restored and shown as missing —
  there the project is present and the user can see and select the session themselves.
- **The stored memory is unreadable or from an older version of the application.** It is treated as
  "no memory" and the application starts normally — a launch must never fail because of it.
- **A session was started, used, and closed within one run, and nothing else selected.** The memory
  still names it — closing does not erase it (FR-005a) — and the next launch restores nothing from
  it, because a closed session cannot be restored (FR-005). The user lands on the overview, which is
  where they were when they quit.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: The application MUST remember, per project, which session was last in front of the
  user, and MUST retain that memory across restarts.
- **FR-002**: On starting, the application MUST make the remembered session of the project it opens
  the current one, so the user resumes where they left off.
- **FR-003**: The remembered session MUST be restored whether or not its process is still running,
  matching what already happens when switching projects within a run, and matching what selecting
  the session by hand does.
- **FR-004**: Restoring a session MUST NOT start, resume, or otherwise change any process. It
  selects and displays; it does not run anything.
- **FR-005**: The application MUST NOT restore a session that has been closed, or whose record no
  longer exists. In those cases it MUST fall back to the same behaviour as a project with no
  memory.
- **FR-001a**: The memory MUST be written durably whenever it changes, and MUST NOT be written when
  a report names the session already remembered. Losing power or force-killing the application MUST
  cost at most the single most recent change, never the whole memory.
- **FR-005a**: A project's memory MUST only ever be replaced by another session becoming current in
  that project. It MUST NOT be erased by the current session merely going away — closing it, or any
  internal loss of the pointer. A memory naming a session that can no longer be restored is
  harmless (FR-005 declines it); a memory erased for a reason the user never took is not, because it
  silently costs them the place they would have returned to.
- **FR-006**: When the remembered session cannot be restored, the application MUST leave the rest of
  the project untouched — its other sessions, its locations, and its open/closed state.
- **FR-007**: When no session can be restored for the project being opened, the application MUST
  show no session as current, rather than choosing one on the user's behalf.
- **FR-008**: The memory MUST be per project, and switching to a project after launch MUST use that
  project's own memory.
- **FR-009**: Forgetting a project MUST discard its memory, so re-opening the same folder later
  starts without one.
- **FR-010**: Unreadable, missing, or outdated stored memory MUST be treated as no memory. Starting
  the application MUST NOT fail, warn, or block on it.
- **FR-011**: The memory MUST be stored on the user's own device alongside the application's other
  state, and MUST NOT be transmitted anywhere.
- **FR-012**: The restored session MUST be presented exactly as a session made current by any other
  means — including being revealed in the side panel — so the user cannot tell from the panel how
  they arrived at it.
- **FR-013**: The restored session MUST be ready to type in, exactly as a session reached by any
  other navigation is. Reopening the application is not a special case: the user is looking at the
  session they left, and typing belongs to it.
- **FR-014**: When the restored session has no output to show, the terminal area MUST describe the
  session's actual state, and MUST NOT say that it is starting. FR-004 forbids the launch from
  starting anything, so a launch that says "starting" is telling the user to wait for an event that
  will never arrive. The wording MUST distinguish a session that is genuinely being launched from
  one that is merely not running, and MUST agree with the state shown elsewhere for the same session
  (BUG-001).

### Key Entities

- **Last-used session**: for one project, the session that was most recently in front of the user.
  At most one per project; absent for a project the user has not yet used a session in.
- **Project**: the folder the application works in. Owns its own last-used session, independent of
  every other project's.
- **Session**: an interactive run inside a project, which may be running or stopped, and which may
  be closed (after which it is no longer listed and cannot be restored).

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: After quitting with a session in front of them and restarting, the user is on that
  same session with zero clicks.
- **SC-002**: The restored session is the one they left, not merely *a* session, in every case where
  it still exists and was not closed.
- **SC-003**: Across two projects last used on different sessions, restarting and visiting both
  lands on the correct session for each, 100% of the time.
- **SC-004**: No launch fails, stalls, or shows an error because of the stored memory — including
  when it is absent, unreadable, or refers to sessions that no longer exist.
- **SC-005**: Restoring a session starts no process: the number of running sessions immediately
  after launch is the same as it would be without this feature.
- **SC-006**: A user who has never used a session in a project sees no change in behaviour.
- **SC-007**: Force-killing the application costs at most the most recently made choice: the session
  in front of the user at the moment of the kill is restored on the next launch, provided it had
  been current long enough for the change to be recorded.
- **SC-008**: No launch describes a session as starting when nothing is starting. Every statement
  the launch screen makes about the restored session's state agrees with every other statement made
  about it on the same screen.

## Assumptions

- "The session I was last using" means the session the application considered current — the same
  notion the side panel marks and the project switcher restores today. This feature persists that
  choice; it does not change how the choice is made.
- The project the application opens at launch is already decided by existing behaviour (the last
  active project). This feature does not change which project opens, only which session is in front
  of the user once it has.
- Restoring is display only. A stopped session shows its state and whatever output was preserved,
  exactly as it does when selected by hand; making it run again remains an explicit action.
  **Corrected 2026-08-14 (BUG-001)**: across a restart, *no* output is preserved — terminal output
  lives only in the client's memory and is rebuilt from frames the daemon streams for a running
  process. So the ordinary case is a restored session with nothing to show, and what the terminal
  area says in that case is a decision this feature has to make rather than inherit (FR-014).
- Sessions' run state is not remembered across restarts and this feature does not change that — so
  the common case at launch is restoring a session that is not running.
- Nothing about the side panel's own open/closed rows is persisted by this feature; the panel's
  behaviour on a restored session follows from that session becoming current, as it already does.
