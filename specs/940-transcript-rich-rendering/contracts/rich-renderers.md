# Contract: the rich renderers and their cache (client, FR-022)

Plugs into #939's seam (specs/939-structured-transcript-view/contracts/block-renderers.md R-1–R-5)
without changing its shape. Decisions: research R5, R8–R11.

## Renderers (`ui/transcript/rich.rs`, GUI exception)

```rust
impl BlockRenderers {
    pub const RICH: Self; // the window's value from #940 on
}
```

| Field | Draws |
|---|---|
| `assistant_text` | `MarkdownView` of the cached `Document`; nothing for empty text; `PLAIN` on a cache miss |
| `thinking` | the same, `.muted(true)` |
| `tool_input` | `InlineDiff` when the tool is `Edit`/`MultiEdit`/`Write`, `!streaming` and the cache holds `Diff(Some(_))`; else `PLAIN.tool_input` |
| `tool_result` | `PLAIN.tool_result`, unchanged (FR-021) |

## Rules

- **RR-1** (#939 R-1, R-2) A rich renderer draws block content only. Its only messages are
  `TranscriptMsg::{OpenLink, CopyCode, ShowInFull}`. It changes no entry, order, fold, status,
  streaming or scroll behaviour.
- **RR-2** The one change outside the renderers (FR-017): the card header view model of
  `ui/transcript/entries.rs` adds `meta = "+{added} −{removed}"` from `RichCache::edit_counts` for
  Edit/MultiEdit/Write cards, drawn with `DisclosureCard::meta`; folded or open alike.
- **RR-3** The lazy key of an entry (#939 R12: `(EntryKey, revision, open, scheme)`) gains the
  entry's `Derived.stamp`.
- **RR-4** (#939 R-4) `RichCache::refresh` runs in update after each applied `StructuredEvents`
  message, for that session only, and prunes keys the transcript no longer holds; `forget` drops a
  session's cache with its transcript.
- **RR-5** Copy, show-in-full and highlight state are per window and per session; nothing in one
  `RichCache` is read by another window or session (Principle II).
- **RR-6** A link that is not activatable sends nothing; `OpenLink` re-checks `classify` and emits
  `Outcome::OpenLink(OpenRequest::Url)` for web and mail addresses only (FR-004).

## Shell (thin glue)

| Request | Shell does |
|---|---|
| `HighlightJob` (from `refresh`, `ShowInFull`) | `spawn_blocking` → `ui::syntax::highlight_lines` in both schemes under the time budget → `TranscriptMsg::Highlighted` |
| `Outcome::ClipboardWriteChecked { text, ticket }` | `clipboard::write(text)` then `clipboard::read()` → `TranscriptMsg::CopyChecked { ticket, read }` (no branch, contract C3 of feature 021) |
| `Outcome::CopyConfirmTimer { ticket, after }` (with `Copied`/`Failed`) | `tokio::time::sleep(after)` → `TranscriptMsg::CopyExpired { ticket }` |
| `Outcome::OpenLink(OpenRequest::Url)` | existing `shell/links.rs::perform` |
