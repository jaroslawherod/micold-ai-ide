# Contract: `TerminalPane` Widget (render + focus-gated input)

**Module**: `src/ui/components/terminal_pane.rs` (gui). A reusable iced `advanced::Widget`
(Principle VIII). Adapted from `iced_term 0.6.0` `view.rs` (MIT — attributed). Renders feature
005's `alacritty_terminal::Term` and turns focused user events into `TermAction`s.

## Construction

```
TerminalPane::new(term: &RuntimeTerminal, palette: &TermPalette, font: FontMetrics,
                  focused: bool) -> Element<'_, Message>
```

- `term` — the displayed session's runtime (borrows its `Term` grid + selection + mode).
- `palette` — `ansi::Color → iced::Color` mapping; defaults from the active theme (see
  `research.md` R3).
- `focused` — mirrors `State.terminal_focused`; seeds the widget's internal `is_focused`.

## Rendering (`draw`) — FR-001..FR-005

Canvas render over `content.grid.display_iter()`:
- **Colors**: fg/bg via `palette.get_color(cell.fg/.bg)` — 16 + bright + dim + 256 + truecolor
  (`ansi::Color::Spec`). Default bg = theme background (container paints it; skip per-cell).
- **Styles** from `cell::Flags`: `BOLD/DIM_BOLD`→bold font; `ITALIC`→italic; `DIM`→fg alpha×0.7;
  `INVERSE` (or within selection range)→swap fg/bg; `UNDERLINE`→underline stroke;
  `STRIKEOUT`→strike stroke; `HIDDEN`→fg=bg.
- **Cursor**: filled cell at `grid.cursor.point` when `TermMode::SHOW_CURSOR`.
- **Perf**: background-run batching + `canvas::Cache`, cache cleared only when `term.dirty`
  (new bytes applied). ≤1 redraw/frame under flood (FR-005a).

## Input (`on_event`) — FR-006..FR-016

- **Auto-resize**: when the widget's layout size changes, publish `Message::TerminalResized {
  cols, rows }` computed from `layout / cell metrics`. (FR-014/FR-015)
- **Focus gate**: `if !state.is_focused { return Status::Ignored }` — unfocused events fall
  through to the app. (FR-009)
- **Keyboard** (focused): build `KeyInput` from the iced event; `keymap::encode(input, mode)`:
  - `ReleaseFocus` → publish `Message::TerminalFocusReleased`.
  - `Copy` → `clipboard.write(term.selectable_content())`.
  - `Paste` → publish `TerminalAction(Paste(clipboard.read()))`.
  - `Bytes(b)` → publish `TerminalAction(Write(b))`.
  - `Ignore` → `Status::Ignored`.
- **Mouse** (focused, cursor in bounds):
  - If `terminal_mode ∩ MOUSE_MODE` and no selection override → `TerminalAction(MouseReport(..))`
    (SGR/normal encoding as in iced_term `backend.rs`). (FR-013a)
  - Else left-press/drag → `SelectStart`/`SelectUpdate` (single/double/triple = simple/semantic/
    lines). Holding **Shift** forces selection even under mouse mode. (FR-013, FR-013b)
  - Wheel → `TerminalAction(Scroll(lines))`; the binary forwards to the PTY on
    `ALT_SCREEN|ALTERNATE_SCROLL`, else scrolls local scrollback. (FR-016 + wheel edge case)
- Any produced action ⇒ `Status::Captured`.

## Focusable

The pane is click-focusable (publishes `Message::TerminalFocused`), and exposes an
`operation::Focusable`/id so the binary can focus it programmatically if needed. Internal
`is_focused` defaults **false** (unlike iced_term).

## Binary application (`src/main.rs`)

The binary applies `Message::TerminalAction(a)` to the displayed, focused session's
`RuntimeTerminal`:
- `Write`/`Paste` → `rt.write(bytes)` **only if `Running`** (FR-012a); else dropped.
- `Scroll` → `rt.scroll(delta)`; `Select*` → `rt.{start,update}_selection`; `MouseReport` →
  `rt.mouse_report(..)`; and marks the pane dirty as needed.

## Tests

- GUI-gated unit tests adapting iced_term's `view.rs` suite: left-press selection vs mouse
  report; cursor-moved grid mapping; wheel→scroll; focus gate (unfocused ⇒ `Ignored`).
- Pure `keymap` + `route_key` tests cover the key path (see other contracts).
- End-to-end via `quickstart.md`.
