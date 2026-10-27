# Quickstart: validating #1464

## Part A, automated
| Command | Proves |
|---|---|
| `mise run test-core` | core unchanged |
| `cargo test -p micold-client --test features_composer` | picker state machine, drop/picker parity (FR-013), refusals, guard |
| `cargo test -p micold-client --test pane_drop_target --test features_session` | routing to the right composer, none to terminals |
| `cargo test -p micold-client --test layout_snapshot` | button fits the composer row |
| `mise run gate` | everything |

## Part B, visual pass (showcase / real GUI; `visual-pass` skill)
1. Showcase: Add attachment button normal, hover, focus, disabled; drop cue on a structured pane.
2. Real app per platform (Wayland, X11, macOS, Windows): drag a PNG and a text file over the transcript, then the composer: cue shows where reported, chip + quoted path appear, nothing reaches a terminal pane.
3. Click the button, pick two files, confirm: same result; cancel: no change; press the shortcut: picker opens; open the shortcut sheet: the chord is listed.
4. UI stays responsive (scroll, type) while the picker is open.
