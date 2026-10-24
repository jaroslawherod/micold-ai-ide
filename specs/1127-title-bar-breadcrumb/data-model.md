# Data model: One 44 px title bar with a breadcrumb and a jump-to field

**Feature**: [spec.md](spec.md) | **Plan**: [plan.md](plan.md) | [research](research.md)

Nothing is persisted and no protocol message changes. Everything below is derived from existing
`app::State` on every view (Principle IV: local, in memory). Paths are relative to
`crates/micold-client/src/`.

## TitleBarModel (NEW, `ui/toolbar.rs`, render-free)

What the bar shows, built by `model(state: &State) -> TitleBarModel` (research R7). Tested
in-crate without a renderer.

| Field | Type | Rule | Requirement |
|---|---|---|---|
| `project` | `Segment` | active project's `display_name`; `Select project` without one | FR-004, US1 s4 |
| `unread` | `usize` | `state.other_projects_unread()` (feature 039); a mark is drawn when > 0 | FR-004, US1 s6 |
| `switcher_open` | `bool` | `state.project.switcher_open` | FR-004 |
| `place` | `Option<Segment>` | see *Place* below; `None` hides the segment and its `/` | FR-006 |
| `jump` | `Option<Message>` | `jump_press(state)`: `None` while Settings is open, else `QuickSwitch(Toggled { terminal_had_focus: state.terminal_focused() })` | FR-009, R6 |
| `settings_open` | `bool` | `state.settings.settings_draft.is_some()`; the Settings button is selected and sends nothing | FR-011, R10 |
| `elevated` | `bool` | `state.app_bar_elevated()` (unchanged) | FR-019 |

The Overview button is not in the model: until US3 lands it is built by today's
`overview_button(state, r)`, unchanged (FR-015); US3 deletes it (FR-016).

### Segment

| Field | Type | Meaning |
|---|---|---|
| `text` | `String` | what the segment says at full width |
| `tooltip` | `Option<String>` | the full text when it differs from `text` (a short branch's full branch); the bar adds the full `text` itself when it shortens it (FR-007) |

### Place (FR-006), first match wins

| State | `place` |
|---|---|
| no active project | `None` |
| Settings open (`settings.settings_draft.is_some()`) | `Settings` |
| Overview open (`overview.open`, normal or full screen) | `Overview` |
| Changes open (`changes.open = Some(view)`) | text of `view.entry` |
| a session is current (`current_session_location() = Some(loc)`) | text of `loc` (Compare keeps this, R7) |
| otherwise | `None` |

**Text of a location**: its card (`ui::overview::location_card(state, loc)`); with a branch,
`micold_core::context_header::branch_chip(branch)` → text = short branch (feature 1120) or the
branch itself, tooltip = the full branch; without a branch, the card's `display_name`, no tooltip.
A location that no longer resolves to a card → `None` (the segment disappears on the same update
as the main area, Principle II edge case).

## Jump-to field (NEW component `material::FieldButton`, research R5)

| Field | Type | Meaning |
|---|---|---|
| `glyph` | `Icon` | `Icon::Search` |
| `label` | `String` | `Jump to location, session or project` |
| `hint` | `Option<String>` | `keymap::switcher_chord_label()` — `Ctrl+Shift+P`, `Cmd+Shift+P` on macOS |
| `on_press` | `Option<M>` | `None` draws the disabled treatment and skips Tab |
| `max_width` | `f32` | `anatomy::app_bar::FIELD_MAX_WIDTH` (360) |

### Fit (state, decided at layout)

| Fit | Drawn | Width |
|---|---|---|
| `Full` | glyph, label, hint | natural, up to `max_width` |
| `GlyphAndHint` | glyph, hint | natural |
| `Glyph` | glyph only | `ICON_TARGET` square, still pressable |

Transitions follow the available width only; there is no stored state.

## Give-way order (FR-010, FR-007; research R2, R3)

When the bar is narrower than every child at its natural width, children give way in this order,
each only as far as needed:

1. the field shrinks from `FIELD_MAX_WIDTH` to its `Full` natural width;
2. the field drops its label (`Full` → `GlyphAndHint`) — planner `Drop(0)`;
3. the field drops its hint (`GlyphAndHint` → `Glyph`) — planner `Drop(1)`;
4. the place segment shortens with an ellipsis to a floor of one character and `…` —
   `Shorten(0)`;
5. the project segment shortens likewise, keeping its drop-down glyph and unread mark —
   `Shorten(1)`.

Never yield: app mark, the project segment's glyph, the Overview button (while FR-015 applies),
Settings, the overflow trigger. At 640 × 480 the never-yield set plus the floors fits (contract B5).

## Anatomy figures (`micold-core/src/tokens/anatomy.rs` `mod app_bar`, research R1)

| Constant | Today | After |
|---|---|---|
| `HEIGHT` | 64 | 43 |
| `DIVIDER` | 1 | 1 |
| `BOTTOM_EDGE` | 65 | 44 (`HEIGHT + DIVIDER`) |
| `PADDING` | 16 | 12 |
| `GAP` | — | 12 (NEW) |
| `ICON_TARGET` | 48 | 40 |
| `LEADING_ICON_PADDING` | 4 | 4 (unused by this bar; kept for its other readers) |
| `MARK` | — | 24 (NEW) |
| `FIELD_HEIGHT` | — | 32 (NEW) |
| `FIELD_MAX_WIDTH` | — | 360 (NEW) |

## Switcher anchor (research R8)

`project::State::switcher_anchor: Option<(u16, u16)>` is unchanged. The view hangs the panel from
`(anchor.x or PADDING + MARK + GAP, BOTTOM_EDGE)`; the reported `y` is not read.
