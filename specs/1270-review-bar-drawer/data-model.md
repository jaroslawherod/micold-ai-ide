# Data model: Review bar and comments drawer (#1270)

**Plan**: [plan.md](./plan.md) | **Research**: [research.md](./research.md) | **Spec**:
[spec.md](./spec.md)

Nothing here is stored or sent on the wire: no store version, no protocol change (spec Out of
scope). Core entities are render-free and independent of #1269; client entities attach to the
per-entry review view #1269 introduces.

## Core: `review::bar` (new, `crates/micold-core/src/review/bar.rs`)

### `ScopeCounts`

| Field | Type | Meaning |
|---|---|---|
| `draft`, `sent`, `applied`, `answered` | `usize` | comments of one scope with that status |

- `ScopeCounts::of(comments: &[ReviewComment], scope: Scope) -> Self` — counts only `scope`'s
  comments (FR-002, Story 2 #5).
- `is_empty()`; `reported_or_sent() = sent + applied + answered` (R2).

### `CountsForm` and `counts_text`

`enum CountsForm { Full, DraftsOnly, Hidden }` — longest first (R3).

`counts_text(&ScopeCounts, CountsForm) -> Option<String>`:

| Form | Counts | Text |
|---|---|---|
| `Full` | all zero | `"No review comments yet"` |
| `Full` | some non-zero | non-zero statuses in order Draft, Sent, Applied, Answered, joined by `" · "`: `"3 drafts · 2 sent · 1 applied · 1 answered"`; `"1 draft"` singular; sent/applied/answered do not inflect |
| `DraftsOnly` | `draft > 0` | `"3 drafts"` / `"1 draft"` |
| `DraftsOnly` | `draft == 0` | `None` |
| `Hidden` | any | `None` |

### `SendButton` and `send_button`

| Field | Type | Meaning |
|---|---|---|
| `label` | `String` | `"Sending…"`, or `"Send review (n)"`, n capped as `"999+"` (R4) |
| `enabled` | `bool` | pressing sends |
| `primary` | `bool` | drawn filled; equals `enabled` |

`send_button(counts, sending: bool, sendable: bool)`: `sending` → `"Sending…"`, disabled (FR-005);
otherwise `enabled = sendable && counts.draft > 0` (FR-003; `sendable` is false for a pull request
not in the list, Edge Cases "Pull request shown").

### `Overflow` and `overflow`

| Field | Type | Rule |
|---|---|---|
| `clear_sent` | `bool` | `counts.reported_or_sent() > 0` (R2, FR-007) |
| `discard_drafts` | `bool` | `counts.draft > 0 && !sending` (FR-007) |

### Wording

- `error_text(why: &str) -> String` — `"Couldn't send the review: {why}"`; the client's
  `send_error_text` delegates to it (FR-006, wording unchanged).

## Core: `review::drawer` (new, `crates/micold-core/src/review/drawer.rs`)

### `DrawerTab`

`enum DrawerTab { Drafts, Replies }` (default `Drafts`).

- `DrawerTab::of(Status)`: Draft → Drafts; Sent, Applied, Answered → Replies (FR-013, SC-006).
- `label(self, n: usize) -> String`: `"Drafts 3"`, `"Replies 4"` (US3 #7).
- `empty_text(self) -> &'static str`: `"No drafts"`, `"No replies yet"` (FR-018).
- `START_HINT: &str` — today's "Click a line number in the diff to write one." shown under
  "No drafts" when the entry has no comments at all (Edge Cases "No comments at all").

### `tab_counts`, `rows_of`

- `tab_counts(comments: &[ReviewComment]) -> (usize, usize)` — (drafts, replies) over every scope
  of the entry (the drawer lists the whole entry, Edge Cases "Pull request shown").
- `rows_of<'a>(comments: &'a [ReviewComment], tab, line_of: impl Fn(&ReviewComment) -> u32) ->
  Vec<&'a ReviewComment>` — the tab's comments ordered by (scope, path bytewise, `line_of`, side,
  created) (FR-014). `line_of` is the client's shown range start (anchors live in the client).

### `Drawer` (state machine)

| Field | Type | Meaning |
|---|---|---|
| `open` | `bool` | drawn; `false` on creation (FR-011) |
| `tab` | `DrawerTab` | selected tab |
| `offsets` | `[u32; 2]` | scroll offset per tab, px (FR-019, R6) |
| `watch` | `Option<SendWatch>` | the send this window started and has not seen a report of (R9) |

`SendWatch { ids: BTreeSet<CommentId> }`.

`enum DrawerEvent`:

| Event | Effect |
|---|---|
| `Toggle` | open ↔ closed; closing drops `watch` (FR-008) |
| `Close` (×) | closed, drops `watch` |
| `Escape` | closed, drops `watch`; returns `false` when already closed (Esc not used) |
| `SelectTab(t)` | `tab = t` |
| `Scrolled { tab, offset }` | `offsets[tab] = offset` |
| `SendStarted(ids)` | `watch = Some(ids)`, replacing any earlier one |
| `SendFailed` | `watch = None` |
| `Reported { ids, shown }` | if `watch` meets `ids`: `watch = None`; then if `shown && !open`: `open = true, tab = Replies`; an open drawer keeps its tab. Otherwise nothing (FR-020) |

`Drawer::apply(&mut self, DrawerEvent) -> bool` — whether anything changed (the Esc arm in the
client uses it to know Esc was consumed).

### `newly_reported`

`newly_reported(before: &[ReviewComment], after: &[ReviewComment]) -> BTreeSet<CommentId>` — ids
whose status in `after` is Applied or Answered and in `before` was Draft or Sent, or which were not
in `before` (R9). A report replacing an earlier report is not new.

### `width`

`width(area: f32) -> f32 = min(anatomy::comments_drawer::WIDTH, area / 2)` (FR-010, Edge Cases
"Narrow window").

## Core: `review::reply` (extended)

- `CARD_LINES: usize = 4` (FR-021, D6).
- `Preview { text: String, cut: bool }` and `card_preview(reply: &str, lines_of: impl Fn(&str)
  -> usize) -> Preview` — `display()`, then the `CARD_CHARS` cap, then
  `tooltip::clamp_to_lines(.., CARD_LINES, lines_of)`; `cut` when either step shortened (R7).

## Core: `tokens::anatomy` (extended)

- `review_bar::HEIGHT = 44.0` (Assumptions), `review_bar::SEND_WIDTH` (R4, value fixed in M2 by
  the component test that measures the widest label).
- `comments_drawer::WIDTH = 360.0`.
- Both added to the anatomy table the module's property tests enumerate.

## Client: the per-entry review view (`features::changes`, after #1269)

`Panel` (today in `OpenView.panel`) becomes `ReviewDrawer`, held in the same per-entry view #1269
keeps across switching (FR-019):

| Field | Type | From |
|---|---|---|
| `state` | `micold_core::review::drawer::Drawer` | new: open, tab, offsets, watch |
| `focus` | `Option<CommentId>` | `Panel.focus` (card outlined after a jump) |
| `hidden` | `Option<Why>` | `Panel.hidden` (why a jump was refused) |
| `to_show` | `bool` | `Panel.to_show` |
| `viewport` | `u32` | `Panel.viewport` |

The per-entry view also gains:

| Field | Type | Meaning |
|---|---|---|
| `send_error` | `Option<String>` | the bar's message after a failed send from this view (FR-006, R10) |
| `overflow_open` | `bool` | the bar's overflow menu is open (registered popover, R8) |

`Composer` gains `place: ComposerPlace { Card, Drawer }`, so an Edit or Answer… started from a
drawer row is drawn in that row; the text stays in the view's composer and so survives closing the
drawer (Edge Cases "Unsaved text in the drawer").

### Messages (new or changed)

| Msg | Replaces | Effect |
|---|---|---|
| `Drawer(DrawerEvent)` for `Toggle`, `Close`, `Escape`, `SelectTab`, `Scrolled` | `PanelToggled`, `PanelScrolled` | applies the event to the shown view's drawer |
| `DrawerRowSelected(CommentId)` | `PanelRowSelected` | today's jump (FR-017) |
| `OverflowToggled`, `OverflowDismissed` | — | the bar's menu |
| `SendFailed { project, worktree_dir, message }` | — (snackbar only) | sets `send_error`, `DrawerEvent::SendFailed` |
| `SendErrorDismissed` | — | clears `send_error` |
| `SendPressed` | same | also clears `send_error` and applies `SendStarted(ids)` |
| `ReviewChanged` | same | also computes `newly_reported` against the previous `ReviewView` and applies `Reported { ids, shown }` to that entry's view |

### Derived read models (render-free, tested in `tests/features_changes.rs`)

- `review_bar(state) -> Option<ReviewBarModel>` — `{ counts: ScopeCounts, send: SendButton,
  overflow: Overflow, error: Option<String>, drawer_open: bool }` for the shown Review tab; `None`
  while a session tab is shown (FR-001). Replaces `send_action`, `tidy_actions`, `status_counts`,
  `review_toolbar_label`.
- `drawer_rows(state, tab) -> Vec<DrawerRow>` — `PanelRow` renamed, built from `rows_of`; a
  Replies row's `reply` is the whole displayable reply up to `CARD_CHARS`, `has_full_reply` when
  longer; `markable` for Sent rows.
- `drawer_groups(state)` — `panel_groups` renamed.
