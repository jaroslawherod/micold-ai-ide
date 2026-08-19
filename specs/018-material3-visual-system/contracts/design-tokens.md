# Contract: Design Tokens (the Material 3 design system)

**Supersedes** `specs/003-material-design-layout/contracts/design-tokens.md`. That contract carried
the design system's *inputs* but deferred elevation entirely ("No numeric elevation tokens in v1"),
carried typography as five bare pixel sizes, and defined state layers for buttons only. This
document replaces it in full. Where feature 003's contract and this one disagree, this one wins.

Durable definition of the single design system every surface draws from. All values live in the
render-free core as pure data (feature 017), so the contrast invariant (FR-004), the type scale, the
elevation levels, the shape scale, the state-layer opacities and the motion tokens are all
exercised by `cargo test --workspace`. Only the conversion of these values into
rendering types is GUI-only. If a value changes here, the core and its tests change with it.

Dimensions are given in Material's density-independent pixels and are applied as the equivalent
count of logical pixels (see spec Assumptions).

---

## 1. Color

### 1.1 How roles are produced

Every color role is a **palette + tone** pair, not an independently chosen hex value (FR-005a, D1).

- One **seed color** produces the key palettes. The seed is **`#6750A4`** — Material 3's own
  baseline seed. The resulting role set is therefore the *published M3 baseline scheme*, and every
  value in §1.2 can be checked against a Material reference rather than against this project's
  judgement. This retires the previous brand blue (`#005DB8` light / `#A6C8FF` dark): the
  application's accent color becomes the baseline purple. That identity change is deliberate and
  accepted (FR-005b).
- Each palette is a **tonal ramp** with the Material tone stops
  `0, 4, 6, 10, 12, 17, 20, 22, 24, 30, 40, 50, 60, 70, 80, 87, 90, 92, 94, 95, 96, 98, 100`,
  where tone 0 is black and tone 100 is white.
- Six palettes are derived: `primary`, `secondary`, `tertiary`, `error`, `neutral`,
  `neutral_variant`.
- Both schemes read the **same ramps at different tones** (D3). Light and dark are therefore
  structurally locked together: a role added once is correct in both, and its contrast follows from
  the tone delta rather than from hand-tuning.

The ramps are checked in as core data. No build-time generation step and no runtime color-science
computation is introduced.

### 1.2 Role → tone map

The single normative table. Light and dark columns are tone values on the named palette.

| Role                          | Palette          | Light tone | Dark tone |
|-------------------------------|------------------|-----------:|----------:|
| `primary`                     | primary          | 40         | 80        |
| `on_primary`                  | primary          | 100        | 20        |
| `primary_container`           | primary          | 90         | 30        |
| `on_primary_container`        | primary          | 10         | 90        |
| `secondary`                   | secondary        | 40         | 80        |
| `on_secondary`                | secondary        | 100        | 20        |
| `secondary_container`         | secondary        | 90         | 30        |
| `on_secondary_container`      | secondary        | 10         | 90        |
| `tertiary`                    | tertiary         | 40         | 80        |
| `on_tertiary`                 | tertiary         | 100        | 20        |
| `tertiary_container`          | tertiary         | 90         | 30        |
| `on_tertiary_container`       | tertiary         | 10         | 90        |
| `error`                       | error            | 40         | 80        |
| `on_error`                    | error            | 100        | 20        |
| `error_container`             | error            | 90         | 30        |
| `on_error_container`          | error            | 10         | 90        |
| `background`                  | neutral          | 98         | 6         |
| `on_background`               | neutral          | 10         | 90        |
| `surface`                     | neutral          | 98         | 6         |
| `on_surface`                  | neutral          | 10         | 90        |
| `surface_dim`                 | neutral          | 87         | 6         |
| `surface_bright`              | neutral          | 98         | 24        |
| `surface_container_lowest`    | neutral          | 100        | 4         |
| `surface_container_low`       | neutral          | 96         | 10        |
| `surface_container`           | neutral          | 94         | 12        |
| `surface_container_high`      | neutral          | 92         | 17        |
| `surface_container_highest`   | neutral          | 90         | 22        |
| `surface_variant`             | neutral_variant  | 90         | 30        |
| `on_surface_variant`          | neutral_variant  | 30         | 80        |
| `outline`                     | neutral_variant  | 50         | 60        |
| `outline_variant`             | neutral_variant  | 80         | 30        |
| `inverse_surface`             | neutral          | 20         | 90        |
| `inverse_on_surface`          | neutral          | 95         | 20        |
| `inverse_primary`             | primary          | 80         | 40        |
| `scrim`                       | neutral          | 0          | 0         |
| `shadow`                      | neutral          | 0          | 0         |

`scrim` and `shadow` are pure black in both schemes; their visible strength comes from the alpha at
which they are drawn (§4, §7), not from a tone difference.

### 1.3 Contrast obligations (FR-004, FR-005)

The following (foreground, background) pairs carry text or icons and MUST meet **WCAG AA ≥ 4.5:1**
in both schemes. The automated test asserts every row and fails the build on any violation.

| Foreground                | Backgrounds it must clear                                                                                              |
|---------------------------|------------------------------------------------------------------------------------------------------------------------|
| `on_background`           | `background`                                                                                                            |
| `on_surface`              | `surface`, `surface_dim`, `surface_bright`, `surface_container_lowest`, `surface_container_low`, `surface_container`, `surface_container_high`, `surface_container_highest` |
| `on_surface_variant`      | `surface_variant`, and every `surface_container_*` level                                                                 |
| `on_primary`              | `primary`                                                                                                               |
| `on_primary_container`    | `primary_container`                                                                                                     |
| `on_secondary`            | `secondary`                                                                                                             |
| `on_secondary_container`  | `secondary_container`                                                                                                   |
| `on_tertiary`             | `tertiary`                                                                                                              |
| `on_tertiary_container`   | `tertiary_container`                                                                                                    |
| `on_error`                | `error`                                                                                                                 |
| `on_error_container`      | `error_container`                                                                                                       |
| `inverse_on_surface`      | `inverse_surface`                                                                                                       |
| `inverse_primary`         | `inverse_surface`                                                                                                       |
| `primary`                 | `surface`, `surface_container_low`, `surface_container` (text/outlined buttons and links draw `primary` on a surface)    |
| `error`                   | `surface`, `surface_container` (error helper text)                                                                       |
| each tag's text tone      | that same tag's fill tone, for all 11 tags (§1.4)                                                                        |

`outline`, `outline_variant`, `scrim` and `shadow` carry no text and are exempt from the text
contrast obligation. `outline` MUST still meet the non-text 3:1 threshold against the surfaces it
divides.

### 1.4 Worktree tag and issue tag colors (FR-006, FR-006a)

The ten per-type worktree tags plus the neutral issue tag are **palette-and-tone pairs like every
other role** — not hand-tuned values outside the system. Each type is assigned one fixed hue, and
its fill and text read that hue at the *same tone recipe the accent roles use* (§1.2):

| Scheme | Fill tone | Text tone |
|--------|----------:|----------:|
| Light  | 40        | 100       |
| Dark   | 80        | 20        |

| Tag        | Hue        |
|------------|------------|
| `feat`     | green      |
| `fix`      | red        |
| `chore`    | brown      |
| `docs`     | teal       |
| `refactor` | deep purple|
| `test`     | blue       |
| `build`    | orange     |
| `ci`       | indigo     |
| `perf`     | pink       |
| `style`    | lime       |
| `issue`    | neutral    |

Consequences, all intended:

- **AA is structural.** Because every tag uses the same tone delta as `primary`/`on_primary`, it
  clears AA by construction, in both schemes and under the hover / pressed / selected state layers
  of §5 (FR-024) — rather than needing 10 × 2 × 3 hand-verified checks.
- **`on_tag` is no longer one shared color.** Each tag carries its own text tone (100 light /
  20 dark) from its own hue, replacing feature 003's single `on_tag` value.
- **Tag values shift from what ships today.** Feature 003's hand-picked Material 2 shades
  (`#1B5E20`, `#B71C1C`, …) are retired. What FR-006 preserves is the *per-type distinguishability*,
  not the specific hex values.
- `refactor`'s deep purple sits near the new baseline-purple accent; it must remain distinguishable
  from `primary` in both schemes, which the tag/accent tone separation and the differing hue angle
  provide.

### 1.5 Outline discipline (FR-002, FR-003)

An outline may be drawn in exactly three situations:

1. **Divider** — a `outline_variant` hairline separating content within a surface.
2. **Outlined component** — the border of an outlined button, outlined text field or outlined chip,
   drawn in `outline`.
3. **Focus indicator** — see §5.

Any other container border is a defect. Container definition comes from surface tone (§1.2) and
elevation (§4).

---

## 2. Typography

### 2.1 Shipped typeface (FR-008, FR-008a, FR-009, D2)

**Roboto**, **SIL Open Font License 1.1**, shipped as two static instances:

| File                | Weight | Used by                                   |
|---------------------|-------:|-------------------------------------------|
| `Roboto-Regular`    | 400    | display, headline, body roles             |
| `Roboto-Medium`     | 500    | title medium/small, all label roles       |

No variable font ships. Weights 400 and 500 are the only weights the Material 3 type scale
specifies, so two static instances express every role faithfully at the smallest binary cost.

The typeface ships alongside the existing `MaterialSymbolsOutlined.ttf` under the same provenance
standard: an in-repo licence text and a `PROVENANCE.md` recording upstream source, the exact
artifact, and how it was produced.

It does **not** share the icon font's licence file. Material Symbols is Apache-2.0; Roboto is under
the OFL, which carries obligations Apache-2.0 does not, so it ships its own verbatim
`assets/fonts/LICENSE-Roboto-OFL.txt`. `PROVENANCE.md` holds the font→licence mapping.

> This section previously said Apache-2.0, which was true historically. Google relicensed Roboto to
> the SIL OFL — `google/fonts`'s `METADATA.pb` for the family records `license: "OFL"` — and the
> shipped licence follows the font. Corrected when the font was vendored (T014a).

The embedded terminal is exempt (FR-012): it keeps its monospaced font and its own grid metrics.
Text whose characters fall outside Roboto's coverage falls back rather than rendering
missing-glyph boxes (FR-013).

### 2.2 Type scale (FR-007)

Fifteen roles. Every text site selects one of these by name (FR-010). Sizes and line heights in dp;
weight is the CSS numeric weight.

| Role             | Size | Line height | Weight | Tracking (recorded, not applied) |
|------------------|-----:|------------:|-------:|---------------------------------:|
| `display_large`  | 57   | 64          | 400    | −0.25                            |
| `display_medium` | 45   | 52          | 400    | 0                                |
| `display_small`  | 36   | 44          | 400    | 0                                |
| `headline_large` | 32   | 40          | 400    | 0                                |
| `headline_medium`| 28   | 36          | 400    | 0                                |
| `headline_small` | 24   | 32          | 400    | 0                                |
| `title_large`    | 22   | 28          | 400    | 0                                |
| `title_medium`   | 16   | 24          | 500    | +0.15                            |
| `title_small`    | 14   | 20          | 500    | +0.10                            |
| `body_large`     | 16   | 24          | 400    | +0.50                            |
| `body_medium`    | 14   | 20          | 400    | +0.25                            |
| `body_small`     | 12   | 16          | 400    | +0.40                            |
| `label_large`    | 14   | 20          | 500    | +0.10                            |
| `label_medium`   | 12   | 16          | 500    | +0.50                            |
| `label_small`    | 11   | 16          | 500    | +0.50                            |

### 2.3 Accepted fidelity gap — tracking (FR-042)

The rendering stack cannot express letter-spacing. The tracking column above is **recorded so the
gap is explicit and auditable, and is not applied at render time.** This is a known, accepted
fidelity gap, not a defect, and it is the only Material 3 type-scale property this system does not
honor.

### 2.4 Sidebar density roles (FR-011)

The worktree sidebar's deliberate ~80% density reduction survives as explicit, named,
sidebar-scoped roles — not as an implicit re-derivation at call sites and not by silent loss. Each
maps to the nearest smaller role in the scale rather than inventing a new size:

| Sidebar role       | Resolves to    | Replaces (feature 003) | Rationale                                 |
|--------------------|----------------|------------------------|-------------------------------------------|
| `sidebar_name`     | `body_small`   | `sidebar::NAME` = 11   | 12/16 ≈ 80% of `body_medium`'s 14         |
| `sidebar_session`  | `body_small`   | `sidebar::SESSION` = 11| same role as the worktree name it nests under |
| `sidebar_tag`      | `label_small`  | `sidebar::TAG` = 10    | 11/16 ≈ 80% of `label_medium`'s 12        |

The sidebar's density decision is thereby preserved as one auditable mapping table rather than
three loose integers.

### 2.5 Migration from the feature 003 sizes

| Feature 003 token   | Size | Replaced by      |
|---------------------|-----:|------------------|
| `type_scale::DISPLAY`  | 32 | `headline_large` |
| `type_scale::HEADLINE` | 24 | `headline_small` |
| `type_scale::TITLE`    | 18 | `title_large`    |
| `type_scale::BODY`     | 14 | `body_medium`    |
| `type_scale::LABEL`    | 12 | `label_medium`   |

Note the deliberate re-anchoring: feature 003's `DISPLAY` (32) was a headline in Material's
vocabulary, not a display. Material's true display roles (36–57) are larger than anything this app
currently renders; they exist in the scale but no call site is required to use them.

---

## 3. Shape (FR-018, FR-019)

| Token         | Radius | Assigned to                                                       |
|---------------|-------:|-------------------------------------------------------------------|
| `none`        | 0      | full-bleed regions, the terminal grid                              |
| `extra_small` | 4      | menus, context menus, snackbars, filled text field top corners      |
| `small`       | 8      | small containers, tooltips                                         |
| `medium`      | 12     | cards, list surfaces, popovers                                      |
| `large`       | 16     | large containers, the sidebar panel                                 |
| `extra_large` | 28     | **dialogs**                                                         |
| `full`        | 9999   | **buttons, chips, tags, icon buttons** (pill)                       |

Changes from feature 003: buttons move from `small` (8) to `full`; dialogs move from `lg` (16) to
`extra_large` (28); `none`, `extra_small` and `large` are new.

---

## 4. Elevation (FR-014, FR-015, FR-016, FR-017)

Six levels. Each level carries **both** a tonal surface role and a drop shadow — the tonal shift is
what makes elevation read in the dark scheme, where shadow is nearly invisible (FR-016).

**One shadow per level.** The rendering stack exposes a single shadow per widget (research R1), so
Material's separate key and ambient shadows are folded into one: the key shadow's offset, with the
blur widened to stand in for the ambient spread.

| Level | Tonal surface role          | Shadow offset-y | Shadow blur | Shadow alpha (light) | Shadow alpha (dark) |
|-------|-----------------------------|----------------:|------------:|---------------------:|--------------------:|
| 0     | `surface`                   | —               | —           | none                 | none                |
| 1     | `surface_container_low`     | 1               | 4           | 0.30                 | 0.45                |
| 2     | `surface_container`         | 2               | 7           | 0.30                 | 0.45                |
| 3     | `surface_container_high`    | 4               | 10          | 0.30                 | 0.45                |
| 4     | `surface_container_high`    | 6               | 12          | 0.30                 | 0.45                |
| 5     | `surface_container_highest` | 8               | 15          | 0.30                 | 0.45                |

Shadows are drawn in the `shadow` role (black) at the stated alpha. The tonal surface role is never
dropped — it is what makes a level read in the dark scheme, where a black shadow on a dark
background is nearly invisible (FR-016). The dark-scheme alpha is higher only so the shadow is not
entirely lost; the tonal shift remains the primary depth cue there.

Overlapping elevated surfaces render in level order, each keeping its own shadow (FR-017).

**Level assignment**

| Surface                        | Level |
|--------------------------------|------:|
| window background, page content| 0     |
| app bar at rest                | 0     |
| cards, the sidebar panel       | 1     |
| app bar when scrolled          | 2     |
| menus, context menus, popovers | 2     |
| dialogs                        | 3     |
| snackbars                      | 3     |

Every one of these replaces a 1px outline that feature 003's contract used to fake depth.

**Scrim**: modal surfaces (dialogs) draw `scrim` at **32%** alpha over everything beneath them.

---

## 5. Interaction states (FR-020, FR-021, FR-022, FR-023, FR-024)

State layers are the content color composited over the container at the stated opacity. Defined
once and applied to **every** interactive surface — list rows, tree items, menu items, chips, tags,
buttons of every variant, text fields and the select control — not buttons alone.

| State              | Opacity | Notes                                                            |
|--------------------|--------:|------------------------------------------------------------------|
| `hover`            | 0.08    |                                                                   |
| `focus`            | 0.10    | accompanies, and does not replace, the focus indicator below       |
| `pressed`          | 0.10    |                                                                   |
| `dragged`          | 0.16    |                                                                   |
| `selected`         | 0.12    | persistent; distinct from hover, and composable with it            |
| `disabled_content` | 0.38    | applied to text and icons                                         |
| `disabled_container`| 0.12   | applied to the container fill                                     |

**Focus indicator (FR-022, FR-043)**: every element that *can* hold keyboard focus draws a **3dp
`secondary` outline** at its own shape radius when focused, in addition to the focus state layer.
It is visible without the pointer being over the element and remains distinguishable when the
element is simultaneously hovered.

That set is **text fields and the select control only**. Buttons, list rows, tree items, menu items
and chips cannot hold focus in the rendering stack — their status model has no focused state
(research R4) — and the application has no keyboard traversal between them. This is accepted
fidelity gap #2 (FR-043), recorded here so the `focus` state-layer opacity above is understood to
apply only where focus is reachable.

**Disabled content (FR-023)**: the existing behavior carries forward, including the case where a
self-coloring icon glyph cannot inherit its disabled parent's text color and must be dimmed to
`disabled_content` explicitly rather than by inheritance.

### 5.1 Ripple (FR-024a – FR-024e)

Material's press indication. Without it, a press is a flat color swap — one of the loudest reasons
the interface does not read as Material.

| Property        | Value                                                                 |
|-----------------|-----------------------------------------------------------------------|
| Origin          | the pointer's position within the element; **center** if no position is known |
| Radius (start)  | 0                                                                     |
| Radius (end)    | far enough to cover the element from the origin — the distance to its furthest corner |
| Color           | the element's state-layer content color                               |
| Opacity         | the `pressed` opacity (0.10), fading to 0                             |
| Clip            | the element's own shape, including its corner radius                  |
| Expand duration | `medium_2` (300 ms), `standard_decelerate`                            |
| Fade duration   | `short_4` (200 ms), `standard`                                        |
| Concurrency     | independent per element; pressing a second element does not disturb the first |
| At rest         | no ripple state retained; the animation clock idles                   |

**Composition.** The rendering stack's button exposes no press position, so the ripple is composed
by the shared component wrapper (feature 017): a pointer-area supplies the press point, and the ripple
is drawn beneath the element's content and above its container. Drawing uses the canvas facility,
which is already enabled and already used in this codebase for the terminal, so no new dependency
or rendering capability is required.

**State ownership (FR-024e).** Which element is rippling, from where, and how far along it is, is
*decision logic* — so it lives in the render-free core alongside the notification queue, not in the
styling layer. Only the drawing is rendering-specific. Per-element identity follows the existing
per-instance animation-key pattern already used for row hover fades.

**Contrast under state (FR-024)**: content on accent-colored and tag-colored surfaces must remain
AA-compliant in the hover, pressed and selected states. The contrast test covers these composited
pairs, not only the resting pair.

---

## 6. Motion (FR-033, FR-034, FR-035)

### 6.1 Durations (ms)

| Token       | Value | Token        | Value | Token      | Value |
|-------------|------:|--------------|------:|------------|------:|
| `short_1`   | 50    | `medium_1`   | 250   | `long_1`   | 450   |
| `short_2`   | 100   | `medium_2`   | 300   | `long_2`   | 500   |
| `short_3`   | 150   | `medium_3`   | 350   | `long_3`   | 550   |
| `short_4`   | 200   | `medium_4`   | 400   | `long_4`   | 600   |

### 6.2 Easing curves

**Standard set** — small, utilitarian transitions:

| Token                 | Cubic bézier          |
|-----------------------|-----------------------|
| `standard`            | (0.2, 0, 0, 1)        |
| `standard_accelerate` | (0.3, 0, 1, 1)        |
| `standard_decelerate` | (0, 0, 0, 1)          |

**Emphasized set** — larger, more expressive transitions:

| Token                   | Cubic bézier            |
|-------------------------|-------------------------|
| `emphasized`            | (0.2, 0, 0, 1)          |
| `emphasized_accelerate` | (0.3, 0, 0.8, 0.15)     |
| `emphasized_decelerate` | (0.05, 0.7, 0.1, 1)     |

### 6.3 Assignment

Every animated behavior already in the app keeps its trigger, start state and end state; only
duration and easing change (FR-035). The last **four** rows are **new** animations introduced by
this feature's own new surfaces — the app bar's elevation transition (FR-025a), the snackbar's
enter/exit (FR-032), the indeterminate progress indicator (FR-031f) and the press ripple
(FR-024a) — not changes to existing behavior. Four is the count FR-035a and SC-010 both carry, and
no fifth animation is permitted.

| Animation           | Duration           | Easing                  | Set        |
|---------------------|--------------------|-------------------------|------------|
| overlay fade in     | `medium_2` (300)   | `emphasized_decelerate` | emphasized |
| overlay fade out    | `short_4` (200)    | `emphasized_accelerate` | emphasized |
| sidebar slide       | `medium_4` (400)   | `emphasized`            | emphasized |
| menu fade in        | `short_3` (150)    | `standard_decelerate`   | standard   |
| menu fade out       | `short_2` (100)    | `standard_accelerate`   | standard   |
| row hover fade      | `short_2` (100)    | `standard`              | standard   |
| app bar elevate     | `short_4` (200)    | `standard`              | standard   |
| snackbar enter      | `medium_1` (250)   | `emphasized_decelerate` | emphasized |
| snackbar exit       | `short_4` (200)    | `emphasized_accelerate` | emphasized |
| progress indeterminate | `long_2` (500)  | `standard`              | standard   |
| ripple expand       | `medium_2` (300)   | `standard_decelerate`   | standard   |
| ripple fade         | `short_4` (200)    | `standard`              | standard   |

---

## 7. Component anatomy (FR-025 – FR-032)

Every component below is reused from or extended within the shared component library and exposed
through its chainable builder API (feature 017). No bespoke one-off widget.

### 7.1 Top app bar (FR-025, D4)

**Small variant only.** The medium and large variants are not adopted.

| Property           | Value                                   |
|--------------------|-----------------------------------------|
| Height             | 64                                      |
| Horizontal padding | 16 (4 before a leading icon button)     |
| Surface at rest    | `surface`, elevation 0                  |
| Surface on scroll  | elevation 2 tonal + shadow              |
| Scroll signal      | the worktree sidebar's scroll offset — the only scroll region beneath the bar (FR-025a) |
| Title role         | `title_large`, color `on_surface`       |
| Icon color         | `on_surface_variant`                    |
| Icon target        | 48 × 48                                 |
| Action control     | a shared button: `IconButton` at the target above where it carries no label, the text button with §7.3's leading-icon slot where it does — never one assembled at the call site (FR-029c) |
| Divider at rest    | 1, `outline_variant`; absent once elevated |
| Bottom edge        | height + divider = **65** — the offset any panel anchored below the bar derives from (FR-029a) |

**Why the bottom edge is a row.** The height is stated above and the divider is drawn by the same
component, so what a surface hanging *below* the bar has to clear is neither figure alone. Nothing
said so, and two components each answered it by eye with the same wrong number (BUG-003). A panel's
offset is not that panel's own anatomy; it is this row, read.

### 7.2 List and tree rows (FR-026, FR-026a)

**Two named densities.** Every row uses one of these; no list invents a third.

Both figures below are **minimum** row heights, not fixed ones. A row is free to be taller than its
density when what it holds is taller — a worktree row's tag chips, a wrapped label — and is never
clipped to the figure. A row is never *shorter* than it. This distinction is the whole of BUG-005:
read as a fixed height, the one-line figure conflicts with the visible-count clause below; read as a
minimum, it does not, and a two-line row keeps the height its own second line asks for.

**One-line rows** — Material 3's single-line list item, carried down the density scale:

| Density    | Min row height | Horizontal padding | Used by                                    |
|------------|---------------:|-------------------:|--------------------------------------------|
| `standard` | 56             | 16                 | known-projects list, all other lists       |
| `dense`    | 44             | 8                  | the worktree sidebar tree                  |

**Two-line rows** — Material 3's two-line list item, for a row carrying a second line beneath its
name (the sidebar's tagged worktree rows, feature 008 FR-001):

| Density    | Min row height |
|------------|---------------:|
| `standard` | 72             |
| `dense`    | 60             |

**Where the four numbers come from.** Material 3 specifies the list item at 56dp for one line, 72dp
for two and 88dp for three. Material's density scale is a separate, generic axis — four steps
0, −1, −2, −3, and `height = base + 4dp × step` — so the `dense` column is the `standard` one at
step −3, which is what FR-026b and FR-026c require and what `density::height` computes. Nothing here
is invented: the base is Material's list spec and the step is Material's density scale. Every figure
is a multiple of 4, so no row resolves to a fractional height.

**Superseded, and why.** ~~`standard` 48 / `dense` 36~~ were Material **2**'s single-line list item
(48dp) on the same density scale, in a feature whose subject is Material 3. Corrected 2026-08-07
under BUG-005, on the owner's decision that §7.2 follows Material 3's list specs.

**The cost, stated rather than discovered.** The `dense` density exists to preserve the sidebar's
deliberate compactness (feature 009, FR-011): before this feature the sidebar's rows sat at a ~28dp
pitch — 23.6dp of content plus the column's 4dp gap, which is a *pitch* and not a row height, and
the two must not be compared as though they were. At the figures above a one-line row's pitch
becomes 48dp and a tagged row's 64dp — 74% and 40% more vertical space each, so between a quarter
and two-fifths fewer worktrees fit without scrolling depending on how many carry tags. That is a
material decrease, and FR-026a is amended to permit it rather than left to be violated quietly:
where Material's published figures and the visible-count clause conflict, this contract now resolves
it in Material's favour, which is the opposite of the resolution BUG-005 was reported against. See
FR-026a and the BUG-005 note in `spec.md`.

The **second line** row is feature 008's tag chips (FR-001). It is stated here because BUG-006 was
an indent computed as `indent + one icon + one gap` against a row that is `indent → twisty → gap →
icon → gap → label`: a different expression that happened to land within 4dp in the sidebar and 47dp
out in the gallery. A figure with nothing stating its intent is the shape all of this section's bugs
have had — §7.3 and §7.5 gained their alignment rows for the same reason.

The **leading icon slot** is fixed for the same reason the twisty's is: a glyph's advance is the
face's business, not the layout's, and `AddWorktree` measures 14dp where the role says 16. Without
the slot the label's column moves with whichever glyph a row carries, and no arithmetic below it can
be right for every row at once.

Shared by both densities:

| Property           | Value                                                       |
|--------------------|-------------------------------------------------------------|
| Leading icon gap   | 16 (standard) / 8 (dense)                                    |
| Primary text       | `body_medium` (standard) / `sidebar_name` = `body_small` (dense) |
| Supporting text    | `body_small`, color `on_surface_variant`                     |
| Second line        | starts at the label's own x — the leading run is followed, never restated |
| Leading icon slot  | fixed at the primary text's size, so the column does not follow glyph advances |
| Shape              | `full` for the selection pill; `none` for the row hit area   |
| Selected           | `secondary_container` fill + `on_secondary_container` text   |
| States             | full state-layer set (§5)                                     |

### 7.3 Buttons (FR-027)

| Property           | Filled              | Outlined                  | Text                | Icon             |
|--------------------|---------------------|---------------------------|---------------------|------------------|
| Height             | 40                  | 40                        | 40                  | 40 container     |
| Min touch target   | 48 × 48             | 48 × 48                   | 48 × 48             | 48 × 48          |
| Shape              | `full`              | `full`                    | `full`              | `full`           |
| Horizontal padding | 24                  | 24                        | 12                  | 8                |
| Container          | `primary`           | transparent, 1dp `outline`| transparent         | transparent      |
| Label / icon       | `on_primary`        | `primary`                 | `primary`           | `on_surface_variant` |
| Label role         | `label_large`       | `label_large`             | `label_large`       | —                |
| Icon size          | 18 (leading)        | 18                        | 18                  | 24               |
| Elevation          | 0                   | 0                         | 0                   | 0                |
| Label alignment    | centred in the height | centred                 | centred             | centred          |

The minimum touch target is honored even where the visible container is smaller — an icon button's
24dp glyph in a 40dp container still presents a 48dp hit area.

The label-alignment row is FR-030a applied here: 40dp of height around a 20dp `label_large` line is
20dp of slack by construction, and a height that does not say where its content sits resolves it
against the top edge. Stated for the same reason §7.6's chip row states it.

Destructive actions substitute `error` / `on_error` for `primary` / `on_primary`.

### 7.4 Dialogs (FR-028)

| Property           | Value                                        |
|--------------------|----------------------------------------------|
| Surface            | `surface_container_high`, elevation 3         |
| Shape              | `extra_large` (28)                            |
| Scrim              | `scrim` at 32%                                |
| Padding            | 24 all sides                                  |
| Icon (optional)    | 24, centered, `secondary`                     |
| Title role         | `headline_small`, `on_surface`                |
| Title → body gap   | 16                                            |
| Body role          | `body_medium`, `on_surface_variant`           |
| Body → actions gap | 24                                            |
| Action row         | trailing-aligned, 8 gap, text buttons          |
| Min width          | 280 — *recorded, not applied* (FR-046)        |
| Max width          | 560                                           |

### 7.5 Menus, context menus, popovers (FR-029)

| Property        | Value                                       |
|-----------------|---------------------------------------------|
| Surface         | `surface_container`, elevation 2             |
| Shape           | `extra_small` (4); popovers `medium` (12)    |
| Vertical padding| 8                                            |
| Item height     | 48, with the item's content centred in it    |
| Item padding    | 12 horizontal                                |
| Item label      | `label_large`, `on_surface`                  |
| Item icon       | 24, `on_surface_variant`                     |
| Between items   | nothing — items abut; a gap is what a divider is for |
| Divider         | 1dp `outline_variant`                        |
| Panel top edge  | §7.1's bottom edge, for a panel anchored below the app bar (FR-029a) |
| Panel width     | 240 for a panel anchored below the app bar; 160 for a cursor-anchored context menu (FR-029c) |
| Panel transition| §6's menu enter and exit — the panel plays both, so every panel of the kind does (FR-029c) |
| States          | full state-layer set (§5)                     |

**One row, built once (FR-029b).** Every figure in this table describes *the* menu item, not "a menu
item per panel". The overflow menu, context menus and the project switcher's project list are the
same row carrying different content, and BUG-003 is what the alternative costs: the item height was
applied to one of two hand-built copies, so two panels hanging off the same bar shipped 12dp apart.
A panel gives an item leading or trailing content; it does not rebuild one.

**And one panel, and one trigger (FR-029c).** The same sentence one level out, because BUG-003
closed the row and left the frame around it forked. The width, surface, offset and transition rows
above describe *the* panel; the control that opens one is §7.1's action control, which is a shared
button — which one depending on whether it carries a label, both figures being §7.3's either way.
BUG-007 is what the alternative costs: a hand-assembled 28dp trigger drawing its glyph at its
label's 14dp role beside a 48dp trigger drawing 24dp, a 260dp panel beside a 240dp one from the same
edge, and an exit transition that reached one of them — because the fade is written in the shared
panel and the copy predates it.

### 7.6 Chips and tags (FR-030)

| Property        | Value                                                     |
|-----------------|-----------------------------------------------------------|
| Height          | 32                                                        |
| Shape           | `full`                                                    |
| Horizontal padding | 12 (8 when a leading icon is present)                  |
| Label role      | `label_large`                                             |
| Label alignment | centred within the height; centred within the padded width (FR-030a) |
| Icon size       | 18                                                        |
| Unselected      | transparent container, 1dp `outline`, `on_surface_variant`|
| Selected        | `secondary_container` fill, `on_secondary_container` label |
| Worktree tag    | per-type fill and its paired text tone (§1.4), `label_small` in the sidebar |
| States          | full state-layer set (§5), AA preserved under each (FR-024) |

**Why alignment is a row here.** The height and the label role are stated independently above, and
32 is deliberately taller than `label_large`'s 20dp line box — so the 12dp of slack has to go
somewhere, and nothing else in this table says where. BUG-001 is what that omission produced: a
chip built to every other row of this table, with all 12dp collected beneath the label. A component
whose height is content-sized (the worktree tag, which sets no height) has no slack and no
alignment question; one with a fixed height always does.

### 7.7 Text fields and select (FR-031, FR-031a – FR-031d)

**Filled variant.** This is the largest single departure from what ships today, so the current
state is recorded alongside the target.

| Property           | Target                                                  | Today (the defect)                        |
|--------------------|---------------------------------------------------------|-------------------------------------------|
| Height             | 56                                                      | ~30 (`spacing::SM` padding around text)   |
| Container          | `surface_container_highest`                             | `surface` — same tone as the dialog behind it |
| Shape              | `extra_small` **top** corners, **square** bottom         | `small` (8) on all four corners            |
| Border             | **none**                                                | uniform 1dp box on all four sides          |
| Active indicator   | bottom edge only: 1dp `on_surface_variant`, **2dp** `primary` when focused | no indicator; focus only recolors the box border |
| Horizontal padding | 16                                                      | 8                                          |
| Label              | **inside** the container, above the value               | outside, above the field, as muted text    |
| Label role         | `body_small`, `on_surface_variant`                      | `label` size, `muted` style                |
| Input role         | `body_large`, `on_surface`                              | default size                               |
| Placeholder        | a genuine example only — never the field's name          | carries the label *and* the hint           |
| Supporting text    | `body_small`, `on_surface_variant`, beneath the container | absent                                     |
| Error state        | indicator, label and supporting text in `error`          | absent                                     |
| Select trailing    | 24 chevron, `on_surface_variant`                        | chevron present                            |
| Select dropdown    | menu surface + elevation 2 per §7.5 (FR-031d)            | `surface` with a 1dp outline               |

**Per-corner radius is required** for the rounded-top/square-bottom container and is supported by
the rendering stack (`Radius { top_left, top_right, bottom_right, bottom_left }`).

**Label placement (FR-031a, FR-044).** The rendering stack's text input has **no label concept** —
only a placeholder. The shared text field therefore composes the label as a sibling of the input
inside the container, and the container owns its own layout so it can put the label in either of
Material's two positions:

| State | Label | Position | Control |
|---|---|---|---|
| Empty and inactive | `body_large` | centred on the value's line | placeholder suppressed |
| Populated *or* active | `body_small` | 8dp from the top | on the line below |

An empty field therefore shows one word on the value's line, not two: while the label rests it *is*
the placeholder. Material's animated transition between the two positions is **not** implemented —
the label snaps. Both endpoints are correct; only the transition is absent. Accepted fidelity gap
#4.

**Adornments (BUG-003 item 1).** The row this section did not have, which is why the label and the
leading icon were positioned by two different rules and drawn on top of each other:

| Property           | Value                                                              |
|--------------------|--------------------------------------------------------------------|
| Leading icon slot  | fixed at 24, **not** the glyph's advance — §7.2's rule, BUG-006's lesson |
| Leading icon gap   | 16, between that slot and everything after it                       |
| Content column     | `padding + slot + gap` when there is a leading icon, `padding` otherwise — followed by the value **and** the label, in either of the label's positions |
| Adornment baseline | both adornments centred on the container's own middle, not on the floating value's line |
| Adornment height   | the **container's**, not the value line's — a trailing `IconButton` keeps §7.3's 48dp target rather than being squeezed into 24dp |

The content column is one figure for two things on purpose. The value was inset past the icon and
the label was pinned at the padding, so an empty unfocused field with a leading icon drew its label
underneath that icon — which is the state every searchable picker opens in. A figure with nothing
stating its intent is the shape all of this section's bugs have had; this is that figure.

The height row is the same defect one slot over. An adornment is not a second line of value, and
offering it only the 24dp value line did not refuse the icon button that wanted 48 — it squeezed it:
8dp of padding top and bottom out of 24 left the glyph an 8dp box, which it drew out of and down the
field, ~11dp below the centre line with half the target §7.3 requires. It cost nothing at the gates,
because the slot was the right size and the child fitted inside it.

**Content migration (FR-031a, FR-031b).** Today's placeholders bundle the field name and a hint
into one string. These split:

| Today's placeholder | → Label | → Supporting text |
|---------------------|---------|-------------------|
| `Ticket (optional, e.g. ABC-123)` | `Ticket` | `Optional — e.g. ABC-123` |
| `Name (e.g. login page)` | `Name` | `e.g. login page` |
| `Scrollback lines` | `Scrollback lines` | — |
| `Script path` | `Script path` | — |
| `Timeout (seconds)` | `Timeout` | `Seconds` |
| `Project name` / `Worktree name` | `Project name` / `Worktree name` | — |

The free-standing `Type` label above the select moves inside that control's container.

This is presentation only — no field changes what it accepts, validates or submits (FR-036).

**The type-ahead (added after this section was written).** Feature 021 replaced the branch picker's
select with a `Typeahead`, so the row above reading "The free-standing `Type` label above the select
moves inside that control's container" now has a sibling: the branch picker's `Branch` label moves
the same way, into the type-ahead's own container. It is the same requirement (FR-031a) reaching a
control that did not exist when the requirement was written, not a new one.

The type-ahead also settled what the select could not. §7.7 asks for the active indicator to respond
to **open** rather than focus (FR-043a), and `pick_list` reported `Opened` to its own style closure
and to no parent — so `Select::active` had to be supplied by a caller, and no caller tracked it.
`Typeahead` takes `.open(bool)` from a caller that already holds that state, so it passes it to the
field as the active flag and the indicator follows it.

**The select is a first-class control here now (feature 022, FR-032).** It is no longer the
rendering stack's `pick_list` behind a style closure: it is a widget of this library's own, floating
the same list the type-ahead floats, from the same `material::picker`. What that changes for this
section is the sentence above. Openness is the widget's own state, so nothing has to be supplied and
`Select::active` is gone from the builder entirely — the indicator answers from the control's own
knowledge of being open. **Accepted fidelity gap #3 is closed and removed from §9.**

Its open and hover states are *also* carried by the **state layer** (§5, FR-021), and both are
wanted: the indicator says which control the list belongs to, the layer says the control is being
used. The layer was written when it was standing in for an indicator that could not answer; it stays
because it is what every other interactive surface does, not because the indicator still cannot.

What remains true, and is a different thing from gap #3: this control has no **focus** state, because
nothing in this application's rendering stack gives a non-text widget one — that is gap #2 (FR-043),
which covers buttons, rows, menu items and chips alike, and the select is simply one more of them.
§7.7 asks the indicator to follow *open* rather than focus, and it does.

### 7.8 Snackbar (FR-032, FR-032a, FR-032b)

Replaces the current inline notification banner. It is a floating, elevated, self-contained
surface — not a strip in the layout.

| Property        | Value                                    |
|-----------------|------------------------------------------|
| Container       | `inverse_surface`, elevation 3            |
| Shape           | `extra_small` (4)                         |
| Min height      | 48                                        |
| Padding         | 16 horizontal, 14 vertical                |
| Message role    | `body_medium`, `inverse_on_surface`       |
| Action label    | `label_large`, `inverse_primary`          |
| Max width       | 600                                       |
| Position        | floating above content, above dialog scrim |

**Semantics (FR-032a, FR-032b).** The snackbar adopts Material's *behavior*, not only its
appearance. This is the single sanctioned exception to the no-behavior-change rule (FR-036a).

| Property           | Value                                                             |
|--------------------|-------------------------------------------------------------------|
| Concurrency        | exactly one visible; further notifications queue and show in turn  |
| Duration — info    | 4 s (Material's short duration)                                    |
| Duration — error   | 10 s (Material's long duration), so an error is not lost unread    |
| Manual dismissal   | always available; clears before the timeout elapses                |
| Deduplication      | preserved from current behavior, now applied to the queue          |
| Queue cap          | preserved from current behavior                                    |

A snackbar raised while a dialog is open renders above the dialog and must not permanently obstruct
the dialog's action row.

### 7.9 Progress indicator (FR-031e, FR-031f)

Material's **linear** progress indicator, used for worktree-creation staging.

| Property          | Value                                                     |
|-------------------|-----------------------------------------------------------|
| Track             | `secondary_container`                                     |
| Active indicator  | `primary`                                                 |
| Thickness         | 4                                                         |
| Shape             | `full` on both track and indicator                        |
| Gap to stage label| `xs` (4)                                                  |
| Stage label role  | `body_small`, `on_surface_variant`                         |

**Determinacy (FR-031f).** The application does not know how much of a worktree creation is
complete — whether the submodule stage runs at all is only known after the branch and worktree
already exist. Material's answer for that case is the **indeterminate** indicator, whose active
segment travels across the track. Today the bar is instead frozen at a fixed 40% fill, which
asserts a completion fraction the application cannot know.

| | Target | Today (the defect) |
|---|---|---|
| Determinacy | indeterminate, animated | static fill at 0.4 |
| Track role | `secondary_container` | `surface_variant` |

The indeterminate animation uses `long_2` (500 ms) on the `standard` easing, looping while the
operation runs. It is the third and final new animation this feature introduces (FR-035a) and, like
the other two, stops when the operation ends so nothing animates at rest.

---

## 8. Spacing (unchanged from feature 003)

`xs = 4`, `sm = 8`, `md = 16`, `lg = 24`, `xl = 32`. All padding and gaps use these steps. This
feature adds no new spacing steps; the component dimensions in §7 are component anatomy, not new
spacing tokens.

---

## 9. What this contract does not cover

> **Gap #3 was here and is closed** (feature 022, FR-032, SC-005). It read: *the stack's select
> reports only active, hovered and open, with no focus concept to observe, so its active indicator is
> driven by the open state instead.* The select is this library's own control now, its openness is
> its own state, and the indicator answers from it — see §7.7. The list is three entries.
>
> **The numbers do not shift up.** #4 stays #4, here and in `form_field.rs`, `state.rs` and
> `anatomy.rs`, which name these by number in prose. Renumbering would silently repoint every one of
> those at a different gap, which is a worse outcome than a list that counts 1, 2, 4.


- **Tracking / letter-spacing at render time** — recorded in §2.2, deliberately unapplied (§2.3).
  Accepted fidelity gap #1 (FR-042).
- **Keyboard focus on non-text widgets** — the rendering stack has no focused state for buttons,
  rows, menu items or chips (§5). Accepted fidelity gap #2 (FR-043).
- **The text field label's float transition** — the stack's text input has no label concept, so the
  label snaps between resting and floating rather than animating (§7.7). Accepted fidelity gap #4
  (FR-044).
- **Information architecture** — no navigation rail, no floating action button, no responsive
  breakpoints.
- **Dynamic color** — no Material You extraction from the host environment; the seed is fixed.
- **User-customizable theming** beyond follow-system / light / dark.
- **Terminal text rendering** — monospaced, own grid metrics, exempt.
