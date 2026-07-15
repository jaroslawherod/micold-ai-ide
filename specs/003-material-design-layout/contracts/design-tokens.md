# Contract: Design Tokens (the design system)

Durable definition of the single design system every surface draws from (FR-002, FR-003,
SC-007). Values live in `src/tokens.rs` as pure data; the GUI converts them to iced types. If a
value changes here, `tokens.rs` and the contrast test change with it.

## Color roles

Material 3 semantic roles. Each row is one role; the two color columns are the values for the
Light and Dark schemes. Foreground (`on_*`) roles are paired with the surface they render on and
MUST meet WCAG AA contrast (≥ 4.5:1) against it — enforced by `tests/tokens.rs`.

| Role                  | Purpose                                  | Light (hex) | Dark (hex) |
|-----------------------|------------------------------------------|-------------|------------|
| `background`          | Window background                        | `#FDFCFF`   | `#1A1C1E`  |
| `on_background`       | Text/icons on background                 | `#1A1C1E`   | `#E2E2E6`  |
| `surface`             | Cards / app bar / dialogs                | `#FFFFFF`   | `#212426`  |
| `on_surface`          | Primary text on surfaces                 | `#1A1C1E`   | `#E2E2E6`  |
| `surface_variant`     | List rows / subtle fills / dividers base | `#EEF0F4`   | `#2B2F31`  |
| `on_surface_variant`  | Secondary text (paths, captions, badges) | `#43474E`   | `#C3C7CF`  |
| `primary`             | Filled primary actions, accents          | `#005DB8`   | `#A6C8FF`  |
| `on_primary`          | Text/icons on `primary`                  | `#FFFFFF`   | `#00325B`  |
| `outline`             | Outlined-button borders, focus rings     | `#73777F`   | `#8D9199`  |
| `error`               | Error text / danger actions              | `#BA1A1A`   | `#FFB4AB`  |
| `on_error`            | Text/icons on `error`                    | `#FFFFFF`   | `#690005`  |

> Values are the reference set; implementation may adjust *only* if the contrast test still
> passes. `on_background`/`on_surface` pair with `background`/`surface`; `on_surface_variant`
> pairs with `surface_variant`; `on_primary` with `primary`; `on_error` with `error`.

### Mapping to `iced::theme::Palette`

iced's base `Palette` has five fields; map: `background ← background`, `text ← on_background`,
`primary ← primary`, `danger ← error`, `success ← primary` (no success role in this UI; reuse
primary). Exact `surface`/`on_surface`/`outline` control comes from per-widget style helpers, not
the generated palette.

## Typography scale (px)

| Token      | Size | Weight   | Used for                                  |
|------------|------|----------|-------------------------------------------|
| `display`  | 32   | Semibold | Large empty-state / dialog headline        |
| `headline` | 24   | Semibold | Active-project name, section headers        |
| `title`    | 18   | Medium   | App-bar title, list-item primary text       |
| `body`     | 14   | Normal   | Default body text, descriptions              |
| `label`    | 12   | Medium   | Paths, captions, badges (e.g. "git")         |

Default font: the iced default font set via `.default_font(...)`; weights via
`iced::Font { weight, ..Font::DEFAULT }`.

## Spacing scale (px)

`xs = 4`, `sm = 8`, `md = 16`, `lg = 24`, `xl = 32`. All padding/gaps use these steps — no
per-widget magic numbers (SC-007).

## Shape / corner radii (px)

`sm = 8` (buttons, badges), `md = 12` (cards / list items / surfaces), `lg = 16` (dialogs),
`full = 9999` (pills, if used).

## Button variants (FR-015)

| Variant    | Container                          | Label color   | Use for                          |
|------------|------------------------------------|---------------|----------------------------------|
| `filled`   | `primary` fill, radius `sm`        | `on_primary`  | The single primary action        |
| `outlined` | transparent, `outline` 1px border  | `primary`     | Secondary actions                |
| `text`     | transparent, no border             | `primary`     | Low-emphasis / inline actions    |

### Interactive states (FR-014)

For each variant, derive states from `button::Status ∈ {Active, Hovered, Pressed, Disabled}`:

- `Hovered` / `Pressed`: overlay the label color onto the container at ~8% / ~12% opacity
  (state layer); outlined/text gain a faint `primary` tint fill.
- `Disabled`: container and label at ~38% opacity of their `Active` values.
- `Active`: the base spec above.

## Elevation

Surfaces are distinguished by `surface` vs. `background` color plus a subtle 1px `outline`-tinted
border (and optional low-alpha shadow where iced supports it). No numeric elevation tokens in v1;
elevation is expressed through the surface color roles above.
