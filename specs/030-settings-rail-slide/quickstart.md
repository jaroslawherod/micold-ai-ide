# Quickstart: The settings rail slides when it collapses and expands

**Feature**: 030-settings-rail-slide | **Plan**: [plan.md](./plan.md) | **Contract**: [contracts/rail-slide.md](./contracts/rail-slide.md)

Validation splits the way Constitution Principle I splits the code. **Part A** is automated and
holds every decision the slide makes: fraction, form, offset, focus, reach, input, timing and the
rest layout. **Part B** is the recorded visual pass for what no layout assertion can see: the clip,
the tint and how the slide looks beside the sidebar's.

Run Part A first. Part B means nothing until it passes.

**Prerequisites**: `mise trust` once per fresh worktree. All commands run from the repository root.

---

## Part A — automated

### A.1 The whole gate

```bash
mise run gate          # fmt --check, clippy (core, workspace, -D warnings), test --workspace, scripts/tests
```

It shares `target-shared/` with every other worktree, so `Blocking waiting for file lock` is a wait,
not a fault (CLAUDE.md).

### A.2 The gates that must contain new coverage

Each row is an obligation: its test must fail against what its "Fails on" column names —
`origin/main`, the WIP's two-rendering rail, or a named mutant of this design (checked once, by
hand, when the test is written, per the TDD loop's deliberate-mutant step).

| Gate | Covers | Fails on | Requirement |
|---|---|---|---|
| `section_list.rs` unit tests | `fraction` clamps and maps 64→0, 272→1; `form` at 0.001, 0.999, badged and not; `offset` makes the drawn icon x equal the line in every form; a selection change moves it by 12·f | — (new functions) | contract §3–§4 |
| `tests/settings_rail_motion.rs` **(new)** `collapsing…` / `expanding…` | widths strictly between on some frame, each way | main | FR-001, SC-001 |
| ″ `the_section_follows_the_rail_edge` | on a slide with at least one intermediate frame, the section region starts at the rail's right edge; content offset unchanged | main (no intermediate frame); mutant: `Rail::layout` returning the destination width while its child is at the animated width | FR-002 |
| `section_list.rs` unit tests (in-crate mount) `the_rail_and_the_sidebar_move_alike` | a mounted `SectionList` and `NavigationDrawer` at equal fractions at equal instants; the fraction at 25% of the duration is not 0.25 ± 0.05 (non-linear); settles within 400 ms + 2 × gap | main (no rail slide) | FR-003, SC-002 |
| ″ `a_second_press_reverses_from_where_it_is` | mid-slide width strictly between 80 and 288; reversal starts at the width reached; no step exceeds an uninterrupted slide's over the same interval | main (no intermediate width to reverse from) | FR-004, SC-004 |
| ″ `the_rail_is_mounted_at_its_width…` | first frame at final width in both states | mutant: the track created at 0 rather than at the target (passes on main) | FR-005, SC-006 |
| ″ `the_keyboard_stays_on_the_collapse_control…` | focus on the control, the reachable count equal to rest's, and the section's focusables in rest order, on every frame including each row's form change | `RowSlide` before the focus handoff (focus lost at the first form change); mutant: `operate` reaching parked forms (count doubles) | FR-007, FR-008, SC-003 |
| ″ `a_pointer_past_a_row_reaches_nothing` | a cursor inside a drawn form's bounds but outside its row's bounds hovers and presses nothing | mutant: `RowSlide` without the cursor mask | FR-009 |
| ″ `the_state_flips_on_the_press` | `collapsed` changed before the first frame; mid-slide selection, Save and Cancel (Settings' only exits) take effect on the frame they happen (`settings_draft` becomes `None`) | mutant: the flag held until the slide settles, or `Rail::update` withholding events while animating | FR-010 |
| ″ `a_settled_rail_asks_for_nothing` | no redraw request after the slide settles | mutant: `Rail::update` requesting a frame unconditionally | FR-011 |
| ″ `rows_keep_their_height_and_icons_their_line` | every row's height is its rest height; per-frame icon step within the SC-007 bound and toward the target | WIP (13dp step at the swap) | FR-013, FR-014, SC-007 |
| ″ `selecting_mid_slide_moves_only_the_changed_icons` | changed rows' icons move by `12 · f`; unchanged rows' by the SC-007 bound only | may pass once `RowSlide` exists; mutant: `RowSlide` caching `x_labelled` from its first layout | FR-014 exception |
| ″ `a_badged_row_keeps_its_mark` | on every frame the badged row draws its `marked` slot (tinted icon, read from which slot is not parked), or a form whose chip node intersects the rail | mutant: `form` returning `Labelled` for a badged row mid-slide (chip cut off, icon untinted) | FR-015 |
| `section_list.rs` unit tests (in-crate mount) `a_row_with_no_icon_keeps_its_chip` | chip inside the row on every frame; never empty | mutant: a no-icon row laid out at 272 like the others | FR-015, spec Edge Cases |
| `tests/settings_rail_motion.rs` `a_click_on_collapse_shows_no_focus_ring` | after a pointer press on **Collapse** and the settled slide, the control is focused and its indicator is not shown — observed as a screenshot of the control's region from the headless renderer `tests/support/layout.rs` already uses, equal pixel for pixel to the same settled state with focus cleared | `RowSlide` before the focus handoff (focus lost as the pressed form parks); mutant: `GiveFocus` calling `focus()`, as the WIP's `FocusNth` did | FR-006, FR-007 |
| ″ `a_press_held_as_its_form_parks_is_released` | a button pressed at `f` just above 0.001 and released after its form parks is not left pressed: after expanding, a bare release over it publishes nothing | `RowSlide` before parked forms get events | FR-009 |
| ″ `a_ripple_in_a_parked_form_settles` | a row pressed just before its form parks: after the slide settles and before the ripple ends a frame still requests a redraw; once both have passed, the rail requests no frame | `RowSlide` before parked forms get events (ripple frozen); mutant: parked forms' redraw requests dropped, or forwarded forever | FR-009, FR-011 |
| ″ `a_badge_appearing_mid_slide_keeps_focus` | a focused badged-row control keeps focus, in the drawn form, when the badge appears or clears mid-slide | `RowSlide` before the focus handoff (focus lost at the form change); mutant: an empty placeholder (another widget type) in the `marked` slot, which loses focus when the badge clears | FR-007, FR-015 |
| `docs/user-guide/settings.md` | describes the **Collapse** control, its expand icon when collapsed, and its slide | main (undocumented) — checked against FR-012's wording in the milestone's Review A | FR-012 |
| `tests/gates/rail_icons_align.rs` | collapsed icons on one axis (unchanged; finds the drawn form because `icons_only` is the last child) | mutant: `icons_only` not the last child slot | FR-006 |
| `tests/layout_snapshot.rs` | regenerated fixture | — | FR-006 |
| `tests/animated_layout_relayouts.rs` | `Rail` reads its track in `layout` and calls `on_layout_frame` in `update` | — | FR-011 |
| `tests/one_overlay_implementation.rs`, `tests/motion_tokens.rs`, `tests/cdk_no_appearance.rs` | no overlay constructed; durations are named tokens; cdk untouched | — | Constraints |

Names marked ″ are in `tests/settings_rail_motion.rs`. The two in-crate mounts are in
`section_list.rs` because `ui::material` is `pub(crate)` and `tests/` reaches the rail only through
the settings view. Final names follow the test list
(`tdd/test-list.md`), and this table is corrected to match by T043, before the milestone's review.

### A.3 The rest layout did not move (SC-005)

The regenerated fixture adds parked forms (x = `-inf`) and renumbers tree paths, so a diff of the
whole file is noise. Compare what is visible instead: per state and pass (`base` or `over`), the
**set** of distinct rectangles with a finite x, ignoring paths, must be the same on `origin/main`
and on the branch. A set, not a list: each `RowSlide` node has exactly the rectangle of the form it
draws, so it adds a repeat of a rectangle already present, never a new one. The two settings states
are `settings-view-with-validation-error` and `settings-view-rail-collapsed`; every other state must
come out unchanged too.

```bash
f=crates/micold-client/tests/fixtures/layout_snapshot.txt
visible() { awk '/^## /{s=$2; next} /^(base|over)/ && $3!="-inf" {print s, $1, $3, $4, $5, $6}' | sort -u; }
diff <(git show origin/main:$f | visible) <(visible < $f) && echo "SC-005: visible rects unchanged"
```

A difference here is a rest-state regression, whatever the rest of the gate says. It is expected to
pass on the WIP's two-rendering rail too: it covers rest, not the per-row forms, which A.2 holds. The set
comparison cannot see a rectangle that was present twice and is now present once; the fixture test
itself, reviewed per path for the two settings states, covers that. The showcase is not in the
fixture; its rest look is B.4's.

### A.4 Fast loop while iterating

```bash
scripts/build-lock.sh cargo test -p micold-client --lib section_list
scripts/build-lock.sh cargo test -p micold-client --test settings_rail_motion --test layout_snapshot
```

---

## Part B — the recorded visual pass

Run with the repository's `visual-pass` skill: a private Xvfb display, lavapipe, a private pin dir
and a short `XDG_RUNTIME_DIR` (for example `/tmp/vp97`). Pin `micold-showcase`, and
`micold-ai-ide` with `micold-daemon` from one build, before launching.

### B.1 Settings rail at rest, both states, both schemes

Open Settings; capture expanded; press **Collapse**; wait > 1 s; capture collapsed.

**Expect**: identical to the same captures on `origin/main` (crop both at the same geometry and
stack them). The current row's filled container, the icons' column and the badge are where they
were.

### B.2 Mid-slide frames

Press **Collapse**, then capture as fast as the pipeline allows; repeat for expand. Keep any frame
that shows a width strictly between 80 and 288.

**Expect**: labels and chips cut off at each row's right edge with nothing drawn past the rail;
the badged row's icon tinted; no row taller or shorter than at rest. The current row's filled
container is cut off square at the row's right edge while it is wider than the row — accepted: it
is content wider than the rail's current width, which FR-013 says is cut off. The focus ring, when
shown, stays inside the row (it is inset by half its width). If no intermediate frame is
caught after five attempts, record that — the skill says mid-flight frames are not reliably
reachable, and Part A holds the timing.

### B.3 Beside the sidebar

In the main window, hide and show the worktree sidebar; in Settings, collapse and expand the rail.

**Expect**: record what was seen. Whether the two *feel* alike is not something lavapipe on Xvfb
can answer (the skill's "What this still cannot answer"); SC-002's equal fractions are Part A's.

### B.4 The showcase rail example

Surfaces section, the rail example (240 high): collapse and expand.

**Expect**: the same as B.1–B.2, with the error-tinted badge on "Session service".

### Recording

Write `specs/030-settings-rail-slide/visual-pass.md`: date, Xvfb + lavapipe, the binaries' pin
check, which of B.1–B.4 were exercised, which were not and why, and cropped comparison images
under `images/`.
