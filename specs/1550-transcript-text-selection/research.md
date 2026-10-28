# Research: Structured View — Select and Copy Transcript Text (#1550)

Each item: **Decision**, **Rationale**, **Alternatives considered**. Names marked *(new)* do not
exist yet; every other path and type was checked against `main` at the plan's base (783f9af1d).

## R1. Where the selection lives, and what a position is (FR-001, FR-002, FR-019, Principle II)

**Decision**: A render-free core module `micold_core::structured::selection` *(new)* owns the model:
a `TextPos { row: RowPos, path: TextPath, offset: usize }` (`offset` a byte offset into the
unit's shown text, always on a grapheme boundary), a `RowPos` *(new)* naming the transcript row
(`Omitted | Entry(EntryId, Under { Body | Prompt(k) }) | End(k)`, data-model.md), a `TextPath`
*(new)* (a short `Vec<u16>` whose lexicographic order is drawing order inside the row), and
`Selection { anchor, focus, dragging }`. `TextPos` derives `Ord` field by field, so reading order
is the order of `(RowPos, TextPath, offset)`. `RowPos` exists because not every drawn row has an
`EntryId` (`ui/transcript/mod.rs:55-61` `RowKey::Omitted`, `RowKey::Prompt(PromptId)`, and request
blocks drawn under an entry by `entries::with_trailing`), yet the spec makes notices and the
descriptive text of permission prompts selectable (*Terms*, Edge Cases *Permission prompts*). Its
variant order is the drawing order: the omitted notice first, each entry's body then the rows under
it, then the request blocks at the end, those by their index as drawn (a `PromptId`'s order is not
drawing order). The window keeps one `Selection` per session it shows, in a render-free client module
`features/transcript_selection.rs` *(new)*, beside `features/structured_transcript.rs`'s per-session
view state (fold set, follow).

**Rationale**: `EntryId` is already the entry's position in one build of the transcript
(`crates/micold-core/src/structured/transcript/mod.rs:276`: `EntryId(self.first_id +
self.entries.len())`, "monotonic, so the entry list is addressed by `id - first id`"), so it orders
entries in reading order in O(1) with no new index, and it is stable while entries grow or are
appended — exactly FR-019's "streaming that adds or grows other text MUST NOT change it". A rebuild
issues new ids, and FR-019 clears the selection on a rebuild anyway. Ordering inside an entry by a
path the renderer assigns in drawing order (block, list item, table row and cell, line) needs no
second ordered list of the whole transcript to draw a highlight: each drawn unit compares its own
`(EntryId, TextPath)` with the two endpoints. A pane shows a session at most once per window
(`crates/micold-core/src/pane_layout.rs:478` "never a duplicate"), so per window and session is per
pane (FR-019, spec *Multi-session*).

**Alternatives considered**: (a) Reusing the terminal's `crates/micold-client/src/selection.rs`
(`LineId` + cell column): the transcript has no grid, no fixed cell width and no line identities;
wrapped proportional text has no columns. Its granularity enum (`SelectGranularity`) is mirrored, not
shared, because its word rule is per-cell. (b) Flattening the transcript into one string with
global offsets: every delta upstream of a selection would shift every offset after it and need
re-mapping, and the flat string would be rebuilt on each event. (c) Keying by `EntryKey`: it is
stable across rebuilds, which FR-019 does not want, and its `Ord` is not reading order (tool cards
sort by id string).

## R2. Shown text: one function per unit kind, shared by copy and checked against the view (FR-002, FR-010, FR-011, SC-001)

**Decision**: What a unit contributes to copied text is its **shown text**, computed render-free:

| Unit | Shown text | Where |
|---|---|---|
| User message, notices, raw rows, turn footer | the string the view passes to its `Text` | existing view-model functions in `ui/transcript/entries.rs` (`footer_view`, `compacted_text`, …), moved or re-exported where a test needs them |
| Plain (#939) block | `plain_text(text)` (tabs expanded, `ui/transcript/blocks.rs`) | unchanged |
| Markdown inline run (paragraph, heading, list item text, table cell) | `markdown::shown_text(&Inlines)` *(new)*: the inline text without emphasis markers, link targets or code backticks; a link is its shown text | `micold_core::markdown` |
| List marker | `"•"` or `"{n}."`, as `ui/material/markdown_view.rs` draws it | `micold_core::markdown::list_marker` *(new)*, which the view then calls |
| Code block | its code, every line in full | `micold_core::markdown::CodeBlock::code` (exists) |
| Inline diff | path; hunk names and notes; each line's text without gutter marker or number | `micold_core::structured::edit_diff` model (exists) |
| Tool card header | tool name; input summary | `ToolCard::name`, `ToolCard::summary` |
| Unfolded tool input / result | `plain_text(input)`, `plain_text(part)` per result part; `[image not shown]` captions | `blocks.rs` |

Units are joined with one `\n`, whatever their kind (FR-010: "each boundary between entries
becomes one line break"; a block boundary is a real line break); text inside a unit keeps its own
`\n`s; soft wrapping adds nothing because the shown text never contains a wrap. Table cells of one
row are joined with a tab, rows with `\n` (R3). The text of folded cards' bodies, control labels,
icons and images is never a unit (FR-002).

The client assembles the copied text with `features/transcript_selection.rs::units(entry, open,
derived) -> Vec<(TextPath, Cow<str>)>` *(new)*, which walks an entry exactly as the renderers draw
it. A **parity test** (`crates/micold-client/tests/transcript_selection_parity.rs` *(new)*) draws
every entry kind of the fixture transcripts through `BlockRenderers::RICH` and `PLAIN`, collects
each `SelectableText`'s `(TextPath, text)` from the widget tree with a widget operation (R5), and
asserts it equals `units()` for that entry: the path order and the text the view draws cannot drift
from what is copied.

**Rationale**: FR-011 (clarified, D5) wants the visible rendered text, which only the renderer's
inputs know; computing it in the view would make copying depend on a render. Keeping it in core and
`features/` keeps it under Principle I's fast tests, and the parity test closes the gap between two
walks of the same entry.

**Alternatives considered**: (a) Copying from the widget tree at copy time (an operation that
collects selected text from drawn widgets): entries scrolled out of view are inside `lazy` but
still in the tree, yet a code line scrolled sideways or a folded card's text would need the tree to
hold text it does not draw, and copying would need a frame; it also makes the copy untestable
without a renderer. (b) Copying markdown source (rejected by D5).

## R3. Markdown specifics (FR-011)

**Decision**: Headings, paragraphs, list items and table cells are each one unit. List markers are
units of their own, before the item text (`"•"` and `"1."`); task-list check icons are icons, not
text, and contribute nothing. Block quotes draw a rule, not a character, so they contribute nothing
but their content. A horizontal rule contributes nothing. Table cells of a row are separated by one
tab and rows by `\n`; the header row is a row. Inline code contributes its text without backticks.

**Rationale**: FR-011 says markers, quote marks and cells are "copied as the characters the view
draws for them". The view (`markdown_view.rs::list`, `::block` `Quote`, `::table`) draws `•`/`n.`
as text, the quote and the rule as filled containers, and table cells in a grid. Tab-separated
cells paste into a spreadsheet or editor as columns, the closest plain-text form of a grid.

**Alternatives considered**: spaces between cells (ambiguous when a cell holds spaces); `| a | b |`
pipes (that is markdown source, which D5 rejects).

## R4. Granularity: word, line, all (FR-005, FR-006, Edge Cases *Wide characters*)

**Decision**: Core functions on a unit's shown text: `snap(text, offset)` to the nearest grapheme
boundary (`unicode-segmentation`, already a dependency of `micold-core` and `micold-material`);
`word_at(text, offset) -> Range<usize>` by Unicode word boundaries (`split_word_bound_indices`),
whitespace and punctuation runs selecting as their own run; `source_line_at(text, offset)` (a
`\n`-delimited line). A **visual** line in wrapped prose is only known to the laid-out paragraph,
so `SelectableText` reports it with the press (R5): the byte range between hit-testing the row's
left and right edges. Triple-click uses the visual line in prose units and the source line in code
and diff units (FR-005). Select-all sets the anchor at the first unit's start and the focus at the
last unit's end, read from `units()` of the first and last entry. Granularity is kept on the
selection, so a drag after a double-click extends by words and after a triple-click by lines, as
the terminal's `SelectGranularity` does.

**Rationale**: Word and source-line rules are pure text and belong under core tests; only the
visual line needs layout. Snapping in core guarantees that no copy splits a grapheme cluster on any
platform.

**Alternatives considered**: `char`-based boundaries (split emoji ZWJ sequences and combining
marks); the terminal's word rule (cell-based, ASCII-centric).

## R5. Drawing and hit-testing: a shared `SelectableText` primitive and a `SelectionArea` wrapper (FR-001, FR-003, FR-004, FR-005, FR-007, FR-020, Principle VIII)

**Decision**: Two new shared widgets in `micold-material` *(new files `selectable_text.rs`,
`selection_area.rs`)*, both `Widget<M, iced::Theme, iced::Renderer>` like `LineClamped` and
`wide_content::Pane`:

- **`SelectableText`**: a run of styled spans (the same span inputs `LinkText` takes) shaped as
  **one paragraph per hard line**, a unit identity (`SelectSlot<K>`: an opaque key the caller
  supplies), and the selected byte range of this unit (`Option<Range<usize>>`). It draws the
  highlight under the selected bytes and draws the selected glyphs in the selection's text colour.
  On a left press, on a cursor move while its own left button is down, and on each redraw while a
  drag is in progress and the (possibly clamped, below) cursor is over it, it publishes
  `on_hit(slot, Hit { offset, click: Single|Double|Triple, line: Range<usize> })`. It never
  captures the event, so a parent sees the press too. Links keep working as in `LinkText`
  (`hit_span` → link message; a press on a link opens it and starts no selection, FR-004).
  It answers a widget operation that reports `(slot, text)` for the parity test (R2).
- **`SelectionArea`**: wraps the transcript's scrollable. It is a keyboard focus target
  (`operation::Focusable`, joined to `State::focused_field` as `ui/focus.rs` requires, as
  `FieldId::Transcript(SessionId)` *(new variant of `features/window.rs::FieldId`)*); a left press
  not captured by a child (fold toggles, copy controls, "jump to latest", permission buttons and
  links all capture) takes focus and publishes `on_press(click_count)`; a press a child captured
  starts nothing (FR-004). While dragging and the cursor is above or below its bounds, it passes its
  children a cursor **clamped** into its bounds (so the unit at the edge row keeps answering `on_hit`)
  and publishes `on_edge(Up|Down)` on each redraw it requests, which the shell turns into a
  `scroll_by` on `transcript_scroll_id(session)` (FR-003). A right press anywhere inside publishes
  `on_context(origin)` and changes nothing else (FR-013). Release publishes `on_release`, which
  writes nothing (FR-012).

**Hit-testing detail**: iced 0.14's `Paragraph::hit_test` returns a `Hit::CharOffset` that is
cosmic-text's `Cursor::index`, a byte index *within one buffer line*, and drops the line number
(`iced_graphics-0.14 text/paragraph.rs:262`). Shaping one paragraph per hard line makes the index
unambiguous; the widget stacks the paragraphs and adds the line's start offset. The visual line for
triple-click is `hit_test(x = 0, y)`..`hit_test(x = width, y)` on the paragraph under the pointer.
The highlight rectangles come from `Paragraph::span_bounds` of the selected span: the widget shapes
its spans split at the selection's ends (cached in its tree state against the range, so a frame
without a selection change re-shapes nothing).

**Colours**: highlight fill `roles.primary_container`, selected text `roles.on_primary_container`,
in every unit kind including code surfaces, diff lines and failed cards (FR-020). Overriding syntax
and diff colours inside the highlight is what guarantees 4.5:1 everywhere (`tokens_contrast`
already pins `on_primary_container` on `primary_container` in both schemes); the
`composition_contrast` gate gains the pair as composed on each surface (SC-006).

**Rationale**: Every text the spec names is drawn by one of five leaves (`Text`, `LinkText`,
`CodeBlock`'s lines, `InlineDiff`'s lines, the card header labels); giving each a `.selectable(slot,
range)` builder that swaps its paragraph for `SelectableText` keeps one implementation of
hit-testing and highlighting (Principle VIII) and leaves every non-transcript caller unchanged.
Letting leaves report hits and the area own gesture, focus and edge-scroll avoids a parent that
must hit-test into arbitrary children, which iced's widget API does not offer.

**Alternatives considered**: (a) iced `text_editor` in read-only mode per block: it owns a cursor and
selection per editor, so a selection could not cross blocks (D1), and it cannot draw markdown
spans, links or syntax colours. (b) A canvas that re-lays out the whole transcript: forks all of
#939/#940's rendering. (c) Using `Span::highlight` through `rich_text` for the highlight: draws the
fill, but `rich_text` exposes no hit-test to the app, so a leaf widget is needed anyway.
(d) Reading `cosmic_text::Buffer` through `iced_graphics::text::Paragraph::buffer()`: works only
on the concrete renderer type and adds a direct `cosmic-text`/`iced_graphics` dependency pinned to
iced's; one paragraph per hard line needs neither.

## R6. Highlight per entry inside `lazy` (SC-003, FR-019)

**Decision**: `RenderCtx` gains `selection: Option<&RowSelection>` *(new)*: the part of the
selection that falls in this entry's body row (`in_row(&RowPos::Entry(id, Under::Body))`) (`Whole`, or `From(path, offset)` / `To(path, offset)` /
`Within(start, end)`). The entry's `lazy` dependency (`ui/transcript/mod.rs`, keyed today by key,
revision, fold state, scheme and cache stamp) adds it, so a drag rebuilds only the entries whose
selected part changed — the first and last entry of the range, and entries entering or leaving it.
Each renderer passes each unit `range_in(row_selection, path, text_len)`. The rows outside `lazy`
(the omitted notice, request blocks and resolution rows) are cheap and redrawn each frame already;
they take `in_row` of their own `RowPos` directly.

**Rationale**: The transcript's performance rests on per-entry `lazy` (#939 research R12). A
selection moving inside one entry must not rebuild the others.

**Alternatives considered**: drawing the highlight as an overlay outside `lazy` (needs every unit's
screen rectangles from the tree each frame, the very coupling R5 avoids).

## R7. Keeping, clearing and validating the selection (FR-019, Edge Cases)

**Decision**: `features/transcript_selection.rs` clears a session's selection when: the transcript
rebuilds (`Transcript::rebuilds()` changed), the anchor or focus entry is evicted from the front
(FR-020 bound), an endpoint lies in the omitted notice and the omitted count changes (its text
changed), a prompt row (`Under::Prompt`, `End`) appears, leaves or changes at or between the
selection's bounds (indices shifted or text added inside the range; compared on the placed list
before and after each prompt message), a card holding part of the range is folded or unfolded (`TranscriptMsg::Toggle` on an
entry in range), no pane of the project's layout shows the session any more (`shell/panes.rs::sync`, run after
every message, already computes the shown terminals as `now`; it calls
`transcript_selection.retain_shown(&sessions, &now)` *(new)*, `sessions` the project's sessions
(`app.core.workspace.session_ids_of_project(&project)`) and `now` filtered to `SessionProcess::Primary` terminals
(the pane that draws a session's transcript), so a selection is dropped only for this project's
sessions no pane shows; which covers `show_or_focus`,
`PaneMsg::Show`, `close` and `prune` in one place; `swap` keeps the session shown and its
selection), or the window forgets the session. After each applied `StructuredEvents` message,
for each entry **in range whose revision changed**, it compares the selected substring of that entry
before and after; a difference (a partial entry replaced by different text, #939 FR-015) clears the
selection. Entries outside the range and growth past the focus are not looked at.

**Rationale**: Comparing only changed entries in range keeps the check proportional to the change,
not to the selection (a select-all during streaming checks one entry per delta). Growth after the
focus offset does not alter the selected substring, so it keeps the selection, as FR-019 requires.

**Alternatives considered**: storing a copy of the selected text and comparing whole (O(selection) per
delta); clearing on any change in range (would clear on every streamed delta into the last entry).

## R8. Copy keys and the interrupt (FR-008, FR-009, FR-019a, D2)

**Decision**: `features/composer.rs::transcript_key(press, phase)` becomes `transcript_key(press,
phase, selecting, focused, mac) -> KeyIntent`, with two new intents `CopySelection` and
`SelectAll`. `selecting` = the session has a non-empty selection; `focused` = `focused_field ==
Some(FieldId::Transcript(session))`. Copy and select-all need `focused` (the table's rows are all
"transcript focused"); with `focused` false the function returns today's result (Ctrl+C
interrupts a busy turn, else nothing), so the arbitration stays in this one pure function rather
than split between it and `update`:

| Press (transcript focused) | selection | busy | Intent |
|---|---|---|---|
| Ctrl+C, Ctrl+Shift+C (Linux/Windows); Cmd+C or Ctrl+C (macOS) | yes | any | `CopySelection` |
| Ctrl+C | no | yes | `Interrupt` (#941 FR-012a, unchanged) |
| Ctrl+C | no | no | `Edit` (nothing) |
| Ctrl+Shift+C, Cmd+C | no | any | `Edit` (nothing; never interrupts) |
| Ctrl+A (Cmd+A on macOS) | any | any | `SelectAll` |
| Escape | — | yes | `Interrupt` (unchanged) |

`mac` is a parameter (the one call site passes `cfg!(target_os = "macos")`), so both platforms'
tables are tested on every host (Principle VI). The listener `ui/composer.rs::transcript_interrupt`
becomes `transcript_keys`: subscribed while the active session shows a transcript and either the
turn can be interrupted (today's condition) **or** `focused_field == FieldId::Transcript(session)`.
A key press reaches it only when no widget captured it, so with the composer focused the composer's
own `composer_key` (#941) still decides (US2 scenario 4); with focus elsewhere in the chrome
`focused_field` is not the transcript and copy and select-all do nothing (FR-019a).

**Rationale**: The arbitration is a pure function already tested in `features/composer.rs`; adding
inputs keeps it there. #1403's fix made `transcript_key` the single place Ctrl+C becomes an
interrupt, so the copy rule is checked before it.

**Alternatives considered**: a separate key subscription for copying (two listeners racing on one
key); reading the platform inside the function (untestable off-platform).

## R9. Writing the clipboard and failure (FR-008, FR-012, FR-014)

**Decision**: A copy by key or menu emits the existing `Outcome::ClipboardWriteChecked { text,
ticket }` (`shell/clipboard.rs::interpret`: write, then read back), with a ticket from a new
`CopyTarget::Selection` (R10). When the read-back differs or is `None`, the pane shows a short
notice through the existing per-session composer notice (`features/composer.rs` `Composers::notice`)
— "Copy failed" — and the selection stays. Nothing writes the clipboard on release (FR-012, D6);
`SelectionArea::on_release` only ends the drag.

**Rationale**: #940's read-back (research R5 there) is the established, cross-platform way this app
knows a copy held. The composer's notice is already drawn in the pane, under the transcript, per
session (Principle II).

**Alternatives considered**: the window snackbar (window-wide, not "in that pane"); unchecked
`ClipboardWrite` (cannot detect FR-014's failure).

## R10. Copy controls (FR-015–FR-018)

**Decision**: `features/transcript_rich.rs`'s copy state, keyed today by `CodeId`
(`Derived::copy: HashMap<CodeId, CopyState>`, `TranscriptMsg::CopyCode`), is generalised to a
`CopyTarget` *(new)*: `Code(CodeId) | ToolValue | ToolInput | ToolResult | DiffPath | Selection`,
and `TranscriptMsg::CopyCode` to `TranscriptMsg::Copy { session, key, target }`. Tickets, the
read-back, `COPY_CONFIRM` expiry and per-window state are unchanged, so FR-018's "confirmation only
on the activated control, in the window where it was activated" holds by construction. The value
of each target, computed in core:

- `ToolValue`: `summary::work_value(name, input_json) -> Option<String>` *(new, in
  `micold_core::structured::transcript::summary`)*: the work field `input_summary` already reads
  (`work_field`), uncut and on as many lines as it has. A header shows the control only when it
  returns `Some` (FR-015 "for every tool whose summary shows one").
- `ToolInput`: the input JSON text as held (`ToolCard::input`), including while streaming.
- `ToolResult`: the result parts' text as held, joined with `\n`, `[kind not shown]` for unshown
  parts.
- `DiffPath`: `EditDiff::path` (`crates/micold-core/src/structured/edit_diff.rs:20`, the call's
  `file_path`).

The controls are the existing `IconButton` copy control `CodeBlock::copy` uses, focusable with
`TakesTheKeyboard` (FR-018 keyboard), on `DisclosureCard`'s header (ToolValue), above the input and
the result blocks (ToolInput, ToolResult), and beside `InlineDiff`'s path (DiffPath).

**Rationale**: SC-004 asks for "byte-for-byte equal to the value the card holds", so the controls copy
the held value; tab expansion is presentation (`plain_text`), like the diff gutter. Reusing #940's
state machine gives FR-018 and SC-004's one-second confirmation with no new timers.

**Alternatives considered**: a second, parallel copy-state map for tool values (duplicates #940's
tickets and expiry, which `mise run duplication` budgets); copying the shortened summary (FR-015
forbids).

## R11. Context menu (FR-013)

**Decision**: The existing `micold_material::ContextMenu` (`menu.rs:547`, keyboard-operable, closes
on Escape and outside click, used by the terminal's menu in `ui/terminal.rs`) with two items:
**Copy** (`message: None` when the selection is empty, which draws it disabled) and **Select all**.
Its state, `Option<(SessionId, origin)>`, sits in `features/transcript_selection.rs`. Choosing Copy
re-focuses the transcript (`FieldId::Transcript`) and copies (FR-019a).

**Rationale**: Reuse (Principle VIII); one menu implementation already meets FR-013's keyboard and
dismissal rules.

**Alternatives considered**: a transcript-specific menu widget (forks `ContextMenu`).

## R12. Performance (SC-003)

**Decision**: Hit-testing touches one unit's paragraphs per event; the highlight re-shapes only the
units whose range changed (R5 cache) inside only the entries whose part changed (R6). Copy builds the
text with one pass over the entries in range (`units()` per entry, no rendering). The existing
release-mode budget test `crates/micold-client/tests/transcript_open_budget.rs` gains a variant:
select-all and copy on a transcript at its event bound in under 1 s, and a drag step (one
`on_hit` → `update` → `view` of the changed entries) under 100 ms.

**Rationale**: SC-003's numbers, measured where the transcript's other budgets already are.

**Alternatives considered**: a manual-only check (not repeatable in CI).

## R13. Sequencing

**Decision**: No feature this plan builds on is unmerged: #939, #940, #941 (with BUG-1403) and #944
are on `main`. The code milestones (plan.md *Delivery*) go core → render-free client state → shared
widgets → selectable leaves → transcript wiring → copy controls and docs, each mergeable on its own: nothing becomes
visible to the user until M4b wraps the transcript in `SelectionArea` (M4a makes the leaves
selectable but nothing yet starts a selection).
