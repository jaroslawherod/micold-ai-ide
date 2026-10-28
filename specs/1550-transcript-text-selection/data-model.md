# Data Model: Structured View — Select and Copy Transcript Text (#1550)

Nothing here is persisted (spec *Out of Scope*: no selection across restarts). All state is per
window; selection state is per session within the window, which is per pane (research R1).

## TextPath *(new, `micold_core::structured::selection`)*

A unit's place inside its entry, as a short sequence of `u16` segments assigned in drawing order
(block index, list item, table row, table cell, line …). Derives `Ord`: lexicographic order is
reading order inside the entry. Built only by the renderers and by `units()`, from the same walk
(research R2); the parity test proves they agree.

## RowPos *(new, core)*

The transcript row a unit is drawn in (`ui/transcript/mod.rs` `RowKey`: rows without an `EntryId`
are selectable too, spec *Terms: Selectable text* "notices", Edge Cases *Permission prompts*):

```
enum RowPos { Omitted, Entry(EntryId, Under), End(u32) }
enum Under { Body, Prompt(u32) }
```

- `Omitted`: the omitted-entries notice (`Row::Omitted`), always first.
- `Entry(id, Body)`: the entry's own units; `Entry(id, Prompt(k))`: the descriptive text of the
  `k`-th request block or resolution row drawn under that entry's card (`entries::with_trailing`).
- `End(k)`: the `k`-th request block after the last entry (`Placement::AtEnd`), `k` its index in
  that list as drawn.

Derives `Ord` in variant order, then fields: `Omitted < Entry(..) < End(..)`, entries by `EntryId`,
an entry's body before what is drawn under it — the drawing order. Indices, not `PromptId`s, order
the prompt rows (a `PromptId`'s `Ord` is not drawing order); a change to the placed prompt list is a
rebuild of those rows and clears a selection that touches them (S-6). `Row::Empty` draws only
"no output" with no other row and holds no unit.

## TextPos *(new, core)*

| Field | Type | Rule |
|---|---|---|
| `row` | `RowPos` | the row the unit is drawn in (above) |
| `path` | `TextPath` | the unit inside the row |
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
- `in_row(row: &RowPos) -> Option<RowSelection>`; `range_in(RowSelection, path, len) ->
  Option<Range<usize>>` (R6).

**State transitions** (all in `features/transcript_selection.rs`):

```
none ──press on text──▶ dragging(anchor=focus=hit) ──hit/edge──▶ dragging(focus moves)
dragging ──release──▶ held                        (clipboard untouched, FR-012)
held ──click without drag──▶ none                 (FR-004)
held ──double/triple click──▶ held(word/line)     (FR-005)
any ──select all──▶ held(first unit start .. last unit end)   (FR-006)
held|dragging ──rebuild | anchor/focus entry evicted | fold/unfold in range
              | selected text replaced | prompt rows in range change
              | no pane shows the session | session forgotten──▶ none (FR-019)
held ──focus leaves the transcript──▶ held (still highlighted, FR-019a)
```

## RowSelection *(new, core)*

The part of a selection inside one row: `Whole | From(TextPath, usize) | To(TextPath, usize) |
Within((TextPath, usize), (TextPath, usize))`. `Hash` + `Eq`: part of an entry's `lazy` key (R6),
which takes `in_row(&RowPos::Entry(id, Under::Body))`. The omitted notice and the prompt rows are
not inside `lazy`; they take their part directly when drawn.

## SessionSelection *(new, `features/transcript_selection.rs`)*

Per window, `HashMap<SessionId, SessionSelection>`:

| Field | Type | Rule |
|---|---|---|
| `selection` | `Option<Selection>` | none = nothing selected |
| `rebuilds` | `u64` | `Transcript::rebuilds()` when the selection was made; a change clears (FR-019) |
| `menu` | `Option<(u16, u16)>` | the open context menu's origin (FR-013) |

A session's entry is dropped when no pane of its project's layout shows it any more (FR-019 "pane
switches to another session"): `shell/panes.rs::sync`, which runs after every message and already
computes the layout's shown terminals (`now`), calls `transcript_selection.retain_shown(project,
&now)` *(new)*. That one call covers every way a pane changes session (`show_or_focus`,
`PaneMsg::Show`, `close`, `prune`); `swap` keeps the session shown and keeps its selection; a
project switch leaves the other project's layout, and so its selections, as they are.

The window's transcript focus is not here: it is `features/window.rs` `State::focused_field ==
Some(FieldId::Transcript(session))` *(new variant)*, joined through `ui/focus.rs` like every other
field (R5, R8).

## Unit *(new, `features/transcript_selection.rs::units`)*

`(TextPath, Cow<'_, str>)`: one drawn piece of selectable text and its shown text (research R2
table). A row's units, in `TextPath` order, joined with `\n`, are what a whole-row selection
copies; rows are joined with `\n` (FR-010). `units` covers every `RowPos`: an entry body
(`entry_units`), the omitted notice (its one shown line), a request block or resolution row (its
descriptive text only — tool name, summary, reason — never button labels or input fields, spec
Edge Cases *Permission prompts*).

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
| `DiffPath` | `edit_diff` `EditDiff::path` (`structured/edit_diff.rs:20`) | an Edit/MultiEdit/Write diff is drawn (FR-017) |
| `Code(id)` | `markdown::CodeBlock::code` (#940, unchanged) | as #940 |

## Messages *(new `TranscriptMsg` variants, `features/structured_transcript.rs`)*

`Select(SelectMsg)` with `SelectMsg = Hit { session, pos: TextPos, click, line } | Press { session, clicks }
| Edge { session, dir } | Release { session } | Context { session, origin } | MenuDismissed {
session } | CopySelection { session } | SelectAll { session }`; `CopyCode` becomes `Copy { session,
key, target }`. `CopyChecked` / `CopyExpired` are unchanged.
