# Contract: the shared picker base

**Modules**: `src/ui/cdk/picker.rs` (renamed from `cdk/typeahead.rs`),
`src/ui/material/picker.rs` (extracted from `material/typeahead.rs`).

Two halves, split the way the library is already split: `cdk` decides where the list sits, what it
captures and when it closes; `material` decides what any of it looks like. Neither half is new — both
are generalisations of what one control already had, and the contract below records what changes and
what deliberately does not.

---

## §1 The behaviour half (`cdk::picker`)

### C1.1 — It composes, it does not draw

`Picker::new(field, menu, open, gap)` takes both halves as already-built elements. It learns nothing
about either. This is unchanged from `cdk::typeahead` and is what lets one mechanism serve a text
field with a result list and a pressable trigger with an option list.

### C1.2 — Anchoring and flipping

The list is returned from `Widget::overlay()`, so the rendering stack positions it from the field's
own on-screen bounds — independent of the parent layout's size constraints, which is what makes it
work inside a content-sized dialog (FR-005, FR-006). Directly beneath the field and exactly as wide;
flips above when there is not room below; never leaves the window. Unchanged.

### C1.3 — Dismissal

A press outside both the list and the field closes it. A press *inside* the field does not — it is an
ordinary click on the thing you are already using. Unchanged.

### C1.4 — The keyboard

Down and up move the highlight, Enter takes the highlighted row, Escape dismisses, Tab dismisses
**and passes through** so focus still moves. The rule itself is
`micold_core::typeahead::intent_for` — render-free, tested, and shared rather than reimplemented
(FR-014). Keys the list claims are captured, so taking a row does not also submit the dialog behind
it. Unchanged.

### C1.5 — **New**: the list outlives its closing

`overlay()` today returns `None` the instant `open` is false, which removes the widget and its
animation state together — so a closing list vanishes between frames instead of leaving.

The base gains one input and one piece of tree state:

- `exit: Duration` — how long the list keeps existing after `open` goes false. A bare value crossing
  into the cdk, for the same reason `gap` already does: how long a thing takes is appearance, and the
  material layer resolves `SHORT_2` and hands the result over.

  > **Corrected during implementation.** This said `f32`, "in the same per-frame units `gap` is in
  > pixels". `Duration` is what `cdk::motion` already trades in — `Progress::on_frame` takes one, and
  > `step_for` converts — so `f32` would have meant converting twice and naming a frame budget in a
  > second place. The rule the contract cares about is unchanged: the cdk names no token, and
  > `cdk_no_appearance.rs` still passes because a `Duration` is not a design token.
- `Visibility { progress: Progress }` — falling from 1.0 to 0.0 across `exit` once closed.

**Invariant**: `progress > 0` ⟺ `overlay()` returns `Some`.

**Two things this must not do.** It must not request a frame at rest — `idle_requests_no_frames.rs`
fails the build if it does. And a list below the visibility threshold must accept no pointer or
keyboard input (FR-022): it is on screen only in the sense that it is still fading, and a press
landing where a row used to be must do nothing.

### C1.6 — Still no appearance

`cdk_no_appearance.rs` reads these sources and fails on a colour role, a type role, a shape size or
the styling layer. `exit` is a number with no unit named in this file, exactly as `gap` is.

### C1.7 — Still the only hand-written overlay in the cdk

`one_overlay_implementation.rs`'s `CDK_OVERLAY_IMPLEMENTORS` holds one entry and its documentation
says "empty is the correct state". The entry's *path* changes with the rename; its count does not.
**`SANCTIONED`'s `select.rs` / `pick_list` entry is removed** — the gate's staleness check fails the
build while a sanction that no longer applies is still listed, so this is forced rather than
remembered.

---

## §2 The presentation half (`material::picker`)

### C2.1 — One row anatomy, both pickers

`row_element(row, highlighted, selected, press, roles)` renders Material's menu item as this library
assembles it: a leading slot, a label, a pressable container carrying the state layer, and a ripple
when there is something to press. Four channels stay deliberately distinct — emphasis, highlight,
selection, disabled.

| Property | Value | Source |
|---|---|---|
| Height | `density::MENU_ITEM_BASE` | §7.5 |
| Horizontal padding | `spacing::SM` | §7.5 |
| Label role | `TypeRole::Body` | **deviation, see C2.2** |
| Selected | tonal fill + leading marker glyph | §7.5 states, contract 021 §4.7 |
| Unselected | the marker's space is reserved, so every label starts at the same x | 021 §4.5 |
| Disabled | label muted, no accent, unpressable, **no ripple** | 021 §4.6 |

### C2.2 — The row label is `Body`, not §7.5's `label_large`

A knowing deviation, recorded here so it is a decision rather than a drift.

§7.5 gives a menu item `label_large`. The search picker's rows are `Body` instead, because part of a
row is *emphasised* and `Action` (this library's `label_large`) is already the medium weight —
emphasis would have nowhere to step up to. A select row carries no emphasis and could take `Action`.

**Both take `Body`.** The feature exists to make the two lists indistinguishable, and typography is
the most visible property there is; a select whose rows were set one weight apart from the search
picker's would fail SC-001 on the first comparison. The generic `material::menu` component is
untouched and keeps §7.5's role — the deviation is scoped to the two pickers, which are the two
controls a person compares side by side.

### C2.3 — One list surface, both pickers

`menu_element(...)` builds the panel on `menu_panel` — the same surface every floating popover in the
app sits on, so elevation, corner and padding are the menu surface's rather than this component's. It
caps at `MAX_ROWS_BEFORE_SCROLL` rows (expressed in rows × the density scale's item height, not a
pixel figure) and scrolls beyond.

An open list with no rows and an empty-message shows that message, muted, as prose rather than as a
row. An open list with no rows and no message occupies nothing.

### C2.4 — **New**: the transition

`animated_menu(panel, open, roles)` wraps the panel:

| | Value | Source |
|---|---|---|
| Enter | `SHORT_3` (150 ms), `STANDARD_DECELERATE` | §6.3 "menu fade in" |
| Exit | `SHORT_2` (100 ms), `STANDARD_ACCELERATE` | §6.3 "menu fade out" |
| Form | `scale` from `MIN_SCALE` (0.96) + `fade` | `material/animation.rs`, existing |

Both curves are `Motion`'s defaults, so neither is restated at the call site. `scale` transforms
**drawing only** — it delegates layout, events and the overlay to its child — which is how FR-023
("nothing outside the list moves") holds by construction rather than by care.

**No new motion token, and no new entry in §6.3.** This is an animation that table already assigns,
reaching a surface that was not drawing it. Feature 018's count of sanctioned new animations
(FR-035a: four, "no fifth animation is permitted") does not move, and SC-007 checks that.

### C2.5 — Interrupting

A transition reversed mid-flight continues from where it is. This is `Progress`'s existing behaviour
and needs nothing; it is stated because FR-021 asks for it and a reader should know where it comes
from.

---

## §3 What the two pickers still decide for themselves

The base is not a template. Each control keeps what is genuinely its own:

| | Search picker | Select |
|---|---|---|
| Field | text input, leading search icon, trailing clear | pressable row: value or placeholder, trailing chevron, state layer, ripple |
| Rows from | matched and ranked results, with emphasis spans | options, `ToString`, no spans |
| Openness | the caller's (coupled to the query) | its own (coupled to nothing) |
| Highlight | the caller's | its own |
| Empty list | "no branches match that search" | the options list is fixed; an empty one is a call-site error, not a search result |

---

## §4 Gates this contract must leave green

Named rather than assumed, because several of them are the reason a decision above went the way it
did.

| Gate | What it holds here |
|---|---|
| `cdk_no_appearance.rs` | `exit` is a number; no role, size or style function in `cdk/picker.rs` |
| `material_boundary.rs` | `pick_list` leaves `WRAPPED_WIDGETS` once nothing wraps it |
| `material_builder_api.rs` | `Select` is still constructed with required inputs and terminates in `.into()` |
| `component_api_opacity.rs` | no progress value appears in any public signature — which is why both tracks are widget-owned |
| `one_overlay_implementation.rs` | one cdk implementor, and the `select.rs` sanction **removed** |
| `typeahead_is_generic.rs` | follows the rename; the rule (no branch/worktree/git in the component) is unchanged and now covers one more file |
| `idle_requests_no_frames.rs` | a settled picker asks for no frames |
| `logical_state_ownership.rs` | openness and highlight are presentation, by that file's own "screen switched off" test |
| `showcase_completeness.rs` / `showcase_captions.rs` | both pickers catalogued; the select becomes `interactive` with a non-empty `live` list |
| `anatomy_size.rs` / `content_placement.rs` | the trigger states its height and puts its content where §7.7 says |
| `tokens.rs` | the select's new pairings clear AA in both schemes |
