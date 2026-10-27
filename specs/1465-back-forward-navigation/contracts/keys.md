# Contract: Back/Forward input (keys and side buttons)

**Feature**: 1465 | [plan](../plan.md) | [research R7, R8, R10, R11](../research.md)

## H1 — The keys (`crates/micold-client/src/keymap.rs`)

`history_key(&KeyInput) -> Option<micold_core::history::Dir>` is the one place the keys are chosen.

| Platform | Back | Forward | Modifiers held (exactly) |
|---|---|---|---|
| Linux, Windows | `NamedKey::ArrowLeft` | `NamedKey::ArrowRight` | Alt |
| macOS | physical `BracketLeft` | physical `BracketRight` | Cmd (`logo`) |

- macOS: when the platform gave no code (`PhysicalKey::Other`), the logical `[` / `]` decide, as
  `nav_chord`'s fallback does.
- Near misses decode to `None`: Alt+Shift+←, Ctrl+Alt+←, Alt+↑, plain ←; on macOS Cmd+Shift+[
  (the session chord, unchanged), Option+←/→, Cmd+Option+[.
- The split is `#[cfg(target_os = "macos")]` inside `keymap.rs`, beside `is_primary_shift`; no other
  module branches on the platform.
- `history_key_labels(dir) -> (String, String)`: `("Alt+←", "Cmd+[")` for Back,
  `("Alt+→", "Cmd+]")` for Forward. `history_key_label(dir)`: the running platform's one.

## H2 — `encode` is unchanged

`history_key` is not consulted by `encode`. With a terminal focused:

| Press | `encode` result (today and after) |
|---|---|
| Alt+← / Alt+→ (Linux, Windows) | `Bytes(ESC [1;3D)` / `Bytes(ESC [1;3C)` |
| Option+← / Option+→ (macOS) | the same bytes |
| Cmd+[ / Cmd+] (macOS) | `Ignore` |

The existing `tests/keymap.rs` byte tests stay as they are (SC-003).

## H3 — Routing a key

| Where the keyboard is | What happens |
|---|---|
| a focused terminal | `ui::subscription` returns before `app_chord_listener`: the key is the terminal's (H2) |
| a focused text field that captured the press (arrows in `text_input` / `text_editor`) | `keyboard::listen` never sees it |
| a focused field that did not capture it (macOS Cmd+[ in a field) | `app_chord` publishes `Pressed { source: Key }`; `history::resolve` sees `State::live_focused_field` and returns `Step::None` |
| a surface open, or Settings | `app_chord` publishes; `navigation::blocked` → `Step::None` (FR-010) |
| anything else (sidebar, nothing, the Review tab, the Overview, Compare) | `Pressed { dir, source: Key }` acts |

- `ui/mod.rs::app_chord` checks `history_key` after the existing chords.
- `ui/mod.rs::overview_arrow` returns `None` for a press that `history_key` decodes, so in the
  Overview Alt+← is Back only, not also a card step.
- Auto-repeat: each `KeyPressed` (repeat or not) is one press (FR-012).

## H4 — The side buttons (`crates/micold-client/src/shell/subscriptions.rs`)

`side_button(event, status, window) -> Option<Message>`, subscribed always with
`iced::event::listen_with`:

| Event | Message |
|---|---|
| `Mouse(ButtonPressed(Back))` | `History(Pressed { dir: Back, source: Button })` |
| `Mouse(ButtonPressed(Forward))` | `History(Pressed { dir: Forward, source: Button })` |
| releases, other buttons, any other event | `None` |

`status` is ignored, so a captured press still navigates (FR-008). The terminal pane reports only
Left, Middle and Right to a program (research R7): a Back or Forward press over a focused pane in
mouse-reporting mode publishes no `TerminalBytes`.

## H5 — Tests

`crates/micold-client/tests/keymap.rs`:
- H1 on the running platform's modifier set decodes Back and Forward; on macOS also by the logical
  fallback with `PhysicalKey::Other`; every H1 near miss is `None`.
- H2: the four rows keep their output.
- No existing chord (`nav_chord`, `is_switcher_chord`, `is_overview_chord`, pane, copy/paste,
  release, new terminal) decodes under `history_key`, and `history_key` presses decode to none of
  them.
- Docs test: both strings of each `history_key_labels` appear in `terminal-panes.md`'s app-chord
  table with "mouse back button" / "mouse forward button" (FR-013).

`crates/micold-client/src/ui/app_chord_tests.rs`: `app_chord` gives `History(Pressed { source:
Key })` for H1 presses and nothing for near misses; `overview_arrow` gives nothing for an H1 press
and a card step for a plain arrow.

`shell/subscriptions.rs` unit tests: H4's table, with `Status::Captured` and `Status::Ignored`.

`ui/material/terminal_pane.rs` in-crate tests (the headless `dispatch` harness): Back and Forward
presses inside a focused pane, with and without mouse mode, publish nothing that
`reaches_the_process`.
