---
description: "Tasks for #1550 — structured view: select and copy transcript text"
---

# Tasks: Structured View — Select and Copy Transcript Text

**Input**: `specs/1550-transcript-text-selection/` — [plan.md](plan.md), [spec.md](spec.md),
[research.md](research.md), [data-model.md](data-model.md), [contracts/](contracts/),
[quickstart.md](quickstart.md)

**Tests**: mandatory (Constitution I). Every behaviour task has a failing test task before it. The
contract IDs (S-n, T-n, A-n, K-n, M-n, C-n) name the cases each test task must cover. Final drawing
uses the GUI exception: the parity test, the pixel-equality and contrast gates and the quickstart
Part B visual pass cover it.

**Documentation**: the user guide's *Structured sessions* section gains selecting, the copy keys and
the context menu in M4b, when they become visible, and the copy controls in M5, with #939's
out-of-scope note (Constitution VII, FR-021).

**Cross-platform**: one implementation; the only platform difference, Cmd vs Ctrl, is the `mac`
parameter of `transcript_key`, tested both ways on every host (research R8); copied text uses LF on
every platform; grapheme snapping is in core (research R4).

**Reuse**: the clipboard write and read-back are #940's `Outcome::ClipboardWriteChecked` and copy
tickets (research R9, R10); the context menu is `micold_material::ContextMenu` (`menu.rs`); copy
controls are the `IconButton` copy control `CodeBlock::copy` uses; the shown strings of footers,
notices and plain blocks are the existing `footer_view`, `compacted_text` and `plain_text`
(`ui/transcript/entries.rs`, `ui/transcript/blocks.rs`), moved or re-exported, never copied.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: can run in parallel (different files, no dependency on an unfinished task)
- **[Story]**: the user story of spec.md the task serves (US1–US4)

---

## Phase 1: Setup

- [x] T001 Add the module `crates/micold-core/src/structured/selection.rs` (module doc pointing to
  `specs/1550-transcript-text-selection/contracts/selection-model.md`), `pub mod selection;` in
  `crates/micold-core/src/structured/mod.rs`, and the integration test file
  `crates/micold-core/tests/structured_selection.rs` using the transcript fixture helpers of
  `crates/micold-core/tests/support/stream_json.rs` (extend them there, do not copy them)

---

## Phase 2: Foundational — positions and selections in core (blocks every story)

**Purpose**: the render-free selection model every later milestone draws, copies and clears from.

### Tests ⚠️ write first, see them fail

- [x] T002 [P] Position tests in `crates/micold-core/tests/structured_selection.rs`
  (contracts/selection-model.md *S-1*, *S-4*; data-model.md *RowPos*, *TextPos*):
  `RowPos::Omitted < Entry(a, Body) < Entry(a, Prompt(0)) < Entry(a, Prompt(1)) < Entry(b, Body) <
  End(0) < End(1)` for `a < b`; `TextPath` orders lexicographically (`[1] < [1, 0] < [2]`);
  `TextPos` orders by row, then path, then offset; a range from the omitted notice into an entry,
  and one from an entry across a request row under it into the next entry and on into an at-end
  request block, list positions in drawing order; `snap` returns a grapheme boundary on CJK, emoji
  ZWJ sequence and combining-mark fixtures, leaves a boundary unchanged and clamps an offset past
  the end to `len`
- [x] T003 Selection tests in the `tests` module of `crates/micold-core/src/structured/selection.rs`
  (contracts/selection-model.md *API (core)*; data-model.md *Selection*, *RowSelection*):
  `Selection::start` then `extend` backwards gives normalised `bounds()`; `is_empty` iff the
  expanded start equals the end; `in_row` gives `Whole` for a row strictly inside, `From`/`To` for
  the first/last row, `Within` when both ends are in one row, `None` outside; `range_in` for each
  variant with a path before, equal to and after each end, clamped to `len`, and `None` for an
  empty slice

- [x] T004 Stability tests in `crates/micold-core/tests/structured_selection.rs`
  (contracts/selection-model.md *S-5*, FR-019, SC-005): on a transcript built from a recorded
  fixture, a selection's `bounds()` and its `in_row` parts for every entry in range are unchanged
  after appending entries, after growing the last entry past the focus, and after changing an
  entry outside the range; every position in an appended entry orders after the focus

### Implementation

- [x] T005 Implement `TextPath(Vec<u16>)`, `Under { Body, Prompt(u32) }`, `RowPos { Omitted,
  Entry(EntryId, Under), End(u32) }`, `TextPos { row, path, offset }` (derived `Ord` in field
  order), `snap` (`unicode-segmentation`) in `crates/micold-core/src/structured/selection.rs`;
  T002 passes
- [x] T006 Implement `Granularity { Char, Word, Line }`, `Selection { anchor, focus, granularity,
  dragging }` with `start`, `extend`, `is_empty`, `bounds`, `in_row`, and `RowSelection { Whole,
  From(TextPath, usize), To(TextPath, usize), Within((TextPath, usize), (TextPath, usize)) }`
  (`Hash` + `Eq`, it joins the entry's `lazy` key) and `range_in` in
  `crates/micold-core/src/structured/selection.rs`; offsets are snapped at construction and an
  empty selection is a value, not a flag pair (Constitution V); T003 and T004 pass

**Checkpoint**: positions order in reading order and a selection splits into per-row parts.

---

## Phase 3: User Story 1 — shown text in core (P1) 🎯 MVP

**Goal**: what each unit copies is its visible text, computed without a renderer; a selection is
stable while entries are appended or grow.

**Independent Test**: `cargo test -p micold-core --test structured_selection` and `cargo test -p
micold-core --lib markdown structured::selection`.

### Tests ⚠️ write first, see them fail

- [x] T007 [P] [US1] Shown-text tests in the `tests` module of `crates/micold-core/src/markdown/mod.rs`
  (contracts/selection-model.md *S-3*, research R2, R3, FR-011): `shown_text(&Inlines)` drops `*`,
  `_` and backticks of emphasis, strong and inline code, keeps a link's shown text and never its
  target, joins soft and hard breaks as the view draws them; `list_marker(start, k)` is `"•"` for a
  bullet list and `"{start + k}."` for an ordered one (`start` 1 and 7)
### Implementation

- [x] T008 [US1] Implement `shown_text` in `crates/micold-core/src/markdown/inline.rs` and
  `list_marker` in `crates/micold-core/src/markdown/block.rs`, both re-exported from
  `crates/micold-core/src/markdown/mod.rs`; `crates/micold-client/src/ui/material/markdown_view.rs`
  `list` draws `micold_core::markdown::list_marker` instead of its own format (no visual change:
  `cargo test -p micold-client --test layout_snapshot` unchanged); T007 passes

**Checkpoint**: shown text exists in core for every markdown run and list marker.

---

## Phase 4: User Story 4 — word and line granularity in core (P3)

**Goal**: double- and triple-click ranges and their drag extension, as pure text rules.

**Independent Test**: `cargo test -p micold-core --test structured_selection` (granularity cases).

### Tests ⚠️ write first, see them fail

- [x] T009 [US4] Granularity tests in `crates/micold-core/tests/structured_selection.rs`
  (contracts/selection-model.md *S-11*, research R4, FR-005): `word_at` selects a word, a
  whitespace run and a punctuation run as their own runs, a CJK word and an emoji whole, and works
  at offset 0 and `len`; `source_line_at` gives the `\n`-delimited line, the last line without a
  trailing `\n`, and an empty line; a `Selection` started with `Granularity::Word` (or `Line`) and
  its expand range, then extended to a later and an earlier position, covers whole words (lines)
  at both ends

### Implementation

- [x] T010 [US4] Implement `word_at` (`split_word_bound_indices`), `source_line_at` and the
  granularity expansion in `Selection::start`, `extend` and `bounds` in
  `crates/micold-core/src/structured/selection.rs`; T009 passes

---

## Phase 5: User Story 3 — copy-control values in core (P2)

**Goal**: the full identifying value behind a tool card's summary.

**Independent Test**: `cargo test -p micold-core --lib structured::transcript::summary`.

### Tests ⚠️ write first, see them fail

- [x] T011 [P] [US3] `work_value` tests in the `tests` module of
  `crates/micold-core/src/structured/transcript/summary.rs` (data-model.md *Copyable value*,
  research R10, FR-015): for every tool `work_field` names, `work_value(name, input_json)` is that
  field's string uncut, multi-line commands on all their lines, longer than the summary's cut;
  `None` for a tool without a work field, a missing field and malformed JSON

### Implementation

- [x] T012 [US3] Implement `work_value(name, input_json) -> Option<String>` in
  `crates/micold-core/src/structured/transcript/summary.rs`, reading the field through the existing
  `work_field` (no second field table); T011 passes

---

## Phase 6: User Stories 1 and 2 — the window's selection state and the copy keys (render-free)

**Goal**: each window keeps one selection per session, clears it by FR-019, assembles the copied
text, and arbitrates the copy keys against the interrupt; nothing draws or starts a selection yet.

**Independent Test**: `cargo test -p micold-client --test features_transcript_selection --test
features_composer --test features_transcript_rich --test features_are_render_free`.

### Tests ⚠️ write first, see them fail

- [x] T013 [US1] Copy-text tests in the new `crates/micold-client/tests/features_transcript_selection.rs`
  (contracts/selection-model.md *API (client)*, *S-2*, *S-3*; data-model.md *Unit*, *TranscriptView*;
  research R2, R3; FR-002, FR-010, FR-011): `rows(view)` lists the omitted notice, each entry's
  body and the request/resolution rows under it, then the at-end request blocks, in drawing order;
  `units(row, view)` gives, per entry kind: user message text; markdown headings, paragraphs, list
  markers as their own units before the item text, table cells joined by a tab and rows by `\n`,
  inline code without backticks, links as shown text; plain blocks through `plain_text` (tabs
  expanded); a folded tool card's name and summary only, an unfolded one's input and each result
  part, `[image not shown]` captions; a code block's lines in full; an inline diff's path, hunk
  labels and line text without gutter marker or number; footers and notices as drawn; the omitted
  notice's line; a request block's descriptive text without button labels or input fields;
  `copy_text` slices the first and last unit at the two offsets and joins units and rows with one
  `\n`, never splitting a grapheme
- [x] T014 [US1] Implement `TranscriptView`, `rows`, `entry_units`, `units` and `copy_text` in the
  new `crates/micold-client/src/features/transcript_selection.rs` (`pub mod` in
  `crates/micold-client/src/features/mod.rs`), assigning `TextPath`s in the renderers' drawing
  order; move or re-export `plain_text`, `footer_view` and `compacted_text` out of
  `crates/micold-client/src/ui/transcript/` so the module stays render-free; T013 passes
- [x] T015 [US1] State tests in `crates/micold-client/tests/features_transcript_selection.rs`
  (contracts/selection-model.md *S-5*–*S-11*, data-model.md *State transitions*, research R7):
  `Press` then `Hit`s then `Release` holds the selection; a full turn of fixture events streamed
  after it leaves `copy_text` unchanged (SC-005); cleared after a rebuild, eviction of the anchor
  or focus entry, an omitted-count change with an endpoint in the notice, a prompt row appearing,
  leaving or changing at or between the bounds, a `Toggle` of an entry in range, a changed
  selected substring of an entry in range (#939 FR-015), `forget(session)`, and `retain_shown`
  without the session; kept by a toggle or change outside the range; two sessions and two windows
  never affect each other's selection or menu (S-7); `Release` and a click without drag emit no
  clipboard outcome (S-8) and the click clears (S-9); `SelectAll` spans the first unit's start to
  the last unit's end and is a no-op on an empty transcript (S-10); `Hit` with a double click
  selects `word_at`, with a triple click the reported line in prose units and `source_line_at` in
  code and diff units (S-11); `Context` opens the menu at its origin and `MenuDismissed` closes it,
  neither changing the selection (M-3)
- [x] T016 [US1] Implement `State` (`HashMap<SessionId, SessionSelection { selection, rebuilds,
  menu }>`), `SelectMsg { Hit, Press, Edge, Release, Context, MenuDismissed, CopySelection,
  SelectAll }` and `update` (gesture, granularity, select-all, menu) in
  `crates/micold-client/src/features/transcript_selection.rs`; `TranscriptMsg::Select(SelectMsg)`
  in `crates/micold-client/src/features/structured_transcript.rs`, applied in
  `crates/micold-client/src/app.rs`, which builds the `TranscriptView`; a `Hit` without a `Press`
  starts nothing; `CopySelection` emits nothing until T023; T015's gesture, S-7–S-11 and menu
  cases pass
- [x] T017 [US1] Implement the clearing rules in
  `crates/micold-client/src/features/transcript_selection.rs`: `applied` after each
  `StructuredEvents` (rebuild, eviction, omitted count, and the selected substring compared only
  for changed entries in range, research R7), the prompt-change hook, the toggle hook called from
  `Toggle` in `crates/micold-client/src/features/structured_transcript.rs`, `forget` and
  `retain_shown`; the rest of T015 (S-5, S-6) passes
- [x] T018 [US1] Two-pane test in `crates/micold-client/tests/features_transcript_selection.rs`:
  it drives `retain_shown` with the sessions of `Workspace::session_ids_of_project` and the shown
  `TerminalRef`s (only `SessionProcess::Primary` counts): switching one pane to another session
  clears that session's selection and keeps the other pane's, a `swap` keeps both, another
  project's selections are untouched (S-6)
- [x] T019 [US1] `crates/micold-client/src/shell/panes.rs` `sync` calls
  `transcript_selection.retain_shown(&sessions, &now)` with the active project's sessions and its
  `SessionProcess::Primary` terminals; T018 passes
- [x] T020 [P] [US2] Key tests in `crates/micold-client/tests/features_composer.rs`
  (contracts/keys-menu-copy.md *K-1*–*K-11*, research R8, FR-008, FR-009, FR-019a, SC-002):
  every row of the K table with `mac` true and false, `selecting` and `focused` each true and
  false, busy and idle; K-8 and K-11 each with `focused` true and false; the existing #941 and
  BUG-1403 cases updated to the new signature with unchanged results
- [x] T021 [US2] Implement `transcript_key(press, phase, selecting, focused, mac)` and
  `KeyIntent::{CopySelection, SelectAll}` in `crates/micold-client/src/features/composer.rs`;
  `FieldId::Transcript(SessionId)` in `crates/micold-client/src/features/window.rs` (its join in
  `crates/micold-client/src/ui/focus.rs` is T041's: nothing reports the transcript's focus before
  `SelectionArea`); `transcript_interrupt` becomes `transcript_keys` in
  `crates/micold-client/src/ui/composer.rs`, subscribed while the active session's turn can be
  interrupted or `focused_field == Some(FieldId::Transcript(session))`, its one call site passing
  `cfg!(target_os = "macos")`; T020 passes
- [x] T022 [US2] Copy tests in `crates/micold-client/tests/features_transcript_selection.rs` and
  `crates/micold-client/tests/features_transcript_rich.rs` (contracts/selection-model.md *S-12*,
  keys-menu-copy.md *M-2*, research R9, FR-008, FR-014, FR-019a): `CopySelection` with a
  non-empty selection emits `Outcome::ClipboardWriteChecked` with `copy_text` and a
  `CopyTarget::Selection` ticket and keeps the selection; with an empty one emits nothing; a
  failed or `None` read-back sets that session's composer notice ("Copy failed") and keeps the
  selection; the menu's Copy sets `focused_field` to `FieldId::Transcript(session)` and copies;
  #940's code-block copy tests pass unchanged (C-6)
- [x] T023 [US2] Implement `CopyTarget { Code(CodeId), Selection }` keying `Derived::copy` and the
  `Copying` ticket record in `crates/micold-client/src/features/transcript_rich.rs` (the tool and
  diff targets follow in T044); the copy outcome and the failure notice through
  `Composers::notice` in `crates/micold-client/src/features/transcript_selection.rs` and
  `crates/micold-client/src/app.rs` (the shell's existing `shell/clipboard.rs` performs the
  checked write); T022 passes

**Checkpoint**: the selection, its clearing, its copied text and the copy keys work in the state;
nothing is drawn.

---

## Phase 7: User Stories 1 and 4 — shared components (`micold-material`)

**Goal**: `SelectableText` and `SelectionArea`, and the `.selectable` builder on the four leaves.

**Independent Test**: `cargo test -p micold-material` and `cargo test -p micold-client --test
showcase_completeness --test material_builder_api --lib ui::material`.

### Tests ⚠️ write first, see them fail

- [x] T024 [P] [US1] `SelectableText` tests by simulated events in the `tests` module of the new
  `crates/micold-material/src/selectable_text.rs` with `crates/micold-material/src/test_support.rs`
  (contracts/selectable-text.md *T-1*–*T-6*): a left press publishes `on_hit` with the grapheme
  boundary nearest the pointer as an offset into the whole text (line start + in-line index, on
  the second and third hard line too) and is not captured; a move with its own button down
  publishes again only when the offset changed; `line` is the visual row's byte range on wrapped
  text; a press on a link span publishes the link message and no `on_hit`; the widget operation
  reports `(slot, text)` equal to the drawn text; double and triple clicks carry `Click::Double`
  and `Click::Triple`
- [x] T025 [P] [US1] `SelectionArea` tests by simulated events in the `tests` module of the new
  `crates/micold-material/src/selection_area.rs` (contracts/selectable-text.md *A-1*–*A-5*): a press
  a child captured publishes nothing; an uncaptured left press takes focus and publishes
  `on_press(clicks)`; while dragging with the cursor above (below) the bounds, children see the
  cursor clamped to the top (bottom) row and `on_edge(Up)` (`Down`) is published per requested
  frame; a right press, on text or a control, publishes `on_context(origin)` and no `on_press`; a
  left release publishes `on_release` only
- [x] T026 [US1] Leaf tests: `.selectable(slot, range, on_hit)` on `Text` and `LinkText` in the
  `tests` modules of `crates/micold-material/src/text.rs` and `crates/micold-material/src/link_text.rs`,
  and on `CodeBlock` and `InlineDiff` in those of `crates/micold-client/src/ui/material/code_block.rs`
  and `crates/micold-client/src/ui/material/inline_diff.rs` (contracts/selectable-text.md *T-4*,
  *Adopted through*; US1 scenario 6): with `range = None` each draws pixel-equal to the leaf
  without `.selectable` (gate); a selected range draws on `primary_container` in
  `on_primary_container`, wrapped rows included; `CodeBlock` reports and copies full lines while
  scrolled sideways; `InlineDiff` exposes the path, hunk labels and line text without gutter;
  callers that do not call `.selectable` are unchanged
- [x] T027 [P] [US1] Contrast gate in `crates/micold-client/src/ui/material/composition_contrast.rs`
  (research R5 *Colours*, FR-020, SC-006): `on_primary_container` on `primary_container` ≥ 4.5:1 as
  composed on every transcript surface (prose, code surface, diff added and removed lines, failed
  card), light and dark, and the highlight fill differs from each of those surfaces

### Implementation

- [x] T028 [US1] Implement `SelectableText`'s layout and pointer in
  `crates/micold-material/src/selectable_text.rs` (one paragraph per hard line,
  `Paragraph::hit_test`, `Hit { offset, click, line }`, links as `LinkText`, the `(slot, text)`
  operation), exported from `crates/micold-material/src/lib.rs`; T024 passes
- [x] T029 [US1] Implement `SelectableText`'s highlight in
  `crates/micold-material/src/selectable_text.rs`: spans split at the selection ends and cached in
  tree state against the range, `Paragraph::span_bounds` rectangles on `primary_container`, selected
  glyphs in `on_primary_container`, nothing extra drawn with `selected = None` (T-4)
- [x] T030 [US1] Implement `SelectionArea` (`operation::Focusable`, the clamped cursor, `on_edge`
  per requested redraw, `on_context`, `on_release`, `on_focus_change`) in
  `crates/micold-material/src/selection_area.rs`, exported from `crates/micold-material/src/lib.rs`;
  T025 passes
- [x] T031 [US1] Implement `.selectable(slot, range, on_hit)` on `Text`
  (`crates/micold-material/src/text.rs`), `LinkText` (`crates/micold-material/src/link_text.rs`),
  `CodeBlock` (`crates/micold-client/src/ui/material/code_block.rs`, each line) and `InlineDiff`
  (`crates/micold-client/src/ui/material/inline_diff.rs`), swapping the paragraph for
  `SelectableText`; T026 and T027 pass
- [x] T032 [US1] Showcase entries for `SelectableText` (with a selected range) and `SelectionArea`
  in `crates/micold-client/src/showcase/sections/atoms.rs` and
  `crates/micold-client/src/showcase/catalogue.rs`; `showcase_completeness` and
  `material_builder_api` pass (Constitution VIII)

**Checkpoint**: the shared components exist, tested; no transcript uses them yet.

---

## Phase 8: User Story 1 — selectable leaves in the transcript (not yet user-visible)

**Goal**: every transcript row draws its units through the selectable leaves with the slots and
ranges of the model; with no `SelectionArea` nothing starts a selection, so the transcript looks
and behaves as before.

**Independent Test**: `cargo test -p micold-client --test transcript_selection_parity --test
layout_snapshot` and `cargo test --release -p micold-client --test transcript_open_budget`.

### Tests ⚠️ write first, see them fail

- [X] T033 [P] [US1] Parity test in the new `crates/micold-client/tests/transcript_selection_parity.rs`
  (research R2, contracts/selectable-text.md *T-6*, FR-002, SC-001): every entry kind of the
  fixture transcripts, folded and unfolded, through `BlockRenderers::RICH` and `PLAIN`, plus the
  omitted notice and request/resolution rows: the `(TextPath, text)` list the widget operation
  collects from each row equals `units(row, view)`; a folded card yields its header units only
- [X] T034 [P] [US1] Budget variant in `crates/micold-client/tests/transcript_open_budget.rs`
  (research R12, SC-003): on a transcript at `MAX_EVENTS`, select-all plus `copy_text` under 1 s,
  and one drag step (`Hit` → `update` → `view` of the changed entries) under 100 ms, release build

### Implementation

- [X] T035 [US1] `RenderCtx.selection: Option<&RowSelection>` in
  `crates/micold-client/src/ui/transcript/blocks.rs` and the entry's `lazy` key in
  `crates/micold-client/src/ui/transcript/mod.rs` gaining `in_row(&RowPos::Entry(id, Under::Body))`
  (research R6); the `PLAIN` renderers in `blocks.rs` draw selectable with their `TextPath` slots
  and `range_in` ranges, their `on_hit` producing `TranscriptMsg::Select(SelectMsg::Hit { .. })`
- [X] T036 [US1] The `RICH` renderers in `crates/micold-client/src/ui/transcript/rich.rs` pass
  slots and ranges to `MarkdownView` (`crates/micold-client/src/ui/material/markdown_view.rs`
  `.selection(..)`/`.on_hit(..)`: a `TextPath` per inline run, list marker and table cell),
  `CodeBlock` and `InlineDiff`
- [X] T037 [US1] Tool-card header labels, unfolded input and result, turn footer, notices, raw rows
  and user text selectable in `crates/micold-client/src/ui/transcript/entries.rs`
- [X] T038 [US1] The descriptive
  text of request blocks and resolution rows in `crates/micold-client/src/ui/transcript/permission.rs`
  (controls not); the omitted notice in `crates/micold-client/src/ui/transcript/mod.rs`, each taking
  `in_row` of its own `RowPos`; T033 and T034 pass and `layout_snapshot` is unchanged

**Checkpoint**: the view draws exactly what is copied (parity); nothing visible changed.

---

## Phase 9: User Stories 1, 2 and 4 — selection in the transcript

**Goal**: drag, double/triple click, edge scroll, focus, select-all, the context menu and the copy
keys work in the app, and the user guide says how.

**Independent Test**: `cargo test -p micold-client --test features_transcript_selection --lib
shell::transcript`; quickstart Part B steps 1–5.

### Tests ⚠️ write first, see them fail

- [x] T039 [US1] Wiring tests in `crates/micold-client/tests/features_transcript_selection.rs` and
  the new `tests` module of `crates/micold-client/src/shell/transcript.rs` (contracts/selectable-text.md
  *A-2*, *A-3*; FR-003, FR-007): `Press` sets `focused_field` to `FieldId::Transcript(session)`;
  `Edge { dir }` yields a `scroll_by` on `transcript_scroll_id(session)` for that session only, in
  the direction given, and none when not dragging
- [x] T040 [US4] Menu tests in `crates/micold-client/tests/features_transcript_selection.rs`
  (contracts/keys-menu-copy.md *M-1*–*M-4*, FR-013): the menu items built for the open menu are
  **Copy** (`message: None` while the selection is empty) and **Select all**, at the right-press
  origin; an empty transcript shows Copy disabled (Edge Cases *Empty transcript*); M-3 is
  covered by T015

### Implementation

- [x] T041 [US1] Wrap the transcript's scrollable in `SelectionArea` in
  `crates/micold-client/src/ui/transcript/mod.rs` (`.id`, `.dragging`, `on_press`, `on_edge`,
  `on_release`, `on_context`, `on_focus_change` → `TranscriptMsg::Select`), joined to
  `FieldId::Transcript(session)` through `crates/micold-client/src/ui/focus.rs`; `Edge` becomes
  `scroll_by` on `transcript_scroll_id(session)` in `crates/micold-client/src/shell/transcript.rs`;
  T039 passes. Also M4a's follow-up: a folded card's selected summary is highlighted (`Ellipsized::highlighted`)
- [x] T042 [US4] The context menu: `micold_material::ContextMenu` with the T040 items over the
  transcript in `crates/micold-client/src/ui/transcript/mod.rs`, closing on Escape and an outside
  click (`MenuDismissed`); T040 passes
- [x] T043 [US1] User guide: `docs/user-guide/worktrees-and-sessions.md` *Structured sessions*
  describes selecting (drag across entries, edge scroll, double- and triple-click, Ctrl+A / Cmd+A
  in the focused transcript, the right-click menu), that releasing never copies (unlike the
  terminal view), the copy keys per platform and their arbitration with the interrupt (with a
  selection and the transcript focused Ctrl+C copies and never interrupts; without one Ctrl+C
  interrupts a busy turn as before; Cmd+C and Ctrl+Shift+C never interrupt), that moving focus to
  the composer keeps the selection, and the "Copy failed" notice (FR-021, Constitution VII)

**Checkpoint**: US1, US2 and US4 work end to end.

---

## Phase 10: User Story 3 — copy controls, and the closing docs (P2)

**Goal**: one-click copy of a tool card's identifying value, input, result and a diff's path.

**Independent Test**: `cargo test -p micold-client --test features_transcript_rich --test
layout_snapshot`; quickstart Part B step 6.

### Tests ⚠️ write first, see them fail

- [x] T044 [P] [US3] Copy-target tests in `crates/micold-client/tests/features_transcript_rich.rs`
  (contracts/keys-menu-copy.md *C-1*–*C-6*, data-model.md *Copyable value*, FR-015–FR-018,
  SC-004): `ToolValue` copies `work_value` byte for byte and exists iff it is `Some`; `ToolInput`
  copies `ToolCard::input` as held, the text so far while streaming; `ToolResult` copies the
  result parts' text joined with `\n`, `[kind not shown]` for unshown parts, and exists only once a
  result exists; `DiffPath` copies `EditDiff::path`; an empty value copies `""`; the confirmation is
  keyed by `(session, key, target)` per window and shows only on the activated control, expiring
  after `COPY_CONFIRM`; #940's `Code(CodeId)` behaviour is unchanged
- [x] T045 [P] [US3] Control tests in the `tests` module of
  `crates/micold-client/src/ui/material/inline_diff.rs` and in
  `crates/micold-client/tests/layout_snapshot.rs` (C-3, C-4, FR-017, FR-018): the `DiffPath`
  control fits beside the path at the narrowest pane width; each copy control is an `IconButton`
  with `TakesTheKeyboard` and Enter or Space publishes the same message as a press

### Implementation

- [x] T046 [US3] Add `CopyTarget::{ToolValue, ToolInput, ToolResult, DiffPath}` and their values in
  `crates/micold-client/src/features/transcript_rich.rs`; `TranscriptMsg::CopyCode` becomes
  `TranscriptMsg::Copy { session, key, target }` in
  `crates/micold-client/src/features/structured_transcript.rs`, callers updated; T044 passes
- [x] T047 [US3] Draw the controls, reusing `CodeBlock::copy`'s `IconButton`: `ToolValue` on the
  tool card's `DisclosureCard` header and `ToolInput`/`ToolResult` above the unfolded input and
  result in `crates/micold-client/src/ui/transcript/entries.rs`; `DiffPath` beside the path in
  `crates/micold-client/src/ui/material/inline_diff.rs`; T045 passes
- [x] T048 [US3] Docs: `docs/user-guide/worktrees-and-sessions.md` *Structured sessions* describes
  the copy controls (header value, input, result, diff path; keyboard; Copied/failed
  confirmation); the out-of-scope note on copying in
  `specs/939-structured-transcript-view/spec.md` says #1550 delivers it (FR-021)
- [x] T049 [US3] Quickstart Part B (steps 1–9) with the `visual-pass` skill on Linux, evidence in
  `specs/1550-transcript-text-selection/visual-pass/` (FR-020, SC-006 screenshots; macOS and
  Windows from the CI-built clients). Recorded in `m4b-visual-pass.md` and `m5-visual-pass.md`;
  step 9's copy key fails in a two-pane split (M4b code), a follow-up in the ledger

---

## Phase 11: Polish (close unit; no code)

- [ ] T050 Run quickstart Part A in full and the gate; record the results in the ledger
- [ ] T051 Converge: check every FR and SC of spec.md against plan *Requirement Coverage* and the
  merged tests; tick the tasks

---

## Dependencies & Execution Order

- **Setup (T001)** → **Foundational (T002–T006)** → US1 core (T007–T008), US4 core (T009–T010);
  US3 core (T011–T012) needs only T001's build.
- **Window state (T013–T023)** needs T006, T008 and T010 (units use `shown_text`, `list_marker`,
  `word_at`); T020–T021 need only `FieldId::Transcript` and can run before T013.
- **Shared components (T024–T032)** need nothing from this feature.
- **Selectable leaves (T033–T038)** need T014, T016 and T031.
- **Transcript wiring (T039–T043)** needs T038, T021 and T030.
- **Copy controls (T044–T049)** need T012 and T023; they edit the files T038 and T041 touch, so they
  run after them.

### Parallel opportunities

- T002 beside T003; T007 beside T009 and T011 (three files).
- T020 beside T013–T019 (different test file); T024, T025 and T027 in parallel.
- T033 beside T034; T044 beside T045.
- M3 (T024–T032) can run at any time beside M1–M2.

## Implementation Strategy

MVP first: M1 gives the selection model and shown text in core; M2 the window's selection, its
clearing, the copied text and the copy-key arbitration, all render-free and tested. M3 builds the
shared components beside them. M4a makes every leaf selectable without changing what the user
sees, proven by parity and pixel-equality; M4b turns selection on and documents it. M5 adds the
copy controls and finishes the docs and the visual pass.

## Milestones

Each milestone merges to `main` on its own, through one PR (speckit-autopilot). Every milestone
depends on nothing beyond `main` and the earlier milestones (research R13).

### M1 — Selection model and shown text in core 🎯 MVP

- **Tasks**: T001–T012
- **Deliverable**: `micold_core::structured::selection` orders positions in reading order, snaps to
  grapheme boundaries, expands by word and source line and splits a selection into per-row parts;
  `markdown::shown_text`, `markdown::list_marker` (drawn by `markdown_view.rs`) and
  `summary::work_value` give the text each unit and tool value copies
- **Satisfies**: US1 scenarios 1–3 and US4 scenarios 1–2 at the model; FR-002, FR-005 (text part),
  FR-010, FR-011, FR-015 (value), FR-019 (stability); SC-001, SC-005 at the model; S-1–S-5, S-11
- **Verify**: `cargo test -p micold-core --test structured_selection` and `cargo test -p
  micold-core --lib markdown structured::selection structured::transcript::summary`
- **Depends on**: —
- **Tier**: full

### M2 — Window selection state and copy keys

- **Tasks**: T013–T023
- **Deliverable**: each window keeps one selection per session, clears it by every FR-019 rule
  (including a pane switching session through `shell/panes.rs::sync`), assembles the copied text
  of every row kind, copies it by `CopySelection` with #940's checked write and a "Copy failed"
  notice, and `transcript_key` copies instead of interrupting when the focused transcript has a
  selection; nothing draws it yet
- **Satisfies**: US1 scenarios 2, 3, 5; US2 scenarios 1–6 at the state; US4 scenario 3 at the
  state; FR-006, FR-008, FR-009, FR-012, FR-014, FR-019, FR-019a; SC-002; S-2, S-3, S-5–S-12,
  K-1–K-11, M-2, M-3
- **Verify**: `cargo test -p micold-client --test features_transcript_selection --test
  features_composer --test features_transcript_rich --test features_are_render_free`
- **Depends on**: M1
- **Tier**: full

### M3 — Selectable text and selection area components

- **Tasks**: T024–T032
- **Deliverable**: `micold_material::SelectableText` and `SelectionArea` exist with builders and
  showcase entries; `Text`, `LinkText`, `CodeBlock` and `InlineDiff` gain `.selectable(..)`, pixel-
  equal when nothing is selected; the selection highlight passes the contrast gate on every
  transcript surface in both schemes
- **Satisfies**: Constitution VIII; FR-001, FR-003, FR-004, FR-005 (visual line), FR-007, FR-020
  at the component; SC-006 (gate); T-1–T-6, A-1–A-5
- **Verify**: `cargo test -p micold-material` and `cargo test -p micold-client --test
  showcase_completeness --test material_builder_api --lib ui::material`
- **Depends on**: —
- **Tier**: full

### M4a — Selectable leaves in the transcript (not yet user-visible)

- **Tasks**: T033–T038
- **Deliverable**: every transcript row draws its text through the selectable leaves with the
  model's slots and ranges, and the parity test proves the view draws exactly the units that are
  copied; select-all + copy at the event bound stays under 1 s; the transcript looks and behaves
  as before (nothing starts a selection until M4b)
- **Satisfies**: FR-002, FR-010, FR-011 in the view; SC-001 (parity), SC-003 (budget)
- **Verify**: `cargo test -p micold-client --test transcript_selection_parity --test
  layout_snapshot` and `cargo test --release -p micold-client --test transcript_open_budget`
- **Depends on**: M1, M2, M3
- **Tier**: full

### M4b — Selection in the transcript

- **Tasks**: T039–T043
- **Deliverable**: in a structured session the user drags, double- and triple-clicks to select
  across entries, the transcript scrolls at its edges, Ctrl+A / Cmd+A selects all, the right-click
  menu copies, and the copy keys copy without interrupting; the user guide describes it
- **Satisfies**: US1 scenarios 1–6, US2 scenarios 1–6, US4 scenarios 1–4 end to end; FR-001,
  FR-003–FR-009, FR-012, FR-013, FR-019a, FR-021 (selection part)
- **Verify**: `cargo test -p micold-client --test features_transcript_selection --lib
  shell::transcript`; quickstart Part B steps 1–5, recorded with the `visual-pass` skill into
  `specs/1550-transcript-text-selection/visual-pass/` (gate.md: the highlight first shows here)
- **Depends on**: M4a
- **Tier**: full

### M5 — Copy controls and docs

- **Tasks**: T044–T049
- **Deliverable**: tool cards have copy controls for their identifying value, input and result,
  diffs one for their path, each copying the held value and confirming only on itself; the user
  guide describes them, #939's out-of-scope note points to #1550, and the visual pass is recorded
- **Satisfies**: US3 scenarios 1–6; FR-015–FR-018, FR-020 (visual), FR-021; SC-004, SC-006
  (screenshots); C-1–C-6
- **Verify**: `cargo test -p micold-client --test features_transcript_rich --test layout_snapshot`;
  quickstart Part B step 6; `specs/1550-transcript-text-selection/visual-pass/`
- **Depends on**: M1, M2, M4b
- **Tier**: full

Polish (T050–T051) changes no code: the close unit does it in the close PR.
