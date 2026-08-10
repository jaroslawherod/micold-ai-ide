# Implementation Plan: Dedicated Select Component on a Shared Picker Base

**Branch**: `feat/dedicated-select-component` | **Date**: 2026-08-07 | **Spec**: [spec.md](./spec.md)

**Input**: Feature specification from `/specs/022-dedicated-select-component/spec.md`

**Bugfix**: 2026-08-09 — [BUG-003](./bugs/BUG-003.md) See "The focus nobody reported".
**Bugfix**: 2026-08-09 — [BUG-002](./bugs/BUG-002.md) Updated from bugfix patch. See
[Interaction states](#interaction-states-bug-002).

## Summary

Make the select a component of this library rather than a re-coloured `pick_list`, put it and the
search picker on one foundation, and animate both lists open and closed.

The approach is **extraction, not invention**. Phase 0 found that nearly every part already exists:
`cdk/typeahead.rs` is already a generic anchored-list mechanism that takes both halves as opaque
elements; `material/typeahead.rs` already has the row and panel presentation; `material/animation.rs`
already has `scale` and `fade` whose *default* curves are the two rows §6.3 assigns to a menu. The
work is to generalise the first two so a second control can consume them, compose the third around
the list, and delete `pick_list`.

One thing is genuinely new and it is small: the base's `overlay()` returns `None` the instant the list
closes, which takes the animation state with it — so a closing list has nothing left to fade. That is
the only unknown, and it is scheduled first (research R10).

Two claims are worth stating up front because they shape the slices. **This deletes more than it
adds**: three test-support files carry machinery whose sole purpose is reaching inside `pick_list`,
and that machinery dissolves once the list is composed in-tree. And **accepted fidelity gap #3 is
closed structurally** — a component that owns its open state has nobody to ask, so `Select::active`
stops existing rather than staying unanswered.

## Technical Context

**Language/Version**: Rust, stable toolchain via `mise`

**Primary Dependencies**: iced 0.14 (no new dependency; this feature *removes* a use of `pick_list`)

**Storage**: N/A — no persisted state (see data-model §5)

**Testing**: `cargo test --workspace` via `mise run test`; render-free logic via `mise run test-core`.
Component behaviour lands in tested render-free or widget-tree-inspectable code; the render glue is
covered by [quickstart.md](./quickstart.md) §B under Principle I's GUI-wiring exception

**Target Platform**: Linux, macOS, Windows (desktop)

**Project Type**: Desktop application — three-crate Rust workspace

**Performance Goals**: 60 fps; the transitions are 150 ms in and 100 ms out; a settled picker must
request **zero** frames (`idle_requests_no_frames.rs`)

**Constraints**: no new motion token, colour role or spacing step; feature 018's count of sanctioned
new animations must not rise (FR-020, SC-007); the existing consumer's behaviour is bit-for-bit
unchanged (FR-030, SC-009)

**Scale/Scope**: two components, one shared base in two halves, one consumer, one gallery page, four
documents. Roughly: `material/select.rs` rewritten (~145 lines today), `cdk/typeahead.rs` renamed and
extended (~460), `material/typeahead.rs` split (~726), plus the `pick_list` removals in `style.rs`,
`style_snapshot.rs` and four test files

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

- [x] **I. Test-First (NON-NEGOTIABLE)**: PASS. The open/close rule, the keyboard rule and the
  visibility invariant are all decisions and land under tests written first. The keyboard rule is
  already render-free in `micold-core` and is *reused*, not reimplemented. The render glue —
  `worktree_form.rs`'s call, the gallery's entries — is the narrow category the GUI-wiring exception
  covers and is validated by [quickstart.md](./quickstart.md) §B. Note what the exception does **not**
  cover here: `overlay()`'s visibility invariant is decision logic, so it is tested through the widget
  tree, not waved through.
- [x] **II. Multi-Session Support**: PASS — not engaged. No session state; see data-model §5.
- [x] **III. Worktree Integration**: PASS — not engaged. No file or VCS operation.
- [x] **IV. Local-First Storage (NON-NEGOTIABLE)**: PASS — not engaged. No new file, directory, key or
  network call. Nothing here is persisted.
- [x] **V. Rust + iced Stack**: PASS. Rust and iced only. The type system carries two invariants:
  `progress > 0 ⟺ overlay() returns Some` is structural rather than checked, and removing
  `Select::active` makes "an open select with a resting indicator" unrepresentable rather than merely
  unlikely.
- [x] **VI. Cross-Platform Parity**: PASS. Nothing platform-conditional; CI covers all three.
- [x] **VII. Documentation First-Class**: PASS. Four documents ship in the same change:
  `design-tokens.md` §7.7 and §9 (the gap list drops to three),
  `docs/development/component-library.md`, and a superseding note on feature 013's contract.
- [x] **VIII. Reusable UI Component Foundation**: PASS, and this is the principle the feature exists
  to serve. Nothing is forked: the select consumes the base the search picker already uses, and where
  the two lists differ today they converge on one definition. `Select` keeps the chainable builder
  terminating in `.into()`, `Roles` still arrives through the constructor, and the base is *promoted*
  from one control's private mechanism into the shared primitive its docs already described it as.

### Re-check after Phase 1 design

No violations. Two design decisions are worth recording because a reviewer will reasonably ask about
each, and both are argued in full in the artifacts rather than here:

1. **The two pickers own their openness differently** (data-model §2.2). The select's is
   widget-owned; the search picker's stays caller-owned because its list content is derived from
   caller-held query state. Both sides of that split satisfy `logical_state_ownership.rs`'s own
   "screen switched off" test. Forcing symmetry would reintroduce the very gap this closes.
2. **Both pickers' rows are set in `Body`, deviating from §7.5's `label_large`** (contract
   picker-base §C2.2). The deviation is scoped to these two controls — the generic `material::menu`
   keeps §7.5 — and exists because typography is the most visible property a side-by-side comparison
   has. This is the one place the feature knowingly disagrees with an existing contract; it is
   recorded in the contract, in the spec's Assumptions, and here.

Neither needs a Complexity Tracking entry: no principle is violated, and no simpler alternative was
rejected for convenience.

## Project Structure

### Documentation (this feature)

```text
specs/022-dedicated-select-component/
├── plan.md                        # This file
├── research.md                    # Phase 0 — twelve findings, all resolved against existing code
├── data-model.md                  # Phase 1 — state, and where each piece lives and why
├── quickstart.md                  # Phase 1 — §A automated gates, §B the manual pass
├── contracts/
│   ├── picker-base.md             # the shared foundation, both halves
│   └── select-component.md        # the select itself, its consumers, and what is removed
├── checklists/requirements.md     # written by /speckit-specify
└── tasks.md                       # Phase 2 — /speckit-tasks, NOT created here
```

### Source code (repository root)

```text
crates/
├── micold-core/
│   └── src/typeahead.rs                 # unchanged — the keyboard rule is reused as-is
└── micold-client/
    ├── src/ui/
    │   ├── cdk/
    │   │   ├── mod.rs                   # picker replaces typeahead
    │   │   └── picker.rs                # ← renamed from typeahead.rs; + exit, + Visibility
    │   ├── material/
    │   │   ├── mod.rs                   # exports
    │   │   ├── picker.rs                # ← extracted from typeahead.rs: rows, panel, transition
    │   │   ├── typeahead.rs             # keeps the search field; consumes material::picker
    │   │   ├── select.rs                # ← rewritten: trigger + base, no pick_list
    │   │   ├── style.rs                 # select_field/select_menu retyped or retired
    │   │   ├── style_snapshot.rs        # pick_list poses removed
    │   │   └── form_field_anatomy.rs    # its Select fixture follows the API change
    │   └── worktree_form.rs             # consumer — call unchanged
    ├── src/showcase/
    │   ├── catalogue.rs                 # Select becomes interactive + live
    │   └── sections/controls.rs         # the select entry drives the real rule
    └── tests/
        ├── one_overlay_implementation.rs  # SANCTIONED loses select.rs/pick_list
        ├── material_boundary.rs           # WRAPPED_WIDGETS loses pick_list
        ├── typeahead_is_generic.rs        # follows the rename
        ├── support/layout.rs              # pick_list special-casing dissolves
        ├── support/covered_states.rs      # ditto
        └── layout_snapshot.rs             # ditto, plus the fixture
docs/development/component-library.md      # Principle VII
specs/018-material3-visual-system/contracts/design-tokens.md   # §7.7, §9 — the gap list
```

**Structure Decision**: the existing three-crate workspace and its two-layer component library
(`ui/cdk/` behaviour, `ui/material/` appearance) are unchanged. This feature adds no crate, no module
directory and no architectural layer — it moves code *within* the layers that exist, which is the
whole argument for calling it an extraction.

## Delivery order

Sliced so each is independently testable and shippable, and **deliberately not in the spec's story
order** — see the note below.

| Slice | Delivers | Gate |
|---|---|---|
| **0 (risk first)** | the base's exit visibility (`exit`, `Visibility`, the `progress > 0 ⟺ Some` invariant) and the transition wrapper, both landed on the **search picker**, which already has an open/close rule and a live gallery entry | `mise run test` incl. `idle_requests_no_frames.rs`; quickstart §B2 on the type-ahead alone |
| **1 (US1, P1)** | the base and presentation generalised and renamed; the select rewritten on them; `pick_list` and its six removal sites gone; the consumer unchanged | full suite incl. `one_overlay_implementation.rs`'s staleness check; quickstart §B1, §B3–§B6 |
| **2 (US2, P2)** | the select's list animates — which by slice 0 is one call | quickstart §B2 on both |
| **3 (US3, P3)** | the gallery entries, the four documents, the gap list dropping to three | `showcase_captions.rs`; quickstart §B7 |

**Why slice 0 inverts the story order.** User Story 2 (motion) is the spec's P2, behind the anatomy
work. But the *mechanism* motion needs — a list that outlives its own closing — is the one part of
this feature that does not already exist, and building it against a control that already works means
an unpleasant surprise arrives before a second control depends on it. Feature 021 made the same call
for the same reason with its hand-written `overlay()`. The spec's priorities are about user value and
are unchanged; this is about where the risk sits.

Once slice 0 lands, slice 2 really is one call — which is why it is listed after slice 1 despite being
the higher-priority story. Nothing is deferred by this; the animation ships in slice 0 for one picker
and slice 2 for the other.

## Risks

| Risk | Handling |
|---|---|
| An exit track that never settles burns frames forever | `idle_requests_no_frames.rs` fails the build on it, and slice 0 is where it would surface. The invariant is stated as `progress > 0 ⟺ overlay() returns Some` precisely so "still fading" and "still on screen" cannot disagree |
| A fading list still accepts a press | FR-022 and contract C1.5. The wrapper is inert below the hidden threshold, but *inert* has to mean the overlay refuses input, not merely that it draws nothing — tested rather than assumed |
| Removing `pick_list` ripples into six sites, three of them test-support | Counted in research R5 and contract §5 before planning rather than discovered during it. Two of the six are gates that **fail the build** until attended to — `one_overlay_implementation.rs`'s staleness check and `material_boundary.rs` — which is the good kind of ripple |
| The rename (`cdk::typeahead` → `cdk::picker`) touches a path-scanning gate | `typeahead_is_generic.rs` scans by path; the rename is mechanical and the compiler finds the rest |
| The select's trigger gains a fixed height and puts its content at the top | BUG-001 and BUG-002 of feature 018 were both exactly this. `anatomy_size.rs` and `content_placement.rs` already exist because of them and already cover the class |
| Losing `pick_list`'s highlight-on-open | Contract §2 states it as a requirement rather than trusting it to survive: the highlight is seeded from the current choice on open. Feature 013's FR-003 is the thing that would silently regress |
| SC-001 and SC-002 cannot be automated | Stated plainly in quickstart §B's preamble rather than papered over. A green suite is not this feature working, and the recorded pass says which half was machine-checked |

## Interaction states (BUG-002)

*Added 2026-08-09 by the bugfix patch, for FR-034 – FR-036.*

**The layer belongs to the container, not to what sits in its slot.** `FilledField` lays its control
into one value line inside the field's 16dp padding, and `Select` paints its layer on the control —
so the layer is 440×24 in a 472×56 field while hover and press are read off the whole field. Moving
the layer onto `FilledField`, which is the thing that knows the container's bounds, fixes the select
and gives the text field and any future field the same treatment without restating it. Patching
`Select` alone would leave the arrangement that caused it.

That places three of the new requirements in the shared field rather than in either picker:

- **FR-034** (extent) — `FilledField` draws the layer over its own bounds; `Select` stops drawing one.
  `Ripple` moves out to the same bounds so a press in the padding ripples, per FR-010.
- **FR-035** (focus) — needs a focused flag the field can read. The text field has one to offer
  (`text_input` reports focus); the select owns its own, as it already owns `open`. Note the select
  is the harder half: `open` currently maps to the *pressed* opacity, and open-and-focused must not
  double up.
- **FR-036** (hover) — `style::field_input` currently discards the `text_input::Status` it is handed,
  which is why a text field shows nothing. The status is already delivered; nothing new is plumbed.

- **FR-036a** (no new token) — costs nothing to satisfy: `state::FOCUS` already sits in
  `crates/micold-core/src/tokens/state.rs` beside `HOVER` and `PRESSED`, published by feature 018 and
  consumed by no input. The work is to *use* the opacity that exists, not to add one, exactly as
  FR-020 required of the menu timings. The contrast gate is where this is held (T047): a new pairing
  is measured, never a new value.

**Ordering**: the extent fix is independent and can land first. Focus and hover share one mechanism
and should land together — building focus alone would put the same layer in twice.

**Risk, and it is the one BUG-001 left behind**: none of this is observable to a test that never
re-runs `view()`, and the anatomy gates assert element sizes rather than the relationship between
two rectangles. The new gate has to compare the shaded rectangle with the pressable one, which is a
kind of assertion this suite does not yet make anywhere.

## The focus nobody reported (BUG-003)

*Added 2026-08-09, for FR-031 (feature 018), FR-034 and FR-035.*

The section above put the focus layer in the field and left one thing unexamined: **who tells the
field it is focused**. Nothing did, at any point in two features. `FormField::active` is supplied,
not observed — correctly, since the state that thickens the indicator is focus for a text input and
*open* for a picker — and no `TextField` call site in the application ever passed it. The component
obeyed, the anatomy gates proved it obeyed, and every field in the running application drew
permanently at rest: label unfloated, indicator a hairline, focus layer never once painted.

**The fix is split at the line the label draws.** Neither of the two routes BUG-003 offered works
alone:

- Observing focus inside `FilledField` **cannot float the label**. `label_floats` decides the
  label's type role and tint inside `FormField::from`, at *build* time. A widget noticing focus
  afterwards can move an element; it cannot change what that element already is. That is symptom 1,
  and it is the one a person notices.
- Screens tracking focus **have no event to track**. iced publishes nothing when a text input gains
  focus; that is the sentence `TextField::active`'s doc opens with, and why `cdk/picker.rs` proxies
  focus with a press inside its bounds.

So the field asks and the application keeps the answer. `FilledField` runs the rendering stack's own
focus traversal over its control after each event — the input's state is the only copy of the fact,
so there is no second opinion to disagree with, which is BUG-002's lesson applied to the keyboard —
and publishes changes. The application holds one `Option<FieldId>` and hands it back on the next
view, in time for the label.

**FR-034 runs both ways.** BUG-002 read it as "grow the layer to the responsive area", which is the
direction the select was wrong in. The text field was wrong in the other: a 24dp control in a 56dp
box meant most of a field shaded and hovered while accepting no press at all. A press on the
container now reaches the control, adornment slots excepted.

**Risk, and it is the one that let this happen**: a component gate that poses its own input can
never see an unwired call site. `field_focus_call_sites.rs` is the answer and it reads source text
rather than pixels — deliberately, because the bug was found by a grep and the property is a
property of the source.

**And the checkbox, whose gap was never really a styling one.** FR-035 recorded the checkbox as out
of reach because `checkbox::Status` has no focused variant to attach a layer to. That is true and it
is the symptom. The cause is that the rendering stack's checkbox **cannot be focused at all**: its
widget state is the label's shaped paragraph, it joins no focus traversal, it answers no key. The
control was reachable by pointer only, so there was no focus to report and no keyboard to report it
from — an accessibility gap wearing a styling gap's clothes.

The fix is the smallest thing that can hold a focus: a wrapper that owns it, takes it on a press,
offers it to the traversal, toggles on Space, and reports changes. Space and not Enter: Enter
belongs to the dialog — today it reaches `TextField::on_submit`, which saves the settings form and
confirms both renames, and a dialog-level default action is the obvious next thing to add. A control
that toggles on Enter is the thing that answers first, and nothing downstream of it ever gets the
chance. Deliberately not a
reimplementation — `FilledField` owns the field's box because §7.7's geometry could not be composed,
and nothing is wrong with the checkbox's geometry. The layer is still composited into the fill,
since `checkbox::Style` still has one opaque background; what changed is that *which* layer is
`Layer`'s to decide, now shared with the field, so a focused **and** hovered box shows one and not
two. `Layer` moved to the styling layer for that reason: two controls settle the same question with
different arithmetic, and the ordering is the part neither may restate.

## Complexity Tracking

No constitutional violation requires justification. Left empty deliberately.
