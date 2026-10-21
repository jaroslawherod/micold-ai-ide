# Feature Specification: Structured Session Transcript View — read-only assistant text, thinking and tool calls

**Feature Branch**: `claude/project-thread-61i2k8`

**Created**: 2026-10-21

**Status**: Closed 2026-10-21 — shipped in PRs #1183, #1189, #1192, #1193, #1198, #1203, #1206 and the close PR

**Input**: GitHub issue #939 "Structured session view: read-only transcript (assistant text,
thinking, tool calls)". "Part of the structured-session (stream-json) UI epic. Keeps the PTY/TUI
session mode as the fallback throughout. Client view rendering the event stream: streaming assistant
text, collapsible thinking and tool-call cards (name, input summary, result), turn/result footer
(cost, duration). No input yet. Acceptance: Streams deltas without flicker; scrolls with
follow-tail. Tool cards collapse/expand; errors are visually distinct. Depends on the structured
session mode issue." labels: enhancement, flow:feature

## Clarifications

### Session 2026-10-21

- Q: How are events produced inside a sub-agent (they name the tool use that started it) shown? →
  A: Not as transcript entries: #944 owns nesting sub-agent transcripts under the parent tool call,
  and this view leaves them out of the main transcript so #944 can add nesting on top. Each such
  event is attributed to the tool-call card of the tool use it names, whose header shows how many
  sub-agent messages it holds; their content is not drawn. Events naming a tool use that is not in
  the transcript are counted on one notice instead, so none is silently dropped (FR-003, FR-024).
  _(agent-resolved: specs/939-structured-transcript-view/spec.md#out-of-scope; issue #944 scope
  "subagent activity nested under the parent tool call")_
- Q: Do a sub-agent's streaming deltas count toward its card's sub-agent event count? → A: No: the
  count is of the sub-agent's messages (assistant and user messages, including tool results), not
  of deltas, since a finished message supersedes its deltas in this view; deltas naming a parent
  tool use are neither shown nor counted. _(agent-resolved:
  specs/939-structured-transcript-view/spec.md#assumptions, "A finished assistant message
  supersedes the deltas of the same message")_

## In short (for non-technical readers)

A structured Claude Code session (#938) today shows a plain technical log: one line per item Claude
reported. This feature replaces that log with a readable conversation transcript. Claude's replies
appear as text that grows while Claude writes it; Claude's reasoning ("thinking") and each tool it
uses appear as cards the user can fold and unfold; each card for a tool says which tool, what it was
asked to do in one line, and what came back; a failed tool or a failed turn looks clearly different
from a successful one. At the end of each answer a small footer says how long it took and what it
cost. The view follows the newest output, like a terminal does, until the user scrolls up to read
something, and it never jumps or blinks while text streams in. The user still cannot type into the
session here (that is #941); terminal sessions are unchanged and stay the default.

## Context: where this sits in epic #945

| Issue | Delivers |
|---|---|
| #938 (merged) | The structured session kind, the event model, the daemon side, the wire, resume, and the plain log this feature replaces |
| **#939 (this)** | The read-only transcript view: streaming assistant text, thinking and tool-call cards, turn footers, follow-tail |
| #940 | Markdown, highlighted code and Edit/Write diffs rendered inside this view |
| #941 | Prompt input, interrupt and queued messages |
| #942 | Permission prompts |
| #943 | Slash commands, model switching, falling back to the terminal mode |
| #944 | Usage, context meter, sub-agents, background tasks |

## Terms

- **Transcript**: the read-only view of one structured session's events in a window, top to
  bottom in the order the session reported them.
- **Entry**: one visible item of the transcript: a session-start line, a user message, an assistant
  text block, a thinking card, a tool-call card, a turn footer, a notice (omitted events, an event
  too large to show, a turn that ended without a result), or a raw row for an unrecognised event or
  an unreadable line.
- **Streaming**: while a turn is in progress, the CLI reports partial-message deltas before the
  finished message; the transcript shows the text as it grows from the deltas, and the finished
  message then takes the partial one's place.
- **Tool-call card**: one entry per tool use, paired with its tool result by the tool use's
  identifier: tool name, a one-line input summary, a status (running, done, failed), and, expanded,
  the full input and the result.
- **Turn footer**: the entry that closes a turn, from its result event: how it ended (success or
  error), its duration, the session's running cost and its number of turns.
- **Follow-tail**: while the view is scrolled to its end, new and growing content keeps the end in
  view; once the user scrolls away from the end, the view stays where the user put it.
- **Block renderer**: the one replaceable unit that draws one kind of block (assistant text,
  thinking, tool input, tool result) inside an entry; #940 swaps some of them for markdown, code and
  diff renderers.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Read a structured session as a conversation (Priority: P1)

A user opens a structured Claude Code session's pane and, instead of the plain log, reads the
conversation: the user messages sent into the session, Claude's replies as text, and each turn
closed by a footer giving its duration and cost. Events the view does not model are still shown, as
raw rows, so nothing is hidden.

**Why this priority**: It is the core of the issue and replaces the plain log; on its own it already
makes a structured session readable.

**Independent Test**: Feed a recorded fixture with a plain text turn and an error turn into the
window's transcript state and check the entries produced, in order: session start, user message,
assistant text, success footer with duration and cost, then the error turn's footer marked as an
error. Delivers a readable transcript without cards or streaming.

**Acceptance Scenarios**:

1. **Given** a structured session whose history holds a completed text turn, **When** the user
   views its pane, **Then** the transcript shows a session-start entry (model and working folder),
   the user's message, Claude's reply text, and a turn footer with the turn's duration, the
   session's running cost (labelled as the session total) and number of turns, in that order, and
   no plain-log line.
2. **Given** a turn whose result reports an error, **When** it is shown, **Then** its footer is
   marked as failed with a label and an icon as well as colour, and shows the error text the result
   carries.
3. **Given** a history that holds an unrecognised event and an unreadable line, **When** the
   transcript is shown, **Then** each appears in its place as a raw row showing its original text,
   visibly distinct from conversation entries.
4. **Given** a history whose earliest events the service no longer holds, **When** the transcript
   is shown, **Then** its first entry says how many events were omitted.
5. **Given** the session's process ended mid-turn, **When** the transcript is shown, **Then** that
   turn ends with a notice that it ended without a result, in place of a footer.
6. **Given** a terminal session, **When** the user views its pane, **Then** it shows the terminal
   exactly as before.

---

### User Story 2 - Fold thinking and tool calls into cards (Priority: P1)

Claude's reasoning and each tool it runs appear as cards. Folded, a tool card says which tool ran,
what it was asked in one line (the command, the file, the pattern) and whether it is running,
succeeded or failed; unfolded, it shows the full input and the result. A failed tool's card is
unmistakable even when folded.

**Why this priority**: Tool calls are most of a Claude session's output; without cards the
transcript is a wall of tool input and output. It is half of the issue's acceptance.

**Independent Test**: Feed a fixture with a tool call and its result, and one with a failing tool
result, into the transcript state; check one card per tool use, paired with its result, folded by
default, with the expected summary and status; toggle a card and check it expands and folds back.

**Acceptance Scenarios**:

1. **Given** a turn with a tool use and its result, **When** it is shown, **Then** one tool card
   appears where the tool use occurred, folded, showing the tool's name, a one-line input summary
   and a done status; the result does not appear as a separate entry.
2. **Given** a folded tool card, **When** the user activates its header, **Then** it unfolds to
   show the full input and the result text; activating it again folds it.
3. **Given** a tool result marked as an error, **When** its card is shown folded or unfolded,
   **Then** the card is marked as failed by a label and an icon as well as colour.
4. **Given** a tool use with no result yet, **When** it is shown, **Then** its card shows a running
   status; **When** the result then arrives, **Then** the same card turns done or failed in place.
5. **Given** a thinking block, **When** it is shown, **Then** it appears as a folded thinking card
   whose header says it is Claude's thinking; unfolding it shows the thinking text.
6. **Given** a tool use whose turn ended without its result (process ended, or the result was
   never reported), **When** the turn is closed, **Then** the card stops showing running and shows
   that no result was received.
7. **Given** the user unfolded some cards, **When** new events arrive or the user switches to
   another pane and back in the same window, **Then** those cards stay unfolded.

---

### User Story 3 - Watch a turn stream in without flicker, following the tail (Priority: P2)

While Claude is answering, the reply grows word by word at the bottom of the transcript and the view
keeps it in sight. Earlier entries do not move, blink or redraw out of order. If the user scrolls up
to reread something, the view stops following and stays put; scrolling back to the end, or using a
"jump to latest" control, resumes following.

**Why this priority**: It is the other half of the acceptance and what makes the view usable live;
it builds on stories 1 and 2, which are already useful on a finished history.

**Independent Test**: Feed a fixture with partial-message deltas into the transcript state one event
at a time and check that each delta grows the same entry, that the finished message replaces the
partial one with no duplicate text, and that entries before it are unchanged after each step; drive
the scroll state at the end and away from it and check follow-tail.

**Acceptance Scenarios**:

1. **Given** a turn in progress, **When** text deltas arrive, **Then** the reply's text entry grows
   in place with each delta, and no other entry is added, removed or changed.
2. **Given** a reply shown from deltas, **When** the finished assistant message arrives, **Then**
   the entry shows the finished text once, with no duplicated or lost text, and stays in the same
   place.
3. **Given** thinking deltas or tool-input deltas, **When** they arrive, **Then** the thinking card
   or the tool card appears at once and grows in place in the same way.
4. **Given** the view is scrolled to its end, **When** content is added or grows, **Then** the end
   stays in view.
5. **Given** the user has scrolled away from the end, **When** content is added or grows, **Then**
   the visible content does not move, and a "jump to latest" control is shown; **When** the user
   activates it or scrolls back to the end, **Then** following resumes and the control is hidden.
6. **Given** a user views a structured session for the first time in a window, **When** its history
   is shown, **Then** the view opens at the end, following.

---

### Edge Cases

- **Empty session**: a structured session with no events yet shows an empty transcript with a short
  "No output yet" note, not an error.
- **Deltas without a finished message**: if a turn's deltas are never followed by the finished
  message (the process ended), the partial text stays shown, the turn ends with the
  ended-without-result notice, and nothing is lost.
- **A finished message without deltas** (history restored from disk, or a CLI run without partial
  messages): it is shown whole, the same as after streaming.
- **Re-sync**: the window views the session again or the service sends a history that does not
  continue what the window holds; the transcript is rebuilt from that history with no duplicated
  entries, and the user's folded/unfolded choices for cards still present are kept.
- **Bounded history**: when the window drops the oldest events to stay within the history bound, the
  entries they made leave the top of the transcript and the omitted notice counts them; a card whose
  tool use was dropped but whose result arrives later is shown as a result without its call, not as
  an error.
- **Event too large to send**: an event the service replaced because it was too large for the wire
  is shown as a notice saying an event of that size was not shown.
- **Very long content**: a tool result or thinking text far longer than the view is shown unfolded in
  full and the view stays scrollable and responsive (SC-004); a one-line input summary never wraps
  onto a second line, it is cut with an ellipsis.
- **Unknown tool or unexpected input**: a tool with no known summary rule, or an input that is not
  the shape expected, gets a generic one-line summary of its input; it is never an error.
- **Tool result content as a block list**: a result whose content is a list of blocks rather than
  text shows the text parts, and names any part it cannot show (an image) instead of hiding it.
- **Sub-agent events** (events carrying a parent tool use): they are not shown as entries. Each
  sub-agent message is counted on the tool-call card of the tool use it names, whose header shows
  the count (FR-024); one naming a tool use not in the transcript (dropped by the history bound, or never seen) is
  counted on a single "sub-agent messages not shown" notice. #944 nests them under the card.
- **Several windows (Principle II)**: two windows viewing the same session show the same entries in
  the same order; each window has its own scroll position, follow-tail state and folded/unfolded
  choices. Two structured sessions viewed side by side in one window never mix entries, fold state
  or scroll state.
- **Session deleted or forgotten**: its transcript state is dropped from the window, like the plain
  log's.
- **Light and dark theme**: every entry kind, including failed cards and footers, is readable and
  distinct in both themes; failure is never conveyed by colour alone.
- **Cross-platform (Principle VI)**: the view behaves the same on Linux, macOS and Windows: the same
  entries, folding, follow-tail and scroll-to-end detection, whatever the platform's scroll input
  (wheel, trackpad with momentum, scrollbar drag, keyboard paging).

## Requirements *(mandatory)*

### Functional Requirements

**Transcript content**

- **FR-001**: A structured session's pane MUST show the transcript in place of the plain one-line
  log of #938 (its FR-022a); a terminal session's pane MUST be unchanged.
- **FR-002**: The transcript MUST show, in the order the session reported them: a session-start
  entry (model and working folder), the text of each user message, each assistant text block, each
  thinking block as a thinking card, each tool use as a tool-call card, and a footer per turn result.
- **FR-003**: Every event the transcript does not model (an unrecognised event, an unreadable line,
  an unmodelled content block) MUST appear in its place as a raw row showing its original text,
  distinct from conversation entries; no event may be silently dropped.
- **FR-004**: The transcript MUST start with a notice of how many events are omitted whenever the
  service or the window no longer holds the earliest events; MUST show a notice in place of an event
  that was too large to send; and MUST end a turn whose process ended mid-turn with a notice that it
  ended without a result.
- **FR-005**: A turn footer MUST show whether the turn succeeded or failed, its duration, the
  cost the result reports labelled as the session's total (a result's `total_cost_usd` is the CLI
  session's running total, not the turn's) and its number of turns, each when the result reports it; a figure the result omits is left out,
  not shown as zero. A failed turn's footer MUST show the error text the result carries.
- **FR-006**: A structured session with no events yet MUST show an empty transcript with a short
  note that there is no output yet.

**Cards**

- **FR-007**: Each tool use MUST be shown as one tool-call card, paired with its result by the tool
  use's identifier; the result MUST be shown inside that card, not as an entry of its own.
- **FR-008**: A tool-call card's header MUST show the tool's name, a one-line input summary and a
  status: running (no result yet), done, failed (the result is marked as an error), or no result
  (the turn ended without one).
- **FR-009**: The input summary MUST show the input field that identifies the work for the tools
  Claude Code uses most: the command for a shell tool, the file path for file read, write and edit
  tools, the pattern for search tools, the URL or query for web tools, and the description for a
  task tool; any other tool, or an input of an unexpected shape, MUST get a generic summary of its
  input. The summary MUST fit on one line, cut with an ellipsis when longer.
- **FR-010**: Tool-call cards and thinking cards MUST be folded when they first appear; the user
  MUST be able to unfold and fold each card individually from its header with the pointer and with
  the keyboard.
- **FR-011**: An unfolded tool-call card MUST show the full input and the full result text; an
  unfolded thinking card MUST show the full thinking text. A result whose content is a block list
  MUST show its text parts and name each part it cannot show.
- **FR-012**: A failed tool-call card and a failed turn footer MUST be distinguishable from
  successful ones by a text label and an icon in addition to colour, folded and unfolded, in both
  light and dark themes.
- **FR-013**: A card's folded or unfolded state MUST persist while the window stays open, across
  new events, switching panes or sessions, and a re-sync of the same session, for as long as the
  card is in the transcript.

**Streaming and scrolling**

- **FR-014**: While a turn is in progress, text, thinking and tool-input deltas MUST grow their
  entry in place as each arrives, without waiting for the finished message.
- **FR-015**: When the finished assistant message arrives, it MUST take the place of the entries its
  deltas built, with no duplicated, lost or reordered text, and no entry before it MUST change.
- **FR-016**: Adding or growing an entry MUST NOT change, reorder or blank any other entry, and MUST
  NOT change the position of content the user is looking at, except as follow-tail moves the end
  into view.
- **FR-017**: While the view is scrolled to its end, it MUST keep the end in view as content is
  added or grows (follow-tail). When the user scrolls away from the end, following MUST stop and a
  "jump to latest" control MUST be shown; activating it, or scrolling back to the end, MUST resume
  following and hide the control.
- **FR-018**: A transcript shown for the first time in a window MUST open at its end, following.

**Isolation, bounds and re-sync**

- **FR-019**: Each window MUST keep each session's transcript, scroll position, follow-tail state
  and card fold state separate from every other session's and every other window's (Principle II).
- **FR-020**: The transcript MUST stay within the same event bound as the plain log it replaces;
  when the window drops its oldest events, their entries MUST leave the top and the omitted notice
  MUST count them; a tool result whose tool use was dropped MUST be shown as a result without its
  call, not as an error.
- **FR-021**: A history that does not continue what the window holds MUST rebuild the transcript
  from that history with no duplicated entries, keeping fold state for cards still present.

**Rendering seam (for #940)**

- **FR-022**: The drawing of each block kind inside an entry (assistant text, thinking text, tool
  input, tool result) MUST go through one replaceable renderer per block kind, so #940 can replace
  the plain-text renderer of a kind with a markdown, code or diff renderer without changing how the
  transcript is built, ordered, folded, streamed or scrolled. In this feature every block renderer
  shows plain, wrapped text.

**Documentation**

- **FR-023**: The user guide's structured-sessions section MUST describe the transcript (entries,
  cards and how to fold them, failure marking, turn footers, follow-tail and the "jump to latest"
  control) in place of the plain log, and MUST keep listing what the later issues of the epic add, in
  the same change that ships the view.

**Sub-agent events (for #944)**

- **FR-024**: An event that names a parent tool use (produced inside a sub-agent) MUST NOT be
  shown as an entry of the main transcript, nor change any entry's text, status or footer. Each
  such message (not its deltas) MUST be counted on the tool-call card of the tool use it names, whose header MUST show that count
  while it is above zero; when no card with that identifier is in the transcript, it MUST be
  counted on one notice saying that many sub-agent messages are not shown. The events MUST stay in
  the window's history so #944 can nest them under the card.

### Constraints

- **Rendering seam**: per-block rendering sits behind the seam of FR-022 (one renderer per block
  kind); #940's markdown, highlighted code and Edit/Write diffs are out of scope here.
- **Shared UI components (Principle VIII)**: cards, footers and the "jump to latest" control are
  built from, or added to, the shared component library with its builder-style interface, not as
  one-off widgets of this view.
- **No input**: the view offers no prompt input, interrupt, permission answer or slash command
  (#941–#943).
- **Client only**: the events, history, wire and daemon of #938 are reused as they are; this
  feature adds no new event type and changes no protocol message.

### Out of Scope

- Markdown, syntax-highlighted code and Edit/Write diff rendering (#940).
- Prompt input, interrupt, queued messages (#941); permission prompts (#942); slash commands, model
  switching and falling back to the terminal mode (#943).
- Usage and context meters beyond the per-turn footer figures, the plan-usage feed, showing
  sub-agent events nested under their tool call, and background tasks (#944). This feature only
  counts sub-agent messages on their card (FR-024).
- Searching the transcript, copying text out of it, and exporting it.
- Remembering fold state or scroll position across application restarts.

### Key Entities

- **Transcript**: per window and per structured session, the ordered entries built from the
  session's events, with the omitted count and the position of the next expected event.
- **Entry**: one visible item (see Terms); text entries and cards hold the content of the block
  they show, built up from deltas until the finished message arrives.
- **Tool-call card**: a tool use's identifier, name, input, input summary, status and result
  (text, error flag), its count of sub-agent messages (FR-024), and its fold state.
- **Turn footer**: outcome (success or error), duration, the session's running cost, number of
  turns and error text, each as reported.
- **View state**: per window and session: scroll position, whether it follows the tail, and the
  fold state of each card.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: For every recorded fixture of #938, 100% of its events are accounted for in the
  transcript: each modelled block becomes its entry or card, each unmodelled event a raw row, and
  every tool use is one card holding its result; no event is dropped.
- **SC-002**: Replaying the partial-message fixture one event at a time, after each event every
  entry before the one being streamed is identical to before, and the final transcript is identical
  to the one built from the same turn without deltas (no duplicated or lost text).
- **SC-003**: In 100% of the follow-tail test cases, content added while at the end keeps the end
  in view, and content added while scrolled away moves the visible content by zero lines.
- **SC-004**: A transcript holding the full history bound of events (5,000), with every card unfolded,
  opens within 1 second in a release build on a CI runner, and while it scrolls or streams the view
  never stops responding to input
  for more than 100 ms.
- **SC-005**: In both themes, 100% of failed cards and failed footers in the reviewed screenshots
  are identifiable as failed in a greyscale copy, by their label and icon alone.
- **SC-006**: Every existing terminal-session test passes unchanged.

## Assumptions

- The event model, history, wire messages and bounds of #938 are sufficient; the transcript is
  built entirely in the window from the events the service already sends, as the plain log is
  today.
- User messages appear in the transcript only when the session reports them as events; this
  feature adds no way to send one (#941).
- Fold state and scroll position are per window and in memory only; they are not saved (out of
  scope above).
- Cards start folded, including failed ones; the failure marking on the folded header is enough to
  find them.
- A finished assistant message supersedes the deltas of the same message; deltas and the finished
  message of one assistant message are matched by their position in the stream, as the CLI reports
  them in order.
- Cost is shown in US dollars as the result reports it; duration in seconds (minutes and seconds
  above one minute).
- The input-summary rules of FR-009 cover Claude Code's built-in tools by name; tools from tool
  servers get the generic summary.
- The turn footer (duration, and the session's running cost as the result reports it, labelled as
  the session total) is this feature's; #944 builds on it and adds the per-turn cost delta, the
  context meter, the plan-usage feed and sub-agent nesting, and does not re-implement the footer.
