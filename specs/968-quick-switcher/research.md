# Research: Quick-Switcher

**Feature**: 968 | **Plan**: [plan.md](./plan.md) | **Spec**: [spec.md](./spec.md)

No `NEEDS CLARIFICATION` remained after clarify. Ledger decision D4 (only the active project's
locations and sessions are entries, option A) is a default pending the user's confirmation. R4 keeps
every "which entries exist" rule in one builder function, so flipping to option B (all projects) is
a change to that function, its tests and the spec's FR-003/FR-004, and nothing else.

## R1 — The chord: one more reserved chord in `keymap.rs`, matched by physical key

- **Decision**: `PhysicalKey` gains `KeyP`; `to_keymap_physical` maps `Code::KeyP` to it, and the
  logical fallback (platform gave no code) maps `Key::Char('p' | 'P')`. A new
  `is_switcher_chord(input: &KeyInput) -> bool` is `is_primary_shift(mods)` plus that physical key.
  `KeyOutput::ToggleSwitcher`, checked in `encode`'s tier 1 beside the navigation and Overview
  chords (never forwarded: 0 bytes, SC-002). `route_key` maps it to `KeyRouting::ToggleSwitcher`
  before its `terminal_focused` check, like `ToggleOverview`. `switcher_chord_labels() -> (String,
  String)` gives `Ctrl+Shift+P` / `Cmd+Shift+P` for the docs test
  ([contracts/keys.md](./contracts/keys.md)).
- **Rationale**: the #803 chords already decode by physical key in the same tier and route the
  same two ways (focused terminal, unfocused `app_chord`); one more row in that shape costs no new
  mechanism. FR-016 needs the physical key: on AZERTY the P position still types `p`, but on layouts
  such as Dvorak it types `l`, which the logical key alone would miss.
- **Alternatives rejected**: `is_primary_shift_chord(&key, mods, 'p')` like the Overview's letter
  chord (logical letter only, fails FR-016); adding a row to `NAV_CHORDS` (it is typed
  `NavAction`, and the switcher is not a step).

## R2 — Matching and ranking: a new core module that reuses the type-ahead's folding

- **Decision**: new `micold-core/src/quick_switch.rs`. It folds names with the type-ahead's
  `Query` and per-character fold (so offsets index the original name) and decides three tiers in
  order: `Prefix` (the folded name starts with the query), `Run` (contains it as one run),
  `Scattered` (characters in order, greedy leftmost). Within a tier the input order (FR-004) is kept
  by a stable sort. The spans for emphasis come from the same pass. `typeahead.rs` exposes its
  `literal` and `subsequence` tier functions as `pub(crate)` for this module instead of a copy.
- **Rationale**: FR-005 says "exactly the entries whose name contains the query's characters in
  order": the type-ahead's `match_one` also keeps single-edit typos from three characters on and has
  no prefix tier, so it would list entries FR-005 forbids. The tier functions themselves are the
  same rules and are reused.
- **Alternatives rejected**: `typeahead::rank` as is (typo tier, no prefix rank); a fuzzy-matching
  crate such as `nucleo` or `fuzzy-matcher` (scores by gaps and bonuses, which is not FR-005's
  three-rank rule, and a new dependency to vet); matching in the client (business rule out of
  `tests/` reach, Principle I).

## R3 — Sessions kept by their location

- **Decision**: each candidate carries `parent: Option<usize>`, the index of its location's
  candidate. `filter` first ranks every candidate on its own name; then a location that matched
  pulls in its sessions that did not match, placed right after it in session order, with the
  location's rank (FR-005, D5). A session that matched on its own name keeps its own rank and
  place; it is not listed twice.
- **Rationale**: D5 ("typing a worktree's name and then moving down reaches its helpers").
- **Alternatives rejected**: grouping every session under its location whatever its own rank (a
  session found by name would then sort by its location's rank, against FR-005's ranking).

## R4 — Which entries exist: one builder in the client, reading the sidebar's own nodes

- **Decision**: `features/quick_switch.rs::candidates(&State) -> Vec<Entry>` builds the list in
  FR-004 order from what the sidebar and switcher already derive: `navigation::location_order`
  (after the tag filter and collapsed run groups, #803 *Location order*), each location's node via
  a new shared `navigation::location_node` (name, tags, `unfolded_sessions()` as *Session order*),
  and `State::switcher_entries()` minus the active project (unavailable ones included,
  `selectable = available`). With no active project (welcome screen) there are no location or
  session entries, and every known project is listed. The list is derived at each view and each
  message, never stored.
- **D4 containment**: option B would change only `candidates` (iterate the other projects' stored
  locations and sessions), its tests, and the choose path for a non-active entry (switch project,
  then select). `quick_switch` in core, the view and the keys are unchanged.
- **Rationale**: FR-008 asks for exactly the sidebar's result; reading the same nodes the sidebar
  draws keeps the names, tags and order identical. `location_node` replaces three copies of the same
  `SidebarEntry` match in `navigation.rs` (`main_session_at`, `session_step`), so the new reader
  removes duplication instead of adding a fourth.
- **Alternatives rejected**: reading `state.session` and `state.worktree` directly (would
  re-derive the filter, the fold and the group rules, and drift from the sidebar).

## R5 — Choosing: the #803 step and its apply path

- **Decision**: a chosen entry resolves to a `navigation::Step`: a session to `Step::Session(id)`;
  a location to `Step::Session(main)` when it has a main session (what a row press selects,
  FR-018), else `Step::Stop(location)` (its start action shows, no session starts, its row takes
  the sidebar's keyboard, US1-7); a project to `Step::Project(path)`. The shell runs
  `State::apply_navigation` / `finish_navigation` exactly as `shell/navigation.rs` does, with the
  `terminal_had_focus` recorded **when the quick-switcher opened** (while it is open the terminal
  never holds the keyboard). A chosen key that is not in a fresh `candidates` (removed while open),
  or an unavailable project, resolves to nothing: the switcher stays open (FR-013).
- **Overview**: `Selected` already closes the Overview. A `Stop` chosen with the Overview open also
  closes it (`Message::Overview` toggle) before applying the stop, so the row it reveals is visible.
- **Rationale**: FR-008 and FR-010 are #803's FR-002/003/004/010 for a different trigger; the apply
  path is tested and handles reveal scroll, row opening and keyboard placement.
- **Alternatives rejected**: sending `SessionMsg::Selected` directly from the switcher (loses the
  stop case and the #803 focus rule, and copies `finish_navigation`).

## R6 — The surface: a registered dialog with a type-ahead

- **Decision**: `QuickSwitcherDialog` is a `FloatingSurface` registered in `overlay/registry.rs` on
  `Layer::Dialog`, cancelled by `quick_switch::Msg::Dismissed`. The registry then gives, with no
  new code: Escape and a click on the scrim close it (`Surface::Dialog` dismisses on Escape and
  OutsideClick, FR-009); the terminal yields the keyboard while it is open and takes it back when
  it closes (`terminal_focused` is derived, feature 023), and the #803 chords do nothing while it
  is open (their U1 gate reads `registry::topmost`, FR-012). The view, `ui/quick_switcher.rs`,
  composes the shared `material::dialog` with a `material::Typeahead` held open (`.open(true)`),
  rows as `picker::Row` (label: the name with match spans; details: kind and context; `enabled`:
  selectable), `.highlighted(..)`, `.selected(..)` for the current entry, `.empty_message(..)`.
- **Rationale**: the registry is the one place surfaces are made dismissible and keyboard-taking;
  the type-ahead already has the field, the ranked-row list, emphasis, truncation with "…" around
  the emphasis (`fit_around`) and Up/Down/Enter/Escape intents. Principle VIII: no new widget.
- **Alternatives rejected**: `Layer::Popover` anchored to the top bar (popovers dismiss on any
  scroll beneath and the switcher is not attached to a trigger); a bespoke list widget (Principle
  VIII, and it would copy the type-ahead's key rules).

## R7 — Highlight and keyboard movement

- **Decision**: the switcher stores the highlighted entry by **key** (`EntryKey`), not index. Core
  `quick_switch::reseat(list, key)` gives its index when it is still listed, else the first
  selectable entry (FR-005, FR-013). `quick_switch::step(list, index, dir)` moves to the next
  selectable entry above or below and stops at the ends (FR-007). The type-ahead's `on_move`
  publishes the direction; its Enter intent picks the highlighted row only when the row is enabled,
  so Enter on nothing or an unavailable project does nothing (FR-014). Tab: the type-ahead's
  `Dismiss` intent, which the switcher treats as a close without choice.
- **Rationale**: `typeahead::move_highlight` saturates but does not skip disabled rows; skipping is
  FR-007's and goes in core beside the rank.
- **Alternatives rejected**: an index highlight (FR-013 needs it to stay on the same entry as the
  list changes).

## R8 — Opening, closing and returning the keyboard

- **Decision**: `quick_switch::Msg::Toggled { terminal_had_focus }` is published by both chord
  paths; the reducer closes an open switcher, opens it only when no surface is open and Settings is
  closed (FR-012, `navigation::update`'s gate), and records `terminal_had_focus` and the sidebar's
  `keyboard_row` at open. `main.rs` focuses the field by id on open (as it focuses the Overview's
  first card). Closing without a choice restores `keyboard_row` (the field's focus cleared it via
  `KeyboardRowLeft`) and the terminal takes the keyboard back by derivation. The chord pressed
  inside the focused field: `app_chord_listener` hears it when the field leaves it uncaptured; a
  client test drives the dialog with the chord to prove it, and if the field captures it the
  dialog's key handler maps it to `Toggled` (contracts/keys.md K3).
- **Rationale**: FR-001, FR-009, FR-010 with no restore stack for the terminal (feature 023's
  derived focus).
- **Alternatives rejected**: remembering an arbitrary iced widget id to refocus (the app has no
  general "focused widget" query; the two places keyboard focus can be before the chord, terminal
  and sidebar row, are both covered, and any other focused field already loses its focus to any
  dialog today).

## R9 — Performance (SC-003)

- **Decision**: `filter` is one pass over the candidates per keystroke with the query folded once
  (the type-ahead's measured shape, 500 candidates within its frame budget, feature 021 SC-002). A
  core test builds 500 candidates and asserts each of 10 successive queries filters in under 100 ms.
- **Alternatives rejected**: caching filtered lists between keystrokes (state to invalidate when
  the lists change, FR-013, for no measured need).

## R10 — Docs

- **Decision**: `docs/user-guide/terminal-panes.md` §Keyboard shortcuts gains a row "Quick-switcher
  (jump to a location, session or project)" with both forms; `docs/user-guide/worktrees-and-sessions.md`
  gains a **Quick-switcher** bullet beside **Navigation** (what it lists, matching and ranking,
  Up/Down/Enter/Escape, unavailable projects, active project only). A client test checks both
  strings of `switcher_chord_labels()` appear in both files (FR-017, SC-006), as #803's docs test does.
