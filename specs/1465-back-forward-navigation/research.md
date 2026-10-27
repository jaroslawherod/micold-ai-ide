# Research: Back/Forward Navigation

**Feature**: 1465 | [plan](./plan.md) | [spec](./spec.md)

No NEEDS CLARIFICATION was left in the Technical Context; each item below records a design choice
with the code it was checked against.

## R1 — Record by observing the shown place, not at each writer

- **Decision**: after every message, the binary's `update` (`crates/micold-client/src/main.rs`,
  beside `follow_header` and `report_window_view`, which exist there because "only here sees every
  message") calls one root method, `State::observe_place()`. It derives the shown place
  ([`State::shown_place`], R3) and records it into the history when it differs from the current
  entry. Nothing else records.
- **Rationale**: FR-002 lists seven ways to move (sidebar, tab strip, pane focus, chords, quick
  switcher, notification, opening a session) plus the app's own moves (edge case *Concurrency*).
  Writers of `session.active` alone are many (`tests/current_session_writers.rs` exists to police
  them); the Overview, Compare and Review tab each have their own openers. Observing the derived
  result covers every writer, present and future, with one call site, and FR-003 (equal place adds
  nothing) falls out of the comparison. Background messages that do not change the shown place
  record nothing.
- **Alternatives considered**: a `record` call in each writer (misses future writers; the
  `view_report` and `follow_header` precedent shows the codebase already solved "derived after every
  message" this way); an `Outcome::PlaceChanged` from features (the same coverage problem, spread
  over features).

## R2 — Moving through history suppresses observation until it lands

- **Decision**: a press sets `history.moving` before its follow-up messages run through the root
  `update` (which recurses, so the post-update hook runs for each follow-up) and the shell clears it
  with `State::finish_history_move()` once they have. While `moving` is set, `observe_place` does
  nothing. `finish_history_move` then observes once: when the move landed on the target, the shown
  place equals the current entry and nothing is recorded (FR-007); if it landed elsewhere, that
  place is recorded like any other move, so the history still ends at what is shown.
- **Rationale**: a cross-project target is reached through `project::Msg::Reopened`, whose switch
  restores the project's foreground session first; observing that intermediate place would truncate
  the forward entries (FR-004) and break FR-007.
- **Alternatives considered**: comparing against the target only (still records the intermediate
  place inside the nested `update`); a counter of nested updates (implicit, harder to test).

## R3 — What a place is, and how it is derived

- **Decision**: `micold_core::history::Place { project: PathBuf, view: View }` with
  `View::{Session { location, id }, Review(location), Overview, Compare(GroupId)}`
  ([data-model.md](./data-model.md)). `State::shown_place()` (root, render-free) answers in the
  order the main area draws: none while Settings is open or no project is active; else the Overview when open; else Compare when open; else the shown Review
  tab (`review_shown()`); else the current session (`session.active` with its catalog location);
  else none (a navigation stop with nothing shown, the welcome screen).
- **Rationale**: this is the order `view_facts` and `terminal_focused` already use for "what takes
  the main area" (`app.rs` 618–663). The Overview and Compare are project-wide views; the location
  under them is not what is shown, so they carry none (spec *Key Entities*: two places are equal
  when what is shown is). The Settings panel is not a place (spec *Assumptions*).
- **Alternatives considered**: a place for a navigation stop (the spec's FR-002 list has none, and
  a stop shows no view of its own); a place carrying the pane layout (out of scope by *Assumptions*).

## R4 — History rules live in `micold-core`, generic over the place

- **Decision**: `micold_core::history::History<P: Clone + PartialEq>` holds the entries, the
  position and the rules: `record`, `step(dir, shown, exists)`, the cap of 50, truncation, removal
  of gone entries, and the invariant that no two neighbouring entries are equal. The client supplies
  `exists` and `shown`.
- **Rationale**: FR-015 asks for reducer tests that need no window; `mise run test-core` runs them
  without iced. Generic `P` lets the core tests use plain integers for the list rules and `Place`
  for the equality rules.
- **Alternatives considered**: rules inside the client feature (only testable through the full
  root `State`, slower, and the rules have no client dependency).

## R5 — Whether a target still exists is decided before anything moves

- **Decision**: `features::history::exists(state, place, lookup)` decides with what the window
  knows, plus one injected lookup for the worktrees of a project that is not active:
  - project: known and available (`switcher_entries`' `available`, or the active one);
  - location: `Default` always; `Worktree(dir)` listed in `worktree.worktrees` for the active
    project, or in `lookup(project)` for another;
  - session: a non-archived record in `workspace.sessions[project]` at that location ("stopped" is
    `CloseRequested`, which archives: `features/session.rs` 1468, 1982);
  - Review: its location exists; Overview: its project; Compare: its project, and for the active
    project its group in `runs.groups`.
  The shell passes `|project| discover_worktrees(git, project)` (`shell/workspace.rs` 91), the same
  synchronous local read a project switch runs; tests pass a map.
- **Rationale**: FR-006 says Back lands on an entry whose target exists. Checking after a project
  switch would leave the window in the other project when the target turned out gone, which is a
  move the user did not ask for. The worktree list of a non-active project is not in `State`
  (`worktree.worktrees` is the active project's), so it is read, not guessed.
- **Known limit**: run groups are read from the service per active project, so a Compare entry of
  another project is checked by its project only. If its group was dismissed meanwhile, the Compare
  view's own `RunGroupsChanged` rule closes it when the groups arrive (existing behaviour) and the
  place then shown is recorded (R1). Recorded here rather than solved: dismissing a group is an
  explicit user action in that project, and the gap is one view for one press.
- **Alternatives considered**: switching first and re-checking (the problem above); a request /
  answer round trip for the worktree read (one more message pair for a read the switch already does
  synchronously).

## R6 — Showing a place reuses the existing messages

- **Decision**: the root's `State::apply_history(step) -> Vec<Message>` maps a target to the
  messages the existing controls send, dispatched through the root `update` as
  `shell/navigation.rs::apply_step` does: a project switch first when the target is in another
  project (`project::Msg::Reopened`, which switches synchronously, discovers worktrees and restores);
  then `session::Msg::Selected(id)` (whose full path closes the Overview and Compare and shows the
  session tab), or the covers closed (`overview::Msg::Closed`, `runs::Msg::CompareClosed`) followed
  by `sidebar::Msg::ReviewChangesRequested(location)` (reopens a closed Review tab, spec edge case),
  `overview::Msg::Opened { settings_open: false }`, or `runs::Msg::CompareOpened { group, project }`.
- **Rationale**: spec *Assumptions*: showing a place restores it as a sidebar click would. One
  apply path per view keeps the history from drifting from the controls.
- **Alternatives considered**: writing the fields directly (bypasses the daemon report, the
  Changes reads and the strip's reveal, which the messages already do).

## R7 — Mouse side buttons: one window-wide listener, status ignored

- **Decision**: `shell/subscriptions.rs` always subscribes `iced::event::listen_with(side_button)`;
  `side_button` maps `mouse::Event::ButtonPressed(Button::Back | Button::Forward)` to
  `Message::History(Msg::Pressed { dir, source: Source::Button })`, whatever the event status, and
  ignores releases and every other event. It is subscribed outside `ui::subscription`'s early
  return for a focused terminal, so FR-008 holds over terminals.
- **Rationale**: iced 0.14 delivers every interaction to `listen_with` with its status
  (`iced_futures/src/event.rs`), so a widget that captured the press does not hide it. winit 0.30
  reports the side buttons as `Back`/`Forward` on X11 (buttons 8/9), Wayland (`BTN_SIDE`/`BTN_BACK`,
  `BTN_EXTRA`/`BTN_FORWARD`), Windows (`XBUTTON1`/`XBUTTON2`) and macOS (buttons 3/4), so one match
  covers Principle VI with no platform branch. The terminal pane reports only Left, Middle and Right
  presses (`ui/material/terminal_pane.rs` 1204, 1367, 1552, 1578) and does not capture `Back` or
  `Forward`, so no bytes reach a program (FR-008, edge case *Terminal mouse reporting*); a pane
  test pins that.
- **Alternatives considered**: handling the buttons inside each widget (every widget would need
  it); `keyboard::listen`-style `event::listen` (drops captured events).

## R8 — Keys: Alt+←/→, Cmd+[ / Cmd+] on macOS, never from a terminal or a field

- **Decision**: `keymap::history_key(&KeyInput) -> Option<HistoryDir>`: on Linux and Windows,
  exactly Alt (no Ctrl, Shift or Super) with `ArrowLeft` / `ArrowRight`; on macOS exactly Cmd with
  physical `BracketLeft` / `BracketRight` (logical `[` / `]` fallback when no code, as `nav_chord`
  does). The platform split sits in `keymap.rs` beside `is_primary_shift`, the existing abstraction
  (Principle VI). `ui/mod.rs::app_chord` maps it to `Msg::Pressed { dir, source: Source::Key }`.
  It is not a tier-1 chord: `encode` is unchanged, so a focused terminal still gets Alt+← bytes
  (`ESC [1;3D`) and Cmd+[ stays `Ignore` (SC-003).
- **Field rule**: `app_chord` hears only presses no widget captured (`keyboard::listen` keeps
  `Status::Ignored`, `iced_futures/src/keyboard.rs`), and is not subscribed while a terminal is
  focused (`ui::subscription`). iced's `text_input` and `text_editor` capture arrow keys with any
  modifier, but not Cmd+[ (`iced_widget` 0.14 `text_input.rs` 1040–1257: no arm for it), so on
  macOS capture alone would let a focused field's Cmd+[ go Back. The reducer therefore also refuses
  a `Source::Key` press while a field holds the keyboard: `state.live_focused_field()`, the
  rule `changes::keys::review_holder` already applies (a closed review comment box, or a session's
  composer and queued message hidden by the Review tab, do not hold it), extracted so both share it.
- **Overview arrows**: `ui/mod.rs::overview_arrow` steps cards on any arrow regardless of modifiers;
  it skips a press that is a history key, so Alt+← in the Overview does one thing.
- **Rationale**: spec Clarifications (macOS chord), FR-009, FR-012 (auto-repeat arrives as further
  `KeyPressed` events, which `app_chord` does not filter).
- **Alternatives considered**: tier-1 (reserved from terminals) — contradicts FR-009; Option+←/→ on
  macOS — word movement (Clarifications).

## R9 — The Review tab's own Back first

- **Decision**: Back, when the shown place is a Review tab and `changes::can_go_back(view)` holds,
  resolves to `Step::ReviewBack`, applied as `changes::Msg::BrowseBack` (what the tab's **Back**
  button sends, `ui/changes.rs` 627). The global position does not move; the view changes inside the
  same place, so observation records nothing. Forward never consults it (FR-011).
- **Alternatives considered**: putting Review browse frames into the global history (two histories
  would then disagree about the same press).

## R10 — Gate: the navigation chords' rule

- **Decision**: a press does nothing while Settings is open or any registered surface is open
  (`overlay::registry::topmost`), exactly `features::navigation::update`'s first line; that check is
  extracted to `navigation::blocked(state)` and both call it. The Review tab's shortcut sheet, the
  quick switcher, dialogs and menus are registered surfaces.
- **Rationale**: FR-010 and the spec's *Surfaces open* edge case say "as the navigation chords do
  today".
- **Alternatives considered**: a second copy of the check in the history feature (two answers that
  can drift; `mise run duplication` budgets exactly this); gating in the listeners (the side-button
  listener cannot see the state, and a subscription that comes and goes per surface would drop a
  press in flight).

## R11 — Discoverability

- **Decision**: `keymap::history_key_labels(dir) -> (String, String)` gives `("Alt+←", "Cmd+[")`
  and `("Alt+→", "Cmd+]")`; the docs row and the Review sheet render from it. The sheet appends two
  rows after the `REVIEW_KEYS` rows in `changes::sheet_model`, whatever `single_keys` is (FR-014);
  `REVIEW_KEYS` is unchanged, so `micold-core/tests/review_keys_doc.rs` and the review keys table in
  `reviewing-changes.md` are untouched (spec *Out of Scope*). `terminal-panes.md`'s app-chord table
  gains Back and Forward rows naming the mouse buttons, and its intro says a focused text field or
  terminal keeps those two keys (FR-013).
- **Alternatives considered**: rows in `REVIEW_KEYS` (its `KeyName` has no arrows, its rows are
  review actions resolved by `review::keys::resolve`, and the guide's review table is compared
  against it, which the spec keeps out of scope); an app-wide sheet (spec *Out of Scope*).
