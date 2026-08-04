# Font Provenance

This directory ships **two typefaces under two different licences**. They are recorded separately,
and each names the licence file that covers it:

| Font | Files | Licence | Licence text |
|------|-------|---------|--------------|
| Material Symbols Outlined | `MaterialSymbolsOutlined.ttf` | Apache-2.0 | `LICENSE` |
| Roboto | `Roboto-Regular.ttf`, `Roboto-Medium.ttf` | SIL Open Font License 1.1 | `LICENSE-Roboto-OFL.txt` |

**Why two licence files rather than one.** Feature 018's T014a required this to be decided
explicitly rather than by default, and the answer is forced: the two works are under *different*
licences. `LICENSE` is the Apache-2.0 text and covers the icon font. Extending it to silently cover
Roboto would misstate Roboto's terms — the OFL has obligations Apache-2.0 does not, notably around
the Reserved Font Name and the requirement that the licence travel with the font. So Roboto ships
its own verbatim `LICENSE-Roboto-OFL.txt`, and this table is the mapping.

> **Note on the spec.** `contracts/design-tokens.md` §2.1 describes Roboto as Apache-2.0. That was
> true historically; Google relicensed Roboto to the SIL OFL, and `google/fonts`'s own `METADATA.pb`
> for the family now records `license: "OFL"`. The shipped licence follows the font, not the spec,
> and the contract has been corrected to match.

---

## Material Symbols Outlined (full coverage)

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


---

## Roboto (Regular 400, Medium 500)

**Files**: `Roboto-Regular.ttf`, `Roboto-Medium.ttf`

### Source

- **Upstream**: [google/fonts](https://github.com/google/fonts), `ofl/roboto/`
- **Original**: `Roboto[wdth,wght].ttf` — the upstream variable font
  - sha256 `d7598e12c5dbef095ff8272cfc55da0250bd07fbdecbac8a530b9b277872a134`
- **Licence**: SIL Open Font License 1.1 (see `LICENSE-Roboto-OFL.txt`, copied verbatim from
  `ofl/roboto/OFL.txt`)
- **Copyright**: Copyright 2011 The Roboto Project Authors
  (<https://github.com/googlefonts/roboto-classic>)

### What is shipped, and why

Two **static instances**, not the variable font (contract §2.1). Weights 400 and 500 are the only
ones the Material 3 type scale specifies, so two static faces express every role faithfully at the
smallest binary cost — and, more importantly, without the risk that a variable font renders every
role at its default weight and silently collapses the scale's 400/500 distinction.

| File | Weight | Backs |
|------|-------:|-------|
| `Roboto-Regular.ttf` | 400 | display, headline, body roles |
| `Roboto-Medium.ttf` | 500 | title medium/small, all label roles |

### How these files were produced

Both axes are pinned: `wdth=100` (normal width) and `wght` selecting the face. `updateFontNames`
rewrites the name table so each instance identifies itself honestly rather than still claiming to be
the variable font it was cut from.

```sh
uv run --with fonttools python - <<'EOF'
from fontTools.varLib import instancer
from fontTools.ttLib import TTFont

for weight, out in [(400, "Roboto-Regular.ttf"), (500, "Roboto-Medium.ttf")]:
    f = TTFont("Roboto[wdth,wght].ttf")
    inst = instancer.instantiateVariableFont(
        f, {"wght": weight, "wdth": 100}, inplace=True, updateFontNames=True, optimize=True
    )
    inst.save(out)
EOF
```

Resulting artifacts:

| File | sha256 | Family (name ID 1) | Typographic family (ID 16) |
|------|--------|--------------------|----------------------------|
| `Roboto-Regular.ttf` | `928c76997bb8f7c298f0f19f92b016333af514d47cedc25ba7cc99c307d4c4f5` | `Roboto` | — |
| `Roboto-Medium.ttf` | `835e5e39f60aee644a9ee55268ad698c755e271b6d3d9a906861b08d41f51885` | `Roboto Medium` | `Roboto` |

The Medium instance putting `Roboto Medium` in name ID 1 is the standard convention — it exists for
software that can only express a family plus regular/bold — while the real family stays in ID 16.
Font matching resolves on the typographic family, so both faces belong to `Roboto` and are told
apart by weight. `crates/micold-client/tests/roboto_font.rs` asserts exactly that, along with the
weights, the static-ness, and coverage of the interface's own text.

### Coverage and fallback

Roboto covers Latin, Latin-1 and the symbols the interface composes its own strings from. It does
**not** cover CJK or arrows, and is not required to: text outside its coverage is *user data* — a
worktree named in Japanese — and FR-013 requires that to fall back to a font that does cover it
rather than render missing-glyph boxes.
