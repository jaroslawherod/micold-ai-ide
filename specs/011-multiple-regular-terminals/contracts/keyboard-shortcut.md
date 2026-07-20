# Contract: `Ctrl+Shift+T` / `Cmd+Shift+T` — open a new Regular Terminal instance

Pure detection in `src/keymap.rs`; gui-side dispatch in
`src/ui/material/terminal_pane.rs`. Governs FR-019.

## Detection (pure, total)

```rust
fn is_new_terminal_chord(key: &Key, mods: Mods) -> bool {
    let is_t = matches!(key, Key::Char(c) if c.eq_ignore_ascii_case(&'t'));
    if !is_t || !mods.shift {
        return false;
    }
    #[cfg(target_os = "macos")]
    { mods.logo && !mods.ctrl && !mods.alt }
    #[cfg(not(target_os = "macos"))]
    { mods.ctrl && !mods.logo && !mods.alt }
}
```

Structurally identical to the existing `is_release_chord` (Ctrl/Cmd+Shift+E) — same modifier-set
shape, same platform split. Checked in `encode()` at the same precedence tier as
`is_release_chord` (before named-key/control-chord/printable handling), so `Ctrl+Shift+T` is
**never** forwarded to the PTY as literal bytes, on either platform.

`KeyOutput` gains one variant: `NewTerminalInstance`.

## Dispatch (gui, `TerminalPane`)

`TerminalPane`'s key-event handler gets one new match arm alongside its existing
`ReleaseFocus`/`Copy`/`Paste` arms:

```rust
KeyOutput::NewTerminalInstance => {
    shell.publish(Message::ShellInstanceOpenRequested);
    event::Status::Captured
}
```

Like every other chord in this handler, it only fires while the terminal pane holds keyboard
focus (`self.focused` — the existing focus gate above the key-match block, unchanged). This is
not a global, focus-independent application shortcut; it matches the existing precedent set by
`Ctrl+Shift+E`/copy/paste, all of which are pane-focus-gated (research.md R4).

## Mode gating (edge case, FR-019)

The chord is detected and dispatched **unconditionally** on a `t`/`T` keypress with the right
modifiers, regardless of the session's current `TerminalMode` — `src/keymap.rs` has no notion of
`TerminalMode` and stays that way. The **binary**-side reducer for
`Message::ShellInstanceOpenRequested` (`src/main.rs`) is what checks the active session's
`mode`:

- `mode == Regular` → opens a new instance exactly as the on-screen "+" affordance would
  (contracts/terminal-instance-switcher-ui.md) — same message, same effect, regardless of
  trigger.
- `mode == AiCli` → no-op. The chord does **not** also switch the session into Regular mode; the
  pane keeps showing the AI CLI process untouched (spec Edge Cases: "Nothing happens").

## Precedence over plain typing

Because detection requires `Shift` held together with `Ctrl` (or `Cmd` on macOS), a plain `t`,
`T`, or `Shift+t` keypress is never affected — those fall through to `copy_paste_action` (no
match) and then to ordinary printable-character handling exactly as before this feature.
