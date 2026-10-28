# Data Model: Structured View — Select and Copy Transcript Text (#1550)

Nothing here is persisted (spec *Out of Scope*: no selection across restarts). All state is per
window; selection state is per session within the window, which is per pane (research R1).

## TextPath *(new, `micold_core::structured::selection`)*

A unit's place inside its entry, as a short sequence of `u16` segments assigned in drawing order
(block index, list item, table row, table cell, line …). Derives `Ord`: lexicographic order is
reading order inside the entry. Built only by the renderers and by `units()`, from the same walk
(research R2); the parity test proves they agree.

## TextPos *(new, core)*

| Field | Type | Rule |
|---|---|---|
| `entry` | `EntryId` | the entry's position in this build of the transcript (R1) |
| `path` | `TextPath` | the unit inside the entry |
| `offset` | `usize` | byte offset into the unit's shown text; always a grapheme boundary (`snap`) |

Derives `Ord` in field order = reading order (spec *Terms: Selection*).

## Selection *(new, core)*

| Field | Type | Rule |
|---|---|---|
| `anchor` | `TextPos` | where the gesture started (spec *Terms*) |
| `focus` | `TextPos` | where it is now or ended |
| `granularity` | `Granularity { Char, Word, Line }` | from the click count; extends by the same unit (R4) |
| `dragging` | `bool` | between `on_press` and `on_release` |

- **Empty** when the expanded `start == end` (spec *Key Entities*); an empty selection copies
  nothing and disables the menu's Copy.
- `bounds() -> (TextPos, TextPos)`: normalised (min, max) and expanded by granularity.
- `in_entry(entry) -> Option<EntrySelection>`; `range_in(EntrySelection, path, len) ->
  Option<Range<usize>>` (R6).

**State transitions** (all in `features/transcript_selection.rs`):

```
none ──press on text──▶ dragging(anchor=focus=hit) ──hit/edge──▶ dragging(focus moves)
dragging ──release──▶ held                        (clipboard untouched, FR-012)
held ──click without drag──▶ none                 (FR-004)
held ──double/triple click──▶ held(word/line)     (FR-005)
any ──select all──▶ held(first unit start .. last unit end)   (FR-006)
held|dragging ──rebuild | anchor/focus entry evicted | fold/unfold in range
              | selected text replaced | pane shows another session | session forgotten──▶ none (FR-019)
held ──focus leaves the transcript──▶ held (still highlighted, FR-019a)
```

## EntrySelection *(new, core)*

The part of a selection inside one entry: `Whole | From(TextPath, usize) | To(TextPath, usize) |
Within((TextPath, usize), (TextPath, usize))`. `Hash` + `Eq`: part of the entry's `lazy` key (R6).

## SessionSelection *(new, `features/transcript_selection.rs`)*

Per window, `HashMap<SessionId, SessionSelection>`:

| Field | Type | Rule |
|---|---|---|
| `selection` | `Option<Selection>` | none = nothing selected |
| `rebuilds` | `u64` | `Transcript::rebuilds()` when the selection was made; a change clears (FR-019) |
| `menu` | `Option<(u16, u16)>` | the open context menu's origin (FR-013) |

The window's transcript focus is not here: it is `features/window.rs` `State::focused_field ==
Some(FieldId::Transcript(session))` *(new variant)*, joined through `ui/focus.rs` like every other
field (R5, R8).

## Unit *(new, `features/transcript_selection.rs::units`)*

`(TextPath, Cow<'_, str>)`: one drawn piece of selectable text and its shown text (research R2
table). An entry's units, in `TextPath` order, joined with `\n`, are what a whole-entry selection
copies; entries are joined with `\n` (FR-010).

## CopyTarget *(new, generalises `features/transcript_rich.rs` `CodeId` keys)*

`Code(CodeId) | ToolValue | ToolInput | ToolResult | DiffPath | Selection`. Keys `Derived::copy:
HashMap<CopyTarget, CopyState>` (was `HashMap<CodeId, CopyState>`) and the `Copying` ticket record.
`CopyState` (`Pending | Copied | Failed`, by `Ticket`) is unchanged. `Selection`'s state is not
drawn on a control; its `Failed` raises the pane notice (FR-014) and its `Copied` does nothing
visible.

## Copyable value *(spec Key Entity)*

| Target | Value (byte for byte, SC-004) | Shown when |
|---|---|---|
| `ToolValue` | `summary::work_value(name, input)` *(new)* | it is `Some` (FR-015) |
| `ToolInput` | `ToolCard::input` as held | card unfolded (FR-016) |
| `ToolResult` | result parts' text, `\n`-joined, `[kind not shown]` for unshown | card unfolded and a result exists (FR-016) |
| `DiffPath` | `edit_diff` `FileDiff::path` | an Edit/MultiEdit/Write diff is drawn (FR-017) |
| `Code(id)` | `markdown::CodeBlock::code` (#940, unchanged) | as #940 |

## Messages *(new `TranscriptMsg` variants, `features/structured_transcript.rs`)*

`Select(SelectMsg)` with `SelectMsg = Hit { session, pos, click, line } | Press { session, clicks }
| Edge { session, dir } | Release { session } | Context { session, origin } | MenuDismissed {
session } | CopySelection { session } | SelectAll { session }`; `CopyCode` becomes `Copy { session,
key, target }`. `CopyChecked` / `CopyExpired` are unchanged.
