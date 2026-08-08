# Contract: `Select` — the dedicated Material select

**Modules**: `src/ui/material/select.rs` (rewritten), `src/ui/material/style.rs`
(`select_field`/`select_menu` retyped or retired), `src/ui/worktree_form.rs` (consumer, unchanged
call), `src/showcase/sections/controls.rs` + `catalogue.rs` (gallery).

**Supersedes**: `specs/013-create-worktree-refinement/contracts/material-select.md`, whose second
revision wrapped the rendering stack's `pick_list`. That revision is not being reversed for taste —
it fixed a real inline-vs-floating bug and was right to. What has changed is that the codebase now
has its own floating mechanism, built for the search picker (feature 021), that solves the same
problem *and* is ours to style. `pick_list` was the only thing that could anchor a dropdown to its
trigger inside a content-sized dialog when feature 013 was written. It no longer is.

---

## §1 Anatomy

### C1.1 — The trigger is a `FormField`, like every other field

Container, label inside it, value, bottom-edge active indicator, optional supporting text, error
state. Composed rather than restated: `FormField` already owns all of it and both `TextField` and
`Select` already use it (FR-002).

| State | Label | Position | Value line |
|---|---|---|---|
| Empty and inactive | `body_large` | centred on the value's line | placeholder suppressed — the resting label *is* the placeholder |
| Populated *or* open | `body_small` | 8dp from the top | the chosen option, or the placeholder, on the line below |

### C1.2 — Inside the container

A pressable row: the value text, a spacer, a trailing chevron (24, `on_surface_variant`, §7.7). It
carries the full state layer (§5) and a press ripple, exactly as every other pressable surface does —
which is what §7.7 already says the select's open and hover feedback should be.

### C1.3 — The active indicator answers for itself

Thickened and accented while the list is open (FR-013). The control knows it is open because it holds
the flag; no caller supplies it, and none can.

**This closes accepted fidelity gap #3.** §9 of `design-tokens.md` lists "Keyboard focus on the select
control" as an accepted gap because the stack's select "reports only active, hovered and open, with no
focus concept to observe". That was true of `pick_list`. A component that owns its own open state has
nobody to ask, so the gap list drops from four entries to three (SC-005), and `design-tokens.md` §7.7
and §9 are updated in the same change.

The gap being *closed* rather than *reworded* is the substantive claim, and it is the reason FR-013
says "from its own knowledge of being open" rather than "shows an open state".

---

## §2 Behaviour

| Action | Result |
|---|---|
| Press the trigger while closed | opens; the list animates in (picker-base §2.4) |
| Press the trigger while open | closes, choice unchanged |
| Press a row | closes **and** reports the choice, in one step |
| Press outside the list and trigger | closes, choice unchanged |
| Escape | closes, choice unchanged |
| Down / Up | moves the highlight |
| Enter | takes the highlighted row |
| Tab | closes and moves focus on |

On opening, the highlight is seeded from the current choice, so the list opens with the current value
marked and reachable — the behaviour `pick_list` gave for free and which must not be lost with it
(feature 013's FR-003).

**The current choice is marked** in the open list with the same leading marker the search picker uses,
and unmarked rows reserve the same space (FR-009).

---

## §3 API

```rust
pub struct Select<'a, T, M> { /* … */ }

impl<'a, T, M> Select<'a, T, M>
where
    T: Clone + ToString + PartialEq + 'a,
{
    pub fn new(
        options: &'a [T],
        selected: Option<T>,
        on_selected: impl Fn(T) -> M + 'a,
        roles: Roles,
    ) -> Self;

    pub fn placeholder(self, placeholder: impl Into<String>) -> Self;
    pub fn label(self, label: impl Into<String>) -> Self;
    pub fn supporting(self, text: impl Into<String>) -> Self;
    pub fn error(self, error: Option<impl Into<String>>) -> Self;
}

impl<'a, T, M> From<Select<'a, T, M>> for Element<'a, M> where M: Clone + 'a { /* … */ }
```

**Bounds are unchanged**, so `ConventionalType` still satisfies them with no changes of its own and
the existing consumer's call is untouched (FR-030).

**`active(bool)` is removed.** It existed only because `pick_list` could not report its open state, and
no caller ever supplied it — the accepted gap in builder form. Its one call site (the gallery's
`form_field` entry, which passes `.active(true)` to pose the chrome's active state) moves to posing
that state through a `TextField` instead, which *can* report focus.

**Nothing about progress, duration or easing appears here.** `component_api_opacity.rs` forbids it, and
the design makes it unnecessary: the animation is the component's own business.

---

## §4 Consumers

### C4.1 — The add-worktree type field (`ui/worktree_form.rs`)

```rust
let type_select = Select::new(ConventionalType::ALL, form.type_, Message::AddWorktreeTypeSelected, r)
    .placeholder("Select a type…")
    .label("Type");
```

Unchanged from today. **No new message, no new form field, no new reducer arm** — the select's
openness is its own (data-model §2.2), so `app.rs` grows by nothing. What the form accepts, validates
and submits is identical (FR-030), and `tests/app_state.rs`'s two existing select tests
(`selecting_a_type_sets_the_form_value`, `type_selection_is_ignored_while_creating`) must still pass
unmodified — they are the regression check on that claim.

### C4.2 — The gallery (`showcase/`)

The `Select` entry becomes `interactive: true` with a non-empty `live` list, which
`showcase_captions.rs` requires. It must **not** be posed open: feature 021's FR-020a (added by
BUG-001) binds every live entry — a live entry pins no state the application cannot leave.

The select and search-picker entries sit adjacent in `sections/controls.rs`, so FR-031's "compare the
two in one place" is satisfied by the page they are already on. The `form_field` entry, which
currently poses the shared chrome through a `Select`, moves to a `TextField` for the reason in §3.

---

## §5 What is removed

Counted here because it is most of the diff, and because two of these are gates that will fail until
they are attended to rather than things a reviewer must remember:

| Site | Change |
|---|---|
| `material/select.rs` | no longer imports or wraps `pick_list` |
| `material/style.rs` | `select_field` / `select_menu` are typed in `pick_list::Status` and `menu::Style`. The look survives; the signatures do not |
| `material/style_snapshot.rs` | drops the three `pick_list` status poses and `pick_list.menu`; the fixture changes with it |
| `tests/one_overlay_implementation.rs` | the `select.rs` / `pick_list` `SANCTIONED` entry — **the staleness check fails the build until it goes** |
| `tests/material_boundary.rs` | `pick_list` leaves `WRAPPED_WIDGETS` |
| `tests/support/layout.rs`, `tests/support/covered_states.rs`, `tests/layout_snapshot.rs` | the special-casing that exists solely to reach `pick_list`'s private open flag and its out-of-tree dropdown **dissolves** — the list is composed in-tree now, so the base walk sees it like any other element |

That last row is a net simplification, not a cost: three test-support files currently carry machinery
whose only purpose is to work around a widget this feature deletes.

---

## §6 Documentation deliverables (Principle VII)

| Document | Change |
|---|---|
| `specs/018-material3-visual-system/contracts/design-tokens.md` §7.7 | the select is a first-class control; its open state is no longer "not left mute" by way of the state layer alone |
| …§9 | accepted fidelity gap #3 **removed**; the list drops from four to three |
| `docs/development/component-library.md` | the select and the shared picker base — what each is for, and that a third picker consumes the base rather than rebuilding it |
| `specs/013-create-worktree-refinement/contracts/material-select.md` | a superseding note pointing here, in that file's own established style — it already carries two revisions and their reasoning |
