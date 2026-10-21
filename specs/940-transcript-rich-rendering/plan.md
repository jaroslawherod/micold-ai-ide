# Implementation Plan: Structured Session Rich Rendering — markdown, highlighted code and Edit/Write diffs

**Branch**: `claude/project-thread-x8n8q6` | **Date**: 2026-10-21 | **Spec**: [spec.md](spec.md)

**Input**: Feature specification from `specs/940-transcript-rich-rendering/spec.md`

> **Starts after #939.** This plan is written against #939's *planned* modules and types
> (specs/939-structured-transcript-view/plan.md, tasks.md, contracts/), whose code milestones
> M1–M6 have not merged. #940's implementation starts only after #939's last milestone (M6) has
> merged to `main`; its first step re-checks the #939 names used here against the merged code and
> records any rename in research.md R14 before any test is written.

## Summary

Draw #939's transcript blocks richly through #939's per-block renderer seam, with no change to the
transcript model, its entries, folding, streaming or follow-tail. Two new render-free core modules
hold the decisions: `micold_core::markdown` parses assistant and thinking text with `pulldown-cmark`
into an owned block model, cut into top-level blocks parsed in isolation so that a **settled** block
is final and only the **open** (last) block changes while streaming, re-parsed incrementally from
its start (research R2); `micold_core::structured::edit_diff` turns a complete Edit, MultiEdit or
Write input into hunks of context/removed/added lines with `similar`, or `None` for the plain
fallback (R6). A render-free client module `features/transcript_rich.rs` is the content of #939's
`BlockCache`: it derives per window, session and entry in update, prunes with the transcript, holds
highlight results (both schemes, computed off the update thread with the Changes view's
`ui/syntax.rs`), copy state (verified by reading the clipboard back) and "show in full" choices
(R4, R5, R7, R8). `BlockRenderers::RICH` draws them with new shared components — `MarkdownView`,
`CodeBlock`, `InlineDiff` (sharing `DiffView`'s row painting), `LinkText` — and the one change
outside the renderers, the `+N −M` header meta on Edit/MultiEdit/Write cards (FR-017). Links open
through the existing opener for web and mail addresses only (R11). Details: [research.md](research.md),
[data-model.md](data-model.md), [contracts/](contracts/).

## Technical Context

**Language/Version**: Rust stable (workspace toolchain)

**Primary Dependencies**: new — `pulldown-cmark` 0.13 (`default-features = false`) and `similar` 2,
both in `micold-core` (vetted in research R1, R6); existing — `iced` 0.14 (`lazy` and `highlighter`
features, already enabled; `table`, `rich_text` and `clipboard::{write, read}` need no feature flag), `serde_json`, the
`ui/material` library, `micold_core::link::{address, detect}`, `micold_core::review::{diff,
limits}`.

**Storage**: none; all derived state is per window, in memory.

**Testing**: `cargo test` — core: `crates/micold-core/tests/markdown_subset.rs`,
`markdown_stream.rs`, `edit_diff.rs` over new recorded fixtures (`markdown-turn.jsonl`,
`edit-turn.jsonl`) and a synthetic `markdown-subset.md` (R13); client reducer:
`crates/micold-client/tests/features_transcript_rich.rs`; renderer swap:
`tests/transcript_rich_renderers.rs`; geometry/contrast gates in `ui/material/` and
`tests/gates/`; the release-only budget `transcript_open_budget.rs` (rich variant); quickstart
Part B for draw code.

**Target Platform**: Linux, macOS, Windows (client only)

**Project Type**: desktop app — iced client over the shared render-free core

**Performance Goals**: SC-004 (#939's, unchanged) — 5,000 events with markdown, code and diffs,
every card open, first show < 1 s in release; no update + layout over 100 ms while streaming.

**Constraints**: no new event, wire message, protocol version or daemon change; nothing fetched to
render (Principle IV); every terminal-session and #939 test passes unchanged except plain-text
assertions of the replaced block kinds (SC-007); #939 merged first (R14).

**Scale/Scope**: replies up to tens of KiB; code blocks and Write contents up to the Changes view's
limits before "Show in full" (R7).

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

- [x] **I. Test-First**: every decision — markdown subset, literal fallbacks, settled/open split
  and incremental parse, fence tokens, link activatability, edit-diff shapes, line diff, notes,
  counts, size limits, cache refresh/prune, copy state machine, link-open request — is in
  `micold-core` (`markdown`, `structured::edit_diff`) or the render-free
  `features/transcript_rich.rs`, test-first. `ui/syntax.rs::knows` is tested in-crate like the rest
  of that module. Only `ui/transcript/rich.rs`, the shared components' drawing and the shell glue
  use the GUI exception, validated by quickstart Part B, the gates and the budget test.
- [x] **II. Multi-Session**: `RichCache` is per window and keyed by `SessionId`; copy, show-in-full
  and highlight state never cross sessions or windows (RR-5); an isolation test drives two sessions
  and two caches. Nothing new is persisted.
- [x] **III. Worktree Integration**: no file or VCS operation; diffs are of the tool call, not the
  file on disk (Out of Scope).
- [x] **IV. Local-First**: nothing is fetched to render; images are shown as text; a link opens only
  on the user's activation through the existing opener.
- [x] **V. Rust + iced**: iced only. `Block`, `EditKind`, `HunkNote`, `CopyState`, `Part` are closed
  enums; `edit_diff` returns `Option`, so a malformed input cannot reach `InlineDiff`; a `CodeId` is
  only issued by `Document`.
- [x] **VI. Cross-Platform**: core never branches on the OS; copy goes through iced's clipboard and
  is verified by read-back on every platform (R5); links use feature 031's per-platform opener;
  CRLF handled in core (R6). Part B runs on each platform's CI-built client.
- [x] **VII. Documentation**: `docs/user-guide/worktrees-and-sessions.md` *Structured sessions*
  describes formatted replies, code blocks and copy, diffs, and which links open, in the milestone
  that makes them visible (FR-024).
- [x] **VIII. Reusable UI**: `MarkdownView`, `CodeBlock`, `InlineDiff`, `LinkText` are new shared
  components with builders and showcase entries; `DisclosureCard` gains `.meta()`; `InlineDiff`
  shares `DiffView`'s marker/tint/span painting rather than copying it (justified divergence from
  reusing `DiffView` itself: Complexity Tracking).

One recorded divergence (component-reuse gate); see Complexity Tracking.

## Project Structure

### Documentation (this feature)

```text
specs/940-transcript-rich-rendering/
├── plan.md
├── research.md
├── data-model.md
├── quickstart.md
├── contracts/
│   ├── markdown-model.md     # core markdown model, rules, guarantees M-1–M-6
│   ├── edit-diff.md          # core edit diff, input shapes, guarantees E-1–E-3
│   └── rich-renderers.md     # BlockRenderers::RICH, the cache rules RR-1–RR-6, shell glue
└── tasks.md                  # next unit
```

### Source Code (repository root)

Paths of #939 are as its plan names them (re-checked first, R14).

```text
Cargo.toml                                  # + pulldown-cmark, similar (workspace deps)
crates/micold-core/Cargo.toml               # + both
crates/micold-core/src/
├── lib.rs                                  # + pub mod markdown
├── markdown/                               # NEW, render-free (R1–R4)
│   ├── mod.rs                              # Document: parse, update (incremental), accessors
│   ├── block.rs                            # Block, ListItem, CodeBlock, CodeId, Align, Span, Style, Link
│   ├── split.rs                            # top-level cuts, settled/open (R2)
│   ├── inline.rs                           # pulldown events → spans, literal fallbacks, links (R3)
│   └── fence.rs                            # fence_token, alias table (R4)
├── structured/mod.rs                       # + pub mod edit_diff
├── structured/edit_diff.rs                 # NEW: edit_diff, EditDiff, EditHunk, EditLine, HunkNote (R6)
└── review/diff.rs                          # split_lines → pub(crate) (reused by edit_diff)
crates/micold-core/tests/
├── markdown_subset.rs                      # NEW: M-1, M-5, M-6 on markdown-subset.md + markdown-turn
├── markdown_stream.rs                      # NEW: M-2–M-4 at every prefix; replay through Transcript
├── edit_diff.rs                            # NEW: E-1–E-3 on edit-turn + synthetic inputs
└── fixtures/stream-json/
    ├── markdown-turn.jsonl                 # NEW recorded (R13)
    ├── edit-turn.jsonl                     # NEW recorded (R13)
    ├── markdown-subset.md                  # NEW synthetic
    └── README.md                           # + their rows
crates/micold-client/src/
├── features/transcript_rich.rs             # NEW render-free: RichCache, Derived, CopyState, HighlightJob (R8)
├── features/mod.rs                         # + module; + Outcome::ClipboardWriteChecked
├── features/structured_transcript.rs       # (#939) forget paths also forget RichCache; TranscriptMsg + six variants
├── app.rs                                  # State.transcript_rich beside structured_transcript; refresh after apply; ClipboardWriteChecked arms in its exhaustive Outcome matches
├── shell/daemon_sync.rs                    # (#939) refresh after each StructuredEvents; run HighlightJobs
├── shell/clipboard.rs                      # ClipboardWriteChecked → write + read-back (no branch)
├── shell/…                                 # TranscriptMsg handling: copy timer, highlight tasks, OpenLink → links::perform
├── ui/syntax.rs                            # + knows(token); fills gains the code surface
├── ui/transcript/rich.rs                   # NEW: BlockRenderers::RICH (GUI exception)
├── ui/transcript/blocks.rs                 # (#939) BlockCache = transcript_rich::RichCache (re-export)
├── ui/transcript/entries.rs                # (#939) header view model: meta "+N −M" (FR-017)
├── ui/transcript/mod.rs                    # (#939) lazy key + stamp; window uses RICH
├── ui/material/markdown_view.rs            # NEW shared component (R9)
├── ui/material/code_block.rs               # NEW shared component
├── ui/material/inline_diff.rs              # NEW shared component
├── ui/material/link_text.rs                # NEW shared component
├── ui/material/diff_view.rs                # marker/tint/span painting extracted to shared helpers
├── ui/material/disclosure_card.rs          # (#939) + .meta(text)
├── ui/material/mod.rs                      # register the four
├── ui/material/composition_contrast.rs, style_outline_discipline.rs, ripple_clipping.rs  # + cases
└── showcase/catalogue.rs, showcase/sections/  # entries for the four
crates/micold-client/tests/
├── features_transcript_rich.rs             # NEW reducer tests
├── transcript_rich_renderers.rs            # NEW renderer-swap test
├── transcript_open_budget.rs               # (#939) + rich variant
├── gates/code_block_fits.rs                # NEW
├── gates/code_block_wheel.rs               # NEW
├── gates/inline_diff_marker_fits.rs        # NEW
└── layout_snapshot.rs                      # + a rich transcript pane state
docs/user-guide/worktrees-and-sessions.md   # Structured sessions: rich rendering (FR-024)
```

**Structure Decision**: the existing three-crate split; the daemon is untouched. Decisions in core
(markdown, edit diff) and a render-free client feature (cache); drawing in `ui/transcript/rich.rs`
and the shared material library.

## Requirement Coverage

Test layers: **core** = `micold-core` tests; **client** = `micold-client` reducer and renderer
tests; **geometry** = material in-crate gates and `tests/gates/`; **budget** =
`transcript_open_budget` (release); **§B** = [quickstart.md](quickstart.md) Part B; **docs** = diff
review of the user guide.

| FR / SC | Design | Test layer |
|---|---|---|
| FR-001 | R1, R3; `markdown` model; `MarkdownView` | core M-1 (subset, literal HTML, image as text); §B 1 |
| FR-002 | R10 `thinking` = `MarkdownView.muted(true)` | client (`transcript_rich_renderers`: thinking drawn through the markdown renderer, muted); §B 6 |
| FR-003 | R3 `Table`; R9 iced `table`, `Fill` columns | core (alignment, header, rows); geometry (`layout_snapshot` table at a narrow width wraps in-cell); §B 1 |
| FR-004 | R3 `Link.activatable` via `classify`; R9 `LinkText`; R11 | core (`http`/`https`/`mailto` activatable, `javascript:`/`file:`/relative not, image not fetched); client (`OpenLink` emits an open for web/mail only); §B 4 |
| FR-005 | R2 incremental `Document::update` per delta | core M-3; §B 1 |
| FR-006 | R2 top-level cuts, slices parsed in isolation | core M-2 at every prefix; §B 1 |
| FR-007 | R2, R3 open code block, literal `**`, table-less paragraph | core M-5 (property), unbalanced cases in `markdown-subset.md` |
| FR-008 | R2 same slices for the finished text | core M-4; replay through `Transcript` in `markdown_stream.rs` |
| FR-009 | R4 `fence_token`, `syntax::knows`, off-thread highlight | core (`fence_token` table); client (job only for known tokens; spans stored); in-crate `syntax::knows`; §B 2 |
| FR-010 | R9 `CodeBlock` horizontal-only `Scrollable` | geometry (`code_block_fits`: block never widens the transcript; `code_block_wheel`); §B 2 |
| FR-011 | R4 copy text; R5 `CopyCode` → `ClipboardWriteChecked`; `TakesTheKeyboard` | core (`code` exact for fenced, indented, open blocks); client (clipboard text = code byte for byte; streaming copy = text so far); in-crate (copy control focus stop, Enter); §B 3 |
| FR-012 | R5 tickets, read-back, `COPY_CONFIRM` | client (Copied / Failed on mismatch / `None`; expiry by ticket; per window and block); `tests/clipboard_request.rs` (interpret has no branch) |
| FR-013 | R6 `edit_diff`; R9 `InlineDiff`; R10 `tool_input` | core E-1; client (renderer chooses `InlineDiff` for complete inputs); §B 5 |
| FR-014 | R6 `HunkNote::ReplaceAll` | core |
| FR-015 | R9 `+`/`−` marker cell and tint (shared with `DiffView`); R4 `syntax::token(path)` spans | geometry (`inline_diff_marker_fits`; `composition_contrast` markers both schemes); §B 7 |
| FR-016 | R6 no numbers; hunks named by ordinal | core (no `@@`/numbers in the model); geometry (no number gutter) |
| FR-017 | RR-2 meta from `edit_counts`; `DisclosureCard::meta` | core E-3; client (header view model carries `+N −M` folded and open); geometry (`disclosure_card_header_fits` with meta) |
| FR-018 | R6 `None`; R10 fallback while streaming or malformed | core E-2; client (`transcript_rich_renderers`: streaming and malformed equal `PLAIN`; failed result keeps the diff) |
| FR-019 | R7 limits, `ShowInFull` | core (`lines`/`bytes` over limit); client (`ShowInFull` per session; no job before it); §B |
| FR-020 | no change to #939's fold set | client (Edit/Write cards start folded with `RICH`) |
| FR-021 | R10 `tool_result = PLAIN.tool_result` | client (`transcript_rich_renderers`: results identical to `PLAIN`) |
| FR-022 | R8, R10; RR-1–RR-4 | client (`transcript_rich_renderers`: order, folds, statuses identical to `PLAIN` but block content and meta; #939's `transcript_renderer_seam` still green) |
| FR-023 | R2, R8, R12 per-block lazy, stamps, off-thread highlight; both-scheme spans | budget; client (a delta refreshes only the changed entry; theme switch needs no job) |
| FR-024 | `docs/user-guide/worktrees-and-sessions.md` | docs |
| SC-001 | R3 | core M-1 |
| SC-002 | R2 | core M-2–M-4 on `markdown-turn` and every prefix |
| SC-003 | R6 | core E-1 over `edit-turn` and the synthetic set |
| SC-004 | R12 | budget (rich variant, CI release step) |
| SC-005 | R5 | client (byte-for-byte clipboard text in every copy case) |
| SC-006 | R4 legible colours on the code surface; R9 markers | geometry (`composition_contrast`, both schemes); §B 7 greyscale |
| SC-007 | no change outside the seam but FR-017 | `scripts/gate.sh` (whole suite) |

## Delivery

Five milestones, cut in tasks.md, all after #939 M6 has merged (R14):

- **M1 — markdown model (core)**: `micold_core::markdown`, `pulldown-cmark`, the fixtures of R13,
  M-1–M-6. *Satisfies*: FR-001, FR-003–FR-008 at the model; SC-001, SC-002.
- **M2 — edit diff (core)**: `structured::edit_diff`, `similar`, E-1–E-3. *Satisfies*: FR-013,
  FR-014, FR-016–FR-019 at the model; SC-003.
- **M3 — the window's rich cache (client, render-free)**: `features/transcript_rich.rs`, the six
  `TranscriptMsg` variants, `Outcome::ClipboardWriteChecked`, shell glue (highlight jobs, copy
  read-back and timer, link open), `syntax::knows`. *Satisfies*: FR-004, FR-009, FR-011, FR-012,
  FR-019, FR-023 at the state; SC-005.
- **M4 — shared components**: `MarkdownView`, `CodeBlock`, `InlineDiff` (with `DiffView`'s painting
  extracted), `LinkText`, `DisclosureCard::meta`, gates, showcase. *Satisfies*: Principle VIII;
  FR-003, FR-010, FR-015, FR-017 (component); SC-006.
- **M5 — rich renderers in the transcript**: `BlockRenderers::RICH`, header meta, lazy stamp, the
  window switched to `RICH`, the budget's rich variant, the user guide and quickstart Part B.
  *Satisfies*: FR-002, FR-013, FR-018, FR-020–FR-024; SC-004, SC-007. The user guide changes here
  because this is where rich rendering becomes visible.

## Complexity Tracking

| Violation | Why Needed | Simpler Alternative Rejected Because |
|---|---|---|
| `InlineDiff` beside `DiffView` instead of reusing `DiffView` (component-reuse gate) | `DiffView<'a>` borrows its `FileDiff`, which #939's lazy entries (`Element<'static, _>`) cannot hold; it virtualises rows in its own vertical scroll, which nests badly inside the transcript's; it draws file line numbers FR-016 forbids | Making `DiffView` own its data, drop its scroll and hide its gutter would rewrite the Changes view's component and its gates for one caller; instead the marker, tint and span painting are extracted and shared by both, so the visual rules stay single (research R9) |
