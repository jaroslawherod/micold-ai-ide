# Feature Specification: A session keeps its name when nothing is running it

**Feature Branch**: `feat/the-name-of-session-should-be-always-visible-even-when-not-loaded`

**Created**: 2026-09-12

**Status**: Closed 2026-09-14 — implemented and shipped; all 30 tasks in [tasks.md](./tasks.md) are done

**Bugfix**: 2026-09-26 — [BUG-002](./bugs/BUG-002.md) FR-005 and FR-006 now say *which* of the AI
CLI's several name records is the one to take: the name that CLI currently holds for the
conversation. `claude` keeps re-emitting the pre-rename `ai-title` after a user's `/rename`, so
"the latest `ai-title`" was not the latest name.

**Input**: User description: "the name of session should be always visible when it was set. Currently after daemon restart when session is not yet active it's shown as `New Session` but it already has name that was assigned once and should be persistent and remembered"

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Find my session again after a restart (Priority: P1)

I have several sessions in a project, each one named after what it is about. The background service
that owns those sessions restarts — because the machine rebooted, because the application was
updated, or because I stopped and started it myself. When the session list comes back, every session
that has not been reopened yet reads "New session", so the list tells me nothing: four rows, four
identical labels, and the only way to tell them apart is to open each one and wait for it to come
back to life.

I want the name a session already has to stay on its row whether or not anything is running it. The
name is what makes the list usable; losing it until I open a session defeats the purpose of having a
list at all.

**Why this priority**: It is the whole complaint. A session list where the names disappear on every
restart is a session list the user cannot navigate.

**Independent Test**: Name a session (let the AI CLI title it), stop and restart the background
service, and look at the session list without opening anything. The name is on the row.

**Acceptance Scenarios**:

1. **Given** a session whose name has been shown to me at least once,
   **When** the background service restarts and I view the project without opening that session,
   **Then** the row shows the same name it showed before the restart.
2. **Given** several named sessions in one project,
   **When** the background service restarts,
   **Then** each row shows its own name, and no two rows are distinguishable only by position.
3. **Given** a named session shown by name after a restart,
   **When** I open it and it comes back to life,
   **Then** the name does not flicker, blank out, or change to "New session" at any point.
4. **Given** I close the whole application and open it again,
   **When** I view a project's sessions,
   **Then** the names are present before I open any session.

---

### User Story 2 - My existing sessions get their names back (Priority: P2)

I have been using the application for months. Every session I have not touched since the last
restart is already showing "New session", even though each of them has a real name recorded by the
AI CLI that ran it. I do not want to have to open all of them once to repair the list, and I
certainly do not want the sessions I created before this change to be the ones that stay broken.

**Why this priority**: Without it the fix only helps sessions named from this point on, and the user
who reported the problem still sees a list of "New session" rows for everything they already have.

**Independent Test**: Take a session the application has no recorded name for but whose AI CLI
records do contain one, view the project without opening it, and see the name on the row.

**Acceptance Scenarios**:

1. **Given** a session the application has never recorded a name for, but whose AI CLI has a name
   for that conversation,
   **When** I view the project without opening the session,
   **Then** the row shows the name from the AI CLI's own records.
2. **Given** that same session,
   **When** the background service restarts again,
   **Then** the name is still shown, without the AI CLI's records having to be consulted a second
   time.

---

### User Story 3 - The name stays true to the conversation (Priority: P3)

A session's name comes from the conversation inside it and changes as that conversation develops.
Remembering the name must not mean freezing it: when the conversation is re-titled, the row follows,
and a session that genuinely has no name yet still says so rather than borrowing someone else's.

**Why this priority**: It protects against the obvious way a fix for this bug goes wrong — a stale
name pinned to a row forever is worse than a blank one, because it is confidently wrong.

**Independent Test**: Rename a running session's conversation, confirm the row follows, restart the
background service, and confirm the row shows the new name rather than the old one.

**Acceptance Scenarios**:

1. **Given** a named session that is running,
   **When** its conversation is re-titled,
   **Then** the row shows the new name, and the new name is the one that survives the next restart.
2. **Given** a session that has never been named,
   **When** I view it before and after a restart,
   **Then** it reads "New session" both times.
3. **Given** two sessions in the same location, one named and one not,
   **When** the background service restarts,
   **Then** the named one keeps its name and the unnamed one keeps the placeholder — neither
   inherits the other's label.

---

### Edge Cases

- **A name is recorded, then the AI CLI's own records for that conversation are deleted.** The
  remembered name stays on the row; losing the transcript must not un-name the session.
- **The application is stopped between a name being shown and it being written down.** At most the
  single most recent name change is lost, and the next time the session is running the name is
  re-observed and recorded again.
- **A name arrives while the session's project state cannot be written** (read-only directory, disk
  full). The name is still shown for as long as the service runs; the failure to persist does not
  blank the row, and it is not reported to the user as a session error.
- **The same session is looked at from two application windows at once.** Both show the same name;
  neither window's view resets the other's.
- **A session is archived or its worktree is deleted.** Wherever such a session is still listed, it
  is listed under its name, on the same rule as any other session.
- **An extremely long or single-character name, or one containing punctuation and non-Latin
  characters.** It is stored and shown exactly as the AI CLI recorded it; any shortening is a
  display concern of the row, unchanged by this feature.
- **A session that has a name in the AI CLI's records but was started outside this application.**
  It is treated like any other session with a recoverable name (US2).
- **The AI CLI's records hold two names for one conversation** — the name it generated, and a name
  the user set by hand there afterwards, which it keeps beside the generated one rather than
  replacing it. The row reads the name the CLI currently uses, which is the user's; the generated one
  it superseded is never recorded or shown, however many times the CLI re-writes it (FR-005,
  BUG-002).

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: The application MUST remember a session's name durably, so that the name shown for a
  session does not depend on anything currently running that session.
- **FR-002**: The application MUST show a session's remembered name in every place a session is
  listed or identified, including when the session is idle, not yet opened, or not yet restored
  after a restart.
- **FR-003**: The application MUST record a session's name whenever the name it shows for that
  session changes, so that the record is never more than one change behind what the user last saw.
- **FR-004**: The application MUST NOT record or display a name for a session that has never had
  one; such a session MUST continue to show the neutral "New session" placeholder.
- **FR-005**: When a session's conversation is re-titled, the application MUST replace the
  remembered name with the new one — the remembered name is the latest known name, never the first.
  When the AI CLI's records hold **more than one** name for the conversation, the latest known name is
  the one that CLI itself currently uses for it, and a name the user set by hand in the CLI is that
  name for as long as the CLI keeps it (BUG-002).
- **FR-006**: For a session the application has no remembered name for, the application MUST
  recover the name from the AI CLI's own records for that conversation when one is available there,
  without requiring the user to open the session. The name recovered MUST be the one FR-005 calls the
  latest, never an earlier name the CLI still keeps beside it (BUG-002).
- **FR-007**: A name recovered under FR-006 MUST be recorded like any other, so that the recovery is
  performed once rather than repeated on every restart.
- **FR-008**: The application MUST keep a remembered name after the AI CLI's own records for that
  conversation become unavailable or are removed.
- **FR-009**: Failing to write a session's name durably MUST NOT change what is displayed, MUST NOT
  interrupt the session, and MUST NOT be surfaced to the user as a session failure.
- **FR-010**: Sessions created before this feature exists MUST get their names back under FR-006;
  the change MUST NOT require the user to re-create, re-open, or rename anything.
- **FR-011**: A session's name MUST remain derived from its conversation and MUST NOT become
  user-editable as a consequence of this feature. Taking a name the user set inside the AI CLI is
  still deriving it from the conversation — the name comes from that conversation's own records, and
  no part of this application gains a way to edit it (BUG-002).
- **FR-012**: Each session's remembered name MUST belong to that session alone — no session may
  display, inherit, or overwrite another session's name, including sessions sharing a location or a
  project.

### Key Entities

- **Session**: A unit of work in a project, identified independently of whether anything is running
  it. Carries a location, the AI CLI it runs, and — for this feature — a name.
- **Session name**: The human-readable label for a session, derived from the conversation inside it.
  Either absent (shown as the "New session" placeholder) or a recorded piece of text. Changes over
  the session's life; the latest is the one that counts.
- **AI CLI records**: The conversation records the AI CLI keeps for itself, outside the
  application's own storage. A secondary source a name can be recovered from, and one the
  application must not depend on for a name it has already recorded. They may hold several names for
  one conversation, of which exactly one is the name that CLI currently uses; that is the one this
  feature reads (FR-005, BUG-002).

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: After the background service restarts, 100% of sessions that had a name before the
  restart show that same name, with no session opened.
- **SC-002**: A user identifies the session they want from the list after a restart without opening
  any other session first.
- **SC-003**: A named session's row shows its name as soon as the session list appears — within the
  same view, with no intermediate "New session" state visible.
- **SC-004**: Of the sessions that existed before this change and have a name in the AI CLI's own
  records, 100% display that name after one restart, with no user action.
- **SC-005**: A conversation re-titled while running shows the new name after the next restart in
  100% of cases; the previous name is never shown again.
- **SC-006**: Zero sessions display a name belonging to a different session, and zero unnamed
  sessions display a name.
- **SC-007**: Making a session's name durable adds no perceptible delay — the session list appears
  as quickly after this change as before it, for a project with 50 sessions.

## Assumptions

- "Daemon restart" in the report means the background service that owns sessions restarting, and the
  same requirement applies when the whole application is closed and reopened; both are covered.
- ~~The name already has a single source — the AI CLI's title for the conversation~~ — and this
  feature changes when and where that name is remembered, not where it comes from. Naming a session
  by hand *in this application* stays out of scope (FR-011).

  Superseded twice, and both times the second half held: by
  [032-untitled-session-labels](../032-untitled-session-labels/spec.md), which adds a lower-priority
  derived label; and by [BUG-002](./bugs/BUG-002.md), which found the CLI's own records holding
  several names for one conversation, of which the latest is the one to read (FR-005). Where the name
  comes from is still the conversation, and still not this application's UI.
- The existing per-project storage is where a session's name belongs; this feature adds no new
  storage location and no separate cache the user has to manage.
- Recovering a name from the AI CLI's records is best-effort: a missing, unreadable, or unparsable
  record yields no name and leaves the placeholder, exactly as it does today.
- Names are shown as recorded; truncating a long name to fit a row is existing display behaviour and
  is unchanged.
- Sessions of every supported AI CLI are covered by the same rule — nothing here is specific to one
  of them.
