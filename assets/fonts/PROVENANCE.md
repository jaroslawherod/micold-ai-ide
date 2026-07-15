# Font Provenance: Material Symbols Outlined (curated subset)

**File**: `MaterialSymbolsOutlined.ttf`

## Source

- **Upstream**: [google/material-design-icons](https://github.com/google/material-design-icons)
- **Original**: `variablefont/MaterialSymbolsOutlined[FILL,GRAD,opsz,wght].ttf`
- **Codepoints reference**: `variablefont/MaterialSymbolsOutlined[FILL,GRAD,opsz,wght].codepoints`
- **License**: Apache License 2.0 (see `LICENSE` in this directory) — matches the repository license.

## How this file was produced

The shipped `.ttf` is a **static instance** of the upstream variable font, **subset** to only
the glyphs this application uses (research R2/R5). Reproduce with `fonttools`:

```sh
# 1. Instantiate a static instance at the default axis values
fonttools varLib.instancer "MaterialSymbolsOutlined[FILL,GRAD,opsz,wght].ttf" \
  wght=400 FILL=0 GRAD=0 opsz=24 -o _static.ttf

# 2. Subset to the curated codepoints
pyftsubset _static.ttf \
  --unicodes=e5d8,f0be,eaf5,f097,f8b6,e2c8,e8fd,e88e \
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

These codepoints are taken from the upstream `.codepoints` manifest and are pinned in
`src/icons.rs`; the mapping is regression-locked by `tests/icons.rs` and glyph presence is
verified by `tests/icons_font.rs`.

## Adding a new icon

1. Look up the glyph's codepoint in the upstream `.codepoints` manifest.
2. Re-run the subset step above with the codepoint added to `--unicodes`.
3. Add the `Icon` variant + codepoint in `src/icons.rs` and extend the mapping table above.
4. Extend the `tests/icons.rs` mapping assertion.
