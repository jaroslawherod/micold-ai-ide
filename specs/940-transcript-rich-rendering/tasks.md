---
description: "Tasks for #940 — structured session rich rendering"
---

# Tasks: Structured Session Rich Rendering — markdown, highlighted code and Edit/Write diffs

**Input**: `specs/940-transcript-rich-rendering/` — [plan.md](plan.md), [spec.md](spec.md),
[research.md](research.md), [data-model.md](data-model.md), [contracts/](contracts/),
[quickstart.md](quickstart.md)

**Starts after #939.** No task below starts before #939's last milestone (M6, "The pane shows the
transcript") has merged to `main` (research R14). T001 is the first task of the run: it re-checks
every #939 name these tasks use against the merged code and records any rename before a test is
written. Names of #939 below (`Transcript`, `Entry`, `EntryKey`, `revision`, `ToolCard`,
`BlockRenderers`, `BlockRef`, `RenderCtx`, `BlockCache`, `TranscriptMsg`,
`features/structured_transcript.rs`, `ui/transcript/{mod,entries,blocks}.rs`,
`transcript_open_budget.rs`, `transcript_renderer_seam.rs`, `DisclosureCard`,
`disclosure_card_header_fits.rs`) are #939's *planned* names.

**Tests**: mandatory (Constitution I). Every behaviour task has a failing test task before it.
`ui/transcript/rich.rs`, the shared components' drawing and the shell glue use the GUI exception:
the geometry gates, the release budget test and quickstart Part B cover them.

**Documentation**: the user guide changes in M5, the milestone that makes rich rendering visible
(Constitution VII, FR-024).

**Cross-platform**: core never branches on the OS; CRLF is handled in core (R6); copy is verified
by clipboard read-back on every platform (R5); links use feature 031's per-platform opener (R11).

**Dependencies**: `pulldown-cmark` 0.13 (`default-features = false`) in M1 and `similar` 2 in M2,
each added in the milestone that first uses it (research R1, R6). No property-test crate is added:
M-5 sweeps deterministic seeded cuts (T007).

**Organization**: phases follow the plan's delivery order (plan *Delivery*): the markdown model in
core, the edit diff in core, the window's render cache, the shared components, and the switch of
the transcript to the rich renderers last, so no half-wired UI reaches `main` (milestones.md rule
6). Each task carries the story it serves.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: can run in parallel (different files, no dependency on an unfinished task)
- **[Story]**: US1 (formatted markdown, streaming), US2 (Edit/MultiEdit/Write diffs), US3 (code
  blocks, highlighting, copy)

---

## Phase 1: Setup

- [ ] T001 Re-check #939's names (research R14): confirm with `git log origin/main` that #939 M6
  has merged (if not, stop and escalate: this run cannot start); then with `LSP`
  (`workspaceSymbol`, `documentSymbol`) find each #939 name listed above in the merged code
  (`crates/micold-core/src/structured/transcript/`, `crates/micold-client/src/features/structured_transcript.rs`,
  `crates/micold-client/src/ui/transcript/`, `crates/micold-client/src/ui/material/disclosure_card.rs`,
  `crates/micold-client/tests/`). Record every rename or changed signature in
  `specs/940-transcript-rich-rendering/research.md` R14 (a *Re-check* paragraph, "none" if none)
  and update the affected task text in this file in the same commit
- [ ] T002 Add `pulldown-cmark = { version = "0.13", default-features = false }` to the workspace
  `[workspace.dependencies]` in `Cargo.toml` and `pulldown-cmark.workspace = true` to
  `crates/micold-core/Cargo.toml`; parser options are `ENABLE_TABLES`, `ENABLE_STRIKETHROUGH`,
  `ENABLE_TASKLISTS` only (research R1); `Cargo.lock` gains `pulldown-cmark` and `unicase` only
- [ ] T003 Create the render-free module `crates/micold-core/src/markdown/` with `mod.rs`,
  `block.rs`, `split.rs`, `inline.rs`, `fence.rs` and add `pub mod markdown;` to
  `crates/micold-core/src/lib.rs` (plan *Project Structure*)
- [ ] T004 [P] Record `crates/micold-core/tests/fixtures/stream-json/markdown-turn.jsonl` with
  `scripts/capture-stream-json.sh` (`--include-partial-messages`): a prompt asking for a reply using
  every element of the subset — headings, bold/italic/strikethrough, inline code, nested bullet and
  numbered lists with a task list, a quote, a rule, a link, a table, and code blocks tagged `rust`,
  an unknown tag and none (research R13). Write the synthetic
  `crates/micold-core/tests/fixtures/stream-json/markdown-subset.md`: raw HTML (block and inline),
  an image, a `javascript:` and a relative link, a reference definition in a later block, CRLF
  text, an indented code block, an unclosed fence, unmatched `**`, a table without its separator
  row. Add both rows to `crates/micold-core/tests/fixtures/stream-json/README.md`. No logged-in
  `claude`: escalate; never hand-write the recorded file

---

## Phase 2: Foundational (blocks every story)

- [ ] T005 Define the model in `crates/micold-core/src/markdown/block.rs` per
  [data-model.md](data-model.md) *Markdown document*: closed `Block::{Heading { level: u8 } (1–6),
  Paragraph, List { start: Option<u64>, tight: bool, items: Vec<ListItem> }, Quote { blocks }, Rule,
  Code(CodeBlock), Table { align: Vec<Align>, header: Vec<Inlines>, rows: Vec<Vec<Inlines>> }}`,
  `ListItem { task: Option<bool>, blocks }`, `CodeBlock { id: CodeId, tag: Option<String>, token:
  Option<String>, code: String, open: bool, lines: u32, bytes: u64 }`, `CodeId` constructible only
  inside `markdown` (Constitution V), `Align::{None, Left, Center, Right}`, `Inlines = Vec<Span>`,
  `Span { text, style: Style, link: Option<Link> }`, `Style { bold, italic, strike, code: bool }`,
  `Link { target: String, activatable: bool }`; the `Document` struct (`blocks`, `settled`,
  `open_from`, `source_len`) with `blocks()`, `settled()`, `code()`, `code_blocks()` accessors in
  `mod.rs`; all `Debug + Clone + PartialEq + Eq`; re-export from `markdown/mod.rs`

**Checkpoint**: the model compiles; parsing comes next.

---

## Phase 3: User Story 1 — the markdown model (P1) 🎯 MVP

**Goal**: assistant and thinking text parse into settled and open blocks, final once settled,
incremental while streaming, never failing (contracts/markdown-model.md M-1–M-6).

**Independent Test**: `cargo test -p micold-core --test markdown_subset --test markdown_stream` and
`cargo test -p micold-core --lib markdown`.

### Tests for User Story 1 ⚠️ write first, see them fail

- [ ] T006 [P] [US3] Unit tests in `crates/micold-core/src/markdown/fence.rs` (`#[cfg(test)]`) for
  `fence_token` (research R4): first word cut at whitespace, `,` or `{`; lower-cased; aliases
  `shell`/`bash`/`zsh`/`console` → `sh`, `golang` → `go`, `yml` → `yaml`, `jsonc` → `json`, `c++` →
  `cpp`; `ts`, `rs`, `py`, `Rust` → `rust` pass through lower-cased; `""` and whitespace-only →
  `None`
- [ ] T007 [P] [US1] Fixture tests in `crates/micold-core/tests/markdown_subset.rs` (M-1, M-5, M-6,
  FR-001, FR-003, FR-004, FR-007, FR-011 at the model, SC-001) over `markdown-subset.md` and the
  final assistant text of `markdown-turn.jsonl`: each row of contracts/markdown-model.md *Rules*
  maps to its model (heading levels, styles, nested lists with `start`, `tight` and task marks,
  quote, rule, table `align`/`header`/`rows`); raw HTML and inline HTML stay literal text; the image
  is the span `"d (u)"` with no link; `http`/`https`/`mailto` and bare `https://` links are
  `activatable`, `javascript:`, `file:` and relative links are not; a reference defined in a later
  block stays literal; CRLF text gives the same blocks as LF; a table without its separator row is a
  `Paragraph`; unmatched `**` is literal; an unclosed fence is `Code { open: true }`;
  `CodeBlock.code` is exact for a fenced block (no fence, no tag), an indented block (four-space
  indent removed) and an open block (text so far), `tag` as written and `token` from `fence_token`;
  the hidden-text check: every received non-punctuation character appears in some span's or code
  block's text; `CodeId`s are document-order ordinals and name the same block in every longer
  prefix (M-6); M-5 over 10,000 seeded pseudo-random byte cuts (a fixed-seed xorshift in the
  test, no new dependency) of both fixtures' texts and of arbitrary UTF-8: no panic, no dropped text
- [ ] T008 [P] [US1] Stream tests in `crates/micold-core/tests/markdown_stream.rs` (M-2–M-4,
  FR-005–FR-008, SC-002) for `markdown-subset.md` and `markdown-turn`'s text: for every char-boundary
  prefix `p` and every longer prefix `q`, `parse(p, true).blocks()[..settled]` equals the same
  blocks of `parse(q, true)` (M-2); feeding the prefixes in order through one `update` gives after
  each a document equal to `parse(prefix, true)` (M-3); `update(full, false)` after the deltas
  equals `parse(full, false)` with `settled == blocks().len()` (M-4); and the delta-by-delta replay of
  `markdown-turn.jsonl` through #939's `Transcript` (`AssistantText` entry text after each applied
  message) gives the same documents, the finished message leaving the blocks unchanged (FR-008)

### Implementation for User Story 1

- [ ] T009 [P] [US3] Implement `fence_token` with its alias table in
  `crates/micold-core/src/markdown/fence.rs`; T006 passes
- [ ] T010 [US1] Implement `crates/micold-core/src/markdown/inline.rs` (research R3): pulldown-cmark
  inline events → `Span`s (soft break → space, hard break → `"\n"`), literal fallbacks (inline HTML,
  footnotes, math as text; image → `"description (address)"`, nothing fetched), `Link.activatable`
  iff `micold_core::link::address::classify(target)` is `Address::Web` or `Address::Mail`, and bare
  addresses in text spans found with `micold_core::link::detect::detect` split out as activatable
  link spans
- [ ] T011 [US1] Implement `crates/micold-core/src/markdown/split.rs` (research R2): run
  pulldown-cmark over the text (`into_offset_iter`), cut at every top-level `Start` offset, slices
  include their trailing blank lines; while streaming the last slice is open, else none is
- [ ] T012 [US1] Implement `Document::parse` and `Document::update` in
  `crates/micold-core/src/markdown/mod.rs`: each slice parsed on its own into `Block`s (block
  events, tables, lists, quotes, code blocks with `CodeId` ordinals and `lines`/`bytes`, through
  T010's inlines); `update` keeps `blocks[..settled]` and re-splits only `text[open_from..]` when
  `text` starts with the old text up to `open_from`, else parses afresh; `streaming: false` settles
  everything; T007, T008 pass

**Checkpoint**: M1 done — the markdown model is final once settled, incremental and total.

---

## Phase 4: User Story 2 — the edit diff (P1)

**Goal**: a complete Edit, MultiEdit or Write input becomes hunks of context, removed and added
lines, or `None` for #939's plain input (contracts/edit-diff.md E-1–E-3).

**Independent Test**: `cargo test -p micold-core --test edit_diff`.

### Tests for User Story 2 ⚠️ write first, see them fail

- [ ] T013 [US2] Setup for the edit diff: add `similar = "2"` to `[workspace.dependencies]` in
  `Cargo.toml` and `similar.workspace = true` to `crates/micold-core/Cargo.toml`; make
  `split_lines` in `crates/micold-core/src/review/diff.rs` `pub(crate)`; create
  `crates/micold-core/src/structured/edit_diff.rs` with the types of data-model.md *Edit diff*
  (`EditDiff { path, kind: EditKind, hunks, added: u32, removed: u32, bytes: u64 }`,
  `EditKind::{Edit, MultiEdit, Write}`, `EditHunk { ordinal: u32, of: u32, note:
  Option<HunkNote>, lines: Vec<EditLine>, old_no_newline: bool, new_no_newline: bool }`,
  `HunkNote::{ReplaceAll, NoChange, EmptyWrite, NewContent}`, `EditLine { kind:
  review::diff::LineKind, text }`) and a `todo!()` `edit_diff(tool: &str, input_json: &str) ->
  Option<EditDiff>`; add `pub mod edit_diff;` to `crates/micold-core/src/structured/mod.rs`. Record
  `crates/micold-core/tests/fixtures/stream-json/edit-turn.jsonl` with
  `scripts/capture-stream-json.sh`: Claude Code Writes a file, Edits it once with `replace_all`,
  and MultiEdits it with two edits (research R13); add its row to the fixtures `README.md`. No
  logged-in `claude`: escalate
- [ ] T014 [US2] Tests in `crates/micold-core/tests/edit_diff.rs` (E-1–E-3, FR-013, FR-014,
  FR-016–FR-019 at the model, SC-003): each shape of contracts/edit-diff.md *Input shapes*; a
  MultiEdit's hunks in edit order with `ordinal`/`of`; notes `ReplaceAll`, `NoChange` (old == new,
  no lines), `EmptyWrite` (Write of `""`), `NewContent` (every Write); CRLF-only difference is no
  change; a final-newline-only change sets `old_no_newline != new_no_newline` with equal lines;
  `added`/`removed` equal the hunks' counts (E-3); `bytes` is the input text size; every malformed
  input of the contract (other tool, not an object, missing or wrong-kind field, empty `edits`,
  invalid JSON) is `None` without panic (E-2); unknown extra fields are ignored; the round trip
  (E-1) over every Edit/MultiEdit/Write `ToolUse` input in `edit-turn.jsonl` and the synthetic set:
  `Context`+`Removed` lines equal the old text's lines, `Context`+`Added` the new text's

### Implementation for User Story 2

- [ ] T015 [US2] Implement `edit_diff` in `crates/micold-core/src/structured/edit_diff.rs` (research
  R6): `serde_json` shape checks per tool; lines split with `review::diff::split_lines`;
  `similar::TextDiff` over the line slices, Myers with a 50 ms deadline; equal → `Context`, delete →
  `Removed`, insert → `Added`; notes, final-newline flags and counts by the contract's *Rules*; no
  line numbers anywhere; T014 passes

**Checkpoint**: M2 done — every recorded edit round-trips.

---

## Phase 5: The window's rich cache (client, render-free; US1, US2, US3 at the state)

**Goal**: per window and session, a render-free cache derives documents and edit diffs from #939's
transcript, prunes with it, issues highlight jobs, holds copy and show-in-full state, and turns link
presses into opens for web and mail only (research R5, R7, R8, R11; contracts/rich-renderers.md
RR-4–RR-6, *Shell*). The transcript still draws with `PLAIN`.

**Independent Test**: `cargo test -p micold-client --test features_transcript_rich --test
clipboard_request` and `cargo test -p micold-client --lib ui::syntax`.

### Tests ⚠️ write first, see them fail

- [ ] T016 [P] [US3] In-crate tests in `crates/micold-client/src/ui/syntax.rs` (`#[cfg(test)]`) for
  `knows(token) -> bool` (FR-009): `rust`, `sh`, `yaml`, `json`, `py` known; an unknown token and
  `""` not; and that `fills(roles)` includes the code surface `surface_container_high`, so `legible`
  lifts colours on it (SC-006)
- [ ] T017 [US1] Reducer tests in a new `crates/micold-client/tests/features_transcript_rich.rs`
  for the cache (RR-4, RR-5, FR-019, FR-023): `refresh(session, &Transcript)` over a replayed
  `markdown-turn` and `edit-turn` derives `Derived::Text` for assistant text and thinking and
  `Derived::Diff` for Edit/MultiEdit/Write cards (`Diff(None)` while the input streams or is
  malformed); a refresh after a delta re-derives only the entry whose `revision` changed; keys no
  longer in the transcript are pruned; `forget(session)` drops one session only; two sessions in one
  cache and two caches never share state; `edit_counts` gives `(added, removed)`; highlight jobs are
  issued only for settled code blocks whose `token` the injected `Grammar` knows and for complete
  diffs whose path's token it knows (`Grammar { knows: fn(&str) -> bool, token: fn(&str) -> String }`,
  the shell's is `ui::syntax::{knows, token}`; the test passes a fake, so it needs no grammar set),
  once each; stored spans hold both schemes, so a theme switch issues no job (FR-023); a part over `review::limits` (`MAX_CHANGED_LINES` lines or `MAX_VERSION_BYTES` bytes) gets
  no job until `show_in_full`, which records the part for that session only and returns its job;
  `highlighted(job, spans)` stores spans and bumps `stamp` only when the part still exists
- [ ] T018 [US3] Copy tests in `crates/micold-client/tests/features_transcript_rich.rs` (after T017,
  same file; FR-011, FR-012, SC-005): `TranscriptMsg::CopyCode` stores `Pending(ticket)` and emits
  `Outcome::ClipboardWriteChecked { text, ticket }` whose `text` equals the block's `code` byte for
  byte (fenced, indented, empty, and an open block's code so far); `CopyChecked { read:
  Some(text) }` → `Copied`, a different text or `None` → `Failed`, each bumping the stamp and
  emitting
  `Outcome::CopyConfirmTimer { ticket, after: COPY_CONFIRM }` (2 s); `CopyExpired` with the current ticket →
  `Idle`, with a stale ticket → unchanged; copy state is per window, session, entry and `CodeId`
- [ ] T019 [US1] Link tests in `crates/micold-client/tests/features_transcript_rich.rs` (after T018,
  same file; FR-004, RR-6): `TranscriptMsg::OpenLink { url }` emits
  `Outcome::OpenLink(OpenRequest::Url(url))` for `http`, `https` and `mailto` addresses and nothing
  for `javascript:`, `file:` and relative addresses
- [ ] T020 [P] [US3] In `crates/micold-client/tests/clipboard_request.rs`: `shell::clipboard::interpret`
  turns `Outcome::ClipboardWriteChecked { text, ticket }` into a write followed by a read mapped to
  `TranscriptMsg::CopyChecked { ticket, read }`, with no branch on the result (feature 021 contract
  C3)

### Implementation

- [ ] T021 [P] [US3] Add `knows(token: &str) -> bool` (a syntax other than plain text exists) to
  `crates/micold-client/src/ui/syntax.rs`, and the code surface to `fills`, whose return type goes
  from `[Rgb; 3]` to `[Rgb; 4]` (update its callers and tests found with `LSP findReferences`,
  among them `ui/material/composition_contrast.rs`); T016 passes
- [ ] T022 [US1] Implement `crates/micold-client/src/features/transcript_rich.rs` per data-model.md
  *Render cache*: `RichCache`, `SessionRich`, `Derived { revision, stamp, body, spans, copy, full }`,
  `DerivedBody::{Text(Document), Diff(Option<EditDiff>)}`, `Part::{Code(CodeId), Diff}`,
  `CopyState::{Pending, Copied, Failed}(Ticket)` (absent = `Idle`), monotonic per-window `Ticket`,
  `HighlightJob { session, key, part, token, lines }`, `Grammar` (T017), `COPY_CONFIRM = 2 s`, and `refresh(session, &Transcript, &Grammar)`,
  `highlighted`, `copy`, `copy_checked`, `copy_expired`, `show_in_full`, `forget`, `get`,
  `edit_counts`; register the module and add `Outcome::ClipboardWriteChecked { text: String,
  ticket: Ticket }` and `Outcome::CopyConfirmTimer { ticket: Ticket, after: Duration }` in
  `crates/micold-client/src/features/mod.rs`, with their arms in every exhaustive
  `Outcome` match in `crates/micold-client/src/app.rs`
- [ ] T023 [US1] Add the six variants `OpenLink { url }`, `CopyCode { session, key, code }`,
  `CopyChecked { ticket, read }`, `CopyExpired { ticket }`, `ShowInFull { session, key, part }`,
  `Highlighted { session, key, part, spans }` to #939's `TranscriptMsg` and route them to
  `RichCache` in `crates/micold-client/src/features/structured_transcript.rs`; make both of its
  `forget` paths also forget the session's `RichCache`; hold `State.transcript_rich` beside
  `structured_transcript` in `crates/micold-client/src/app.rs`, refreshed after each applied
  `StructuredEvents` message for that session only; re-export `BlockCache` as
  `transcript_rich::RichCache` in `crates/micold-client/src/ui/transcript/blocks.rs`; T017–T019
  pass
- [ ] T024 [US3] Shell glue (GUI exception, contracts/rich-renderers.md *Shell*): in
  `crates/micold-client/src/shell/daemon_sync.rs` pass `Grammar { knows: ui::syntax::knows, token:
  ui::syntax::token }` to `refresh` and run the `HighlightJob`s returned by `refresh` (and
  by `ShowInFull`) on `spawn_blocking` with `ui::syntax::highlight_lines` in both schemes under a
  time budget (the `highlight_within` pattern) → `TranscriptMsg::Highlighted`; in
  `crates/micold-client/src/shell/clipboard.rs` interpret `ClipboardWriteChecked` as
  `iced::clipboard::write(text)` chained with `iced::clipboard::read()` →
  `TranscriptMsg::CopyChecked`; `Outcome::CopyConfirmTimer { ticket, after }` as a
  `tokio::time::sleep(after)` task, as `shell/pr_status.rs` does, → `TranscriptMsg::CopyExpired {
  ticket }`; `Outcome::OpenLink` through the existing
  `crates/micold-client/src/shell/links.rs::perform`; T020 passes, and #939's
  `transcript_renderer_seam` and `features_structured_transcript` stay green

**Checkpoint**: M3 done — the window's cache is filled, pruned and answers copy, highlight and link
messages; nothing draws it yet.

---

## Phase 6: Shared components (US1, US2, US3; Principle VIII)

**Goal**: `MarkdownView`, `CodeBlock`, `InlineDiff`, `LinkText` and `DisclosureCard::meta` exist
in the material library with builders, showcase entries and their gates (research R9).

**Independent Test**: `cargo test -p micold-client --test showcase_completeness --test
material_builder_api --test layout_snapshot` and `cargo test -p micold-client --lib ui::material`.

### Tests ⚠️ write first, see them fail

- [ ] T025 [P] [US3] Gates `crates/micold-client/tests/gates/code_block_fits.rs` (copy control and
  tag stay in the header row at a narrow width; a line wider than the transcript scrolls inside the
  block, which does not widen its parent: FR-010), `crates/micold-client/tests/gates/code_block_wheel.rs`
  (a vertical wheel over a code block scrolls the enclosing scrollable, not the block: R12) and
  `crates/micold-client/tests/gates/inline_diff_marker_fits.rs` (each `+`/`−` marker lies inside its
  cell, both schemes, no number gutter: FR-015, FR-016), each wired into
  `crates/micold-client/tests/layout_snapshot.rs` with `#[path]` as the existing gates are
- [ ] T026 [P] [US2] Add cases to `crates/micold-client/src/ui/material/composition_contrast.rs`
  (code text, inline code, link text, diff `+`/`−` markers and note text on their fills, both
  schemes: SC-006), `crates/micold-client/src/ui/material/style_outline_discipline.rs` (the code
  surface draws no border), `crates/micold-client/src/ui/material/ripple_clipping.rs` (the copy
  control's ripple stays in its shape); and the meta case to
  `crates/micold-client/tests/gates/disclosure_card_header_fits.rs` (a `+12 −3` meta never
  ellipsized, beside the summary and the trailing status, at a narrow width: FR-017)
- [ ] T027 [P] [US1] In-crate tests (`#[cfg(test)]`) in
  `crates/micold-client/src/ui/material/link_text.rs` and
  `crates/micold-client/src/ui/material/code_block.rs` with `ui/material/test_support.rs`: one
  keyboard focus stop per activatable link, Enter emits its `on_link` message, a non-activatable
  link has no stop and emits nothing (FR-004); the copy control is a focus stop, pointer press and
  Enter both emit `on_copy`, its label follows the copy state (copy / "Copied" / "Copy failed",
  FR-011, FR-012); a part over the limit shows "Large code block: N lines" and a "Show in full"
  button (FR-019); a `MarkdownView` test that its `copy_state`, `full` and `on_show` reach the
  `CodeBlock` of the matching `CodeId` (a copied block shows "Copied"; a large block not in `full`
  shows "Show in full" and its press emits `on_show(id)`)


### Implementation

- [ ] T028 [US2] Extract `DiffView`'s marker cell, line `tint` and span-over-tint `rich_text`
  builder in `crates/micold-client/src/ui/material/diff_view.rs` into `pub(super)` helpers that
  `DiffView` itself now uses (no change to its drawing; the Changes view's gates and
  `layout_snapshot` stay green)
- [ ] T029 [P] [US3] Implement `CodeBlock` in `crates/micold-client/src/ui/material/code_block.rs`:
  `CodeBlock::new(code, roles).language(tag).spans(&scheme_spans).copy(state, on_copy).large(lines,
  on_show).into()`; monospace unwrapped lines in a horizontal-only `material::Scrollable` on
  `surface_container_high`, the tag in the header row, an `IconButton` copy control in
  `TakesTheKeyboard` with `Icon::Copy` / `Icon::CardDone` + "Copied" / the error role + "Copy failed"
  (research R5, R9); T027's code-block tests pass
- [ ] T030 [P] [US1] Implement `LinkText` in `crates/micold-client/src/ui/material/link_text.rs`:
  `rich_text` with link spans opening on press (`on_link_click`) and one `TakesTheKeyboard` focus
  stop per activatable link; non-activatable links underlined without a link (research R9); T027's
  link tests pass
- [ ] T031 [US2] Implement `InlineDiff` in `crates/micold-client/src/ui/material/inline_diff.rs`
  with T028's helpers: `InlineDiff::new(&edit_diff, roles).spans(&scheme_spans).large(on_show)
  .into()`; the path header, per hunk "Edit n of m" and its note ("Replaces every occurrence", "No
  change", "Writes an empty file", "New content", "No newline at end"), lines with a `+`/`−` marker
  cell and tint in a horizontal-only `Scrollable`, no number gutter; over the limit
  `diff_view::large_message(added, removed)` and "Show in full" (FR-013–FR-016, FR-019)
- [ ] T032 [US1] Implement `MarkdownView` in `crates/micold-client/src/ui/material/markdown_view.rs`
  (after T029, T030): `MarkdownView::new(&doc, roles).muted(bool).on_link(f).on_copy(f)
  .code_spans(f).copy_state(f).full(f).on_show(f).into()` (`code_spans: CodeId -> Option<&Spans>`,
  `copy_state: CodeId -> Option<CopyState>`, `full: CodeId -> bool`, `on_show: CodeId -> Message`),
  handed to each `CodeBlock`'s `.spans`, `.copy` and `.large`; each settled block in its own iced `lazy` keyed by `(block index, stamp,
  scheme, muted)`; headings on the `TypeRole` title/headline scale, paragraphs through `LinkText`
  (bold, italic, strikethrough, inline code in `Font::MONOSPACE` on `surface_container_high`), lists
  with bullet or number gutters and read-only checkbox glyphs, quotes with the `outline_variant` rule
  on the left, rules with the shared `Divider`, tables with iced's `table` widget (header row in
  label weight, column alignment from the model, `Fill` columns that wrap: FR-003), code blocks
  through `CodeBlock`; an empty document draws nothing
- [ ] T033 [P] [US2] Add `DisclosureCard::meta(text)` in
  `crates/micold-client/src/ui/material/disclosure_card.rs`: a short, never-ellipsized text between
  the summary and the trailing status (FR-017); T026's header case passes
- [ ] T034 [US1] Register the four components in `crates/micold-client/src/ui/material/mod.rs` and
  add showcase entries in `crates/micold-client/src/showcase/catalogue.rs` with samples in
  `crates/micold-client/src/showcase/sections/surfaces.rs` (`MarkdownView` over
  `markdown-subset.md`, `CodeBlock` idle/copied/failed/large, `LinkText`) and
  `crates/micold-client/src/showcase/sections/review.rs` (`InlineDiff` of an Edit, a MultiEdit, a
  Write, a large diff); T025–T027 and `showcase_completeness`, `material_builder_api` pass

**Checkpoint**: M4 done — the four components and the card meta are in the showcase, gated.

---

## Phase 7: Rich renderers in the transcript (US1, US2, US3)

**Goal**: the window draws transcripts with `BlockRenderers::RICH`: formatted replies and
thinking, code blocks with copy, Edit/MultiEdit/Write diffs with `+N −M` headers; tool results and
everything else as #939 draws them; the user guide describes it.

**Independent Test**: `cargo test -p micold-client --test transcript_rich_renderers --test
transcript_renderer_seam --test layout_snapshot`; `cargo test --release -p micold-client --test
transcript_open_budget -- --ignored`; `scripts/gate.sh`; quickstart Part B.

### Tests ⚠️ write first, see them fail

- [ ] T035 [P] [US2] Renderer test `crates/micold-client/tests/transcript_rich_renderers.rs`
  (FR-002, FR-013, FR-018, FR-020–FR-022, contracts/rich-renderers.md RR-1–RR-3) on #939's
  `transcript_renderer_seam.rs` pattern over a replayed `markdown-turn` + `edit-turn` transcript
  and its `RichCache`: with `RICH`, entry order, fold state (Edit/MultiEdit/Write cards start
  folded), statuses and headers equal `PLAIN`'s except the `+N −M` meta of Edit/MultiEdit/Write
  cards, folded and open alike; `assistant_text` goes through the markdown renderer and `thinking`
  through it muted; `tool_result` output equals `PLAIN`'s (a `Read` result included); a streaming
  and a malformed Edit input equal `PLAIN`'s input; a failed Edit keeps its diff and #939's failed
  mark; a cache miss draws `PLAIN`; the lazy key changes when the entry's `stamp` changes and only
  then
- [ ] T036 [P] [US1] Add the rich variant to `crates/micold-client/tests/transcript_open_budget.rs`
  (SC-004, research R12): the 5,000-event transcript from repeated `markdown-turn` and `edit-turn`
  turns, every card open, drawn with `RICH` after `refresh`: first layout + draw < 1 s; each replayed
  delta's update + refresh + view + layout < 100 ms (release; #939 already runs this test in CI's
  release-budget step of `.github/workflows/ci.yml`)
- [ ] T037 [P] [US1] A rich transcript pane state in `crates/micold-client/tests/layout_snapshot.rs`
  (a `markdown-turn` reply with its table at a narrow width wrapping in-cell, a code block, one open
  Edit card) with its reviewed fixture (FR-003)

### Implementation

- [ ] T038 [US1] Implement `BlockRenderers::RICH` in a new `crates/micold-client/src/ui/transcript/rich.rs`
  per contracts/rich-renderers.md *Renderers*: `assistant_text` = `MarkdownView` of the cached
  `Document` (nothing for empty text) with `code_spans`, `copy_state` and `full` read from the
  entry's `Derived.spans`, `Derived.copy` and `Derived.full`, `on_copy` → `CopyCode`, `on_show` →
  `ShowInFull { part: Code(id) }`, `thinking` = the same `.muted(true)`, `tool_input` =
  `InlineDiff` for `Edit`/`MultiEdit`/`Write` when `!streaming` and the cache holds `Diff(Some(_))`
  else `PLAIN.tool_input`, `tool_result` = `PLAIN.tool_result`; `PLAIN` on a cache miss; messages
  only `TranscriptMsg::{OpenLink, CopyCode, ShowInFull}` (RR-1)
- [ ] T039 [US2] In `crates/micold-client/src/ui/transcript/entries.rs` add `meta = "+{added}
  −{removed}"` from `RichCache::edit_counts` to the header view model of Edit/MultiEdit/Write cards,
  drawn with `DisclosureCard::meta` (RR-2, FR-017)
- [ ] T040 [US1] In `crates/micold-client/src/ui/transcript/mod.rs` add the entry's `Derived.stamp`
  to the lazy key (RR-3), register `rich`, and switch the window's renderers from `PLAIN` to `RICH`;
  T035–T037 pass and #939's tests pass unchanged except plain-text assertions of the replaced block
  kinds (SC-007)
- [ ] T041 [US1] Extend *Structured sessions* in `docs/user-guide/worktrees-and-sessions.md`
  (FR-024): formatted replies and thinking, code blocks with highlighting and the copy control
  (Copied / Copy failed), sideways scrolling, Edit/MultiEdit/Write diffs with `+`/`−`, notes and
  `+N −M` headers, "Show in full" for large blocks, and which links open (`http`, `https`,
  `mailto`) and which do not
- [ ] T042 [US3] Run quickstart Part B with the `visual-pass` skill and record it in
  `specs/940-transcript-rich-rendering/visual-pass/` (FR-002–FR-017, US3 scenario 6, SC-006,
  Principle II two-window check)

**Checkpoint**: M5 done — feature complete.

---

## Dependencies & Execution Order

- #939 M6 merged → T001 → Phase 1 → Phase 2 → Phase 3 (M1) → Phase 4 (M2) → Phase 5 (M3) →
  Phase 6 (M4) → Phase 7 (M5).
- Phase 4 needs only Phase 1's T001 and the existing `review::diff`, but merges after M1 so one PR
  is open at a time. Phase 5 needs M1 (`Document`) and M2 (`edit_diff`). Phase 6 needs M1's and
  M2's types (`MarkdownView` draws a `Document`, `InlineDiff` an `EditDiff`) and M3's `CopyState`.
  Phase 7 needs M1–M4.
- Within a phase: tests before the implementation they cover; tasks on the same file in ID order.
- `features_transcript_rich.rs` is written by T017, T018, T019 in that order; `markdown/mod.rs` by
  T005 then T012; `Cargo.toml` by T002 then T013; `layout_snapshot.rs` by T025 then T037.

### Parallel opportunities

- T004 beside T002/T003. Phase 3: T006, T007, T008 together; T009 beside T010.
- Phase 5: T016 and T020 beside T017; T021 beside T022.
- Phase 6: T025, T026, T027 together; T029, T030, T033 together after T028 (T031 after T028).
- Phase 7: T035, T036, T037 together.

## Implementation Strategy

Decisions first, in core, where the recorded fixtures are: the markdown model (M1), then the edit
diff (M2); then the window's render-free cache with its shell glue (M3); the shared components,
gated and in the showcase (M4); and the switch of the transcript to `RICH` last (M5), measured
against SC-004 and documented in the same PR, so `main` never shows half-built rich rendering and
#939's plain renderers keep drawing until `RICH` replaces them.

## Milestones

Each milestone merges to `main` on its own, through one PR (speckit-autopilot). None starts
before #939 M6 has merged (research R14); M1 begins with T001's re-check.

### M1 — Markdown model 🎯 MVP

- **Tasks**: T001–T012
- **Deliverable**: `micold_core::markdown::Document` turns assistant and thinking text into
  headings, styled spans with classified links, lists, quotes, rules, code blocks and tables,
  settled blocks final at every prefix and the open block re-parsed incrementally
- **Satisfies**: US1 scenarios 1–5, 7 and US3 scenarios 2–4 at the model; FR-001, FR-003–FR-008,
  FR-009 and FR-011 (tag, token, copy text); SC-001, SC-002
- **Verify**: `cargo test -p micold-core --test markdown_subset --test markdown_stream` and
  `cargo test -p micold-core --lib markdown`
- **Depends on**: — (#939 M6 merged)
- **Tier**: full

### M2 — Edit diff

- **Tasks**: T013–T015
- **Deliverable**: `micold_core::structured::edit_diff` turns every recorded Edit, MultiEdit and
  Write into hunks that round-trip to the old and new text, with notes and counts, and malformed
  inputs into `None`
- **Satisfies**: US2 scenarios 1–4, 9 at the model; FR-013, FR-014, FR-016, FR-017 (counts),
  FR-018 (fallback), FR-019 (sizes); SC-003
- **Verify**: `cargo test -p micold-core --test edit_diff`
- **Depends on**: M1
- **Tier**: full

### M3 — The window's rich cache

- **Tasks**: T016–T024
- **Deliverable**: each window keeps a render-free cache per structured session that derives
  documents and diffs from the transcript, prunes with it, runs highlight jobs off the update
  thread, copies code with a read-back check and confirmation, and opens web and mail links only
  (the transcript still draws with `PLAIN`)
- **Satisfies**: US1 scenario 5, US3 scenarios 3–4 at the state; FR-004, FR-009, FR-011, FR-012,
  FR-019, FR-023; SC-005; Principle II
- **Verify**: `cargo test -p micold-client --test features_transcript_rich --test
  clipboard_request` and `cargo test -p micold-client --lib ui::syntax`
- **Depends on**: M1, M2
- **Tier**: full

### M4 — Shared rich components

- **Tasks**: T025–T034
- **Deliverable**: `material::{MarkdownView, CodeBlock, InlineDiff, LinkText}` and
  `DisclosureCard::meta` exist with builders, gates and showcase entries; `InlineDiff` and
  `DiffView` share one marker, tint and span painting
- **Satisfies**: Constitution VIII; FR-003, FR-004, FR-010, FR-011, FR-015, FR-016, FR-017
  (component); SC-006
- **Verify**: `cargo test -p micold-client --test showcase_completeness --test
  material_builder_api --test layout_snapshot` and `cargo test -p micold-client --lib
  ui::material`; the showcase's surfaces and review sections show the components
- **Depends on**: M1, M2, M3
- **Tier**: full

### M5 — Rich rendering in the transcript

- **Tasks**: T035–T042
- **Deliverable**: a structured session's transcript draws formatted replies and thinking,
  highlighted code blocks with copy, and Edit/MultiEdit/Write diffs with `+N −M` headers, within
  the SC-004 budget; tool results unchanged; the user guide describes it; quickstart Part B
  recorded
- **Satisfies**: US1 scenarios 1–7, US2 scenarios 1–9, US3 scenarios 1–6; FR-002, FR-013,
  FR-017, FR-018, FR-020–FR-024; SC-004, SC-007
- **Verify**: `cargo test -p micold-client --test transcript_rich_renderers --test
  transcript_renderer_seam --test layout_snapshot`; `cargo test --release -p micold-client --test
  transcript_open_budget -- --ignored`; `scripts/gate.sh`; quickstart Part B
  (`specs/940-transcript-rich-rendering/visual-pass/`)
- **Depends on**: M1, M2, M3, M4
- **Tier**: full
