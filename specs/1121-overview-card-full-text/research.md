# Research: Overview cards, full text without truncation

Builds on feature 1120's code: `overview_card.rs`, `card_grid.rs` (measure-then-stretch ribbon
`Grid`), `ellipsized.rs`, `micold_core::overview_text`.

## R1 — Where the expansion rules live

- **Decision**: a new core module `micold_core::overview_expansion` with a reducer `Expansion` fed
  by events; the app holds one in `features::overview::State` and sends every trigger to it as
  `Msg::Expansion(Event)`. The view reads `Expansion::expanded()`.
- **Rationale**: FR-016 asks for the rules render-free. Escape (D1) is decided in `app.rs::on_escape`,
  which needs to know whether a card is expanded; the menu, Overview open/close and listing are app
  state already. Holding it in the widget tree would put the Escape and menu rules out of reach of
  the app and of core tests.
- **Alternatives**: grid-widget state (rejected: Escape and menu live in the app, and the rules
  would only be testable through a mounted GUI); a client render-free module (rejected: nothing
  client-specific in the rules, and `SessionLocation` is a core type).

## R2 — Collapsed title on 3 lines, activity on 2

- **Decision**: reuse `LineClamped` (`ui/material/line_clamp.rs`, today `pub(super)`, Tooltip's
  label): make it `pub(crate)`, add `.tint(Rgb)`, use the role's spaced line height as
  `Ellipsized::at_role_spaced` does. Title `LineClamped::new(title, CardTitle, 3)`, activity
  `LineClamped::new(activity, Body, 2)`. Branch stays `Ellipsized` (FR-001).
- **Rationale**: it already shapes with `Wrapping::WordOrGlyph` at the width on offer and cuts by
  `micold_core::tooltip::clamp_to_lines` (tested in core), so an unbroken run ends in an ellipsis
  on its last line (FR-007). No second clamp widget (Principle VIII, duplication budget).
- **Alternatives**: a new `Ellipsized::max_lines` (rejected: duplicates `LineClamped`).

## R3 — The expanded card's form

- **Decision**: `OverviewCard::expanded(true)` builds the same wrapper chain (ContextArea → Ripple →
  TakesTheKeyboard → button → face) with a different face: header row; title wrapped in full
  (`Text` with `WordOrGlyph`); then an inner `Scrollable` (id `card_scroll_id(location)`) holding
  display name (when it differs from the title), full branch (when the title is not the branch),
  worktree directory, sessions line, full activity, progress, pull requests — all wrapped, no
  ellipsis. Order and text from `ExpandedText` (contract K5). Elevation shadow
  `style::elevation_shadow(r, elevation::MENU)`.
- **Rationale**: the outer wrappers keep their tree position, so iced's tree diff keeps the
  card's focus (`TakesTheKeyboard`'s `Focus`) and ripple state across expand/collapse — focus ring,
  ripple and Enter/Space are unchanged (FR-011). Only the inner face subtree, which holds no state
  that matters, is rebuilt. Header and title outside the scrollable stay in view (FR-008).
- **Alternatives**: a separate overlay element for the expanded form (rejected: a new tree loses
  focus and ripple state, and the menu target mark and focus ring would have to be redrawn on a
  second widget); iced `Widget::overlay` (rejected: escapes the Overview's clip and the grid's
  scroll, and nests the header tooltips' overlays inside an overlay).

## R4 — Drawing over neighbours without reflow

- **Decision**: `CardGrid::expanded(index, twin)` takes the expanded card and its collapsed twin.
  `Grid::layout` measures the twin (an extra child, never drawn or sent events) for that card's
  `naturals` entry, so `row_heights` and every slot are what they are with nothing expanded
  (FR-009); it lays the expanded card out at its slot's x, width and top with height
  `min(natural expanded, view height)`. `draw` skips the expanded card in the piece loop and draws
  it last in `renderer.with_layer(view)`, translated by `place` (R6) with `Layout::with_offset`.
  `update` and `mouse_interaction` send events to the expanded card first; every other card gets
  `cursor.levitate()` while the cursor is inside the expanded card's drawn rectangle (FR-010).
- **Rationale**: the twin keeps row heights exact even as the expanded card's text changes
  (Edge Case *content changes*). `levitate` is iced's own "something is above you" cursor:
  `is_over` is false for it, so hover layers, presses and the context area beneath stay inert.
- **Alternatives**: remember the row heights from the last unexpanded layout (rejected: stale when
  content changes while expanded).
- **During a zoom** (feature 970): nothing expands while `ZoomState::ends` is set; the grid
  publishes no rest and the app drops expansion on `FullScreenToggled` (spec: zoom unaffected).

## R5 — Pointer rest

- **Decision**: the grid keeps one `micold_core::tooltip::ShowTimer` for the card under the pointer
  (topmost: the expanded card first), observed on every event with `EXPAND_REST` (400 ms); when it
  opens, the grid publishes that card's `on_rest` message; when the pointer leaves the card it was
  over, it publishes that card's `on_leave`. It asks for a redraw at `wake_at`. The card under the
  pointer is recomputed after a scroll (Edge Case *scrolling under the pointer*).
- **Rationale**: `ShowTimer` is the tested "wait after entering, not restarted by movement" rule;
  400 ms is under the spec's 0.5 s (SC-006) and near the tooltip wait. The messages are data on
  each `OverviewCard`, so the grid stays generic.
- **Alternatives**: `RestTimer` (rejected: restarts on movement; a slow pointer would never expand).

## R6 — Shift, cap and scrolling inside

- **Decision**: `overview_expansion::place(slot, natural, view) -> Placement { top, height,
  scrolls }`: height `min(natural, view.height)`; top is the slot top, moved up just enough that the
  bottom is in view, never above `view.top` while the slot top is in view and never below the slot
  top (contract K3). The grid stores the last viewport it was given in `update`/`draw` and computes
  `place` per frame, so the shift follows scrolling with no relayout.
  The wheel needs nothing: iced's scrollable captures a wheel only when it scrolled, so at the
  card's end or with no overflow the Overview scrolls (checked in `iced_widget-0.14` scrollable).
  Page Up / Page Down: the card claims them (`.on_page`) only while expanded and focused and
  `page_target` says the card can move that way, from the inner scroll's last reported edges
  (`Msg::CardScrolled`); `main.rs` answers `Msg::CardPaged` with `scrollable::scroll_by` of one
  visible height. Unclaimed, Page keys behave as today (FR-008).
- **Alternatives**: a custom paging scroll widget (rejected: more code than one claim and one task).

## R7 — Escape (D1, localised)

- **Decision**: `app::on_escape`, after the registry (an open card menu answers first, FR-005 /
  feature 972 FR-007) and Settings, returns `Msg::Expansion(Event::Escape)` when
  `overview.expansion.expanded()` is some, else `Msg::Closed` as today.
- **If jaro picks the other D1 option** (one Escape collapses and closes): drop that one arm; `Closed`
  already resets the expansion. One arm and its test change; nothing else depends on D1.

## R8 — The worktree directory on the card

- **Decision**: `CardModel.dir: PathBuf`, filled in `ui/overview.rs::card` from `Worktree::path`
  (orphan/invalid: the recorded path) and from the project root for *Default*.
- **Rationale**: the card model is the one place card text is derived from (feature 1120 FR-018).

## R9 — The directory's form (D2, localised)

- **Decision**: `overview_text::worktree_dir(path, home: Option<&Path>) -> String`: a path inside
  `home` is `~` plus the platform separator plus the rest (`Path::strip_prefix`, so separators are
  the platform's), `home` itself is `~`, anything else is shown whole. The client passes the home
  folder it already resolves (`dirs::home_dir` / the shell helper in use; tasks pick the existing
  one).
- **If jaro picks another D2 option** (relative to the project root, or folder name only): only this
  function and its tests change.

## R10 — Snapshot, showcase, docs

- **Decision**: a covered state `overview-expanded` (Overview open, first card expanded by focus)
  in `tests/support/covered_states.rs`, fixture regenerated; the showcase surfaces section adds an
  expanded card beside the collapsed ones (`unscrolled` grid with `.expanded(0, twin)`); user guide
  per plan *Docs*. The text-overflow and containment gates run on the new state.
