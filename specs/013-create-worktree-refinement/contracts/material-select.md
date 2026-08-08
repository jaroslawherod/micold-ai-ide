# Contract: `Select` — Material List-Select Component

**Modules**: `src/ui/material/select.rs` (new), `src/ui/material/mod.rs` (export), `src/ui/
worktree_form.rs` (first consumer), `src/ui/style.rs` (`select_field`/`select_menu`).

> **Third revision (current — feature 022)**: the `pick_list` wrapper described below has been
> replaced by a select this library writes itself, over the shared picker base
> (`specs/022-dedicated-select-component/contracts/picker-base.md`). The second revision's reasoning
> was right when it was written and is worth keeping exactly for that: `pick_list` implemented
> `Widget::overlay()` and this codebase had nothing that did, so it was the only thing that could
> position a dropdown from its trigger's own on-screen bounds inside `Modal`'s content-sized dialog.
> What changed is not the reasoning but the premise. Feature 021 built `cdk::picker` for the branch
> type-ahead, which solves that same positioning problem and is this library's to style — so the
> select's list can be the *same* list the type-ahead floats, row for row, instead of the rendering
> stack's menu behind a style closure that nothing else could see into.
>
> What that costs and buys, plainly: `style::select_field` and `style::select_menu` are **gone**,
> because both were written against `pick_list`'s own types and neither could outlive the widget;
> the look they encoded is assembled from what the library already had, and `select_menu` turned out
> to be a hand-kept second copy of `menu_surface` that had already drifted from it. `Select::active`
> is gone from the builder, because the control holds its own openness now and the indicator answers
> from it — which is accepted fidelity gap #3 closing. `AddWorktreeTypeSelected` is unchanged, and
> `src/app.rs` still gains nothing for this control.
>
> Everything below this note is history. The `pick_list` entry in
> `tests/one_overlay_implementation.rs`'s `SANCTIONED` list was **removed** rather than left
> standing, and that gate fails the build while a sanction that no longer applies is still listed —
> so this supersession is enforced rather than remembered.

> **Implementation note (second revision — superseded, kept for history)**: the inline-panel design below (this
> contract's first revision) shipped, then was reported as reading wrong in review: the list
> visibly pushed the rest of the form down instead of floating above it like every other
> dropdown in the app. It was replaced by wrapping iced's own built-in `pick_list` widget instead
> of hand-rolling the panel. `pick_list` implements `Widget::overlay()` directly — its dropdown is
> positioned from the trigger's on-screen bounds by iced's own overlay system, independent of the
> parent layout's size constraints, which is exactly what the first revision's `Length::Fill`-vs-
> `Modal`'s-`Shrink`-height conflict (described below, now historical) ruled out for the
> hand-rolled `stack!`-based approach. `pick_list` also seeds the open menu's highlighted row from
> the current value on open, so reopening the list visibly marks the current selection (FR-003)
> for free. Net effect: `SelectItem`/`SelectTrigger`/`SelectOverlay` and `WorktreeForm.
> type_menu_open`/`Message::AddWorktreeTypeMenuToggled` are all gone — `pick_list` owns the
> open/closed state itself, so there is nothing left in `src/app.rs` for this control beyond
> `AddWorktreeTypeSelected` setting `type_`. The single `Select` builder below wraps `pick_list`
> with Material styling (`style::select_field`/`style::select_menu`, mirroring `style::input`/
> `style::menu_surface`'s look) — reuse of iced's own overlay machinery rather than of this
> codebase's `menu_panel`, which is the more direct fix for a floating-vs-inline positioning bug.
>
> **First revision (superseded, kept for history)**: this contract originally specified a
> floating overlay (`base` + invisible full-window backdrop + `stack!`), mirroring `MenuOverlay`/
> `ProjectSwitcherOverlay` exactly. During implementation that design was believed unsafe for this
> consumer — the trigger lives inside `Modal`'s dialog box, a fixed-width (`Length::Fixed(520.0)`),
> content-sized (`Shrink` height) container, and a `Length::Fill`-seeking backdrop/panel nested
> inside a `Shrink`-height parent has no bounded space to fill against — so the list was instead
> revealed **inline**, directly below the trigger, mirroring the sidebar's tag-filter accordion
> (`src/ui/sidebar.rs`'s `filter_accordion`, feature 009). That reasoning only applied to the
> hand-rolled `stack!`-based mechanism `MenuOverlay`/`ProjectSwitcherOverlay` use, not to iced's
> own `Widget::overlay()` mechanism `pick_list` uses instead — the second revision above.

## Types (as implemented, second revision)

```rust
/// A Material-styled select control wrapping `iced::widget::pick_list`. Builder form
/// (Principle VIII): `Select::new(options, selected, on_selected, roles).placeholder("...").into()`.
pub struct Select<'a, T, M> {
    options: &'a [T],
    selected: Option<T>,
    on_selected: Box<dyn Fn(T) -> M + 'a>,
    placeholder: String,
    roles: Roles,
}

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

    /// Text shown when nothing is selected (default: "Select…").
    pub fn placeholder(mut self, placeholder: impl Into<String>) -> Self;
}

impl<'a, T, M> From<Select<'a, T, M>> for Element<'a, M>
where
    T: Clone + ToString + PartialEq + 'a,
    M: Clone + 'a,
{ /* iced::widget::pick_list(options, selected, on_selected)
    .placeholder(placeholder).width(Length::Fill).padding(spacing::SM).text_size(type_scale::BODY)
    .style(style::select_field(roles)).menu_style(style::select_menu(roles)) */ }
```

`style::select_field` (mirrors `style::input`'s look: `surface` fill, `outline` border switching
to `primary` on hover/open) and `style::select_menu` (mirrors `style::menu_surface`'s look, plus
`primary`-tinted `selected_background`/`selected_text_color` for the highlighted row) live in
`src/ui/style.rs`, alongside every other themed-widget style function in this codebase.

## Behavior

- **Closed state**: shows the currently selected option's `ToString` rendering, or `placeholder`
  when nothing is selected yet (FR-003) — `pick_list`'s own field rendering, styled by
  `select_field`.
- **Open state**: `pick_list`'s own overlay renders the option list *floating* above the rest of
  the view (positioned from the trigger's on-screen bounds via `Widget::overlay()`), not inline —
  nothing else on the form moves. The row matching the current value is highlighted on open
  (`pick_list` seeds its internal `hovered_option` from `selected`), satisfying FR-003's "indicate
  which type is selected when reopened" with no state this app has to manage.
- **Selecting an item is terminal**: `pick_list` closes its own overlay and emits `on_selected`
  in the same step — the Material "select" idiom (pick → close) is `pick_list`'s built-in
  behavior, not something the consumer's reducer has to arrange.
- **Dismissing without picking**: clicking the trigger again, clicking elsewhere, or Escape all
  close the list unchanged — again `pick_list`'s own behavior.

## First consumer: the add-worktree type field (`src/ui/worktree_form.rs`)

Replaces the former per-`ConventionalType` chip row with:

```rust
let type_select = Select::new(
    ConventionalType::ALL,
    form.type_,
    Message::AddWorktreeTypeSelected,
    r,
)
.placeholder("Select a type…");
```

`ConventionalType` already derives `Clone, Copy, PartialEq, Eq` and implements `Display` (hence
`ToString`), satisfying `Select`'s bounds with no changes of its own.

## Tests (`tests/app_state.rs`)

- `selecting_a_type_sets_the_form_value`: `AddWorktreeTypeSelected(t)` sets `type_ = Some(t)`.
- `type_selection_is_ignored_while_creating`: guarded by `status == Editing`, same as every other
  form-field message.
- No test covers `pick_list`'s own open/closed/highlight-on-reopen behavior — that state is
  internal to the widget (not part of this app's `State`), so it's out of scope the same way
  `GitCli`'s subprocess calls have no unit test: it's a well-tested upstream primitive, not this
  codebase's decision logic.
