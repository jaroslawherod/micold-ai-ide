# Data model: Overview cards, full text without truncation

## Expansion (core, `micold_core::overview_expansion`)

Held in `features::overview::State.expansion`; reset to `Default` in `ui/overview.rs::settle_card_menu`'s
`!open` branch, which every way of closing passes through (`Closed`, `Toggled`, `CardActivated`),
so each opening starts from `Default`. Not persisted.

| Field | Type | Meaning |
|---|---|---|
| `expanded` | `Option<(SessionLocation, Cause)>` | The one expanded card and why (FR-006) |
| `focused` | `Option<SessionLocation>` | The card holding keyboard focus — shown focus only; a pointer press that takes the keyboard does not set it (U1) |
| `focus_escaped` | `bool` | The focused card was collapsed by Escape since it received focus |
| `pointer_on` | `Option<SessionLocation>` | The card the pointer last rested on and has not left |
| `pointer_escaped` | `bool` | That card was collapsed by Escape while the pointer stayed on it |
| `menu_open` | `bool` | A card menu is open: nothing changes form (FR-012) |
| `body_edges` | `(bool, bool)` | The expanded card's inner scroll can move (up, down) — for `page_target` |

`Cause` = `Pointer | Focus`.

### Events and transitions

| Event | Effect |
|---|---|
| `PointerRested(l)` | menu open → none. Else `pointer_on = l`, `pointer_escaped = false`; expand `(l, Pointer)` |
| `PointerLeft(l)` | if `pointer_on == l`: clear it. If `expanded == (l, Pointer)`: collapse; then if `focused` is some, not `focus_escaped`, and no other rest followed, expand `(focused, Focus)` (FR-005 re-expand) |
| `FocusGained(l)` | menu open → record `focused` only. Else `focused = l`, `focus_escaped = false`; expand `(l, Focus)` |
| `FocusLost(l)` | if `focused == l`: clear; if `expanded == (l, Focus)`: collapse |
| `Escape` | collapse the expanded card `c`, whatever its cause; if `focused == c` → `focus_escaped = true`; if `pointer_on == c` → `pointer_escaped = true` |
| `WindowBlurred` | collapse; `focused` kept (no re-expand on return, Edge Case) |
| `MenuOpened` / `MenuClosed` | set `menu_open`; form unchanged (FR-012) |
| `Listed(&[l])` | a card not listed: clear `expanded`/`focused`/`pointer_on` that name it |
| `BodyScrolled{up, down}` | set `body_edges` |

Each collapse clears `body_edges`. An expand of a card other than the expanded one replaces it.

## CardModel (core, `micold_core::overview`)

Gains `dir: PathBuf` — the worktree's directory, or the project root for *Default* (R8).

## CardText / ExpandedText (core, `micold_core::overview_text`)

- `CardText`: drops `title_tooltip`, `branch.tooltip`, `activity.tooltip` (FR-004).
- `ExpandedText { title, display_name: Option<String>, branch: Option<String>, dir: String,
  activity: Option<String> }` — contract K5.

## Placement (core)

`Placement { top: f32, height: f32, scrolls: bool }` from `place(slot, natural, view)` — contract K3.
