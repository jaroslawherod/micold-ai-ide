# Contract: selection model (core and render-free client)

`micold_core::structured::selection` *(new)* and `crates/micold-client/src/features/transcript_selection.rs`
*(new)*. Tested without a renderer (Principle I). IDs are referenced by tasks.md.

## API (core)

```rust
pub struct TextPath(/* Vec<u16> */);            // Ord = drawing order inside an entry
pub enum Under { Body, Prompt(u32) }            // an entry's own units, then what is drawn under it
pub enum RowPos { Omitted, Entry(EntryId, Under), End(u32) } // Ord = drawing order of rows
pub struct TextPos { pub row: RowPos, pub path: TextPath, pub offset: usize } // Ord = reading order
pub enum Granularity { Char, Word, Line }
pub struct Selection { anchor: TextPos, focus: TextPos, granularity: Granularity, dragging: bool }
pub enum RowSelection { Whole, From(TextPath, usize), To(TextPath, usize), Within((TextPath, usize), (TextPath, usize)) }

pub fn snap(text: &str, offset: usize) -> usize;              // nearest grapheme boundary
pub fn word_at(text: &str, offset: usize) -> Range<usize>;    // Unicode word bounds
pub fn source_line_at(text: &str, offset: usize) -> Range<usize>;
impl Selection {
    pub fn start(at: TextPos, granularity: Granularity, expand: Range<usize>) -> Self;
    pub fn extend(&mut self, to: TextPos, expand: Range<usize>);
    pub fn is_empty(&self) -> bool;
    pub fn bounds(&self) -> (TextPos, TextPos);
    pub fn in_row(&self, row: &RowPos) -> Option<RowSelection>;
}
pub fn range_in(part: &RowSelection, path: &TextPath, len: usize) -> Option<Range<usize>>;
```

Core also gains `markdown::shown_text(&Inlines) -> String`, `markdown::list_marker(start, k) ->
String`, and `structured::transcript::summary::work_value(name, input_json) -> Option<String>`.

## API (client, render-free)

```rust
pub fn entry_units<'a>(entry: &'a Entry, open: bool, derived: Option<&'a Derived>) -> Vec<(TextPath, Cow<'a, str>)>;
pub fn units<'a>(row: &RowPos, view: &'a TranscriptView<'a>) -> Vec<(TextPath, Cow<'a, str>)>; // every row kind
pub fn retain_shown(&mut self, project: &Path, shown: &[TerminalRef]);
pub fn copy_text(transcript: &Transcript, open: &HashSet<EntryKey>, cache: &RichCache, sel: &Selection) -> String;
impl State { pub fn update(&mut self, msg: SelectMsg, ...) -> Effect; pub fn applied(&mut self, session, transcript, ...); }
```

## Invariants

- **S-1 Order**: for any two positions, `a < b` iff `a` is drawn before `b` in reading order (row
  order — omitted notice, each entry's body then the request/resolution rows under it, then the
  request blocks at the end — then `TextPath`, then offset). FR-002. Tests cover a range from the
  omitted notice into an entry, and from an entry across a request block under it into the next
  entry and on into an at-end request block.
- **S-2 Exact text**: `copy_text` = the shown text of every unit between `bounds()`, sliced at the two
  offsets, units joined by `\n`; no control label, icon, folded body or image caption of a folded
  card. A request block or resolution row contributes its descriptive text only (no button label,
  no input field); the omitted notice its shown line. FR-002, FR-010, Edge Cases *Permission
  prompts*.
- **S-3 As shown**: markdown copies `shown_text` (no `*`, `_`, `` ` ``, `#`, fences, link targets;
  links as shown text); list markers as drawn; table cells tab-separated, rows `\n`; diff lines
  without marker or number; soft wraps add nothing; indentation kept. FR-010, FR-011, SC-001.
- **S-4 Graphemes**: every offset stored or sliced is a grapheme boundary; copied text never holds a
  partial cluster. Edge Cases *Wide characters*.
- **S-5 Stability**: appending entries, growing an entry after the focus, or changing entries
  outside the range leaves `anchor`, `focus` and `copy_text` unchanged. FR-019, SC-005.
- **S-6 Clearing**: the selection is `None` after a rebuild, the eviction of its anchor or focus
  entry, a change of the omitted count while an endpoint is in the omitted notice, a change of the
  placed request list while a prompt row is in range, a toggle of an entry in range, a change of
  the selected substring of an entry in range, `forget(session)`, or `retain_shown` without the
  session (no pane shows it). A test with two panes: switching one pane to another session clears
  that session's selection and leaves the other pane's; a `swap` keeps both. FR-019.
- **S-7 Isolation**: a message for session A never changes session B's selection or menu, in the
  same window or another. Principle II.
- **S-8 No copy on release**: `Release` and a click without drag emit no clipboard outcome. FR-004,
  FR-012.
- **S-9 Click clears**: `Press` without a following `Hit` that moved, then `Release`, leaves no
  selection. FR-004.
- **S-10 Select all**: `SelectAll` selects from the first unit of the first entry to the end of the
  last unit of the last entry; on an empty transcript it does nothing. FR-006, Edge Cases *Empty*.
- **S-11 Granularity**: double-click selects `word_at`, triple-click the reported visual line (prose)
  or `source_line_at` (code, diff); a drag after them extends by that unit. FR-005.
- **S-12 Copy**: `CopySelection` with a non-empty selection emits `ClipboardWriteChecked` with
  `copy_text` and a `CopyTarget::Selection` ticket and keeps the selection; with an empty one it
  emits nothing. A failed read-back sets the session's composer notice and keeps the selection.
  FR-008, FR-014.
