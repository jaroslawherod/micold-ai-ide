# Contract: navigation chords (keys)

**Feature**: 803 | [plan](../plan.md) | [research R1, R2, R9, R10](../research.md)

## K1 — The chord table (the only place a chord is chosen)

`micold-client/src/keymap.rs`:

```text
const NAV_CHORDS: [(NavAction, PhysicalKey); 6] = [
  (Session  Previous, BracketLeft),
  (Session  Next,     BracketRight),
  (Location Previous, Comma),
  (Location Next,     Period),
  (Project  Previous, PageUp),     // D4, provisional
  (Project  Next,     PageDown),   // D4, provisional
];
```

Every chord is this physical key with the platform primary modifier plus Shift: Ctrl+Shift on Linux
and Windows, Cmd+Shift on macOS, and no other modifier (Alt, and Ctrl on macOS / Super elsewhere,
disqualify). Changing a chord (if the user answers D4 differently) is: this table, the K4 docs rows,
and nothing else; the K5 tests follow the table.

## K2 — Decoding

| Input | `encode` result |
|---|---|
| primary + Shift + physical key of a K1 row | `KeyOutput::Navigate(that action)` (tier 1, before Pane, Copy/Paste, named keys, control bytes) |
| `physical = Other` (platform gave no code) | fallback on the logical key: `[`/`{` → BracketLeft, `]`/`}` → BracketRight, `,`/`<` → Comma, `.`/`>` → Period, named PageUp/PageDown |
| same key, Shift missing (e.g. Ctrl+[) | unchanged: Ctrl+[ → `0x1b`, Ctrl+] → `0x1d` |
| same key, primary missing, or Alt added | unchanged (not a chord) |
| primary + Shift + E, T, O, D, H, W, Arrows, C, V | unchanged (their existing outputs) |

`KeyInput.physical` is filled from `iced::keyboard::Event::KeyPressed { physical_key, .. }` by
`to_keymap_physical`; layouts that move or AltGr-shift `[` still give `Code::BracketLeft` (FR-016).

## K3 — Routing

| Terminal focused | Who acts | PTY bytes |
|---|---|---|
| yes | `terminal_pane.rs` publishes `Message::Navigate(action)`, captures the event | 0 (SC-002) |
| no | `ui/mod.rs` `app_chord_listener` (`fn app_chord`) publishes `Message::Navigate(action)` | — |

`app::route_key` maps `KeyOutput::Navigate(a)` to `KeyRouting::Navigate(a)` before its
`terminal_focused` check, like `ToggleOverview`. Exactly one of the two paths fires per press.
Key repeat: each repeated `KeyPressed` is one press.

## K4 — Labels and docs

`nav_chord_labels(action) -> (String, String)`: the Linux/Windows and macOS forms, built from K1
(`Ctrl+Shift+[`, `Cmd+Shift+[`; `PageUp`/`PageDown` spelled so). The docs rows:

- `docs/user-guide/terminal-panes.md` §Keyboard shortcuts: three rows (session, location,
  project), each "previous / next" with both forms.
- `docs/user-guide/worktrees-and-sessions.md`: a **Navigation** bullet beside **Overview**, with the
  wrap rule, what is skipped (filtered or collapsed-group worktrees, unavailable projects), and the
  macOS laptop form Cmd+Shift+Fn+Up/Down.

## K5 — Tests (`crates/micold-client/tests/keymap.rs`)

- Each K1 row on Linux/Windows modifiers decodes to its action (and on the macOS set under
  `cfg(target_os = "macos")`), by physical key and by the K2 fallback, with logical `{` etc.
- Near misses from K2 keep their outputs; Ctrl+[ still gives `0x1b`.
- No two K1 rows share a key; no K1 chord decodes to an existing reserved output (R9).
- `route_key(false, Navigate(a)) == route_key(true, Navigate(a)) == KeyRouting::Navigate(a)`.
- Docs test: both strings of every `nav_chord_labels` appear in both K4 files.
