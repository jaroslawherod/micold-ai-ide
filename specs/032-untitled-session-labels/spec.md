# Feature Specification: A session the AI CLI never titled still gets a label

**Feature Branch**: `fix/the-name-of-past-session-is-still-not-shown`

**Created**: 2026-09-19

**Status**: Closed 2026-09-26 — shipped in PRs #386, #388, #395, #396, #399

**Input**: User description: "bug: see the screen show the name of past session is still not displayed
even when the bug was reported and fixed". Traced in
[029 BUG-001](../029-persistent-session-names/bugs/BUG-001.md): four of the five `claude` sessions in
the report have real conversations with prompts the user typed, but `claude` never recorded a title
(`ai-title`) for any of them. Feature 029 remembers and recovers every title that exists, and by its
FR-004 a session with no title keeps the "New session" placeholder for good. Showing a name for these
sessions needs a label source that 029 never intended, so it is specified here rather than patched
into 029 (BUG-001 *Size decision*).

## Clarifications

### Session 2026-09-19

- Q: FR-002: which text the user typed becomes the label? → A: (b) the first turn: a slash
  command's arguments when it has them, else the command name; otherwise the prompt text.
  _(decided by user)_
- Q: FR-006: does a title that arrives after a label has been shown replace it? → A: Yes. The
  title replaces the label on the row and in what is remembered. _(decided by user)_
- Q: FR-012: does GitHub Copilot get the same treatment as `claude`? → A: Yes. `claude` and
  Copilot sessions both get the derived label; `pi` stays out of scope. _(decided by user)_
- Q: Is a `claude` slash command that the CLI handles itself and never sends to the model (`/model`,
  `/login`, `/usage`, `/clear`) a turn, so a session opened with a bare `/model` reads "/model"? →
  A: No. Only a slash command that sends a prompt to the model (a custom command or skill, such as
  `/speckit-autopilot`) is a turn; a CLI-handled command is skipped. A session holding only such
  commands has no typed prompt and keeps "New session" (US1 scenario 4, FR-004), the same outcome
  as Copilot, which records no turn for them. Seen on the development machine: 2 of 87 recent
  `claude` sessions open with a bare `/model`. _(agent-resolved: specs/032-untitled-session-labels/spec.md#User Story 1 scenario 4, FR-001, FR-012)_
- Q: When two sessions have the same first turn (two sessions both opened with a bare
  `/speckit-autopilot`), are their labels made distinct? → A: No. Each shows its own derived label,
  identical or not; no suffix or disambiguation is added, as none is added to two identical titles.
  _(agent-resolved: specs/029-persistent-session-names/spec.md#Requirements; specs/029-pi-cli-provider/spec.md#FR-011)_
- Q: Does a derived label look different on the row from a title? → A: No. It is shown exactly
  where and as a title is shown (row, tooltip, terminal bar), with no new styling or UI element.
  _(agent-resolved: specs/032-untitled-session-labels/spec.md#Assumptions)_
- Q: Must FR-014's bound also reach the first turn of the untitled Copilot sessions? → A: Yes. The
  bound must label all four reported `claude` sessions (SC-001) and reach the first turn of all 44
  Copilot sessions (SC-009 since D9), which is at record 2–10. _(agent-resolved: specs/032-untitled-session-labels/spec.md#Copilot evidence)_
- Q: Is the `summary:` key that Copilot 1.0.36 and older wrote to `workspace.yaml` instead of
  `name:` read as that session's title? → A: Yes ("read the old copilot summary as the name
  too"). It is Copilot's own title: `name:` wins when both are present, and `summary:` ranks above
  a derived label (FR-016). _(decided by user)_
- Q: The 44 old Copilot sessions are in no Copilot index and are not rows at all; should the
  application also discover them by scanning Copilot's session directories? → A: No. FR-016 applies
  to the Copilot sessions the application lists; US1 #6, SC-008 and SC-009 are verified with
  listed-session fixtures, and discovery is unchanged. _(decided by user)_

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
5. **Given** a GitHub Copilot session whose records hold a typed prompt but neither a `name:` nor a
   `summary:` title (for example, one Copilot has not summarised yet),
   **When** I view the project without opening that session,
   **Then** the row shows a label taken from that session's first turn, on the same rules as a
   `claude` session.
6. **Given** a GitHub Copilot session **that the application lists** (it is in Copilot's index for
   the location, or already in the application's own records), run by Copilot 1.0.36 or older, whose
   `workspace.yaml` holds a `summary:` and no `name:` (the shape of all 44 such sessions on the
   development machine, see *Copilot evidence*),
   **When** I view the project without opening that session,
   **Then** the row shows that `summary:` as the session's title, not "New session" and not a
   derived label (FR-016).

---

### User Story 2 - The AI CLI's own title still wins (Priority: P2)

The AI CLI's title is a better name than anything derived from a raw prompt, and most sessions get
one (65 of the 80 most recent conversations in the BUG-001 survey). A label must fill the gap and
never compete with a real title.

**Why this priority**: Without it the fix degrades the sessions that already work. A titled session
suddenly showing a raw prompt would be a new bug.

**Independent Test**: For a session that has a title, confirm the row shows the title, not a label.
For a session that shows a label, have the AI CLI title the conversation and confirm the row shows
the title, before and after a restart (FR-006).

**Acceptance Scenarios**:

1. **Given** a session whose AI CLI has titled the conversation,
   **When** I view the project, running or not,
   **Then** the row shows the title, exactly as it does today, and never a derived label.
2. **Given** a session showing a derived label,
   **When** the AI CLI later titles that conversation,
   **Then** the row shows the title, and after the next restart the row shows the title, not the
   label (FR-006).
3. **Given** a session showing a derived label that is not running,
   **When** its AI CLI records gain a title (for example, the conversation was resumed outside this
   application) and I restart the background service,
   **Then** the row shows the title (FR-006). Having shown a label does not
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
   **Then** the row changes to the title as it does today (FR-006).

---

### Edge Cases

- **Empty input.** A conversation with no typed prompt: only tool output, only a command the
  application ran on the user's behalf, or nothing at all. It keeps "New session" (029 FR-004 still
  holds for it).
- **A prompt that is only whitespace, or only a slash command with no text.** A `claude` slash
  command with no arguments stands for itself: the label is the command name, such as
  `/speckit-autopilot` (FR-002). A command `claude` handles itself without prompting the model
  (`/model`, `/login`) is not a turn (FR-012). A turn with no text left after FR-003's whitespace rules is not a
  turn for FR-002, and the next turn is looked at instead. A Copilot slash command with no
  arguments (`/model`, `/usage`) records no turn at all, so it never becomes a label.
- **Two sessions with the same first turn.** Both rows show the same label; nothing is added to
  tell them apart, as nothing is added to two identical titles.
- **Text the AI CLI inserted as if the user had typed it.** `claude` command output and injected
  reminders, and Copilot's skill context, custom-instruction reminders and autopilot continuations
  (FR-012), are not typed prompts and never become a label.
- **A very long, multi-line prompt, or one with punctuation, code, emoji or non-Latin characters.**
  The label is one line, at most 80 user-perceived characters, ending in "…" when cut (FR-003). The
  row's existing ellipsis and the hover tooltip then treat the label as they treat a title: the
  tooltip shows the whole label, which is the cut text, not the whole prompt. A label is not styled
  differently from a title.
- **A Copilot `summary:` written in a form the application does not read** (a multi-line block
  scalar, seen only beside a `name:` on the development machine). It is treated as absent: the
  session gets a derived label if it has a typed prompt (FR-016).
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
  session's own conversation and no other. It is the conversation's **first turn**: when that turn
  is a slash command, the command's arguments if it has any, else the command name (with its
  leading `/`); otherwise the prompt text. A turn with no text after FR-003's whitespace rules is
  skipped. What counts as a turn for each AI CLI is FR-012. This is candidate (b) of *Label-source
  evidence*, and the same rule as `pi`'s "first user message" (029-pi-cli-provider FR-011).
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
  MUST replace the label, both on the row and in what is remembered, as 029 FR-005 keeps "the
  latest known name". A label never replaces a title (FR-005), so the change only ever goes from
  label to title.
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
- **FR-012**: The rule applies to `claude` and GitHub Copilot sessions. A **turn** is:
  - for `claude`, a prompt record the user entered, as opposed to tool results, command output and
    text `claude` or the application inserted; a slash command is recorded with its name and its
    arguments kept apart, which is what FR-002 reads. A slash command is a turn only when it sends a
    prompt to the model (a custom command or skill); one `claude` handles itself (`/model`,
    `/login`, `/usage`, `/clear`) is not a turn;
  - for Copilot, a `user.message` record in the session's `events.jsonl` whose `content` the user
    entered: not one Copilot inserted (a record carrying a `source`, such as `skill-<name>` skill
    context or `instruction-discovery` reminders) and not an autopilot continuation
    (`isAutopilotContinuation`). The label is taken from `content`, never `transformedContent`,
    which Copilot wraps in timestamps and reminders. Copilot already strips a slash command before
    recording the turn: `/plan <text>` is recorded as `<text>`, `/fleet <text>` as
    `Fleet deployed: <text>`, and a bare command records no turn, so FR-002's "command name" case
    never arises for Copilot and the recorded `content` is the label.

  `pi` already falls back to the first user message (029-pi-cli-provider FR-011) and is out of
  scope.
- **FR-013**: A session's label MUST remain derived from its conversation and MUST NOT become
  user-editable because of this feature (029 FR-011).
- **FR-014**: Producing a label MUST look at a bounded part of the conversation from its start, so
  labelling a row costs the same whether its conversation is a minute or a year old (the trade-off
  029-pi-cli-provider FR-011 made for `pi`). When that part holds no usable text for FR-002, the row
  keeps the placeholder. The bound MUST be large enough that FR-002 labels all four reported
  `claude` sessions (SC-001) and the first turn of all 44 Copilot sessions of *Copilot evidence*
  (SC-009).
- **FR-015**: The user guide MUST say what the row of a session the AI CLI never titled reads, and
  that a title arriving later replaces it (FR-006) (Principle VII).
- **FR-016**: For a GitHub Copilot session, the `summary:` key of the session's `workspace.yaml` MUST
  be read as the session's title when `name:` is absent; `name:` wins when both are present. It is a
  title for FR-005 and FR-006: it ranks above a derived label, is remembered as a title, and replaces
  a derived label when it becomes known. A `summary:` value the application cannot read as a single
  line is treated as absent.

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
slash commands by the records, which is why (a) picks them. The user chose (b) (Clarifications).

### Copilot evidence

Read from the 281 session directories in `~/.copilot/session-state/` on the development machine on
2026-09-19, against the application's current Copilot title rule, which reads only the `name:` key of
`workspace.yaml`:

- 142 sessions have an `events.jsonl` (a recorded conversation). 97 have a `name:`; 44 have at least
  one `user.message` and no `name:`; 1 has no `user.message`.
- All 44 were run by Copilot 1.0.10–1.0.36, and every one of them has a one-line `summary:` key
  instead of `name:` in `workspace.yaml`. Every session run by 1.0.37 or later that holds a
  `user.message` has a `name:`. The two multi-line (`summary: |-`) values on the machine both sit
  beside a `name:`.
- **None of the 44 is a row today** (found in plan review, 2026-09-19): Copilot's per-location index
  (`~/.copilot/sidebar-sessions-state/*.json`, 4 ids in total on the machine) lists none of them, the
  application discovers Copilot sessions only through that index (026 research R3), and the
  application's own records hold none of them. FR-016 therefore applies to Copilot sessions the
  application lists; discovering sessions missing from Copilot's index is out of scope (D10).
- The label rule (FR-002, FR-012) applies to a listed Copilot session with neither key, which is
  every running session until Copilot summarises it (US3). The 44 sessions' records are the shapes
  its fixtures are modelled on (SC-008, SC-009).
- The first qualifying `user.message` is at record 2–10 in all 44, well inside any bound FR-014 sets
  for `claude`'s record 7–8.
- Of 972 `user.message` records, 12 carry a `source` (skill context, `instruction-discovery`) and 28
  are autopilot continuations with empty `content`; FR-012 excludes both.
- Copilot's command history holds `/plan …` and `/fleet …`; their sessions record the text after the
  command (`/fleet` with a `Fleet deployed: ` prefix). Bare commands (`/model`, `/usage`, `/login`)
  record no `user.message`.

A running Copilot session reads "New session" until Copilot writes `name:` (or, before 1.0.37,
`summary:`), so US3 applies to it in that window.

### Key Entities

- **Session**: A unit of work in a project, with a location and the AI CLI it runs. For this feature
  it carries at most one of: a title, a derived label, or neither.
- **Title**: The name the AI CLI gives a conversation (`claude`'s `ai-title`; Copilot's `name:`, or
  `summary:` when `name:` is absent, FR-016). Always preferred over a derived label.
- **Derived label**: Short text taken from what the user typed in the conversation. It stands in for a
  title that does not exist yet, is remembered like one, and is remembered as a stand-in, distinct from
  a title (FR-008). A later title replaces it (FR-006).
- **Typed prompt**: A turn in the conversation that the user entered, as opposed to tool output or
  text the application or AI CLI inserted. What counts as one for each AI CLI is FR-012.

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
- **SC-008**: Every listed Copilot session whose `workspace.yaml` holds a one-line `summary:` and no
  `name:` shows that `summary:` as its title after one restart, and none shows a derived label;
  verified on listed-session fixtures shaped on the 44 sessions of *Copilot evidence*.
- **SC-009**: Every listed Copilot session with a typed turn and neither `name:` nor a readable
  `summary:` shows its first qualifying `content`, shaped by FR-003, as its label (FR-002, FR-012,
  FR-014); verified on listed-session fixtures shaped on the same 44 sessions' first turns.

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
  AI CLI are covered by the same rule" (FR-012 applies the label rule to `claude` and Copilot;
  `pi` keeps its own). It narrows 029 FR-004 (FR-004 here) and keeps every other 029 requirement.
- Copilot's record format is the one read on 2026-09-19 from versions 1.0.10–1.0.83 (*Copilot
  evidence*). A later Copilot that records turns differently yields no label, as an unrecognised
  record does (Edge Cases), until the provider is updated.

### Out of scope

- Reducing the noise of titles the AI CLI itself produces from late prompts ("Status", "M2
  progress"). Those are titles; this feature does not second-guess them (BUG-001 *Size decision*).
- Honouring a user's `/rename` (`custom-title` records) or `agent-name` records over the AI CLI's
  title. That is a candidate 029 bug, recorded in the BUG-001 ledger's follow-ups.
- Letting the user name a session by hand.
- Changing how `pi` labels a session (029-pi-cli-provider FR-011).
- Discovering Copilot sessions that Copilot's own per-location index no longer lists (the 44 old
  sessions of *Copilot evidence*). Surfacing them would need a scan of every
  `session-state/*/workspace.yaml` per project open (D10).
