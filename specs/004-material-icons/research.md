# Phase 0 Research: Material Design Icons

**Feature**: `004-material-icons` | **Date**: 2026-07-15

Consolidated decisions resolving the Technical Context unknowns. Format per decision:
Decision / Rationale / Alternatives considered.

## R1 — Icon delivery mechanism: bundled icon font vs SVG assets

**Decision**: Bundle a single **Material Symbols (Outlined)** icon font, embedded in the
binary, and render each icon as a glyph via a `text` widget in a dedicated font.

**Rationale**:
- The UI is already text-first; icons rendered as text inherit the exact same sizing
  (typography scale) and coloring (foreground color roles) path already used everywhere,
  so light/dark theming and disabled states come "for free" through existing style code.
- One embedded file covers every icon — no per-icon asset wiring, no second (recolored)
  asset set per theme.
- Apache-2.0 licensed, matching the repository, so no licensing friction (FR-010).

**Alternatives considered**:
- **Per-icon SVG assets via the svg widget**: crisp at any size, but tinting is not a
  first-class text color — each icon needs recoloring per theme or two asset sets, plus
  many files to manage. Rejected: heavier and inconsistent with the token-driven color
  path. (This matches the user's explicit decision to use the icon font.)

## R2 — Font variant: static instance vs variable font

**Decision**: Bundle a **static** instance of Material Symbols Outlined at a fixed axis
configuration (weight 400, fill 0, grade 0, optical size 24). One `.ttf`.

**Rationale**:
- iced 0.13 renders text through cosmic-text; a fixed static instance guarantees identical
  glyph shapes across Linux/macOS/Windows (SC-007, Principle VI) with no per-platform
  variable-axis resolution differences.
- The current surfaces need exactly one visual weight/fill; variable axes are unused
  headroom this pass (see spec Assumptions).

**Alternatives considered**:
- **Variable font with axis selection at render time**: more flexible but iced 0.13 exposes
  no ergonomic per-`text` axis control, and variable-instance rasterization can differ by
  platform. Rejected as premature.

## R3 — Registering and selecting the font in iced 0.13

**Decision**: Register the embedded font bytes at startup via the application builder's
`.font(..)` loader (alongside the existing `.default_font(..)` call in `src/main.rs`), and
select it at each call site with a `Font` constant carrying the font's family name.

**Rationale**:
- iced 0.13's `application(..)` builder accepts one or more `.font(bytes)` registrations
  that make the family available process-wide before the first frame; icons therefore
  cannot silently fall back to "tofu" because the family is guaranteed loaded (edge case:
  font load failure).
- Selecting via a named `Font` constant keeps call sites declarative:
  `text(glyph).font(MATERIAL_SYMBOLS).size(..)`.

**Open implementation detail (not a blocker)**: the exact family **name** string the font
advertises (e.g. `"Material Symbols Outlined"`) must be read from the shipped file with a
font inspector during implementation and pinned in one constant. This is a lookup, not a
design decision.

**Alternatives considered**:
- **Lazy/on-demand font loading**: unnecessary; a single small font is cheap to load once
  at startup and eliminates a first-render race.

## R4 — Where the name→glyph mapping lives (core vs GUI split)

**Decision**: Model icons as a **closed `Icon` enum in the render-free core**
(`src/icons.rs`), where each variant exposes its glyph as a `char` (its codepoint in the
Private Use Area). The GUI layer owns only font registration and a thin
`icon(Icon, size, Rgb) -> Element` render helper.

**Rationale**:
- A closed enum makes an unknown/misspelled icon **unrepresentable** — a wrong name is a
  compile error, never a runtime blank box (SC-005, FR-003, Principle V "make invalid
  states unrepresentable").
- The mapping is pure data, so it is unit-tested under `cargo test --no-default-features`
  with no iced dependency (FR-008, SC-006).
- Mirrors the established pattern: `src/tokens.rs` holds pure design data; `src/ui/style.rs`
  converts to iced types. Icons follow the same core/GUI seam.

**Alternatives considered**:
- **String-keyed lookup (HashMap<&str, char>)**: allows unknown keys at runtime; rejected
  — defeats the build-time guarantee.
- **Constants in the GUI layer**: would pull the mapping behind the `gui` feature and out
  of the no-GUI test run. Rejected.

## R5 — Curated icon set and codepoint pinning

**Decision**: Bundle only a **curated subset** of glyphs — the concepts the current
surfaces need plus small near-term headroom — not the full Material Symbols catalog. Each
`Icon` variant's codepoint is pinned from the font's official codepoints manifest during
implementation and asserted by a test.

**Curated concepts (initial set)** mapped to Material Symbols glyph names:

| `Icon` variant | Glyph name        | Used by (surface)                          |
|----------------|-------------------|--------------------------------------------|
| `Help`         | `help`            | app-bar Help action                        |
| `About`        | `info`            | Help menu → About action                   |
| `OpenProject`  | `folder_open`     | empty-state + "open another" buttons       |
| `Rename`       | `edit`            | known-projects list item                   |
| `Git`          | `commit`          | git badge (selector + known list)          |
| `ActiveMarker` | `check_circle`    | active known-project marker (replaces `●`) |
| `Unavailable`  | `error`           | unavailable known-project marker           |
| `NavigateUp`   | `arrow_upward`    | selector "Up" navigation (headroom)        |

**Rationale**: keeps the embedded font small and its license/provenance auditable;
codepoints pinned-and-tested so a font swap that moves a glyph is caught (SC-005).

**Note**: glyph-name→codepoint values are resolved from the shipped font at implementation
time; the table above fixes the *concepts and names*, which is the design contract.

## R6 — Licensing & provenance

**Decision**: Vendor the font under `assets/fonts/` together with its upstream `LICENSE`
(Apache-2.0) and a short provenance note (source, version/commit, axis instance, and the
codepoints reference used).

**Rationale**: FR-010 requires the resource's license and provenance recorded in-repo; the
constitution's Licensing constraint requires OSI-compatible, vetted dependencies. Apache-2.0
matches the repo license exactly.

## R7 — Accessibility / icon-only controls

**Decision**: Preserve each control's existing meaning. Where a control keeps a visible text
label, add the icon beside it. Where a control becomes icon-only, retain the prior wording
as the control's accessible/tooltip meaning so the action stays identifiable (FR-011).

**Rationale**: The feature is visual-only and must not regress identifiability; deeper
screen-reader labelling is out of scope this pass (spec Assumptions).
