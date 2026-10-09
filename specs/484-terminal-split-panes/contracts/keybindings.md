# Contract: pane chords

| Action | Linux/Windows | macOS |
|---|---|---|
| Split vertical (side by side) | Ctrl+Shift+D | Cmd+Shift+D |
| Split horizontal (stacked) | Ctrl+Shift+H | Cmd+Shift+H |
| Close focused pane | Ctrl+Shift+W | Cmd+Shift+W |
| Focus left/right/up/down | Ctrl+Shift+Arrow | Cmd+Shift+Arrow |

Handled in `keymap.rs` before terminal encoding; never forwarded. A test enumerates feature 006's forwarded chords and asserts none equals a pane chord, and that existing `Ctrl/Cmd+Shift+E/T/C/V` are unchanged. A refused action (cap 6, minimum size, last pane) shows a transient visible reason.
