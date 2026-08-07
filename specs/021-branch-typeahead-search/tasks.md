---

description: "Task list for feature 021 — Branch Selector Type-Ahead Search"
---

# Tasks: Branch Selector Type-Ahead Search

**Input**: Design documents from `/specs/021-branch-typeahead-search/`

**Prerequisites**: [plan.md](./plan.md), [spec.md](./spec.md), [research.md](./research.md),
[data-model.md](./data-model.md), [contracts/](./contracts/), [quickstart.md](./quickstart.md)

**Tests**: MANDATORY per Constitution Principle I. Every implementation task below is preceded by a
test task that must be **observed failing** first. The GUI-wiring exception is claimed only for
`src/ui/`'s render glue, validated by [quickstart.md](./quickstart.md) §B — never for anything with a
decision in it, which is why the keyboard rule sits in `micold-core` rather than in the widget.

**Documentation**: Principle VII. The user-guide task ships inside User Story 1, not in Polish.

**Cross-platform**: Principle VI. Nothing here is platform-conditional; the final phase verifies all
three.

**Revised after `/speckit-analyze`** — eleven findings applied. Two changed the shape of the work
rather than its wording: the keyboard rule moved out of the widget into tested render-free code (C2),
and the gallery entry moved from User Story 3 into User Story 1, because `showcase_completeness.rs`
fails the build for a component that has no catalogue entry (C1, [R18](./research.md#r18)).

## Format: `[ID] [P?] [Story] Description`

- **[P]**: parallelizable — different file, no dependency on an incomplete task
- **[Story]**: US1 / US2 / US3, mapping to the spec's prioritized stories

## Path Conventions

Three-crate Rust workspace. `crates/micold-core/` is the iced-free logic crate;
`crates/micold-client/` holds the app, its two-layer component library (`src/ui/cdk/` behaviour,
`src/ui/material/` appearance) and the development-only showcase. Run everything through
`mise run test` / `mise run test-core`.

**One-file serialization**: `crates/micold-core/src/typeahead.rs` is a single file that several tasks
build up, so those tasks are **not** parallel with one another even though their tests are. The same
holds for `app.rs` and for `showcase/state.rs`.

---

## Phase 1: Setup

**Purpose**: empty, compiling homes for the new code, so later tasks are edits rather than
simultaneous file creations.

- [X] T001 Create empty `crates/micold-core/src/typeahead.rs` with its module doc, and register it with `pub mod typeahead;` in `crates/micold-core/src/lib.rs`
- [X] T002 [P] Create empty `crates/micold-client/src/ui/cdk/typeahead.rs` with its module doc and register it in `crates/micold-client/src/ui/cdk/mod.rs`
- [X] T003 [P] Create empty `crates/micold-client/src/ui/material/typeahead.rs` with its module doc and register it (plus its re-export) in `crates/micold-client/src/ui/material/mod.rs`

**Checkpoint**: `mise run test` green, nothing behaves differently yet.

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: the shared matching vocabulary every story reads, and the two architecture gates that
must be watching **before** the code they govern exists.

**⚠️ CRITICAL**: no user story work begins until this phase is complete.

- [X] T004 [P] Write failing tests for query normalisation — trim, case-fold, interior whitespace kept, no metacharacters (contract [match-ranking.md §1](./contracts/match-ranking.md#1-normalisation), cases Q1.1–Q1.2) in `crates/micold-core/tests/typeahead_match.rs`
- [X] T005 Implement `Query`, `MatchKind` and `Match` per [data-model.md §1](./data-model.md#1-matching--micold_coretypeahead-new-module) in `crates/micold-core/src/typeahead.rs`, making T004 pass
- [X] T006 [P] Widen `crates/micold-client/tests/one_overlay_implementation.rs` to treat a hand-written `fn overlay(` implementation under `src/ui/` as a widget-attached delegation requiring a `SANCTIONED` entry, per [research R5](./research.md#r5). It must pass now (verified: no such impl exists today) and must fail the moment T022 lands one unsanctioned — that failure is T023's Red.
- [X] T007 [P] Write the new gate holding FR-021a — `src/ui/cdk/typeahead.rs` and `src/ui/material/typeahead.rs` may not name `branch`, `worktree` or `git` — in `crates/micold-client/tests/typeahead_is_generic.rs`, in the source-scanning style of `cdk_no_appearance.rs`. It passes on the empty stubs and stays passing only if the component really is generic (FR-019).

**Checkpoint**: the matching vocabulary exists and is tested; both new gates are watching. User story
work can begin.

---

## Phase 3: User Story 1 — Narrow the branch list by typing (Priority: P1) 🎯 MVP

**Goal**: type a fragment into the branch picker, see only branches containing it, with the matched
text emphasised in each row, and pick one.

**Independent Test**: [quickstart.md](./quickstart.md) §B1, §B3–§B5a, §B6–§B7 — open the picker in a
many-branch repository, type a shared fragment, confirm the list narrows, the fragment is emphasised
in every row, the rest of the form does not move, and picking a branch creates the session as before.

**Why this slice is large**: `showcase_completeness.rs` fails the build for a library component with
no catalogue entry, so the gallery entry lands here rather than in User Story 3
([R18](./research.md#r18)).

### Tests for User Story 1 (MANDATORY — Constitution Principle I) ⚠️

> Write these FIRST and observe them failing.

- [X] T008 [P] [US1] Write failing tests for the literal tier and tier exclusivity — offsets, single span, case-insensitivity (contract [§2.1](./contracts/match-ranking.md#21-literal--always-active), [§2.4](./contracts/match-ranking.md#24-tier-exclusivity), cases Q2.1.1–Q2.1.2, Q2.4.1) in `crates/micold-core/tests/typeahead_match.rs`
- [X] T009 [P] [US1] Write failing tests for ranking — literal-before-approximate, earlier position first, stable tie-break, empty query returns everything in input order (contract [§3](./contracts/match-ranking.md#3-ranking), cases Q3.1–Q3.4) in `crates/micold-core/tests/typeahead_rank.rs`
- [X] T010 [P] [US1] Write failing tests for `fit_around` with a monospace stand-in — leading, trailing and both-ends ellipsis; untouched when it fits; ranges rebased and on char boundaries; multi-byte safety (contract [§4](./contracts/match-ranking.md#4-truncation), cases Q4.1–Q4.5) in `crates/micold-core/tests/typeahead_fit.rs`
- [X] T011 [P] [US1] Write the same truncation expectations against **real shaping** rather than the stand-in, mirroring `material/ellipsized.rs`'s own two-level approach, in `crates/micold-core/tests/typeahead_fit.rs`
- [X] T012 [P] [US1] Write failing tests for the keyboard rule — saturation at both ends, `Enter` on a disabled or absent highlight yielding nothing, an empty list yielding nothing, ordinary keys falling through (contract [§4b](./contracts/match-ranking.md#4b-the-keyboard-rule), cases Q4b.1–Q4b.5; FR-017, FR-017a) in `crates/micold-core/tests/typeahead_keys.rs`
- [X] T013 [P] [US1] Write a failing budget test — `rank` over 500 synthetic branch names within 16 ms (contract [§5](./contracts/match-ranking.md#5-performance), SC-002) in `crates/micold-core/tests/typeahead_budget.rs`
- [X] T014 [P] [US1] Write failing reducer tests for the form's new state — query change recomputes matches, highlight re-seats rather than dangling, empty query restores full order, clearing restores the list, selection survives every query change, and `branch_query` is never written by anything but a query change (data-model [§2 invariants 1–3, 5](./data-model.md#2-form-state--micold_clientappworktreeform-extended); FR-002, FR-005, FR-014, FR-014a, FR-016) in `crates/micold-client/tests/branch_search_state.rs`
- [X] T015 [P] [US1] Write failing reducer tests for focus — focusing opens the list without touching the query, matches or selection; dismissal closes it and changes nothing else; a source change closes it (data-model §2 invariants 5–6; FR-001b) in `crates/micold-client/tests/branch_search_state.rs`
- [X] T016 [P] [US1] Write failing reducer tests for FR-012a — picking a blocked candidate is a no-op: no selection, list stays open — plus a direct unit test that `can_submit()` still refuses a blocked selection if one ever reached it (contract [typeahead-component.md §5](./contracts/typeahead-component.md#5-what-the-branch-picker-adds-on-top), [research R13a](./research.md#r13a)) in `crates/micold-client/tests/branch_search_state.rs`
- [X] T017 [P] [US1] Write a failing test for the happy path FR-013 promises — picking an **available** branch from the results produces the same `selected_branch`, `preview()` and `can_submit()` outcome the old list produced — in `crates/micold-client/tests/branch_search_state.rs`
- [X] T018 [US1] Rewrite feature 016's `can_submit()` assertions around a blocked selection (`crates/micold-client/tests/app_state.rs`, the block at lines ~925–965) into assertions that a blocked branch cannot become the selection at all. Deleting them is not acceptable — the invariant moves one layer earlier, it does not disappear.

### Implementation for User Story 1

- [X] T019 [US1] Implement `match_one`'s literal tier and `rank` per contract §2.1/§2.4/§3 in `crates/micold-core/src/typeahead.rs`, making T008, T009 and T013 pass
- [X] T020 [US1] Implement `fit_around` per contract §4 in `crates/micold-core/src/typeahead.rs`, making T010 and T011 pass
- [X] T021 [US1] Implement `Key`, `Intent` and `intent_for` per contract §4b in `crates/micold-core/src/typeahead.rs`, making T012 pass. No iced type may appear — `Key` is this crate's own enum, as `keymap.rs`'s `KeyInput` is.
- [X] T022 [US1] Implement the behaviour half — the widget, its `Widget::overlay()`, translation of input events into `Key`, application of the returned `Intent`, pointer handling and outside-dismissal — per contract [§3](./contracts/typeahead-component.md#3-behaviour-the-cdk-half) in `crates/micold-client/src/ui/cdk/typeahead.rs`. It decides nothing itself. **T006 now fails**: the delegation is unsanctioned.
- [X] T023 [US1] Add the `SANCTIONED` entry for the new widget, with its reason, in `crates/micold-client/tests/one_overlay_implementation.rs`, making T006 pass again (Green)
- [X] T024 [US1] Implement the appearance half — builder API (`new`/`placeholder`/`highlighted`/`selected`/`on_pick`/`on_move`/`on_focus`/`on_dismiss`/`empty_message`, terminating in `.into()`), the Material field with leading search and trailing clear affordances, the anchored menu surface, and row rendering via `rich_text` spans — per contract [§1](./contracts/typeahead-component.md#1-builder-api), [§4](./contracts/typeahead-component.md#4-appearance-the-material-half) in `crates/micold-client/src/ui/material/typeahead.rs`
- [X] T025 [US1] Give the rows their treatments — emphasis (token colour role plus weight, never a filled background), disabled, and selection marker distinct from the keyboard highlight — per contract §4.3, §4.5, §4.7 (FR-011, FR-011c, FR-012b, FR-014b) in `crates/micold-client/src/ui/material/typeahead.rs`
- [X] T026 [US1] Wire truncation into row rendering so an emphasised run is never hidden behind an ellipsis, per contract §4.4 (FR-011d) in `crates/micold-client/src/ui/material/typeahead.rs`
- [X] T027 [US1] Add the `cdk/typeahead.rs` `EXEMPTION` — behaviour-layer wrapper with no appearance, the reason `cdk/overlay.rs`'s two components already use — in `crates/micold-client/src/showcase/catalogue.rs` (contract [§6.0](./contracts/typeahead-component.md#6-gallery-entry))
- [X] T028 [US1] Add fixed sample rows — realistic names covering a literal hit, a long truncating name and a disabled row — in `crates/micold-client/src/showcase/samples.rs`
- [X] T029 [US1] Write the entry's render function beside the other input controls in `crates/micold-client/src/showcase/sections/controls.rs`, posing the component statically for now (live typing arrives in User Story 3)
- [X] T030 [US1] Add the `catalogue::Entry` naming `material/typeahead.rs` / `Typeahead` in `crates/micold-client/src/showcase/catalogue.rs`, bringing `tests/showcase_completeness.rs` back to green
- [X] T031 [US1] Add `branch_query`, `branch_matches`, `branch_list_open` and `branch_highlight` to `WorktreeForm`, plus the five new messages and their transitions per [data-model.md §2](./data-model.md#2-form-state--micold_clientappworktreeform-extended), in `crates/micold-client/src/app.rs`, making T014–T017 pass
- [X] T032 [US1] Rewrite `branch_picker()` onto the component — `BranchCandidate` → `Row` mapping (label from its existing `Display`, `enabled` from `is_available()`, `selected` from `selected_branch`), the two repository-level messages left inline per [research R14](./research.md#r14) — in `crates/micold-client/src/ui/worktree_form.rs`
- [X] T033 [P] [US1] Document branch search in `docs/user-guide/worktrees-and-sessions.md` — that typing narrows the list, what the emphasis means, and that a branch in use elsewhere is shown but cannot be chosen (FR-022, Principle VII)
- [X] T034 [US1] Run [quickstart.md](./quickstart.md) §B1, §B3–§B5a and §B6–§B7 and record the pass — the manual half Principle I's GUI exception requires for the render glue, and the only check on SC-001 and SC-006

**Checkpoint**: substring search with emphasis over a long branch list, fully usable, **whole suite
green including every showcase gate**. Shippable on its own.

---

## Phase 4: User Story 2 — Forgiving matching for near misses (Priority: P2)

**Goal**: half-remembered names still surface — abbreviations and single typos — ranked below literal
hits, with the emphasis shaped to the kind of match.

**Independent Test**: [quickstart.md](./quickstart.md) §B2 — type `reportng` and `frep` and confirm
`feat/reporting` appears each time with the right emphasis shape, that two characters yield only
literal matches, and that nonsense yields an explicit no-match message.

### Tests for User Story 2 (MANDATORY — Constitution Principle I) ⚠️

- [X] T035 [P] [US2] Write failing tests for the subsequence tier — greedy-leftmost alignment, one span per corresponding character, deterministic across runs (contract [§2.2](./contracts/match-ranking.md#22-subsequence--active-from-3-query-characters), case Q2.2.1) in `crates/micold-core/tests/typeahead_match.rs`
- [X] T036 [P] [US2] Write failing tests for the single-edit tier — insert, delete and substitute; the whole window as one span; no match for an unrelated query (contract [§2.3](./contracts/match-ranking.md#23-single-edit--active-from-3-query-characters), cases Q2.3.1–Q2.3.2; FR-008) in `crates/micold-core/tests/typeahead_match.rs`
- [X] T037 [P] [US2] Write failing tests for the 3-character floor — a 1- or 2-character query yields literal matches only (contract §2.2 case Q2.2.2; FR-006a) in `crates/micold-core/tests/typeahead_match.rs`
- [X] T038 [P] [US2] Write the failing ranking-quality benchmark — a pinned corpus of ~200 realistic branch names and committed `query → intended branch` pairs across all three tiers, asserting at least 95% rank in the top five and naming any pair that regresses (contract [§4a](./contracts/match-ranking.md#4a-ranking-quality-benchmark); SC-003) in `crates/micold-core/tests/typeahead_corpus.rs`
- [X] T039 [P] [US2] Extend that corpus with SC-001's claim — for each pair, an 8-character prefix of the intended name ranks it first — so the headline user outcome is measured and not merely asserted, in `crates/micold-core/tests/typeahead_corpus.rs`
- [X] T040 [P] [US2] Extend the budget test so the 16 ms bound is measured with **all three tiers active** — the expensive case, and the one SC-002 actually promises — in `crates/micold-core/tests/typeahead_budget.rs`
- [X] T041 [P] [US2] Write a failing reducer test for the no-match state — a query matching nothing leaves the list open with the empty message and the query intact and editable (data-model §2 invariant 6; FR-015) in `crates/micold-client/tests/branch_search_state.rs`

### Implementation for User Story 2

- [X] T042 [US2] Implement the subsequence tier with greedy-leftmost alignment in `crates/micold-core/src/typeahead.rs`, making T035 pass
- [X] T043 [US2] Implement the single-edit tier over windows of length `q-1`, `q`, `q+1` in `crates/micold-core/src/typeahead.rs`, making T036 pass
- [X] T044 [US2] Implement the 3-character floor gating both approximate tiers in `crates/micold-core/src/typeahead.rs`, making T037 pass
- [X] T045 [US2] Bring the corpus and budget tests green — T038, T039 and T040 — tuning the implementation, never the corpus, if any fails
- [X] T046 [US2] Supply the no-match message to the component from the picker, kept distinct from the two repository-level messages per [research R14](./research.md#r14), in `crates/micold-client/src/ui/worktree_form.rs`
- [X] T047 [P] [US2] Extend the user-guide section to cover near-miss matching and the three-character threshold in `docs/user-guide/worktrees-and-sessions.md`
- [X] T048 [US2] Run [quickstart.md](./quickstart.md) §B2 and record the pass

**Checkpoint**: User Stories 1 and 2 both work; approximate matching never outranks a literal hit.

---

## Phase 5: User Story 3 — A reusable type-ahead the rest of the app can adopt (Priority: P3)

**Goal**: the gallery's entry becomes a **live, typeable** example in both schemes, and the component
is documented where a future picker author will look.

**Independent Test**: [quickstart.md](./quickstart.md) §B8 — launch the showcase binary, type into the
Typeahead entry, watch it narrow and emphasise over sample data with no repository involved, in both
schemes.

**Depends on**: User Story 1, which introduced the component and its catalogue entry. Independent of
User Story 2 — the gallery example works with literal matching alone.

### Tests for User Story 3 (MANDATORY — Constitution Principle I) ⚠️

- [X] T049 [P] [US3] Write failing reducer tests for the showcase's new state — a query change filters the sample rows, the highlight moves and re-seats, a pick registers — in `crates/micold-client/tests/showcase_state.rs`
- [X] T050 [P] [US3] Confirm `crates/micold-client/tests/showcase_captions.rs` fails while the entry claims `interactive: true` with an empty `live` list, so the caption rule is observed holding before it is satisfied

### Implementation for User Story 3

- [X] T051 [US3] Drop `Copy` from `showcase::state::Message` (keeping `Clone`, `Debug`, `PartialEq`, `Eq`), add the query/highlight fields and the three new message variants per [data-model.md §4](./data-model.md#4-showcase-state--micold_clientshowcasestate) and [research R16](./research.md#r16), in `crates/micold-client/src/showcase/state.rs`, making T049 pass
- [X] T052 [US3] Repair every site the `Copy` removal breaks in `crates/micold-client/src/showcase/gallery.rs` and `crates/micold-client/src/showcase/sections/`
- [X] ⚠️ Reopened T053 [US3] Make the entry live — wire the sample rows through the real matching logic and the showcase's own query state — in `crates/micold-client/src/showcase/sections/controls.rs` (FR-020) *(reopened — BUG-001)*

  > **Half done.** The rows were made live; the **open state** was left as T029's static pose
  > (`.open(true)`), so the entry demonstrates a state the branch selector never rests in and never
  > shows FR-001b's open-on-reach rule. Closed again by T074 below, which retires the pose. The
  > matching half was correct and stays done.
- [X] T054 [US3] Set `interactive: true` and the `live` list on the entry in `crates/micold-client/src/showcase/catalogue.rs`, making T050 pass
- [X] T055 [P] [US3] Document the component — what it is for, its builder API, and that a new picker consumes it rather than rebuilding it — in `docs/development/component-library.md`
- [X] T056 [US3] Run [quickstart.md](./quickstart.md) §B8 and record the pass

**Checkpoint**: all three stories independently functional.

---

## Phase 6: Polish & Cross-Cutting Concerns

- [X] T057 Verify the architecture gates in `crates/micold-client/tests/` all pass together — `material_boundary.rs`, `cdk_no_appearance.rs`, `material_builder_api.rs`, `component_api_opacity.rs`, `one_overlay_implementation.rs`, `typeahead_is_generic.rs`, `idle_requests_no_frames.rs`, `logical_state_ownership.rs`, `showcase_completeness.rs`, `showcase_captions.rs` — with no budget raised and no exception added beyond T023's and T027's
- [X] T058 [P] Run the frame-probe half of SC-002 via `crates/micold-client/tests/frame_probe_glue.rs` and confirm typing requests no frames beyond those the input itself causes
- [X] T059 [P] Cross-cutting documentation review — links, navigation and index entries across `docs/`, including `docs/README.md`
- [X] T060 Confirm `mise run test` passes on Linux, macOS and Windows via `.github/workflows/ci.yml` (Principle VI)
- [X] T061 Run [quickstart.md](./quickstart.md) end to end and record the full pass, including its date and platform

---

## Dependencies & Execution Order

### Phase dependencies

- **Setup (Phase 1)**: no dependencies
- **Foundational (Phase 2)**: needs Setup; **blocks all stories**
- **US1 (Phase 3)**: needs Foundational
- **US2 (Phase 4)**: needs Foundational. Independent of US1 in the core crate; its two client tasks
  (T041, T046) need US1's picker
- **US3 (Phase 5)**: needs **US1** — the component and its catalogue entry both arrive there
- **Polish (Phase 6)**: needs every story that is being shipped

### Ordering that is not obvious

- **T006 and T007 before the code they govern.** Both gates must be watching first, so T022 produces a
  real Red and T023 is a real Green. Reversed, a gate is written to fit code that already exists —
  the arrangement Principle I exists to prevent.
- **T027–T030 cannot be deferred to User Story 3.** `showcase_completeness.rs` fails in both
  directions, so the moment T024 introduces a `pub struct Typeahead` under `src/ui/material/`, the
  suite is red until the catalogue names it ([R18](./research.md#r18)). Scheduling the entry two
  stories later would have made the MVP unshippable.
- **T018 is not optional.** Feature 016's tests assert the old refusal path. They are rewritten to
  assert the new invariant, because the guarantee they protect still holds — one layer earlier.
- **T019 → T020 → T021 → T042 → T043 → T044** all edit `crates/micold-core/src/typeahead.rs` and are
  strictly sequential, however parallel their tests are.
- **T031 before T032.** The picker cannot be rewritten onto messages that do not exist yet.
- **T051 before T052.** The compiler is what finds the `Copy` breakage; make it break first.

### Within each story

Tests written and observed failing → core logic → component → catalogue → reducer → view → docs →
recorded manual pass. No story is done until its tests pass, its docs exist, and its quickstart
section is recorded.

---

## What is left

Nothing. All 66 tasks are complete.

**How the manual passes (T034, T048, T056, T061) were closed.** Signed off by the maintainer on
2026-08-05 rather than transcribed from an observed walkthrough — recorded that way deliberately, so
the record says what actually happened. The automated half (§A) ran green on Linux, macOS and Windows
(CI run 30984445064); §B's steps were verified as *specifications* against the demo branch list — every
query in the script produces the result the script claims — but the rendered output they describe was
signed off rather than watched by the author of this record.
- **T060 is done.** CI ran `mise run test` green on `ubuntu-latest`, `macos-latest` and
  `windows-latest` (run 30984445064), alongside `fmt + clippy` and the docs check. Nothing in this
  feature is platform-conditional and nothing needed to be.

---

## Phase 8: Material conformance

Raised in review: *is the type-ahead a valid Material Design 3 component?* Strictly it cannot be —
MD3 defines no type-ahead. What it defines is **a text field with an attached menu**, and the audit
found the assembly was reaching for raw widgets where the library already had the Material component.

- [X] T067 Extend `material::TextField` with Material's leading-icon and trailing-action slots, and make it always resolve to one shape so a caller that starts offering a trailing action cannot destroy the input's focus, in `crates/micold-client/src/ui/material/text_field.rs` (contract C4.1)
- [X] T068 Put `style::menu_row`'s state layers on `tokens::state` — it hardcoded `0.12` for pressed, which is the *selected* opacity, so a pressed row and a selected one rendered identically — in `crates/micold-client/src/ui/material/style.rs` (contract C4.2)
- [X] T069 Compose every part of the component from its library counterpart — `TextField`, `menu_panel`, `Scrollable`, `Ripple`, `Glyph`, `Text` — instead of raw widgets styled in place, in `crates/micold-client/src/ui/material/typeahead.rs` (contract C4.1a)
- [X] T070 Give rows Material's menu-item height from `density::MENU_ITEM_BASE`, and ripple only the ones that can be pressed, in `crates/micold-client/src/ui/material/typeahead.rs` (contract C4.1a, C4.1b)
- [X] T071 Accept the layout snapshot: the extra `Row` per text field adds a node without moving anything, verified by diffing the geometry columns — no recorded box was removed or displaced

---

## Deviations recorded during implementation

Four, each with the reason it was better than what was planned:

- **T011 landed in `crates/micold-client/tests/typeahead_fit_shaping.rs`**, not in
  `micold-core`'s `typeahead_fit.rs` as written. Real shaping needs the rendering stack, and
  `micold-core` is iced-free by construction — putting the test where it was planned would have
  meant giving the core crate an iced dependency to test truncation. The core file carries a
  comment pointing at where the shaping half lives.
- **T006/T023 use a `CDK_OVERLAY_IMPLEMENTORS` list**, not the existing `SANCTIONED` one. Widening
  the gate exposed that it detected overlays by the literal text `fn overlay(`, which misses the
  real signature (`fn overlay<'b>(`) and cannot tell *constructing* a floating surface from merely
  *forwarding* a child's — four existing modules only forward. The gate now keys on
  `overlay::Element::new(`, which is the actual act, and holds both directions with a test of its
  own.
- **`Typeahead::new` takes `rows: Vec<Row>`**, not `&[Row]` as the contract's §1 table first said.
  Rows are derived per frame from `branch_matches`, so there is nowhere for a borrowed slice to
  live between frames; `TreeView::new(Vec<TreeItem>)` already set this idiom. The contract table is
  updated to match.
- **Tab dismisses the list, and is the one key the list does not capture.** FR-001b's "closes on
  blur" had no implementation: the rendering stack's text input publishes nothing when it loses
  focus, so the list outlived the focus that opened it and went on claiming Enter from whatever was
  tabbed to — the next Enter would have picked a branch instead of pressing Create. Found by review;
  the rule is in `micold-core` with the rest of the keyboard rule, and the contract's C3.2 records
  what "focus" and "blur" actually resolve to.
- **Emphasis is drawn with a custom `EmphasisedLabel` widget, not `rich_text`/`span`.** The plan's
  Technical Context and T024 both name `rich_text`; it cannot be used, because truncation has to
  happen at layout time — that is when the renderer can shape text and the available width is known
  — and `fit_around` decides which characters survive before any span exists. `ellipsized.rs` is a
  widget for the same reason. The widget binds its renderer's font to `iced::Font` so emphasis can
  name a weight (T065).
- **`Icon::Search` was added to the icon vocabulary** for T024's leading search affordance —
  a magnifier, distinct from `Icon::Filter`'s funnel. Not anticipated by the plan, which assumed
  the affordance could come from the existing set.

---

## Parallel Opportunities

### Phase 2

```text
T004 (core tests)  ‖  T006 (overlay gate)  ‖  T007 (generic-component gate)
```

### User Story 1 — eleven test tasks, all independent files or regions

```text
T008  typeahead_match.rs      (literal + exclusivity)
T009  typeahead_rank.rs
T010  typeahead_fit.rs        (stand-in)
T011  typeahead_fit.rs        (real shaping)
T012  typeahead_keys.rs       (the keyboard rule)
T013  typeahead_budget.rs
T014  branch_search_state.rs  (query/highlight/selection)
T015  branch_search_state.rs  (focus/dismissal)
T016  branch_search_state.rs  (blocked candidates)
T017  branch_search_state.rs  (the FR-013 happy path)
```

Implementation then serializes through `typeahead.rs` (T019 → T020 → T021), with T028 (samples) and
T033 (documentation) running alongside.

### User Story 2

```text
T035  T036  T037   typeahead_match.rs — write together, they share no case
T038  T039         typeahead_corpus.rs
T040               typeahead_budget.rs
T041               branch_search_state.rs
```

### User Story 3

```text
T049  showcase_state.rs   ‖   T050  caption-rule confirmation   ‖   T055  component-library.md
```

---

## Implementation Strategy

### MVP — User Story 1 only

1. Phase 1 Setup → Phase 2 Foundational → Phase 3 US1.
2. **Stop and validate**: quickstart §B1, §B3–§B5a, §B6–§B7.
3. This is a complete feature: substring search with emphasis over a long branch list, its component
   catalogued, the whole suite green. Ship it.

### Incremental delivery

1. Setup + Foundational → the matching vocabulary and two watching gates.
2. **US1** → the picker is a type-ahead, and the component is in the gallery. Demo.
3. **US2** → near misses surface. Entirely additive inside `micold-core` plus one message.
4. **US3** → the gallery entry comes alive. Touches no application code.

### Riskiest task first

T022 — the hand-written `Widget::overlay()` — is the least-charted work in the feature. It sits early
in US1 deliberately, so an unpleasant surprise arrives before anything is built on top of it. The four
existing hand-written `Widget` impls in `src/ui/material/` are the patterns to read first. It is also
now the *thinnest* it can be: with `intent_for` holding the keyboard rule, the widget translates and
applies rather than decides.

---

## Notes

- `[P]` means a different file and no dependency on an incomplete task.
- Commit after each task or logical group; the gates run on every `mise run test`.
- Do not raise a gate budget to make a task pass. Every gate in T057 is currently at zero or at a
  closed list, and this feature ends stricter than it started — two gates wider, in fact.
- Do not tune the SC-003 corpus to make T045 pass. The corpus is the specification of ranking quality;
  changing it is a visible diff to a named file and needs the same argument any other spec change does.

---

## Phase 7: Convergence

Appended by `/speckit-converge`. Each item is remaining work found by assessing the code against
`spec.md`, `plan.md` and the contracts — not a change to any of them.

- [X] T062 Open the result list when the search field takes focus by any route, not only a left press inside its bounds — a developer who reaches the field with Tab currently sees nothing until they type, which is precisely what "before anything is typed" rules out — in `crates/micold-client/src/ui/cdk/typeahead.rs`, per FR-001b and US1/AC1 (partial)
- [X] T063 Reconcile the 5-character single-edit floor with the spec: a 3- or 4-character substitution typo now surfaces nothing (`lagi` finds no `feat/login`), while SC-004 promises single-typo tolerance with no length qualifier and FR-006a puts the approximate floor at 3. Either narrow the claim in `spec.md` or widen the tier in `crates/micold-core/src/typeahead.rs`; the reasoning behind the floor is in [contracts/match-ranking.md §2.2](./contracts/match-ranking.md), per SC-004 and FR-006a (contradicts)
- [X] T064 Add this feature's emphasis pairings to the AA-contrast gate — `primary` on `surface`, `primary` on `secondary_container` (emphasis on the selected row's tonal fill), and `on_surface_variant` on `surface` (a disabled row) — in `crates/micold-core/tests/tokens.rs`, so "legible in both appearances" is measured rather than assumed, per FR-011 (missing)
- [X] T065 Settle emphasis weight: `EmphasisedLabel` shapes every segment with the renderer's default font, so emphasis is colour alone. FR-011c permits "colour role and/or type weight", but [contracts/typeahead-component.md §4.3](./contracts/typeahead-component.md) says "plus type weight" — either add the weight step in `crates/micold-client/src/ui/material/typeahead.rs` or amend C4.3 to what shipped, per plan: contract C4.3 (partial)
- [X] T066 [P] Record the `rich_text` → custom-widget deviation in the Deviations section of this file: the plan's Technical Context and T024 both name `rich_text`/`span` as the emphasis mechanism, and a custom `EmphasisedLabel` was built instead because truncation has to happen at layout time. The reason already sits at `crates/micold-client/src/ui/material/typeahead.rs`; the other five deviations are recorded here and this one is not, per plan: Technical Context (partial)

---

## Phase 9: Bugfix — BUG-001

Appended by `/speckit-bugfix-patch`, after the convergence pass above. The gallery's Typeahead entry
holds its result list permanently open, so the one page that exists to teach the component shows a
state the branch selector is never in at rest and never shows FR-001b's open-on-reach rule. See
`bugs/BUG-001.md`.

**One false completion.** T053 is reopened above. It made the entry's *rows* live and left T029's
`.open(true)` pose in place; T074 retires the pose and closes it again. T029 itself is untouched —
posing the list open was correct for a static entry, which is what T029 built.

**Tests first (Constitution Principle I).** The open/close rule is a decision about state, so it
lands in `showcase/state.rs` under `tests/showcase_state.rs`, exactly as the query, highlight and
pick rules already do — not in the render glue, which `tests/showcase_glue.rs` holds to deciding
nothing.

- [X] T072 [US3] Failing reducer tests first, in `crates/micold-client/tests/showcase_state.rs`: a fresh showcase has its type-ahead list **closed** (extend `a_fresh_showcase_is_at_rest`, which is the existing home for "nothing is mid-transition on launch"); reaching the field opens it; typing opens it; picking a row closes it; dismissing closes it. Confirm each fails before T073 (FR-020a, SC-007a, Principle I)
- [X] T073 [US3] Add `typeahead_open: bool` (false at rest), a `typeahead_open()` accessor, and the `TypeaheadFocused` / `TypeaheadDismissed` message variants in `crates/micold-client/src/showcase/state.rs`, making T072 pass. The rule MUST mirror `app.rs`'s branch-picker arms rather than invent a second one — `AddWorktreeBranchFocused` → open, `AddWorktreeBranchQueryChanged` → open, `AddWorktreeBranchSelected` → close, `AddWorktreeBranchDismissed` → close — because a gallery whose open rule differed from the application's would be the same class of defect this bug is (FR-020a)
- [X] T074 [US3] Retire T029's pose in `crates/micold-client/src/showcase/sections/controls.rs`: `.open(…)` from the reducer, `.on_focus(Message::TypeaheadFocused)`, `.on_dismiss(Message::TypeaheadDismissed)`, and pick closing through the existing `TypeaheadPicked`. Replace the "Always open, because the list is the half worth looking at" comment with what the entry now shows and why — the comment is the record of the decision being reversed. Closes the reopened T053 (FR-020a)
- [X] T075 [P] [US3] Add the open/close line to the entry's `live` captions in `crates/micold-client/src/showcase/catalogue.rs`, so the page describes the behaviour it now has rather than leaving its most conspicuous one unnamed — the gap that let `tests/showcase_captions.rs` stay green through this defect (FR-020a)
- [ ] T076 [US3] Update [quickstart.md](./quickstart.md) §B8 to have the reviewer confirm the list is **closed** on launch, opens on a press in the field, closes on a pick and closes on a press outside — then run it and record the pass (FR-020a, SC-007a)

  > **§B8 written; the visual pass is not recorded.** The four steps are in the quickstart and the
  > binary was launched and renders with the entry rewired, but confirming what is on screen needs a
  > human at the display, so the record says so rather than claiming a pass nobody watched. The rule
  > itself is not waiting on this: it is driven by five tests in `tests/showcase_state.rs`, and only
  > the glue that applies the answer is what §B8 checks.

**Bugfix**: 2026-08-07 — BUG-001 Updated from bugfix patch: reopened T053, added T072–T076. T053's
reopen is kept visible rather than erased; its matching half stays done and only its pose is retired.
