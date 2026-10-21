# Feature Specification: Structured Session Rich Rendering — markdown, highlighted code and Edit/Write diffs

**Feature Branch**: `claude/project-thread-x8n8q6`

**Created**: 2026-10-21

**Status**: Draft

**Input**: GitHub issue #940 "Structured session view: markdown, syntax-highlighted code and diffs
for Edit/Write". "Part of the structured-session (stream-json) UI epic. Keeps the PTY/TUI session
mode as the fallback throughout. Rich rendering in the transcript: markdown (headings, lists, inline
code, links, tables), highlighted code blocks (iced `highlighter`), and unified diffs for
Edit/MultiEdit/Write tool calls. Acceptance: Markdown subset renders correctly while streaming (no
layout jumps on partial blocks). Edit/Write shown as a diff with file path. Copy button on code
blocks." labels: enhancement, flow:feature

## Clarifications

### Session 2026-10-21

- Q: Do Edit, MultiEdit and Write cards start unfolded, so the diff shows without a click? → A: No:
  they start folded like every other card (#939 FR-010), and their folded header shows the file path
  and the added/removed line counts (FR-017); unfolding shows the diff. The fold policy stays #939's,
  so this feature's only change outside the block renderers is the header line counts (FR-022).
  _(agent-resolved: specs/939-structured-transcript-view/spec.md#assumptions, "Cards start folded,
  including failed ones"; #939 FR-010; issue #940 acceptance "Edit/Write shown as a diff with file
  path", which the unfolded card meets)_
- Q: Which tool results get rich rendering? → A: None: every tool result, including Read and Task
  (sub-agent) results, stays #939's plain monospace text (#939 FR-011). Issue #940 scopes rich
  rendering to markdown text, code blocks and Edit/MultiEdit/Write tool calls, and names no tool
  results; sub-agent output belongs to #944. _(agent-resolved: issue #940 body, "Rich rendering in
  the transcript: markdown …, highlighted code blocks …, and unified diffs for Edit/MultiEdit/Write
  tool calls"; specs/939-structured-transcript-view/spec.md#context-where-this-sits-in-epic-945,
  #944 "sub-agents")_

## In short (for non-technical readers)

The structured session transcript of #939 shows Claude's replies, thinking and tool calls as plain
text. Claude writes its replies in markdown, so today the user reads raw `#`, `**`, backticks and
pipe-drawn tables. This feature draws them as they are meant to look: headings, bold and italic,
lists, quotes, links, tables, and code blocks coloured by language with a button that copies the
code. It does so while Claude is still writing, without the text above jumping around as more
arrives. When Claude edits or writes a file, the tool card shows the change as a diff — the file's
path, the removed lines and the added lines — instead of the raw tool input. Nothing else about the
transcript changes, and terminal sessions stay the default and are unchanged.

## Context: where this sits in epic #945

| Issue | Delivers |
|---|---|
| #938 (merged) | The structured session kind, the event model, the daemon side, the wire, resume |
| #939 (design merged, code pending) | The read-only transcript view, its entries, cards, streaming, follow-tail, and the per-block renderer seam (its FR-022) |
| **#940 (this)** | Markdown, highlighted code blocks with copy, and Edit/MultiEdit/Write diffs, drawn through #939's renderer seam |
| #941–#944 | Input, permission prompts, slash commands, usage and sub-agents |

## Terms

- **Transcript, entry, card, streaming, block renderer**: as defined by #939
  (specs/939-structured-transcript-view/spec.md#terms).
- **Markdown subset**: the markdown this feature draws as formatting: headings, paragraphs, bold,
  italic, strikethrough, inline code, links, bullet and numbered lists (nested, and task-list
  checkboxes as read-only marks), block quotes, horizontal rules, fenced and indented code blocks,
  and tables. Anything else in the text (raw HTML, footnotes, images, math) is shown as its literal
  text, or for an image as its description and address.
- **Code block**: a fenced or indented code block in markdown text, with its language tag when the
  fence names one.
- **Open block**: while an entry is streaming, the last markdown block of its text, which more text
  may still extend or close (an unclosed code fence, a list still growing, a table still gaining
  rows). Every block before it is **settled**.
- **Edit diff**: the change an Edit, MultiEdit or Write tool call asks for, shown as removed and
  added lines under the file's path.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Read Claude's replies as formatted markdown, also while they stream (Priority: P1)

A user reads a structured session's transcript and sees Claude's replies formatted: headings stand
out, lists are indented with their bullets or numbers, inline code and links look like code and
links, and tables are drawn as tables. While Claude is writing, the reply is already formatted as it
grows, and the part already shown stays where it is.

**Why this priority**: Assistant text is most of what the user reads; raw markdown is the main
readability gap the issue names, and the streaming acceptance criterion applies here.

**Independent Test**: Feed a recorded fixture whose assistant text uses every element of the
markdown subset into the window's transcript state; check the formatted blocks it produces (kind,
level, text, link targets, table cells). Replay a streamed fixture one delta at a time and check
that every settled block is unchanged after each delta. Delivers formatted replies with no code
blocks or diffs needed.

**Acceptance Scenarios**:

1. **Given** a finished assistant reply with headings, bold and italic text, inline code, nested
   bullet and numbered lists, a block quote and a link, **When** it is shown, **Then** each is drawn
   as its formatting, and no markdown punctuation that served as formatting is visible.
2. **Given** a reply containing a table, **When** it is shown, **Then** it is drawn as a table with
   a header row and aligned columns, and a cell longer than its column wraps within the cell.
3. **Given** a reply being streamed, **When** each delta arrives, **Then** the reply grows in
   place, every settled block keeps its content and height, and only the open block changes.
4. **Given** a streamed reply whose open block is an unclosed code fence, a half-written table row
   or an unmatched `**`, **When** it is shown, **Then** it is drawn without failing and without
   hiding any received text, and the blocks before it do not change.
5. **Given** a link in a reply, **When** the user activates it, **Then** an `http`, `https` or
   `mailto` address opens in the system's default handler; any other address is shown as text and
   is not activatable.
6. **Given** a thinking card unfolded, **When** its text contains markdown, **Then** it is drawn
   with the same formatting as assistant text, in the thinking card's muted style.
7. **Given** a reply with markdown the subset does not cover (raw HTML, an image), **When** it is
   shown, **Then** that part appears as its literal text (an image as its description and address),
   and the image is not fetched.

---

### User Story 2 - See Edit, MultiEdit and Write tool calls as diffs (Priority: P1)

When Claude edits or writes a file, the user unfolds its tool card and sees the file's path and the
change as a unified diff — removed lines and added lines, coloured and marked — instead of the raw
input JSON with escaped newlines.

**Why this priority**: File changes are the most consequential tool calls to review, and the raw
input is unreadable; it is one of the issue's three acceptance criteria.

**Independent Test**: Feed fixtures with an Edit, a MultiEdit with several edits, a Write, and one
of each with input of an unexpected shape into the transcript state; check the diff produced for
each (file path, hunks, removed and added lines) and that malformed inputs fall back to #939's plain
input.

**Acceptance Scenarios**:

1. **Given** an Edit tool call, **When** its card is unfolded, **Then** it shows the file path and a
   diff in which the replaced text's lines are removed lines and the replacement's lines are added
   lines, with the lines both share shown as unchanged context.
2. **Given** a MultiEdit tool call with several edits, **When** its card is unfolded, **Then** it
   shows the file path once and one hunk per edit, in the order of the edits.
3. **Given** a Write tool call, **When** its card is unfolded, **Then** it shows the file path and
   the written content as added lines, labelled as the file's new content.
4. **Given** an Edit that replaces every occurrence, **When** its diff is shown, **Then** it says so.
5. **Given** an Edit, MultiEdit or Write card, **When** it is folded, **Then** its header shows the
   number of added and removed lines next to #939's input summary.
6. **Given** a diff, **When** it is shown, **Then** each added and removed line is marked by a `+`
   or `−` sign as well as by colour, in both light and dark themes, and the lines are highlighted for
   the file's language when its extension names a known one.
7. **Given** an Edit, MultiEdit or Write whose result is an error, **When** its card is shown,
   **Then** the diff of the requested change is still shown, and the card is marked failed as #939
   draws it.
8. **Given** an Edit, MultiEdit or Write whose input is still streaming, **When** its card is
   unfolded, **Then** it shows the input as #939 does until the input is complete, then the diff in
   its place.
9. **Given** an Edit, MultiEdit or Write whose input lacks the expected fields or has fields of the
   wrong kind, **When** its card is unfolded, **Then** it shows the input as #939 does, without an
   error.

---

### User Story 3 - Read highlighted code blocks and copy them (Priority: P2)

Code blocks in Claude's replies are drawn in a monospace font on their own surface, coloured for the
language their fence names, with a copy button that puts exactly the code on the clipboard.

**Why this priority**: It completes the issue's rendering and its copy acceptance criterion, and
builds on story 1's markdown blocks.

**Independent Test**: Feed a reply with code blocks tagged with a known language, an unknown
language and no language; check the highlighted spans for the known one and plain monospace for the
others; activate each copy button and check the clipboard text equals the block's code.

**Acceptance Scenarios**:

1. **Given** a code block whose fence names a known language, **When** it is shown, **Then** it is
   drawn in monospace with that language's highlighting, in colours readable in the current theme.
2. **Given** a code block with an unknown language tag or none, **When** it is shown, **Then** it
   is drawn in monospace without highlighting, and shows the tag when there is one.
3. **Given** any code block, **When** the user activates its copy button with the pointer or the
   keyboard, **Then** the clipboard holds the block's code exactly — no fence, no language tag,
   line breaks and indentation kept — and the button confirms the copy for a moment.
4. **Given** a code block still streaming, **When** the user activates its copy button, **Then**
   the clipboard holds the code received so far.
5. **Given** a code block wider than the transcript, **When** it is shown, **Then** its lines are
   not wrapped and it scrolls sideways within its own surface, without the transcript scrolling
   sideways.
6. **Given** the theme switches between light and dark, **When** the transcript is shown, **Then**
   code highlighting and diff colours follow the new theme.

---

### Edge Cases

- **Empty text**: an empty assistant text or thinking block draws nothing and takes no extra height;
  an empty code block draws an empty code surface with its copy button (copying the empty string).
- **Unbalanced markdown while streaming**: an unclosed fence is drawn as a code block that is still
  open; unmatched emphasis markers are drawn as literal characters until they close; a table whose
  separator row has not arrived is drawn as text until it does. None fails, none hides received
  text, and none changes a settled block.
- **Settled blocks are final**: once a block is settled, later text never changes its kind, content
  or height. A trailing line that may still become something else (a `---` that may be a heading
  underline, a list item that may gain a continuation) belongs to the open block.
- **Finished message replacing the streamed text** (#939 FR-015): when the finished text equals the
  streamed text, the drawn blocks are unchanged; nothing is re-laid out.
- **Very large content**: a very long reply, a code block of thousands of lines, or a Write of a
  large file stays scrollable and responsive (SC-004); a diff or code block over a size limit shows
  its line counts and a control to show it in full, as the Changes view does, rather than blocking
  the view.
- **Edit whose old and new text are identical**, or a Write of an empty file: the diff says there is
  no change, or that the file is written empty, rather than drawing nothing.
- **Line endings and trailing newline**: Windows line endings and a missing final newline do not
  produce spurious changed lines; a change in only the final newline is shown as such.
- **Unknown file extension** in a diff, or a language highlighting does not know: shown
  unhighlighted, never an error.
- **Malicious or odd link targets** (`javascript:`, `file:`, relative paths): shown as text, not
  activatable.
- **Clipboard unavailable** (e.g. a Wayland compositor that refuses the write): the copy button does
  not claim success; the failure is shown on the button, and nothing else changes.
- **Several windows and sessions (Principle II)**: two windows showing the same session draw the
  same formatting; a copy confirmation, a "show in full" choice and every rendering cache are per
  window and per session, and never appear in or affect another window or session.
- **Session deleted or forgotten, history dropped or rebuilt** (#939 FR-020, FR-021): everything this
  feature derived for the dropped entries is dropped with them; a rebuilt transcript draws the same
  formatting as before.
- **Cross-platform (Principle VI)**: formatting, highlighting, diffs, copy and opening links behave
  the same on Linux, macOS and Windows; copy uses each platform's clipboard and links each
  platform's default handler.

## Requirements *(mandatory)*

### Functional Requirements

**Markdown**

- **FR-001**: Assistant text blocks MUST be drawn as formatted markdown covering the markdown subset
  (see Terms); a part of the text outside the subset MUST be shown as its literal text, and no
  received text may be hidden.
- **FR-002**: Unfolded thinking cards MUST draw their text with the same markdown formatting as
  assistant text, keeping the thinking card's muted style.
- **FR-003**: Tables MUST be drawn with a header row, column alignment as the separator row
  specifies, and cell text wrapping within its column when the table is wider than the entry.
- **FR-004**: Links with an `http`, `https` or `mailto` address MUST be activatable with the pointer
  and the keyboard and open in the system's default handler; links with any other address MUST be
  shown as text and MUST NOT be activatable. Images MUST NOT be fetched; an image is shown as its
  description and address.

**Streaming**

- **FR-005**: While an entry streams, it MUST be drawn as formatted markdown as it grows, not as
  plain text until it finishes.
- **FR-006**: A settled block MUST NOT change its kind, content or height as later text arrives; only
  the open block may change. Text that may still change the kind of the block it belongs to MUST be
  held in the open block.
- **FR-007**: Partial or unbalanced markdown in the open block (an unclosed fence, unmatched emphasis,
  a table without its separator row) MUST be drawn without failing and without hiding received text.
- **FR-008**: When the finished message replaces the streamed text with the same text (#939 FR-015),
  the drawn blocks MUST be identical to those drawn from the streamed text.

**Code blocks**

- **FR-009**: Code blocks MUST be drawn in a monospace font on a distinct surface, with the
  highlighting of the language their fence names when it is a known language, and unhighlighted
  otherwise; a language tag that is present MUST be shown on the block.
- **FR-010**: Code block lines MUST NOT be wrapped; a code block wider than the entry MUST scroll
  sideways within its own surface.
- **FR-011**: Every code block MUST have a copy control, operable with the pointer and the keyboard,
  that puts the block's code on the clipboard exactly (without fence or tag, with line breaks and
  indentation kept), including the code received so far while it streams.
- **FR-012**: After a copy, the control MUST confirm success for a short time, or show that the copy
  failed when the clipboard refused it; the confirmation MUST be per window and per block.

**Edit, MultiEdit and Write diffs**

- **FR-013**: An unfolded Edit, MultiEdit or Write tool card whose input is complete and of the
  expected shape MUST show the file's path and a unified diff of the requested change in place of
  the raw input: for an Edit, the replaced text as removed lines and the replacement as added lines
  with their shared lines as context; for a MultiEdit, one hunk per edit in order; for a Write, the
  content as added lines labelled as the file's new content.
- **FR-014**: An Edit or MultiEdit edit that replaces every occurrence MUST say so on its hunk.
- **FR-015**: A diff MUST mark added and removed lines by a sign as well as by colour, in both light
  and dark themes, and MUST highlight lines for the file's language when its extension names a known
  one.
- **FR-016**: The diff MUST NOT show file line numbers it cannot know from the tool call.
- **FR-017**: A folded Edit, MultiEdit or Write card's header MUST show the number of added and
  removed lines, in addition to #939's input summary (which already names the file path). This
  extends #939's input-summary rule (its FR-008, FR-009) for these three tools; it is the one change
  this feature makes outside the block renderers (FR-022).
- **FR-018**: While the input is incomplete, or when it lacks the expected fields or has fields of
  the wrong kind, the card MUST show the input as #939 shows it, and MUST NOT show an error. A failed
  result MUST NOT hide the diff.
- **FR-019**: A diff or code block over a size limit MUST show its line counts and a control to show
  it in full, rather than make the view unresponsive.
- **FR-020**: Edit, MultiEdit and Write cards MUST start folded, as every card does (#939 FR-010);
  this feature does not change #939's fold policy.

**Tool results**

- **FR-021**: Tool results MUST stay drawn as #939 draws them (plain monospace text, its FR-011),
  including Read and Task (sub-agent) results; this feature gives no tool result rich rendering.

**Scope of change and seam**

- **FR-022**: All rendering of this feature MUST go through #939's per-block renderer seam (its
  FR-022): it replaces the renderers of block kinds and changes nothing about which entries exist,
  their order, folding, status, streaming or follow-tail. The only exception is the header line
  counts of FR-017.
- **FR-023**: While one entry streams, every other entry MUST keep its drawing unchanged and the view
  MUST stay responsive to input (SC-004); a theme switch MUST redraw highlighting and diff colours in
  the new theme (US3 scenario 6).

**Documentation**

- **FR-024**: The user guide's structured-sessions section MUST describe the formatted replies, code
  blocks and their copy control, Edit/MultiEdit/Write diffs, which links open and which do not, in
  the same change that ships them.

### Constraints

- **Depends on #939's code**: this feature plugs into #939's transcript and its block renderer seam
  (specs/939-structured-transcript-view/contracts/block-renderers.md, R-1 to R-5), and uses its
  tool card name and input, result parts, and entry key plus revision as its cache key
  (contracts/transcript-model.md, "Seams for later issues"). #939's milestones must have landed
  before this feature's code can. This feature changes #939's behaviour only where FR-017 (header
  line counts, an extension of #939 FR-009) says.
- **Shared UI components (Principle VIII)**: the markdown view, the code block with its copy
  control, and the inline diff are shared components with the builder interface, reusing the
  existing diff view and highlighting of the Changes view where they fit, not one-off widgets of
  the transcript.
- **Client only**: no new event type, no protocol message and no daemon change.
- **Local-first (Principle IV)**: nothing is fetched to render the transcript; a link opens only on
  the user's explicit activation.

### Out of Scope

- Rich rendering of tool inputs other than Edit, MultiEdit and Write, and of any tool result,
  including Read and Task results (FR-021).
- Applying, reverting or opening a diffed file in an editor; comparing against the file's content on
  disk.
- Selecting or copying arbitrary transcript text, search and export (as in #939); copy is only the
  code block control.
- Images, math, diagrams (mermaid) and raw HTML rendering.
- Input, permission prompts, slash commands, usage and sub-agent nesting (#941–#944).
- Terminal sessions, which stay unchanged.

### Key Entities

- **Rendered text**: per window, session, entry and revision, the markdown blocks derived from a
  text block — headings, paragraphs with styled spans and links, lists, quotes, rules, code blocks,
  tables — and which of them are settled.
- **Code block**: its code, language tag, highlighted spans (when the language is known) and copy
  state (idle, copied, failed) per window.
- **Edit diff**: the file path, the hunks (one per edit, each with its replace-every-occurrence flag),
  each line's kind (context, added, removed) and highlighted spans, and the added/removed line
  counts.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: For a fixture covering every element of the markdown subset, 100% of elements are drawn
  as their formatting and 100% of the received characters that are not formatting punctuation appear
  in the drawn text.
- **SC-002**: Replaying a streamed fixture one delta at a time, after each delta 100% of the settled
  blocks are identical (kind, content, height) to before, and the final drawing equals the one built
  from the finished message alone.
- **SC-003**: For every Edit, MultiEdit and Write call in the recorded fixtures, the diff shows the
  file path, and applying its removed and added lines to the replaced text reproduces the
  replacement exactly; every malformed input falls back to the plain input with no error.
- **SC-004**: A transcript holding the full history bound of events (5,000) with every card unfolded
  and markdown, code and diffs in its entries still opens within 1 second in a release build on a CI
  runner, and while it scrolls or streams the view never stops responding to input for more than
  100 ms (#939's SC-004, unchanged).
- **SC-005**: In 100% of copy test cases, the clipboard text equals the code block's code byte for
  byte.
- **SC-006**: In both themes, 100% of added and removed lines in the reviewed screenshots are
  identifiable in a greyscale copy by their sign alone, and highlighted code is readable (text
  contrast at least 4.5:1 against its surface).
- **SC-007**: Every existing terminal-session and #939 transcript test passes unchanged, except tests
  that assert plain-text drawing of the block kinds this feature replaces.

## Assumptions

- #939's design is implemented as specified, in particular its renderer seam (one replaceable
  renderer per block kind, renderers may add messages for their own content, a per-window cache keyed
  by session, entry key and revision, filled in update and pruned with the transcript). #939's R-3
  allows drawing a streaming block as plain text until it finishes; this feature draws it formatted
  instead (FR-005), which R-3 permits.
- The markdown flavour is CommonMark with the common GitHub extensions for tables, strikethrough and
  task lists, as Claude Code writes.
- The set of known languages for highlighting is the one the application's existing highlighting
  (the Changes view's) supports; fence tags are matched to it by name and common aliases (`rs`,
  `py`, `sh`, `ts`), and file extensions in diffs the same way.
- Edit and MultiEdit inputs give only the replaced and replacement text, not their position in the
  file, so the diff is of that text (FR-016); a Write's previous content is unknown, so it is shown
  as new content.
- The tool input field names are Claude Code's: `file_path`, `old_string`, `new_string`,
  `replace_all`, `edits` and `content`. A tool from a tool server with the same name but a different
  input shape falls back to the plain input (FR-018).
- The size limits of FR-019 reuse the Changes view's limits where they apply.
- The copy confirmation lasts about two seconds.
