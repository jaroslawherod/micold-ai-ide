# Implementation Plan: Material Design Layout & Theming

**Branch**: `003-material-design-layout` | **Date**: 2026-07-15 | **Spec**: [spec.md](./spec.md)

**Input**: Feature specification from `/specs/003-material-design-layout/spec.md`

## Summary

Restyle the entire application shell to a Material Design 3 visual language driven by a
single, centralized design system, and add light/dark theming that follows the OS by
default and is user-overridable and persisted. No new screens and no logic changes to
project/workspace behavior — this feature changes *presentation* plus *theme selection*.

Technical approach: keep the render-free core / iced-binary split intact. All the
**values and decisions** are pure and testable without iced: a `tokens` module holds the
design system (color roles for light and dark as plain RGB data, a typography size scale,
a spacing scale, and shape radii); a `theme` module holds the `ThemePreference`
(FollowSystem / Light / Dark) and `ColorScheme` (Light / Dark) enums plus the pure
resolution function `resolve(pref, system) -> ColorScheme`. The iced binary is the only
place that touches iced and the OS: it maps the resolved `ColorScheme` to an
`iced::Theme::custom` built from the token palette via the builder's `.theme(|state| …)`
closure, detects the OS scheme with the `dark-light` crate (gated behind the `gui`
feature), and pushes live OS changes in through a polling `Subscription`
(`iced::time::every` → `SystemThemeChanged`) because iced 0.13 emits no theme-changed
event. A small `SettingsStore` (a `settings.json` alongside the existing `projects.json`,
reusing the same `serde_json` + `directories` pattern) persists the preference. Every
existing `ui/*` surface is restyled through shared style helpers that read colors from the
token palette / `theme.extended_palette()`, so there are zero per-widget magic numbers
(SC-007).

## Technical Context

**Language/Version**: Rust, stable toolchain (managed via `mise`; provisioned by feature 001)

**Primary Dependencies**: iced 0.13 (existing; the `.theme()` and `.default_font()` builder
hooks, per-widget `.style(|theme, status| …)` closures, and `Theme::custom(name, Palette)`
are all used). **New GUI-only dep**: `dark-light` 1.x (`detect() -> Mode::{Dark,Light,Default}`;
cross-platform via XDG desktop portal on Linux, registry on Windows, `AppleInterfaceStyle`
on macOS) — made **optional** and pulled in by the `gui` feature so the render-free core
still compiles and tests without it. No new core (non-GUI) dependencies.

**Storage**: A second local JSON file, `settings.json`, in the same per-user data directory
as `projects.json`, written through a `SettingsStore` that reuses the existing atomic
write + missing/corrupt-recovers-to-default pattern from `store.rs`. Local-first, offline,
no new backend.

**Testing**: `cargo test --no-default-features --all-targets` — pure unit tests for theme
resolution (the FollowSystem/Light/Dark × system truth table), token invariants (every
`on_*` role meets AA 4.5:1 contrast against its surface in both schemes — SC-005), and a
settings save→load roundtrip incl. missing/corrupt → default (FollowSystem). Rendering and
live OS switching are validated via `quickstart.md` + the CI GUI build on all three OSes.

**Target Platform**: Desktop — Linux, macOS, Windows (feature parity required, including OS
theme detection on all three).

**Project Type**: Desktop application (GUI) — restyles the existing shell from features 001/002.

**Performance Goals**: Theme switches (user override or OS change) apply within 1 second
(SC-003) with no flicker; the OS-scheme poll runs off the render path on a sub-second (~500 ms)
interval — comfortably under the 1 s bound worst-case — and only emits a message when the
detected scheme actually *changes* (a transient detection error is not a change — it retains the
last-known scheme; see FR-021). *(Bugfix 2026-07-21 — BUG-001: original text was silent on
detection-call reliability, letting a `dark_light::detect()` timeout under CPU load masquerade as
a real OS change.)*

**Constraints**: Fully offline; no `cfg(target_os)` branching in core logic (OS detection is
isolated in the binary behind the `dark-light` boundary, mirroring the `FolderScanner`
pattern); design token values live in exactly one place; no third-party Material component
library is adopted (styling is built in-house on iced primitives — research R7).

**Scale/Scope**: One application window; ~5 existing surfaces restyled (app bar, shell
header/empty state, known-projects list, About, project selector, rename); two color schemes;
one persisted preference.

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

- [x] **I. Test-First (NON-NEGOTIABLE)**: PASS — every decision (theme resolution, contrast
  invariants over the token palettes, settings persistence roundtrip incl. corrupt-recovery)
  lives in the render-free core and is covered by failing-first `cargo test --no-default-features`
  tests written and reviewed before implementation. The iced styling layer carries no decision
  logic; it is validated by the CI GUI build + the `quickstart.md` manual walkthrough.
- [x] **II. Multi-Session Support**: PASS (not applicable) — the theme preference is a single
  app-global setting, not session state, and introduces no per-session state. It is stored
  separately from the projects catalog, so a future feature could layer per-session theming on
  top without reworking this; nothing here blocks Principle II.
- [x] **III. Worktree Integration**: PASS (not applicable) — no filesystem-of-project, git, or
  worktree interaction. OS theme detection reads only OS appearance settings.
- [x] **IV. Local-First Storage (NON-NEGOTIABLE)**: PASS — the preference persists to a local
  `settings.json`; the feature is fully functional offline; OS theme detection is a local OS
  query (DBus portal / registry / macOS defaults), never a network call. A missing/corrupt
  settings file degrades to FollowSystem rather than crashing (FR-019).
- [x] **V. Rust + iced Stack**: PASS — Rust + iced only; `dark-light` is a small OS-appearance
  library, not a GUI framework. Invalid states are unrepresentable: `ThemePreference` and
  `ColorScheme` are enums, so "some other theme" cannot exist and the resolved scheme is always
  exactly Light or Dark. Token roles are a fixed struct, not a stringly-keyed map.
- [x] **VI. Cross-Platform Parity**: PASS — `dark-light` detects the scheme on all three OSes;
  the polling subscription and the token palettes are identical across platforms; OS detection
  is confined behind one boundary in the binary (no `cfg(target_os)` in core). When the OS
  reports no preference (`Mode::Default`, e.g. a Linux box with no portal), FollowSystem falls
  back to Light (FR-018). CI already builds + tests on Linux, macOS, and Windows.
- [x] **VII. Documentation First-Class**: PASS — a user-guide page for the new look and theme
  selection (`docs/user-guide/appearance-theming.md`) ships in the same change, is linked from
  `docs/README.md`, and the CI docs job asserts it exists.

**Result**: All gates PASS. No violations → Complexity Tracking left empty.

## Project Structure

### Documentation (this feature)

```text
specs/003-material-design-layout/
├── plan.md              # This file (/speckit-plan command output)
├── research.md          # Phase 0 output (/speckit-plan command)
├── data-model.md        # Phase 1 output (/speckit-plan command)
├── quickstart.md        # Phase 1 output (/speckit-plan command)
├── contracts/           # Phase 1 output (/speckit-plan command)
│   ├── design-tokens.md     # The design system: color roles (light+dark), type/spacing/shape scales, button-variant mapping (durable contract)
│   ├── theme-behavior.md    # Preference→scheme resolution truth table + live-update behavior (contract)
│   └── settings-schema.md   # On-disk settings.json format for the theme preference (durable contract)
├── checklists/
│   └── requirements.md  # Spec quality checklist (/speckit-specify output)
└── tasks.md             # Phase 2 output (/speckit-tasks command - NOT created by /speckit-plan)
```

### Source Code (repository root)

```text
Cargo.toml               # add dark-light as an OPTIONAL dep enabled by the `gui` feature

src/
├── main.rs              # add .theme(theme_fn) + .default_font(...); add OS-scheme poll subscription;
│                        #   load settings on boot; persist preference on change
├── lib.rs               # export new core modules: tokens, theme, settings
├── app.rs               # extend State (theme_pref, system_scheme) + Message
│                        #   (ThemePreferenceChanged, SystemThemeChanged) + reducer arms
├── tokens.rs            # NEW (pure): design system — Rgb color roles for Light & Dark,
│                        #   typography size scale, spacing scale, shape radii. No iced.
├── theme.rs             # NEW (pure): ThemePreference + ColorScheme enums; resolve(pref, system)
├── settings.rs          # NEW (pure core + std impl): Settings value + SettingsStore trait
│                        #   + JsonFileSettingsStore (serde_json + directories, atomic write)
├── project.rs           # (existing, unchanged)
├── workspace.rs         # (existing, unchanged)
├── selector.rs          # (existing, unchanged)
├── fs_scan.rs           # (existing, unchanged)
├── store.rs             # (existing, unchanged)
└── ui/
    ├── mod.rs           # wrap base in themed background; dispatch unchanged
    ├── style.rs         # NEW (gui): iced Theme builder from tokens (light/dark) + shared
    │                    #   style helpers (surface, app bar, filled/outlined/text buttons, list item)
    ├── theme_menu.rs    # NEW (gui): the Follow system / Light / Dark selector control
    ├── toolbar.rs       # restyle → Material top app bar; host the theme menu
    ├── shell.rs         # restyle header / empty state / known-projects list as Material surfaces
    ├── about.rs         # restyle to the design system
    ├── project_selector.rs  # restyle to the design system
    └── rename.rs        # restyle to the design system

tests/
├── theme.rs             # NEW: resolve() truth table (FollowSystem/Light/Dark × Light/Dark/Default)
├── tokens.rs            # NEW: AA contrast invariants for every on_* role vs its surface (both schemes)
├── settings_roundtrip.rs    # NEW: save→load roundtrip; missing/corrupt → FollowSystem default
└── (existing test files unchanged)

docs/
├── README.md           # add link to the new user-guide page
└── user-guide/
    └── appearance-theming.md  # NEW: the new look + choosing and persisting a theme (Principle VII)

.github/
└── workflows/
    └── ci.yml          # docs job: also assert docs/user-guide/appearance-theming.md exists
```

**Structure Decision**: Continue the render-free-core + iced-binary layout from 001/002. All
design-system *values* and theme *decisions* go in three new pure core modules (`tokens.rs`,
`theme.rs`, `settings.rs` — the last trait-fronted like `store.rs` so its logic is testable
without the real data dir). The iced binary gains one styling module (`ui/style.rs`) that
converts tokens into an `iced::Theme` and exposes the shared style helpers every surface uses,
plus a small `ui/theme_menu.rs` control; the existing `ui/*` surfaces are restyled in place.
The single OS-detection boundary (`dark-light`, `gui`-gated) mirrors the `FolderScanner`
boundary, keeping `cfg(target_os)` out of the core.

## Complexity Tracking

> No constitution violations. Section intentionally empty.
