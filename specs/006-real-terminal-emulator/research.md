# Phase 0 Research: Real Terminal Behavior for Embedded Session Terminals

All Technical-Context unknowns are resolved below. Evidence was gathered by reading the
cached crate sources: `iced_term 0.6.0` (`backend.rs`, `view.rs`, `bindings.rs`, `theme.rs`,
`settings.rs`, `terminal.rs`) and `alacritty_terminal 0.25.1` (`term/mod.rs`, `grid/mod.rs`),
plus the current implementation (`src/ui/terminal.rs`, `src/main.rs`, `src/app.rs`).

## R1 — Renderer & input engine: adapt iced_term's view, not its backend

**Decision**: Build a custom iced `advanced::Widget` (`TerminalPane`) that renders feature
005's existing `alacritty_terminal::Term` grid (colors + styles on a `canvas`) and, when
focused, translates key/mouse events into actions applied to that session's PTY/Term. Adapt
`iced_term 0.6.0`'s MIT-licensed `view.rs` (canvas draw, focus-gated `on_event`, mouse/scroll),
`bindings.rs` (key→escape-sequence table), and `theme.rs` (`ansi::Color` → color) as the
blueprint. **Do not** use `iced_term::Terminal`/`Backend`. **Remove** the `iced_term`
dependency (unused once we render ourselves).

**Rationale**:
- `iced_term`'s `BackendSettings` exposes only `{ program, args }` and constructs the PTY via
  `tty::Options { shell: Some(Shell::new(program, args)), ..Default::default() }` — **no cwd
  and no env**. Feature 005 requires `claude` to run with cwd = the session's worktree
  (FR-013) and sets env; iced_term's backend cannot express this without a fork.
- `iced_term`'s `Backend` *owns* process spawning (`tty::new` + alacritty `EventLoop`), which
  would displace 005's `RuntimeTerminal` (`portable-pty`), `SessionRouter`, `--session-id` /
  `--resume`, crash-restart, and persistence. Adopting it means re-implementing all of that on
  a pre-1.0, single-maintainer crate — and still not solving cwd.
- Its *view* layer, by contrast, is exactly what we need and is decoupled from process
  management: `view.rs` already renders the alacritty grid with fg/bg colors, bold/italic,
  dim (alpha), inverse (swap), underline, cursor, and selection highlight (with background-run
  batching for perf), and its `on_event` implements precisely the focus gate we want
  (`if !state.is_focused { return Status::Ignored }`), key encoding, mouse-report-vs-selection,
  wheel scroll, and clipboard copy/paste.
- We already depend on `alacritty_terminal 0.25` and `portable-pty 0.9` and use them directly;
  the grid we render already carries every cell attribute (005's `screen_text()` merely
  discards them). So the change is *view-only*, preserving all hard-won 005 lifecycle logic and
  the pure `TerminalBackend` seam (Principle I).

**Alternatives considered**:
- *Fork/vendor iced_term to add cwd/env*: heavier to maintain than our own view widget, still
  forces reconciling 005's lifecycle with iced_term's process model. Rejected.
- *Keep the plain `text()` renderer and only add spans/keys*: iced `text` cannot express
  per-cell backgrounds, cursor, or selection; a `canvas` is required for a faithful terminal.
  Rejected.

**Adaptation fixes to apply (bugs/inapplicable defaults in iced_term to NOT copy verbatim)**:
- `bindings.rs` maps `Ctrl+U` to `\x51` ('Q'); the correct control byte is `\x15`. Fix in
  `keymap`.
- `view.rs` `TerminalViewState::is_focused` defaults to `true`; our focus MUST default to
  **false** (focus only via explicit action, FR-010).

## R2 — Focus model & moving focus in/out

**Decision**: Track focus as a core-`State` boolean `terminal_focused` (pure, testable).
Acquire focus by clicking the terminal pane (explicit action, FR-010), shown by a visible
focus ring on the pane. Release focus by (a) clicking outside the pane, (b) a reserved chord
**Ctrl+Shift+E** (macOS **Cmd+Shift+E**), or (c) an on-screen "release focus" affordance in the
pane header. The reserved chord is intercepted by the widget and never written to the PTY.

Key routing (the "propagate only when focused" requirement):
- **Focused**: `TerminalPane::on_event` handles key/mouse events and returns `Captured`; the
  app's global keyboard `Subscription` is suppressed while `terminal_focused` (checked in
  `ui::subscription`), so app shortcuts never fire and Esc reaches `claude` rather than closing
  overlays. The reserved release chord is matched first and emits `TerminalFocusReleased`.
- **Unfocused**: `on_event` returns `Ignored`; the app's subscription/shortcuts handle keys and
  nothing reaches any PTY (FR-009).

**Rationale**: Escape must reach `claude` (its UI uses it), so it cannot be the escape hatch;
a reserved chord guarantees a keyboard-only user is never trapped (clarification Q1). Putting
focus in core `State` makes the app-vs-terminal routing a pure predicate (`route_key`) unit-
testable without a GUI. Centralizing the chord as a constant leaves room to make it
configurable later (mirrors 005's centralized naming formats), consistent with the spec noting
the exact key is a planning decision.

**Alternatives considered**: iced's `text_input::focus`/`operation::Focusable` (used by
iced_term) — works, but keeping the boolean in core `State` is simpler and directly testable,
and we do not need text-input semantics. We still expose the pane as focusable for click focus.

## R3 — Color & style mapping honoring the app theme

**Decision**: Map `alacritty ansi::Color` → iced `Color` following iced_term's `theme.rs`:
`Spec(rgb)` → truecolor; `Indexed(0..=15)`/`Named(...)` → a 16+bright+dim palette; `Indexed(16..)`
→ the standard 6×6×6 + grayscale 256-color cube. The palette's **default foreground/background**
are taken from the app's active light/dark design tokens (`tokens::roles(scheme)`), so the
terminal's default colors follow the app theme and update on theme change (FR-003); the 16 ANSI
colors use a fixed, theme-independent terminal palette (a standard scheme), as is conventional.
Styles come from `cell::Flags`: `BOLD`→bold font, `ITALIC`→italic, `DIM`→fg alpha×0.7,
`INVERSE`→swap fg/bg, `UNDERLINE`→drawn underline; `HIDDEN`→fg=bg; strikethrough drawn like the
underline stroke.

**Rationale**: Directly reuses a proven mapping and satisfies FR-001/FR-002/FR-003. Truecolor
is representable; when a backend approximates, nearest-color is acceptable (spec assumption).

**Alternatives considered**: Deriving all 16 ANSI colors from Material tokens — rejected;
programs assume conventional ANSI colors, and remapping them harms fidelity (SC-002). Only the
*defaults* follow the theme.

## R4 — Streaming & redraw coalescing (responsiveness)

**Decision**: Keep 005's model — a per-session reader thread appends PTY bytes to a shared
buffer; the UI drains it into the `Term` via the parser. Add a per-session `canvas::Cache` in
`TerminalPane`, cleared **only** when new bytes were applied (a dirty flag), so repaint
coalesces to ≤1/frame regardless of output rate (FR-005a). The existing `every(TERMINAL_POLL)`
tick drives draining; the cache prevents needless redraws and bounds cost to the visible grid.

**Rationale**: Meets SC-008 (≤~100 ms perceived latency, memory bounded by scrollback) with the
least disruption. iced_term's own draw uses `canvas::Cache` and background-run batching, which
we adopt. An event-driven `Subscription::run_with_id` channel is a possible future refinement
but is not required to meet the target.

**Alternatives considered**: Repaint on every tick unconditionally — wastes frames under idle
and under flood. Rejected in favor of dirty-flagged cache.

## R5 — Sizing & resize

**Decision**: Replace the fixed `ROWS=30/COLS=100` constants. `TerminalPane` computes
`cols = floor(layout_width / cell_width)` and `rows = floor(layout_height / cell_height)` from
the measured monospace cell (as iced_term's `backend.rs::resize` does) and, when the size
changes, emits `TermAction::Resize { cols, rows }`. The binary resizes **both** the PTY
(`RuntimeTerminal::resize`, already present) and the `Term` (`Term::resize(TermSize::new(cols,
rows))`), so `claude` lays out to the visible area and reflows on window/pane resize (FR-014,
FR-015).

**Rationale**: `alacritty_terminal 0.25.1` supports `Term::resize`; the PTY resize already
exists. Straightforward and cross-platform.

## R6 — Scrollback (configurable) & Settings

**Decision**: Scrollback is `alacritty_terminal::term::Config.scrolling_history` (default
**10 000** in 0.25.1 — matches the spec default). Each session's `Term` is created with
`Config { scrolling_history: settings.scrollback_lines, ..default() }`. Wheel/PageUp-Down map to
`Scroll::Delta`/`Scroll::PageUp`/`PageDown`, forwarding to the process instead when
`ALT_SCREEN|ALTERNATE_SCROLL` (iced_term's `backend.rs::scroll`), so the mouse-wheel edge case
resolves correctly. Persist the limit by adding `scrollback_lines: usize` to `Settings` and
`StoredSettings` (with `#[serde(default = ...)]`, default 10 000), reusing `JsonFileSettingsStore`;
bump `SETTINGS_VERSION` to 2 (documentation only — missing field still defaults). Apply the
configured value to sessions spawned after the change (FR-020 minimum); runtime application to
existing terminals via `Grid::update_history` is possible but out of the required scope.

**Rationale**: Zero new dependency; the knob already exists. Serde default keeps old settings
files loading unchanged (Principle IV recovery guarantee preserved).

## R7 — Settings form & toolbar entry (UI reuse)

**Decision**: Add `Overlay::Settings` + a `SettingsDraft` and an `Overlay::Settings` arm in
`ui::view` and `on_escape`, reusing the modal/form pattern of `rename.rs`/`worktree_form.rs`.
Add a `Settings` item to `toolbar::overflow_items` via the shared `MenuItem` with a new
`Icon::Settings` (Material Symbols "settings" glyph `\u{e8b8}`). The form edits the scrollback
limit (validated to a sane range) and, on save, the binary persists via the settings store.

**Rationale**: Constitution VIII — reuse existing menu/form/modal primitives rather than a
bespoke settings surface. Matches the user's request (Settings item in the toolbar dropdown
opening a Settings form).

**Alternatives considered**: A separate settings window — heavier and inconsistent with the
existing overlay-based dialogs. Rejected.

## R8 — Keystrokes to a non-Running session

**Decision**: The binary applies `TermAction::Write` bytes only when the displayed, focused
session is `SessionLifecycle::Running`; otherwise it drops them (FR-012a). Focus, scrolling,
selection, and copy remain available in non-Running states, and the pane header shows the 005
status label (starting…/restarting…/failed). No input buffering.

**Rationale**: Buffering risks replaying stale keystrokes into a freshly `--resume`d process
(clarification Q). Gating at the single write site (the binary, which owns lifecycle) keeps it
authoritative and testable via the focus-routing predicate.
