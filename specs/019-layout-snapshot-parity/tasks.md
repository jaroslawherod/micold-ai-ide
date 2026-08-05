# Tasks: Layout Snapshot Parity Gate

**Input**: Design documents from `specs/019-layout-snapshot-parity/`

**Prerequisites**: [plan.md](./plan.md), [spec.md](./spec.md), [research.md](./research.md), [data-model.md](./data-model.md), [contracts/layout-fixture.md](./contracts/layout-fixture.md), [quickstart.md](./quickstart.md)

**Tests**: Per Constitution Principle I (NON-NEGOTIABLE), test tasks are mandatory and precede implementation. This feature is unusual in that its *deliverable* is a test — but the apparatus underneath it (normalisation, path emission, overlay traversal, failure messages) is real logic with real defects available, and it gets tested first like anything else. The Principle I GUI-wiring exception is **not** invoked anywhere here; this feature is that exception's replacement for the layout dimension.

**Documentation**: This feature is not user-facing, so Principle VII's obligation lands on developer documentation (`docs/development/`), per the precedent feature 017 T046 set. FR-015 makes the covered/not-covered boundary a requirement rather than a courtesy.

**Cross-platform**: Principle VI is the central technical risk here, not a checkbox. FR-006 requires byte-identical output on all three platforms; research R2 removes the mechanism that would have broken it, and T005 is the guard that fails loudly if a host font subverts the fix.

**Organization**: Tasks are grouped by user story so each is independently implementable and testable.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this task belongs to (US1, US2, US3)
- Include exact file paths in descriptions

## Path Conventions

Three-crate Cargo workspace. This feature touches **only** `crates/micold-client/tests/` and `docs/`. No file under `crates/micold-client/src/`, `crates/micold-core/src/` or `crates/micold-daemon/src/` is modified — that is FR-019, and T037 checks it mechanically.

---

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: The baseline and the assets everything else rests on.

- [X] T001 Record the pre-change test count from `mise run test` as a note in `specs/019-layout-snapshot-parity/tasks.md`, before any other task runs. A later drop means a test was lost in the change, not that the suite got faster — the same trap feature 017's T001 was written to catch. **Baseline: 893 passing, 0 failing** (`mise run test`, 2026-07-28, commit a7105de)
- [X] T002 [P] Add the reference typeface with `crates/micold-client/tests/fixtures/FONT-PROVENANCE.md` recording source, version and the Apache-2.0 licence. This is the same asset feature 018's T015 must register as the shipped application font — it MUST NOT be committed twice (research R2)

  > **Its one instruction was not followed, by both features.** 019 committed `tests/fixtures/Roboto-Regular.ttf`; 018 then shipped `assets/fonts/Roboto-Regular.ttf`. The two were **different builds of Roboto with different bytes** — a tenth of a pixel apart over the guard string — so the snapshot was measuring against a face the application does not draw with. Reproducible, stable, and wrong; no test could see it because both sides were self-consistent. Corrected on 2026-08-04: the duplicate is deleted and the snapshot reads `assets/fonts/` directly, loading **all three** faces the application registers — Regular, Medium (type roles at weight >= 500) and `MaterialSymbolsOutlined` (icons are glyphs and are measured like any other text). Missing the third is what CI caught in T035.
- [X] T003 [P] Declare `pub mod layout;` in `crates/micold-client/tests/support/mod.rs` and create the empty `crates/micold-client/tests/support/layout.rs` that the apparatus will fill

---

## Phase 2: Foundational — the measuring apparatus

**Purpose**: Resolve layout headlessly and turn a widget tree into normalised records. **Blocking**: no user story works without this.

### Tests for Phase 2 (write first, confirm they FAIL) ⚠️

- [X] T004 [P] Failing test in `crates/micold-client/tests/layout_apparatus.rs` asserting the headless renderer constructs with no display, no GPU and no window manager, and reports the `tiny-skia` backend. Must also pass under `env -u DISPLAY -u WAYLAND_DISPLAY` (FR-001, research R1)
- [X] T005 Failing **font guard** test in `crates/micold-client/tests/layout_apparatus.rs`: the committed face parses via `ttf-parser` as the expected family and weight, **and** a pinned reference string measures to a pinned width. The second half is the load-bearing one — the host's fonts are still loaded (391 faces measured on the development machine), so a same-named system font winning the family lookup would shift every measurement at once. Without this the failure would read as a mass layout regression instead of a font problem (FR-006, research R2 residual risk)
- [X] T006 [P] Failing tests in `crates/micold-client/tests/layout_record_format.rs` for numeric normalisation per `contracts/layout-fixture.md` §2: rounded to one decimal, **exactly** one fractional digit always present, `-0.0` written `0.0`, fixed-width alignment, and no value formatted through `{:?}` on an `f32` (FR-012)
- [X] T007 Failing test in `crates/micold-client/tests/layout_record_format.rs` asserting emission is in depth-first tree order and is **never sorted** — sorting would conceal a structural reordering, which is a change the gate exists to report (FR-002, contract §3)
- [X] T008 Failing test in `crates/micold-client/tests/layout_apparatus.rs` asserting determinism: two consecutive walks of the same state, in the same process and on the same commit, produce identical records (FR-005)
- [X] T009 Failing test in `crates/micold-client/tests/layout_apparatus.rs` asserting the reproducible sampling point: a freshly built widget tree reports every animation at rest and every scrollable at offset zero, so no frame pumping or timing tolerance is needed. Both come free from a fresh `Tree`, but free is not the same as checked (FR-010, FR-011, research R6/R7)

### Implementation for Phase 2

- [X] T010 Implement the headless renderer constructor in `crates/micold-client/tests/support/layout.rs`: load the committed face into the global font system, then `<iced::Renderer as Headless>::new(reference_font, 16px, Some("tiny-skia"))`. The backend hint is what makes `iced_wgpu` decline before constructing a `wgpu::Instance` (FR-001, FR-006)
- [X] T011 Implement `LayoutRecord` and the normalisation/formatting helpers in `crates/micold-client/tests/support/layout.rs` per `data-model.md` and contract §2 (FR-012)
- [X] T012 Implement the depth-first walker in `crates/micold-client/tests/support/layout.rs`, emitting `(path, depth, layer, geometry)` in tree order from a laid-out `layout::Node` (FR-002, FR-005)
- [X] T013 Implement the overlay pass in `crates/micold-client/tests/support/layout.rs` — call `Widget::overlay` and lay out the returned element as layer `over`. Dialogs and menus are composed in-tree and need nothing special, but `material::Select` wraps `pick_list`, a genuine `Widget::overlay` implementor whose dropdown the base walk cannot see (FR-009, research R5)

**Checkpoint**: a widget tree can be turned into deterministic, normalised records. **Reached** — 18 tests green across `layout_apparatus.rs` and `layout_record_format.rs`.

> The font guard (T005) was strengthened during implementation. A single pinned width would have proved only that the number had not changed; it now also asserts that every shaped glyph matches the committed face's *own* `hmtx` advance exactly, which ties the measurement to the file rather than to a constant. Measured: Roboto 182.2px vs the host fallback 193.8px for the same string, so the pin is demonstrably doing work.

---

## Phase 3: User Story 1 — A layout regression fails the build and names what moved (Priority: P1) 🎯 MVP

**Goal**: Convert the class of defect that reached a person in feature 017 into a build failure that names the element.

**Independent Test**: Introduce a deliberate one-off spacing change in any covered state, run the check, confirm it fails naming that element. Revert; confirm it passes.

### Tests for User Story 1 (write first, confirm they FAIL) ⚠️

- [X] T014 [US1] Failing test in `crates/micold-client/tests/layout_snapshot.rs` asserting the generated text matches the committed fixture byte-for-byte. Fails initially because no fixture exists yet, which is the correct Red (FR-003, SC-001)
- [X] T015 [US1] Failing test in `crates/micold-client/tests/layout_snapshot.rs` asserting a mismatch names the covered state, the element — by anchor name where one covers the path, otherwise by path — and the recorded versus observed geometry side by side. Driven by a synthetic mismatch, not by editing the application. A message reading only "the layout changed" must fail this test (FR-004, SC-001, contract §5)
- [X] T016 [US1] Failing test in `crates/micold-client/tests/layout_snapshot.rs` asserting coverage never narrows silently: a covered state that can no longer be constructed fails naming it, and an anchor whose path no longer resolves fails naming it (FR-014, US1 acceptance scenario 4)
- [X] T017 [US1] Failing test in `crates/micold-client/tests/layout_snapshot.rs` asserting every covered state resolved in the scheme the fixture does **not** record yields byte-identical geometry, failing and naming the state if it differs. An equality assertion, not a second fixture (FR-008a)

### Implementation for User Story 1

- [X] T018 [US1] Implement `CoveredState` and `Anchor` in `crates/micold-client/tests/support/layout.rs` per `data-model.md`. Window size and colour scheme are deliberately **not** fields — both are uniform by requirement and declared once in the fixture header (FR-008a, FR-008b)
- [X] T019 [US1] Register feature 017's reduced parity set in `crates/micold-client/tests/layout_snapshot.rs`: main shell with the sidebar expanded, main shell with the sidebar collapsed, the add-worktree dialog in each of its two branch-source modes, and one open menu — every one built from the in-memory fixtures in `crates/micold-client/tests/support/mod.rs`, never from the developer's workspace (FR-007, FR-008, SC-004)
- [X] T020 [US1] Register the empty and error layouts. **`error-add-worktree-failed` had to open the dialog**: `worktree_error` is rendered by the add-worktree modal and nowhere else (`ui/mod.rs:357`), so setting it on the main shell covered nothing and the state was byte-identical to `main-shell-sidebar-expanded`. Distinctness is now checked — 9 states, 9 distinct layouts.
- [X] ~~T020a~~ (numbering artefact, ignore) Register the empty and error layouts in `crates/micold-client/tests/layout_snapshot.rs`: no project open, an unavailable project, a disconnected daemon. `State::default()` is already the no-project state (FR-008c, SC-004)
- [X] T021 [US1] Implement the fixture emitter in `crates/micold-client/tests/support/layout.rs` per `contracts/layout-fixture.md` §1 — header carrying renderer, font, window and scheme **once**, then one section per covered state with its anchor block and records (FR-003, FR-008a, FR-008b)
- [X] T022 [US1] Implement the byte-for-byte assertion and the failure-message construction in `crates/micold-client/tests/layout_snapshot.rs` (FR-003, FR-004)
- [X] T023 [US1] Declare the anchors in `crates/micold-client/tests/layout_snapshot.rs` — at minimum the sidebar row's label and its close button, the toolbar title, and the dialog action row. These are what a failure quotes and what T025 asserts against (FR-004, research R3)

  **Closed in full, 2026-08-05, after being unticked.** It was marked done twice while incomplete — first with none of the four anchors declared, then with two. The residue was the toolbar title and the dialog action row.

  `toolbar.title` is one path (`0/0/0/0/0/0/0`) on both shell states. **`dialog.actions` is not one path and cannot be**: it is the last child of a field column whose length depends on which inputs the form shows, so it lands at index 6 on the new-branch form, 3 on the existing-branch form, 7 once a validation error adds a row, and 8 on Settings. Five declarations, one per dialog state.

  **The interesting part is what was *not* there.** `worktree_form.rs` pushes `preview(...)` between the fields and the actions, so by the source the action row is index 7 on the new-branch form. It is index 6. `preview` returns `Space::new().height(0.0)` when the form has nothing to preview, and that produces no layout node at all — while other zero-sized elements in the same fixture (`0/0/0/1`, the toolbar's `Space::Fill`) are recorded normally. Reading the tree would have put every one of these five anchors one index too high; only the resolved geometry says otherwise. This is the same lesson as T023's first failure, in the opposite direction: an anchor that looks right in the source and points at the wrong node is worse than no anchor.

  So each name is held to a property only the thing it names satisfies, in `layout_snapshot.rs`: `the_toolbar_title_anchor_measures_the_application_name` shapes `APP_NAME` at `type_scale::BODY` and requires the anchored node to be that wide (83.1px), with `Toolbar`'s own signature — a full-width zero-height spacer as the next sibling — as a second check for anything that happens to measure the same. `the_action_row_anchors_are_action_rows` asserts the signature rather than the index: last in its column, two or more controls, side by side on one line. A field fails the second and third; a field label fails all three.

  Both were verified against failing runs before being trusted. Pointing `toolbar.title` at a trailing action reports *"toolbar.title is 69.3px wide but "Micold AI IDE" measures 83.1px"*; pointing `dialog.actions` at the name field reports *"not the last child of its column — a field was added below it, or the anchor is pointing at a field"*. `measure` moved into `support/layout.rs` so both files shape text the same way.

  Fixture diff: **7 added `@` lines, no geometry changed** — nothing moved, the failures just say what they are now. 1142 passing, 0 failing; no application source touched (FR-019, FR-004, SC-002, SC-007)
- [X] T024 [US1] Generate the committed fixture `crates/micold-client/tests/fixtures/layout_snapshot.txt`, and confirm `style_snapshot` still passes **with no regeneration** — that is the mechanical proof the application was not touched (FR-003, FR-019)
- [X] T025 [US1] **Demonstrated: the gate catches the defect it was built for.** With 017's fix undone in `material/ellipsized.rs`, `tests/layout_text_overflow.rs` fails naming the real label — `"A deliberately long worktree name that crowds its controls" wants 283.5px in 187.6px at node 0/0/0/2/0/0/0/2/0/0/2/0/1`. With the fix restored it passes. The geometry fixture stays green in both runs, confirming the split is structural rather than incidental (FR-018, SC-003)

  > **Two earlier attempts at this task failed, and the reasons are worth keeping.** The first
  > claimed success on a false positive whose numbers were identical with and without the defect.
  > The second, after `containing_width` was corrected, could not fire at all — because
  > `with_project()` never opened a project, so the label was never rendered. Nine covered states
  > collapsed to six distinct layouts, with sidebar-expanded and sidebar-collapsed byte-identical.
  >
  > `workspace_with` ends by clearing `active`, which every other caller wants and this one did not.
  > One line and an assertion fixed it: 6 distinct layouts became 8, and the fixture went from 708
  > to 1335 lines. The assertion is there so a covered state that stops covering fails loudly rather
  > than pinning an empty screen.

**Checkpoint**: **US1 complete.** 919 passing (baseline 893). Both gates validated in both directions. **All nine covered states now resolve to nine distinct layouts**, so every registered state covers something no other one does. SC-006 remains unmet (23s against 10s). *(Historical: SC-006 was later amended and met — T033.)*

> **917 passing, 0 failing** (baseline 893, +24). `style_snapshot` passes with no regeneration and
> `git status` over `crates/*/src/` is empty — the FR-019 proof. Fixture is 708 lines over 9 covered
> states.
>
> **Two findings recorded rather than absorbed.**
>
> 1. **Scheme-independence is not absolute.** Two elements resolve differently between light and
>    dark: the row carrying the resolved theme's own name and its text child. The cause is content,
>    not layout — `"Micold Light"` is one word wider than `"Micold Dark"` (`material/style.rs:66`),
>    so the row that shrinks to fit it is 2.4px narrower in dark. Structure is identical everywhere,
>    with no exemption. FR-008a's premise holds for *structure* without qualification and for
>    *geometry* with two declared exemptions, each carrying its reason and each required to keep
>    firing — a stale one fails the test rather than silently widening the gate.
> 2. **SC-006 is not met: the gate runs in 23.3s against a 10s budget.** The cost is text shaping —
>    roughly 100 full view resolutions across six tests, since three of them each emit the whole
>    fixture and the scheme test resolves every state twice. Caching the generated fixture behind a
>    `OnceLock` and sharing one renderer would cut most of it. Not attempted yet; recorded as
>    outstanding rather than reported as met.
>
>    *(Historical. The caching was done and cut it to 12.2s; the budget itself was the remaining
>    problem and was amended after measurement — T033, and SC-006 in `spec.md`.)*

---

## Phase 4: User Story 2 — An intended layout change is easy to accept and to review (Priority: P2)

**Goal**: A gate that is painful to satisfy gets bypassed or deleted. Make acceptance one command and the diff review evidence.

**Independent Test**: Make an intentional layout change, run the documented regeneration command, confirm the diff is human-readable and limited to the affected elements.

### Tests for User Story 2 (write first, confirm they FAIL) ⚠️

- [x] T026 [P] [US2] **Done.** `tests/layout_snapshot_regeneration.rs` — five tests over the three moments that matter (passing run, failing run, missing fixture) plus both halves of the ratchet. **The write decision was extracted into `support::layout::compare_or_regenerate` so the test drives the code the gate runs**, not a restatement of it: a test that reimplements the branch it checks agrees with itself whatever the gate does. All three negative cases were **verified against a deliberately unguarded implementation** — with the `if regenerate` guard removed they fail naming the exact failure ("a fixture mismatch did not fail the gate at all", "a missing fixture did not fail the gate, so the snapshot can silently cover nothing", and `Regenerated` where `Matched` was expected) — then pass once it is restored. `an_explicit_regeneration_does_write_the_fixture` holds the other direction, since the first three are satisfied by a function that never writes at all and a fixture that can never be updated is no better. `only_the_documented_variable_triggers_regeneration` reads the gate's own source, because the behavioural tests pass the flag in directly and therefore cannot see the trigger widening; verified by swapping the call site to `cfg!(debug_assertions)`, which it names and rejects. Every case runs against `CARGO_TARGET_TMPDIR`, never the committed fixture — a check about not clobbering a baseline would be a poor thing to implement by clobbering the baseline (FR-013)

### Implementation for User Story 2

- [x] T027 [US2] **Done, and it had been done since T024 without a checkbox** — the fixture could not have been generated otherwise. Recorded here because "already true" is exactly how an unverified claim survives: nothing asserted it until T026, and the branch it guards is the one that quietly rewrites the baseline. Now behind `UPDATE_LAYOUT_SNAPSHOT=1` in `support::layout::compare_or_regenerate`, called from `layout_snapshot.rs`, mirroring 017's `UPDATE_STYLE_SNAPSHOT` (FR-013, contract §6)
- [x] T028 [US2] **Done — quickstart Part C run in full, 2026-08-04.** The intentional change: the sidebar content column's left/right padding, `spacing::XS` to `spacing::SM` (`ui/sidebar.rs`). Applied temporarily as a probe and reverted; app source is byte-identical after (FR-019).

  - **The fixture updates and the check passes.** Yes. `UPDATE_LAYOUT_SNAPSHOT=1` regenerated; a re-run was green.
  - **The diff is limited to the affected elements.** Yes, and demonstrably so rather than by assertion: **610 changed lines, 610 removed — exactly balanced**, so nothing was added or dropped, only moved. Ten of eleven states churned, and the one that did not is `empty-no-project-open`, the only state with no sidebar. Churn tracked sidebar content: 71 lines in each state rendering a full sidebar, 26 in `empty-project-without-worktrees`, and 16 in `main-shell-sidebar-collapsed`, whose drawer is parked. A sidebar change touching every sidebar is the correct blast radius, not churn.
  - **Each changed line identifies an element and its state without running the application.** Yes. The fixture carries `## state` headers and each record leads with its path; the failure message names the covered state, the element and both geometries side by side — e.g. `main-shell-sidebar-expanded`, path `0/0/0/2/0/0/0/0`, recorded `4.0 43.2 252.0 26.2` versus observed `8.0 43.2 244.0 26.2`. The 4px shift and the 8px narrowing are both legible as the padding change that caused them.
  - **A run without the variable never rewrites the file.** Verified the way Part C asks rather than by trusting T026: fixture restored with `git checkout`, gate re-run against the still-modified source, gate failed, and `git status --short` reported the fixture **unmodified**.

  (SC-005)

**Checkpoint**: intended changes are cheap to accept and legible in review.

---

## Phase 5: User Story 3 — Coverage is visible and cheap to extend (Priority: P3)

**Goal**: Feature 017's real failure was not a missing test — it was an unclear boundary between what CI verified and what a human still had to. Keep this gate from acquiring the same ambiguity.

**Independent Test**: Add a new covered state, confirm it takes a single registration step, and confirm the documented coverage boundaries match reality.

### Tests for User Story 3 (write first, confirm they FAIL) ⚠️

- [x] T029 [P] [US3] **Done.** `tests/layout_coverage_registry.rs` scans every `.rs` under `tests/` and asserts that `CoveredState` and `RevealingState` are constructed only in `support/covered_states.rs`, in the shape of `one_overlay_implementation`. Held both ways: a second site fails the one-place check, and a registry that stops constructing either kind fails a staleness check — without the second, the first would pass vacuously over a codebase where the scan had stopped recognising registrations. **Both halves were verified against induced failures**, not assumed: adding a `CoveredState` to `layout_snapshot.rs` fails naming the file and the kind, and pointing `REGISTRY` at a file that registers nothing fails with "constructs no covered states at all". A third test pins the one piece of judgement in the file — that `pub struct CoveredState {`, `impl … for CoveredState {`, `&[CoveredState]` and `MyCoveredState {` are none of them registrations — because `support/layout.rs` both defines the types and names them in signatures, and a scan that counted those would report the definition site as a second registry with no way to satisfy it. The source stripper removes comments, **string literals and char literals**: the first run reported the scan's own file, whose assertions quote `CoveredState {`, and exempting it by name would have hidden a real registration there later. Char literals matter because `ui_glyph_literals.rs` contains `'"'`, which a naive stripper reads as opening a string and then swallows the code after it

### Implementation for User Story 3

- [x] T030 [US3] **Done.** `docs/development/layout-snapshot.md` leads with a decisive "Would this be caught?" table answering every category in quickstart Part G, then states each boundary with its reason: appearance and pixels (`style_snapshot`'s), animation (records taken at rest — with the one narrow exception that `revealing_states()` pins a mid-reveal frame for the containment invariant *only*, and nothing checks its geometry), scrolling (offset zero), typography (measured against a pinned Roboto, so consistent rather than faithful until 018 ships — flagged as the largest gap), and path stability (one structural edit renumbers descendants, so a large diff can be correct). It also writes down the three exemptions currently in force — `KNOWN_ESCAPES`, off-window parking, and the two dark-scheme geometry differences — each with why it exists and the note that it must keep firing. Original: Add `docs/development/layout-snapshot.md` documenting what the gate covers and — explicitly — what it does not: colour and border (owned by `style_snapshot`), pixels, mid-animation geometry, scrolled geometry, production typography until 018 ships, and path stability across structural edits. SC-007 makes this testable: a reader must be able to answer "would this catch X?" from the documentation alone (FR-015, SC-007)
- [x] T031 [US3] **Done.** Linked from `docs/README.md` under Development, above the component showcase, with a summary naming the three checks and what none of them cover (Principle VII)
- [x] T032 [US3] **Done, and it found FR-016 to be false before it was true.** Added `settings-dialog-with-validation-error` — the Settings modal, the only covered state with a checkbox, four labelled inputs in one modal, and a validation error on a text field. Values are invented and fixed rather than `SettingsDraft::default()`, whose fields track the shipped defaults and would re-record the fixture the day one of them changes (FR-007). **The fixture gained exactly that state: 180 insertions, 0 deletions, one new `##` header.** Suite **1081 passing, 0 failing**.

  **The finding.** Registering it took a change in *two* places, not one: the state renders a sidebar, so the collapsed filter accordion appeared, and `no_layout_node_escapes_its_parent` demanded a matching `CLIP_REVEALED` entry before it would go green. FR-016 had held for the first nine states only because they were all added at once — the failure mode nobody notices. Fixed at the cause rather than by adding the entry: the exemption is now keyed by **node path alone**, since being a clip-revealing wrapper's child is a property of the widget and not of the screen it appears on. Seven entries collapse to two (the same accordion under two shell arrangements, the disconnection banner shifting the shell index from 2 to 3). What it trades away is written down in the file: a *different* node arriving at one of those paths would be exempted silently, bounded by the staleness assertion and by `the_recorded_escapes_are_the_accordion_reveal` re-deriving attribution from behaviour. Registering a covered state is now genuinely one edit in one file, which is what US3 acceptance scenario 1 asked to be demonstrated rather than asserted.

  **Cost re-measured with the tenth state**: suite **37.50s** with, **35.06s** without → **2.44s per state** against SC-006a's 3s ceiling, and 37.50s against SC-006's 60s budget. Both still met. Measured by adding and removing the state, not derived (US3 acceptance scenario 1)

**Checkpoint**: all three stories complete; the boundary is written down and enforced.

---

## Phase 6: Polish & Cross-Cutting Concerns

- [x] T038 **Containment invariant added as a third gate** (`tests/layout_containment.rs`), prompted by BUG-001 landing on `main` during a rebase. Asked whether the text-overflow gate would catch it; it cannot, for three independent reasons — it compares **widths** only and BUG-001 is vertical; no covered state opens the filter panel; and the defect exists only at `0 < progress < 1` while both gates resolve one settled frame. **The deeper finding is that the geometry fixture could not catch it either, and not for want of data**: `layout_snapshot.txt` already records every box involved. A byte-compare fixture records whatever it is shown as correct, so a defect older than the fixture is regenerated into the expected value. Snapshots catch *changes*; this needed an *assertion*. The invariant — no child node laid outside the parent that owns it — fires on **7 of 9** covered states, all one defect: the collapsed accordion, a 40–42px child inside a 0px `Expand`. Attribution is proven rather than inferred (`the_recorded_escapes_are_the_accordion_reveal` opens the panel and shows the same nodes come clean). Recorded in `KNOWN_ESCAPES` with a staleness assertion, **not fixed** (FR-019). One limit stated in the file: nodes parked entirely off-window are exempt, which `NavigationDrawer` relies on and which buys silence about accidentally off-screen content
- [x] T039 **Mid-reveal state added, so the gate covers the overlap a user actually sees.** T038 caught BUG-001's cause at rest, where `Expand::draw` returns early and nothing paints. `revealing_states()` pins the sidebar's filter panel two frames into its 90ms reveal (~0.356) via a redraw pump (`resolve_revealing`), where the child is both oversized and painting. **Deterministic despite pinning an animation**: a track steps a fixed amount per redraw rather than by elapsed time, and the `Instant` in the event is never read. **Deliberately not in the fixture** — T030 excludes mid-animation geometry, and a recorded mid-reveal frame would churn on any change to a duration or easing curve; `revealing_states` is a second *list* in `covered_states.rs`, not a second registration *site*, and T029's scan should expect both there and neither elsewhere. Both assertions were verified against failing runs: frame 0 fails the pin check, and **applying BUG-001's own recommended fix fails the escape check** while the pin still reads 0.356. That probe also caught a design error — `expect_between` originally measured the revealing node against its child, which reads 1.0 at every moment once the child is clipped, so a *fix* would have been reported as a broken pin; it now measures against the fully open height. And it showed the fix must clip recursively: with one level clipped, the grandchild escaped instead. Recorded on BUG-001 for whoever writes the fix
- [x] T041 **The overlay pass was recording nothing, and nothing said so.** Found while reviewing T032: `layout_snapshot.txt` contained **zero `over` records**. The overlay pass (T013, FR-009) had been implemented, documented, shipped and run over every covered state — and the only widget in this application reached through `Widget::overlay` is `material::Select`'s `pick_list` dropdown, which no covered state ever opened. Nothing failed, because a pass that records nothing is indistinguishable from a pass that found nothing. That is the same defect this feature exists to correct, arrived at from the opposite direction: FR-009's claim was true about the code and false about the fixture.

  **Fixed by opening one.** `pick_list`'s open flag is private widget-tree state with no accessor, so it cannot be set — only *caused*. `StateUnderTest::pressing` dispatches a left press at the named control's centre, the way a person opens it, and `add-worktree-dialog-type-menu-open` registers it. The fixture now carries four `over` records: a 472x342 dropdown at (404.0, 397.1).

  **The press was swallowed at first, and the reason is worth keeping.** A probe pressing all 128 nodes of the add-worktree dialog changed *nothing* — no overlay, no base-tree difference. A dialog mounts at progress 0 by design (`Motion::enter`, "a dialog is mounted precisely because it is opening") and `Fade::update` returns early below `HIDDEN` for every event that is not a `Window` event. So a modal refuses clicks until it has finished appearing, which is correct behaviour and invisible from the outside. `resolve_pressing` now settles the entrance with eight pumped redraws first — layout is not recomputed between them, since `Fade` is layout-neutral and eight full layouts per state per scheme would cost more than the state is worth.

  **Held both ways.** `the_overlay_pass_records_something_somewhere` fails if no covered state produces an overlay record, verified by removing the press; `a_state_that_presses_a_control_records_the_control_it_opened` fails if a state that presses something opens nothing, which is the only evidence the press landed at all.

  **Cost: 2.09s** for this state (suite 36.99s with, 34.90s without), inside SC-006a's 3s ceiling. **Measured warm** — the first attempt read 6.23s because a rebuild triggered by the edit landed inside the timing, which is worth recording since it would have read as a breached criterion (FR-009, SC-006a)

- [x] T040 **Containment gate folded into the `layout_snapshot` binary** (`tests/gates/containment.rs`, reached by `#[path]`). Cargo makes one binary per file directly under `tests/`, and each is its own process, so a `OnceLock` cannot cross between them — standing alone the gate re-resolved the same nine covered states `layout_snapshot` had already resolved. Sharing the process makes `cached_records` serve both. **Measured: the containment gate's marginal cost on `mise run test` falls from 8.9s to ~0.8s, and the suite returns to 35.1s — essentially its 34.3s size before the gate existed.** It remains a distinct gate with its own tests and failures; only the process is shared. **This does not fix SC-006 and moves its two clauses in opposite directions** — see T033
- [x] T033 [P] **SC-006 amended, then met — and the order matters.** Against the amended criterion: `mise run test` **35.1s** (budget 60s) and **2.21s** per covered state (ceiling 3s), the latter measured by adding a tenth state and removing it again rather than derived from resolution counts. **Nothing got faster to achieve this**; the criterion changed because measurement showed the original was unmeetable *and* mis-specified. The amendment is recorded in `spec.md` with the original text and three reasons: it named a test binary, which the gates now share and move between freely; its 10% share tightened whenever the rest of the suite got faster, so unrelated speedups could fail it; and its number came from R9 predicting that font-system construction would dominate, when per-state text shaping does. R9 now records that outcome against the prediction. **The floor is unchanged and still real**: 9 states × 2 schemes ≈ 12s, and the three ways under it all cost something (fewer states weakens FR-008/SC-004; dropping the dark pass violates FR-008a; faster shaping is not ours). Anyone re-tightening this should move SC-006a, not SC-006 — a fixed total becomes a record of what happened rather than a budget. Superseded measurement: `layout_snapshot` **14.9s** against a 10s budget, and feature 019's gates total **22.7s of a 35.1s suite (65%)** against a 10% ceiling. **T040 moved the two clauses in opposite directions**: it cut ~7.9s of duplicated resolution from the suite, which is the second clause's subject, while making the `layout_snapshot` binary itself slower (12.3s → 14.9s) because the containment tests now run inside it. That is the right trade — total work done is what a developer waits for, and the first clause's 10s was never reachable anyway — but it should be recorded rather than presented as an improvement across the board. **The floor has not moved: 9 covered states × 2 schemes of real text shaping is ~12s**, and the three ways under it all cost something real (fewer states weakens FR-008/SC-004; dropping the dark pass violates FR-008a; faster shaping is not ours to write). Recommend amending SC-006 to a budget derived from what the work actually costs, and to name the *suite* rather than one binary now that the gates share processes. Earlier measurement: Gate alone **12.2s** (budget 10s); full suite **34.3s**, so the gate is **~35%** of it (ceiling 10%). Down from 23.3s after caching each covered state's records once per scheme — six tests were resolving ~71 full views; it is now 18, which is the floor without giving something up. **The remaining cost is text shaping across 9 screens × 2 schemes, and the three ways to go lower all cost something real**: fewer covered states (weakens FR-008/SC-004), dropping the dark pass (violates FR-008a), or faster shaping (not ours). Recommend amending SC-006 rather than gaming it by cutting coverage — the budget was written before anyone knew what resolving real text costs. Original text: Measure SC-006: `layout_snapshot` completes in under 10 seconds locally, and adds no more than 10% to `mise run test` runtime. Record **both** numbers — measured with and without the gate — rather than asserting the budget was met
- [x] T034 [P] **Done. 1081 passing, 0 failing** (`mise run test`, 2026-08-04), against T001's baseline of **893**. No decrease to explain. The gap is larger than this feature's own contribution — the rebase onto `origin/main` brought in the BUG-001 fix's `animated_layout_relayouts.rs`, the frame probe and the re-authored palette's contrast tests — so this confirms nothing was *lost*, which is all it was ever able to confirm
- [x] T035 **Done, 2026-08-04. Green on all three platforms on commit `7065bba`, PR #62** — ubuntu-latest 5m38s, macos-latest 1m23s, windows-latest 2m59s, plus fmt+clippy and docs. Same commit, same committed fixture.

  **It earned its keep on the first attempt, which failed.** Ubuntu went red while macOS and Windows passed — the opposite of the predicted risk, and on the very platform the fixture was recorded on. The cause was not text but **icons**: a glyph from `Material Symbols Outlined` is shaped and measured like any other text, and the apparatus loaded only the Roboto faces, so every icon resolved through the host's fallback at whatever width that machine offered. Icon nodes came out 8.4px where 6.3px was recorded, shifting every adjacent label 2.1px. Loading the icon face changed **831 lines of the fixture locally too** — the local recording had been wrong all along, merely self-consistently.

  Three local runs and two of three runners were green on that defect. Nothing except running on all three platforms could have found it, which is the whole argument for FR-017 and Principle VI being requirements rather than formalities. Guarded against regression by `the_icon_face_parses_and_is_the_face_that_measures`, which pins the glyph to the committed face's own advance rather than to a constant, and was verified against a run with the face deliberately unloaded (9.6px against the committed 16.0px).

  The same run also caught fmt and two genuine clippy findings, neither stylistic: a `Default::default()` field-assignment pattern in the covered-state builders and a needless reference in the scheme-equality check. Both had been present for the whole feature and never seen, because the branch had never been pushed (FR-017, Principle VI)
- [x] T036 **Done, 2026-08-04. Parts A–G run; 37 of 38 boxes ticked.** The one left is Part E's "CI green on Linux, macOS and Windows", which is T035 and cannot be checked locally by construction. Two steps were found to be wrong and are amended in place with their evidence rather than quietly reworded.

  **Part A** — suite green; every gate binary also passes under `env -u DISPLAY -u WAYLAND_DISPLAY` (9 + 9 + 12 + 2 + 3 + 5 tests). **B1** — the padding change fails naming state, element and both geometries side by side. **B2 — amended: it cannot pass, and it must not.** It asked that wrapping an element in a no-op container leave the check green; identity in the fixture is the depth-first path, a wrapper *is* a node with geometry of its own, and everything under it renumbers. Measured: the header kept its geometry exactly and moved one level down. The premise contradicts T007, which requires tree order precisely so a structural reordering cannot hide — a fixture blind to inserted nodes is a worse gate. `docs/development/layout-snapshot.md` already had the right account under **Path stability**; the two documents disagreed and the quickstart was wrong. **B3** — held by two tests that run every suite rather than a one-off manual edit. **B4** — the motivating defect still fails after the rebase: 283.5px wanted in 187.6px, overflow 95.9px, and the geometry fixture stays green throughout, which is what proves the split between the checks is structural. **C** — see T028. **D** — see T032. **F** — 37.0s against 60s, 2.09s per state against 3s. **G** — all seven categories answered directly by the boundary table.

  **T023 was a false completion, found by B4.** It is marked done and asked for anchors on "the sidebar row's label and its close button, the toolbar title, and the dialog action row" — **none of which existed**. Only `shell.root`, `shell.body` and `dialog.root` were ever declared, so FR-004's "by anchor name where one covers the path" fell through to a bare path for every real failure, including the one message this whole feature is justified by. Partially closed: `sidebar.row.label` and `sidebar.row.delete_button` are now declared, identified from the running gate rather than by reading the tree, and `layout_text_overflow` now resolves anchors — B4 reports `sidebar.row.label (0/0/0/2/0/0/0/2/0/0/2/0/1)` where it reported only the path. The adjacent control is `row_actions_cluster`'s **Delete**, not a close button, so it is named for what it is. **Still missing: the toolbar title and the dialog action row.** Recorded rather than guessed — a mislabelled anchor is worse than an honest path (SC-002, SC-007).

  *Superseded 2026-08-05*: T023 was unticked and closed in full. See its own entry.
- [x] T037 **Done, 2026-08-04. Empty, as required.** `git diff --stat $(git merge-base origin/main HEAD)..HEAD -- crates/micold-client/src crates/micold-core/src crates/micold-daemon/src` returns nothing: across the whole feature, 26 files and 6,320 insertions, **not one line under any `src/`**. The merge base is the right comparison and a plain `git diff origin/main` is not — `origin/main` is 23 commits ahead, so that form reports upstream's own work as though it were this branch's, which it did on first run. Every probe that touched application source (`animation.rs` twice, `sidebar.rs` twice, `ellipsized.rs` twice) was restored from a copy and re-verified; layout defects found along the way were recorded on BUG-001 and in `CLIP_REVEALED`, never silently fixed

---

## Dependencies & Execution Order

### Phase Dependencies

- **Phase 1 Setup**: no dependencies. T001 must run **before any other task** — the baseline cannot be taken afterwards
- **Phase 2 Foundational**: depends on Phase 1. **Blocks every user story**
- **Phase 3 (US1)**: depends on Phase 2. Delivers the MVP
- **Phase 4 (US2)**: depends on Phase 3 — regeneration needs something to regenerate
- **Phase 5 (US3)**: depends on Phase 3. Independent of Phase 4
- **Phase 6 Polish**: depends on everything

### Within Phase 3

T018 → T019/T020 (the registry needs the type) → T021 → T022 → T023 → T024 → T025. T014–T017 precede all of them and must be observed failing.

### Parallel Opportunities

Genuinely limited, and worth stating honestly rather than dressing up: this feature concentrates in four files (`tests/support/layout.rs`, `tests/layout_snapshot.rs`, and two smaller test binaries), so most tasks touch a file another task is already in.

Real opportunities:

- T002, T003 — asset and module skeleton, different files
- T004 and T006 — two different test binaries
- T026, T029 — each a new standalone test file
- T033, T034 — independent measurements

Everything in Phase 2's implementation block (T010–T013) lands in one file and is sequential.

---

## Implementation Strategy

### MVP (User Story 1)

1. Phase 1 Setup — **T001 first, always**
2. Phase 2 Foundational — the apparatus
3. Phase 3 US1
4. **STOP and VALIDATE** — T025 is the gate on the gate. If reintroducing the sidebar overlap does not fail the check, nothing downstream is worth building

US1 alone is a complete, deliverable increment: layout regressions fail the build and name what moved. US2 and US3 make it pleasant and honest, respectively; neither is required for it to be useful.

### Incremental Delivery

- US1 → the gate exists and catches regressions
- \+ US2 → intended changes are cheap to accept and reviewable
- \+ US3 → coverage is extensible and its limits are documented

### Risk Notes

- **The font family lookup is the one silent failure mode.** T005 exists because a host font named Roboto winning the lookup would shift every measurement at once and read as a mass layout regression. This risk survives feature 018 — Roboto is a common system font name — so T005 is permanent, not scaffolding.
- **Path churn is expected, not a defect.** Inserting a container near the root renumbers its descendants, so one structural edit can produce a large but entirely correct diff. Anchors are re-pointed by hand as part of that change.
- **The gate pins what *is*, not what is *correct*.** A layout defect present when T024 generates the fixture is baked in until someone notices it by eye. T037 enforces that finding one is a finding to raise, not an edit to make. **T038 narrows this for one defect class**: an invariant asserted over the same records catches a pre-existing violation that the fixture would only regenerate as expected. It is worth noting how far that generalises — containment is checkable because it is a relation between two recorded numbers. "This spacing is wrong" is not, and stays a matter for the eye.
- **This feature depends on neither 018 nor 020** (spec D1 as resolved in planning, and D2). It can start immediately and blocks nothing.
