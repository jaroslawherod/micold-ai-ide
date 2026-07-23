# Research: Material Design Layout & Theming

Phase 0 decisions. Each item resolves an unknown from the Technical Context. API facts were
verified against the vendored source of **iced 0.13.1** (`iced_core` 0.13.2, `iced_widget`
0.13.4, `iced_winit` 0.13.0) and **dark-light 1.1.1** in the local cargo registry.

## R1 — Dynamic theme selection in iced 0.13

**Decision**: Use the application builder's `.theme(|state: &State| -> iced::Theme)` closure to
pick the theme from state every frame.

**Rationale**: `iced::application(...)` exposes `.theme(f: impl Fn(&State) -> Theme)`
(`iced-0.13.1/src/application.rs:339`). It is re-evaluated each render, so switching a theme is
just changing state (the resolved `ColorScheme`) — no restart, no window rebuild. Chains
alongside the existing `.subscription(...)` / `.run_with(...)`.

**Alternatives considered**: The sibling `.style(|state, &Theme| Appearance)` hook only sets the
window background/text appearance; it does not select a theme, so it is insufficient alone
(we still use the palette-derived background via the chosen theme).

## R2 — Custom Material palette vs. built-in themes

**Decision**: Build two custom themes with `iced::Theme::custom(name, Palette)` — one from the
Material light palette, one from the Material dark palette — rather than reusing built-ins like
`Theme::Light` / `Theme::Dark`.

**Rationale**: `iced::theme::Palette` has five color fields (`background, text, primary, success,
danger`); `Theme::custom` runs `palette::Extended::generate` to derive the widget-ready
`base/weak/strong` shades and an `is_dark` flag (`iced_core-0.13.2/src/theme/palette.rs`).
Mapping our Material color roles onto these five gives iced-native styling for free while
keeping our exact brand colors. Built-in `Light`/`Dark` would not match Material 3 and would
scatter our color intent.

**Alternatives considered**: (a) `custom_with_fn` to fully hand-author the extended palette —
more control than needed for v1; keep `custom` and let iced generate shades, overriding only
where a specific role (surface, on-surface) must be exact via per-widget style helpers.
(b) Reusing a built-in — rejected: no Material fidelity, tokens not centralized.

## R3 — OS dark/light detection, cross-platform

**Decision**: Use the `dark-light` crate, ~~version 1.x, calling `dark_light::detect() -> Mode`~~
**version 2.x, calling `dark_light::detect() -> Result<Mode, Error>`** (see Bugfix note below).

**Rationale**: ~~One synchronous, infallible call returns `Mode::{Dark, Light, Default}`~~ One
synchronous call returns `Result<Mode, Error>` (`Mode::{Dark, Light, Default}` on success) and is
implemented for macOS (`AppleInterfaceStyle`), Windows (registry), and Linux/BSD (XDG desktop
portal over DBus, with GTK/KDE config fallback, bounded by a hardcoded 25 ms timeout on the portal
round-trip). This satisfies cross-platform parity (Principle VI) behind a single boundary. A
successful `Mode::Default` (undetectable — e.g. a headless Linux box with no portal) maps to our
FollowSystem fallback of Light (FR-018). An `Err` (e.g. `Error::Timeout`, most often seen under
CPU contention when the 25 ms portal round-trip budget is missed) is a **distinct, transient**
condition and must NOT be collapsed into `Mode::Default`/`Unspecified`: the caller (the poll in
R4) keeps the last-known `SystemScheme` on `Err` rather than overwriting it, per FR-021.

**Alternatives considered**: Per-OS crates / raw `winit` — winit's `ThemeChanged` exists but iced
0.13 does not surface it (see R4), and per-OS code would violate the "no `cfg(target_os)` in core"
constraint. ~~`dark-light` 2.x adds `detect() -> Result` and a `subscribe()` stream but is not the
pinned version; staying on 1.1.1 avoids an unnecessary churn and the polling approach (R4) works
regardless.~~ **Superseded**: `Cargo.toml` in fact pins `dark-light = "2.0"`; this analysis was
written against a 1.1.1 assumption that was never actually adopted. `subscribe()` in 2.x remains
unused (R4's polling approach stands), but the fallible `detect()` this version actually has must
be handled per the Rationale above.

**Bugfix**: 2026-07-21 — BUG-001 corrected R3 to describe the actually-pinned `dark-light = "2.0"`
(fallible `detect() -> Result`) instead of the stale "infallible 1.1.1" premise, and decided that
a detection `Err` is distinct from a genuine `Mode::Default`/`Unspecified` (see FR-021).

## R4 — Live OS-theme-change updates

**Decision**: Detect OS scheme changes with a custom iced `Subscription` that polls
`dark_light::detect()` on a low-frequency interval (`iced::time::every(Duration)`), comparing to
the last-seen scheme in state and emitting `SystemThemeChanged(scheme)` only when it changes.

**Rationale**: iced 0.13 emits **no** theme-changed event — `iced_core` `Event` is only
`Keyboard | Mouse | Window | Touch`, and `window::Event` has no theme variant
(`iced_core-0.13.2/src/{event.rs,window/event.rs}`). `dark-light` 1.1.1 has no `subscribe()`
stream. Polling is therefore the idiomatic 0.13 path; a sub-second (~500 ms) interval keeps
worst-case latency comfortably under SC-003 ("within 1 second") at negligible cost, and emitting
only on change prevents render churn/flicker.
The poll only drives the theme while the preference is FollowSystem; a fixed override ignores it.

**Alternatives considered**: Adopt `dark-light` 2.x `subscribe()` and wrap it via
`Subscription::run` — cleaner but a version bump for marginal benefit at v1; revisit if polling
proves insufficient. Reacting to a winit theme event — not exposed by iced 0.13.

**Bugfix**: 2026-07-21 — BUG-001: this design assumed the only source of a changed reading is a
genuine OS change; it did not anticipate `dark_light::detect()` itself returning a transient
`Err` under CPU load (see R3). The poll must map `Err` to "no change" (retain the last-known
`SystemScheme`) rather than dispatching a phantom `SystemThemeChanged`; see FR-021 and tasks
T020/T021 (reopened).

## R5 — Where the theme preference is persisted

**Decision**: A new `settings.json` in the same per-user data directory as `projects.json`,
written by a `SettingsStore` trait with a `JsonFileSettingsStore` impl that reuses `store.rs`'s
pattern (serde_json, `directories::ProjectDirs`, atomic temp-file + rename, missing/corrupt →
defaults).

**Rationale**: Keeps UI/appearance settings cleanly separate from the projects catalog (single
responsibility, mirrors the 002 separation of catalog vs. active pointer) while introducing no new
storage *backend* — same mechanism, second file. A missing or unparseable file degrades to
`Settings { theme: FollowSystem }` (FR-019), matching the catalog's recover-to-empty behavior.

**Alternatives considered**: Add a `theme` field to the existing `projects.json` `StoredCatalog`
— rejected: conflates project data with app preferences and would complicate that durable schema.
A new storage engine (sled/SQLite) — rejected: overkill and against the minimal-deps constraint.

## R6 — Design tokens as pure, testable data

**Decision**: Put all design-system *values* in a pure `tokens` core module: color roles for Light
and Dark as a plain `Rgb` struct, a typography size scale (u16 px), a spacing scale (u16), and
shape radii (u16). The GUI layer converts `Rgb` → `iced::Color` and builds the `iced::Palette`.

**Rationale**: Centralizing values in core (no iced dependency) satisfies SC-007 (one source, no
per-widget magic numbers) and makes them testable — notably the AA contrast invariant (R8) — under
`cargo test --no-default-features`. The GUI layer only *renders* these values.

**Alternatives considered**: Define colors directly as `iced::Color` in the GUI — rejected: pulls
tokens behind the `gui` feature, untestable in the logic core, and risks scatter.

## R7 — Build styling in-house vs. a Material component library

**Decision**: Build the Material look on iced 0.13 primitives (containers, buttons, text) via
shared style-helper functions in `ui/style.rs`; adopt no third-party Material widget crate.

**Rationale**: The spec's non-goal explicitly leaves this open, and Constitution Principle V
permits iced only — a competing widget toolkit is disallowed and unnecessary. Material 3 surfaces,
elevation, button variants (filled/outlined/text), and list items are expressible with iced
`container`/`button` styling closures (`Fn(&Theme, Status) -> Style`, with `button::Status ∈
{Active, Hovered, Pressed, Disabled}` for the interactive states in FR-014).

**Alternatives considered**: A Material crate on top of iced — rejected on maintenance-health/
license vetting grounds and the single-stack principle.

## R8 — Accessibility contrast (SC-005) as a testable invariant

**Decision**: Choose the Material light/dark palettes so every `on_*` foreground role meets WCAG
AA contrast (≥ 4.5:1 for normal text, ≥ 3:1 for large text) against its paired surface, and encode
that as a pure unit test over the token values (WCAG relative-luminance formula, no iced needed).

**Rationale**: Turns a qualitative "legible" requirement into a compile-plus-test guarantee
(Principle I), and pins the palettes so a later color tweak that breaks contrast fails CI.

**Alternatives considered**: Manual visual inspection only — rejected: not repeatable, not a gate.

## R9 — Typography scale via iced fonts

**Decision**: Set an app default font with the builder's `.default_font(Font)` and express the
type scale (display/headline/title/body/label) as per-widget `.size(n)` values plus a small set of
`Font { weight, ..DEFAULT }` weights, all sourced from the `tokens` scale.

**Rationale**: iced 0.13 `Application::default_font` sets the base font; `iced::Font` carries a
configurable `weight` (`Weight ∈ Thin..Black`); the builder has no global `default_text_size`
setter, so the scale is applied per widget from the token constants. Keeps typography centralized
in tokens (R6).

**Alternatives considered**: Bundling a custom typeface via `.font(bytes)` — deferred; the system
default font is sufficient for v1 and avoids licensing/embedding scope. The scale, not the
typeface, is what the design system requires.
