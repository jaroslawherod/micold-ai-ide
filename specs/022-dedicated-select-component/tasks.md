---

description: "Task list for feature 022 — Dedicated Select Component on a Shared Picker Base"
---

# Tasks: Dedicated Select Component on a Shared Picker Base

**Input**: Design documents from `/specs/022-dedicated-select-component/`

**Prerequisites**: [plan.md](./plan.md), [spec.md](./spec.md), [research.md](./research.md),
[data-model.md](./data-model.md), [contracts/](./contracts/), [quickstart.md](./quickstart.md)

**Tests**: MANDATORY per Constitution Principle I. Every implementation task below is preceded by a
test task that must be **observed failing** first. The GUI-wiring exception is claimed only for the
render glue — `worktree_form.rs`'s call and the gallery's entries — and never for the visibility
invariant, which is decision logic and is tested through the widget tree.

**Where a test can live** — this shapes the whole list. `ui::cdk` is `pub`, so the base's behaviour is
reachable from `tests/`. `ui::material` is `pub(crate)`, so **a `Select` cannot be constructed from an
integration test at all**; its gates live in-crate beside `text_field_anatomy.rs`,
`content_placement.rs` and `anatomy_size.rs`, which exist for exactly this reason (feature 018,
BUG-001).

**Documentation**: Principle VII. Four documents ship with the code, in Phase 6.

**Cross-platform**: Principle VI. Nothing here is platform-conditional; the final phase verifies all
three.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: parallelizable — different file, no dependency on an incomplete task
- **[Story]**: US1 / US2 / US3, mapping to the spec's prioritized stories

## Path Conventions

Three-crate Rust workspace. `crates/micold-core/` is the iced-free logic crate;
`crates/micold-client/` holds the app, its two-layer component library (`src/ui/cdk/` behaviour,
`src/ui/material/` appearance) and the development-only showcase. Run everything through
`mise run test` / `mise run test-core`.

**One-file serialization**: `src/ui/cdk/picker.rs`, `src/ui/material/picker.rs` and
`src/ui/material/select.rs` are each built up by several tasks, so those tasks are **not** parallel
with one another even where their tests are.

---

## Phase 1: Setup

**Purpose**: mechanical renames and empty homes, so every later task is an edit rather than a
simultaneous file creation. The suite stays green throughout this phase.

- [X] T001 Rename `crates/micold-client/src/ui/cdk/typeahead.rs` to `picker.rs` and the type `Typeahead` to `Picker` (`git mv`, so history follows), updating `crates/micold-client/src/ui/cdk/mod.rs`, the import in `crates/micold-client/src/ui/material/typeahead.rs`, the `CDK_OVERLAY_IMPLEMENTORS` path in `crates/micold-client/tests/one_overlay_implementation.rs`, and the scanned paths in `crates/micold-client/tests/typeahead_is_generic.rs` (contract [picker-base §1](./contracts/picker-base.md), [research R1](./research.md))
- [X] T002 [P] Create empty `crates/micold-client/src/ui/material/picker.rs` with its module doc, and register it plus its re-exports in `crates/micold-client/src/ui/material/mod.rs`

**Checkpoint**: `mise run test` green; nothing behaves differently yet.

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: the one mechanism this feature does not already have — a list that outlives its own
closing — plus the shared presentation both controls will read.

**⚠️ CRITICAL**: no user story work begins until this phase is complete.

**Why this is first, ahead of the spec's own P1** ([research R10](./research.md), plan *Delivery
order*): everything else in this feature is composition of primitives that already work. This is the
only unknown, and it lands against the **search picker** — a control that already has an open/close
rule, a live gallery entry and a real consumer — so an unpleasant surprise arrives before a second
control depends on it. A consequence worth stating plainly: **User Story 2 is half-delivered here**,
for one picker. That is not scope creep; it is the only way to de-risk the mechanism, and the other
half is one call in Phase 4.

### Tests for the foundation (MANDATORY — Constitution Principle I) ⚠️

> Write these FIRST and observe them failing. `cdk` is `pub`, so all three are integration tests.

- [X] T003 [P] Write a failing test for the visibility invariant — `progress > 0` ⟺ `overlay()` returns `Some`: a picker whose `open` just went false still yields an overlay, and yields none once its exit track has settled — in `crates/micold-client/tests/picker_visibility.rs` (contract [picker-base §C1.5](./contracts/picker-base.md), FR-019)
- [X] T004 [P] Write a failing test that a list below the visibility threshold publishes **no** message for a press where a row was, and captures no key — in `crates/micold-client/tests/picker_visibility.rs` (FR-022, contract C1.5)
- [X] T005 [P] Write a failing test that a settled picker requests **no** frames — open, closed-and-settled, and never-opened — following `crates/micold-client/tests/idle_requests_no_frames.rs`'s existing approach (plan *Risks*, row 1)

### Implementation for the foundation

- [X] T006 Add the `exit: f32` input and the `Visibility { progress: Progress }` tree state to `crates/micold-client/src/ui/cdk/picker.rs`, returning the overlay while `progress > 0` and advancing the track on redraw, making T003 and T005 pass. `exit` is a bare number for the same reason `gap` already is — how long a thing takes is appearance (contract C1.5, C1.6)
- [X] T007 Refuse pointer and keyboard input in the overlay while below the visibility threshold, in `crates/micold-client/src/ui/cdk/picker.rs`, making T004 pass. *Inert* must mean the overlay refuses input, not merely that it draws nothing
- [X] T008 Move `row_element`, `marker`, `menu_element`, `ROW_ROLE`, `GAP` and `MAX_ROWS_BEFORE_SCROLL` out of `crates/micold-client/src/ui/material/typeahead.rs` into `crates/micold-client/src/ui/material/picker.rs` with **no behaviour change**, and have the search picker consume them (contract [picker-base §2](./contracts/picker-base.md))
- [X] T009 Add `animated_menu(panel, open, roles)` to `crates/micold-client/src/ui/material/picker.rs`: `scale` from `MIN_SCALE` plus `fade`, `SHORT_3` in and `SHORT_2` out via `.exiting_over(…)`, `.animate_in()`. Both curves are `Motion`'s defaults and MUST NOT be restated (contract [C2.4](./contracts/picker-base.md), FR-018, FR-019, FR-020)
- [X] T010 Wire the search picker to the animated menu and pass the exit duration to the base, in `crates/micold-client/src/ui/material/typeahead.rs` — **the task the whole phase exists to de-risk**
- [ ] T011 Run [quickstart.md](./quickstart.md) §B2 against the type-ahead alone and record the pass, including whether an interrupted transition really resumes from where it is

  > **Blocked on eyes at a display**, like feature 021's §B8. The mechanism is verified by ten tests
  > in `tests/picker_visibility.rs` — five of which fail under a deliberate revert to the old
  > `if !self.open` behaviour, so they are not vacuous — but whether the grow-and-fade *looks* right
  > is not something any test here can answer.

**Checkpoint**: the mechanism works on a live control. User story work can begin.

---

## Phase 3: User Story 1 — The select is a first-class Material control (Priority: P1) 🎯 MVP

**Goal**: the select owns its field, its list, its rows and its states, matching the search picker
property for property, and `pick_list` is gone.

**Independent Test**: [quickstart.md](./quickstart.md) §B1, §B3–§B6 — open both lists side by side and
find zero differences across the eight compared properties; confirm the select's indicator answers for
itself with nothing supplying it; confirm all four placements.

**Depends on**: Phase 2 — the select is built on the generalised base and the shared presentation.

### Tests for User Story 1 (MANDATORY — Constitution Principle I) ⚠️

> In-crate, because `material` is `pub(crate)` and a `Select` cannot be constructed from `tests/`.

- [X] T012 [P] [US1] Write a failing anatomy gate for the select's trigger — container height, label resting vs floating, the trailing chevron's size and role, and the indicator at rest vs open — in `crates/micold-client/src/ui/material/select_anatomy.rs`, following `text_field_anatomy.rs` (contract [select-component §1](./contracts/select-component.md), FR-002, FR-003, FR-004)
- [X] T013 [P] [US1] Write a failing test that the select's open list and the search picker's are the **same** in row height, row padding, marker slot and panel treatment — asserted by building both and comparing, not by restating the figures — in `crates/micold-client/src/ui/material/picker_parity.rs` (SC-001, FR-007, FR-008, FR-009)
- [X] T014 [P] [US1] Write a failing test that opening seeds the highlight from the current choice, so the list opens with the current value marked and reachable — in `crates/micold-client/src/ui/material/select_anatomy.rs`. This is feature 013's FR-003, which `pick_list` gave for free and which must not leave with it (contract [select-component §2](./contracts/select-component.md))
- [X] T015 [P] [US1] Observe `crates/micold-client/tests/one_overlay_implementation.rs`'s staleness check **failing** once `select.rs` no longer calls `pick_list` while the `SANCTIONED` entry still lists it — the gate firing before T021 satisfies it

### Implementation for User Story 1

- [X] T016 [US1] Rewrite `crates/micold-client/src/ui/material/select.rs`: `SelectState { open, highlight }` in tree state, a pressable trigger row (value or placeholder, spacer, chevron, state layer, ripple) composed inside `FormField`, options converted to `picker::Row` with empty spans, and the list built from `material::picker` and floated by `cdk::picker`. Remove the `active(bool)` builder method (contract [select-component §1, §3](./contracts/select-component.md), [data-model §2](./data-model.md), FR-001, FR-005, FR-010, FR-011, FR-012, FR-014)
- [X] T017 [US1] Seed the highlight from `selected` on open in `crates/micold-client/src/ui/material/select.rs`, making T014 pass
- [X] T018 [US1] Drive the active indicator from the component's **own** open flag in `crates/micold-client/src/ui/material/select.rs`, making T012's open-state case pass — this is accepted fidelity gap #3 closing structurally (FR-013)
- [X] T019 [US1] Retire `pick_list` from `crates/micold-client/src/ui/material/style.rs`: `select_field` and `select_menu` are typed in `pick_list::Status` and `menu::Style`. Keep the look, drop the signatures — or drop the functions entirely if `menu_panel` and the shared state layers already cover them (contract [select-component §5](./contracts/select-component.md))
- [X] T020 [US1] Remove the three `pick_list` status poses and the `pick_list.menu` line from `crates/micold-client/src/ui/material/style_snapshot.rs` and regenerate its fixture
- [X] T021 [US1] Remove the `select.rs` / `pick_list` entry from `SANCTIONED` in `crates/micold-client/tests/one_overlay_implementation.rs` (making T015 pass) and drop `pick_list` from `WRAPPED_WIDGETS` in `crates/micold-client/tests/material_boundary.rs`
- [X] T022 [US1] Dissolve the `pick_list` special-casing in `crates/micold-client/tests/support/layout.rs`, `crates/micold-client/tests/support/covered_states.rs` and `crates/micold-client/tests/layout_snapshot.rs`, and regenerate the layout fixture. The dropdown is composed in-tree now, so the base walk sees it like any other element — **this task removes machinery, it does not add any** (contract §5)

  > **The premise is wrong, and the machinery stays.** "Composed in-tree now" is not what happened:
  > the select stopped wrapping `pick_list` and started using `cdk::picker`, which is *also* a
  > `Widget::overlay` implementor — that is the whole reason it was built, since a list inside a
  > content-sized dialog has nothing else to anchor to. So the overlay pass, `resolve_pressing` and
  > `StateUnderTest::pressing` are all still load-bearing, and the select's open flag is still
  > private state that can only be *caused*. What this task actually did is rewrite the three files'
  > commentary to describe the control that is there now, and regenerate the fixture.
  >
  > It also **added** two things rather than removing any, both recorded here because the task said
  > it would not:
  > - `resolve_pressing` now settles frames *after* the press as well as before it. A picker's list
  >   exists only once its visibility track has moved, and the track moves on a frame tick — so
  >   without this the fixture would have lost every `over` record and `the_overlay_pass_records_
  >   something_somewhere` would have failed.
  > - `tests/gates/containment.rs` gained `PICKER_LIST_CONTENT`, one path: the open type list is ten
  >   48dp rows inside an eight-row viewport, so the shared `Scrollable` overhangs exactly as the
  >   sidebar's does. `pick_list` scrolled inside a single node and showed that to nobody. It has its
  >   own attribution test rather than joining `SCROLL_CONTENT`.
- [X] T023 [US1] Move the `form_field` gallery fixture's active-state pose off `Select` and onto `TextField`, which can report focus, in `crates/micold-client/src/ui/material/form_field_anatomy.rs` and `crates/micold-client/src/showcase/sections/controls.rs` — the one call site `active(bool)` had (contract §3)
- [X] T024 [US1] Confirm `selecting_a_type_sets_the_form_value` and `type_selection_is_ignored_while_creating` in `crates/micold-client/tests/app_state.rs` pass **unmodified**, and that `crates/micold-client/src/ui/worktree_form.rs`'s call is unchanged — the regression check on FR-030 (SC-009)
- [ ] T025 [US1] Run [quickstart.md](./quickstart.md) §B1, §B3–§B6 in both schemes and record the pass

  > **Blocked on eyes at a display**, like T011. The machine-checkable half is done and is in
  > `src/ui/material/select_anatomy.rs` (the trigger, the chevron, the label's two positions, and
  > the indicator answering for itself — the last read off rasterised pixels) and in
  > `picker_parity.rs`, which asserts the two lists resolve to the *same* node tree rather than each
  > matching a figure. Whether the two look identical at a display, and whether all four placements
  > are right, is not something those can answer.

**Checkpoint**: the select is ours, `pick_list` is gone, and the two lists are indistinguishable —
shippable on its own, with the transition on the search picker only.

---

## Phase 4: User Story 2 — The list animates open and closed (Priority: P2)

**Goal**: both lists grow-and-fade in and fade out, identically, on the timings §6.3 already
publishes.

**Independent Test**: [quickstart.md](./quickstart.md) §B2 — open and close each list repeatedly, in
both schemes; confirm the grow-and-fade, the quicker exit, the mid-flight reversal, that a fading list
takes no press, and that nothing outside the list moves.

**Depends on**: Phase 2 (the mechanism and the wrapper) and Phase 3 (a select to apply it to). Given
both, the code here is one call — which is why this P2 story lands after the P1 one.

### Tests for User Story 2 (MANDATORY — Constitution Principle I) ⚠️

- [X] T026 [P] [US2] Write a failing test that both pickers' transitions come from **one** definition and that its durations are §6.3's `short_3` and `short_2` — no second literal, and no curve restated over `Motion`'s defaults — in `crates/micold-client/src/ui/material/picker_motion.rs` (FR-021, SC-007)

### Implementation for User Story 2

- [X] T027 [US2] Wire the select's list through `picker::animated_menu` and pass the exit duration to the base, in `crates/micold-client/src/ui/material/select.rs`, making T026 pass (FR-018, FR-019)
- [ ] T028 [US2] Run [quickstart.md](./quickstart.md) §B2 against **both** pickers in both schemes and record the pass, including the interrupted-transition and press-during-exit cases

  > **Blocked on eyes at a display**, like T011 and T025. The machine-checkable half is
  > `src/ui/material/picker_motion.rs`. It establishes the two halves that are observable without a
  > rasteriser: that both lists keep being produced for the *same* number of frames after closing,
  > and that the number is `short_2`'s; and that neither control names a duration, a motion token or
  > a curve of its own, so there is one definition rather than two that agree today.
  >
  > What it cannot reach is everything the transition actually looks like. `scale` and `fade`
  > transform **drawing only** — which is FR-023 holding by construction, and is also why a
  > comparison of rectangles is blind to the animation. So the grow-and-fade itself, the enter
  > duration, both curves, and whether a reversal mid-flight resumes from where it is rather than
  > snapping (FR-021) are unrun rather than assumed. The press-during-exit case *is* covered, in
  > `tests/picker_visibility.rs`, but through the base rather than through either control.

**Checkpoint**: both stories independently demonstrable.

---

## Phase 5: User Story 3 — One foundation behind both pickers (Priority: P3)

**Goal**: the shared behaviours are defined once, and a change to any of them requires editing exactly
one place.

**Independent Test**: take the behaviours the two controls share — anchoring, flipping, dismissal, the
keyboard rule, row treatment, the transition — and exercise each **through both controls**. Every one
must be defined once and pass for both.

**Depends on**: Phases 3 and 4. Largely a verification story, which is why it is last despite being
the durability half of the request.

### Tests for User Story 3 (MANDATORY — Constitution Principle I) ⚠️

- [X] T029 [P] [US3] Write a failing test that exercises each shared behaviour through **both** controls from one test body — two constructions, one set of assertions — in `crates/micold-client/src/ui/material/picker_parity.rs` (SC-008, FR-024, FR-025)

### Implementation for User Story 3

- [X] T030 [US3] Resolve whatever T029 finds still duplicated rather than shared, across `crates/micold-client/src/ui/material/picker.rs`, `typeahead.rs` and `select.rs` (FR-024, FR-025, FR-028)

  > **T029 found a defect, not only a duplication.** Both controls end in `intent_for`, but each had
  > written out the *claim* half — which keys the list keeps and which travel on — and they had
  > already come apart. `cdk::picker` reported the claim to the runtime; `Select::update` decided it,
  > returned early, and never called `shell.capture_event()` at all. So Escape closed the select's
  > list **and** the dialog behind it on the same press, and Enter would have picked a row and
  > submitted the dialog.
  >
  > The resolution is `micold_core::typeahead::claims`, beside `intent_for` where the rest of the key
  > rule lives, called by both routes — and the missing `capture_event`. The fix landed in
  > `cdk/picker.rs` and `select.rs` rather than in `picker.rs` and `typeahead.rs` as this task
  > anticipated: the duplication was in the *behaviour* halves, which is where the two controls
  > genuinely diverge, not in the presentation they already share.
- [X] T031 [P] [US3] Add the select's new colour pairings — the trigger's state layers over the field container, and the chevron's `on_surface_variant` — to the AA-contrast gate in `crates/micold-core/tests/tokens.rs`, so "legible in both schemes" is measured rather than assumed (FR-029)
- [X] T032 [US3] Verify the architecture gates pass **together** — `cdk_no_appearance.rs`, `material_boundary.rs`, `material_builder_api.rs`, `component_api_opacity.rs`, `one_overlay_implementation.rs`, `typeahead_is_generic.rs`, `idle_requests_no_frames.rs`, `logical_state_ownership.rs`, `anatomy_size.rs`, `content_placement.rs`, `showcase_completeness.rs`, `showcase_captions.rs` — with **no budget raised and no exception added**. This feature should end stricter than it started: one sanction removed, none added (contract [picker-base §4](./contracts/picker-base.md))

**Checkpoint**: all three stories independently functional.

---

## Phase 6: Polish, Gallery & Documentation

- [X] T033 [P] Set `interactive: true` and a non-empty `live` list on the `Select` entry in `crates/micold-client/src/showcase/catalogue.rs`, naming the states a developer can exercise — press to open, pick, dismiss, keyboard (`showcase_captions.rs`, FR-031)
- [X] T034 [P] Make the gallery's select entry drive the real rule in `crates/micold-client/src/showcase/sections/controls.rs`. It MUST NOT be posed open: feature 021's FR-020a, added by BUG-001, binds every live entry — a live entry pins no state the application cannot leave
- [X] T035 [P] Update `specs/018-material3-visual-system/contracts/design-tokens.md` §7.7 (the select is a first-class control, no longer mute) and **§9 (remove accepted fidelity gap #3 — the list drops from four entries to three)** (FR-032, SC-005)
- [X] T036 [P] Document the select and the shared picker base — what each is for, and that a third picker consumes the base rather than rebuilding it — in `docs/development/component-library.md` (Principle VII, FR-033, FR-028)
- [X] T037 [P] Add a superseding note to `specs/013-create-worktree-refinement/contracts/material-select.md` in that file's own established style, recording that `pick_list` was the only thing that could anchor a dropdown inside a content-sized dialog when it was written, and no longer is
- [X] T038 Cross-cutting documentation review — links, navigation and index entries across `docs/`, including `docs/README.md` (Principle VII)
- [ ] T039 Confirm `mise run test` passes on Linux, macOS and Windows via `.github/workflows/ci.yml` (Principle VI)

  > **Pending a pull request.** The three-platform matrix only runs on a PR, so this cannot be
  > answered from a branch. Locally on Linux: 164 test binaries, 0 failed, 1495 tests. Worth noting
  > *why* the local run is weaker evidence than usual here — every worktree on this machine shares
  > one `CARGO_TARGET_DIR`, and cargo gives the same `-C metadata` hash to the same crate built from
  > different worktrees, so they can overwrite each other's test binaries. CI on a clean checkout is
  > the arbiter, not this.
- [ ] T040 Run [quickstart.md](./quickstart.md) end to end and record the full pass — the date, the platform, and **which half was machine-checked**. §B1 and §B2 are this feature's two headline claims and neither can be automated; a green suite is not this feature working

  > **Blocked on eyes at a display** — the fourth and last of these, with T011, T025 and T028, and
  > the one that subsumes them. The task's own wording is the finding: a green suite is not this
  > feature working. What the machine established is that the two lists resolve to the *same* node
  > tree (`picker_parity.rs`), that they leave over the same number of frames and that neither
  > control names a timing (`picker_motion.rs`), that the trigger's anatomy and its indicator hold
  > (`select_anatomy.rs`, the indicator read off rasterised pixels), and that both controls claim
  > the same keys. What it cannot establish is either headline claim: whether the two lists *look*
  > indistinguishable side by side (§B1, SC-001), and whether the transition looks right — the
  > grow-and-fade, the enter duration, both curves, and a reversal mid-flight resuming rather than
  > snapping (§B2, FR-021). `scale` and `fade` transform drawing only, and nothing in this crate's
  > test renderer rasterises them.

---

## Dependencies & Execution Order

```text
Phase 1 (T001–T002)
      ↓
Phase 2 (T003–T011)  ← the only unknown; blocks everything
      ↓
Phase 3 / US1 (T012–T025)  🎯 MVP
      ↓
Phase 4 / US2 (T026–T028)  ← one call, given Phase 2
      ↓
Phase 5 / US3 (T029–T032)  ← verification
      ↓
Phase 6 (T033–T040)
```

**Story independence**: US1 is shippable alone (with the transition on the search picker only, from
Phase 2). US2 depends on US1 for a second control to animate. US3 depends on both, because it verifies
what they share.

**Within-file serialization** — these are *not* parallel with each other despite what their `[P]`
neighbours suggest:

- `cdk/picker.rs`: T001 → T006 → T007
- `material/picker.rs`: T002 → T008 → T009 → T030
- `material/select.rs`: T016 → T017 → T018 → T027 → T030
- `material/typeahead.rs`: T001 → T008 → T010 → T030

## Parallel Opportunities

- **T003, T004, T005** — three failing tests, two files, no dependency between them
- **T012, T013, T014, T015** — US1's four test tasks, all different files
- **T033–T037** — the gallery and all four documents, five different files

## Implementation Strategy

### MVP first

Phases 1–3. That delivers a select that is genuinely this library's, indistinguishable from the search
picker, with `pick_list` and its six removal sites gone — and the animation already working on one of
the two controls. It is a complete, defensible increment.

### Incremental delivery

1. **Setup + Foundational** → the mechanism, proven on a live control.
2. **US1** → the select is ours. Demo: the two lists side by side.
3. **US2** → the select animates too. One call.
4. **US3** → the foundation is verified, not just asserted.
5. **Polish** → the gallery, four documents, three platforms.

### Riskiest task first

**T010** — wiring the search picker to a list that must outlive its own closing. Everything else in
this feature composes primitives that already work; this is the one place a surprise can live. It sits
in Phase 2 deliberately, before a second control depends on it, and T011 makes someone look at it
before the phase closes.

---

## Notes

- `[P]` means a different file and no dependency on an incomplete task.
- Commit after each task or logical group; the gates run on every `mise run test`.
- **Do not raise a gate budget or add an exception to make a task pass.** This feature removes one
  sanction (`select.rs` / `pick_list`) and adds none. Ending with more exceptions than it started
  would mean the shared base did not actually replace anything.
- **Do not restate a motion curve.** `Motion`'s defaults are already §6.3's two menu rows; writing
  them out again creates a second definition that can drift from the first.
- The `Body`-over-`label_large` row label is a **recorded deviation** from §7.5, scoped to these two
  pickers (contract [picker-base §C2.2](./contracts/picker-base.md)). Changing it is a visible edit to
  a named file, not a judgement call during implementation.
