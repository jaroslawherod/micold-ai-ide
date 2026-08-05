# Contract: The `Typeahead` Component

**Modules**: `micold_client::ui::cdk::typeahead` (behaviour) and
`micold_client::ui::material::typeahead` (appearance) | **Feature**: [../spec.md](../spec.md)

The shared primitive FR-018 asks for. One consumer at delivery — the existing-branch picker — plus a
live instance in the component gallery. Any future picker consumes this rather than building a second
one (SC-007).

---

## §1 Builder API

Principle VIII, and `tests/material_builder_api.rs` holds it: construct with the required inputs,
configure with `self`-consuming methods, terminate in `.into()`.

```rust
Typeahead::new(query, rows, on_input, roles)
    .placeholder("Search branches…")
    .highlighted(Some(0))
    .selected(Some(2))
    .on_pick(Message::BranchSelected)
    .on_move(Message::HighlightMoved)
    .on_dismiss(Message::ListDismissed)
    .empty_message("No branches match that search.")
    .into()
```

| Input | Required | Meaning |
|---|---|---|
| `query: &str` | yes | what is in the field right now |
| `rows: Vec<Row>` | yes | the already-matched, already-ranked rows (§2), owned — a caller builds them from whatever it matched this frame, as `TreeView::new` takes its items |
| `on_input: Fn(String) -> M` | yes | emitted on every keystroke |
| `roles: Roles` | yes | the active scheme's tokens |
| `placeholder` | no | field placeholder |
| `highlighted` | no | which row the keyboard is on; `None` means no highlight |
| `selected` | no | which row is the caller's current selection, marked as such (FR-014b); independent of `highlighted` |
| `on_pick` | no | emitted with the picked row's index; without it the list is read-only |
| `on_move` | no | emitted when Up/Down moves the highlight |
| `on_focus` | no | emitted when the field takes focus, so the caller can open the list (FR-001b) |
| `on_dismiss` | no | emitted when the list closes without a pick — Escape, a click outside, or the field losing focus |
| `empty_message` | no | shown inside the list when `rows` is empty and the query is not |

**C1.1** The component performs **no** matching, ranking or ordering. It renders `rows` in the order
given. Filtering lives in [match-ranking.md](./match-ranking.md) and is applied by the caller.
**C1.2** The component holds no knowledge of branches, worktrees or git (FR-019). Its row type is
`{ label, spans, enabled }` and nothing else. **Enforced, not asserted** (FR-021a): a check fails the
build if either `typeahead.rs` names `branch`, `worktree` or `git`, in the same spirit as
`cdk_no_appearance.rs` — every other rule about these two layers is held by a gate, and one held only
by review would be the weak link.
**C1.3** No parameter exposes how the component is rendered, positioned or animated
(`tests/component_api_opacity.rs`).

---

## §2 Rows

```rust
pub struct Row { pub label: String, pub spans: Vec<Range<usize>>, pub enabled: bool }
```

- `spans` are byte ranges into `label`, ascending and non-overlapping. They are rendered with the
  emphasis treatment of §4 and nothing else is.
- `enabled == false` renders the row present but not pickable, and it is never hidden (FR-012,
  FR-012a). Whatever explains the unavailability must already be part of `label` — the component has
  no second text slot and no knowledge of *why* a row is disabled.
- Ranges that fall outside `label` are ignored rather than panicking — a malformed row degrades to an
  unemphasised one.
- `Row` stays a **plain record** the caller fills in, as `MenuItem`, `ProjectRow` and `TreeItem` do.
  It must not gain a `From<Row> for Element` impl: the shared inventory would then class it as a
  component, and it would need a gallery entry of its own.

---

## §3 Behaviour (the `cdk` half)

**C3.1 — Anchoring.** The result list is returned from the widget's `overlay()`, so the rendering
stack positions it from the field's own on-screen bounds. It therefore works inside a content-sized
dialog, and no other element in the form moves when the row count changes (FR-001a).

**C3.2 — Opening and closing.** Whether the list is shown is the **caller's** state, not the
component's: the component renders the list when told to and emits the events that let the caller
decide. A caller that wires `on_focus` and `on_dismiss` gets FR-001b's "opens on focus, closes on
blur" without the component holding any state of its own.

> **What "focus" and "blur" resolve to, decided during implementation.** The rendering stack's text
> input owns its focus and publishes nothing on either edge, so neither event is available to
> observe directly. `on_focus` is therefore emitted on a **press inside the field's bounds while the
> list is closed**, which is how a pointer reaches the field. `on_dismiss` is emitted on Escape, on a
> press outside both the field and the list, and on **Tab** — which is how the keyboard leaves it.
> Tab is the one key the list reacts to without capturing: it dismisses *and* passes through, so
> focus still moves. Without it, an open list outlives the focus that opened it and goes on claiming
> Enter and the arrows from whatever was tabbed to — so the next Enter would pick a branch instead
> of pressing the Create button the developer had just reached. An open list with no rows and a non-empty query shows `empty_message` (FR-015); an open list with
no rows and an empty query shows nothing at all.

**C3.3 — Keyboard.** While the list is open, the overlay translates each key event into
`micold_core::typeahead::Key`, calls `intent_for`, and emits the message the returned `Intent` names.
**It decides nothing itself** — the rule lives in
[match-ranking.md §4b](./match-ranking.md#4b-the-keyboard-rule) and is tested there, because FR-017
and FR-017a are decision logic and Principle I's GUI exception does not reach them. Every key that
yields no intent falls through to the field, so typing never leaves it (FR-017).

**C3.4 — Pointer.** Clicking an enabled row picks it. Clicking a disabled row does nothing —
specifically, it does not dismiss the list and it does not emit `on_pick` (FR-012a). Clicking outside
emits `on_dismiss`.

**C3.3a — The keyboard obeys the same rule.** Enter on a disabled highlighted row does nothing.
Up/Down may still land on a disabled row, so that its reason can be read; it just cannot be chosen
(FR-017a). This falls out of `intent_for` returning `None`, so it is held by §4b's tests rather than
by the widget.

**C3.5 — No appearance.** The `cdk` half names no colour, no token and no style function;
`tests/cdk_no_appearance.rs` enforces this by reading the source.

**C3.6 — No frames at rest.** The component never calls `Shell::request_redraw`
(`tests/idle_requests_no_frames.rs`).

**C3.7 — Overlay sanction.** This is the third entry on `tests/one_overlay_implementation.rs`'s
`SANCTIONED` list, for the reason recorded in [research R5](../research.md#r5). That gate is widened
by this feature to also see hand-written `fn overlay(` implementations, so a fourth still has to be
argued for in a diff.

---

## §4 Appearance (the `material` half)

**C4.1** The field **is** `material::TextField`, with Material's two named slots filled: a leading
icon saying what the field is for, and a trailing action that empties the query in one press
(FR-016, FR-011a). Both slots were added to `TextField` rather than assembled here — the spec's own
assumption is that a gap in the shared library is closed by extending it, and a search field is not
the last thing that will want a clear button.

**C4.1a — Every part is a library component.** Material Design 3 has no type-ahead: the pattern it
sanctions is *a text field with an attached menu*, and this component is that assembly. So each part
is the library's existing Material component rather than a widget styled here:

| Part | Component |
|---|---|
| the field | `material::TextField` (+ leading icon, trailing action) |
| the surface the list sits on | `material::menu_panel` — the same panel every popover uses |
| the scroll behaviour | `material::Scrollable` |
| a row's press feedback | `material::Ripple`, as `material::menu`'s own items do |
| the selected row's marker | `material::Glyph` |
| the no-match message | `material::Text` at `Caption`, muted |

A row is Material's **menu item**, assembled exactly as `material::menu` assembles its own, and
differing in two places that its content forces: the label is an `EmphasisedLabel` rather than a
`Text`, because part of it is emphasised; and it is set at `Body` rather than `Action`, because
`Action` is already the medium weight and emphasis would have nowhere to step up to. Row height is
`density::MENU_ITEM_BASE`, so a short label keeps its touch target.

**C4.1b — A row ripples only when it can be pressed.** The ripple's message is "that did something",
and pressing an unavailable branch does nothing (FR-012a), so the wrapper is absent rather than
present and lying.

**C4.2** The list is a menu surface anchored to the field: elevation, corner shape, row height,
padding and separators all come from the token set. No literal colour, size or spacing appears in
either half (FR-011b, `tests/material_boundary.rs`). **State-layer opacities come from
`tokens::state`** — never from a number chosen here. The first draft of `style::menu_row` hardcoded
`0.12` for pressed, which is the *selected* opacity, so a pressed row and a selected one rendered
identically; that is the same bug feature 019 had already fixed everywhere else, reintroduced by a
function written before those tokens existed.

**C4.3** Emphasis is a token-backed colour role plus type weight — not a filled background, which is
where the row's own hover / keyboard-highlight / selected states already live (FR-011c, research R7).

**C4.4** Rows are single-line and truncate via `fit_around`, so an emphasised run is never hidden
behind an ellipsis (FR-011d).

**C4.5** A disabled row is distinguished by more than the absence of emphasis — it carries its own
muted treatment, so "unavailable" and "unmatched" never look alike (FR-011).

**C4.7** The selected row carries a selection marker distinct from the keyboard highlight, so
"where the keyboard is" and "what is already chosen" are never confused. Both may sit on the same row
at once and must remain individually legible (FR-014b).

**C4.6** Both schemes are honoured; the gallery poses the component in each (FR-020).

---

## §5 What the branch picker adds on top

Everything branch-shaped stays in the feature module, not the component:

- turning `BranchCandidate` into a `Row` (label from its existing `Display`, so origin and in-use
  suffixes survive verbatim);
- `enabled = candidate.is_available()`, which is now what refuses a blocked branch — the refusal moves
  from the point of action to the point of choice (FR-012a);
- `selected = ` the index of `selected_branch` among the current rows, when it is among them (FR-014b);
- the two repository-level messages, which stay inline under the label rather than moving into the
  list (research R14).

`can_submit()`'s blocked-branch guard becomes unreachable through the picker, because
`selected_branch` can no longer hold a blocked candidate. It is **kept** rather than deleted: it is
the invariant's last line of defence, and a guard that costs one comparison is cheaper than the class
of bug its absence permits. Its test changes from "the form refuses a selected blocked branch" to "a
blocked branch cannot become the selection in the first place", and the guard keeps a direct unit test
of its own.

---

## §6 Gallery entry

**C6.0 — It lands with the component, not after it.** `tests/showcase_completeness.rs` fails in both
directions, so a component that exists in the library with no catalogue entry is a **build failure**
from the moment it compiles. The entry is therefore part of introducing the component, not a later
polish step. The behaviour half takes an `EXEMPTION` instead — it has no appearance of its own, the
same reason `cdk/overlay.rs`'s two components carry theirs.

**C6.1** One `catalogue::Entry` naming `material/typeahead.rs` / `Typeahead`, with `interactive: true`
and a non-empty `live` list — the caption must say which states are exercised rather than posed
(`tests/showcase_captions.rs`).
**C6.2** Its sample rows are fixed data in `showcase::samples` and its render function lives in
`showcase::sections::controls`, beside the other input controls, so the page is identical on every
launch (feature 020, FR-022).
**C6.3** The example is genuinely typeable: it filters and re-emphasises as the developer types
(FR-020). This is why `showcase::state::Message` gives up `Copy` (research R16).
