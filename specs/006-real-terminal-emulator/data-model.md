# Phase 1 Data Model: Real Terminal Behavior for Embedded Session Terminals

Only the deltas over feature 005 are listed. "Pure" = render-free core, testable under
`--no-default-features`. "GUI" = compiled only with the `gui` feature.

## Pure core (`src/`)

### `keymap` (NEW, pure) — `src/keymap.rs`

Key-encoding logic, decoupled from iced so it is unit-testable without a GUI. The gui widget
maps iced key events onto these types and applies the result.

- **`KeyInput`** — a normalized key press:
  - `key: Key` — enum: `Char(char)` | `Named(NamedKey)`.
  - `NamedKey` — enum covering the required set (FR-006): `Enter, Backspace, Delete, Tab,
    Escape, Insert, Home, End, PageUp, PageDown, ArrowUp/Down/Left/Right, F1..=F20, Space`.
  - `mods: Mods` — bitflags: `SHIFT, CTRL, ALT, LOGO`.
  - `text: Option<String>` — the printable text iced resolved for the press (used when no
    binding matches).
- **`TermMode`** — the subset of terminal mode that changes encoding: `app_cursor: bool`,
  `alt_screen: bool` (mirrors `alacritty TermMode::APP_CURSOR`/`ALT_SCREEN`).
- **`KeyOutput`** — enum, the decoded action:
  - `Bytes(Vec<u8>)` — write these bytes to the PTY (printable text or a control/escape
    sequence). (FR-006, FR-007, FR-008)
  - `Copy` — copy the current selection to the clipboard. (FR-013)
  - `Paste` — paste clipboard text into the PTY. (FR-013)
  - `ReleaseFocus` — the reserved focus-out chord was pressed. (FR-011)
  - `Ignore` — not handled here; let the app/subscription see it.
- **`fn encode(input: &KeyInput, mode: TermMode) -> KeyOutput`** — the single pure entry point.
  Rules adapted from `iced_term/bindings.rs` (full table in `contracts/key-encoding.md`), with
  the `Ctrl+U → \x15` fix. Platform copy/paste chords selected via `cfg` (macOS `Cmd+C/V`,
  else `Ctrl+Shift+C/V`).

**Validation / invariants**: `encode` is total (always returns a variant); printable input with
no modifier and `text=Some` yields `Bytes(text)`; unknown combinations yield `Ignore`.

### `State` (extended) — `src/app.rs`

- **`terminal_focused: bool`** (NEW) — whether the embedded terminal holds input focus. Default
  `false`. Set true on `TerminalFocused`, false on `TerminalFocusReleased`, and on
  `SessionSelected` / session close / project switch (explicit focus only — FR-010). Drives the
  focus gate (FR-009/FR-011/FR-012).
- **`settings_draft: Option<SettingsDraft>`** (NEW) — present while the Settings overlay is open.
- **`Overlay`** gains **`Settings`** (NEW variant).

- **`SettingsDraft`** (NEW):
  - `scrollback_lines: String` — the editable field value (string for input; parsed/validated on
    submit).
  - `error: Option<String>` — validation message (e.g. out-of-range).

**Pure predicate** — **`fn route_key(terminal_focused: bool, overlay: Overlay, out: KeyOutput)
-> KeyRouting`** (or equivalent): decides whether a key drives the app or the terminal. Used to
prove FR-009 (unfocused ⇒ app) and the focus gate (focused ⇒ terminal, except `ReleaseFocus`).
Unit-tested in `tests/terminal_focus.rs`.

### `Settings` (extended) — `src/settings.rs`

- **`Settings.scrollback_lines: usize`** (NEW) — the configured per-session scrollback limit.
  Default `10_000`. Validated to a sane inclusive range (e.g. `100..=1_000_000`); values outside
  are clamped/rejected with a message (FR-020, FR-021).
- **`StoredSettings.scrollback_lines`** — `#[serde(default = "default_scrollback")]` so existing
  files (without the field) load with the default (backward compatible; Principle IV recovery
  preserved). `SETTINGS_VERSION` → `2` (documentation of the schema change; not required for
  loading).

### `Message` (extended) — `src/app.rs`

New variants (feature 006):
- `TerminalFocused` / `TerminalFocusReleased` — focus acquire/release (click, chord, affordance).
- `TerminalAction(TermAction)` — a decoded terminal action for the displayed session (see GUI).
- `TerminalResized { cols: u16, rows: u16 }` — pane size changed (FR-014/FR-015).
- `SettingsOpened` / `SettingsScrollbackChanged(String)` / `SettingsSaved` / `SettingsCancelled`.

**Removed** (end state): `TerminalInputChanged(String)`, `TerminalLineSubmitted`, and
`State.terminal_input` — the line-buffered input box is deleted (FR-008). *Sequencing*: these are
retained through the Foundational phase and US1 (so US1 ships as an independent MVP with the
existing input box) and removed in US2 (task T020). `TerminalTick` is **retained** (drives PTY-
output draining) and is not removed.

## GUI (`src/ui/`, `src/main.rs`)

### `TermAction` (NEW) — the widget→binary command set

Carried by `Message::TerminalAction`. Applied by the binary to the *displayed, focused* session's
`RuntimeTerminal` (and only written to the PTY when the session is `Running`, FR-012a):
- `Write(Vec<u8>)` — bytes for the PTY (from `keymap::encode`). Gated on `Running`.
- `Scroll(i32)` — scroll the `Term` display (local scrollback) or forward to the process on
  `ALT_SCREEN`. (FR-016 + wheel edge case)
- `SelectStart(SelectionKind, (f32, f32))` / `SelectUpdate((f32, f32))` — text selection into
  `Term.selection`. (FR-013, FR-013b)
- `MouseReport(button, mods, point, pressed)` — forward a mouse event to the PTY when the
  process enabled mouse reporting. (FR-013a)
- `Copy` — copy `Term` selectable content to the clipboard (handled in-widget via iced
  clipboard). (FR-013)
- `Paste(String)` — paste clipboard text to the PTY. Gated on `Running`. (FR-013)

### `RuntimeTerminal` (extended) — `src/ui/terminal.rs`

- Fields `rows`/`cols` become **dynamic** (no `const ROWS/COLS`); add `resize(cols, rows)` that
  resizes **both** the PTY (existing) and the `Term` (`Term::resize(TermSize::new(cols, rows))`).
  (FR-014/FR-015)
- `Term` is created with `Config { scrolling_history: settings.scrollback_lines, ..default() }`.
  (FR-016)
- Add pass-through methods used by `TermAction`: `scroll(delta)`, `start_selection`,
  `update_selection`, `selectable_content() -> String`, `mouse_report(...)`, and a `dirty` flag
  set on `pump()` to drive `canvas::Cache` invalidation. (FR-005a)
- The plain-text `screen_text()` is superseded by the grid renderer (may be kept for tests).

### `TerminalPane` (NEW widget) — `src/ui/material/terminal_pane.rs`

A reusable iced `advanced::Widget` exposed as a **chainable builder** terminating in `.into()`
(Constitution v1.2.0 Principle VIII builder-API rule): `TerminalPane::new(rt, palette)
.focused(bool).into()`. It borrows the active `&RuntimeTerminal` (its `Term` grid) and the
color palette (derived from the active theme, including the focus-indicator accent); `focused`
defaults to false. Lives in the shared `src/ui/material/` component library.
- **Widget state**: `is_focused, is_dragged, last_click, scroll_pixels, keyboard_modifiers,
  size, mouse_position_on_grid` (adapted from iced_term `TerminalViewState`; `is_focused`
  defaults **false**).
- **`draw`**: canvas render of `grid.display_iter()` — per-cell fg/bg color, bold/italic font,
  dim/inverse/underline/strikethrough, cursor, selection highlight, with background-run batching
  and `canvas::Cache`. (FR-001..FR-005)
- **`on_event`**: `if !state.is_focused { return Ignored }`; else map key events via
  `keymap::encode` and mouse/scroll via the geometry helpers into `TermAction`s published as
  `Message::TerminalAction`; auto-emit `TerminalResized` when the layout size changes. Reserved
  release chord → `Message::TerminalFocusReleased`. (FR-006..FR-016)

## Relationships & lifecycle (unchanged from 005, reaffirmed)

- One `RuntimeTerminal` per live `Session`; only the `active_session`'s terminal is displayed
  and can be focused. Background sessions keep running; input never routes to them (FR-012).
- Focus is a single global flag scoped to the displayed terminal; switching sessions clears it
  (re-focus is an explicit action). Lifecycle (Running/Starting/Restarting/Failed), persistence,
  and crash-restart are exactly as in feature 005 (FR-017).
