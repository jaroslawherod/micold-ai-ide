# Feature Specification: Collapsed Summaries for Raw Events in the Structured View

**Feature Branch**: `feat/1664_structured-view-raw-events-are-shown-inline-in-the`

**Created**: 2026-11-03

**Status**: Draft

**Input**: User description: "Structured view: raw events are shown inline in the transcript; present them as collapsed summaries (GitHub issue #1664). The structured session view shows raw events (unrecognised event, unreadable line, unknown block, unknown stream event) and other low-signal rows (session start, turn footer, orphan result) inline, as they arrive. They clutter the transcript and push the conversation out of view. Proposal: one summary line per event with the raw JSON in an expandable row collapsed by default; a 'Show raw events' toggle in the view header, off by default, remembered per user. Grouping runs, a raw log in the session menu and an unsupported-event marker can follow."

## Terms

- **Structured view**: the session view that renders a session's transcript as entries (messages, tool cards, notices) rather than a terminal.
- **Low-signal entry**: a transcript entry that records plumbing, not conversation. Today: *raw* entries (unrecognised event, unreadable line, unknown block, unknown stream event), *session start*, *turn footer* and *orphan result*.
- **Summary line**: the single collapsed line that stands for one low-signal entry (for example "Session started · model X", "Turn done · 12s · 3 turns", "Unrecognised event (stream_event)").
- **Detail**: what a summary line expands to: for a raw entry its original text, for the others the fields the entry holds today.
- **Show raw events toggle**: the control in the structured view's header that decides whether low-signal entries appear in the transcript.

## Clarifications

### Session 2026-11-03

None yet. Decisions taken as assumptions are listed under Assumptions.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Read the conversation without plumbing in the way (Priority: P1)

A user follows a structured session. Today raw entries and orphan results print their body inline, while the session start and the turn footer are already compact rows. After this feature each of the four is one short collapsed line that stays out of the way, and the conversation reads as a conversation.

**Why this priority**: This is the issue's core complaint: the transcript is cluttered and the conversation is pushed out of view.

**Independent Test**: Open a transcript containing one of each low-signal entry between two messages. Each shows as one summary line; no raw JSON is visible; expanding one shows its detail.

**Acceptance Scenarios**:

1. **Given** a transcript with a raw entry, **When** it is rendered, **Then** it shows one summary line naming what happened (for example "Unrecognised event (stream_event)") and none of the raw text.
2. **Given** a summary line, **When** the user expands it, **Then** the detail appears below it (a raw entry shows its original text), and collapsing it hides the detail again.
3. **Given** a session start, turn footer or orphan result, **When** it is rendered, **Then** it shows one summary line carrying the facts the entry holds (session start: model; turn footer: outcome, duration and number of turns; orphan result: whether it is an error) and expands to the rest (a footer's cost and error text; an orphan result's output).
4. **Given** a new low-signal entry arrives while the user is reading, **When** it is appended, **Then** it appears as a collapsed summary line and does not move the reading position beyond one line's height.

---

### User Story 2 - Hide low-signal entries altogether, and get them back (Priority: P2)

The user wants a transcript with only the conversation. A "Show raw events" toggle in the view header, off by default, hides the low-signal entries; turning it on shows their summary lines. The choice is remembered for this user.

**Why this priority**: Summary lines still take a line each; the toggle gives the clean view the issue asks for, and the acceptance criteria hinge on it.

**Independent Test**: With the toggle off, open a transcript with raw entries: no low-signal entry and no raw JSON appears. Turn it on: every one appears as a summary line. Restart the app: the toggle is as left.

**Acceptance Scenarios**:

1. **Given** the toggle is off (the default), **When** a transcript renders, **Then** no low-signal entry and no raw JSON appears in it.
2. **Given** the toggle is off, **When** the user turns it on, **Then** every low-signal entry appears in place as a collapsed summary line, with no entry missing.
3. **Given** the user turned the toggle on, **When** the app restarts and a structured view opens, **Then** the toggle is on.
4. **Given** the toggle is off and entries are hidden, **When** the view renders, **Then** it shows that events are hidden and how many, so the user knows they exist and can reveal them.
5. **Given** two structured views open at once, **When** the user changes the toggle in one, **Then** the other follows (the choice is the user's, not the view's).

---

### User Story 3 - Same behaviour for live and resumed transcripts (Priority: P2)

A session resumed from its saved history shows its low-signal entries the same way a live session does.

**Why this priority**: The issue's acceptance names it; a fix for live sessions alone leaves resumed ones cluttered.

**Independent Test**: Resume a session whose history holds raw entries: with the toggle off none appears; with it on each appears as a summary line.

**Acceptance Scenarios**:

1. **Given** a resumed session, **When** its history loads, **Then** its low-signal entries follow the toggle exactly as live ones do.
2. **Given** a live session, **When** it is later resumed, **Then** the same events are summarised the same way.

---

### Edge Cases

- Empty or whitespace-only raw text: the summary line still shows, the detail says the event was empty.
- Very large raw text: the detail is bounded in height and scrolls or truncates with a marker; collapsed, it costs one line.
- Unreadable line (not valid JSON): summary names it as unreadable and the detail shows the text as received.
- Search, selection and copy: hidden entries are not matched or copied while hidden; revealed entries behave like any entry.
- Entry still streaming or updating (a footer arriving after its turn): the summary line updates in place.
- Many consecutive low-signal entries: each keeps its own line (grouping is out of scope).
- Several sessions, several windows: the toggle is one per user, not per session; expansion state is per view.
- Platforms: behaviour and stored preference are the same on Linux, macOS and Windows.
- A transcript made only of low-signal entries with the toggle off: shows the hidden-events notice, not an empty-looking view.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: The structured view MUST render each low-signal entry (raw, session start, turn footer, orphan result) as one summary line.
- **FR-002**: A summary line MUST state what the entry is in plain words and carry its key facts where it has them (model for a session start; outcome, duration and number of turns for a turn footer; ok or error for an orphan result; the reason for a raw entry).
- **FR-003**: A summary line MUST expand to its detail and collapse again; it is collapsed when first shown.
- **FR-004**: A raw entry's detail MUST show its original text with no content changed or dropped (tabs may be drawn as spaces).
- **FR-005**: The structured view's header MUST hold a "Show raw events" toggle, off by default.
- **FR-006**: With the toggle off, the transcript MUST contain no low-signal entry and no raw JSON, except as FR-013 says.
- **FR-007**: With the toggle on, every low-signal entry MUST appear as a summary line, in transcript order.
- **FR-008**: While entries are hidden, the view MUST show how many are hidden and offer a way to reveal them.
- **FR-009**: The toggle's state MUST be remembered per user, across restarts and across views.
- **FR-010**: No event MAY be dropped: every hidden entry stays in the transcript data and is reachable by turning the toggle on.
- **FR-011**: FR-001 to FR-010 MUST hold for live and for resumed transcripts alike.
- **FR-012**: Conversation entries (messages, thinking, tool cards, command output, compaction and other session notices) MUST be unchanged by this feature.
- **FR-013**: [NEEDS CLARIFICATION: with the toggle off, do an error turn footer and an error orphan result stay visible as summary lines (default, so a failed turn is never silent), or are they hidden like the rest and counted in the hidden-events notice?]

### Key Entities

- **Low-signal entry**: a transcript entry classed as plumbing; classified by its kind, not by its content.
- **Show raw events preference**: one boolean per user, default off.
- **Summary line**: a view-only rendering of an entry; holds no data of its own.

## Success Criteria *(mandatory)*

- **SC-001**: With the toggle off, no raw JSON text is visible in any transcript, live or resumed, in 100% of the test transcripts.
- **SC-002**: With the toggle on, the count of summary lines equals the count of low-signal entries in the transcript.
- **SC-003**: A low-signal entry occupies one line height while collapsed.
- **SC-004**: The preference survives an app restart in 100% of runs.

## Assumptions

- The low-signal set is the issue's list; other notices (interrupted, oversized event, sub-agent notice, turn ended without result, terminal period) stay as they are.
- The preference lives with the user's other app settings (the per-user app configuration the client already persists), not in the session or worktree.
- The toggle is a control in the structured view's header, as the issue says, not a menu item.
- Summary wording is English and follows the existing transcript's text style.

## Out of Scope

Follow-ups named by the issue, left for later issues:

- Grouping runs of consecutive low-signal rows into one "N events" chip.
- "Copy raw log" and "Open event log" in the session menu.
- The "unsupported event" marker with the payload sent to a log instead of the transcript. (Until then a raw entry's payload stays reachable in its expandable detail.)
- Changes to how events are parsed or stored.
