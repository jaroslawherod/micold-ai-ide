# Research: Bottom Status Bar

**Feature**: [spec.md](spec.md) | **Plan**: [plan.md](plan.md)

Each decision names what was chosen, why, and what was rejected. Paths are relative to
`crates/micold-client/src/` unless they start with `crates/` or `docs/`.

## R1. Where the bar is mounted

- **Decision**: In `ui::view_with`'s window column (`ui/mod.rs`, the `material::Surface` column
  built after `body`), as the last child: `toolbar`, `connection_banner`, `sandbox_banner`, `body`,
  `status_bar::view(..)`. The column already sits outside every body branch, so the bar is drawn in
  the session view, project surface, Overview (incl. full screen), Changes, Compare, Settings and
  the empty screen (FR-002) by construction. The install-location screen returns before the column
  is built, so it never draws the bar (spec edge case).
- **Rationale**: The banners are mounted there for the same reason ("Nothing may nest this inside
  an `if`"); one site covers every branch. Being inside `base`, the bar sits under every overlay
  pushed afterwards (FR-013) and inside `KeyboardElsewhere`, which keeps focus with the terminal.
- **Rejected**: (a) one bar per body branch — six call sites, the exact failure the column comment
  warns about; (b) an overlay surface pinned to the bottom edge — it would draw over the body
  instead of taking its 28 px, and above menus it must stay under.

## R2. Which rows go, which stay

- **Decision**: Remove the bring-up row (`sandbox_status::indicator(attempt_line(..))`) from the
  column. `connection_banner` returns its zero-size `Space` for `Disconnected` as well as
  `Connected`. `sandbox_banner` is unchanged (its three states all carry an action). The four
  action-carrying connection states keep their banners (FR-009, D3).
- **Rationale**: FR-003 / FR-009 / clarification D3: only action-free conditions leave the banner
  slot. `Disconnected` is the only connection banner without `.action(..)`.
- **Rejected**: moving every banner into the bar — US3 rejects hiding conditions that need the
  user; keeping `Disconnected` as a banner — contradicts D3.

## R3. A new shared component `StatusBar` in the material library

- **Decision**: NEW `ui/material/status_bar.rs`: `StatusBar::new(roles)` with
  `.item(StatusItem)` (leading group) and `.hint(StatusItem)` (trailing group), terminated by
  `.into()`. `StatusItem::new(text)` with `.dot(ItemTone)`, `.busy(bool)`, `.tone(ItemTone)`,
  `.tooltip(String)`, `.give_way(GiveWay)`. Registered in the showcase (`showcase/sections/`) and
  so in `tests/showcase_completeness.rs` and `tests/material_builder_api.rs`.
- **Rationale**: Principle VIII requires a shared builder-API component; no existing component
  draws a fixed-height row of dot+text items with a priority-drop rule. It composes existing
  primitives: `Ellipsized` (one-line shortening, `material/ellipsized.rs`), `BusyIndicator`
  (`material/busy_indicator.rs`, busy ring that requests frames only while busy), `Tooltip`.
- **Rejected**: (a) reusing `Toolbar` — 64 dp app-bar anatomy with action buttons, wrong height and
  role; (b) building the row inline in `ui/mod.rs` — a feature-local one-off (Principle VIII);
  (c) `StatusChip` — a Jira-status chip on `Tag`, filled and bounded, not a bare text item.

## R4. Narrow widths: who gives way, in what order

- **Decision**: `StatusBar` implements `Widget::layout` itself. It measures each item's natural
  width, then, while the total exceeds the bar's width: drops items marked `GiveWay::Drop(rank)` in
  rank order (key hints rank 0, plan usage rank 1); then shortens items marked
  `GiveWay::Shorten(rank)` in rank order down to a floor of the dot plus one character and the
  ellipsis (CLI rank 0, sandbox rank 1, connection rank 2 — the connection item last, FR-012). A
  dropped item takes no space and no hit area. The tooltip of a shortened item carries its full text.
- **Rationale**: FR-012 fixes the order; doing it in layout uses the renderer's shaping, which the
  view cannot do (`window_size` alone cannot measure text). `Ellipsized` already binary-searches the
  longest prefix that fits a given width.
- **Rejected**: deciding in the view from `state.window.window_size` with character-count
  heuristics — wrong for proportional fonts and long remote names; `Wrapping::None` — paints past
  the box, which `tests/layout_text_overflow.rs` forbids.

## R5. A render-free model for the bar's words

- **Decision**: NEW `ui/status_bar.rs` with `pub fn model(state: &State, connection:
  &ConnectionStatus, sandbox: &Sandbox, now: i64, offset: UtcOffset) -> StatusModel` returning plain
  data (`StatusModel { connection, sandbox, cli, usage, hints }`, see data-model.md) and
  `pub fn view(model, roles) -> Element` that feeds it to `StatusBar`. The wording lives in
  `model`, unit-tested without a renderer, like `sandbox_status::stage_line`.
- **Rationale**: Principle I: the claims ("Daemon connected · local", "Sandbox ready",
  "Getting image 42%") are the part easy to get wrong; a pure function tests them per state.
  `features/` is not used because the bar owns no state and no messages (a feature without a `Msg`
  would be noise in the feature registry).
- **Rejected**: putting the model in `micold-core` — it reads client-only types
  (`ConnectionStatus`, `features::sandbox::Sandbox`, `State`).

## R6. Connection item text and "where the service runs"

- **Decision**: Text by `ConnectionStatus`: `Connected` → "Daemon connected · {location}" (success
  dot); `Disconnected` → "Daemon disconnected · reconnecting…" (error dot); `Displaced` /
  `ProjectBusy` → "Read-only · another window" (warning dot); `VersionMismatch` / `BuildMismatch` →
  "Daemon connected · {location}" with a warning dot and the mismatch sentence as tooltip.
  `{location}`: the location key is the active session's location from
  `State::current_session_location()` (`features/sidebar.rs`), as the wire names it —
  `SessionLocation::Worktree(dir)` → `dir`, `SessionLocation::Default` → `""` — and `""` (Default)
  whenever it returns `None` (no active session, its record gone, or its worktree unknown). `State::daemon_binding(key)` resolves it; `Binding::Bound(id)` whose
  `state.settings.daemons` entry is `DaemonRuntime::Host` → "local", any other runtime → the entry's
  name; `Binding::NoDaemon` (or a missing entry) → `NO_DAEMON_LABEL` ("no daemon"), the same text
  `daemon_label_of` gives; no project open → "local" (the window's own host service). This is the
  same lookup `daemon_label_of` uses, factored into one shared helper rather than copied.
- **Rationale**: FR-004 and spec Assumption "uses the label the daemon list already shows" —
  `features/daemons.rs::views` shows `"local"` for `DaemonRuntime::Host` and the name otherwise.
- **Rejected**: the daemon list's `endpoint` string (`127.0.0.1:port`, ssh targets) — not a name a
  user recognises in a 12 px bar; showing nothing for no project — FR-004 always names the location.

## R7. Sandbox item text

- **Decision**: `Running` → "Sandbox ready" (neutral); `Disabled` → "Sandbox off" (neutral, no
  banner); `Probing` / `Acquiring` / `Starting` → short stage words "Checking runtime…",
  "Getting image…" (+ " {percent}%" when `Progress::percent` is `Some`), "Starting sandbox…" with a
  busy mark; the tooltip is `sandbox_status::attempt_line(sandbox)` (label + runtime detail + retry
  reason, FR-006). With a fallback taken → "No sandbox" (warning); `Stale` → "Sandbox out of date"
  (warning); `Failed` → "Sandbox failed" (warning); tooltip `Sandbox::persistent_notice()`.
- **Rationale**: FR-005/FR-006 reuse the two existing wording sources, so the bar and banner never
  disagree. `stage_line`/`attempt_line` stay; `sandbox_status::indicator`/`view` and the
  `sandbox_indicator` re-export are deleted with the row (their only callers are `ui/mod.rs` and
  `tests/sandbox_progress.rs`, whose layout test moves to the bar).
- **Rejected**: keeping `StageProgress` (a 4 dp indeterminate bar) inside the 28 px bar — the busy
  ring of `BusyIndicator` already exists for an inline busy mark.

## R8. CLI item: which session, when shown

- **Decision**: Shown only when the session view is the visible body: no Settings draft, an active
  project, `!overview.open`, no `changes.open`, no Compare, and `session.active` is `Some` — the
  same branch order `view_with` uses. Text "CLI: {display_name}" from
  `AiCli::provider().display_name()` (`crates/micold-core/src/provider.rs`) of the session found as
  `ui/terminal.rs::session_provider` finds it. A shared helper
  `fn session_view_visible(state) -> bool` is extracted so the bar and `view_with` agree.
- **Rationale**: `session.active` is the focused pane's session: `shell/panes.rs::sync` keeps the
  selection and the layout's focused terminal in step ("The active project's selection then follows
  the restored focused pane"), so focusing a pane changes `session.active` (US4 s2). `display_name`
  is the provider trait's user-facing name ("Claude Code").
- **Rejected**: `command()` ("claude") — the tab label, not a display name (US4 s1 says
  "Claude Code"); reading `App::pane_layouts` — binary-only, not in `State`.

## R9. Key hints notation

- **Decision**: "{chord} Overview" from `keymap::overview_chord_label()` and "{chord} Jump" from
  `keymap::switcher_chord_labels()` (Ctrl or Cmd form by `cfg!(target_os = "macos")`). macOS reads
  "Cmd+Shift+O Overview", matching the Overview button's tooltip and the user guide's chord tables.
- **Rationale**: One notation across the app; US4 s4's "⌘⇧O" is an example and its requirement is
  "Cmd on macOS, Ctrl elsewhere", which the existing labels satisfy. A glyph form would be a second
  notation used nowhere else.
- **Rejected**: new glyph labels (⌘⇧O) — a second chord vocabulary to keep in step.

## R10. Plan usage relocation

- **Decision**: Move `toolbar::usage_indicator` (and its test) to `ui/status_bar.rs`, unchanged in
  logic: the bar adds the `UsageIndicator` as an item after CLI, `GiveWay::Drop(1)`, only when
  `state.plan_usage.view(now, offset)` is `Some`. `toolbar::view` stops adding it. The clock and
  zone are read only when a reading is held, as today.
- **Rationale**: FR-011 / D4 keep label, warning look, details and show/hide rules; reusing the
  component and the function carries them over verbatim. `StatusBar::item_element(..)` accepts an
  arbitrary element for this one item so `UsageIndicator` is reused, not re-drawn.
- **Rejected**: a text-only usage item — loses the hover details and warning look.

## R11. Height, colours, platform

- **Decision**: Height a NEW anatomy figure `pub mod status_bar { pub const HEIGHT: f32 = 28.0; }`
  in `crates/micold-core/src/tokens/anatomy.rs`, registered in `anatomy::ALL` (47 → 48 entries) and
  checked in `crates/micold-core/tests/tokens_anatomy.rs`. It lives in anatomy rather than
  `density` for the same reason `app_bar::HEIGHT` and `snackbar` do: the bar is fixed by the design
  (the mockup's 28 px, FR-001) and does not scale with the density axis. `material/status_bar.rs`
  references it, which is the binding `crates/micold-client/tests/anatomy_call_sites.rs` requires.
  Surface `roles.surface_container`
  with a top `outline_variant` divider; text `TypeRole::Label` small in `on_surface_variant`; dot
  colours `success` / `warning` / `error`; warning words in `warning`. The bar uses the shipped
  Roboto (fixed height on every platform) and draws flush to the window edge (the window surface
  already clips macOS corners).
- **Rationale**: Spec Assumption (no new colour roles); `tests/composition_contrast.rs`-style
  contrast checks cover warning text on `surface_container` in both schemes.
- **Rejected**: platform-specific heights — Principle VI.

## R12. Overlap with parallel redesign threads

- **Decision**: Keep edits to shared files to the minimum: in `ui/mod.rs` only the window column
  (one line added, one removed) and `connection_banner`'s `Disconnected` arm; in `ui/toolbar.rs`
  only the usage block and its import/test. Covered states get a `.sandbox(..)` builder on
  `StateUnderTest` and new states appended at the end of `covered_states()`.
- **Rationale**: #1125 (sidebar) and #1126 (worktree header/tabs) also edit `ui/mod.rs`'s body
  branch, `tests/support/covered_states.rs` and regenerate `tests/fixtures/layout_snapshot.txt`;
  #1127 rebuilds `ui/toolbar.rs`. The fixture will conflict on every merge whichever lands second:
  the second PR regenerates it after rebasing (`UPDATE_LAYOUT_SNAPSHOT=1`), never hand-merges it.
