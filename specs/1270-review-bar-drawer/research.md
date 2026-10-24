# Research: Review bar and comments drawer (#1270)

**Plan**: [plan.md](./plan.md) | **Spec**: [spec.md](./spec.md)

Each decision names what was chosen, why, and what was rejected. Paths are workspace-relative.

## R1 — Where the bar and drawer rules live

**Decision**: two new render-free modules in `micold-core`: `review::bar` (counts, the counts'
shortened forms, send button, overflow availability, error wording) and `review::drawer` (tab
membership, row order, tab labels and empty texts, the drawer state machine with Esc and the
auto-open rule, the drawer width rule). `review::reply` gains the card's 4-line preview rule. They
take `&[ReviewComment]`, a `Scope` and plain flags; nothing in them names `OpenView`, `State` or
iced.

**Rationale**: the spec's acceptance asks for "pure logic in micold-core with unit tests"
(FR-023). #1269 rewrites `features::changes::State.open` into a map keyed by entry while this plan
is written, so rules that read `State` would conflict with it; rules that read only comments do
not. The client's existing `status_counts`, `send_action`, `tidy_actions` and `panel_rows`
(`crates/micold-client/src/features/changes.rs`) become thin callers of the core rules in M3,
after #1269 lands.

**Alternatives considered**:
- Keep the rules in `features/changes.rs` (render-free, so Principle I allows it): rejected; it
  is the file #1269 restructures, so every rule would be written twice or wait for #1269.
- A new crate: rejected; `micold-core::review` already owns comments, scopes and `reply`.

## R2 — What "Clear sent" counts as sent

**Decision**: Clear sent is available when the shown scope has any comment that is not a Draft
(Sent, Applied or Answered), exactly what `EntryReview::clear_sent`
(`crates/micold-core/src/review/comment.rs`) removes and what today's `tidy_actions` checks.

**Rationale**: FR-007 says "today's conditions"; today's condition is "not pending". Offering
Clear sent only for status Sent would leave Applied/Answered comments with no way to clear them.

**Alternatives considered**: status `Sent` only — rejected as above.

## R3 — Counts that shorten when the bar is narrow

**Decision**: `bar::CountsForm { Full, DraftsOnly, Hidden }` and `bar::counts_text(counts, form)`.
The bar component picks, at layout time, the longest form whose text fits the room left of the
trailing controls; `Hidden` draws nothing. Full with no comments is "No review comments yet";
`DraftsOnly` is "*n* draft(s)" and `None` when there are no drafts.

**Rationale**: text width is known only once shaped, as `StatusBar` (feature 1124,
`crates/micold-client/src/ui/material/status_bar.rs`) found; the order of shortening is a rule
(Edge Cases "Narrow window") and so lives in core, the measuring in the widget.

**Alternatives considered**: ellipsizing the full text (`Ellipsized`) — rejected: "3 drafts · 2
se…" hides the draft count, which the spec keeps longest. Fixed width breakpoints — rejected:
the counts' length varies with the numbers.

## R4 — A send button that never moves

**Decision**: the bar's trailing group is `[Comments] [⋮] [Send review (n)]`, laid out from the
bar's right edge; the send button has a fixed width, `anatomy::review_bar::SEND_WIDTH`, set to fit
the widest of "Sending…" and "Send review (999)" in the Label role (measured in M2's component
test). A count over 999 reads "Send review (999+)". The counts and the error message take the
leading, shrinking part.

**Rationale**: SC-001 asks for 0 px difference across drawer state, 0 vs 99 drafts and with or
without an error; a content-sized button changes width with its label.

**Alternatives considered**: a content-sized button placed last — rejected: its left edge moves
with "Send review (9)" vs "(99)" vs "Sending…". Putting the error message to the right —
rejected: it would push the button.

## R5 — The drawer over the diff

**Decision**: a new shared `material::SideSheet` (builder: `SideSheet::new(content, roles)
.title(..).on_close(msg).width(..)`), drawn with iced `stack![diff_pane, sheet]` aligned right in
the diff area, so the diff's layout is computed as if the sheet were absent. Width:
`drawer::width(area) = min(360, area / 2)` (`anatomy::comments_drawer::WIDTH = 360`).
No slide animation in this slice.

**Rationale**: FR-010/SC-002 require the diff's width and first line not to change; a stacked
layer cannot change its sibling's layout. `NavigationDrawer`
(`crates/micold-client/src/ui/material/navigation_drawer.rs`) pushes its sibling and leaves a
rail, which is the opposite of what FR-010 asks. Principle VIII: the sheet is a reusable surface,
so it is a library component, not a container styled inside `ui/changes.rs`.

**Alternatives considered**: a third column that appears on demand — rejected: changes the diff's
width. An overlay (`iced::overlay`) — rejected: overlays float above everything including the bar
and dialogs' scrims, and the registry treats overlays as floating surfaces with dismissal rules;
the drawer is part of the view, not a floating surface. Extending `NavigationDrawer` with an
overlay mode — rejected: its whole design is the rail swap.

## R6 — Drawer rows and their scroll

**Decision**: each tab is a `material::Scrollable` column with an `Id` per tab
(`DRAWER_DRAFTS_SCROLL_ID`, `DRAWER_REPLIES_SCROLL_ID`) and `.on_scroll_offset`; the offset is
kept per tab in the drawer state and restored with a `scroll_to` task when the drawer is shown or
the tab changes, as the diff jump already does with `DIFF_SCROLL_ID`. Rows are the existing
`ReviewPanelRow`, extended with a wrapped whole-reply mode and Mark applied / Answer… actions.

**Rationale**: Replies rows show the whole reply wrapped (FR-016), so row height is not known
before shaping. `VirtualRows` needs a fixed row height (its `extras` must be known before layout).
An entry's comments number in the hundreds at most (946 SC-006 tested 200), which a plain column
lays out well inside a frame.

**Alternatives considered**: `VirtualRows` with `extras` estimated from character counts —
rejected: wrong estimates jump the scroll. One scrollable for both tabs — rejected: FR-019 keeps a
scroll position per tab.

## R7 — Cards show at most 4 reply lines

**Decision**: `reply::CARD_LINES = 4` and `reply::card_preview(reply, lines_of) -> Preview { text,
cut }`, built on `tooltip::clamp_to_lines` (`crates/micold-core/src/tooltip.rs`) after
`display()` and the existing 4,000-character cap (which bounds shaping cost). The card draws the
reply with `LineClamped` (`crates/micold-client/src/ui/material/line_clamp.rs`), which gains
`.when_cut(element)`: an element laid out under the text only when the measure cut it. The card
passes its "Show all" button there. Drawer rows keep today's 4,000-character rule (FR-016).

**Rationale**: "lines as drawn at the card's width" is only known at layout time. `LineClamped`
already measures and cuts at layout time with a core-tested rule; adding the "only when cut"
companion keeps the decision in one place rather than guessing it from character counts in the
reducer.

**Alternatives considered**: counting `\n` and characters per line in the reducer — rejected: the
card's width and the font decide wrapping. Always showing "Show all" — rejected by US4 #2. A
new card-only measuring widget — rejected by Principle VIII.

## R8 — Esc order

**Decision**: `app::on_escape` (`crates/micold-client/src/app.rs`) keeps its order — the overlay
registry's topmost surface, the Settings draft, the Overview — and adds one last arm: the shown
Review tab's drawer is open → `Message::Changes(Msg::Drawer(DrawerEvent::Escape))`. The bar's
overflow menu is a registered popover (`ReviewOverflowMenu`, `Layer::Popover`, in
`crates/micold-client/src/overlay/registry.rs`), so the registry answers Esc for it, for the
Discard confirmation and the Show all dialog (both already registered dialogs) and for any open
select list, before the drawer.

**Rationale**: Edge Cases "Esc order" and SC-003; the registry already orders every floating
surface (feature 021), so the drawer only has to come after it.

**Alternatives considered**: making the drawer a registered surface — rejected: the registry's
surfaces are floating and closed by outside clicks and scrolls (`DismissalRules`); a click in the
diff must not close the drawer (US3: the user jumps from a row and reads).

## R9 — Auto-open: which send, which report

**Decision**: when Send review is pressed and a send effect is produced, the reducer records the
ids of the drafts it sends (the shown scope's drafts at that moment) as the drawer's `SendWatch`.
On every `ReviewChanged` for the entry, `drawer::newly_reported(before, after)` gives the comments
that are now Applied or Answered and were Draft or Sent (or absent) before. The first time that
set meets the watch, the watch is consumed; the drawer opens on Replies only if the entry's Review
tab is shown and the drawer is closed (an open drawer keeps its tab). Closing the drawer (×, Esc,
Comments) or a failed send drops the watch. A new send replaces it.

**Rationale**: FR-020's four limits map one-to-one: "a send started in this window" (the watch
exists only in the window that pressed), "first report, once per send" (consumed on first hit),
"only while the tab is shown" (a flag passed in), "not closed since" (closing drops the watch).
The send's ids are not on the wire (`OperationResult::ReviewSent` carries session and `started`),
and they need not be: the window knows which drafts it sent, and the service refuses a second
concurrent send.

**Alternatives considered**: watching any Sent→reported transition — rejected: another window's
send would open this window's drawer (Edge Cases "Two windows"). Adding the ids to
`ReviewSent` — rejected: protocol change out of scope.

## R10 — Send error in the bar, success in the snackbar

**Decision**: `PendingOp::ReviewSend` (`crates/micold-client/src/shell/daemon_sync.rs`) gains the
entry key (project, worktree dir). On failure the shell sends `Msg::SendFailed { project,
worktree_dir, message }` to the Changes reducer, which stores `bar::error_text(message)` in that
entry's view and drops the drawer's watch; the error snackbar remains as today. Success keeps
`sent_text` and its snackbar unchanged. The error is cleared by the next `SendPressed` of that
view or by `Msg::SendErrorDismissed` (the message's × in the bar).

**Rationale**: FR-006 ("stays until the next send starts or dismissed"); D2 in the ledger. The
entry key is needed because #1269 holds a view per entry, and the failure must land in the view
that sent, not the shown one.

**Alternatives considered**: drop the error snackbar — rejected: wording and snackbars are "as
today" (Assumptions); the bar adds persistence, not a replacement.

## R11 — Splitting the work around #1269

**Decision**: two layers. Layer A, independent of #1269: M1 core rules (R1–R4, R7, R9 rules) with
unit tests; M2 shared components (`ReviewBar`, `SideSheet`, `LineClamped::when_cut`,
`ReviewPanelRow` reply mode) with their in-crate tests and showcase entries. Layer B, after #1269
merges: M3 the reducer and shell wiring in `features/changes.rs`, `shell/`, `app.rs`, `overlay/`;
M4 the Review tab's view, layout snapshot re-record, geometry gate, docs and visual pass.

**Rationale**: the orchestrator's constraint and the spec's dependency note; Layer A touches no
file #1269 is likely to change (`features/changes.rs`, `ui/changes.rs`, the shell's view routing).

**Alternatives considered**: build Layer B against today's single `State.open` and port it after
#1269 (the spec's Assumptions allow it) — rejected as the default: two ports of the same wiring.
It remains the fallback if #1269 stalls; the ledger records the choice.
