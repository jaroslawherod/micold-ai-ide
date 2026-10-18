# Contract: the switcher chord (keys)

**Feature**: 968 | [plan](../plan.md) | [research R1, R8, R10](../research.md)

## K1 — The chord

`micold-client/src/keymap.rs`: the platform primary modifier plus Shift (Ctrl+Shift on Linux and
Windows, Cmd+Shift on macOS; Alt, and Ctrl on macOS / Super elsewhere, disqualify:
`is_primary_shift`) on `PhysicalKey::KeyP`. `is_switcher_chord(&KeyInput) -> bool` is the one place
the chord is chosen; `switcher_chord_labels()` renders `("Ctrl+Shift+P", "Cmd+Shift+P")` from it.

## K2 — Decoding

| Input | `encode` result |
|---|---|
| primary + Shift + physical `KeyP` | `KeyOutput::ToggleSwitcher` (tier 1, beside Navigate and ToggleOverview) |
| `physical = Other` (no code) | fallback on the logical key: `p` / `P` → `KeyP` |
| Ctrl+P (no Shift) | unchanged: `0x10` to the PTY |
| Shift+P, Alt+Shift+P, primary + Alt + Shift + P | unchanged |
| primary + Shift + E, T, O, D, H, W, C, V, Arrows, `[ ] , .` PageUp PageDown | unchanged (FR-002) |

`to_keymap_physical` maps `Code::KeyP` to `PhysicalKey::KeyP`, so a layout that types another
character at the P position still opens the switcher (FR-016).

## K3 — Routing

| Where the keyboard is | Who acts | PTY bytes |
|---|---|---|
| a terminal | `terminal_pane.rs` publishes `quick_switch::Msg::Toggled { terminal_had_focus: true }`, captures the event | 0 (SC-002) |
| elsewhere (sidebar, nothing, the switcher's own field) | `ui/mod.rs` `app_chord` publishes `Toggled { terminal_had_focus: false }` | — |

`route_key` maps `KeyOutput::ToggleSwitcher` to `KeyRouting::ToggleSwitcher` before its
`terminal_focused` check. Exactly one path fires per press. While the switcher's field has the
keyboard the chord must still reach `Toggled` (FR-001 "close"): a client test drives a primary +
Shift + P press into the open dialog and asserts `Toggled`; if the text field captures it, the
dialog's content maps the press itself (research R8).

## K4 — Docs

- `docs/user-guide/terminal-panes.md` §Keyboard shortcuts: one row, "Quick-switcher", with both
  forms.
- `docs/user-guide/worktrees-and-sessions.md` §Interacting with the terminal: a **Quick-switcher**
  bullet beside **Navigation**.

## K5 — Tests (`crates/micold-client/tests/keymap.rs`)

- K1 on the Linux/Windows modifier set decodes to `ToggleSwitcher` (and on the macOS set under
  `cfg(target_os = "macos")`), by physical key and by the K2 fallback with logical `p` and `P`.
- Physical `KeyP` with a logical key other than `p` (another layout) decodes to `ToggleSwitcher`.
- Each K2 near miss keeps its output; Ctrl+P still gives `0x10`.
- No existing reserved chord decodes to `ToggleSwitcher`, and the switcher chord decodes to nothing
  else (FR-002).
- `route_key(true, ToggleSwitcher) == route_key(false, ToggleSwitcher) == KeyRouting::ToggleSwitcher`.
- Docs test: both strings of `switcher_chord_labels()` appear in both K4 files (FR-017, SC-006).
