# Feature Specification: Structured View — Select and Copy Transcript Text

**Feature Branch**: `claude/project-thread-zj503u`

**Created**: 2026-10-27

**Status**: Draft

**Input**: GitHub issue #1550 "Structured view: text and values cannot be selected or copied"
(labels: bug, flow:bug; switched to flow:feature). In the structured session view, transcript text
(assistant messages, thinking, code blocks, tool-call inputs and outputs, paths, commands, values,
diffs) cannot be selected with the mouse or copied; the terminal view can. Expected: drag-selecting
highlights text across lines and blocks, Ctrl+C / Cmd+C copies the selection, and individual values
can be copied with a copy button. Ctrl+C in the structured view also interrupts a busy turn
(#1403); with a selection active it must copy instead.

**Builds on**: `specs/939-structured-transcript-view` (the transcript, its entries, folding,
follow-tail; it listed "copying text out of it" as out of scope), `specs/940-transcript-rich-rendering`
(markdown, code blocks with their copy control FR-011/FR-012, diffs), and
`specs/941-structured-prompt-input` (FR-012/FR-012a: Ctrl+C interrupts a busy session unless text
is selected; BUG-1403).

## Terms

- **Transcript, entry, tool-call card, thinking card, turn footer, follow-tail**: as defined in
  `specs/939-structured-transcript-view/spec.md#terms`.
- **Selectable text**: every piece of text the transcript currently draws as content: user
  messages, assistant text (as rendered by #940), unfolded thinking text, tool-call card headers
  (tool name and input summary), unfolded tool input and result text, diff lines and their file
  path, code block code, turn footers, notices and raw rows. Not selectable: the labels and icons of
  controls (fold toggles, copy controls, "jump to latest", permission-prompt buttons), images, and
  text hidden by a folded card.
- **Selection**: one contiguous range of selectable text in reading order (top to bottom, then left
  to right within a line), from the point where a drag started (the anchor) to where it is now or
  ended (the focus). A selection may start and end in different entries.
- **Copy control**: a small control on a value that puts exactly that value on the clipboard and
  confirms the copy, the way #940 FR-011/FR-012 already do for code blocks.
- **Copy keys**: Ctrl+C and Ctrl+Shift+C on Linux and Windows; Cmd+C on macOS, and Ctrl+C on macOS
  while a selection exists.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Drag to select transcript text and copy it with the keyboard (Priority: P1)

A user reads a reply in a structured session and wants to reuse part of it: a sentence, a few lines
of a tool's output, or a span that runs from the end of one message through a tool card into the
next message. They press the pointer down on the text, drag, and the text between becomes
highlighted, across lines and entries. They press Ctrl+C (Cmd+C on macOS) and paste the text
elsewhere.

**Why this priority**: It is the core of the issue: today nothing in the transcript can be selected
or copied, so a user has to retype commands, paths and output by hand or switch to the terminal
view.

**Independent Test**: In a structured session with an assistant message, an unfolded tool-call card
with a result and a second assistant message, drag from the middle of the first message to the
middle of the second; check the highlight covers exactly that range, press the copy key and check
the clipboard holds that text in reading order with line breaks between lines and entries.

**Acceptance Scenarios**:

1. **Given** a transcript with an assistant message, **When** the user presses on a word and drags
   to a later word in the same message, **Then** exactly the characters between the two points are
   highlighted while dragging and stay highlighted after release.
2. **Given** a selection that starts in one entry and ends in another, **When** the user presses the
   copy key, **Then** the clipboard holds the selected text of every entry in between, in reading
   order, with each entry's text separated from the next by a line break, and without the labels of
   any control in between.
3. **Given** a selection that crosses a folded tool-call card, **When** it is copied, **Then** the
   card contributes only its visible header text (tool name and input summary), never its hidden
   input or result.
4. **Given** a drag that reaches the top or bottom edge of the transcript, **When** the pointer stays
   there, **Then** the transcript scrolls in that direction and the selection keeps growing.
5. **Given** a selection exists, **When** the user clicks once in the transcript without dragging,
   **Then** the selection is cleared and the clipboard is not changed.
6. **Given** a selection inside a code block wider than its surface (scrolling sideways, #940
   FR-010), **When** whole lines are selected and copied, **Then** the clipboard holds the full
   lines, including the parts scrolled out of sight.

---

### User Story 2 - Copy a selection instead of interrupting the turn (Priority: P1)

While Claude is working, the user selects a command from an earlier tool card and presses Ctrl+C to
copy it. The command is copied and the turn keeps running. When nothing is selected, Ctrl+C keeps
interrupting the busy turn as #1403 made it do.

**Why this priority**: Without this rule, the most common copy key would stop the agent's work every
time the user tries to copy during a turn, which makes selection unsafe to use while busy.

**Independent Test**: Start a turn, select text in the transcript, press Ctrl+C: the clipboard holds
the selection and the session is still busy. Clear the selection and press Ctrl+C: the turn is
interrupted.

**Acceptance Scenarios**:

1. **Given** a busy session and a non-empty transcript selection, with the transcript focused,
   **When** the user presses Ctrl+C, **Then** the selection is copied, the selection stays, and no
   interrupt is sent.
2. **Given** a busy session, the transcript focused and no transcript selection, **When** the user
   presses Ctrl+C, **Then** the turn is interrupted (#941 FR-012a), and nothing is copied.
3. **Given** an idle session, the transcript focused and no selection, **When** the user presses
   Ctrl+C, **Then** nothing happens: no interrupt, no clipboard change.
4. **Given** a transcript selection and the composer focused, **When** the user presses Ctrl+C,
   **Then** the key acts on the composer as #941 FR-012a says (copy the composer's selection, or
   interrupt a busy turn when the composer has none); the transcript selection is not copied.
5. **Given** macOS, **When** the user presses Cmd+C with a transcript selection, **Then** it is
   copied whether or not the session is busy; Cmd+C never interrupts.
6. **Given** a transcript selection, **When** focus moves to the composer, **Then** the transcript
   selection stays highlighted, Ctrl+C acts on the composer (scenario 4), and choosing **Copy** from
   the transcript's context menu still copies the transcript selection (FR-019a).

---

### User Story 3 - Copy a single value with one click (Priority: P2)

The user wants the exact command a Bash tool ran, the full path a Read or Edit touched, a tool's
full input, or its whole output, without dragging over long text. They activate a copy control next
to the value and the exact value is on the clipboard.

**Why this priority**: The issue names copy buttons for individual values; dragging over a long tool
result or a path shortened in a one-line summary is error-prone, but selection already gives a way
to copy, so this is second.

**Independent Test**: Unfold a Bash tool card and an Edit tool card; activate the copy control on the
command, on the tool input, on the result and on the diff's file path; check each time that the
clipboard holds exactly that value and that the control confirms the copy.

**Acceptance Scenarios**:

1. **Given** a tool-call card whose header shows an input summary, **When** the user activates the
   header's copy control, **Then** the clipboard holds the full identifying value the summary is
   drawn from (#939 FR-009: the command, path, pattern or URL), not the shortened summary.
2. **Given** an unfolded tool-call card with input, **When** the user activates the input's copy
   control, **Then** the clipboard holds the full input text exactly as the card shows it unfolded.
3. **Given** an unfolded tool-call card with a result, **When** the user activates the result's copy
   control, **Then** the clipboard holds the full result text exactly, including any part not in
   view.
4. **Given** an Edit, MultiEdit or Write diff (#940), **When** the user activates the copy control on
   its file path, **Then** the clipboard holds the full path.
5. **Given** any copy control, **When** the user reaches it with the keyboard and activates it,
   **Then** it copies exactly as with the pointer.
6. **Given** a tool card still running (input or result still streaming), **When** the user
   activates a copy control on it, **Then** the clipboard holds the value received so far.

---

### User Story 4 - Select by word, line and everything; copy from a context menu (Priority: P3)

The user double-clicks a word to select it, triple-clicks to select a whole line, presses Ctrl+A
(Cmd+A on macOS) in the focused transcript to select everything it shows, or right-clicks a
selection and chooses **Copy** from a menu, as they do in the terminal view and most applications.

**Why this priority**: These are conveniences on top of drag selection and the copy keys; the
feature is usable without them.

**Independent Test**: Double-click a word in an assistant message and check exactly the word is
selected; triple-click and check the line is; press Ctrl+A and check every selectable text is
highlighted; right-click and choose Copy, and check the clipboard.

**Acceptance Scenarios**:

1. **Given** a word in any selectable text, **When** the user double-clicks it, **Then** exactly
   that word is selected.
2. **Given** a line of selectable text, **When** the user triple-clicks it, **Then** the whole line
   is selected (one visual line of wrapped prose; one source line in a code block or diff).
3. **Given** the transcript is focused, **When** the user presses Ctrl+A (Cmd+A on macOS), **Then**
   all selectable text of the transcript is selected; the composer's text is not.
4. **Given** a selection, **When** the user right-clicks anywhere inside the transcript, **Then** a
   menu offers **Copy** and **Select all**, the selection is unchanged, and choosing **Copy** copies
   it. With no selection, **Copy** is shown disabled.

---

### Edge Cases

- **Empty transcript** (no events yet, #939 FR-006): dragging selects nothing; the copy keys and
  Ctrl+A do nothing and the clipboard is unchanged; the context menu's **Copy** is disabled.
- **Empty values**: an empty tool input or result shows its copy control, and copying it puts the
  empty string on the clipboard (as #940 does for an empty code block).
- **Clipboard refuses the write** (the system clipboard is unavailable or rejects it): a copy
  control shows that the copy failed (#940 FR-012); a copy by key or menu shows a short notice in
  the pane that the copy failed; the selection stays so the user can retry; nothing is interrupted.
- **Content streams in while a selection exists**: new entries and growing entries below or inside
  the selection do not clear, move or change the selected characters; the selection stays on the
  same text (#939 FR-016). Follow-tail continues to behave as #939 FR-017 says; selecting does not
  start or stop following on its own.
- **Selected entry is replaced**: when the finished assistant message takes a partial entry's place
  (#939 FR-015) and the selected text changes as a result, the selection is cleared; if the selected
  text is unchanged, it stays.
- **Folding**: folding or unfolding a card that holds any part of the selection clears the selection;
  pressing on a fold toggle, copy control or other control never starts a selection.
- **Transcript rebuilt** (#939 FR-021: a history that does not continue the window's transcript):
  the selection is cleared.
- **Very large selection** (Ctrl+A on a transcript at its event bound, #939 FR-020): the highlight,
  the copy and the scroll stay responsive (SC-003).
- **Focus leaves the transcript**: the selection stays highlighted; the copy keys and select-all
  act on whatever now has focus (FR-019a); the context menu still copies the transcript selection.
- **Multi-session (Principle II)**: each pane keeps its own selection for the session it shows; a
  selection in one session's transcript never highlights, copies or blocks the interrupt of another
  session's pane, in the same window or another. The copy key acts only on the focused pane. When a
  pane switches to another session, its selection is cleared.
- **Same session in two windows**: each window has its own selection, as each has its own scroll
  position (#939 FR-019).
- **Permission prompts and other interactive entries** (#942): their buttons and inputs are not
  part of a selection; a selection crossing them copies only their descriptive text.
- **Cross-platform (Principle VI)**: selection, highlighting, word/line selection, the context menu,
  copy controls and the copy keys behave the same on Linux, macOS and Windows; only the modifier
  differs (Cmd on macOS, Ctrl elsewhere), as defined in Terms. Copied text is identical on every
  platform.
- **Wide (CJK), emoji and combining characters**: selection boundaries never split a character
  (a grapheme cluster); the copied text holds whole characters only, on every platform.

## Requirements *(mandatory)*

### Functional Requirements

**Selecting**

- **FR-001**: Pressing the pointer on selectable text and dragging MUST select the text between the
  press point and the pointer, highlighted while dragging and after release, within one line,
  across lines, and across entries.
- **FR-002**: A selection MUST cover exactly the selectable text between its anchor and focus in
  reading order; text hidden by a folded card and the labels of controls MUST NOT be part of it.
- **FR-003**: Dragging past the top or bottom edge of the transcript MUST scroll the transcript in
  that direction while the pointer stays there, extending the selection.
- **FR-004**: A single click in the transcript without dragging MUST clear any selection and MUST
  NOT change the clipboard. Pressing on a control (fold toggle, copy control, link, "jump to latest",
  permission-prompt control) MUST operate the control and MUST NOT start a selection.
- **FR-005**: Double-click MUST select the word under the pointer; triple-click MUST select the
  line under the pointer (a visual line in wrapped prose, a source line in code blocks and diffs).
- **FR-006**: With the transcript focused, Ctrl+A (Cmd+A on macOS) MUST select all selectable text
  of that transcript.
- **FR-007**: Pressing in the transcript to select MUST give the transcript keyboard focus, so that
  the copy keys act on the transcript selection.

**Copying**

- **FR-008**: With the transcript focused and a non-empty transcript selection, the copy keys MUST
  put the selected text on the system clipboard, keep the selection, and MUST NOT interrupt the
  session, whether it is busy or idle.
- **FR-009**: With the transcript focused and no transcript selection, Ctrl+C MUST keep the
  behaviour of #941 FR-012/FR-012a (interrupt a busy session; nothing while idle). Cmd+C on macOS
  and Ctrl+Shift+C MUST never interrupt.
- **FR-010**: Copied text MUST be the selected text as shown: each visual line break that is a real
  line break in the content, and each boundary between entries, becomes one line break; soft
  wrapping of long prose lines MUST NOT add line breaks; code and tool output keep their
  indentation; diff lines are copied without line numbers and without the added/removed markers
  in their gutter.
- **FR-011**: Rendered markdown MUST be copied as
  [NEEDS CLARIFICATION: the visible rendered text (no `**`, `#`, list markup or link targets), or
  the markdown source of the selected span?].
- **FR-012**: Releasing the pointer at the end of a drag
  [NEEDS CLARIFICATION: must also copy the selection to the clipboard automatically, as the
  terminal view does, or must leave the clipboard unchanged until a copy key, menu or control is
  used?].
- **FR-013**: A right-click anywhere inside the transcript, on text or on a control, MUST open a
  menu with **Copy** (enabled only while a selection exists; copies it) and **Select all** (as
  FR-006); the right-click itself MUST NOT change or clear the selection, wherever it lands. The menu
  MUST be operable with the keyboard and close on Escape or a click elsewhere.
- **FR-014**: A copy by key or menu that the clipboard refuses MUST show a short notice in that pane
  that the copy failed, and MUST keep the selection.

**Copy controls**

- **FR-015**: A tool-call card's header MUST have a copy control that copies the full identifying
  value its input summary is drawn from (#939 FR-009), for every tool whose summary shows one.
- **FR-016**: An unfolded tool-call card MUST have a copy control for its full input and, once a
  result exists, one for its full result, each copying the exact text the card shows unfolded,
  including text received so far while it streams.
- **FR-017**: An Edit, MultiEdit or Write diff (#940 FR-013) MUST have a copy control for its file
  path.
- **FR-018**: Every copy control MUST be operable with the pointer and the keyboard and MUST confirm
  success or show failure as #940 FR-012 specifies for code blocks; the confirmation MUST show only
  on the control that was activated, in the window where it was activated. The existing code-block
  copy control (#940 FR-011) is kept as it is.

**Scope of a selection**

- **FR-019**: A selection MUST belong to one pane and the session it shows in one window; it MUST be
  cleared when the pane shows another session, when the transcript is rebuilt (#939 FR-021), when a
  card holding part of it is folded or unfolded, and when the text it covers is replaced by
  different text (#939 FR-015). Streaming that adds or grows other text MUST NOT change it.
- **FR-019a**: When keyboard focus leaves the transcript (to the composer, another pane or other
  app chrome), the transcript selection MUST stay highlighted and is not cleared; the copy keys and
  Ctrl+A / Cmd+A then act on the newly focused target, not on the transcript selection. The
  selection can still be copied from the context menu (FR-013), which gives the transcript focus
  again without clearing it.
- **FR-020**: In the light and dark themes and in every entry kind (including code surfaces, diff
  lines and failed-card styling), the selection highlight MUST be visibly distinct from the
  unselected background around it, and selected text MUST keep a contrast ratio of at least 4.5:1
  against the highlight.

**Documentation**

- **FR-021**: The user guide's structured-sessions section MUST describe selecting (drag, double-
  and triple-click, select all, context menu), the copy keys and their arbitration with the
  interrupt key per platform, and the copy controls; the transcript's out-of-scope note about
  copying in `specs/939-structured-transcript-view` no longer applies.

### Key Entities

- **Transcript selection**: per window and per pane: the session it belongs to, its anchor and focus
  positions in the transcript's selectable text, and whether a drag is in progress. Empty when
  anchor equals focus.
- **Copyable value**: a value with its own copy control: a tool card's identifying value, its full
  input, its full result, a diff's file path, a code block's code (#940), with its per-window copy
  confirmation state.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: For each kind of selectable text named in Terms, a user can select and copy a span of
  it, and the pasted text matches the shown text character for character (per FR-010/FR-011), on
  Linux, macOS and Windows.
- **SC-002**: In 100% of attempts, a copy key pressed with a non-empty transcript selection during a
  busy turn copies the selection and leaves the turn running; with no selection, Ctrl+C during a
  busy turn interrupts it, as before this feature.
- **SC-003**: On a transcript at its event bound, the highlight follows the pointer without visible
  lag (within 100 ms of pointer movement), and select-all followed by copy completes within 1
  second.
- **SC-004**: Every copy control puts exactly its value on the clipboard (byte-for-byte equal to the
  value the card holds) and confirms within 1 second.
- **SC-005**: Streaming new content for a full turn while a selection exists leaves the selected
  text unchanged and still highlighted in 100% of runs, unless FR-019 says it is cleared.
- **SC-006**: In both themes and every entry kind, selected text measures a contrast ratio of at
  least 4.5:1 against the selection highlight, and the highlight is visibly distinguishable from the
  unselected background in a screenshot comparison.

## Assumptions

- Selection is cross-block (one range across entries), not per block: the issue expects selection
  "across lines and blocks", and the terminal view behaves that way.
- One selection per pane at a time; the composer keeps its own separate selection (#941), and the
  copy key acts on whichever of the two has focus.
- Ctrl+Shift+C is accepted as a copy key on Linux and Windows because the terminal view uses it
  (user guide, "Interacting with the terminal"); it never interrupts.
- On macOS, Ctrl+C with a selection copies, for consistency with #941 FR-012a ("with a selection,
  Ctrl+C copies it and does not interrupt").
- Copied text uses line feeds as line breaks on all platforms, as #940's code-block copy does.
- Diff gutter markers (+/-) and line numbers are presentation, not content, so they are left out of
  copied text; the diff's file path has its own copy control.
- No "copy whole message" control is added: selecting the message (or triple-click / select all)
  covers it.
- The existing clipboard access used by #940's code-block copy is reused; no new platform
  dependency.

## Out of Scope

- Searching the transcript and exporting it (still out of scope from #939).
- Copying images or attachments, and rich-text (HTML/RTF) clipboard formats; only plain text is
  copied.
- Selecting text in the composer, the session bar or other panes (the composer already has its own
  selection, #941).
- Changing the terminal view's selection or copy behaviour.
- Keyboard-only selection (Shift+arrow keys moving a caret through the transcript); the transcript
  has no caret.
- Remembering a selection across application restarts.
