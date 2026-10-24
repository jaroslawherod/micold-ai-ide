# Research: One 44 px title bar with a breadcrumb and a jump-to field

**Feature**: [spec.md](spec.md) | **Plan**: [plan.md](plan.md)

Each decision names what was chosen, why, and what was rejected. Paths are relative to
`crates/micold-client/src/` unless they start with `crates/`, `docs/` or `specs/`. Since #1262
(merged after this research), `material/<file>` for a generic component (`toolbar`, `status_bar`,
`button`, `icon_button`, `menu`, `keyboard_focus`, `ellipsized`, and this feature's NEW
`give_way`, `breadcrumb`, `field_button`, `app_mark`) means `crates/micold-material/src/<file>`;
`material/composition_contrast.rs` and `material/content_placement.rs` stay in the client.

## R1. The bar's figures: change `anatomy::app_bar`, keep its name

- **Decision**: In `crates/micold-core/src/tokens/anatomy.rs` `mod app_bar`: `HEIGHT` 64 → **43**,
  `DIVIDER` stays 1, so `BOTTOM_EDGE` = **44** (FR-001, SC-001: "44 px tall, divider included").
  `PADDING` 16 → **12** and a NEW `GAP` = 12 (the mockup's `padding:0 12px; gap:12px`).
  `ICON_TARGET` 48 → **40**, the same as `density::BUTTON_BASE`, so every bar control is one 40 dp
  height with 1.5 dp of slack above and below. NEW `MARK` = 24 (the app mark's side), NEW
  `FIELD_HEIGHT` = 32 and NEW `FIELD_MAX_WIDTH` = 360 (the jump-to field), all entered in `ALL`
  and pinned in `crates/micold-core/tests/tokens_anatomy.rs`. The module keeps the name `app_bar`;
  its doc comment says the figures are feature 1127's, superseding 018's §7.1 table
  (`specs/018-material3-visual-system/contracts/design-tokens.md` §7.1 stays as history; this
  feature's [contract](contracts/title-bar-ui.md) T1 is the current table).
- **Rationale**: every panel already hangs from `BOTTOM_EDGE` (`material/menu.rs` fallback anchor,
  `tests/gates/panel_placement.rs`, `tests/gates/sibling_parity.rs`), and the session bar's
  `rising_above(app_bar::HEIGHT)` reads `HEIGHT`; changing the figures moves all of them at once
  (BUG-003's lesson: read, never restate). A 48 dp target cannot stand in a 43 dp bar; 40 dp is the
  labelled button's height, so the bar has one control height.
- **Rejected**: (a) a NEW `title_bar` module beside `app_bar` — two modules for one bar, and every
  reader of `app_bar::BOTTOM_EDGE` would need to be moved by hand or stay wrong; (b) `HEIGHT` = 44
  with the divider drawn inside — `BOTTOM_EDGE` would become 45 or need redefining, and 018's
  "height + divider" rule would read wrong; (c) keeping 48 dp targets with a 44 dp bar — clipped
  targets, FR-010 forbids clipping; (d) the mockup's 36 dp buttons — `Button` is 40 dp everywhere
  else and a bar-only size would be a second button height (Principle VIII).
- **Accessibility note**: `anatomy::button::MIN_TOUCH_TARGET` (48) is the touch figure; this is a
  pointer-and-keyboard desktop app and every other labelled button is already 40 dp.

## R2. The bar component: reshape the shared `material::Toolbar`

- **Decision**: `material/toolbar.rs` keeps the type `Toolbar` (one top bar, Principle VIII) and
  changes shape: `Toolbar::new(roles)` (no title), `.mark(element)` (leading, never yields),
  `.leading(element)` (the breadcrumb, which shortens), `.leading_fixed(element)` (the Overview
  button while FR-015 applies, never yields), `.centre(element)` (the jump-to field), `.action(element)`
  (trailing, never yields), `.elevated(bool)` unchanged. `From<Toolbar>` builds a measuring widget
  that lays the trailing group and fixed leading items at natural width, gives the centre child the
  free space up to its maximum and centres it there, and lets the breadcrumb have the rest
  (contract B1–B4). The divider and the elevation rule are unchanged (FR-019).
- **Rationale**: FR-008 centres the field in the *free space between the groups*, and FR-010 and
  FR-007 give a fixed give-way order across three children; neither is expressible with a `row!`
  and `Space::Fill` spacers (a `Fill` child takes the line before any child shrinks — see
  `cdk/reflow.rs`'s module comment). The `StatusBar` (feature 1124 R4) already lays itself out for
  the same reason.
- **Rejected**: (a) a second bar component (`TitleBar`) beside `Toolbar` — the old one would be
  dead or forked (Component-reuse gate); (b) `row![mark, crumb, Space::Fill, field, Space::Fill,
  actions]` — the field cannot both centre and yield first, and a long breadcrumb would push it
  (clarification "Where is the jump-to field centred").

## R3. One give-way planner, shared with the status bar

- **Decision**: Move `plan()`, `Widths` and `GiveWay` out of `material/status_bar.rs` into NEW
  `material/give_way.rs` (`pub(crate)`; `GiveWay` stays re-exported from `material` for the status
  bar's builder). `StatusBar` and `Toolbar` both call it. The title bar's order (FR-010, FR-007,
  data-model *Give-way order*) maps to: field spare width first (handled before the planner, R2),
  then field text `Drop(0)`, field hint `Drop(1)`, place segment `Shorten(0)`, project segment
  `Shorten(1)`.
- **Rationale**: one rule for "who yields first" (Search-before-writing rule; `mise run
  duplication`). The planner is already pure and unit-tested.
- **Rejected**: a second planner in `toolbar.rs` — a copy the duplication budget catches.

## R4. The breadcrumb: a NEW shared `material::Breadcrumb`

- **Decision**: NEW `material/breadcrumb.rs`: `Breadcrumb::new(project_label, roles)`,
  `.on_press(msg)` (the project segment is a text `Button` with a trailing drop-down glyph),
  `.trailing_mark(count, tooltip)` (feature 039's unread mark, as `Button::trailing_mark`),
  `.place(text, tooltip)` (a plain `Ellipsized` label at the label role, never a control),
  `.open(bool)` (the segment drawn selected while the switcher is open), terminating in `.into()`.
  Each segment's label is an `material::Ellipsized`; the segment's tooltip holds the full text
  only when it is shortened or differs from the label (FR-007). `Button` gains `.trailing(Icon)`
  (a trailing glyph slot at `LEADING_ICON` size, mirroring `.leading`) for the drop-down glyph.
- **Rationale**: a `project / place` path is a reusable pattern (the Overview's header may want
  it); Principle VIII says a new element is created in the library and consumed from there.
  `Ellipsized` already measures and clips (its module comment), so shortening needs no new text
  code.
- **Rejected**: assembling `row![Button, text("/"), text(place)]` in `ui/toolbar.rs` — a one-off
  widget at the call site (Component-reuse gate) with no ellipsis.

## R5. The jump-to field: a NEW shared `material::FieldButton`

- **Decision**: NEW `material/field_button.rs`: `FieldButton::new(Icon::Search, label, roles)`,
  `.hint(text)` (an outlined key-cap), `.on_press_maybe(Option<M>)` (none → disabled treatment,
  content at 38%, FR-017), `.max_width(f32)`, `.id(Id)`, terminating in `.into()`. A pill
  (`shape::FULL`) of `FIELD_HEIGHT` on `surface_container_highest` with an `outline_variant`
  hairline; at 3:1 the outline is `outline` (contract T2 picks the role that passes,
  `composition_contrast.rs` checks it). It reports its three widths (full, glyph + hint, glyph) to
  the bar through the planner items of R3. Pressable with the pointer and, wrapped in
  `TakesTheKeyboard`, by Enter and Space (FR-009, FR-013).
- **Rationale**: "looks like a field, acts like a button" is a reusable control (a search
  launcher); the existing `FilledField` and `Typeahead` are real text inputs with focus-for-typing
  semantics the spec rules out ("pressed, not typed into").
- **Rejected**: (a) a real `text_input` that forwards to the switcher — it would take focus and
  typed keys away from the terminal and duplicate the switcher's query; (b) an outlined `Button`
  with a long label — no hint slot, no give-way, wrong shape.

## R6. What the field sends

- **Decision**: `ui/toolbar.rs::jump_press(state) -> Option<Message>`: `None` while Settings is
  open (`state.settings.settings_draft.is_some()`), else
  `Some(Message::QuickSwitch(quick_switch::Msg::Toggled { terminal_had_focus:
  state.terminal_focused() }))`. The press goes through the same reducer as the chord
  (`features/quick_switch.rs` `update`), and the shell's `shell/quick_switch.rs` already focuses
  `QUICK_SWITCHER_FIELD_ID` when the switcher opens (FR-009, US2 s1). With the switcher open, the
  bar sits under its `Dialog`-layer scrim, so a press on the field is a press outside the switcher
  and dismisses it through the existing `cancelled_by: Message::QuickSwitch(Msg::Dismissed)` —
  no second switcher can open (US2 s3).
- **Rationale**: FR-009 "the same effect as the shortcut in the same state"; reusing the reducer
  makes that true by construction, including its `topmost(state).is_none()` guard.
  `terminal_had_focus` is read at view time, before the click, so closing the switcher returns the
  keyboard to the terminal exactly as after the chord.
- **Rejected**: a new `quick_switch::Msg::FieldPressed` — a second entry point that could drift
  from the chord's rules.

## R7. The place segment's text

- **Decision**: NEW render-free `ui/toolbar.rs::model(state) -> TitleBarModel` (data-model). The
  place is, in order: Settings open → `Settings`; Overview open → `Overview`; Changes open → its
  `OpenView::entry`; otherwise `state.current_session_location()`; no project or no location →
  absent (FR-006). A location's text comes from its card (`ui::overview::location_card`, the source
  the context header of feature 1126 already reads): with a branch, `context_header::branch_chip`
  (made `pub`) gives the short branch (feature 1120) as text and the full branch as tooltip;
  without one, the card's `display_name`. Compare (feature 483) shows a run group, not a location,
  so it keeps the current session's location. The project segment is
  `workspace.active_project().display_name`, else `Select project` (FR-004).
- **Rationale**: one rule for "short branch, full branch on hover" already exists in
  `micold-core/src/context_header.rs` and is tested; reusing it keeps the bar and the context
  header from disagreeing (SC-005). Following `current_session_location()` means a location removed
  by another window or the daemon changes the segment on the same update as the main area
  (Principle II edge case), because both read the same state.
- **Rejected**: (a) the issue title (feature 1126's header shows it; spec Assumptions); (b) a new
  short-branch function — duplicate of `overview_text::short_branch` + `branch_chip`.

## R8. The project switcher panel's anchor

- **Decision**: The panel hangs from `(x, anatomy::app_bar::BOTTOM_EDGE)`: `x` is the project
  segment's reported leading edge (`ContextArea::on_primary_bounds`, BUG-1023, unchanged) and, when
  the switcher is opened from the keyboard (no press point), the computed leading edge
  `app_bar::PADDING + app_bar::MARK + app_bar::GAP`. The `y` of the reported corner is ignored:
  the segment is a 40 dp button inside a 43 dp bar, so its own bottom is 1.5 dp above the bar's
  (FR-005, SC-004 within 1 px). The clamp (`features::project::clamp_menu_anchor`) is unchanged.
- **Rationale**: FR-005 states the panel's top edge *at the bar's bottom edge*; the bar's edge is a
  constant (R1), the segment's `x` is not (the app mark is fixed, but a computed `x` is only the
  fallback, so the BUG-008 rule "hang from the press, not a figure in the view" still holds for the
  pointer). Today a keyboard open falls back to the window's trailing edge (`menu.rs`'s
  `Anchor::TopEnd`), which FR-005 rules out.
- **Rejected**: making the segment 43 dp tall so its corner is the bar's edge — a bar-only button
  height (R1 (d)).

## R9. The app mark

- **Decision**: The 64 × 64 RGBA icon `shell/startup.rs` embeds as private `ICON_RGBA`
  (`assets/icon/icon-64.rgba`) becomes `pub(crate) APP_ICON_RGBA` there (startup keeps reading it),
  and `ui/toolbar.rs` builds one `LazyLock<image::Handle>` from it. NEW generic
  `crates/micold-material/src/app_mark.rs` `AppMark::new(handle, name, roles)` draws an
  `iced::widget::image` of `app_bar::MARK` side wrapped in `Tooltip` with the name; the bar passes
  `metadata::APP_NAME` (FR-003, US1 s8). The library holds no Micold asset (#1262's
  `no_domain_types` rule). It has no press handler and no `TakesTheKeyboard`, so it is neither a
  control nor a Tab stop.
- **Rationale**: spec Assumptions "the existing application icon"; the raw RGBA needs no decoder
  (the reason startup embeds it raw); one `Handle` keeps one texture id across frames, so the mark
  costs no upload per frame (idle budget, `tests/idle_requests_no_frames.rs`).
- **Rejected**: (a) `assets/icon/png/icon-32.png` via `Handle::from_bytes` — a decode per new
  handle; (b) a glyph from the symbol font — not the application's mark.

## R10. Settings button

- **Decision**: An `IconButton` (`Icon::Settings`) after the field, before the overflow trigger,
  tooltip `Settings`, sending `Message::Settings(SettingsMsg::Opened)` while Settings is closed and
  nothing while it is open (`settings_press(state)`). `IconButton` gains `.selected(bool)`: the
  selected state layer, drawn also when the button has no press, so "open and inert" reads as
  selected, not disabled (FR-011, edge case "Settings open").
- **Rationale**: `settings::opened` calls `clear_for_dialog()` and returns
  `Outcome::SettingsOpened` even when Settings is already open, so re-sending it is not a no-op;
  sending nothing is. The overflow menu keeps its Settings item (spec Assumptions; one writer,
  feature 027 FR-026e).
- **Rejected**: a disabled-looking button while open — FR-011 asks for "selected".

## R11. Keyboard reach

- **Decision**: Every bar control takes the keyboard through the library's one wrapper,
  `material/keyboard_focus.rs` `TakesTheKeyboard` (Enter and Space send the press), in bar order
  (FR-013). `Button` already wraps itself when pressable (`material/button.rs`, FR-030), so the
  project segment and the Overview button need nothing. `IconButton` gains an opt-in
  `.keyboard(true)` that wraps it the same way, and `MenuTrigger` (built on `IconButton`) turns it
  on; the Settings button and the overflow trigger use it. `FieldButton` (R5) wraps itself when
  pressable, so the field while Settings is open is skipped by Tab. The app mark is not wrapped.
  `KeyboardElsewhere` still strips the bar's focus while the terminal holds the keyboard
  (018 BUG-016).
- **Rationale**: rendering-stack buttons hold no focus (`material/keyboard_focus.rs` module
  comment); the wrapper is the library's one implementation (`micold-material`), so reach is a
  component option, not something `ui/toolbar.rs` assembles.
- **Rejected**: (a) making every `IconButton` a Tab stop by default — it changes the sidebar's and
  every dialog's Tab order, outside this feature; (b) a bar-specific focus ring — a second
  implementation of the wrapper.

## R12. Story 3 waits for #1125 M3

- **Decision**: The Overview button's removal (FR-016, US3) is its own last milestone, scheduled
  after #1125's milestone M3 ("Overview as the sidebar's first row") is on `origin/main`. As of
  this plan (`git fetch`, 2026-10-24) #1125 M1 and M2 are on `origin/main`; M3 is `todo` in
  `specs/1125-sidebar-state-groups/autopilot.md`. Earlier milestones keep `overview_button`,
  `badge_look`, `overview_press` and their tests in `ui/toolbar.rs` untouched (FR-015) and pass it
  to `Toolbar::leading_fixed`. #1125 M3 moves `overview_tooltip` into `features/overview.rs`
  (its T036); whichever lands second rebases onto the other's `toolbar.rs`.
- **Rationale**: spec clarification "Can the bar rebuild land before #1125"; without the row,
  removing the button leaves only the chord (US3 "Why this priority").
- **Rejected**: stacking a Story-3 branch on #1125's unmerged branch — the unit rules forbid work
  on an open PR's base.

## R13. Tests that pin today's bar

- **Decision**: These change with the bar, in the milestone that changes it: `tests/toolbar.rs`
  (`origin.y < app_bar::HEIGHT` keeps working); `tests/switcher_anchor.rs` (`(300, 64)` → the
  panel's `y` is `BOTTOM_EDGE` whatever the report); `tests/gates/sibling_parity.rs` (the icon-only
  shape reads the new `ICON_TARGET`); `tests/support/covered_states.rs` component paths named
  `toolbar.title` (the title is gone: the anchor becomes `toolbar.mark`); the content-placement
  test in `material/content_placement.rs` (builds `Toolbar::new(LABEL, r)`); the showcase's
  `Toolbar` entry (`showcase/sections/surfaces.rs`); the layout snapshot fixture, regenerated
  (`UPDATE_LAYOUT_SNAPSHOT=1`), where every state's body moves up 21 dp.
- **Rationale**: listing them up front lets the tasks unit put each in the milestone that breaks
  it rather than discovering them at the gate.
