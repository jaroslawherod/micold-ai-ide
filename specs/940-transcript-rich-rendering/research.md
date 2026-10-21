# Research: Structured Session Rich Rendering (#940)

Every decision builds on #939's design (specs/939-structured-transcript-view/), whose code (its
milestones M1–M6) has not merged when this plan is written. Names of #939 types and modules below
are the ones its plan.md, data-model.md, tasks.md and contracts/ give; the tasks unit of #940 and
its implementation re-check them against the merged code (R14).

## R1. Where markdown is parsed, and with what (FR-001, FR-005–FR-008, Principle I)

- **Decision**: a new render-free core module `micold_core::markdown` parses text into an owned
  block model (data-model.md *Markdown document*) with **`pulldown-cmark` 0.13**
  (`default-features = false`; options `ENABLE_TABLES`, `ENABLE_STRIKETHROUGH`,
  `ENABLE_TASKLISTS` only). The client never parses markdown; it draws the model.
- **Rationale**: the settled-block guarantee (FR-006), the literal fallbacks (FR-001, FR-007) and the
  link rule (FR-004) are decision logic, so Principle I puts them in tested core code, where the
  recorded fixtures already are. pulldown-cmark is CommonMark-compliant with the GitHub table,
  strikethrough and task-list extensions the spec assumes, gives byte offsets for every event
  (`into_offset_iter`, used by R2), has no required dependencies once default features are off, is
  MIT-licensed and actively maintained (it is the parser iced's own `markdown` feature and rustdoc
  use). Vetting (constitution *Dependencies*): one crate plus `unicase` (MIT/Apache-2.0, new to the
  lockfile); `memchr` and `bitflags` are already in it; MIT, releases through 2025.
- **Alternatives**:
  - iced's `markdown` feature (`iced::widget::markdown`) — rejected: its parse lives in the client,
    where the settled guarantee could not be tested in core; its `Content::push_str` re-parses the
    tail by its own rule, which this feature could not pin to FR-006; its `view` borrows the items,
    which iced `lazy` (#939 research R12, `Element<'static, _>`) cannot hold; and it draws code
    blocks and links in its own widgets, outside the shared library (Principle VIII).
  - `comrak` — rejected: an arena AST and a larger dependency tree for the same CommonMark + GFM.
  - `markdown` (markdown-rs) — rejected: mdast output with no streaming offsets; slower in its own
    benchmarks.
  - A hand-written subset parser — rejected: CommonMark's list, emphasis and fence rules are where
    hand parsers go wrong, and SC-001 covers every element.

## R2. Settled blocks and incremental parsing while streaming (FR-005–FR-008, SC-002)

- **Decision**:
  - **Split**: the text is cut into **top-level block slices**: pulldown-cmark is run over the text
    and every top-level `Start` event's byte offset is a cut. A slice runs from its cut to the next
    one (trailing blank lines included).
  - **Settled vs open**: while the entry streams, the last slice is the **open block** and every
    earlier slice is settled; when it stops streaming (#939 `streaming: false`), every slice is
    settled. Text that may still change the kind of the block it is in (a paragraph line that a
    later `---` or `===` would make a heading, a `| a |` row that a separator row would make a
    table, a list that a later item may loosen) is always inside the last slice, because
    pulldown-cmark only starts a new top-level block after the previous one has ended.
  - **Isolation**: each slice is parsed **on its own**. Link reference definitions therefore resolve
    only inside their own block; a reference defined in a later block stays literal text. This is
    what makes a settled block final: nothing later can reach back into it.
  - **Incremental**: `Document::update(text, streaming)` keeps the settled blocks and the byte offset
    `open_from` where the open block starts. When the new text starts with the old text up to
    `open_from`, only `text[open_from..]` is split and parsed again; the new cuts in it settle more
    blocks. Otherwise (a rebuild, or a finished message whose text differs from the streamed one)
    it parses from scratch.
  - **Finished message** (#939 FR-015): the finished text equal to the streamed text gives the same
    slices, so the same blocks (FR-008); `update` with `streaming: false` settles the open block
    with the parse it already has.
- **Tested by** (core, `crates/micold-core/tests/markdown_stream.rs`): for every recorded and
  synthetic markdown fixture and **every byte-prefix** of its text that ends on a char boundary,
  (a) the incremental document after each prefix equals `Document::parse(prefix, true)`;
  (b) every settled block after a prefix is equal to the same block after every longer prefix
  (SC-002, "settled blocks are final"); (c) `Document::parse(full, false)` equals the incremental
  document after the last delta with `streaming: false` (FR-008); and the delta-by-delta replay of
  `markdown-turn.jsonl` through #939's `Transcript` gives the same (SC-002's replay).
- **Rationale**: a top-level cut is the coarsest unit that is final, and parsing each slice alone is
  the simplest rule under which "final" is provable by test rather than by argument. Re-parsing only
  the open block bounds a delta's cost by the open block's size, not the reply's (FR-023, SC-004).
- **Alternatives**: re-parse the whole text per delta and diff the block lists — rejected, O(n²)
  over a long reply and a settled block could still change (reference definitions); hold every
  streaming block as plain text until the entry finishes (#939 R-3 allows it) — rejected by FR-005;
  settle on blank lines only — rejected, a fence may contain blank lines and a list spans them.

## R3. The subset, literal fallbacks and links (FR-001, FR-003, FR-004, FR-007)

- **Decision**:
  - Block kinds: heading (1–6), paragraph, bullet and ordered list (with start number, tight or
    loose, nested, task-list marks as read-only checked/unchecked), block quote, rule, code block
    (fenced with optional tag, or indented), table (header, alignment per column, body rows).
  - Inline styles: bold, italic, strikethrough, inline code, link; soft breaks join with a space and
    hard breaks are kept as a line break.
  - **Literal fallbacks**: raw HTML (block and inline) is kept as its literal source text in a
    paragraph or span; an image becomes the span `description (address)` with no fetch (FR-004,
    Principle IV); footnotes and math are not enabled, so pulldown-cmark leaves them as text. A
    table whose separator row has not arrived is a paragraph until it does; an unclosed fence is a
    code block marked `open: true`; an unmatched `**` stays literal (pulldown-cmark's rule). The
    hidden-text check of SC-001 (`tests/markdown_subset.rs`) asserts that every received character
    that is not formatting punctuation appears in the model's text.
  - **Links**: a link's target is classified with the existing
    `micold_core::link::address::classify`: `Address::Web` and `Address::Mail` are activatable;
    `File` and `NotFollowable` (`javascript:`, relative paths, …) are kept as styled text that is
    not activatable. Bare `http(s)://` addresses in text spans are found with the existing
    `micold_core::link::detect::detect` and become activatable links too (GitHub's autolink
    extension, which Claude Code's output relies on).
- **Rationale**: one classifier for links in terminals and transcripts keeps the "what may be
  opened" rule in one place (feature 031's FR-011). Literal fallbacks keep FR-001's "no received
  text may be hidden".
- **Alternatives**: rendering raw HTML — out of scope; dropping images — rejected, it hides text;
  a transcript-only link scheme list — rejected, duplicates `classify`.

## R4. Code blocks: language, highlighting, copy text (FR-009–FR-011)

- **Decision**:
  - **Tag → grammar token** (core, `markdown::fence_token`): the first word of the info string
    (cut at whitespace, `,` or `{`), lower-cased, through a small alias table (`shell`, `bash`,
    `zsh`, `console` → `sh`; `golang` → `go`; `yml` → `yaml`; `jsonc` → `json`; `c++` → `cpp`;
    `ts`, `rs`, `py` and language names pass through, which syntect's `find_syntax_by_token`
    already matches by extension or name). The tag is kept as written for display (FR-009).
  - **Known language**: the client's existing `ui/syntax.rs` gains `syntax::knows(token) -> bool`
    (a syntax other than plain text exists for it); unknown → no highlight job, plain monospace.
    Diff lines use the existing `syntax::token(path)` for the file's extension (FR-015).
  - **Highlighting**: only **settled** code blocks and **complete** diffs are highlighted, off the
    update thread, in **both schemes at once** (the Changes view's `Spans { light, dark }`, so a
    theme switch needs no new work: US3 s6, FR-023), with the existing `syntax::highlight_lines`
    under a time budget (the `highlight_within` pattern, #771). Until the spans arrive, and for an
    open (streaming) code block, the block is drawn in plain monospace; the arrival bumps the entry's
    render stamp (R8) so its lazy element is rebuilt.
  - **Legibility**: the code surface (`surface_container_high`) joins `syntax::fills`, so
    `syntax::legible` lifts every colour to 4.5:1 on it as well as on the diff fills (SC-006).
  - **Copy text**: the code block's `code` exactly as pulldown-cmark yields it — fence contents, or
    an indented block's lines without their four-space indent — with line breaks and indentation
    kept and no fence or tag. While streaming, the open code block's code so far (FR-011, US3 s4).
- **Rationale**: highlighting is the costliest step; doing it once per settled block, off-thread,
  keeps a delta's update cheap. Reusing `ui/syntax.rs` means one grammar set, one theme pair and one
  legibility rule for diffs in the Changes view and in the transcript.
- **Alternatives**: iced's `highlighter` widget per code block — rejected: it highlights in `view`
  and in one theme; highlighting the open code block on every delta — rejected, its cost grows with
  the block and syntect state cannot be resumed across our re-parse.

## R5. Copy, its confirmation and its failure (FR-011, FR-012)

- **Decision**: the copy control sends `TranscriptMsg::CopyCode { session, key, code }` (`code` is
  the block's `CodeId`). The render-free reducer looks up the current code text, stores
  `CopyState::Pending(ticket)` for that window, session, entry and block, and emits a new
  `Outcome::ClipboardWriteChecked { text, ticket }`. `shell/clipboard.rs::interpret` turns it,
  without a branch (contract C3), into `iced::clipboard::write(text)` chained with
  `iced::clipboard::read()` mapped to `TranscriptMsg::CopyChecked { ticket, read }`. The reducer
  decides: `read == Some(text)` → `Copied`, else `Failed`; then it asks the shell for
  `TranscriptMsg::CopyExpired { ticket }` after `COPY_CONFIRM` (2 s, a `tokio::time::sleep` task
  as `shell/pr_status.rs` does), which returns the state to `Idle` only if the ticket is still the
  current one. The control shows `Icon::Copy` idle, `Icon::CardDone` + "Copied" on success and the
  error role + "Copy failed" on failure.
- **Rationale**: iced's clipboard write reports no error, and FR-012 asks to show a refused write
  (e.g. a Wayland compositor). Reading back what was written is the one platform-neutral check
  (Principle VI) that needs no second clipboard library; it goes through the existing clipboard
  boundary (feature 021, C1–C3), so the copy is testable without a windowing system. Tickets keep a
  stale timer from clearing a newer confirmation.
- **Alternatives**: `arboard::Clipboard::set_text` — rejected: on X11 the selection is lost when the
  `Clipboard` value is dropped unless a thread keeps serving it, and it would be a second clipboard
  path beside iced's; plain `Outcome::ClipboardWrite` with an unconditional "Copied" — rejected by
  FR-012's failure case.

## R6. Edit, MultiEdit and Write diffs (FR-013–FR-018, SC-003)

- **Decision**: a new render-free core module `micold_core::structured::edit_diff`:
  `edit_diff(tool: &str, input_json: &str) -> Option<EditDiff>`.
  - **Shapes** (Claude Code's field names): `Edit {file_path, old_string, new_string,
    replace_all?}`; `MultiEdit {file_path, edits: [{old_string, new_string, replace_all?}]}`;
    `Write {file_path, content}`. Any other tool name, a missing field, a field of the wrong JSON
    kind, or JSON that does not parse → `None`, and the renderer falls back to #939's plain input
    (FR-018). A streaming input is never passed (the renderer checks `streaming` first).
  - **Line diff**: old and new text are split into lines with the line-ending rule of
    `review::diff` (`\n` and `\r\n` both stripped; the existing private `split_lines` becomes
    `pub(crate)` and is reused) and diffed with **`similar` 2** (`TextDiff` over the line slices,
    Myers with a deadline of 50 ms, after which it returns a correct but coarser diff). Each edit
    is one hunk: equal lines are `LineKind::Context`, deletes `Removed`, inserts `Added` (the
    Changes view's `LineKind`). A difference only in the final newline sets the hunk's
    `old_no_newline` / `new_no_newline`, which the view shows as "No newline at end" (Edge Cases).
  - **Write**: one hunk of `Added` lines, labelled "New content" (its previous content is unknown).
  - **Notes**: `replace_all: true` → the hunk says "Replaces every occurrence" (FR-014); equal old
    and new text → "No change"; a Write with empty content → "Writes an empty file" (Edge Cases).
  - **Counts**: `added` and `removed` over all hunks, for the folded header (FR-017).
  - **No numbers**: lines carry no file line numbers (FR-016); hunk headers are the edit's ordinal
    ("Edit 2 of 3") and note, never `@@ -a,b +c,d @@`.
- **Tested by** (core, `crates/micold-core/tests/edit_diff.rs`): each shape; MultiEdit order; notes;
  CRLF and final-newline cases; malformed inputs (missing, wrong kind, not JSON, other tool name);
  and the **round trip of SC-003** over every Edit/MultiEdit/Write in the fixtures and in a
  synthetic set: dropping `Added` lines reproduces `old_string`, dropping `Removed` lines reproduces
  `new_string`, line for line.
- **Rationale**: the decision logic is pure and fixture-tested in core. `similar` is the
  maintained, dependency-free Rust line differ (Apache-2.0, by the author of `insta`), with a
  deadline so a pathological edit cannot stall update (SC-004).
- **Alternatives**: a hand-written Myers — rejected, more code to own for the same result;
  `imara-diff` — rejected, a lower-level token-interning API with no deadline; `git diff
  --no-index` on temp files — rejected, a process and file I/O to show a tool call; reusing
  `review::diff::Hunk`/`DiffLine` — rejected, their old/new numbers and `@@` header are exactly what
  FR-016 forbids, so only `LineKind` is shared.

## R7. Size limits and "show in full" (FR-019, Edge Cases)

- **Decision**: the Changes view's limits (`micold_core::review::limits`) apply per diff and per code
  block: above `MAX_CHANGED_LINES` (5,000) lines (a diff's added + removed, a code block's lines) or
  `MAX_VERSION_BYTES` (2 MiB) of text, the block shows its counts (the existing
  `diff_view::large_message` wording for a diff, "Large code block: N lines" for code) and a
  **Show in full** button; it is neither highlighted nor laid out until pressed. The choice is
  `TranscriptMsg::ShowInFull { session, key, part }`, held per window and session in the render
  cache (R8), dropped with the entry.
- **Rationale**: the spec's assumption ("reuse the Changes view's limits where they apply") and one
  rule for the user.
- **Alternatives**: virtualising a long code block with `VirtualRows` — rejected for now, it needs
  its own vertical scroll inside the transcript's; kept as a follow-up if the budget test (R12)
  fails at the limit.

## R8. The render cache: what is derived, when, and how it is pruned (FR-022, FR-023, Principle II)

- **Decision**: #939's `BlockCache` (contracts/block-renderers.md R-4) gets its content from a new
  render-free client module `features/transcript_rich.rs` (`RichCache`), so the decision logic of
  filling and pruning is reducer-tested; `ui/transcript/blocks.rs` keeps the name `BlockCache` as a
  re-export so the seam's shape is unchanged.
  - **Per window, per session**: `RichCache` sits beside #939's `structured_transcript::State` in
    the window's `State`; one `SessionRich` per `SessionId`, dropped by the same two `forget` paths
    (session deleted or forgotten, #939 FR-019, FR-020).
  - **Fill**: after each applied `StructuredEvents` message, `RichCache::refresh(session,
    &Transcript)` walks the entries and, for each assistant-text, thinking or Edit/MultiEdit/Write
    card whose `revision` differs from the cached one, derives the new value: `Document::update`
    for text (incremental, R2) and `edit_diff` for a card whose input is complete. It returns the
    highlight jobs for newly settled known-language code blocks and newly complete diffs; the shell
    runs them on blocking tasks and their results come back as `TranscriptMsg::Highlighted`.
  - **Key**: entries are held by `(SessionId, EntryKey)` with the `revision` they were derived from,
    which is #939's `(SessionId, EntryKey, revision)` key with older revisions replaced in place,
    not kept.
  - **Stamp**: every change to an entry's derived drawing that is not a revision change (spans
    arrived, copy state, show in full) bumps that entry's `stamp`. #939's lazy key `(EntryKey,
    revision, open, scheme)` gains the stamp, so the lazy element is rebuilt then and only then.
  - **Prune**: after a refresh, cached keys not present in the transcript are dropped (eviction,
    rebuild, #939 FR-020, FR-021); a rebuild re-derives from scratch and gives the same drawing
    (Edge Cases).
- **Rationale**: the cache is where R-4 of #939's seam puts derived state; holding it render-free
  makes refresh, prune, isolation and copy state testable in `tests/features_transcript_rich.rs`.
- **Alternatives**: deriving in `view` — rejected by #939 R10 (parse outside `view`) and SC-004;
  keeping derived data inside core's `Entry` — rejected, it would change #939's model for a drawing
  concern (FR-022).

## R9. Shared components (Principle VIII)

- **Decision** (all in `ui/material/`, builders ending in `.into()`, showcase entries, both schemes):
  - **`MarkdownView`** — `MarkdownView::new(&doc, roles).muted(bool).on_link(f).on_copy(f)
    .code_spans(f).into()`: draws a `Document` block by block, each settled block in its own iced
    `lazy` keyed by `(block index, stamp, scheme, muted)` so only the open block re-lays out per
    delta (R12). Headings use the type scale (`TypeRole` title/headline roles), paragraphs
    `TypeRole::Body` through the shared `Text`/`rich_text` spans (bold, italic, strikethrough,
    inline code in `Font::MONOSPACE` on `surface_container_high`), lists with bullet or number
    gutters and read-only checkbox glyphs, quotes with the `outline_variant` rule on the left, rules
    with the shared `Divider`, tables with iced's `table` widget (header row in label weight,
    column alignment from the model, cells wrapping in `Fill` columns: FR-003).
  - **`CodeBlock`** — `CodeBlock::new(code, roles).language(tag).spans(&scheme_spans)
    .copy(state, on_copy).large(lines, on_show).into()`: monospace lines, unwrapped, inside a
    horizontal-only `material::Scrollable` (FR-010; iced aligns a vertical wheel delta away on a
    horizontal-only scrollable, so the transcript still scrolls — pinned by a gate, R12), the tag
    shown in the header row, and an `IconButton` copy control wrapped in `TakesTheKeyboard`
    (pointer and keyboard, FR-011) with its tooltip/label from the copy state (R5).
  - **`InlineDiff`** — `InlineDiff::new(&edit_diff, roles).spans(&scheme_spans).large(on_show)
    .into()`: the file path header, then per hunk its note row and its lines, each with a `+`/`−`
    marker cell and the diff tint, unwrapped in a horizontal-only `Scrollable`, no number gutter
    (FR-016). Its marker, tint and span painting are **extracted** from `DiffView` (`tint`, the
    marker cell and the span-over-tint `rich_text` builder become `pub(super)` helpers both use), so
    the two cannot drift.
  - **`LinkText`** — `rich_text` with link spans that open on press (`on_link_click`) and one
    keyboard focus stop per activatable link (Tab moves between them, Enter opens), built on
    `TakesTheKeyboard`; non-activatable links are drawn as underlined text without a link (FR-004).
  - **`DisclosureCard::meta(text)`** (extends #939's card): a short, never-ellipsized text between
    the summary and the trailing status — the `+N −M` of FR-017, with `+`/`−` signs so greyscale
    reads. Joins `disclosure_card_header_fits`.
  - **Gates**: `composition_contrast` (code, inline code, link, marker and note text on their fills,
    both schemes, SC-006), `style_outline_discipline` (code surface draws no border),
    `ripple_clipping` (copy control), new `tests/gates/code_block_fits.rs` (copy control and tag
    stay in the header row at a narrow width; a long line scrolls inside, the block does not widen
    the transcript), `tests/gates/inline_diff_marker_fits.rs` (markers in their cell, both schemes).
- **Rationale**: the spec's Principle VIII constraint names exactly these three components; the
  diff painting is shared with the Changes view rather than copied.
- **Alternatives**: `DiffView` as is — rejected: it borrows its `FileDiff` (`DiffView<'a>`), which
  iced `lazy`'s `Element<'static, _>` cannot hold; it virtualises rows inside its own vertical
  scroll, which nests badly in the transcript's; and it draws file line numbers FR-016 forbids. Its
  row painting is reused instead (Complexity Tracking). iced's `markdown::view` — see R1.

## R10. The renderers (FR-001, FR-002, FR-013, FR-018, FR-021, FR-022)

- **Decision**: `ui/transcript/rich.rs` defines `BlockRenderers::RICH`:
  - `assistant_text`: the cached `Document` through `MarkdownView`; an empty text draws nothing.
  - `thinking`: the same with `.muted(true)` (FR-002).
  - `tool_input`: for `Edit`/`MultiEdit`/`Write`, when `!streaming` and the cache holds an
    `EditDiff`, `InlineDiff`; otherwise `BlockRenderers::PLAIN.tool_input` (FR-018: streaming,
    malformed or other tools). The card's status and failure stay #939's, so a failed result never
    hides the diff (US2 s7).
  - `tool_result`: `BlockRenderers::PLAIN.tool_result`, unchanged (FR-021).
  The window's `BlockRenderers` value becomes `RICH`. A cache miss (not yet derived) draws `PLAIN`.
- **Tested by**: #939's `transcript_renderer_seam.rs` pattern, extended in a new
  `tests/transcript_rich_renderers.rs`: with `RICH`, entry order, fold state and headers equal
  `PLAIN`'s except the FR-017 meta; tool results equal `PLAIN`'s output; a malformed Edit equals
  `PLAIN`'s input.
- **Rationale**: the seam was designed for this swap (#939 R10); falling back to `PLAIN` per call
  keeps FR-018 in one place.
- **Alternatives**: a trait object per kind — #939 R10 rejected it.

## R11. Opening links (FR-004, Principle IV, VI)

- **Decision**: a link press sends `TranscriptMsg::OpenLink { url }`; the reducer re-classifies it
  (`link::address::classify`) and, for `Web`/`Mail` only, emits the existing
  `Outcome::OpenLink(OpenRequest::Url(url))`, which `shell/links.rs::perform` hands to the existing
  per-platform `LinkOpener` (feature 031); its `LinkOpenFinished` answer reports a failure as
  terminal links do. No confirmation dialog: a web or mail address opens nothing on this machine's
  files (the sandbox confirmation of feature 031 applies to host paths only, which are never
  activatable here).
- **Rationale**: one opener, already cross-platform and tested; the second classify is defence in
  depth for a message built elsewhere.
- **Alternatives**: `open`/`webbrowser` crates — rejected, the app has an opener.

## R12. Performance (SC-004, FR-023)

- **Decision**: per-delta cost is bounded by the open block (R2 incremental parse, R8 refresh of
  changed revisions only, per-block `lazy` in `MarkdownView`); highlighting runs off-thread (R4);
  large parts wait behind "Show in full" (R7). #939's release-only `transcript_open_budget.rs`
  gains a rich variant: the 5,000-event transcript built from repeated `markdown-turn` and
  `edit-turn` turns, every card open, drawn with `RICH`, first layout + draw < 1 s; each replayed
  delta's update + view + layout < 100 ms. A gate `tests/gates/code_block_wheel.rs` checks that a
  vertical wheel over a code block scrolls the transcript, not the block.
- **Rationale**: SC-004 is #939's budget unchanged; the same measuring test keeps one definition.
- **Alternatives**: highlight while streaming — rejected (R4); a windowed list — #939 R12's fallback.

## R13. Fixtures (SC-001–SC-003)

- **Decision**: two new **recorded** fixtures in `crates/micold-core/tests/fixtures/stream-json/`
  with the existing `scripts/capture-stream-json.sh`: `markdown-turn.jsonl` (a prompt asking for a
  reply using every element of the subset, with code blocks in a known language, an unknown one and
  none, and a table; `--include-partial-messages` as every recorded run) and `edit-turn.jsonl` (a
  prompt making Claude Code Write a file, Edit it once with `replace_all`, and MultiEdit it twice).
  One **synthetic** `markdown-subset.md` (plain markdown, not stream-json) covers what a recording
  may not: raw HTML, an image, a `javascript:` and a relative link, a reference definition in a
  later block, CRLF text, an unclosed fence, unmatched `**`, a table without its separator. The
  README's table gains the three rows. If no logged-in `claude` is available to the implementer,
  the recorded ones are an escalation, not hand-written.
- **Rationale**: SC-003 asks for the recorded Edit/Write calls, which no existing fixture has.
- **Alternatives**: hand-written stream-json for Edit/Write — rejected, SC-003 names recorded calls
  and a hand-written input shape could drift from Claude Code's; one fixture for both — rejected,
  a failure in one would hide the other's coverage.

## R14. Sequencing behind #939 (Constraints)

- **Decision**: #940's implementation starts only after #939's last milestone (M6, "The pane shows
  the transcript") has merged to `main`. The tasks unit of #940 may be written before that; its
  first implementation step re-checks this plan's #939 names (`Transcript`, `Entry`, `EntryKey`,
  `revision`, `ToolCard`, `BlockRenderers`, `BlockRef`, `RenderCtx`, `BlockCache`, `TranscriptMsg`,
  `features/structured_transcript.rs`, `ui/transcript/{mod,entries,blocks}.rs`,
  `transcript_open_budget.rs`, `DisclosureCard`) against the merged code with `LSP`, and records any
  rename in this file before writing tests. Core modules (`markdown`, `edit_diff`) depend only on
  #939's `Transcript` for their replay tests and could start after #939 M2, but the flow keeps one
  PR open at a time, so they wait too.
- **Rationale**: the spec's constraint ("#939's milestones must have landed before this feature's
  code can"); planning against names that may still move is safe only with that re-check.
- **Alternatives**: start the core milestones (M1, M2) before #939 M6 merges — rejected, the flow
  keeps one PR open at a time and M1's replay test needs #939's `Transcript`; plan against the code
  only once it lands — rejected, the design of both can be reviewed now and only names may move.
