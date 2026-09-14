# Research: The settings rail slides when it collapses and expands

**Feature**: 030-settings-rail-slide | **Plan**: [plan.md](./plan.md) | **Spec**: [spec.md](./spec.md)

Technical Context carried no NEEDS CLARIFICATION: the stack, the motion primitive and the motion
assignment are all fixed by earlier features. What had to be decided is how a stateless component
gets a slide that meets FR-013–FR-015 at once. Geometry used below is today's, measured from
`section_list.rs` and the layout fixture on `origin/main`:

| Quantity | Value | Source |
|---|---|---|
| Rail width, labelled / icons only | 288 / 80 | `RAIL_WIDTH`, `RAIL_WIDTH_COLLAPSED` |
| Rail padding | 8 (`spacing::SM`) | `rendering` |
| Row width at rest, labelled / icons only | 272 / 64 | rail width − 2 × padding |
| Button inset, `Filled` (current row) / `Text` | 24 / 12 | `PADDING_FILLED`, `PADDING_TEXT` |
| Glyph width | 14 | `TypeRole::Action` glyph box |
| Icon x at rest, labelled: other row / current row | 20 / 32 | 8 + inset |
| Icon x at rest, icons only (either variant, centred) | 33 | 8 + (64 − 14) / 2 |
| Largest rest-to-rest icon move | 13 | 33 − 20 |

---

## R1 — Where the slide lives

**Decision**: Split it across two private widgets in `ui/material/section_list.rs`.

- **`Rail`** owns *time*: one `Progress` (R7), advanced by `on_layout_frame`, and the width
  `W = 80 + (288 − 80) · p` at which it lays out its one child — the same
  `container(column(rows, space, control)).padding(SM)` rendering `SectionList` builds today, with
  the container's fixed width removed. Its `size()` stays `Fixed(width_of(collapsed))` so a parent
  sees the destination width, as today. It forwards `update`, `operate`, `mouse_interaction` and
  `overlay` to that child.
- **`RowSlide`** owns *form*: one per destination row and one for the collapse control. It holds
  the row's rest forms (R3), each laid out at its own rest width, chooses one per layout from the
  width it is given (R2), offsets it (R4), clips it (R6), and routes input and operations to it
  alone (R5).

**Rationale**: FR-014 and FR-015 are per-row properties. The icon of the current row and of every
other row start from different rest positions (32 and 20) and end at the same one (33), and a
badged row needs a different form mid-slide from an unbadged one. A choice made once for the whole
rail — the WIP's swap between two complete renderings at the end of the slide — cannot satisfy
both: the WIP measured an icon jump of 13dp at the swap, and a chip clipped out of view on every
frame before its icon turned tinted. Keeping time in one place (`Rail`) and form in the row keeps a
single source of progress, so every row is at the same fraction on the same frame by construction.

**Alternatives considered**:

- *Compose `NavigationDrawer(expanded, collapsed)`* — rejected in D3: the drawer forwards
  operations to its parked child (Tab reaches a hidden copy), loses focus on toggle, and closes to
  0 rather than 80.
- *Keep the WIP's two whole renderings and animate the swap* — cannot meet FR-014 (the swap moves
  every icon at once) or FR-015 (the chip is clipped before the tint appears).
- *One rendering laid out at the current width* — the labels shrink and wrap each frame, which is
  exactly the reflow FR-013 forbids, and the centring of the icons-only form moves with the width.
- *A `Rail`-level custom column that lays out and offsets rows itself* — duplicates `column`'s
  spacing and fill rules, which the rest layout depends on for SC-005.

## R2 — How a row knows how far the slide has gone

**Decision**: From the width it is given. `fraction(w) = clamp((w − 64) / (272 − 64), 0, 1)`,
where `w` is the maximum width in the row's layout limits.

**Rationale**: `Rail` lays its child out at `W`; the container subtracts `2 × 8`; the column hands
each child that width. So `w = W − 16 = 64 + 208 · p` and `fraction(w) = p`, the eased progress,
with no second channel from `Rail` to the rows. It also makes a `RowSlide` correct wherever it is
laid out: at either rest width it reports exactly 0 or 1.

**Alternatives considered**: passing `p` down through a shared `Cell` or a tree-state lookup —
two sources of truth for one number; iced's layout already carries it.

## R3 — Which form a row draws

**Decision**: A row with an icon has up to three forms, each the row as it is drawn at rest today:

| Form | Built as | Laid out at | Exists when |
|---|---|---|---|
| `Labelled` | `row_parts(collapsed = false, ..)` | 272 | always |
| `Marked` | `Labelled`, with the icon tinted as the icons-only form tints it | 272 | the row has a badge |
| `IconsOnly` | `row_parts(collapsed = true, ..)` | 64 | always |

and the form drawn for fraction `f` is the pure function

```text
form(f, has_badge) =
    IconsOnly  if f ≤ ICONS_ONLY (0.001)
    Labelled   if f ≥ FULL (0.999)
    Marked     if has_badge
    Labelled   otherwise
```

The collapse control has `Labelled` (HideSidebar + "Collapse") and `IconsOnly` (ShowSidebar) only.

A `Sliding` row always has three child trees, in the fixed order `[labelled, marked, icons_only]`,
and every slot holds a real form of the same widget type: a row without a badge, and the control,
build `marked` as a copy of `labelled` that is never drawn and is not laid out (a zero-size parked
node). iced diffs children by position and replaces a child's tree whenever the widget's tag
changes (`Tree::diff`), so a placeholder of another type in the `marked` slot would discard a
focused `Marked` form's `Focus` when a badge clears mid-slide (a validation error fixed), before
`layout` could hand focus over; and an optional slot would shift `icons_only`'s tree into the
wrong position when a badge toggles. With same-typed slots, the tree survives and R5's handoff
moves focus to the drawn form.

**Rationale**:

- *No reflow (FR-013)*: every form is laid out at a rest width, so no label is ever measured
  against a width it does not have at rest. What does not fit is clipped (R6).
- *Rest identical (FR-006, SC-005)*: at `f = 0` and `f = 1` the drawn form is today's row, laid out
  at today's width, in today's position.
- *Badges visible (FR-015)*: mid-slide a badged row draws `Marked`, whose tinted icon is inside the
  row from 272 down to 64, and then `IconsOnly`, whose tinted icon is its rest mark. The chip may be
  clipped; the tint never is.
- *`ICONS_ONLY` reused*: the WIP's threshold, below which the width is within 0.2dp of 80, so the
  swap's horizontal change is sub-pixel (R4 carries the rest).
- The tint turns on with the collapse press and off only when an expand settles, which reads as
  the mark "moving" from the chip to the icon rather than blinking.

**Alternatives considered**: switching forms at the midpoint (the chip is clipped long before, so
FR-015 still needs `Marked`, and the label vanishes while there is room for it); tinting the icon in
`Labelled` at rest too (changes the rest state, FR-006).

### R3a — A destination with no icon

**Decision**: A row with no icon has a single form, laid out at the width it is given.

**Rationale**: `row_parts` keeps such a row's label and chip in both states, so its two rest forms
differ only in width. Laid out at the given width, its chip stays at the row's right edge on every
frame (FR-015) and it is never empty. Its label may wrap as the rail narrows and its height
follows. FR-013 exempts this row, SC-007's height clause binds rows with an icon and the collapse
control, and the spec's edge case says the row follows the rail's width. Those three were amended
in the plan phase (ledger D12): as first written, the spec asked this row to keep its height, be cut
off at the rail's edge and show its badge on every frame, and a badged row with no icon cannot do all
three, since cutting a 272-wide form off hides its chip and there is no icon to tint. More basically,
its two rest heights differ (collapsed, its name wraps in 64dp beside the chip), so no design keeps
its height, badged or not. Its height passes between the two rest heights, so the rows below it move
vertically and the column is never taller than at its taller rest state (FR-009 holds). No shipped
rail has such a row (Settings and the showcase give all four destinations an icon), so the case is
held by a component-level test only.

**Alternatives considered**: laying it out at 272 like the others (its chip is clipped away while
the row still shows a label, failing FR-015); a no-wrap single line at the current width (its
height then jumps on the first or last frame, where it meets its wrapped rest form); moving the
chip in front of the label mid-slide (a different row from either rest form, and a jump when it
moves back).

## R4 — Keeping the icon on its line

**Decision**: The drawn form is translated sideways by

```text
offset(f, x_labelled, x_icons, x_drawn) = x_icons + (x_labelled − x_icons) · f − x_drawn
```

where `x_labelled` and `x_icons` are the icon's rest x in the `Labelled` and `IconsOnly` forms and
`x_drawn` is its rest x in the form being drawn, each read, every layout, as the x of the **first
leaf node in depth-first order** of that form's laid-out node, relative to that node and before any
form is parked or offset (a parked node's absolute x, about −8.5e37, has no precision left at dp
scale). The glyph is that leaf in every form
that has one, at whatever depth: `button/0/0/0` labelled, one level deeper icons-only because of the
centring container. A unit test pins the lookup against both depths. The collapse control uses the
same rule on its HideSidebar/ShowSidebar glyph. A `Single` row has no icon and takes no offset.

**Rationale**:

- The icon's drawn x is then `x_icons + (x_labelled − x_icons) · f` on every frame, in every form,
  so it is continuous across the form swap (FR-014) and moves by exactly
  `|x_labelled − x_icons| · Δf` between frames, never away from the target (SC-007's bound with the
  0.5dp to spare for float rounding).
- Reading positions from laid-out nodes, not from the inset constants, keeps the rule true if the
  button's inset or glyph size change.
- *The selection exception comes for free*: selecting a section mid-slide rebuilds the row with a
  different variant, which changes `x_labelled` (20 ↔ 32) and nothing else, so the icon moves at once
  by `12 · f` — 12dp expanded, 0 collapsed, in proportion between — which is FR-014's exception to
  the letter.

**Alternatives considered**: interpolating the whole row's x between its two rest rectangles
(the icon's offset inside the row differs per variant, so the icon still jumps); animating the
inset (a layout change per frame, which is reflow).

## R5 — Focus, keyboard reach and pointer input

**Decision**:

- `operate` visits only the drawn form. Parked forms are invisible to every operation, so Tab
  reaches each drawn control exactly once (FR-008) and the settings section's own order is untouched.
- Focus follows the drawn form without remembering which form was drawn before. On every `layout`,
  after choosing the form, `RowSlide` asks each parked form whether one of its focusables is
  focused; if one is, it unfocuses that parked form and focuses the same-numbered focusable in the
  drawn form, before the frame is drawn (FR-007, SC-003). The handoff carries the whole focus
  state, not only "focused": `TakesTheKeyboard`'s `Focus` records whether focus came by keyboard
  (`visible`), and a pointer press clears it (BUG-013). `Focusable::focus()` sets `visible = true`,
  so the WIP's `FocusNth`, which calls it, would turn a click on **Collapse** into a focus ring on
  ShowSidebar that lasts through rest, where `origin/main` shows none (FR-006). So
  `TakesTheKeyboard::operate` also offers its `Focus` through `operation.custom`, and two crate-private
  operations in `section_list.rs` use it: `TakeFocus` (the index of the focused `Focus` in a subtree
  and its `visible`, clearing both) and `GiveFocus { index, visible }` (set on the n-th, clear the
  rest). Each runs over one form's tree with a `Layout` built from that form's node, and downcasts
  what `custom` offers with `downcast_mut::<Focus>()`, skipping anything else. `custom` is offered
  under the same `enabled` condition as `focusable`, so the n-th `Focus` is the n-th focusable (`Ripple::operate`
  already offers its `RippleState` the same way).
  The step is idempotent: when nothing parked holds focus it does nothing, so a layout from a
  rebuild with no frame, or a second layout on the same frame, changes nothing. This is done in `layout` because that is where the form
  is chosen: a handoff deferred to the next `update` would leave one drawn frame with focus on a
  parked control.
- `update` and `mouse_interaction` reach the drawn form, with the cursor passed as unavailable
  when it is outside the `RowSlide`'s own bounds (FR-009). The drawn form can be wider than the row;
  without the mask its clipped-off part would still take hover and presses.
- Parked forms receive a narrow set of events, always with the cursor unavailable:
  `RedrawRequested` (a `Button`'s status lives on the widget, is rebuilt with every view and is
  computed only on that event; a ripple advances on it), and left-button release, finger lifted or
  lost, and cursor left (the button's pressed flag is tree state, and a press held when its form
  parked is released rather than frozen). Nothing else reaches them: no press, no key. They are
  updated against a local `Shell`: messages and event capture are dropped, and redraw requests and
  layout or widget invalidation are copied to the outer shell. So a ripple that was running when its
  form parked finishes on schedule instead of freezing, and keeps frames coming only for its own
  duration; once it and the slide have settled, nothing requests a frame (FR-011).

**Rationale**: The WIP established that handing focus over by operation works with the client's
`Button`; what changes is only that the handoff is per row, so it happens at the frame that row
changes form rather than once for the whole rail.

**Alternatives considered**: keeping every form operable and filtering by visibility in the
settings screen (leaks the component's internals into every screen that uses it, Principle VIII);
not handing focus over and relying on the user to re-focus (fails FR-007); remembering the last
drawn `RowForm` in tree state and handing over only on a change, as the WIP's `labels_shown` did
(a second record of what the width already says, which a rebuild can leave stale); handing over
with `Focusable::focus()` (shows a ring pointer focus never showed, FR-006); resetting a parked
form's tree instead of forwarding releases (drops a ripple and the focus state along with the press).

**Consequence for hover**: `Button` sets its status on `RedrawRequested`, and draws
`status.unwrap_or(Disabled)`. Because parked forms receive every `RedrawRequested` (with the cursor
unavailable), a form becoming drawn already has a status and never draws the disabled look. When the
layout changed, iced's runtime updates the interface again with the same `RedrawRequested` before
drawing, so the newly drawn form also has its hover status for that frame.

## R6 — Clipping

**Decision**: `RowSlide::draw` draws its one form inside `renderer.with_layer(bounds, ..)` when that
form's bounds are wider than the row's; otherwise it draws it directly. `Rail` does not clip: its
child is laid out at its own width, so nothing it lays out extends past it.

**Rationale**: Clipping at the row rather than at the rail keeps each row's own inset visible on
its right side as it narrows, and the current row's filled container ends at the row's edge rather
than at the rail's. The rail's bounds contain every row's, so FR-009's "draws nothing outside its own
bounds" holds.

**Alternatives considered**: one clip at the rail (the WIP): the filled container of the current
row reaches the rail's edge and then loses its 8dp right inset at the swap.

## R7 — The curve, and the sidebar

**Decision**: Both `Rail` and `NavigationDrawer` build their track as
`Progress::new(initial).easing(EMPHASIZED.x1, EMPHASIZED.y1, EMPHASIZED.x2, EMPHASIZED.y2)` with
duration `MEDIUM_4`. `Rail` names the two tokens as `SLIDE` and `SLIDE_CURVE`, as the WIP does.

**Rationale**: 018 §6.3 *sidebar slide* assigns `medium_4` and `emphasized`; the drawer set only the
duration and so defaulted to linear (D7). Both panels width their content by `full · progress`, so
equal progress at equal elapsed time is equal fraction of width change, which is SC-002's second
sentence. No existing test pins the drawer to linear: its unit tests assert which child is laid out
and its gate asserts relayout requests.

**Alternatives considered**: a shared `sidebar_slide()` constructor in `cdk::motion` — the cdk names
no tokens (`tests/cdk_no_appearance.rs`), so it would have to live in `material`, for two call sites;
not worth a module until a third panel appears.

## R8 — No new motion assignment

**Decision**: Record in plan.md that the rail uses the *sidebar slide* row as it stands. 018's
contract is not edited and no row is added.

**Rationale**: The spec's Assumptions settle it; the trigger "a side panel changes width" describes
the rail. `motion_tokens.rs` checks that every duration is a named token, and `SLIDE` is.

**Alternatives considered**: adding a *rail slide* row to 018's contract (edits another feature's
contract, outside this flow, for a trigger, duration and curve the existing row already describes); a shorter duration for
the narrower travel (the request is that the two move alike, FR-003).

## R9 — How the slide is tested

**Decision**: Three layers, following the constitution's split.

1. **Pure functions, unit tests in `section_list.rs`**: `fraction`, `form`, `offset` — their
   boundaries (`ICONS_ONLY`, `FULL`, clamping), the selection case, and the no-icon case.
2. **Mounted surface, `tests/settings_rail_motion.rs`** (harness from the WIP: mount the settings
   surface, press the control, pump layout frames at chosen instants, read nodes and run operations).
   The WIP's frame runs one layout and one `update(RedrawRequested)` with the cursor unavailable;
   it is extended to match iced_winit's loop: a frame takes an optional cursor position and repeats
   `update` with the same instant, then relays out, while the layout was invalidated, at most three
   times. Hover and held-press tests are only meaningful against that loop.
   One test per observable requirement: widths strictly between (FR-001, SC-001); section region at
   the rail's edge with its content offset (FR-002); reversal from the width reached with no step
   larger than an uninterrupted slide's (FR-004, SC-004); mounted at final width (FR-005, SC-006);
   focus and reachable count per frame (FR-007, FR-008, SC-003); pointer outside a row's bounds
   reaches nothing (FR-009); state flips on press (FR-010); no redraw request once settled
   (FR-011); row heights and per-frame icon movement (FR-013, SC-007); icon continuity across the
   swap and the selection exception (FR-014); a badged row's drawn form has its tinted icon or its
   chip inside the rail on every frame (FR-015). Two tests need components Settings does not
   compose — the rail beside a `NavigationDrawer` driven by the same instants (FR-003, SC-002) and a
   rail with a badged no-icon row (FR-015) — and `ui::material` is `pub(crate)`, so they mount the
   components in `section_list.rs`'s test module on `test_support::renderer()` instead.
3. **Rest layout**: `tests/layout_snapshot.rs` against the regenerated fixture, plus the check in
   quickstart §A.3 that the *visible* rectangles (parked nodes, x = `-inf`, excluded) of every
   fixture state, both settings states included, are the same set as on `origin/main` (FR-006,
   SC-005). The showcase is not in the fixture; its rest look is quickstart §B.4.

What stays with the recorded visual pass (quickstart §B): the clip itself and how the tint and the
slide look.

**Rationale**: Every branch the slide takes is in layers 1–2; layer 3 is the rest-state guarantee
that makes the regenerated fixture reviewable.

## R10 — Fixture and anchors

**Decision**: Regenerate `tests/fixtures/layout_snapshot.txt` and move the `settings.rail` anchor in
`tests/support/covered_states.rs` to the new tree path (the drawer wrapper's levels go, and `Rail`
adds one). It keeps naming the rail's padded container, as on `origin/main`, so that
`tests/gates/rail_icons_align.rs`, which takes rows two levels below the anchor, finds the
`RowSlide` nodes as its rows; the implementing task reads the path from the regenerated fixture. That gate is not changed: its
`glyph()` takes the *last* node under a row narrower than the row, and with the fixed child order
`[labelled, marked, icons_only]` that is the drawn `IconsOnly` form's glyph in the collapsed state
it checks. (A filter on non-finite x would do nothing: the records hold raw bounds, where a parked x
is finite; `-inf` appears only in the fixture text after `normalise`.) Source-scanning gates to keep
green:

- `animated_layout_relayouts.rs`: `Rail::layout` reads its track's value and `Rail::update` calls
  `on_layout_frame`; `RowSlide` holds no track and reads none.
- `one_overlay_implementation.rs`: counts constructions of `overlay::Element::new(`; both widgets
  only forward their child's overlay.
- `cdk_no_appearance.rs`: nothing is added to `cdk`.

**Rationale**: The fixture is the rest-layout record; a tree change that adds nodes must be recorded
there, and quickstart §A.3 makes the regenerated file reviewable.

**Alternatives considered**: excluding parked nodes from the snapshot writer (changes a shared gate
for every feature to make one diff smaller, and would hide a parked node that stopped being parked).

## R11 — Reusing the WIP

**Decision**: Start Phase 4 from WIP `14d9c0e4` (patch kept in the session scratchpad) for
`settings_view.rs`, `support/layout.rs`, the `Rail` shell, the test harness and the user-guide
paragraph, and replace its two-rendering form choice with R1–R6. Its 027 spec, plan, task and bug
edits are not reused: D5 withdrew them, and BUG-004 as merged is the record.

**Rationale**: The WIP's routing, progress wiring and harness were exercised against the real
surface; only its form choice is wrong for this spec. The user-guide paragraph needs the expand
icon added for FR-012.

**Alternatives considered**: starting from `origin/main` (discards a harness and routing already
run against the real surface); taking the WIP whole and patching the swap (its whole-rail form choice
is the part R1 replaces, so little of `Rail::layout` would survive anyway).
