# Research: Worktree Overview

**Feature**: 813 | **Plan**: [plan.md](./plan.md) | **Spec**: [spec.md](./spec.md)

The spec has no open `NEEDS CLARIFICATION`. Clarification D5 (the shortcut while Settings is open)
was deferred to this plan and is settled in R2. Each entry is a decision with the alternatives
rejected. Paths are under `crates/` unless they start with `docs/` or `specs/`.

## R1 — Where the Overview renders and how it closes

- **Decision**: a new main-pane view, picked in `micold-client/src/ui/mod.rs` `view_with` after
  Settings and before Changes and Compare: Settings → **Overview** → Changes → Compare → terminal →
  `shell::view`. `main_content_key` gets the sentinel `u64::MAX - 3` (so the switch crossfades
  through the existing `material::ViewFade`), and the same list in `app.rs` `terminal_focused` and
  `view_facts.main_area_taken` gains the Overview. Opening the Overview does not close Changes or
  Compare: they stay underneath, so closing it returns the view shown before (FR-010) with no
  "previous view" bookkeeping. A sidebar selection (`main.rs` `Message::Session(SessionMsg::Selected)`,
  which already closes Changes and Compare) also sends `overview::Msg::Closed` (FR-023).
- **Escape**: `app.rs` `on_escape` consults the overlay registry first (a dialog or menu over the
  Overview closes first), then the Settings draft, then closes the Overview. Escape reaches
  `on_escape` only while the terminal is not focused; the terminal is never focused while the
  Overview is open (it is not on screen, and `terminal_focused` returns false then).
- **Rationale**: the spec's clarification (D4) puts it in the main pane like Changes and Compare;
  the stack order means the Overview always wins while open, and what was under it is untouched.
- **Alternatives rejected**: closing Changes/Compare on open and re-opening them on close (needs a
  saved view and re-reads); a `Registered` floating surface (spec: not a dialog).

## R2 — The shortcut and button while Settings is open (clarification D5)

- **Decision**: while Settings is open the shortcut and the button do nothing; the Settings draft
  is neither saved nor discarded. The Overview's open flag is not changed either, so an Overview
  that was open before Settings shows again when Settings closes.
- **Rationale**: nothing yields Settings to another main-pane view today. Settings takes the whole
  content area (sidebar included), closes only by save or cancel (`features/settings.rs` `saved`,
  `cancelled`), and the Changes and Compare views are reached from the sidebar, which is hidden
  while Settings is open. Toggling the Overview would either discard an unsaved draft or open a
  view that cannot show; doing nothing follows the existing behaviour. The top bar stays on
  screen above Settings (`ui/mod.rs` renders `toolbar::view` above every body branch), so the
  button is drawn disabled while Settings is open, and the shortcut is ignored.
- **Alternatives rejected**: closing Settings (loses or silently saves a draft); queuing the
  toggle until Settings closes (surprising delayed view change).

## R3 — The reserved chord

- **Decision**: `micold-client/src/keymap.rs`: `fn is_overview_chord(key, mods) -> bool` =
  `is_primary_shift_chord(key, mods, 'o')`, made `pub` so both paths share it; `encode` returns a
  new `KeyOutput::ToggleOverview` in tier 1 beside `ReleaseFocus`, `NewTerminalInstance` and
  `Pane` (never forwarded). `ui/material/terminal_pane.rs` publishes it like `NewTerminalInstance`
  and captures the event, so the PTY receives 0 bytes (SC-006). `app.rs` `route_key`/`KeyRouting`
  route it to `overview::Msg::Toggled`. Unfocused: a new `keyboard::listen().filter_map(
  overview_chord)` in `ui/mod.rs` `subscription`, with a named `fn` (iced keys subscriptions by
  `TypeId`, so no closure), active while the terminal is not focused, exactly the condition under
  which the existing Escape/Tab listeners run.
- **Rationale**: the same predicate decides in both paths, so the focused and unfocused chords
  cannot drift; the tier-1 position is the one FR-011 names ("reserved like the focus-out and
  new-terminal chords"). `O` differs from E, T, D, H, W, arrows, C and V.
- **Alternatives rejected**: one global listener for both cases (the terminal pane captures the key
  before a global listener sees it, so the PTY would still get `^O` / `0x0f`).

## R4 — Card state rules

- **Decision**: a pure core module `micold-core/src/overview.rs` with `CardState` (a closed enum
  whose derived `Ord` is worst first: `NeedsAction < NeedsAttention < Working < Done < Idle`),
  `CardReason`, `CardSignals` and `card_state(&CardSignals) -> (CardState, CardReason)`, exactly
  the table in [data-model.md](./data-model.md). The location state is feature 905's
  `location_state` over the location's un-archived sessions (decision D3), so card and sidebar row
  cannot disagree.
- **Rationale**: one total function over closed inputs is exhaustively testable (SC-003); the
  `Ord` gives both "worst wins" and the card order (FR-007).
- **Alternatives rejected**: deriving in the view (untestable glue); a numeric score (hides the
  reason the tooltip needs).

## R5 — Agent requests per location

- **Decision**: `features/agent_confirm.rs` `Prompt` gains `session: SessionId`, filled from the
  wire's `caller` in `shell/daemon_sync.rs` (the `DaemonMsg::ConfirmationRequested` arm). A
  location has an agent request waiting when a queued prompt's session is one of the location's
  un-archived sessions (`Session::location`). Client only; no protocol change.
- **Rationale**: the wire already names the caller (`protocol/messages.rs` `ConfirmationRequested`);
  the client dropped it. A withdrawn or answered prompt leaves the queue, so the state clears with
  no extra message.
- **Alternatives rejected**: matching by `caller_label` (not unique); a new daemon message.

## R6 — Pull request counts and the conflict signal

- **Decision**: extend the existing status reading (`micold-core/src/pull_request.rs`
  `status_query`/`parse_status`), with no new poller and no new cadence (FR-019):
  - `FRAGMENT` gains `mergeable`; `PullRequestStatus` gains `conflict: bool` (`CONFLICTING` →
    `true`; `MERGEABLE` and `UNKNOWN`, which GitHub reports while it computes, → `false`).
  - Two light aliases per branch: `oc{i}: pullRequests(headRefName: $b{i}, states: OPEN, first:
    100) { nodes { isCrossRepository } }` and `mc{i}` the same with `states: MERGED`. The counts
    are the non-cross-repository nodes, so forks with the same branch name are not counted, the
    same rule `select_pull_request` uses. A full page reads as "100+" (`PrCounts::capped`).
  - `parse_status` returns the counts beside the statuses: `StatusReading { statuses, counts:
    BTreeMap<String, PrCounts> }`, with a count entry for **every** asked branch (zero included),
    so "no pull request" (0 / 0) differs from "unknown" (no entry).
  - Client `features/pr_status.rs` `State` gains `counts`; `Outcome::Ok` and `Msg::StatusesRead`
    carry them; `Err(Unavailable)` clears them with the statuses; a disabled reading, a
    rate-limited pause with no reading yet, or a branch not yet read gives `None` (unknown).
- **Rationale**: the status reading already asks per listed branch every 300 s
  (`shell/subscriptions.rs` `PR_STATUS_INTERVAL`) and on triggers; two node-light aliases add
  little to the query cost. `totalCount` was rejected because it counts fork pull requests.
- **Unknown reasons** (tooltip, edge case): *pull request status is off* (`enabled == false`),
  *GitHub is not reachable* (`Unavailable`), *rate-limited until HH:MM* (`pause_until` with no
  reading), *not read yet*.
- **Default row**: never has pull request data (`features/sidebar.rs` `row_pull_request`; the
  project root's branch is not asked), so its card shows no counts and no PR-derived state.
- **Alternatives rejected**: feature 775's per-branch list (`list_query`, first 100, full nodes;
  read only on demand from Changes); `totalCount`; raising `o{i}` to `first: 100` (heavy nodes).

## R7 — Location type and ticket

- **Decision**: core `overview::location_type(worktree facts) -> LocationType`: Included when
  `Worktree::included`; OrphanInvalid when `WorktreeStatus` is `Missing` or `Invalid`; ProjectRoot
  for Default; else by the branch type from `naming::parse_tags(dir_name)`'s `Tag::Type`:
  `Feat → Feature`, `Fix → Bugfix`, any other `ConventionalType → Chore`, none → Other. The ticket
  is `Tag::Issue`'s raw ticket; `naming.rs` gains `pub fn ticket(dir_name) -> Option<String>`
  (wrapping the private `split_name`) rather than parsing the `#123` label back.
- **Rationale**: the sidebar's tags already come from `parse_tags`, so card and sidebar agree.

- **Alternatives rejected**: parsing the sidebar's `#123` issue label back into a ticket (loses
  Jira keys' case and couples to display text); a second type parser in the client (could drift
  from the sidebar's tags).

## R8 — Issue titles

- **Decision**: the client keeps `overview::State::titles: BTreeMap<u64, TitleLookup>` per project
  (`Pending | Known(String) | Unavailable`). On opening the Overview it asks, once per app run per
  ticket, for numeric tickets of worktrees with no entry, through `micold_core::github::GhCli`'s
  existing `IssueLookup::read_issue` in `spawn_blocking`, only when the project's issue tooling is
  GitHub and available (`shell/capabilities.rs` `issue_tooling`). Issues the New-worktree form
  already holds (`features/worktree_form.rs` `IssueList::held`) seed the map. Jira tickets, an
  unavailable lookup, offline or an error leave the branch name as the summary (FR-015, edge case).
- **Rationale**: no title cache exists in the client today; the spec's assumption allows a lookup
  "the app already makes" and forbids one only when the lookup is off or offline. One cached read
  per ticket is not polling (FR-019). Titles are held in memory only, never logged (FR-020).
- **Alternatives rejected**: the ledger's quoted issue title (only for autopilot worktrees);
  refreshing titles on a timer (polling).

## R9 — Task progress from the worktree's files

- **Decision**: pure core `micold-core/src/task_progress.rs`:
  - `feature_dir(feature_json, spec_dirs, ticket) -> Option<String>`: `.specify/feature.json`'s
    `feature_directory`, else the one `specs/<NNN>-*` whose leading number equals the numeric
    ticket, else none (edge case: several feature folders).
  - `checkbox_ratio(text) -> Option<Ratio>`: lines matching `^\s*[-*] \[( |x|X)\] `; other lines
    ignored; no checkbox → `None` (never 0/0).
  - `ledger_phase(text) -> Option<Phase>` from the line `- **Phase**: <word>`: `spec`, `clarify`,
    `bug` → Spec; `plan` → Plan; `tasks` → Tasks; `milestones`, `bugfix`, `chore` → Implement;
    `close` → PR; `done` → Done; a numeric prefix (`4-milestones`) is stripped; any other word →
    `None` (the template's vocabulary: `.claude/skills/speckit-autopilot/templates/autopilot-ledger.md`).
  - `artifact_phase(spec, plan, tasks, ratio)`: tasks.md with a ticked box → Implement; tasks.md →
    Tasks; plan.md → Plan; spec.md → Spec.
  - `phase(ledger, card_state, pr_open, artifact)`: ledger wins; else Done when the card is Done;
    else PR when the branch has an open pull request; else the artifact phase.
  - `read(root, ticket) -> FileProgress`: the I/O shell (bounded: at most 1 MiB per file read; a
    missing, unreadable or non-UTF-8 file counts as absent).
- **Off the UI thread**: a new `micold-client/src/shell/overview_watch.rs`, one subscription per
  valid worktree while the Overview is open, built on the existing watch loop of
  `shell/changes_watch.rs` (notify watcher, `micold-core/src/review/watch.rs` `Debouncer` 300 ms
  quiet / 1 s max) watching `<root>/specs` and `<root>/.specify` recursively; it reads with
  `spawn_blocking` and emits `overview::Msg::ProgressRead { dir, progress }`. The loop is shared,
  not copied: `changes_watch::run` is generalised over the read it performs (its key and message
  stay as they are). Reads run once on open, then on change: well inside the 5 s of FR-008.
- **Rationale**: no spec-kit reader exists anywhere; the parsers are pure and testable; the watch
  pattern exists. Watching only while open keeps 30 locations from holding 30 watchers all day;
  the badge does not depend on files.
- **Alternatives rejected**: polling the files (blocks or wastes); reading in `view` (blocks the
  window).

## R10 — Hook-fed activity (Story 5)

- **Decision**: pure core `micold-core/src/hook_activity.rs`: `HookActivity { todos: Vec<Todo>,
  prompt: Option<String> }`, `Todo { text, status: Pending | InProgress | Completed }`,
  `HookActivity::apply(&mut self, body: &serde_json::Value) -> bool` (changed):
  `PostToolUse` with `tool_name == "TodoWrite"` replaces `todos` from `tool_input.todos[]`
  (`content`, `status`, `activeForm`; at most 50 items, each text at most 200 chars);
  `UserPromptSubmit` sets `prompt` to its first non-empty line, at most 200 chars. Anything else
  changes nothing. `current(&self) -> Option<Activity>`: the in-progress todo's `activeForm` (else
  its `content`), else the prompt; ratio = completed / total when todos exist.
- Daemon: `micold-daemon/src/hooks.rs` `classify_hook` already parses the body as a `Value`; it
  returns the `Value` with `HookClass::Event` so `handle_connection` calls
  `state.note_hook_activity(session, &body)` beside `note_activity`; `LiveSession` gains
  `hook_activity: HookActivity`, dropped with the live entry (no timer). `overlay_live_summaries`
  copies it to `SessionSummary::hook_activity: Option<HookActivity>` (`serde(default)`;
  `PROTOCOL_VERSION` 48 → 49; `tests/schema_hash.rs` re-pinned). The catalog is broadcast when
  `note_activity` or `note_hook_activity` reports a change.
- Client: `catalog_sync.rs` adopts it at the two places `paused_on_user` is adopted;
  `Session::hook_activity` (runtime only, never persisted).
- **Privacy (FR-020)**: the text is never logged (no `Debug` in a log line; `hooks.rs` keeps
  never logging bodies), never written to the catalog file (`catalog.rs` `session_summary` sets
  `None`), and never returned by the daemon's MCP tools (they build their own views; a test pins
  that `hook_activity` is absent from them). It travels only over the local socket to the window.
- **Rationale**: the daemon already receives these hooks per session (`HooksMap` installs
  `UserPromptSubmit` and `PostToolUse`) and is the only process that sees them.
- **Alternatives rejected**: reading the agent's transcript (heavier, CLI-specific); keeping the
  whole todo history (only the latest list is shown).

## R11 — Card colours, icons and shared components

- **Decision**:
  - Theme roles: `micold-core/src/tokens/mod.rs` `Roles` gains `warning`, `on_warning`,
    `warning_container`, `on_warning_container`, `success`, `on_success`, `success_container`,
    `on_success_container`, fixed palette tones per scheme like feature 482's `diff_added` /
    `diff_removed`, contrast-gated in `ui/material/composition_contrast.rs`. Needs action uses the
    `error` family, Needs attention `warning`, Working `primary`, Done `success`, Idle
    `surface_container_low` with `on_surface_variant` text (≥ 4.5:1, FR-006).
  - Icons (`icons.rs`, pinned in `tests/icons.rs` and present in the bundled font,
    `tests/icons_font.rs`): `Overview`, card states `CardNeedsAction`, `CardNeedsAttention`,
    `CardDone` (Working and Idle reuse 905's `StateRunning`, `StateIdle`), location types
    `TypeFeature`, `TypeBugfix`, `TypeChore`, `TypeOther`, `TypeIncluded`, `TypeOrphan` (project
    root reuses `ProjectRoot`). Codepoints chosen in the implementation and pinned by the tests.
  - Shared components in `ui/material/` (Principle VIII, builder API, showcase entries):
    `OverviewCard` (a pressable, focusable card: `new(model, roles).on_press(msg)`), `CardGrid`
    (responsive columns from the available width via iced `responsive`, min card width 280 px,
    one column at minimum, vertical scroll), `ProgressBar` (determinate; `new(ratio, roles)`),
    `Button::selected(bool)` and `Button::mark(UnreadMark)` (a configured trailing mark, so the
    badge can be tinted and worded), and `UnreadMark::icon(Icon)`.
- **Rationale**: no warning or success role exists (`style.rs` notes the gap; `usage_indicator.rs`
  falls back to `error`); "every colour from the theme's roles" (FR-021) needs them. No grid,
  determinate progress, generic card or toggled button exists (`StageProgress` is indeterminate
  by design).
- **`ProgressBar` beside `StageProgress`**: `ui/material/progress.rs` `StageProgress` is an
  animated indeterminate bar plus a stage line, built for durations that cannot be measured (its
  module doc). A determinate bar has no animation and no stage text, so a `.determinate(ratio)`
  mode would leave most of `StageProgress` unused in that mode; `ProgressBar` is a separate small
  primitive in the same file's neighbourhood, and the two share the track and fill styling through
  one helper in `progress.rs` (no copied styling).
- **Alternatives rejected**: tertiary as "yellow" (scheme-dependent hue, not yellow); hard-coded
  colours (FR-021).

## R12 — Card activation

- **Decision**: core `overview::activation(location facts) -> Activation`:
  `OpenSession(id)` when `main_session` is `Some`; `OfferStart(location)` when the location has no
  un-archived session and is not orphan / invalid; `RevealOnly(location)` otherwise. The client
  closes the Overview, then: `OpenSession` sends the existing `SessionMsg::Selected(id)` (same path
  as the sidebar, which views and focuses the session); `OfferStart` reveals the row and sends the
  existing `SessionMsg::StartMenuOpened { location, unavailable_default: None }` (the same menu and
  CLI choices as the sidebar's start chevron; nothing starts until the user picks); `RevealOnly`
  reveals the row. "Reveal" = the sidebar's new `revealed: Option<SessionLocation>` (scrolled into
  view, drawn with `TreeItem`'s existing selected look) until the next selection.
- **Rationale**: reuses the sidebar's selection and start paths (FR-013); a worktree without
  sessions has no selectable row today, so "select the location" needs the reveal mark.
- **Risk to check in tasks**: the start menu anchors to the row's start button by a measured
  position (`StartMenuAnchored`); when opened from a card the row must be scrolled into view first
  (reveal, then open).

- **Alternatives rejected**: starting a session directly on a card click (FR-013 forbids starting
  without confirmation); selecting nothing for a location without sessions (the user would land on
  an unrelated view with no sign of where the location is).

## R13 — Keyboard traversal

- **Decision**: each `OverviewCard` takes focus like `Button` (`TakesTheKeyboard`); Tab /
  Shift+Tab use the existing `Message::FocusMoved` (`focus_next` / `focus_previous` +
  `scroll_focused_into_view`). While the Overview is open and the terminal unfocused, the
  Overview's listener maps arrow keys to `FocusMoved { forward }` (Right/Down forward, Left/Up
  back). Enter and Space activate the focused card through the card's own press handling. Opening
  the Overview focuses the first card.
- **Rationale**: no new focus system; matches how the rest of the window moves focus.

- **Alternatives rejected**: a new focus system or a roving-focus grid widget (iced's
  `focus_next`/`focus_previous` already traverse focusable widgets in layout order, which is the
  card order); arrow keys handled inside the card grid widget (needs widget-level key capture the
  shared library does not have).

## R14 — Stale data and several windows

- **Decision**: no new notice. The window already shows `connection_banner` between the top bar
  and the body on every branch (`ui/mod.rs`; "content on screen is now stale", `shell/daemon_sync.rs`
  `on_disconnected`), so it shows above the Overview too. Cards are derived from what the window
  still holds: the sessions in its core keep their last state (to confirm in the tasks unit: `on_disconnected` removes `daemon_catalogs[daemon]`; if that drops the sessions, `overview::State` keeps the last card list while disconnected instead); the pull request reading is
  `Released` on disconnect, so counts read unknown and PR-derived states are not claimed (the
  spec's "PR status unreadable" edge case). On reconnect the new catalog and reading replace them.
  A client state test pins that the card list built after `on_disconnected` keeps the
  session-derived states. Each window holds its own `overview::State`; card state is derived from
  the same daemon catalog and pull request reading, so windows agree.
- **Alternatives rejected**: a dedicated stale notice inside the Overview (duplicates the
  window-wide banner, which already sits above every body branch); clearing the cards while
  disconnected (the spec's edge case asks to keep the last state).
