# Research: Overview full-screen mode

No `NEEDS CLARIFICATION` remained after clarify (spec *Clarifications*). The decisions below fix
how the mode is built. Paths are relative to `crates/micold-client/src/` unless stated.

## R1 — Where the mode lives

- **Decision**: A `full_screen: bool` field on the Overview's state (`features/overview.rs`,
  `State`), read only through a derived `State::full_screen()` = `open && full_screen`. Opening the
  Overview (`Msg::Toggled` from closed) clears the field. A new `Msg::FullScreenToggled { settings_open }`
  flips it while the Overview is open, Settings is closed and no activation is pending.
- **Rationale**: FR-001 wants the mode off each time the Overview opens and never remembered. With
  the derived accessor, every route that closes the Overview also leaves the mode, with no edit:
  `Closed`, `CardActivated` (both reveal arms included), and the three places in `app.rs` that set
  `overview.open = false` (`ChangesRequested`, `ChangesClosedForCompare`, `MainAreaTaken`). A stale
  `true` behind a closed Overview cannot show, because nothing reads the field directly, and opening
  clears it (Principle V: "full screen while closed" is unrepresentable to readers). Ignoring the
  toggle while an activation waits out its delay follows feature 971's "the first one stands".
- **Alternatives considered**:
  - An enum `Closed | Open | OpenFullScreen` replacing `open: bool`: stronger typing, but `open` has
    about twenty readers and writers across `app.rs`, `ui/mod.rs`, `shell/*` and tests; every one
    would change for no behaviour. Rejected as churn; the derived accessor gives readers the same
    guarantee.
  - Keep the flag in `features/sidebar.rs` as a third sidebar state: FR-009 says the mode must
    never change the user's sidebar state, and the sidebar persists its state (`shell/persist.rs`);
    a flag there would invite a write. Rejected.
  - Persist the flag: FR-001 forbids it.

## R2 — Hiding the sidebar without touching its state (FR-002, FR-009)

- **Decision**: The mode never writes `state.sidebar.hidden` or `state.sidebar.width`. The view
  hides the sidebar: `ui/mod.rs` builds the same `NavigationDrawer` with a new builder
  `.retracted(state.overview.full_screen())`. A retracted drawer animates its **whole** width
  (panel with handle, or the collapsed rail) to zero and, once settled there, parks every child
  (no draw, no events), so neither the strip nor the resize handle is on screen or draggable.
- **Rationale**: Restoring "the state and width it had when the mode was entered" (FR-009, SC-004)
  is structural when nothing is saved and nothing is written: leaving the mode shows the drawer as
  `state.sidebar` describes it, which is what the user left. The sidebar's only state writers are its
  own hide/show buttons (`features/sidebar.rs` `hidden_toggled`, line ~1685) and the resize handle
  (`drag_moved`); neither is on screen while retracted, and no keyboard chord flips `hidden`
  (grep: `sidebar.hidden =` has the one writer), which settles spec Edge Case *The sidebar's own
  toggle*. Spec Assumptions: "full-screen mode reads them and puts them back, never writes them".
- **Alternatives considered**:
  - Snapshot `hidden`/`width` on entry and write them back on leaving (the Key Entity's literal
    wording): it writes the sidebar's state, which the spec's Assumptions rule out, and it would
    persist through `shell/persist.rs` if the app quit mid-mode. Rejected: nothing to restore when
    nothing changed.
  - Omit the drawer from the row in full-screen mode: no zoom (FR-005), and the drawer's tree state
    would be dropped, so leaving could not animate back. Rejected.
  - Set `sidebar.hidden = true`: shows the strip (FR-002 forbids it) and changes user state.

## R3 — The zoom: one track in the drawer, the main area follows (FR-005, FR-007)

- **Decision**: `NavigationDrawer` gains a second `Progress` track, *presence* (1 shown, 0
  retracted), on the same `SLIDE` duration (`duration::MEDIUM_4`, 400 ms) and `EMPHASIZED` curve as
  its open/close slide (018 §6.3 *sidebar slide*). Layout multiplies the drawer's current width by
  the eased presence and translates its content left by the difference, so the sidebar leaves at the
  window's left edge. The main area is the drawer's `row!` sibling with `Length::Fill`, so its left
  edge is the drawer's right edge on every frame: the Overview grows as the sidebar leaves, with no
  gap or overlap (FR-005, SC-002 "edges touching").
- **Rationale**: "Zoom in" in the spec's *Terms* is the Overview growing from its area to the whole
  width while the sidebar leaves; a width that grows is that, without scaling pixels. `Progress`
  already reverses from its current value when retargeted (`retarget` sets `from = value`), which is
  FR-007; it already sizes per real elapsed time (`on_layout_frame`), so 400 ms is wall-clock and
  under the 500 ms bound. The track starts at its destination (`Progress::new(target)`), so a drawer
  rebuilt after Settings shows the full-screen layout with no zoom (FR-011, Edge *Settings while
  full-screen*), as the open/close track already does
  (`a_drawer_starts_at_its_destination_rather_than_animating_to_it`).
- **Alternatives considered**:
  - A scale transform of the Overview (e.g. iced `float` scaling): scales text and cards as an
    image, so glyphs blur and the cards do not reflow (FR-004/FR-006 want real widths). Rejected.
  - An app-owned transition advanced by a `window::frames()` subscription: puts per-frame state back
    in the application, which feature 017 moved out (`ui/cdk/motion.rs` module docs) for identity and
    idle-quiescence reasons. Rejected.
  - A separate wrapper around the drawer (`material::expand` horizontally): `Expand` animates one
    child toward nothing and knows nothing of the drawer's rail/panel swap; two nested tracks on one
    edge would have to agree on parking the handle. Rejected in favour of one owner of the edge.

## R4 — Reduced motion (FR-008)

- **Decision**: A new app setting `reduce_motion: bool` (`#[serde(default)]`, off) in
  `micold_core::settings::Settings`, edited on the Settings *Appearance* page with the shared
  `LabelledToggle`, carried through the draft (`AppearanceDraft.reduce_motion`,
  `ValidSettings.reduce_motion`) and applied on Save to `SettingsState.reduce_motion` next to
  `theme_pref`; persisted by `shell/persist.rs` the way the theme is. The view passes it to the
  drawer as `.instant(reduce_motion)`: an instant drawer snaps its presence track
  (`Progress::restart_at`) instead of easing it, including a track already moving (Story 3 #3).
  The open/close slide of the sidebar keeps its animation (FR-008: "governs this feature's zoom
  only").
- **Rationale**: Clarification 2 (user): a new app setting, OS preference not read; clarification 4:
  Settings' one-form rule (Save applies, Cancel discards), which the draft already implements for
  every field. The grid's reflow (R5) is driven by the drawer's width, so a snapped drawer snaps
  the cards with it: one switch governs the whole zoom and cannot disagree with itself. Turning the
  setting on mid-zoom can only happen through Settings, which replaces the body; the drawer is then
  rebuilt at rest (R3), so Story 3 #3 also holds structurally.
- **Alternatives considered**:
  - Read the OS preference: excluded by clarification 2 and *Out of Scope*.
  - A duration of zero passed through `Progress::on_layout_frame`: `step_across` clamps a step to at
    least `f32::EPSILON` of the range and divides by the duration; a zero duration is undefined
    there. `restart_at` is the documented snap. Rejected.
  - Store it daemon-side (service-owned like `diff_layout`): a reduced-motion choice is a property
    of this desktop, not of the daemon host, and the theme beside it is client-side. Rejected.

## R5 — Cards during the zoom: interpolated grid on a ribbon (FR-004, FR-006, SC-002)

- **Decision**: `CardGrid` gains `.zoom(Zoom { full_screen, freed })` (new). When `full_screen`
  flips, the grid's own tree state records the zoom's two end widths: the width it has now and that
  width plus (entering) or minus (leaving) `freed`, the drawer's full width the Overview computes
  from `state.sidebar` (`sidebar_width_px() + ResizeHandle::WIDTH` expanded, the strip's width
  collapsed). At each layout during the zoom it computes:
  - `f` = how far the current width `W` lies between the two end widths, clamped to `0..=1`;
  - the two rest layouts (feature 1024's `columns` rule at each end width) and the card width
    `w = lerp(w_start, w_end, f)`;
  - a **ribbon** placement: card `i` at offset `s = i·(w + gap)` along rows of length `W + gap`, row
    `floor(s / (W + gap))`; a card whose span crosses a row's end is drawn in two clipped pieces, the
    end of one row and the start of the next.
  At `f = 0` and `f = 1` the ribbon is exactly the rest grid (`W + gap = n·(w + gap)` at a rest
  width), so the zoom starts and ends on feature 1024's layout (FR-004). Outside a zoom the grid lays
  out by the rest rule as today.
- **Rationale**: FR-006 asks for continuous widths and positions, cards that change row to travel,
  and no overlap. Moving each card on a straight line from its old to its new cell (FLIP) overlaps:
  for 4 → 5 columns, card 4 moves from row 1, column 0 to row 0, column 4 and crosses cards 1–3 on
  the way (offset of neighbours `(1−e)(−3p, H) + e(p, 0)` reaches `(0, H/4)` at `e = 0.75`). On the
  ribbon, cards keep reading order and a constant pitch `w + gap ≥ w`, so no two overlap on any
  frame; a card that changes row slides off one row's end and in at the next row's start, never
  vanishing. Driving `f` from the laid-out width, not a timer, keeps cards and sidebar in lockstep
  for free: a reversal (FR-007), a snap under reduced motion (R4) and the drawer's real timing all
  reach the grid through `W`. Interpolating `w` between the two rest widths keeps every frame's card
  width between the start and end values (SC-002); following the rest rule frame by frame would not:
  from a 3-column 1150 px grid to a 4-column 1400 px grid, the rule's width climbs to 384 px at the
  1168 px threshold, above both ends (372.7 and 338 px).
- **Consequence for Story 1 #2**: when the zoom raises the column count, a card can be narrower in
  full screen than in the normal layout (the 372.7 → 338 px case above). The scenario's
  requirement, feature 1024's rules at the new width, holds (FR-004); its "so each card is at least
  as wide" clause is explanatory and false in that case. SC-001 (row width) holds in every case.
- **Ends of a zoom**: the grid leaves zoom mode when `W` reaches an end width (within 0.5 px), or
  when `W` leaves the range between them (a window resize mid-zoom), or `SLIDE` plus 100 ms after the
  flip, whichever is first; then the rest rule applies again. A grid built fresh (Overview opened,
  Settings closed) records `full_screen` without a zoom.
- **Alternatives considered**:
  - FLIP (straight-line travel between cells): overlaps, as shown. Rejected by FR-006.
  - A cross-fade between the two layouts: cards disappear and reappear; rejected by FR-006.
  - A fractional column count chasing the rest rule on its own track: widths leave the start–end
    range near a threshold (SC-002), and a second clock can drift from the drawer's. Rejected.
  - Let the grid infer the end width without `freed`: it cannot know it until the drawer stops, by
    which time the interpolation is over. Rejected.

## R6 — Keeping the first visible card on screen (Story 2 #5)

- **Decision**: During a zoom, `CardGrid` finds, each frame, the first card whose rectangle meets the
  scroll viewport (the `viewport` iced hands `update`) and, when the new placement moves it by
  `dy`, publishes `on_reflow(AbsoluteOffset)` (new builder) with the scroll offset moved by `dy`.
  The Overview maps it to `Msg::Reflowed(offset)`; the root answers with
  `iced::widget::operation::scroll_to(GRID_SCROLL_ID, offset)`, as `shell/changes.rs` does for the
  diff. The grid's `Scrollable` gets the id `GRID_SCROLL_ID` (new). Only while zooming; at rest
  nothing is published.
- **Rationale**: Zooming in reduces the row count, so a card well down the grid moves up by whole
  rows (card 20, 4 → 5 columns: one row) and would leave a fixed viewport. Re-reading the first
  visible card each frame means a user's scroll during the zoom changes the anchor rather than being
  fought (Story 2 #5 "the change shows"). A widget cannot move its parent scrollable itself; the
  `publish` → `scroll_to` route is the existing one.
- **Alternatives considered**: scroll once at the end of the zoom (the card leaves view mid-zoom and
  the end jumps); padding the grid's top to hold the anchor in place (leaves a blank band to remove
  at the end). Rejected.

## R7 — The toggle and the shortcut (FR-001, FR-012, FR-014)

- **Decision**:
  - The Overview gains a header row above the grid with, at its end, a material `Button::outlined`
    with a leading glyph: *Full screen* with `Icon::FullScreen` when off, *Exit full screen* with
    `Icon::FullScreenExit` when on (two new `Icon` variants, Material Symbols `fullscreen` U+E5D0
    and `fullscreen_exit` U+E5D1, in the shipped full-coverage font, `assets/fonts/PROVENANCE.md`),
    wrapped in the shared `Tooltip` naming the action and chord, like the top bar's Overview button
    (`ui/toolbar.rs`, `overview_tooltip`).
  - The chord is **Ctrl+Shift+F** (**Cmd+Shift+F** on macOS): `keymap::is_overview_full_screen_chord`
    and `overview_full_screen_chord_label()` (new), built on `is_primary_shift_chord`. It is matched
    only in the window's app-chord path (`ui/mod.rs` `app_chord`), which emits
    `Msg::FullScreenToggled`; the update ignores it while the Overview is closed (clarification 3).
    It is not added to the terminal keymap's reserved tier, so with the Overview closed and a
    terminal focused the key still reaches the terminal as today.
- **Rationale**: `Button` already holds keyboard focus through `TakesTheKeyboard`
  (`ui/material/button.rs`), so Tab reaches it (FR-012) with no new widget; `IconButton` has no
  focus wrapper. The label and glyph change with the mode, which "shows whether the mode is on".
  F is the one Primary+Shift letter near O not taken (E, T, C, V, O, P, D, H, W are; grep of
  `keymap.rs`). The Overview is open only while the terminal is not focused
  (`State::terminal_focused` excludes it), so the app-chord path always sees the chord when it
  matters.
- **Alternatives considered**: `IconButton` (no keyboard focus; would need extending), F11 (the
  usual OS window full-screen key, which the spec puts out of scope and could be read as such), a
  reserved terminal-tier chord (would steal Ctrl+Shift+F from terminal programs while the Overview is
  closed, where it does nothing).

## R8 — Leaving the Overview from full screen (FR-010, Story 1 #4)

- **Decision**: No new code path. Every exit sets `overview.open = false`; `full_screen()` turns
  false on the same update; the drawer's presence target becomes 1 and it zooms out while the main
  area shows the destination (session, previous view) on the same frame. The `NavigationDrawer`
  sits at the same place in the widget tree whether the Overview is open or not (the `row!` in
  `ui/mod.rs`), so its tree state, and the zoom in flight, survive the main area's change; leaving
  during a zoom in reverses from where it is (Edge *Leaving while a zoom in is running*).
- **Rationale**: One action does both (FR-010, SC-005) because one state change drives both.

## R9 — Platforms and several windows (FR-014, Principle VI)

- **Decision**: No platform code. The chord label follows the existing `cfg!(target_os = "macos")`
  pattern of `overview_chord_label`; widths are logical pixels. The mode is per window: it is a
  field of the window's `State`, like `overview.open`.
