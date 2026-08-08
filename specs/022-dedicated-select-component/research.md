# Phase 0 Research: Dedicated Select Component on a Shared Picker Base

Every item below was resolved by reading the code that already exists, not by choosing between
hypotheticals. The headline finding is that **almost nothing here needs building** — the behaviour
half of the shared base, the animation primitives with the exact curves §6.3 assigns to a menu, and
the row presentation are all already in the tree. This feature is mostly a *rewiring*, and the plan
is sized accordingly.

---

## R1 — What is the shared base, concretely?

**Decision**: `ui/cdk/typeahead.rs` is the shared base. It is generalised (and renamed) rather than
joined by a sibling.

**Rationale**: it already takes both halves as opaque elements —
`Typeahead::new(field, menu, open, gap)` — and decides only anchoring, flipping, capture and
dismissal. It names no colour, type or shape (`cdk_no_appearance.rs` holds it there), and it knows
nothing about text input: the "field" is whatever element the caller hands it. A select's trigger is
a different element, and that is the whole of the difference.

Its keyboard rule is already the one FR-014 wants, and already lives in render-free code
(`micold_core::typeahead::intent_for`) with the `Claim::Taken` / `Claim::PassOn` split that lets Tab
dismiss *and* move focus.

**Alternatives rejected**:
- *A second cdk primitive for the select.* `one_overlay_implementation.rs`'s
  `CDK_OVERLAY_IMPLEMENTORS` says "Empty is the correct state" and holds exactly one entry. A second
  hand-written overlay is the accretion that file exists to prevent — and it would be the same
  mechanism twice.
- *Leaving `cdk/typeahead.rs` alone and making the select call it directly under its current name.*
  Works mechanically, reads as a lie: a select is not a type-ahead. The name is the contract.

**Consequence for the name**: `cdk::typeahead::Typeahead` becomes `cdk::picker::Picker` (or similar
— T-time decision). `typeahead_is_generic.rs` currently scans `cdk/typeahead.rs` and
`material/typeahead.rs` by path and must follow the rename; that gate's *rule* (the component may not
name branches, worktrees or git) is unchanged and now covers one more file.

---

## R2 — Who owns the select's open state, given `logical_state_ownership.rs`?

**Decision**: the select owns it, in its own widget-tree state. The search picker keeps its
caller-owned openness. The base supports both because it takes `open: bool` and emits events; who
holds the bool is the *material* layer's business, per control.

**Rationale**: `logical_state_ownership.rs` draws the line at "would this value still mean something
with the screen switched off". A dropdown's openness would not — it is not persisted, not restorable,
and means nothing to a headless reader of the state. It is the same category as a dialog's fade
progress, which that gate explicitly assigns to the component. This is also the status quo: `pick_list`
owns its open flag today, and `contracts/material-select.md` records that as the reason
`AddWorktreeTypeMenuToggled` was deleted.

The search picker is genuinely different and stays different: its list content *is* a function of the
query, which is application state, so its openness is coupled to something the application owns. FR-013
asks only that the select need no caller to supply it, and the asymmetry is the honest answer.

**Consequence**: this closes accepted fidelity gap #3 structurally rather than by asking callers to
cooperate. `Select::active` disappears as a builder method — nothing supplies it because nothing has
to.

**Alternatives rejected**:
- *Caller-owned openness for the select too, for symmetry with the search picker.* It is the
  arrangement that produced the gap: three consumers, none tracking it, an indicator permanently at
  rest. Symmetry between two controls with different state shapes is not a reason.

---

## R3 — How is the list animated, and does it need new machinery?

**Decision**: compose the existing `material::scale` and `material::fade` wrappers around the menu
element. No new animation machinery, no new tokens.

**Rationale**: `material/animation.rs` already provides exactly this feature's requirement.

| What FR-018/FR-019 ask for | What already exists |
|---|---|
| grow from slightly compressed to full | `scale`, `MIN_SCALE = 0.96`, transforms **drawing only** — "never reflows the layout around it", which is FR-023 for free |
| fade in on the way in | `fade` |
| decelerate in, accelerate out | `Motion`'s defaults are `STANDARD_DECELERATE` / `STANDARD_ACCELERATE` — §6.3's two menu rows exactly, with no override needed |
| a shorter exit than entrance | `.exiting_over(Duration)` |
| animate on appearing, not on mount | `.animate_in()` |
| stop accepting input once gone | the wrapper is inert below the hidden threshold, and `.on_hidden(msg)` can announce it |

Durations come from `motion::duration::SHORT_3` (150, in) and `SHORT_2` (100, out) — the values §6.3's
"menu fade in" and "menu fade out" rows already name. **FR-020 is satisfied by construction**: this is
an assigned animation reaching a surface that was not drawing it, so feature 018's count of new
animations does not move.

**Alternatives rejected**:
- *A bespoke transition inside the cdk.* `cdk_no_appearance.rs` forbids the cdk naming a duration or
  a curve, and rightly — how a list arrives is appearance.
- *`expand` instead of `scale`.* `expand` reveals by changing height, so the surrounding layout
  reflows. For an in-tree accordion that is the point; for a floating list it violates FR-023.

---

## R4 — The list must outlive `open == false` to fade out. Where does that go?

**Decision**: in the base's `overlay()`, which today returns `None` the moment `open` is false. It
must instead keep returning the overlay while the transition is still running, and stop when it has
finished.

**Rationale**: this is the one genuinely new mechanism, and it is small. `MenuOverlay` already solves
the same problem the other way — "a closed menu still yields a surface … the panel has to outlive the
state that opened it or there would be nothing left on screen to fade out" — but it can, because a
window-level `Surface` is always in the tree. An `overlay()` that returns `None` removes the widget
outright, taking its animation state with it.

**Approach**: the wrapper's progress lives in the menu child's tree state, which the base already
keeps across a close and reopen (`children()` returns both, deliberately: "the menu keeps its widget
state across a close and reopen"). So the base needs to know whether the child is still visible. Two
candidates, to settle in Phase 1 design:

- **(a)** the base takes a `visible: bool` alongside `open: bool`, where the material layer computes
  visibility from its own track. Honest, but leaks a second flag into the base's API.
- **(b)** the base keeps its own `Progress` for "is the overlay still needed", advanced on the same
  frames, with the duration handed in as a plain number — the same arrangement `cdk/typeahead.rs`
  already uses for `gap`, which arrives from the caller "because how far apart they sit is a spacing
  decision, and spacing is appearance".

**(b) is preferred**: it keeps one source of truth for "is the list on screen", and the precedent for
a bare number crossing that boundary is already set and already argued in the module's own docs.

**Risk**: `idle_requests_no_frames.rs` fails the build if anything asks for frames at rest. The
overlay must stop requesting them the moment the transition settles — which is what `Progress`
already does, but it is the specific thing to verify rather than assume.

---

## R5 — What happens to `pick_list`?

**Decision**: it goes, and four things follow it.

**Rationale and scope** — this is the largest mechanical part of the work, and it is worth counting
before planning rather than discovering during it:

| Site | What must happen |
|---|---|
| `material/select.rs` | rewritten; no longer imports `pick_list` |
| `material/style.rs` | `select_field` / `select_menu` are typed in `pick_list::Status` / `menu::Style`. The *look* they encode is still wanted; the signatures are not. |
| `material/style_snapshot.rs` | poses `pick_list` in three statuses and records `pick_list.menu`; the snapshot fixture changes with it |
| `tests/one_overlay_implementation.rs` | the `SANCTIONED` entry for `select.rs` must be **removed**. The gate has a staleness check that fails when a sanction no longer applies, so this is forced rather than remembered — a genuinely useful gate behaving well. |
| `tests/material_boundary.rs` | `pick_list` leaves `WRAPPED_WIDGETS` once nothing wraps it |
| `tests/support/layout.rs`, `tests/support/covered_states.rs`, `tests/layout_snapshot.rs` | all three carry commentary and special-casing about `pick_list`'s private open flag and its out-of-tree dropdown. Once the select's list is composed in-tree, the special cases *dissolve* — the base walk sees it like any other element. |

That last row is a net simplification the plan should claim explicitly: three test-support files
currently contain machinery whose only purpose is to reach inside a widget this feature deletes.

---

## R6 — Do the two lists share row presentation, given their different item types?

**Decision**: yes, through the existing `material::typeahead::Row` record, which the shared
presentation layer takes. The select converts its `T: ToString` options into `Row`s with **no**
matched spans.

**Rationale**: `Row` is already "a plain record the caller fills in, like `MenuItem` and `TreeItem` —
deliberately not a component", carrying `label`, `spans` and `enabled`. A select row is a `Row` whose
`spans` are empty, and `EmphasisedLabel` with no spans is a plain label. Nothing needs widening, and
FR-017 (an option may be shown-but-unavailable) is `Row::disabled()`, already built.

**Consequence for the spec's recorded divergence**: the row label keeps `TypeRole::Body`, not §7.5's
`label_large`. The spec assumed this and flagged it; the code confirms the reason — `ROW_ROLE` is
`Body` because "`Action` is already the medium weight and emphasis would then have nowhere to step up
to". A select row has no emphasis, so it *could* take `Action`; making it do so would mean the two
lists differ in the one property the feature exists to unify. **Body for both.** This is a knowing
deviation from §7.5 for these two controls, and Phase 1 records it in the contract so it is a visible
decision rather than an accident.

---

## R7 — Does the select's trigger stay a `FormField`?

**Decision**: yes, unchanged.

**Rationale**: `FormField` already owns the container, the label's rest/float rule, the active
indicator, the supporting text and the error state, and both `TextField` and `Select` already compose
it (the showcase's `form_field` entry poses the chrome *through* a `Select` for exactly that reason).
FR-002 asks for parity with the text field's anatomy, which is what composing the same wrapper means.

What changes is only what sits *inside* it: a row of `Text` + chevron with a state layer and a ripple,
instead of a `pick_list`. That makes the trigger an ordinary pressable surface, which is what FR-010
and §7.7's "its open and hover states are carried by the state layer instead" already describe.

---

## R8 — Does the gallery need new state?

**Decision**: no new application state; one showcase field, mirroring what BUG-001 just added for the
search picker.

**Rationale**: the select owns its openness (R2), so the gallery does not hold it. What the gallery
does need is `interactive: true` and a `live` caption list on the `Select` entry —
`showcase_captions.rs` requires a non-empty `live` list for an interactive entry, and the entry is
currently posed. FR-031 asks for the two pickers comparable on one page, which they now are: the
gallery's `select` and `typeahead` entries sit adjacent in `sections/controls.rs`.

**Note carried from BUG-001**: FR-020a of feature 021 now binds *every* live catalogue entry — a live
entry pins no state the application cannot leave. The select entry must therefore not be posed open
either.

---

## R9 — Is `micold-core` involved at all?

**Decision**: barely. The keyboard rule (`intent_for`, `move_highlight`) is already there, already
generic, already tested, and the select uses it unchanged. No new core module.

**Rationale**: FR-014 asks the select to answer the same keys with the same meanings, and "the same"
is best achieved by calling the same function. The only core-adjacent question is whether a select's
highlight needs its own re-seating rule when options change; it does not — a select's options do not
change under it the way a search's results do.

---

## R10 — What is the delivery risk, and where is it concentrated?

**Decision**: R4 (keeping the overlay alive through its exit) is the only genuinely unknown part.
Everything else is composition of primitives that already work.

It should therefore be built **first**, against the search picker — which already has an open/close
rule, a gallery entry that now exercises it (BUG-001), and a real consumer — before the select exists
to depend on it. That inverts the spec's story order for one slice, and it is worth it: an unpleasant
surprise in the exit animation surfaces before a second control is built on top of it. This is the
same reasoning feature 021 applied to its hand-written `overlay()` ("riskiest task first").

---

## R11 — Does anything about this touch persistence, sessions or worktrees?

**Decision**: no. Presentation and interaction only; FR-030 states it and the plan's Constitution
Check records Principles II, III and IV as unaffected.

---

## R12 — Reduced motion?

**Decision**: out of scope, as the spec states. The application observes no accessibility preference
anywhere today, and introducing the first one inside a component change would be a feature of its own
with its own storage and settings questions.
