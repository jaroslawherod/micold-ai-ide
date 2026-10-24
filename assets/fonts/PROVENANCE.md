# Font Provenance

This directory ships **three typefaces under two different licences**. They are recorded separately,
and each names the licence file that covers it:

| Font | Files | Licence | Licence text |
|------|-------|---------|--------------|
| Material Symbols Outlined | `MaterialSymbolsOutlined.ttf` | Apache-2.0 | `LICENSE` |
| Roboto | `Roboto-Regular.ttf`, `Roboto-Medium.ttf`, `Roboto-SemiBold.ttf` | SIL Open Font License 1.1 | `LICENSE-Roboto-OFL.txt` |
| Fira Mono | `FiraMono-Regular.ttf`, `FiraMono-Bold.ttf` | SIL Open Font License 1.1 | `LICENSE-FiraMono-OFL.txt` |

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
| `Search`       | `search`               | `E8B6`         |
| `AiCli`        | `auto_awesome`         | `E65F`         |
| `RegularTerminal` | `terminal`          | `EB8E`         |
| `ReleaseFocus` | `keyboard_hide`        | `E31A`         |
| `ProjectRoot`  | `home`                 | `E88A`         |
| `AddTerminalInstance` | `add_box`      | `E146`         |
| `Close`       | `close`                | `E5CD`         |
| `ActivityWorking` | `radio_button_checked` | `E837`     |
| `ActivityEnded` | `radio_button_unchecked` | `E836`   |
| `SelectChevron` | `expand_more`         | `E5CF`         |
| `SessionService` | `dns`                | `E875`         |
| `Daemons`        | `lan`                | `EB2F`         |
| `Refresh`      | `refresh`              | `E5D5`         |
| `IssueMapping` | `label`                | `E892`         |
| `JiraIssues`   | `assignment`           | `E85D`         |
| `MoveUp`       | `keyboard_arrow_up`    | `E316`         |
| `MoveDown`     | `keyboard_arrow_down`  | `E313`         |
| `FullScreen`   | `fullscreen`           | `E5D0`         |
| `FullScreenExit` | `fullscreen_exit`    | `E5D1`         |
| `Thinking`     | `psychology`           | `EA4A`         |
| `ToolCall`     | `build` (alias)        | `F8CD`         |
| `ToolRunning`  | `progress_activity`    | `E9D0`         |
| `ToolDone`     | `check_circle_outline` (alias of `check_circle`) | `E92D` |
| `ToolFailed`   | `error_outline` (alias of `error`) | `E001` |
| `ToolNoResult` | `do_not_disturb_on`    | `F08F`         |
| `JumpToLatest` | `arrow_downward`       | `E5DB`         |
| `Subagent`     | `smart_toy`            | `F06C`         |
| `BackgroundTask` | `pending_actions`    | `F1BB`         |
| `TaskDone`     | `check_box`            | `E834`         |
| `TaskOpen`     | `check_box_outline_blank` | `E835`      |
| `MoreHorizontal` | `more_horiz`         | `E5D3`         |

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
| `Roboto-SemiBold.ttf` | 600 | the Overview card's semibold words (BUG-1298) |

The third instance came with BUG-1298: the Overview Card Redesign mockup sets the card's title,
state word, issue link, "NOW", pull request pills and alarming status words at weight 600. Without
a 600 face the matcher would fall back to Medium and the card would read a shade lighter than its
design. It is cut from the same variable font with the same command, `wght` 600:
`(600, "Roboto-SemiBold.ttf")`.

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
| `Roboto-SemiBold.ttf` | `4f87cafb0738d613b4ad4c873255aaed4f169cb567b50ce78f76a30d480c94f7` | `Roboto SemiBold` | `Roboto` |

`Roboto-SemiBold.ttf` was cut on 2026-10-24 with fonttools 4.67.0 from the variable font with the
sha256 above.

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

---

## Fira Mono (Regular 400, Bold 700)

**Files**: `FiraMono-Regular.ttf`, `FiraMono-Bold.ttf`

The monospace face the application draws all code-like text with (feature 1260): code blocks,
inline code, diffs, tool output and terminal cells. `Font::MONOSPACE` resolves to it ahead of any
host font of the same name (`crates/micold-client/src/ui/material/mono.rs`).

### Source

- **Upstream**: [mozilla/Fira](https://github.com/mozilla/Fira), `ttf/FiraMono-Regular.ttf` and
  `ttf/FiraMono-Bold.ttf` at commit `fd8c8c0a3d353cd99e8ca1662942d165e6961407`
- **Version**: Fira Mono 3.206, shipped as upstream builds them (no instancing or subsetting)
- **Licence**: SIL Open Font License 1.1. `LICENSE-FiraMono-OFL.txt` is upstream's `LICENSE` with
  LF line endings where upstream uses CRLF; the text is otherwise identical. It is a separate file
  from `LICENSE-Roboto-OFL.txt` because each OFL text names its own copyright holders and Reserved
  Font Names.

| File | Weight | sha256 |
|------|-------:|--------|
| `FiraMono-Regular.ttf` | 400 | `8c86f2963208a353c3435e28ecb38a99ab68f14d7433ba00c1822cef9a9c1b44` |
| `FiraMono-Bold.ttf` | 700 | `2a28efd740e8da1da75d40ac79f0db4f60a0f1aead0b15ca16b4694a11b45fc6` |

Both report family `Fira Mono` (name ID 1), are fixed-pitch, and advance 600 units per 1000-unit
em for every printable ASCII character; the tests in `mono.rs` assert it. Characters the face lacks
fall back to a system font, as for Roboto.
