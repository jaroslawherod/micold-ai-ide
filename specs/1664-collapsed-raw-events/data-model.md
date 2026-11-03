# Data Model

No stored entry changes; no wire or protocol change.

- **`LowSignal`** (core, new, closed enum): `Raw`, `SessionStart`, `Footer`, `OrphanResult`. `Entry::low_signal() -> Option<LowSignal>` is an exhaustive `match` on `EntryKind` (no `_` arm).
- **Always shown** (FR-013): `Footer` with `Outcome::Error`; `OrphanResult` with `is_error`.
- **`shown(&Entry, show_raw: bool) -> bool`**: `show_raw || low_signal().is_none() || always_shown`.
- **`Transcript::hidden_count(show_raw) -> u64`**: entries with `!shown`; always 0 when `show_raw`. Error entries never count.
- **`Settings.show_raw_events: bool`**: `#[serde(default)]`, default false; client-owned, the daemon never reads it. Mirrored in `features::settings::State.show_raw_events`.
- **`Row`** (client view-model): gains `Hidden(u64)`; `rows(transcript, show_raw)` emits `Omitted`, then `Hidden(n)` when `n > 0`, then shown entries in order. The `Empty` row appears only when the transcript has no entries at all (a transcript of only hidden entries shows the notice, per the edge case).
- **Summary line**: derived each draw from the entry (title + facts); no state. Expansion: existing `State.open: HashSet<EntryKey>` per session.

Summary wording (English, in `shown.rs`):

| Entry | Title | Facts |
|---|---|---|
| Raw, Unrecognised | "Unrecognised event" | none (the spec's `(stream_event)` example: raw has no type field, so the type is shown only if the raw text parses as JSON with a `type`; else omitted) |
| Raw, Unreadable | "Unreadable line" | none |
| Raw, UnknownBlock | "Unknown content block" | none |
| Raw, UnknownStreamEvent | "Unknown stream event" | none |
| SessionStart | "Session started" | model when present |
| Footer | "Turn done" / "Turn failed" | duration, turns when present |
| OrphanResult | "Result without a call" / "Result without a call · error" | none |

Details open to: raw text (empty text reads "Empty event"); footer cost and error text; orphan output via `renderers.tool_result`; session start cwd.
