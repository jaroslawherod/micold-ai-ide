# Contract: the quick-switcher (state and surface)

**Feature**: 968 | [plan](../plan.md) | [data model](../data-model.md) | [research R3–R8](../research.md)

## U1 — Opening gate

`Toggled` with the switcher open closes it (as `Dismissed`). With it closed, it opens only when
`state.settings.settings_draft.is_none()` and `overlay::registry::topmost(state).is_none()` (no
dialog, context menu, project switcher panel or filter panel: FR-012). Otherwise nothing changes.
On open: `query = ""`, `highlighted` = the first selectable entry, `terminal_had_focus` from the
message, `keyboard_row_before = sidebar.keyboard_row`.

## U2 — The list

`candidates(&State)` in FR-004 order (data-model *Entry*), then `quick_switch::filter` with the
query, then `reseat` with `highlighted`. Derived on every view and message, so a session or
worktree created, renamed, closed or removed while open appears or leaves without closing the
switcher or clearing the query (FR-013).

| State | List |
|---|---|
| active project | its locations (location order) each followed by its open sessions (session order); then the other known projects in switcher order |
| welcome screen (no active project) | every known project |
| no known project | empty: "Nothing to switch to" |
| query matches nothing | empty: "No matches" |

## U3 — Choosing

`Chosen(key)` looks `key` up in a fresh `candidates(state)`:

| Key | Result | Step (applied by `apply_navigation` / `finish_navigation`, #803 U3) |
|---|---|---|
| not listed | nothing; stays open | — |
| `Project(p)`, unavailable | nothing; stays open | — |
| `Session(id)` | close | `Step::Session(id)` |
| `Location(l)` with a main session | close | `Step::Session(main)` |
| `Location(l)` without | close; closes the Overview if open | `Step::Stop(l)` |
| `Project(p)`, available | close | `Step::Project(p)` |

`terminal_had_focus` is the one recorded at open (FR-010). No step starts, stops or closes a
session (FR-011, SC-005).

## U4 — Closing without a choice

`Dismissed` (Escape, scrim click, Tab — the registry's cancellation) or `Toggled` while open:
`open = None`, `sidebar.keyboard_row = keyboard_row_before`. The current project, location and
session are unchanged (FR-009); the terminal takes the keyboard back by derivation when it had it.

## U5 — Keys inside the switcher

| Key | Effect |
|---|---|
| typed text, paste, IME | `QueryChanged` (the field's own input; FR edge *Text input*) |
| Down / Up | `Moved(Next / Previous)`: `quick_switch::step`, stops at the ends, skips unavailable projects |
| Enter | `Chosen(highlighted)` when the highlighted row is enabled; else nothing (FR-014) |
| Escape | `Dismissed` |
| switcher chord | `Toggled` (closes) |
| #803 chords | nothing (their gate reads `registry::topmost`) |

## U6 — Surface and view

- `QuickSwitcherDialog`: `floating_surface!(.., id: SurfaceId::new("quick_switcher"), layer:
  Dialog, cancelled_by: Message::QuickSwitch(Msg::Dismissed))`, `Registered::open_in` reads
  `state.quick_switch.open.is_some()`; listed in `overlay/registry.rs`'s `REGISTERED` with
  `crate::ui::quick_switcher::dialog`.
- `ui/quick_switcher.rs::dialog(&State) -> Element`: `Surface::new(material::dialog::fields(column![typeahead]),
  SurfaceKind::Dialog, roles)` (dialog.rs exposes free functions `fields`/`actions`/`body`/`confirm`,
  not a type; no `actions` row, the switcher has no buttons), the column holding a
  `material::Typeahead::new(query, rows, QueryChanged, roles).open(true).highlighted(i)
  .selected(current).empty_message(..).on_move(Moved).on_pick(Chosen(key of row))
  .on_dismiss(Dismissed)`. Each row: `picker::Row::new(name, spans).details(context_line)` with
  `.disabled()` for an unavailable project; `context_line` is `"<Kind> · <context>"` from
  `Entry` (FR-006). Names on one line, truncated around the emphasis by the type-ahead's
  `fit_around` (edge *Long names*). The list scrolls inside the panel (edge *Many entries*).
- Glue only: every decision is in `features/quick_switch.rs` or core (Principle I exception).
