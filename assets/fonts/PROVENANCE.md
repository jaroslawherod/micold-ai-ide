# Font Provenance: Material Symbols Outlined (full coverage)

**File**: `MaterialSymbolsOutlined.ttf`

## Source

- **Upstream**: [google/material-design-icons](https://github.com/google/material-design-icons)
- **Original**: `variablefont/MaterialSymbolsOutlined[FILL,GRAD,opsz,wght].ttf`
- **Codepoints reference**: `variablefont/MaterialSymbolsOutlined[FILL,GRAD,opsz,wght].codepoints`
- **License**: Apache License 2.0 (see `LICENSE` in this directory) — matches the repository license.

## How this file was produced

The shipped `.ttf` is a **static instance** of the upstream variable font, with **full glyph
coverage** — every codepoint the upstream font maps, not a curated subset (feature 009,
research R6). Earlier revisions of this file subset to only the glyphs the app used at the
time; that was replaced with full coverage so adding a new `Icon` variant never again requires
regenerating this binary — only `src/icons.rs` + `tests/icons.rs` change. Reproduce with
`fonttools`:

```sh
# 1. Instantiate a static instance at the pinned axis values
fonttools varLib.instancer "MaterialSymbolsOutlined[FILL,GRAD,opsz,wght].ttf" \
  wght=400 FILL=0 GRAD=0 opsz=24 -o _static.ttf

# 2. "Subset" to every Unicode codepoint the font already maps (drops the now-unused variable
#    axis tables — gvar/avar/fvar/HVAR — without dropping any glyph).
pyftsubset _static.ttf \
  --unicodes='*' \
  --output-file=MaterialSymbolsOutlined.ttf --name-IDs='*' --recalc-bounds
```

## Pinned facts (asserted by tests)

- **Font family name**: `Material Symbols Outlined`
- **Axis instance**: weight 400, fill 0, grade 0, optical size 24.

### Curated icon → glyph name → codepoint

| `Icon` variant | Material Symbols glyph | Codepoint (U+) |
|----------------|------------------------|----------------|
| `Help`         | `help`                 | `E8FD`         |
| `About`        | `info`                 | `E88E`         |
| `OpenProject`  | `folder_open`          | `E2C8`         |
| `Rename`       | `edit`                 | `F097`         |
| `Git`          | `commit`               | `EAF5`         |
| `ActiveMarker` | `check_circle`         | `F0BE`         |
| `Unavailable`  | `error`                | `F8B6`         |
| `NavigateUp`   | `arrow_upward`         | `E5D8`         |
| `AddSession`   | `add`                  | `E145`         |
| `AddWorktree`  | `account_tree`         | `E97A`         |
| `Menu`         | `more_vert`            | `E5D4`         |
| `LightMode`    | `light_mode`           | `E518`         |
| `DarkMode`     | `dark_mode`            | `E51C`         |
| `AutoMode`     | `brightness_auto`      | `E1AB`         |
| `HideSidebar`  | `left_panel_close`     | `F717`         |
| `ShowSidebar`  | `left_panel_open`      | `F716`         |
| `Settings`     | `settings`             | `E8B8`         |
| `Delete`       | `delete`               | `E872`         |
| `Copy`         | `content_copy`         | `E14D`         |
| `Filter`       | `filter_list`          | `E152`         |
| `AiCli`        | `auto_awesome`         | `E65F`         |
| `RegularTerminal` | `terminal`          | `EB8E`         |
| `ReleaseFocus` | `keyboard_hide`        | `E31A`         |
| `ProjectRoot`  | `home`                 | `E88A`         |
| `AddTerminalInstance` | `add_box`      | `E146`         |
| `Close`       | `close`                | `E5CD`         |
| `ActivityWorking` | `radio_button_checked` | `E837`     |
| `ActivityEnded` | `radio_button_unchecked` | `E836`   |

**Why the activity dots are radio-button glyphs** (BUG-004): this file is a static instance
pinned at **FILL=0**, and at that axis value the nominally-solid dots — `circle` (`EF4A`),
`lens` (`E3FA`), `fiber_manual_record` (`E061`) — all render as *rings*, not discs (verified by
rasterizing them from this file). `radio_button_checked` is the only same-diameter glyph here
with a genuinely filled centre, so this pair is what actually reads as filled-vs-hollow in *this*
font. Picking by icon name alone would have reproduced the bug in a new form: two rings that look
identical. If the FILL axis is ever changed, re-check these two before trusting the names.

This table documents which of the font's (now much larger) set of glyphs the app actually
*uses* — it is no longer the list of what the shipped file *contains* (that's every upstream
codepoint). Used codepoints are pinned in `src/icons.rs`; the mapping is regression-locked by
`tests/icons.rs` and glyph presence is verified by `tests/icons_font.rs`.

## Adding a new icon

1. Look up the glyph's codepoint in the upstream `.codepoints` manifest.
2. Add the `Icon` variant + codepoint in `src/icons.rs` and extend the mapping table above.
3. Extend the `tests/icons.rs` mapping assertion.

The font already contains every upstream glyph, so no regeneration step is needed here.
