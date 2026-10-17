# Contract: required fields in the material components (937)

The UI contract every form relies on (FR-001 to FR-008, FR-010, FR-011). Paths relative to
`crates/micold-client/src/ui/material/`.

## C1 — `.required(FieldRequirement)` on every field builder

Added to the `field_text_builders!` macro's `label_supporting` arm (`form_field.rs`), so
`TextField`, `Select` and `Typeahead` all get it; `FieldText` gains `required: FieldRequirement`
(default `Optional`).

```text
TextField::new("", &draft.name, r)
    .label("Name")
    .required(draft.required.state(FieldId::AddWorktreeName, req))
    ...
```

- `Required` and `Missing`: the label renders as `"{label}*"` (`required::MARKER`), same role, same
  tint, floating and resting alike. A field with no label gets no marker.
- `Missing`: the field is drawn invalid (`FilledFieldState::error`, label and indicator in the
  error role) and the supporting slot shows `required::MESSAGE`, replacing the supporting text.
- An `.error(Some(e))` set on the same field wins: `e` is shown, not the message (FR-010).
- No call site appends `*` to a label or passes `MESSAGE` as an error (gate C5).

## C2 — `Select::on_blur(M)`

Published once when the select goes from engaged (`focused || open`) to not engaged without a pick
in between (research R2). Not published on a pick, nor on a select that was never engaged.

## C3 — `RequiredLegend`

```text
RequiredLegend::new(roles).into()          // "* required", Caption, muted
```

Builder form (Principle VIII), no options today. Forms push it as the first child of their fields
column only when `RequiredMarks::legend(..)` is true (FR-004). Settings pages push it at the top of
the section content.

## C4 — `TrackFocus` for `Select`

`impl TrackFocus for Select<'_, T, Message>`: `.on_blur(Message::Window(WindowMsg::FieldFocusChanged(id, false)))`,
so selects join the one blur route (research R3). The `focused` argument is unused: a select holds
its own focus.

## C5 — Gate `tests/required_call_sites.rs`

Fails when a file under `src/ui/` outside `material/` contains the `LEGEND` or `MESSAGE` literal or
a `.label(` argument ending in `*`, and when an inventory view file (research R5) does not call
`RequiredLegend` (SC-006).

## C6 — Showcase

`showcase/sections/controls.rs` gains a *Required field* entry: legend, then a `TextField` and a
`Select` each in `Required` (empty, resting), `Required` (filled) and `Missing` (error) states
(FR-011). `tests/showcase_completeness.rs` lists it.
