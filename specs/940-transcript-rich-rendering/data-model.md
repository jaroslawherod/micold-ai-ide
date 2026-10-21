# Data Model: Structured Session Rich Rendering (#940)

Nothing here is persisted or sent on the wire (*Client only*). #939's types (`Transcript`,
`Entry`, `EntryKey`, `revision`, `ToolCard`, `BlockRenderers`, `RenderCtx`, `BlockCache`,
`TranscriptMsg`) are used as #939's data-model.md and contracts define them (research R14).

## Markdown document (core, new `micold_core::markdown`; research R1–R3)

### Document

| Field | Meaning |
|---|---|
| `blocks: Vec<Block>` | top-level blocks in order; `blocks[..settled]` are final |
| `settled: usize` | how many leading blocks are settled; `blocks.len()` when not streaming |
| `open_from: usize` | byte offset in the source text where the open block starts (`text.len()` when none) |
| `source_len: usize` | length of the text the document was built from |

Operations: `Document::parse(text: &str, streaming: bool) -> Document`; `update(&mut self, text:
&str, streaming: bool)` (incremental when `text` extends the old text up to `open_from`, else a
fresh parse, research R2); `code(&self, id: CodeId) -> Option<&CodeBlock>`; `code_blocks(&self) ->
impl Iterator<Item = (CodeId, &CodeBlock, bool settled)>`.

Invariants (tested, contracts/markdown-model.md M-1–M-5): the incremental document equals a fresh
parse of the same text; a settled block never changes on a later `update` of a longer text; with
`streaming: false`, `settled == blocks.len()`; every non-punctuation character of the text appears
in the blocks' text.

### Block

| Variant | Fields |
|---|---|
| `Heading` | `level: u8` (1–6), `text: Inlines` |
| `Paragraph` | `text: Inlines` (also raw HTML blocks, as literal text) |
| `List` | `start: Option<u64>` (ordered when `Some`), `tight: bool`, `items: Vec<ListItem>` |
| `Quote` | `blocks: Vec<Block>` |
| `Rule` | — |
| `Code` | `CodeBlock` |
| `Table` | `align: Vec<Align>` (`None`·`Left`·`Center`·`Right`), `header: Vec<Inlines>`, `rows: Vec<Vec<Inlines>>` |

`ListItem { task: Option<bool>, blocks: Vec<Block> }` (`task`: a task-list mark, checked or not).

### CodeBlock

| Field | Meaning |
|---|---|
| `id: CodeId` | ordinal of the code block in document order (nested ones included); stable, since settled blocks are final and the open block is last |
| `tag: Option<String>` | the fence's info string's first word, as written (shown, FR-009) |
| `token: Option<String>` | `fence_token(tag)`: the grammar token for highlighting (research R4) |
| `code: String` | exactly what is copied (FR-011): no fence, no tag, line breaks and indentation kept |
| `open: bool` | an unclosed fence (streaming, FR-007) |
| `lines: u32`, `bytes: u64` | for the size limit (research R7) |

### Inlines and Span

`Inlines = Vec<Span>`; `Span { text: String, style: Style, link: Option<Link> }`;
`Style { bold, italic, strike, code: bool }` (`code` = inline code); a hard break is a `"\n"` in
`text`. `Link { target: String, activatable: bool }` — `activatable` iff
`link::address::classify(target)` is `Web` or `Mail` (FR-004). An image is a span
`"{description} ({address})"` with no link (research R3).

### Functions

| Function | Rule |
|---|---|
| `fence_token(tag: &str) -> Option<String>` | first word, lower-cased, alias table (research R4); `None` for an empty tag |

## Edit diff (core, new `micold_core::structured::edit_diff`; research R6)

`edit_diff(tool: &str, input_json: &str) -> Option<EditDiff>`; `None` = fall back to the plain
input (FR-018).

### EditDiff

| Field | Meaning |
|---|---|
| `path: String` | `file_path` |
| `kind: EditKind` | `Edit` · `MultiEdit` · `Write` |
| `hunks: Vec<EditHunk>` | one per edit, in order (FR-013); one for a Write |
| `added: u32`, `removed: u32` | totals for the folded header (FR-017) |
| `bytes: u64` | input text size, for the size limit (research R7) |

### EditHunk

| Field | Meaning |
|---|---|
| `ordinal: u32`, `of: u32` | "Edit 2 of 3" (no file line numbers, FR-016) |
| `note: Option<HunkNote>` | `ReplaceAll` (FR-014) · `NoChange` · `EmptyWrite` · `NewContent` (Write) |
| `lines: Vec<EditLine>` | `EditLine { kind: LineKind, text: String }`, `LineKind` = `review::diff::LineKind` (Context · Added · Removed) |
| `old_no_newline`, `new_no_newline: bool` | the old or new text ends without a newline (a final-newline-only change shows as such) |

Invariant (SC-003): the `Context` + `Removed` lines are the old text's lines and the `Context` +
`Added` lines the new text's, in order.

## Render cache (client, new render-free `features/transcript_rich.rs`; research R8)

| Item | Meaning |
|---|---|
| `RichCache.sessions: HashMap<SessionId, SessionRich>` | per window; one per structured session; dropped by #939's `forget` paths |
| `SessionRich.entries: HashMap<EntryKey, Derived>` | derived drawing state of present entries only |
| `Derived.revision: u64` | the #939 entry revision it was derived from |
| `Derived.stamp: u64` | bumped on any change to the derived drawing that is not a revision change; part of the lazy key |
| `Derived.body: DerivedBody` | `Text(Document)` (assistant text, thinking) · `Diff(Option<EditDiff>)` (Edit/MultiEdit/Write cards; `None` = fallback) |
| `Derived.spans: HashMap<Part, Spans>` | highlight results in both schemes (`review::diff::Spans`), by part |
| `Derived.copy: HashMap<CodeId, CopyState>` | `Idle` (absent) · `Pending(Ticket)` · `Copied(Ticket)` · `Failed(Ticket)` |
| `Derived.full: HashSet<Part>` | parts the user chose to show in full (FR-019) |

`Part` = `Code(CodeId)` · `Diff`. `Ticket(u64)` is monotonic per window.

Operations (tested in `crates/micold-client/tests/features_transcript_rich.rs`):
`refresh(session, &Transcript) -> Vec<HighlightJob>` (derive changed revisions, prune absent keys);
`highlighted(job, spans) -> bool` (store if the part still exists; bumps the stamp);
`copy(session, key, code) -> Option<(String, Ticket)>`; `copy_checked(ticket, read:
Option<String>)`; `copy_expired(ticket)`; `show_in_full(session, key, part)`; `forget(session)`;
`get(session, key) -> Option<&Derived>`; `edit_counts(session, key) -> Option<(u32, u32)>`.

`HighlightJob { session, key, part, token: String, lines: Vec<String> }` — run off the update thread
by the shell with `ui::syntax` (research R4); answered by `TranscriptMsg::Highlighted`.

## Messages (client; added to #939's `TranscriptMsg`)

| Variant | From → effect |
|---|---|
| `OpenLink { url }` | a link press or Enter on a focused link → `Outcome::OpenLink(OpenRequest::Url)` for web/mail only (research R11) |
| `CopyCode { session, key, code: CodeId }` | the copy control → `Pending`, `Outcome::ClipboardWriteChecked { text, ticket }` |
| `CopyChecked { ticket, read: Option<String> }` | the shell's read-back → `Copied` or `Failed`; a `CopyExpired` timer task |
| `CopyExpired { ticket }` | after `COPY_CONFIRM` (2 s) → `Idle` if still current |
| `ShowInFull { session, key, part }` | the large part's button → `full` gains it; a highlight job if known language |
| `Highlighted { session, key, part, spans: Spans }` | a finished highlight job → stored, stamp bumped |

New `Outcome::ClipboardWriteChecked { text: String, ticket: Ticket }`: interpreted by
`shell/clipboard.rs` without a branch (research R5).

## Draw-side types (client, GUI exception)

`BlockRenderers::RICH` (`ui/transcript/rich.rs`) and the shared components `MarkdownView`,
`CodeBlock`, `InlineDiff`, `LinkText`, `DisclosureCard::meta`: see
[contracts/rich-renderers.md](contracts/rich-renderers.md) and research R9, R10.
