---
description: "Task list for Real Terminal Behavior for Embedded Session Terminals"
---

# Tasks: Real Terminal Behavior for Embedded Session Terminals

**Input**: Design documents from `specs/006-real-terminal-emulator/`

**Prerequisites**: plan.md, spec.md, research.md, data-model.md, contracts/, quickstart.md

**Tests**: MANDATORY (Constitution Principle I). Every user story writes failing tests BEFORE
implementation (Red-Green-Refactor). Pure logic (`keymap`, focus routing, settings) is tested
under `cargo test --no-default-features`; gui widget behavior under `cargo test --features gui`.

**Documentation**: MANDATORY per story (Constitution Principle VII) — each user-facing story ships
its docs in `docs/user-guide/` in the same change.

**Cross-platform**: Linux, macOS, Windows (Constitution Principle VI). Platform specifics
(copy/paste chords, PTY) stay behind `cfg`/`portable-pty`; core logic is platform-agnostic.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependency on an incomplete task)
- **[Story]**: US1 / US2 / US3 / US4 / US5 (setup, foundational, polish have no story label)

## Path Conventions

Single Rust project: render-free core in `src/*.rs` + `tests/*.rs`; gui-gated layer in `src/ui/`.

---

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: Dependency hygiene, module/asset wiring, toolchain.

- [X] T001 In `Cargo.toml`, remove the now-unused `iced_term = "=0.6.0"` dependency (this feature renders the terminal directly); confirm `alacritty_terminal = "0.25"` and `portable-pty = "0.9"` remain (used directly by `RuntimeTerminal`) and iced keeps features `canvas`, `advanced`, `lazy`, `tokio`. Add a comment noting the terminal view/key-encoding is adapted from iced_term (MIT); `rust-version` unchanged (no crate added).
- [X] T002 [P] Declare the new pure module in `src/lib.rs`: `pub mod keymap;` (empty stub compiling under `--no-default-features`).
- [X] T003 [P] Add gui module stubs and wire them in `src/ui/mod.rs`: `mod settings_form;` and create `src/ui/components/terminal_pane.rs` (empty widget stub) exported from `src/ui/components/mod.rs`.
- [X] T004 [P] Add `Icon::Settings` to `src/icons.rs` (Material Symbols "settings" glyph `\u{e8b8}`) and include it in the icon-coverage list/test (`tests/icons_font.rs`).

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: State/Message scaffolding, the runtime-terminal size/scrollback refactor, and the
`TerminalPane` skeleton that every story builds on. The existing line-input box is preserved here
and removed later in US2, so US1 remains an independently shippable increment.

**⚠️ CRITICAL**: No user story work can begin until this phase is complete.

- [X] T005 [P] Write failing tests for the extended app base state in `tests/app_state.rs`: `State.terminal_focused` defaults `false`; new `Message` variants (`TerminalFocused`, `TerminalFocusReleased`, `TerminalAction(..)`, `TerminalResized{..}`, `SettingsOpened`, `SettingsScrollbackChanged`, `SettingsSaved`, `SettingsCancelled`) and `Overlay::Settings` wire as pure no-ops; `terminal_input` still present (removed in US2, T020).
- [X] T006 Extend `src/app.rs` to make T005 pass: add `State.terminal_focused: bool`, `State.settings_draft: Option<SettingsDraft>`, the `SettingsDraft` struct, `Overlay::Settings`, and the new `Message` variants with no-op reducer branches. Keep `terminal_input`/`TerminalLineSubmitted` for now (removed in US2 — data-model.md).
- [X] T007 [P] Refactor `RuntimeTerminal` in `src/ui/terminal.rs`: replace the fixed `ROWS=30/COLS=100` with dynamic `rows`/`cols`; add `resize(cols, rows)` that resizes BOTH the PTY (existing) and the `Term` (`Term::resize(TermSize::new(cols, rows))`); create the `Term` with `Config { scrolling_history, ..default() }` (param, default 10_000); add a `dirty` flag set in `pump()` (research R4/R5/R6).
- [X] T008 Thread the active runtime into the view: change `ui::terminal::pane` and `ui::view` to accept `Option<&RuntimeTerminal>` (instead of `Option<&str>`), and update `src/main.rs`'s `view` call to pass the active session's `&RuntimeTerminal`; define the `TermAction` enum (`Write`, `Scroll`, `Resize`, `SelectStart`, `SelectUpdate`, `MouseReport`, `Copy`, `Paste`) in `src/ui/terminal.rs` and a no-op `apply` seam in `src/main.rs` (data-model.md).
- [X] T009 [P] Create the `TerminalPane` `advanced::Widget` skeleton in `src/ui/components/terminal_pane.rs`: widget struct borrowing `&RuntimeTerminal` + palette + font + `focused: bool`, internal `TerminalViewState` (with `is_focused` defaulting **false**), and empty `draw`/`on_event` returning `Ignored`. Export it from `src/ui/components/mod.rs` (contracts/terminal-render-input.md).

**Checkpoint**: State/messages, dynamic-size runtime, view plumbing, and the pane skeleton compile on all three platforms.

---

## Phase 3: User Story 1 - See colored, faithful terminal output (Priority: P1) 🎯 MVP

**Goal**: Render the session's `Term` grid with full ANSI color + text styling and a visible
cursor; full-screen TUIs redraw cleanly; default colors follow the app light/dark theme.

**Independent Test**: Start a session; run colored/styled output (colored diff, `claude` TUI);
confirm colors (16/bright/256/truecolor) and styles (bold/dim/italic/underline/reverse) match a
standalone terminal, defaults follow the theme, and full-screen redraws leave no artifacts. (The
existing line-input box still sends commands, so the pane is usable as an MVP.)

### Tests for User Story 1 (MANDATORY — Constitution Principle I) ⚠️

> Write these FIRST and ensure they FAIL before implementation.

- [X] T010 [P] [US1] Write failing gui tests for the color/style mapping in `tests/terminal_palette.rs` (run with `--features gui`): `ansi::Color::Spec` → truecolor; `Indexed(0..=15)`/`Named` → 16+bright+dim palette; `Indexed(16..)` → 256-cube; and default fg/bg derive from a given light/dark theme (contracts/terminal-render-input.md, research R3).

### Implementation for User Story 1

- [X] T011 [P] [US1] Implement the `TermPalette` (`ansi::Color -> iced::Color`) in `src/ui/terminal.rs`, deriving default foreground/background from `tokens::roles(scheme)` and using a fixed conventional ANSI palette for the 16 colors; make T010 pass (FR-001, FR-003).
- [X] T012 [US1] Implement `TerminalPane::draw` in `src/ui/components/terminal_pane.rs`: canvas render over `grid.display_iter()` with per-cell fg/bg, bold/italic font, dim (alpha), inverse (swap), underline + strikethrough strokes, cursor cell, background-run batching, and a `canvas::Cache` invalidated only when `RuntimeTerminal.dirty` (FR-001, FR-002, FR-004, FR-005, FR-005a). *(Pixel rendering has no practical unit test — validated by the gui palette test T010 + quickstart T042; this deviation is recorded per Constitution I.)*
- [X] T013 [US1] Rebuild `ui::terminal::pane` to render the colored `TerminalPane` (from the active `&RuntimeTerminal` + `TermPalette::from(scheme)`) as the terminal body, replacing the plain `scrollable(text(...))`; keep the existing input box for now; ensure a theme change re-derives the palette (FR-003).
- [X] T014 [US1] Document colored/faithful terminal output (colors, styles, theme-following defaults, full-screen redraw) in `docs/user-guide/worktrees-and-sessions.md` (Principle VII).

**Checkpoint**: Terminal output renders with full color/style and follows the theme; MVP demoable.

---

## Phase 3b: Builder-API Component Migration (Constitution v1.2.0) — run before US2

**Purpose**: Constitution Principle VIII now mandates that shared UI components expose a
chainable builder terminating in `.into()` (iced widget idiom), not free functions with many
positional parameters. Migrate the existing `src/ui/material/` components (and feature 006's
`terminal_pane`) so US2–US5 build on and add compliant builders. Behavior-preserving; the safety
net is a green `cargo test --no-default-features` + `cargo build --features gui` after each.

- [X] T043 Convert `icon_button` → `IconButton` builder in `src/ui/material/icon_button.rs` (`IconButton::new(icon, on_press)` + `.surface(..)`/`.disabled(..)`/`.size(..)`/`.tooltip(..)` + `impl From<IconButton> for Element`); update all call sites; verify green.
- [X] T044 Convert `with_tooltip` → `Tooltip` builder in `src/ui/material/mod.rs`; update call sites; verify green.
- [X] T045 Convert `toolbar` → `Toolbar` builder in `src/ui/material/toolbar.rs`; update `src/ui/toolbar.rs`; verify green.
- [X] T046 Convert `menu_trigger`/`menu_overlay`/`MenuItem` → builders in `src/ui/material/menu.rs`; update `src/ui/toolbar.rs` + `src/ui/mod.rs`; verify green.
- [X] T047 Convert `tree_view`/`TreeItem` → `TreeView` builder in `src/ui/material/tree_view.rs`; update `src/ui/sidebar.rs`; verify green.
- [X] T048 Convert `terminal_pane` → `TerminalPane` builder in `src/ui/material/terminal_pane.rs` (`TerminalPane::new(rt).focused(bool).roles(r).into()`); update `src/ui/terminal.rs::pane`; verify green.
- [X] T049 Reconcile feature-006 artifacts (data-model.md/plan.md/tasks.md) to the builder convention (Settings form + `TerminalPane` described as builders); confirm both build configs green.

**Checkpoint**: All shared components are chainable builders; new US2–US5 components MUST follow.

---

## Phase 4: User Story 2 - Work interactively with the claude CLI (Priority: P1, scheduled after US1)

**Goal**: Deliver keystrokes live to the focused session's PTY, encoded like a real terminal
(printable, named keys, control chords), plus mouse selection, mouse reporting, and copy/paste.
Remove the line-buffered input box.

**Independent Test**: Focus the terminal; navigate a `claude` menu with arrows, autocomplete with
Tab, edit a multi-line prompt, interrupt with Ctrl+C; select+copy text and paste — all as in a
standalone terminal, character-by-character (no line buffering).

### Tests for User Story 2 (MANDATORY — Constitution Principle I) ⚠️

> Write these FIRST and ensure they FAIL before implementation.

- [X] T015 [P] [US2] Write failing pure tests for `keymap::encode` in `tests/keymap.rs` (`--no-default-features`): named-key base encodings; arrows/Home/End in and out of `app_cursor`; Ctrl+`a`..`z` full range incl. the **Ctrl+U == `\x15`** regression; reserved focus-out chord → `ReleaseFocus`; copy/paste chords → `Copy`/`Paste` (per-`cfg`); printable `text` → `Bytes`; unmapped → `Ignore`; totality (contracts/key-encoding.md).
- [X] T016 [P] [US2] Write failing gui tests for `TerminalPane` mouse handling in `tests/terminal_mouse.rs` (`--features gui`), adapting iced_term's `view.rs` suite: left press/drag → `SelectStart`/`SelectUpdate` (single/double/triple = simple/semantic/lines); `MouseReport` when `TermMode::MOUSE_MODE`; Shift forces selection while mouse-mode; and the focus gate (`is_focused == false` ⇒ event `Ignored`) (contracts/terminal-render-input.md, FR-013, FR-013a, FR-013b, FR-009).

### Implementation for User Story 2

- [X] T017 [US2] Implement `src/keymap.rs` (`Key`, `NamedKey`, `Mods`, `TermMode`, `KeyOutput`, `encode`) to pass T015; select copy/paste/release chords via `cfg(target_os)`; attribute the adapted iced_term `bindings.rs` table (MIT) (FR-006, FR-007, FR-008, FR-011, FR-013).
- [X] T018 [US2] Implement `TerminalPane::on_event` keyboard path in `src/ui/components/terminal_pane.rs`: `if !is_focused { Ignored }`; build `KeyInput` from the iced event, call `keymap::encode`, and publish `Message::TerminalFocusReleased` / clipboard `Copy` / `TerminalAction(Paste)` / `TerminalAction(Write)`; a click on the pane publishes `Message::TerminalFocused` (FR-006, FR-008, FR-010).
- [X] T019 [US2] Implement `TerminalPane::on_event` mouse path in `src/ui/components/terminal_pane.rs` to pass T016: left press/drag → `SelectStart`/`SelectUpdate` (single/double/triple = simple/semantic/lines); `MouseReport` when `TermMode::MOUSE_MODE` (SGR/normal encoding); Shift forces selection while mouse-mode; middle-click/auto-copy-on-select supported; right-click context menu for copy/paste (FR-013, FR-013a, FR-013b).
- [X] T020 [US2] In `src/main.rs`, apply `TermAction::{Write,Paste,SelectStart,SelectUpdate,MouseReport,Copy}` to the displayed session's `RuntimeTerminal` (write bytes to the PTY, mutate `Term.selection`, read `selectable_content()` for copy); **remove** the `text_input` box, `State.terminal_input`, `Message::TerminalInputChanged`, and `Message::TerminalLineSubmitted` (FR-008, FR-014).
- [X] T021 [US2] Document interactive `claude` usage (typing, arrows/Tab/Ctrl+C, multi-line prompts, copy/paste gestures, mouse selection) in `docs/user-guide/worktrees-and-sessions.md` (Principle VII).

**Checkpoint**: The interactive `claude` TUI is fully operable from the embedded terminal.

---

## Phase 5: User Story 3 - Keys reach the terminal only when focused (Priority: P2)

**Goal**: Enforce the focus gate — app shortcuts work only when unfocused; keys reach the process
only when focused; a reliable way out of focus that never traps the user; discard input to a
non-Running process.

**Independent Test**: Unfocused → app shortcuts work, nothing reaches the process; click to focus
(visible indicator) → keys reach the process; press the reserved chord / click outside / use the
header affordance → focus returns to the app, session keeps running; Esc while focused reaches
`claude`, not the app.

### Tests for User Story 3 (MANDATORY — Constitution Principle I) ⚠️

> Write these FIRST and ensure they FAIL before implementation.

- [X] T022 [P] [US3] Write failing pure tests for the focus-routing predicate `route_key(focused, overlay, KeyOutput)` and lifecycle write-gating in `tests/terminal_focus.rs` (`--no-default-features`): unfocused ⇒ App for all keys; focused ⇒ TerminalWrite/Copy/Paste/ReleaseFocus; `ReleaseFocus` never yields PTY bytes; `Write`/`Paste` dropped unless session is `Running` (contracts/focus-model.md, FR-009, FR-011, FR-012a).

### Implementation for User Story 3

- [X] T023 [US3] Implement `route_key` in `src/app.rs` (or `src/keymap.rs`) to pass T022.
- [X] T024 [US3] Gate app keyboard handling in `src/ui/mod.rs::subscription`: when `state.terminal_focused`, return `Subscription::none()` for key handling so app shortcuts/Esc are not consumed while the terminal owns the keyboard; otherwise keep the existing overlay Esc behavior (FR-009).
- [X] T025 [US3] Implement focus release in `src/app.rs`/`src/ui`: handle `Message::TerminalFocusReleased` (reserved chord from `keymap`, click-outside via a surrounding `mouse_area`, and a header "release focus" affordance) → set `terminal_focused = false`; render a visible focus indicator/ring in `TerminalPane::draw`; ~~`SessionSelected`~~/close/project-switch clear focus (FR-010, FR-011). *(Bugfix BUG-001: the `SessionSelected` clause is superseded — selecting a session now auto-focuses its terminal, see T050. Session close and project switch still clear focus.)*
- [X] T026 [US3] Enforce write-gating + isolation in `src/main.rs`: apply `TermAction::Write`/`Paste` only when the displayed session is `SessionLifecycle::Running` (drop otherwise, no buffering) and only to the displayed session's runtime; show the session status label in the pane header for non-Running states (FR-012, FR-012a).
- [X] T027 [US3] Document focus behavior (how to focus, the reserved release chord, click-outside, that shortcuts propagate only when focused, non-Running input is discarded) in `docs/user-guide/worktrees-and-sessions.md` (Principle VII).

**Checkpoint**: Focus gating is correct and safe; app and terminal coexist without key leakage.

---

## Phase 6: User Story 4 - Correct sizing, resize, and scrollback (Priority: P3)

**Goal**: Report the visible size to the process so `claude` fits; reflow on window/pane resize;
scroll back through history via wheel/PageUp.

**Independent Test**: `claude` fits the pane; resize the window/pane and the UI reflows within a
redraw; produce more output than fits and scroll back up to the scrollback limit.

### Tests for User Story 4 (MANDATORY — Constitution Principle I) ⚠️

> Write these FIRST and ensure they FAIL before implementation.

- [X] T028 [P] [US4] Write failing gui tests in `tests/terminal_resize_scroll.rs` (`--features gui`): layout+cell-metrics → (cols, rows) computation; wheel delta → `Scroll(n)`; alt-screen scroll forwarded to the PTY vs local scrollback (adapting iced_term `view.rs`/`backend.rs` behavior) (contracts/terminal-render-input.md, research R5). *(BUG-002: the "wheel delta" coverage here was whole-line only; pixel-precision deltas are covered by T053 and widened by T055.)*

### Implementation for User Story 4

- [X] T029 [US4] In `TerminalPane::on_event`, emit `Message::TerminalResized { cols, rows }` when the layout size changes; in `src/main.rs`, apply it via `RuntimeTerminal::resize(cols, rows)` (resizing PTY + `Term`) so the process reflows (FR-014, FR-015).
- [X] T030 [US4] Wire scrolling: `TerminalPane` wheel/PageUp-Down → `TermAction::Scroll`; `src/main.rs` applies `Term::scroll_display` (local scrollback) or forwards to the PTY when `ALT_SCREEN|ALTERNATE_SCROLL`; confirm the per-session `Term` was created with the scrollback history from settings (FR-016, wheel edge case).
- [X] T031 [US4] Document sizing/resize/scrollback behavior in `docs/user-guide/worktrees-and-sessions.md` (Principle VII).

**Checkpoint**: The terminal sizes correctly, reflows on resize, and scrolls back.

---

## Phase 7: User Story 5 - Configure the terminal via Settings (Priority: P3)

**Goal**: A Settings form opened from the toolbar overflow menu lets the user set the scrollback
limit; the value persists across restarts and applies to new sessions.

**Independent Test**: Toolbar menu → Settings → change scrollback → save → a new session honors it;
restart → value retained; out-of-range input → validation message, not saved.

### Tests for User Story 5 (MANDATORY — Constitution Principle I) ⚠️

> Write these FIRST and ensure they FAIL before implementation.

- [X] T032 [P] [US5] Write failing pure tests in `tests/settings_scrollback.rs` (`--no-default-features`): serialize↔deserialize preserves `scrollback_lines`; a document without the field loads the `10_000` default; out-of-range values are clamped/rejected with a message; a corrupt file still yields `Settings::default()` (contracts/settings-schema.md, FR-020, FR-021).

### Implementation for User Story 5

- [X] T033 [US5] Extend `src/settings.rs`: add `Settings.scrollback_lines: usize` and `StoredSettings.scrollback_lines` with `#[serde(default = "default_scrollback")]` (10_000), a validate/clamp helper, and bump `SETTINGS_VERSION` to `2`; make T032 pass (FR-021).
- [X] T034 [US5] Implement the Settings overlay: reducer branches for `SettingsOpened`/`SettingsScrollbackChanged`/`SettingsSaved`/`SettingsCancelled` and `Overlay::Settings` in `src/app.rs` (+ `on_escape`), and the modal in `src/ui/settings_form.rs` (reusing the shared modal/form pattern) editing the scrollback field with inline validation (FR-019, FR-020, VIII).
- [X] T035 [US5] Add the `Settings` item to `src/ui/toolbar.rs::overflow_items` (via shared `MenuItem` + `Icon::Settings`); add the `Overlay::Settings` arm in `src/ui/mod.rs::view`; in `src/main.rs`, persist on `SettingsSaved` via `SettingsStore::save` and pass `settings.scrollback_lines` into new sessions' `Term` config (FR-019, FR-020, FR-021).
- [X] T036 [US5] Create `docs/user-guide/settings.md` documenting the Settings form and the scrollback limit; link it from the user-guide index (Principle VII).

**Checkpoint**: Scrollback is user-configurable, persisted, and applied.

---

## Phase 8: Polish & Cross-Cutting Concerns

**Purpose**: Cleanup and cross-cutting verification. (Per-feature docs shipped within their story.)

- [X] T037 [P] Remove dead code left by the input rework (e.g. unused `screen_text`) and ensure the MIT attribution comment for the adapted iced_term view/key-encoding is present in `src/ui/components/terminal_pane.rs` and `src/keymap.rs` (Principle V). **Keep `TerminalTick`** — it drives PTY-output draining (data-model.md).
- [X] T038 [P] Cross-cutting documentation review and user-guide index/navigation updates in `docs/`.
- [X] T039 Confirm `iced_term` is gone from `Cargo.lock` (`cargo update -p iced_term --dry-run` / a clean `cargo build --features gui`) and no reference remains (Principle V).
- [X] T040 Regression checkpoint for feature 005 (FR-017): confirm the existing session lifecycle, isolation, persistence, and crash-restart tests (e.g. `tests/session_lifecycle.rs`, `tests/session_store.rs`) still pass unchanged, proving this feature altered only rendering/input.
- [ ] T041 **(DEFERRED)** Verify build + full test suite pass on Linux, macOS, and Windows in CI (Principle VI), both `--no-default-features` and `--features gui` (SC-006). Linux verified locally; macOS + Windows postponed to CI.
- [ ] T042 **(DEFERRED)** Run `specs/006-real-terminal-emulator/quickstart.md` end-to-end (all 9 manual steps + SC checks, incl. SC-008 responsiveness under flood). Postponed — needs a manual GUI run on a display.

---

## Phase 9: Bugfix BUG-001 — auto-focus the terminal on session select/start (P1)

**Goal**: Selecting or starting a session focuses that session's terminal so the user can interact
with the AI CLI immediately, with no intervening click. Preserve every other focus guarantee
(release still works; `Running`-only write-gate; only the displayed session focused).

**Independent Test**: Start a session → without clicking, type → input reaches `claude`. Select a
different session in the sidebar → without clicking, type → input reaches that session's `claude`.
Release focus (Ctrl+Shift+E / click outside) → keys drive the app again; the session keeps running.

### Tests for BUG-001 (MANDATORY — Constitution Principle I) ⚠️

> Write/adjust these FIRST and ensure they FAIL before implementation.

- [X] T050 [BUG-001] Add failing pure tests in `tests/terminal_focus.rs` / `tests/app_state.rs` (`--no-default-features`): `State::default().terminal_focused == false` (unchanged base state); after `State::update(Message::SessionStarted(session))` the state is `terminal_focused == true`; after `State::update(Message::SessionSelected(id))` the state is `terminal_focused == true`; `Message::TerminalFocusReleased` still sets it back to `false`; `SessionCloseRequested`/project-switch leave/clear focus with no session displayed (contracts/focus-model.md BUG-001, FR-010/FR-010a).

### Implementation for BUG-001

- [X] T051 [BUG-001] In `src/app.rs`, set `self.terminal_focused = true` in the `Message::SessionStarted` and `Message::SessionSelected` reducer branches (auto-focus the newly displayed session); keep `SessionCloseRequested`/project-switch clearing focus. Make T050 pass (FR-010, FR-010a).
- [X] T052 [BUG-001] Ensure the sidebar-click auto-focus wins over the click-outside release in the gui path: in `src/main.rs` (and/or `src/ui`), guarantee that when a `SessionSelected` originates from a sidebar click the resulting state is focused even though the same click publishes `Message::TerminalFocusReleased` (e.g. by ordering/precedence so `SessionSelected` is applied last, or by the sidebar not emitting a spurious release). Update the focus docs in `docs/user-guide/worktrees-and-sessions.md` (T027) to describe auto-focus-on-select/start and how to release focus (FR-010a, FR-011, Principle VII).

**Checkpoint**: Selecting/starting a session focuses its terminal immediately; release still works; no key leakage to non-`Running` or background sessions.

**Bugfix**: 2026-07-17 — BUG-001 Updated from bugfix patch. T025 `SessionSelected`-clears-focus clause superseded; added Phase 9 (T050–T052) for auto-focus on select/start.

---

## Phase 10: Bugfix BUG-002 — sub-line scroll deltas discarded (touchpad scrolling dead)

**Goal**: Scrolling works with continuous, high-resolution scroll sources, not just discrete wheels
(FR-016b, SC-010).

**Independent Test**: In a session that reports scroll travel in units finer than one text line
(e.g. a touchpad under Wayland), two-finger scroll over the terminal pane moves the viewport
through the scrollback and the scrollbar appears.

### Tests for BUG-002

- [X] T053 [BUG-002] Write failing gui tests for the delta→lines mapping in
  `src/ui/material/terminal_pane.rs` (`cargo test --features gui`): successive sub-line
  `ScrollDelta::Pixels` events summing past one cell height produce exactly one line; the
  remainder after a whole line carries into the next event; reversing direction discards the banked
  residual; `ScrollDelta::Lines` passthrough is unchanged (contracts/terminal-render-input.md,
  FR-016b). Completed in `31a6e48`.

### Implementation for BUG-002

- [X] T054 [BUG-002] Extract `wheel_lines(delta, cell_height, &mut residual)` in
  `src/ui/material/terminal_pane.rs`, add `PaneState.scroll_residual`, and use it in the
  `WheelScrolled` arm for both the local-scrollback and mouse-report branches, replacing the
  per-event `.round()` that discarded any delta under one line. Make T053 pass (FR-016, FR-016b,
  FR-013a). Completed in `31a6e48`.
- [ ] T055 [BUG-002] ⚠️ Reopened scope of T028 — extend `tests/terminal_resize_scroll.rs` so the
  widget-level wheel coverage exercises pixel-precision deltas end-to-end (event → `TerminalScrolled`
  / mouse report), not just the pure mapping helper. T028 tested only whole-line deltas, which is
  why BUG-002 was invisible to CI on every platform.
- [X] T056 [BUG-002] Update `docs/user-guide/worktrees-and-sessions.md` (T031) to state that
  scrolling works with a mouse wheel *and* a touchpad/precision pointing device (Principle VII).
  Also documents the scrollbar's hide-at-live-bottom behavior, since "no scrollbar" was the
  user-visible face of BUG-002 and is otherwise easy to read as a defect.

**Checkpoint**: Touchpad scrolling moves the viewport on every supported platform and windowing
system; discrete-wheel behavior is unchanged.

**Bugfix**: 2026-07-20 — BUG-002 Updated from bugfix patch. T028's "wheel delta → `Scroll(n)`" test
scope was too narrow (whole-line deltas only) — widened via T055 rather than reopening T028, whose
original scope was met. Added Phase 10 (T053–T056) for the sub-line scroll accumulator.

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies - start immediately.
- **Foundational (Phase 2)**: Depends on Setup - BLOCKS all user stories.
- **User Stories (Phase 3-7)**: All depend on Foundational.
  - US1 is the MVP and is independently shippable (colored output with the existing input box).
  - US2 depends on Foundational; US3 builds on US2's focus acquisition; US4 and US5 are largely
    independent and can follow in any order after Foundational.
- **Polish (Phase 8)**: After the desired stories are complete.

### User Story Dependencies

- **US1 (rendering, P1)**: Foundational only. No dependency on other stories.
- **US2 (interactive input, P1)**: Foundational. Removes the line-input box; needs the pane skeleton.
- **US3 (focus gating, P2)**: Builds on US2 (focus acquisition + `keymap`); adds routing/gating.
- **US4 (sizing/scrollback, P3)**: Foundational (dynamic-size runtime); independent of US2/US3.
- **US5 (settings, P3)**: Foundational; independent (settings/toolbar/overlay). Applies its value
  to US4's scrollback but is testable on its own.

### Within Each User Story

- Tests written and FAILING before implementation (Principle I).
- Pure modules (`keymap`, `route_key`, settings) before their gui consumers.
- User-guide docs ship with the story (Principle VII).
- Story complete only when tests pass, docs exist, and it works on all three platforms.

### Parallel Opportunities

- Setup: T002, T003, T004 in parallel.
- Foundational: T005, T007, T009 in parallel (different files); T006 after T005; T008 after T007.
- US2 tests T015 (pure) and T016 (gui) in parallel before their implementations.
- Different stories can proceed in parallel once Foundational is done (US4/US5 fully; US3 after US2).

---

## Parallel Example: User Story 2

```bash
# Write the failing tests first (parallel — different files):
Task: "keymap::encode failing tests in tests/keymap.rs"          # T015
Task: "TerminalPane mouse failing gui tests in tests/terminal_mouse.rs"  # T016
# Then implement (keymap before its widget consumers):
Task: "Implement src/keymap.rs encode()"                          # T017
Task: "TerminalPane keyboard on_event in src/ui/components/terminal_pane.rs"  # T018
Task: "TerminalPane mouse on_event in src/ui/components/terminal_pane.rs"     # T019
```

---

## Implementation Strategy

### MVP First (User Story 1)

1. Phase 1 Setup → 2. Phase 2 Foundational → 3. Phase 3 US1 (colored rendering).
4. STOP and VALIDATE: colored/styled output follows the theme; the existing input box still
   sends commands. Demo the MVP.

### Incremental Delivery

1. Setup + Foundational → foundation ready.
2. US1 → colored rendering (MVP). 3. US2 → live interactive input (replaces line box).
4. US3 → focus gating correctness. 5. US4 → sizing/resize/scrollback. 6. US5 → configurable
   scrollback in Settings. Each story is an independently testable increment.

---

## Notes

- [P] = different files, no dependency on an incomplete task.
- The pure seams (`keymap::encode`, `route_key`, settings serde/validate) carry the TDD weight
  under `--no-default-features`; the gui widget's event→action mapping is covered by gui-gated
  tests (T010 palette, T016 mouse, T028 resize/scroll); pixel rendering (T012) is validated by
  quickstart (recorded deviation, Principle I).
- Adapted iced_term code (MIT) must be attributed; apply the `Ctrl+U == \x15` fix and the
  default-unfocused behavior (do not copy iced_term's `is_focused = true`).
- Commit after each task or logical group; stop at any checkpoint to validate a story.
