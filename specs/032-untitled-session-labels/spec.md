# Feature Specification: A session the AI CLI never titled still gets a label

**Feature Branch**: `fix/the-name-of-past-session-is-still-not-shown`

**Created**: 2026-09-19

**Status**: Draft

**Input**: User description: "bug: see the screen show the name of past session is still not displayed
even when the bug was reported and fixed". Traced in
[029 BUG-001](../029-persistent-session-names/bugs/BUG-001.md): four of the five `claude` sessions in
the report have real conversations with prompts the user typed, but `claude` never recorded a title
(`ai-title`) for any of them. Feature 029 remembers and recovers every title that exists, and by its
FR-004 a session with no title keeps the "New session" placeholder for good. Showing a name for these
sessions needs a label source that 029 never intended, so it is specified here rather than patched
into 029 (BUG-001 *Size decision*).

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Tell my past untitled sessions apart (Priority: P1)

I have several sessions in a project. I talked to the assistant in each of them: I typed the task,
answered its questions, told it to continue. For some of them the AI CLI never produced a title, so
after a restart the sidebar shows a column of rows that all read "New session". The fix for
remembering names (029) did nothing for them, because there was never a name to remember. I cannot
tell which row is which without opening each one.

I want every session that has a conversation in it to show something from that conversation on its
row, so the list is readable without opening anything.

**Why this priority**: It is the reported bug. Four of the five rows in the report read "New session"
although each holds a conversation with between 5 and 16 prompts the user typed (BUG-001
*Reproduction*).

**Independent Test**: Take a project containing a session whose AI CLI records hold typed prompts but
no title. Restart the background service and view the project without opening the session. The row
shows a label taken from that conversation, not "New session".

**Acceptance Scenarios**:

1. **Given** a session whose AI CLI records contain at least one prompt the user typed but no title,
   **When** I view the project without opening that session,
   **Then** the row shows a label derived from that conversation instead of "New session".
2. **Given** several such sessions in one project, each with different typed prompts,
   **When** I view the project,
   **Then** each row shows its own label, and each label comes from that session's conversation only.
3. **Given** a session that shows a label under scenario 1,
   **When** the background service restarts, or the whole application is closed and opened again,
   **Then** the row shows the same label as soon as the list appears, with no intermediate
   "New session" state.
4. **Given** a session whose conversation contains no prompt the user typed (it was started and never
   used),
   **When** I view the project,
   **Then** the row still reads "New session".

---

### User Story 2 - The AI CLI's own title still wins (Priority: P2)

The AI CLI's title is a better name than anything derived from a raw prompt, and most sessions get
one (65 of the 80 most recent conversations in the BUG-001 survey). A label must fill the gap and
never compete with a real title.

**Why this priority**: Without it the fix degrades the sessions that already work. A titled session
suddenly showing a raw prompt would be a new bug.

**Independent Test**: For a session that has a title, confirm the row shows the title, not a label.
For a session that shows a label, have the AI CLI title the conversation and confirm the row does
what FR-006 decides, before and after a restart.

**Acceptance Scenarios**:

1. **Given** a session whose AI CLI has titled the conversation,
   **When** I view the project, running or not,
   **Then** the row shows the title, exactly as it does today, and never a derived label.
2. **Given** a session showing a derived label,
   **When** the AI CLI later titles that conversation,
   **Then** the row shows the title, and after the next restart the row shows the title, not the
   label (subject to the FR-006 decision).
3. **Given** a session showing a derived label that is not running,
   **When** its AI CLI records gain a title (for example, the conversation was resumed outside this
   application) and I restart the background service,
   **Then** the row shows the title (subject to the FR-006 decision). Having shown a label does not
   stop the application from finding a title that arrives later.

---

### User Story 3 - A session I am working in gets its label too (Priority: P3)

A session I am working in right now, whose AI CLI has not titled it, should not read "New session"
while the same conversation would read a label after a restart. The row should agree with what it
will show later.

**Why this priority**: The report is about past sessions (US1). This keeps a running session
consistent with them, but a running session is also the one the user has just been looking at, so
its placeholder costs less.

**Independent Test**: Start a session, type a prompt, and do not let the AI CLI title it. Without
stopping or reopening the session, watch its row: it shows the same label it would show after a
restart.

**Acceptance Scenarios**:

1. **Given** a running session with at least one typed prompt and no title,
   **When** I look at the sidebar,
   **Then** the row shows the derived label within 60 seconds of the prompt being submitted, without
   the session being stopped, restarted or reopened.
2. **Given** a running session showing a derived label,
   **When** the AI CLI titles the conversation,
   **Then** the row changes to the title as it does today (subject to the FR-006 decision).

---

### Edge Cases

- **Empty input.** A conversation with no typed prompt: only tool output, only a command the
  application ran on the user's behalf, or nothing at all. It keeps "New session" (029 FR-004 still
  holds for it).
- **A prompt that is only whitespace, or only a slash command with no text.** It is not a usable
  label on its own; which text, if any, stands for it is part of the label-source decision (FR-002).
- **A very long, multi-line prompt, or one with punctuation, code, emoji or non-Latin characters.**
  The label is one line, at most 80 user-perceived characters, ending in "…" when cut (FR-003). The
  row's existing ellipsis and the hover tooltip then treat the label as they treat a title: the
  tooltip shows the whole label, which is the cut text, not the whole prompt.
- **The AI CLI's records are missing, unreadable, or in a format the application does not
  recognise.** No label; the row reads "New session" (or its remembered label or title, if it has
  one) and the session is not reported as failed.
- **The AI CLI's records are deleted after a label was shown.** The remembered label stays, as a
  remembered title does (029 FR-008).
- **A very large conversation** (thousands of records, as the reported ones are). Producing its label
  costs no more than producing a label for a short one (FR-014). When the part of the conversation
  the application looks at holds no usable prompt, the row reads "New session".
- **Sensitive text in the prompt** (a pasted token, a password). The label shows it on the row, as
  the AI CLI's own records and the terminal already hold it. It stays on this machine (Principle IV)
  and in the same per-project storage as a title; no redaction is attempted. `pi` rows already do the
  same (029-pi-cli-provider FR-011).
- **Concurrency (Principle II).** Two sessions in the same worktree, one titled and one not: each row
  shows its own name or label, and neither takes the other's. The same session viewed from two
  application windows shows the same label in both. A label and a title that arrive for the same
  session at the same moment: the title is what is shown and remembered (FR-005). A conversation
  whose last record is only partly written yields a label from its complete records, or no label, and
  never an error.
- **Cross-platform (Principle VI).** Linux, macOS and Windows show the same label for the same
  conversation, although the AI CLI keeps its records in a different place on each and writes them
  with each platform's own conventions. A session run in the sandbox is labelled on the same rule as
  one run directly on the machine.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: For a session whose AI CLI has not titled the conversation, the application MUST show
  a label derived from that conversation wherever the session is listed or identified, when the
  conversation contains at least one prompt the user typed.
- **FR-002**: The label MUST be text the user typed in that conversation, and MUST come from that
  session's own conversation and no other [NEEDS CLARIFICATION: which text? See *Label-source
  evidence* below for what each candidate gives on the four reported sessions: (a) the first plain
  typed prompt, not a slash command; (b) the first turn, as its slash command's arguments when it
  has them, else the command name, else its plain text; (c) the first turn with free text of its own
  (a slash command's arguments or a plain prompt), skipping bare commands and one-word prompts such
  as `continue`. The `pi` precedent, "first user message", is (b) in `claude` terms.].
- **FR-003**: The label MUST be a single line of at most 80 user-perceived characters: line breaks
  and runs of whitespace collapsed to single spaces, leading and trailing whitespace removed, and a
  longer text cut between two user-perceived characters and ended with "…". A title is short by
  nature; a prompt can be pages long, and a row's name is not the place to keep a whole prompt.
- **FR-004**: A session whose conversation contains no usable typed prompt MUST keep the "New
  session" placeholder. This narrows 029 FR-004 to exactly those sessions.
- **FR-005**: Whenever the AI CLI's title for a conversation is known, the application MUST show the
  title and not a derived label. A derived label MUST NEVER replace a title, whether shown or
  remembered.
- **FR-006**: When a title becomes known for a session that has shown a derived label, the title
  MUST replace the label, both on the row and in what is remembered [NEEDS CLARIFICATION: Does a
  title that arrives after a label has been shown replace it? BUG-001 *Size decision* recommends yes
  (029 FR-005, "the latest known name"); the alternative keeps a label once shown, so a row's text
  never changes under the user.].
- **FR-007**: The application MUST remember a derived label durably, as it remembers a title (029
  FR-001), so that the row shows it after a restart without the AI CLI's records being read again for
  that purpose.
- **FR-008**: A remembered label MUST be distinguishable from a remembered title, so that FR-005
  holds after a restart as well as while the application runs.
- **FR-009**: Sessions that existed before this feature MUST get their labels without any user
  action: no re-creating, re-opening or renaming.
- **FR-010**: For a running session with no title, the row MUST show the derived label within 60
  seconds of the first usable typed prompt being submitted, without the session being stopped,
  restarted or reopened.
- **FR-011**: Failing to read the AI CLI's records, or failing to remember a label, MUST NOT change
  anything else on the row, MUST NOT interrupt the session and MUST NOT be reported as a session
  failure (029 FR-009).
- **FR-012**: The rule applies to `claude` sessions. [NEEDS CLARIFICATION: Does GitHub Copilot need
  the same treatment? Its session name is absent until Copilot has summarised the conversation, so
  it can show the same symptom. `pi` already falls back to the first user message under
  029-pi-cli-provider FR-011 and is out of scope unless its fallback turns out to fail the same
  way.]
- **FR-013**: A session's label MUST remain derived from its conversation and MUST NOT become
  user-editable because of this feature (029 FR-011).
- **FR-014**: Producing a label MUST look at a bounded part of the conversation from its start, so
  labelling a row costs the same whether its conversation is a minute or a year old (the trade-off
  029-pi-cli-provider FR-011 made for `pi`). When that part holds no usable text for FR-002, the row
  keeps the placeholder. The bound MUST be large enough that the chosen FR-002 source labels all
  four reported sessions (SC-001).
- **FR-015**: The user guide MUST say what the row of a session the AI CLI never titled reads, and
  that a title replaces it or not, per FR-006 (Principle VII).

### Label-source evidence

What each FR-002 candidate gives on the four untitled sessions in the report (BUG-001
*Reproduction*), read from their `claude` records on 2026-09-19. Each of the four starts with a slash
command; record numbers count every line of the record, not only prompts.

| Session | (a) first plain typed prompt | (b) first turn | (c) first turn with free text |
|---|---|---|---|
| `9a536c7e` | `continue` (record 790) | `/speckit-autopilot` (record 8, no arguments) | "the link should change cursor icon when it's clickable" (record 1112) |
| `2dc1bd13` | `/speckit-bugfix-verify` (record 242) | "I would like to start pi coding session in same way as I start copilot or claude session…" (record 7) | same as (b) |
| `e0912fd9` | `/speckit-bugfix-verify` (record 187) | "Currently when creating new worktree that for example has multiple git submodules may take 30 second…" (record 7) | same as (b) |
| `f2e8ef75` | `/speckit-bugfix-verify` (record 262) | "When running sessions withing sandbox it displays a warning that session files are available in read…" (record 7) | same as (b) |

(a) gives three identical rows, which fails US1 scenario 2 and SC-005. (b) and (c) give four
different rows; (b) needs only the first few records, (c) needs a bound past record 1112 for one
session. The recorded `/speckit-bugfix-verify` prompts are typed as plain text, not recognised as
slash commands by the records, which is why (a) picks them.

### Key Entities

- **Session**: A unit of work in a project, with a location and the AI CLI it runs. For this feature
  it carries at most one of: a title, a derived label, or neither.
- **Title**: The name the AI CLI gives a conversation. Unchanged by this feature, and always preferred
  over a derived label.
- **Derived label**: Short text taken from what the user typed in the conversation. It stands in for a
  title that does not exist yet, is remembered like one, and is remembered as a stand-in, distinct from
  a title (FR-008). Whether a later title replaces it is FR-006.
- **Typed prompt**: A turn in the conversation that the user entered, as opposed to tool output or
  text the application or AI CLI inserted.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: On the reporter's project (BUG-001 *Reproduction*, the five sessions in the
  screenshot), after the update and one restart, 5 of 5 rows show a name or label, where 1 of 5 does
  today. No session is opened to achieve this.
- **SC-002**: Of sessions whose AI CLI records contain a usable typed prompt but no title, 100% show
  a derived label after one restart, with no user action.
- **SC-003**: 100% of sessions that have a title show that title, not a derived label, before and
  after this change.
- **SC-004**: Zero sessions show a label or title taken from a different session's conversation, and
  zero sessions with no typed prompt show a label.
- **SC-005**: On the reporter's project (SC-001), the four rows that read "New session" today read
  four different labels.
- **SC-006**: Adding labels adds no perceptible delay: the session list of a project with 50 sessions,
  including conversations of more than 1,000 records, appears as quickly after this change as before.
- **SC-007**: A running untitled session shows its label within 60 seconds of the first typed prompt.

## Assumptions

- The AI CLI's decision on when to title a conversation is outside this application's control and is
  not explained (BUG-001 *When `claude` titles a conversation*). The fix does not wait for, or try to
  trigger, a title.
- 029's storage of a session's name is where a derived label belongs; this feature adds no new place
  the user has to manage.
- Reading the AI CLI's records is best-effort and local (Principle IV): a record that is missing,
  unreadable or unrecognised yields no label, exactly as it yields no title today.
- The label is shown only where a session's name is shown today (the sidebar row, its tooltip and the
  terminal bar); no new UI element is added.
- This feature supersedes two of 029's assumptions: "The name already has a single source — the AI
  CLI's title" (a derived label is a second, lower-priority source), and "Sessions of every supported
  AI CLI are covered by the same rule" (FR-012 limits the rule to `claude` until clarified). It
  narrows 029 FR-004 (FR-004 here) and keeps every other 029 requirement.

### Out of scope

- Reducing the noise of titles the AI CLI itself produces from late prompts ("Status", "M2
  progress"). Those are titles; this feature does not second-guess them (BUG-001 *Size decision*).
- Honouring a user's `/rename` (`custom-title` records) or `agent-name` records over the AI CLI's
  title. That is a candidate 029 bug, recorded in the BUG-001 ledger's follow-ups.
- Letting the user name a session by hand.
- Changing how `pi` labels a session (029-pi-cli-provider FR-011), unless FR-012's clarification
  brings it in.
