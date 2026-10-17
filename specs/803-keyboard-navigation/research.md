# Research: Keyboard Navigation of Projects, Worktrees and Sessions

**Feature**: 803 | **Plan**: [plan.md](./plan.md) | **Spec**: [spec.md](./spec.md)

No `NEEDS CLARIFICATION` remained after clarify. D4 (PageUp/PageDown for the project chords) and
D5 (no direct jumps) are recommended defaults pending the user's confirmation; R1 keeps every chord
in one table so a different choice is a one-table change plus the docs.

## R1 — One chord table, matched by physical key

- **Decision**: `micold-client/src/keymap.rs` gains a render-free `PhysicalKey` (`BracketLeft`,
  `BracketRight`, `Comma`, `Period`, `PageUp`, `PageDown`, `Other`), a `NavAction { level:
  NavLevel, dir: NavDir }` and **one** `const NAV_CHORDS: [(NavAction, PhysicalKey); 6]`
  ([contracts/keys.md](./contracts/keys.md) K1). `pub fn nav_chord(input: &KeyInput) ->
  Option<NavAction>` requires the platform primary modifier plus Shift with no other modifier (the
  `is_primary_shift_chord` modifier rule, factored out as `is_primary_shift(mods)`) and looks the
  physical key up in `NAV_CHORDS`. `nav_chord_labels(action) -> (String, String)` renders both platform
  forms from the same table (`Ctrl+Shift+[`, `Cmd+Shift+[`), so a test can compare the docs with it.
- **Physical key (FR-016)**: `KeyInput` gains `physical: PhysicalKey`, filled in
  `ui/material/terminal_pane.rs` by a new `to_keymap_physical(&keyboard::key::Physical)` mapping
  `Code::BracketLeft|BracketRight|Comma|Period|PageUp|PageDown`. When the platform reports
  `Physical::Unidentified`, the logical key is the fallback: `[ {` → BracketLeft, `] }` →
  BracketRight, `, <` → Comma, `. >` → Period, named PageUp/PageDown. Matching the logical key alone
  would fail on a US layout already, because Shift turns `[` into `{`.
- **Rationale**: one table decides the focused path, the unfocused path and the labels; changing a
  chord (D4) edits one array entry and the docs rows, and the keymap tests read the table.
- **Alternatives rejected**: six `is_*_chord` functions like the Overview's (six places to change
  per decision, and letter-only `eq_ignore_ascii_case` cannot express punctuation); matching the
  produced character (breaks on non-US layouts, FR-016).

## R2 — Routing: tier 1, never forwarded, both focus states

- **Decision**: `encode` checks `nav_chord` in tier 1 beside the Overview chord and returns
  `KeyOutput::Navigate(NavAction)`. `app::route_key` routes it to `KeyRouting::Navigate` whatever
  the terminal focus, as it does `ToggleOverview`. `terminal_pane.rs` publishes
  `Message::Navigate(action)` and captures the event when the pane is focused (0 bytes, SC-002).
  Unfocused: the existing `overview_chord_listener` becomes `app_chord_listener` with one named
  `fn app_chord(event) -> Option<Message>` answering both the Overview and the navigation chords
  (one listener, one `TypeId`, active under the same condition as today).
- **Rationale**: the 813 pattern (research R3 there) proven against double handling: the focused
  pane publishes only when `self.focused`, the listener only runs when the terminal is not focused.
  Tier 1 precedes named-key and control-byte encoding, so Ctrl+Shift+PageUp never becomes an
  escape sequence and Ctrl+Shift+[ never becomes ESC (FR-007, FR-008); plain Ctrl+[ has no Shift
  and still reaches `ctrl_byte`.
- **Alternatives rejected**: a separate listener per chord family (two `TypeId`s doing the same
  decode); handling in a global subscription only (the pane captures first, 813 R3).

## R3 — Where the order and the step live

- **Decision**: a pure `micold-core/src/navigation.rs`: `Dir { Next, Previous }` and
  `fn step<T: PartialEq + Clone>(order: &[T], current: Option<&T>, dir: Dir) -> Option<T>` with the
  FR-005 rules (wrap both ends; one entry stays; empty → `None`; `current` absent from `order` →
  first for Next, last for Previous, as the spec's "no current location" edge case). The client
  builds the three orders from what the views already draw:
  - **Location order**: `App::location_order()` in `features/navigation.rs`, walking
    `sidebar_entries()` — `Default`, each `Worktree`, and the `RunRow::Worktree` rows of an
    **expanded** run group in drawn order. A collapsed group's worktrees are not shown, so they are
    skipped like filtered ones (spec Terms: "as shown").
  - **Session order**: the current location node's `listed_sessions()` (main, then helpers in
    spawn-tree order, feature 551).
  - **Project order**: `switcher_entries()` filtered to `available`.
- **Rationale**: the orders are read from the same functions the sidebar and switcher render from,
  at the moment of the press (Principle II edge case), so the keyboard cannot disagree with the
  screen. The wrap arithmetic is generic and tested once in core.
- **Alternatives rejected**: a cached order updated on catalog changes (stale under background
  changes); three copies of the wrap rule.

## R4 — "Current location" when the keyboard lands on a location with no session

- **Decision**: clicking a location without a session selects nothing (776 FR-018), so landing on
  one cannot change `session.active`. The sidebar keeps a **navigation stop**,
  `sidebar.nav_stop: Option<SessionLocation>`, set when a location chord lands on such a location
  (empty, or missing/invalid with no session). The current location for the next press is
  `nav_stop` when set and still in location order, else the current session's location
  (`current_session_location`). `nav_stop` is cleared by every change of the current session
  (`set_current_session` and `session::selected`) and by `project_entered`. The terminal keeps
  showing what it showed; the row shows its start action as it always does when free; the
  sidebar's keyboard focus moves to that row (FR-010) and scrolls it into view (FR-009).
- **Rationale**: without the stop a second press would compute from the old session's location and
  land on the same empty row forever. Nothing is started (FR-011) and SC-004's highlight rule stays
  true: the highlighted row is still the session the terminal shows; the keyboard focus ring marks
  the stop.
- **Alternatives rejected**: skipping empty locations (contradicts the spec's "a stop like any
  other"); starting a session there (FR-011).

## R5 — Selecting a session: the click path plus reveal

- **Decision**: a session step dispatches the existing `Message::Session(SessionMsg::Selected(id))`
  so everything a click does happens (closes Changes, Compare and the Overview, `daemon_sync::
  on_session_selected`, `focus_terminal`), then applies what a click deliberately does not
  (024 §3.0: a clicked row is already visible): `Outcome::RevealScrollArmed`, so a row off screen
  is scrolled to (FR-009). The current location's row is opened by the existing
  `location_open`/`effective_open` rule, which already reveals the current session's location.
  `tests/current_session_writers.rs` stays green: no new writer of `session.active`.
- **Rationale**: "exactly as if its row had been clicked" (FR-002, FR-003) is met by sending the
  click's own message.
- **Alternatives rejected**: calling `set_current_session` directly (skips the shell side of
  `Selected`, such as the daemon foreground report).

## R6 — Projects: the switcher's own message

- **Decision**: a project step dispatches `Message::Project(ProjectMsg::Reopened(path))`, the
  message the switcher's menu item sends. With no active project (welcome screen) `step` gets
  `current = None`, so Next → first available, Previous → last (spec edge case). No available
  project → nothing.
- **Rationale**: FR-004 "same result as picking it in the switcher", including session restore and
  the Overview behaviour of a project switch.

## R7 — Keyboard focus after a press (FR-010)

- **Decision**: the handler reads `state.terminal_focused()` **before** acting. If it was true and
  the step showed a session, `Selected` already focuses the terminal: done. Otherwise the terminal
  is released (`release_terminal`) and `sidebar.keyboard_row: Option<RowRef>` (`RowRef::Location(
  SessionLocation)` or `RowRef::Session(SessionId)`) names the row to hold the keyboard. For a
  project step, the row is the restored current session's (`RowRef::Session`), resolved by identity
  once the project's rows arrive. `keyboard_row` is cleared when the row reports losing focus
  (`on_focus_change(false)`: a click elsewhere, Tab) and by any pointer selection.
- **Rendering**: `TreeItem` (shared, `ui/material/tree_view.rs`) gains
  `.keyboard_focus(focused: bool, on_change: impl Fn(bool) -> M)`: the row is wrapped in the
  existing `TakesTheKeyboard` with `focused` supplied by the app and the focus indicator drawn. Only
  the row named by `keyboard_row` is wrapped as enabled, so the Tab order of the rest of the
  sidebar is unchanged. `TakesTheKeyboard` gains `shown(bool)`: an app-supplied keyboard focus is
  keyboard use and draws the ring (today a supplied focus sets `focused` but not `visible`).
- **Rationale**: focus by supplied flag is the mechanism the application already uses
  (`keyboard_focus.rs` `Focus::supplied`, BUG-004), so no widget-id operation is needed and the
  state is testable without a GUI.
- **Alternatives rejected**: `iced::widget::operation::focus(id)` (rows have no ids; an operation is
  not observable in the render-free tests); making every row focusable (changes the Tab order of a
  long list, out of scope).
- **Scroll**: a `RowRef::Location` stop is not the current session's row, so the reveal scroll is
  generalised: `reveal_scroll_offset` takes its row index from `keyboard_row` when set, else from
  the current session (`current_session_row` → a new `row_index(entries, &RowRef)` that both use).

## R8 — When the chords do nothing (FR-012)

- **Decision**: `features/navigation.rs` `update` returns no step when
  `state.settings.settings_draft.is_some()` or `overlay::registry::topmost(state).is_some()` (every
  dialog, context menu — the terminal's own included — and the project switcher panel are
  registered surfaces). The Overview is a main-area view, not a surface: a location or session step
  closes it through `Selected` (R5); a project step leaves it to the project switch (813 FR-010).
- **Rationale**: the registry is the one maintained list of open surfaces (024 FR-009), as
  `any_surface_takes_keyboard` already relies on.

## R9 — Conflicts (FR-006, SC-003)

- **Decision**: a keymap test asserts that no `NAV_CHORDS` entry decodes to any other `KeyOutput`
  with Shift and the primary modifier (E, T, O, D, H, W, Arrows, C, V) and that every entry differs
  from the others. The platform-reserved list is checked by review against spec *Clarifications*
  (D3); it is not machine-checkable.

## R10 — Documentation (FR-013, SC-006)

- **Decision**: rows for the six chords in `docs/user-guide/terminal-panes.md` §Keyboard shortcuts
  (whose intro becomes "the app takes them" for both focus states) and a **Navigation** bullet in
  `docs/user-guide/worktrees-and-sessions.md` beside the Overview bullet, with the wrap rule, the
  skipped entries (filtered, collapsed group, unavailable project) and the macOS laptop Fn note.
  A client test (`tests/keymap.rs`) reads both docs and checks both forms of every `nav_chord_labels` appear
  in each, so a chord change (D4) cannot leave the docs behind.
