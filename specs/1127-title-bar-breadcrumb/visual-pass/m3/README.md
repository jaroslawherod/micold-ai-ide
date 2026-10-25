# Visual pass, milestone M3 (quickstart §B 4, 6, 7 — the jump-to field)

Run 2026-10-25 on Xvfb (tiny-skia), the pinned M3 build (b03f8d13 + 4782f85b, client and daemon
from one build), 1280 × 800 and 640 × 480, no project open (the field opens the switcher there too,
US2 s4), light then dark.

| Check | Result | Evidence |
|---|---|---|
| §B4 The field: search glyph, `Jump to location, session or project`, an outlined `Ctrl+Shift+P` key-cap, a 360 dp pill centred in the free space between the Overview button and Settings (x 556–916; free space 282–1180) | pass | `b4-field-1280.png` |
| §B4 Press the field: the quick switcher opens with its input focused — typed keys land in it | pass | `b4-press-opens-switcher.png`, `b4-switcher-input-focused.png` |
| §B4 Press the field's position with the switcher open: the switcher closes, no second one opens | pass | `b4-press-again-closes.png` |
| §B4 Settings open: the field is drawn at the disabled 38%, the gear selected; a press opens nothing | pass | `b4-settings-field-disabled.png` |
| §B5 Hover the field: tooltip `Jump to location, session or project (Ctrl+Shift+P)` below the bar, hover layer on the pill | pass | `b5-field-tooltip.png` |
| §B6 640 × 480: the field drops its words and keeps glyph and hint, still centred; every control in the bar | pass | `b6-640-field-drops-label.png` |
| §B6 Narrower still (glyph alone, then the place and the project shortening) | not reachable with `Select project` at the 640 minimum; covered by `toolbar::tests::the_bar_gives_way_in_the_five_step_order` and `gates/title_bar_fits` (640, 60-character names: the field keeps its glyph) | — |
| §B7 Dark scheme: words and edges legible on the bar | pass | `b7-dark-1280.png` |
| §B7 macOS hint `Cmd+Shift+P` | not driven (Linux host); `keymap::switcher_chord_label` is the existing per-OS label the status bar already shows | — |
