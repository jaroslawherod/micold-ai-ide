# Data model: Daemon settings overlay

In-memory UI state only (`crates/micold-client/src/features/daemons.rs`, `State`); nothing new is persisted.

| Entity | Shape | Change |
|---|---|---|
| Daemon overlay draft | existing `Draft` in `State::draft: Option<Draft>`; `Some` means the form overlay is open; `Draft::editing: Option<DaemonId>` (None = add) | no change to fields |
| Details target | `State::details: Option<DaemonId>` | replaces `details_shown: BTreeSet<DaemonId>` |
| Opener | `State::opener: Option<Opener>`, `enum Opener { Add, Row(DaemonId, RowAction) }` (`Opener` and `RowAction` are both new: Edit / Details) | new; set on open, taken on close to return focus (FR-009) |
| Pending confirmation | existing `Pending` / `RemovalPrompt`; `Pending::Edit` is shown in the form dialog | new view accessor `State::form_confirmation()`; `standalone_prompt()` excludes `Pending::Edit` |
| Registered surfaces | `DaemonFormDialog { confirming: bool }`, `DaemonDetailsDialog` | new `FloatingSurface`s; `open_in` reads `draft` / `details` |
| Daemon configuration | `DaemonRegistry` entry | unchanged; written by `Msg::Saved` / `confirm` |

## Transitions

- `AddOpened` / `EditOpened(id)`: `draft = Some(..)`, `opener` set. Overlay opens. (Unknown or opaque runtime: unchanged, stays closed.)
- `Saved` with a validation error: `form_error` set, `draft` kept. Overlay stays.
- `Saved` needing confirmation: `pending = Some(Pending::Edit)`, `draft` kept; dialog shows the confirmation.
- `RemovalConfirmed` on `Pending::Edit`: registry updated, `draft = None`. `RemovalCancelled`: `pending = None`, `draft` kept.
- `Saved` success, `DraftDiscarded` (Cancel / Escape / scrim): `draft = None` (and `form_error`, flipped sections cleared); focus returns to `opener`; queued prompts shown.
- `DetailsOpened(id)` sets `details`; `DetailsClosed` clears it. If the daemon leaves the registry the surface reports closed (research R6). Content is computed at render from the live state (`features::daemons::details`).

Invariants: `draft` and `details` are never both `Some` (opening one is only reachable from the page, which is blocked while the other is open); a drift prompt is never created while either is `Some`.
