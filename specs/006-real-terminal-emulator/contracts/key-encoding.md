# Contract: Key Encoding (`keymap::encode`)

**Module**: `src/keymap.rs` (pure; `--no-default-features`). **Consumers**: `TerminalPane`
(gui) maps iced key events → `KeyInput`, calls `encode`, applies the `KeyOutput`.

Adapted from `iced_term 0.6.0` `bindings.rs` (MIT — attributed in the module), which encodes
the standard xterm/alacritty key map. **Deviations from that source are normative here.**

## Signature

```
fn encode(input: &KeyInput, mode: TermMode) -> KeyOutput
```

- `KeyInput { key: Key, mods: Mods, text: Option<String> }`
- `Key = Char(char) | Named(NamedKey)`
- `Mods = { SHIFT, CTRL, ALT, LOGO }` (bitflags)
- `TermMode = { app_cursor: bool, alt_screen: bool }`
- `KeyOutput = Bytes(Vec<u8>) | Copy | Paste | ReleaseFocus | Ignore`

## Rules (precedence top-to-bottom)

1. **Reserved focus-out chord** → `ReleaseFocus`. Never forwarded to the PTY. (FR-011)
   - Non-macOS: `Ctrl+Shift+E`. macOS: `Cmd+Shift+E`.
2. **Copy / Paste chords** → `Copy` / `Paste`. (FR-013)
   - Non-macOS: `Ctrl+Shift+C` → Copy, `Ctrl+Shift+V` → Paste.
   - macOS: `Cmd+C` → Copy, `Cmd+V` → Paste.
3. **Named keys** → `Bytes(escape sequence)` per the table below (varies with `mods` and
   `mode.app_cursor`/`alt_screen`). (FR-006)
4. **Control chords on letters/symbols** → `Bytes(control byte)` (see table). (FR-007)
5. **Printable** (`Key::Char` or a `Named` with associated text, no matching binding): if
   `text = Some(s)` → `Bytes(s.as_bytes())`. (FR-006, FR-008)
6. Otherwise → `Ignore`.

## Named-key base encodings (no modifiers)

| Key | Bytes |
|-----|-------|
| Enter | `\x0d` |
| Backspace | `\x7f` |
| Tab | `\x09` |
| Escape | `\x1b` |
| Space | `\x20` |
| Insert | `\x1b[2~` |
| Delete | `\x1b[3~` |
| PageUp / PageDown | `\x1b[5~` / `\x1b[6~` |
| F1..F4 | `\x1bOP` `\x1bOQ` `\x1bOR` `\x1bOS` |
| F5..F12 | `\x1b[15~ 17~ 18~ 19~ 20~ 21~ 23~ 24~` |
| Home / End (¬app_cursor) | `\x1b[H` / `\x1b[F` |
| Home / End (app_cursor) | `\x1bOH` / `\x1bOF` |
| Arrows Up/Down/Left/Right (¬app_cursor) | `\x1b[A` `\x1b[B` `\x1b[D` `\x1b[C` |
| Arrows (app_cursor) | `\x1bOA` `\x1bOB` `\x1bOD` `\x1bOC` |

Modifier variants (CSI `1;<n>` form, n = 2 shift, 3 alt, 5 ctrl, and combos 4/6/7/8) follow the
`bindings.rs` table for Home/End/PageUp/PageDown/Arrows/F-keys; Shift+Tab → `\x1b[Z`.

## Control-chord bytes (Ctrl + key)

Ctrl+`a`..`z` → `\x01`..`\x1a` **in order**, i.e. Ctrl+C=`\x03`, Ctrl+D=`\x04`, Ctrl+R=`\x12`,
Ctrl+W=`\x17`, Ctrl+Z=`\x1a`. Also `Ctrl+[`=`\x1b`, `Ctrl+]`=`\x1d`, `Ctrl+\`=`\x1c`,
`Ctrl+-`=`\x1f`, `Ctrl+Space`=`\x00`.

**Normative fix vs iced_term**: `Ctrl+U` MUST be `\x15` (iced_term's `bindings.rs` erroneously
maps it to `\x51`). A test MUST assert `encode(Ctrl+u) == Bytes([0x15])`.

## Required test coverage (`tests/keymap.rs`, written first — TDD)

- Every `NamedKey` base encoding, both `app_cursor` states for arrows/Home/End.
- Ctrl+`a`..`z` full range incl. the Ctrl+U=`\x15` regression.
- The reserved focus-out chord → `ReleaseFocus` (both platform variants via `cfg`).
- Copy/Paste chords → `Copy`/`Paste` (platform variants).
- Printable char with `text=Some` → `Bytes`; modifier-only / unmapped → `Ignore`.
- `encode` is total (never panics) for all enum inputs.
