# Contract: Icon Font Full Coverage

Governs the `Icon::Filter` addition and the accompanying font-asset regeneration (research
R5/R6/R7). This is an asset/build-process contract, not a runtime API.

## `Icon::Filter` (src/icons.rs)

- New variant, appended to the `Icon` enum and to `Icon::ALL`.
- `glyph()` maps it to `'\u{e152}'` (Material Symbols `filter_list` — three descending-length
  lines; chosen over `filter_alt`/`ef4f` per direct user request, research R7).
- `tests/icons.rs`: `expected()` gains a `Icon::Filter => '\u{e152}'` arm;
  `all_covers_every_variant_without_duplicates` bumps its expected `Icon::ALL.len()` from `18`
  to `19`.
- `tests/icons_font.rs` requires no code change — `every_icon_codepoint_has_a_glyph` already
  iterates `Icon::ALL` generically, so it automatically covers `Icon::Filter` once the font is
  regenerated to include full upstream coverage (which also includes `U+E152`, verified
  directly — no second regeneration was needed for the R7 codepoint change).

## Font regeneration (assets/fonts/MaterialSymbolsOutlined.ttf)

Reproducible pipeline (extends the one already documented in `PROVENANCE.md`), using a
`uv`-managed Python environment with `fonttools`:

```sh
# 1. Fetch the upstream variable font (same source as the existing PROVENANCE.md).
curl -L -o _variable.ttf \
  "https://raw.githubusercontent.com/google/material-design-icons/master/variablefont/MaterialSymbolsOutlined%5BFILL%2CGRAD%2Copsz%2Cwght%5D.ttf"

# 2. Instantiate a static instance at the pinned axis values (unchanged from today).
fonttools varLib.instancer _variable.ttf wght=400 FILL=0 GRAD=0 opsz=24 -o _static.ttf

# 3. Subset to the FULL upstream codepoint set (every glyph the manifest lists), not a
#    curated subset — this is the only change from the existing PROVENANCE.md recipe.
curl -L -o _static.codepoints \
  "https://raw.githubusercontent.com/google/material-design-icons/master/variablefont/MaterialSymbolsOutlined%5BFILL%2CGRAD%2Copsz%2Cwght%5D.codepoints"
pyftsubset _static.ttf \
  --unicodes-file=<(awk '{print $2}' _static.codepoints | paste -sd,) \
  --output-file=assets/fonts/MaterialSymbolsOutlined.ttf --name-IDs='*' --recalc-bounds
```

- Family name and axis instance (weight 400 / FILL 0 / GRAD 0 / opsz 24) stay pinned exactly as
  today — `tests/icons_font.rs::font_advertises_the_pinned_family_name` requires no change.
- `PROVENANCE.md` is updated to document this as the standing regeneration recipe, replacing
  the narrow `--unicodes=<curated list>` step, and its "Adding a new icon" section is simplified
  to just: "1. Look up the glyph's codepoint in the upstream manifest. 2. Add the `Icon`
  variant + codepoint in `src/icons.rs`. 3. Extend `tests/icons.rs`." (no more font rebuild
  per icon).
- The curated `Icon` → glyph-name → codepoint table in `PROVENANCE.md` is kept (documents which
  of the font's many glyphs the app actually *uses*), but the "how this file was produced"
  section changes from "subset to only the glyphs this application uses" to "full static
  instance — every glyph the upstream variable font ships, at one fixed axis position".

## Invariants

1. Every existing `Icon` variant's glyph is still present at its existing codepoint after
   regeneration (`tests/icons.rs::glyph_maps_every_variant_to_its_pinned_codepoint` +
   `tests/icons_font.rs::every_icon_codepoint_has_a_glyph` both continue to pass unchanged for
   the pre-existing 18 variants).
2. The font's advertised family name stays `"Material Symbols Outlined"` (unchanged pin).
3. Adding a future `Icon` variant never again requires touching the `.ttf` binary, only
   `src/icons.rs` + `tests/icons.rs` — the font already carries the glyph.
