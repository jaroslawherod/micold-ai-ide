# Contract: Icon Vocabulary & Rendering API

**Feature**: `004-material-icons` | **Date**: 2026-07-15

This desktop app exposes no network/CLI interface; the relevant contract is the **internal
UI contract** between the render-free core (the icon vocabulary) and the GUI layer (font
loading + rendering). These signatures are the durable surface other code depends on.

## Core contract — `micold_ai_ide::icons` (no `gui` feature required)

```rust
/// A named Material symbol. Closed set: an unknown icon is a compile error, never a
/// runtime blank box (FR-003, SC-005).
pub enum Icon {
    Help,
    About,
    OpenProject,
    Rename,
    Git,
    ActiveMarker,
    Unavailable,
    NavigateUp,
}

impl Icon {
    /// The font codepoint for this icon (total function — every variant maps).
    pub const fn glyph(self) -> char;

    /// Every variant, for exhaustive iteration/testing.
    pub const ALL: &'static [Icon];
}
```

**Guarantees:**
- `glyph` is total: no `Option`, no panic path.
- `ALL` contains every variant exactly once (guards against a variant added without a
  codepoint).
- No dependency on iced; compiles and is tested under `cargo test --no-default-features`.

## GUI contract — rendering helper (behind the `gui` feature)

```rust
/// The embedded icon font's family, pinned from the shipped file.
pub const MATERIAL_SYMBOLS: iced::Font;

/// Render an icon as an element at a design-system size, tinted with a foreground role.
/// `size` comes from `tokens::type_scale`; `color` is a `tokens::Rgb` foreground role.
pub fn icon<'a, M>(icon: Icon, size: u16, color: tokens::Rgb) -> iced::Element<'a, M>;
```

**Guarantees:**
- The font family is registered at application startup before the first frame (research
  R3), so the helper never renders a fallback box for a known `Icon`.
- Color/size flow through the same `Rgb → iced::Color` path as existing text, so both
  themes and disabled states render consistently (FR-004, FR-007).

## Behavioral contract — surface application (FR-005, FR-006)

Each listed surface renders the mapped icon; **all pre-existing actions and messages are
unchanged**:

| Surface / element                          | Icon           | Prior behavior preserved                 |
|--------------------------------------------|----------------|------------------------------------------|
| App-bar Help action                        | `Help`         | `Message::HelpMenuToggled`               |
| Help menu → About                          | `About`        | `Message::AboutOpened`                   |
| Empty-state + "open another" buttons       | `OpenProject`  | `Message::ProjectSelectorOpened`         |
| Known-project list item — Open             | `OpenProject`  | `Message::*` reopen (enabled/disabled)   |
| Known-project list item — Rename           | `Rename`       | `Message::RenameStarted(..)`             |
| Git badge (selector + known list)          | `Git`          | badge only; no action                    |
| Active known-project marker (was `●`)      | `ActiveMarker` | marker only; no action                   |
| Unavailable known-project marker           | `Unavailable`  | reopen stays blocked                     |
| Selector "Up" navigation                   | `NavigateUp`   | existing up-navigation action            |

## Test contract (Principle I — tests precede implementation)

**Core (no GUI):**
- `glyph` returns the pinned codepoint for every variant (regression-locks the mapping).
- `ALL` length equals the number of variants and contains no duplicates.

**GUI/asset:**
- Every `Icon::glyph()` codepoint is present as a real glyph in the shipped font (no tofu;
  SC-005).
- The pinned `MATERIAL_SYMBOLS` family name matches the shipped file.
