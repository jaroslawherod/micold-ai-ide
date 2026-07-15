# Phase 1 Data Model: Material Design Icons

**Feature**: `004-material-icons` | **Date**: 2026-07-15

This feature adds a small amount of **pure, stateless** data (an icon vocabulary). It
introduces no persisted state, no session-scoped state, and no message/flow changes.

## Entity: `Icon` (render-free core — `src/icons.rs`)

A single named symbol representing one concept/action. Modeled as a **closed enum** so an
unknown icon is unrepresentable (build-time safety; FR-003, SC-005).

**Variants (initial curated set — see research R5):**

`Help`, `About`, `OpenProject`, `Rename`, `Git`, `ActiveMarker`, `Unavailable`,
`NavigateUp`.

**Behavior / fields (derived, not stored):**

| Member          | Type   | Meaning                                                        |
|-----------------|--------|---------------------------------------------------------------|
| `glyph(self)`   | `char` | The font codepoint (Private Use Area) for this icon.          |
| `ALL`           | `&[Icon]` | Enumeration of every variant, for exhaustive testing.      |

**Validation rules:**
- Every variant MUST map to exactly one `char` (total function; no `Option`).
- Every variant's codepoint MUST resolve to a real glyph in the bundled font (asserted by
  a pinned-codepoint test; SC-005).
- The set is closed: adding a surface that needs a new concept MUST add a variant (a
  compile-time change), never a free-form string.

**State transitions:** none. `Icon` is immutable value data.

## Entity: `Icon Set` (the embedded font resource)

The single backing resource for all `Icon` glyphs.

| Attribute        | Value                                                             |
|------------------|------------------------------------------------------------------|
| Format           | Static TrueType instance, Material Symbols Outlined (research R2) |
| Location         | `assets/fonts/` (vendored, embedded via `include_bytes!`)        |
| Family name      | Pinned constant in the GUI layer (read from the file; research R3)|
| License          | Apache-2.0, vendored alongside with provenance (FR-010, research R6) |

**Relationship**: each `Icon::glyph()` codepoint indexes exactly one glyph in this
resource. The core owns the codepoints; the GUI owns loading the resource and selecting its
family.

## Rendering contract (GUI layer — `src/ui/`)

A thin helper turns an `Icon` + design-token inputs into an iced element. No new state.

- Input: an `Icon`, a size from `tokens::type_scale`, and a foreground color role (`Rgb`)
  from `tokens::Roles`.
- Output: an iced `Element` rendering the glyph in the icon font at that size and color.
- The helper reuses the existing `Rgb → iced::Color` conversion already in `src/ui/style.rs`
  so light/dark theming and disabled states follow the same path as all other text.

## Impact on existing entities

None. `State`, `Workspace`, `Project`, `Settings`, and the `Message` set are unchanged.
Surfaces swap/augment text with icons at the view layer only (FR-006: no behavior change).
