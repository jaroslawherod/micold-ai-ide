# Phase 0 Research: Sidebar Filter Toolbar Button

## R1: Where does the new popover live — sidebar header or app top toolbar?

- **Decision**: The filter trigger button lives in the sidebar's own header row
  (`src/ui/sidebar.rs`, alongside `add_worktree`/`hide`), not the app-level top toolbar
  (`src/ui/toolbar.rs`).
- **Rationale**: Tag filtering is scoped entirely to the sidebar's worktree list; every
  existing filter control (`filter_bar`, `filter_chip`) already lives in `sidebar.rs`. Placing
  the trigger there keeps the feature colocated with the state and content it controls, and
  matches the codebase's existing separation (the app toolbar only hosts project- and
  app-level actions: project switcher, overflow menu).
- **Alternatives considered**: App top toolbar (`Toolbar::action(...)`) — rejected because it
  would visually and semantically separate the trigger from the sidebar content it filters,
  and the existing `Toolbar` actions are all app/project-scoped, not list-scoped.

## R2: Overlay primitive — reuse `MenuOverlay`/`ProjectSwitcherOverlay` or add a new one?

> **Superseded by R7**: mid-implementation direction changed the presentation from a floating
> overlay to an inline accordion. `FilterOverlay` (described below) was built, then removed;
> only `FilterTrigger` remains. Kept here for the record of why a floating overlay was the
> initial design.

- **Decision**: Add one new shared primitive, `FilterTrigger`/`FilterOverlay`
  (`src/ui/material/filter_panel.rs`), following the exact trigger+overlay split and
  backdrop/stack/fade idiom of `MenuOverlay`, but built for the filter panel's own content and
  anchor point.
- **Rationale**: `MenuOverlay` renders a fixed `Vec<MenuItem<M>>` as plain list-item buttons,
  anchored top-right of the *window* (`TOP_OFFSET` below the app toolbar) by default, or at an
  arbitrary window-space point via `.anchor(Point)`. The filter panel's content is the existing
  `filter_bar()`/`filter_chip()` toggle-pill layout — a different shape than a `MenuItem` list
  — and it needs to anchor near the sidebar header (left side of the window, at the sidebar's
  own width), not the app toolbar's top-right corner. Forcing the filter content through
  `MenuItem` would lose the toggle/active-color chip styling; forking a bespoke one-off instead
  of a shared, builder-style primitive would violate Principle VIII. A small new primitive that
  reuses the same `stack![base, backdrop, panel]` + `super::fade` idiom is the closest fit.
- **Alternatives considered**:
  - Reuse `MenuOverlay` with `.anchor(point)` and pass pre-built filter-chip elements disguised
    as `MenuItem`s — rejected: `MenuItem` only carries an icon + label + one message, it cannot
    express a chip's active/inactive fill-color toggle without changing `MenuItem` itself
    (which would affect the existing overflow-menu and context-menu callers).
  - Reuse `ProjectSwitcherOverlay` — rejected for the same content-shape mismatch, plus its
    `open: bool` (no fade) is less consistent with the "slide out" requirement (FR-003) than
    `MenuOverlay`'s `progress: f32` fade, which this feature adopts instead.

## R3: Escape-to-dismiss for a non-modal popover

- **Decision**: Extend the pure `on_escape(state)` in `src/app.rs` to check
  `state.sidebar_filter_open` and return `Some(Message::SidebarFilterMenuToggled)` when true
  (before falling through to the existing `state.overlay` match, since the two are mutually
  exclusive in practice — the filter popover and a full modal `Overlay` never coexist). Mirror
  this with a new, non-capturing-closure branch in `ui/mod.rs::subscription()`, following the
  existing per-`Overlay`-variant pattern (`on_key_press` requires a non-capturing `fn`, which is
  why each dismiss target already gets its own literal closure rather than one generic
  dispatcher).
- **Rationale**: The spec (FR-006) requires Escape to dismiss the filter panel. Today, only
  full `Overlay` modals get Escape treatment; the two existing lightweight popovers
  (`help_menu_open`, `project_switcher_open`) do not (confirmed: `ui/mod.rs::subscription()`
  only branches on `state.overlay`, returning `Subscription::none()` whenever it's `None`,
  which is also true whenever a popover is open). This feature is the first lightweight popover
  to support Escape; it does not retrofit the other two, since that's out of this feature's
  scope and not requested.
- **Alternatives considered**: Leave Escape unsupported for consistency with the other two
  popovers — rejected because the spec explicitly requires it (FR-006/User Story 3), and
  outside-click + re-toggle alone were judged insufficient by the feature request.

## R4: Communicating "filter active" while the panel is closed

- **Decision**: Tint the filter trigger icon with the `primary` role color when
  `!state.sidebar_filters.is_empty()`, and `on_surface_variant` (the existing muted/inactive
  tint used by `hide`) otherwise — no badge/dot, no new `IconSurface` role needed.
- **Rationale**: Every existing icon-button in the sidebar header already communicates
  state purely through tint (`add_worktree` uses `primary`, `hide` uses
  `on_surface_variant`); reusing that convention needs no new visual language and passes the
  existing WCAG-AA icon-contrast test (`tests/icon_roles.rs`) for free, since both roles are
  already asserted there against the `surface` background the sidebar header sits on.
- **Alternatives considered**: A small numeric/dot badge showing the active filter count —
  rejected as unnecessary complexity (YAGNI) for a boolean "something is filtered" signal;
  tint alone satisfies SC-002 (glanceable without opening the panel).

## R5: Icon codepoint for the filter glyph

> **Superseded by R7**: the user directly requested the `filter_list` glyph (three
> descending-length lines) instead. `Icon::Filter` now maps to `filter_list`, `U+E152`. Kept
> here for the record of the initial choice and why it was made.

- **Decision (superseded)**: `Icon::Filter` maps to Material Symbols Outlined `filter_alt`,
  codepoint `U+EF4F`.
- **Rationale**: Verified directly against the upstream
  `google/material-design-icons` `variablefont/MaterialSymbolsOutlined[FILL,GRAD,opsz,wght].codepoints`
  manifest (`filter_alt` → `ef4f`; the alternative `filter_list` → `e152` is the classic
  "list with descending bars" glyph, more associated with sort/list-density controls than a
  filter toggle). `filter_alt` (funnel shape) is the glyph Material Design itself uses for
  filter-toggle actions.
- **Alternatives considered**: `filter_list` (`e152`) — initially rejected as a less immediately
  recognizable "filter" affordance for a toggle button; both are valid Material Symbols glyphs.
  Superseded per direct user request (R7) — `filter_list` is what actually ships.

## R6: Font asset regeneration strategy — narrow subset vs. full coverage

- **Decision**: Regenerate `assets/fonts/MaterialSymbolsOutlined.ttf` as a **full static
  instance** of the upstream variable font (every upstream codepoint, at the pinned axis
  values weight 400 / FILL 0 / GRAD 0 / opsz 24), instead of re-running the narrow
  `pyftsubset --unicodes=<curated list>` step with just `ef4f` appended.
- **Rationale**: Explicit user direction ("create full coverage for material icons in one shot
  to not do one by one") — the existing per-feature pattern (subset to exactly the icons used
  so far, re-subset on every new icon) means every future feature that adds an `Icon` variant
  must also regenerate this binary asset, a recurring toil this feature can eliminate once.
  `Icon` stays a closed, curated Rust enum either way (Principle V — invalid icons remain
  unrepresentable); only the *font's* glyph coverage changes from "exactly today's curated set"
  to "every glyph the upstream font ships", so a future `Icon` variant only needs a name +
  codepoint added in `src/icons.rs`, never a font rebuild.
  Verified feasible in this environment: the upstream variable font
  (`variablefont/MaterialSymbolsOutlined[FILL,GRAD,opsz,wght].ttf`, ~10.6 MB) is fetchable, and
  a `fonttools`-capable Python environment (`uv`-managed, with `pyftsubset`/`varLib.instancer`)
  is available to reproduce the exact `PROVENANCE.md` pipeline, this time without the
  `--unicodes` restriction (instantiate only, no subsetting by codepoint — or subset to the
  full upstream `.codepoints` list, which is equivalent).
- **Alternatives considered**:
  - Keep the narrow per-icon subset pattern (add just `ef4f` to the existing `--unicodes`
    list) — rejected per explicit user direction; keeps the recurring-toil problem for every
    future icon.
  - Ship the full **variable** font (weight/fill/grade/opsz all adjustable at runtime) —
    rejected: the app has no use for runtime axis variation (Principle VIII/theming uses fixed
    color tints, not variable glyph weight), and it's several MB larger than a single static
    instance for no behavioral benefit.
- **Size impact**: Current curated subset is 4 KB; a full static instance (all glyphs, one
  axis position) is expected in the low single-digit MB range — a one-time, acceptable
  increase for a desktop app with no distribution-size constraint in the constitution.

## R7: Mid-implementation pivot — icon, position, and accordion presentation

After User Stories 1 and 2 were implemented against the R2/R5 design (floating `FilterOverlay`,
`filter_alt` icon, trigger grouped with the other right-side header actions), the user gave
direct, specific correction:

- **Icon**: use the `filter_list` glyph (three descending-length lines), not `filter_alt` (the
  funnel). `Icon::Filter` now maps to `U+E152` (see R5's superseded note). No font regeneration
  was needed — the full-coverage font from R6 already contains this codepoint.
- **Position**: move the trigger to the left of the sidebar header, ahead of the "Worktrees"
  title, rather than grouped with `add_worktree`/`hide` on the right.
- **Presentation**: replace the floating `FilterOverlay` (backdrop + `stack!` + top-left window
  anchor, per R2) with an inline **accordion** — the filter content expands/collapses in the
  sidebar's own layout flow, below the header, pushing the worktree list down, rather than
  floating over it.

**Decision**: Implemented all three directly as instructed (this is direct user input on the
feature's design, not an inference). The accordion is built as a new shared animation
primitive, `material::expand` (`src/ui/material/animation.rs`), a vertical sibling to the
existing `slide` (horizontal, edge-anchored reveal used for the sidebar itself): `expand`
anchors to the *top* instead (never translates its child, so it always reveals top-down),
which is the natural accordion behavior and a small, well-scoped addition alongside an
existing, structurally identical primitive. `FilterOverlay` was removed from
`src/ui/material/filter_panel.rs` entirely (no more backdrop, no more `stack!`, no more
floating anchor math) — only `FilterTrigger` remains, now purely a trigger button. The
accordion's content is composed directly in `src/ui/sidebar.rs` via
`expand(container(filter_bar(state, r))..., filter_progress)`, with `filter_progress` threaded
in from `ui/mod.rs` (`motion.get(MotionKey::SidebarFilter)`) through `sidebar::view`'s
parameter list, mirroring exactly how `HandleHover`'s progress is already passed as a plain
`f32` into `sidebar::handle`.

**Consequence for dismissal (FR-006, User Story 3)**: an inline accordion has no "outside" to
click — there's no floating panel over other content, so outside-click dismissal (originally
one of three dismissal paths) no longer applies and was dropped from the spec. Escape and
re-toggling the trigger remain the two dismissal paths; both were already implemented
independently of the overlay/accordion choice (Escape via `on_escape`/`subscription()`,
toggling via the same `Message::SidebarFilterMenuToggled` reducer), so neither needed rework.

**Alternatives considered**: Keep the floating overlay and just restyle it to look
accordion-like (e.g., anchor it directly under the header with no backdrop) — rejected because
that's a "floating panel with no dismiss-on-outside-click", an inconsistent hybrid; a true
inline accordion (reflowing layout, no floating/backdrop machinery at all) is the more honest
implementation of what was asked for, and iced's existing `slide` primitive already proved the
"animated reveal via a passthrough widget" pattern works well for exactly this kind of
transition.
