# Contract: the title bar (UI)

**Feature**: [spec.md](../spec.md) | [data model](../data-model.md) | [research](../research.md)

Paths are relative to `crates/micold-client/src/`. Each row names the test that holds it
([quickstart](../quickstart.md) Part A).

## T. Tokens and look

| ID | Rule |
|---|---|
| T1 | `anatomy::app_bar`: `HEIGHT` 43, `DIVIDER` 1, `BOTTOM_EDGE` 44, `PADDING` 12, `GAP` 12, `ICON_TARGET` 40, `MARK` 24, `FIELD_HEIGHT` 32, `FIELD_MAX_WIDTH` 360. Supersedes 018's §7.1 height, padding, target and bottom edge; the surface, elevation, divider and icon colour rows of §7.1 are unchanged (FR-019). |
| T2 | The field: `surface_container_highest` fill, full pill shape, a 1 dp outline in the outline role that reaches 3:1 against the bar in both schemes; glyph and hint key-cap outline at 3:1, label and hint text in `on_surface_variant` at 4.5:1 (FR-017). Disabled: content at the shared 38% treatment, outline likewise. |
| T3 | Breadcrumb: project segment at the label role in `on_surface`; separator `/` and place segment in `on_surface_variant` (place at the label role, emphasised weight as the mockup's `font-weight:600`) — each ≥ 4.5:1 on the bar's surface at rest and elevated, both schemes (FR-017). |

## B. Bar component (`material::Toolbar`, research R2)

| ID | Rule |
|---|---|
| B1 | `Toolbar::new(roles).mark(e).leading(e).leading_fixed(e).centre(e).action(e).elevated(b).into()`. The bar is `HEIGHT` tall plus `DIVIDER` at rest, `HEIGHT` alone when elevated, full window width, `PADDING` at each end, `GAP` between children, children centred vertically. |
| B2 | Order from the leading end: mark, leading (breadcrumb), leading-fixed items, [free space], centre, [free space], actions in insertion order (FR-002). |
| B3 | The centre child is centred in the free space between the end of the leading group and the start of the trailing group (equal free space either side), never on the window's centre line (FR-008). |
| B4 | Give-way order: data-model *Give-way order*, through the shared planner (`material/give_way.rs`). No child is laid out outside the bar, and no two children's bounds overlap, at any width ≥ 640 (FR-010). |
| B5 | At 640 dp with an unread mark, the Overview button and its badge, a 60-character project name and a 60-character branch: every control is visible, inside the bar's height, not overlapping; the field shows at least its glyph (SC-003). |
| B6 | Opening a panel (switcher, overflow menu, tooltip) changes no child's bounds (edge case "Panels over the bar"). |

## C. Controls (`ui/toolbar.rs`)

| ID | Control | Press | Tooltip | Keyboard |
|---|---|---|---|---|
| C1 | App mark (`material::AppMark`) | none | `Micold` | not a Tab stop (FR-003) |
| C2 | Project segment (`material::Breadcrumb`) | `Project(SwitcherToggled)`; reports its leading corner via `ContextArea::on_primary_bounds` | `Switch project` (no shortcut), then the full project name on a second line when shortened; the unread mark keeps `unread_total_tooltip` | Tab stop, Enter/Space (FR-004, FR-013) |
| C3 | Place segment | none | the full branch (or full text when shortened) | not a Tab stop (FR-006) |
| C4 | Overview button (until US3) | today's `overview_press` | today's `overview_tooltip` | Tab stop (`Button`) (FR-015) |
| C5 | Jump-to field (`material::FieldButton`) | `jump_press(state)` (data-model) | `Jump to location, session or project (Ctrl+Shift+P)` / `Cmd+Shift+P` | Tab stop when pressable (FR-009, FR-013) |
| C6 | Settings (`IconButton`, `Icon::Settings`, `.selected(open)`) | `Settings(Opened)` while closed; none while open, still drawn selected | `Settings` | Tab stop (`.keyboard(true)`) (FR-011) |
| C7 | Overflow trigger (`MenuTrigger`, `Icon::Menu`) | `Help(MenuToggled)` | `More` | Tab stop (FR-012) |

Tab order is C2, C4, C5, C6, C7 (bar order). Shortcut chords are unchanged (FR-014): no new key
handling is added; `ui/mod.rs` `app_chord` and `keymap.rs` are not edited by this feature.

## P. Panels

| ID | Rule |
|---|---|
| P1 | The switcher panel's top edge is at `BOTTOM_EDGE` (±1 dp) and its leading edge at the project segment's leading edge (±1 dp), opened by pointer or keyboard, in every body branch (FR-005, SC-004). Its rows, counts, actions, context menu, scroll and dismissal are unchanged. |
| P2 | The overflow menu keeps its three items in order (Settings, Session service diagnostics, About) and opens below `BOTTOM_EDGE` at the trailing end, under its trigger (FR-012). |
| P3 | With the quick switcher open, a press at the field's position dismisses the switcher (`QuickSwitch(Dismissed)`) and opens nothing (US2 s3). |

## O. Overview removal (US3, after #1125 M3)

| ID | Rule |
|---|---|
| O1 | The bar has no Overview button and no badge in any body branch, Overview full screen included (FR-016). |
| O2 | Ctrl+Shift+O / Cmd+Shift+O opens and closes the Overview as before, also with the terminal focused and in full screen; the sidebar's Overview row opens it (feature 1125). |
| O3 | `Toolbar::leading_fixed` has no caller left in `ui/toolbar.rs`; the builder method stays only if the showcase or another caller uses it, else it is removed with its tests. |
