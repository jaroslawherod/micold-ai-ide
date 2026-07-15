# Data Model: Material Design Layout & Theming

All types below live in the render-free core (no iced dependency) unless noted. Enums are used
so invalid states are unrepresentable (Constitution Principle V).

## Entities

### `ThemePreference` (core, `theme.rs`)

The user's persisted choice of how the app selects its color scheme.

| Variant        | Meaning                                                        |
|----------------|----------------------------------------------------------------|
| `FollowSystem` | Track the OS light/dark preference (default).                  |
| `Light`        | Force the light scheme regardless of the OS.                   |
| `Dark`         | Force the dark scheme regardless of the OS.                    |

- Derives: `Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize`.
- `Default` → `FollowSystem` (FR-005, FR-019).

### `ColorScheme` (core, `theme.rs`)

A concrete, resolved scheme. Exactly two values — there is no "unset" at this level.

| Variant | Meaning              |
|---------|----------------------|
| `Light` | Light Material theme |
| `Dark`  | Dark Material theme  |

- Derives: `Debug, Clone, Copy, PartialEq, Eq`.

### `SystemScheme` (core, `theme.rs`)

What the OS reports, including "no preference". Mirrors `dark_light::Mode` without depending on
that crate (the binary maps `Mode → SystemScheme` at the boundary).

| Variant       | Meaning                                                  |
|---------------|----------------------------------------------------------|
| `Light`       | OS prefers light.                                        |
| `Dark`        | OS prefers dark.                                         |
| `Unspecified` | OS reports no preference / undetectable (`Mode::Default`).|

- Derives: `Debug, Clone, Copy, PartialEq, Eq`.

### `Settings` (core, `settings.rs`)

The persisted application settings document. v1 holds only the theme preference; the struct
exists so later settings extend it without a new storage concept.

- `theme: ThemePreference`
- Derives: `Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default`.

### Design tokens (core, `tokens.rs`)

Centralized design-system values. Pure data; the GUI converts them to iced types.

- `Rgb { r: u8, g: u8, b: u8 }` — a plain color; `Debug, Clone, Copy, PartialEq, Eq`.
- `Roles { background, on_background, surface, on_surface, surface_variant, on_surface_variant,
  primary, on_primary, outline, error, on_error }` — the semantic color roles, each an `Rgb`.
- `const LIGHT: Roles` and `const DARK: Roles` — the two palettes (values fixed in
  `contracts/design-tokens.md`).
- `type_scale` — named sizes in px (`u16`): `display, headline, title, body, label`.
- `spacing` — the spacing steps in px (`u16`), e.g. `xs=4, sm=8, md=16, lg=24, xl=32`.
- `shape` — corner radii in px (`u16`), e.g. `sm`, `md`, `lg`, `full`.
- `fn roles(scheme: ColorScheme) -> Roles` — selects `LIGHT`/`DARK`.

## State extension (core, `app.rs::State`)

Add two fields (both `Copy`, default-derived):

- `theme_pref: ThemePreference` — loaded from `Settings` on boot; changed by the theme menu.
- `system_scheme: SystemScheme` — last value reported by the OS poll; seeded on boot.

`State` gains a helper:

- `fn color_scheme(&self) -> ColorScheme` → `theme::resolve(self.theme_pref, self.system_scheme)`.

## Messages (core, `app.rs::Message`)

Two new variants, handled by the pure reducer:

- `ThemePreferenceChanged(ThemePreference)` — user picked Follow system / Light / Dark. Reducer
  sets `theme_pref`; the binary persists `Settings` afterward (I/O boundary).
- `SystemThemeChanged(SystemScheme)` — the OS poll observed a (changed) scheme. Reducer sets
  `system_scheme`. No persistence (transient OS state, not a user choice).

Neither message opens/closes overlays; `on_escape` is unaffected.

## Behavior / validation rules

- **Resolution** (`theme::resolve`, pure — full table in `contracts/theme-behavior.md`):
  - `Light` pref → `ColorScheme::Light`; `Dark` pref → `ColorScheme::Dark` (OS ignored).
  - `FollowSystem` + system `Light` → `Light`; + `Dark` → `Dark`; + `Unspecified` → `Light`
    (FR-018 fallback).
- **Persistence** (FR-009, FR-019): `ThemePreference` round-trips through `settings.json`; a
  missing or corrupt file yields `Settings::default()` (`FollowSystem`). Writes are atomic
  (temp-file + rename), matching `store.rs`.
- **Contrast** (SC-005): for both `LIGHT` and `DARK`, every `on_X` role vs. its `X` surface meets
  WCAG AA (≥ 4.5:1). Enforced by a token unit test.

## Persistence mapping (core ⇄ disk)

`Settings` ⇄ `settings.json` via serde. Schema (`settings_version`, `theme`) and forward-compat
rules are the durable contract in `contracts/settings-schema.md`.

## GUI mapping (binary only — not core)

- `Roles → iced::theme::Palette` and `ColorScheme → iced::Theme::custom(name, palette)` in
  `ui/style.rs`; the `.theme(|state| …)` closure calls `state.color_scheme()` then this builder.
- `dark_light::Mode → SystemScheme` at the poll boundary in `main.rs`
  (`Dark→Dark, Light→Light, Default→Unspecified`).
