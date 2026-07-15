# Implementation Plan: Material Design Icons

**Branch**: `004-material-icons` | **Date**: 2026-07-15 | **Spec**: [spec.md](spec.md)

**Input**: Feature specification from `/specs/004-material-icons/spec.md`

## Summary

Add a shared, cross-application Material icon vocabulary. A closed `Icon` enum in the
render-free core maps each named concept to a font codepoint; a single embedded Material
Symbols (Outlined) static font, registered at startup, backs the glyphs; and a thin GUI
helper renders any icon at a typography-scale size in a design-token foreground color. All
existing surfaces (app bar, buttons, known-projects list, git badge, active/unavailable
markers, selector navigation) adopt icons with **no behavior change**. Correct in light and
dark themes, identical across platforms, licensed Apache-2.0 (matches the repo).

## Technical Context

**Language/Version**: Rust, stable toolchain (via `mise`); `rust-version = 1.80`.

**Primary Dependencies**: iced 0.13 (GUI, behind the `gui` feature); no new crates — the
icon font is vendored data embedded via `include_bytes!`, not a dependency.

**Storage**: N/A — the feature adds no persisted or session state. The icon font is a
static embedded asset (local-first, offline).

**Testing**: `cargo test` — render-free core tests run under `--no-default-features`
(mapping/vocabulary); asset/font-integrity tests run under `--features gui`.

**Target Platform**: Linux, macOS, Windows desktop (feature parity required).

**Project Type**: Single-project desktop application (render-free core lib + iced GUI
binary).

**Performance Goals**: No regression to first-frame render; one small font loaded once at
startup. Rendering an icon is a normal text glyph draw (no measurable overhead).

**Constraints**: Offline/local-first; no network fetch or system-font dependency; embedded
Apache-2.0 font; no "tofu" ever reaches the UI (unknown icons unrepresentable at build
time).

**Scale/Scope**: Small curated icon set (~8 concepts initially); ~6 existing surfaces
touched; no new user actions or messages.

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-checked after Phase 1 design.*

- [x] **I. Test-First (NON-NEGOTIABLE)**: The `Icon` mapping and `ALL` invariants are
  specified as failing core tests first (contracts/icon-api.md → Test contract); font
  glyph-presence and family-name tests precede the GUI helper. No production code without a
  prior red test.
- [x] **II. Multi-Session Support**: No new state of any kind; nothing session-scoped, so
  isolation/persistence are unaffected. N/A but non-violating.
- [x] **III. Worktree Integration**: No file/VCS operations introduced. N/A but
  non-violating.
- [x] **IV. Local-First Storage (NON-NEGOTIABLE)**: Font embedded in the binary; fully
  offline; no network or external service. PASS.
- [x] **V. Rust + iced Stack**: Rust + iced only; the closed `Icon` enum makes an invalid
  icon unrepresentable (compile error), encoding the invariant in the type system. PASS.
- [x] **VI. Cross-Platform Parity**: Static embedded font renders identically on all three
  OSes; codepoints/font-name isolated behind the core enum + one GUI constant; CI tests on
  all three. PASS.
- [x] **VII. Documentation First-Class**: The user guide is updated in the same change to
  describe the icon vocabulary (FR-013); docs verified in CI. PASS.

**Result**: All gates pass. No entries required in Complexity Tracking.

## Project Structure

### Documentation (this feature)

```text
specs/004-material-icons/
├── plan.md              # This file
├── research.md          # Phase 0 output
├── data-model.md        # Phase 1 output
├── quickstart.md        # Phase 1 output
├── contracts/
│   └── icon-api.md      # Phase 1 output — internal UI contract
└── tasks.md             # Phase 2 output (/speckit-tasks — NOT created here)
```

### Source Code (repository root)

```text
src/
├── icons.rs             # NEW — closed `Icon` enum + `glyph()`/`ALL` (render-free core)
├── lib.rs               # MODIFIED — `pub mod icons;`
├── tokens.rs            # (unchanged) type_scale + Roles reused by the icon helper
├── main.rs              # MODIFIED — register embedded icon font via builder `.font(..)`
└── ui/
    ├── mod.rs           # MODIFIED — expose the `icon(..)` helper + MATERIAL_SYMBOLS font
    ├── style.rs         # (reused) Rgb→iced::Color conversion for icon tint
    ├── toolbar.rs       # MODIFIED — Help / About icons
    ├── shell.rs         # MODIFIED — OpenProject / Rename / Git / Active / Unavailable
    └── project_selector.rs  # MODIFIED — NavigateUp / Git badge / OpenProject

assets/
└── fonts/
    ├── MaterialSymbolsOutlined.ttf   # NEW — vendored static instance
    ├── LICENSE                        # NEW — Apache-2.0
    └── PROVENANCE.md                  # NEW — source, version, axis instance, codepoints ref

tests/
├── icons.rs             # NEW — core mapping/ALL invariants (no GUI)
└── icons_font.rs        # NEW — glyph presence + family-name (--features gui)

docs/user-guide/
└── appearance-theming.md  # MODIFIED (or new icons section) — icon vocabulary
```

**Structure Decision**: Single-project layout, following the established core/GUI seam:
pure icon data in the render-free core (`src/icons.rs`, mirroring `src/tokens.rs`), and font
loading + rendering in the GUI layer (`src/main.rs`, `src/ui/`, mirroring `src/ui/style.rs`).
The font asset is vendored under `assets/fonts/` and embedded, keeping the app offline and
cross-platform-deterministic.

## Complexity Tracking

No constitution violations — no entries required.
