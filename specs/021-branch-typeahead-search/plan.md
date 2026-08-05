# Implementation Plan: Branch Selector Type-Ahead Search

**Branch**: `feat/make-branch-selector-a-type-a-head-search` | **Date**: 2026-08-04 | **Spec**: [spec.md](./spec.md)

**Input**: Feature specification from `/specs/021-branch-typeahead-search/spec.md`

## Summary

Replace the existing-branch picker's `Select` with a type-ahead: a Material search field whose result
list floats beneath it, narrowing as the developer types, with the matched text emphasised inside each
row. Package it as a shared component so the next picker that needs this consumes it rather than
rebuilding it, and pose it in the component gallery.

The technical shape follows from one collision. The spec asks for **per-character emphasis inside
result rows** and for a list that **floats without reflowing a content-sized dialog**. Every ready-made
mechanism in the stack gives one and not the other: `pick_list` and `combo_box` anchor correctly but
draw each row as one flat `Text` ([R1](./research.md#r1), [R2](./research.md#r2)); `Float` keeps its
content's layout space ([R3](./research.md#r3)); a window-level `cdk::overlay::Surface` has no way to
learn the field's position inside a centred, content-sized dialog ([R4](./research.md#r4)) — the same
wall `material/select.rs` records the original hand-rolled dropdown hitting. So the component is a
**hand-written widget that implements `Widget::overlay()`** ([R5](./research.md#r5)), which the library
already does four times over for other reasons, split across the two library layers the way feature 017
requires ([R6](./research.md#r6)).

Three things follow that are worth naming up front:

1. **Neither the matching nor the keyboard rule is in the component.** Both are render-free modules in
   `micold-core` over plain strings and a platform-neutral `Key` enum, with the reducer recomputing on
   each keystroke — so the rules are tested directly, the component stays ignorant of branches, and
   `src/ui/` stays inside Principle I's glue exception ([R9](./research.md#r9),
   [R15](./research.md#r15)). "Down saturates rather than wrapping" is a business rule, not glue;
   `keymap.rs` already sets the precedent for keeping such a rule out of the widget.
2. **The overlay closed list gets widened, not just extended.** `tests/one_overlay_implementation.rs`
   detects *calls* to three named widgets; a hand-written `fn overlay(` is invisible to it. Adding our
   entry without teaching the gate to see implementations would leave the guarantee weaker than we
   found it ([R5](./research.md#r5)).
3. **The showcase's `Message` loses `Copy`.** A typeable example carries a `String`, and every variant
   today is a toggle or an index ([R16](./research.md#r16)). Mechanical, but it touches files the
   showcase's own gates read, so it is planned rather than discovered.
4. **The gallery entry ships with the component, in User Story 1.** `showcase_completeness.rs` fails
   in both directions, so a component with no catalogue entry is a red suite from the commit that
   introduces it. The entry is part of introducing a component here, not a later polish step
   ([R18](./research.md#r18)).

Feature 016's picker behaviour is preserved with **one deliberate change**, settled in the post-plan
clarification pass: unavailable branches stay listed with their reasons, but they are no longer
pickable (FR-012a). Feature 016 allowed picking one and refused at Create only because `pick_list`
could not disable a row — a constraint this feature removes
([R13a](./research.md#r13a)). `can_submit()`'s guard is kept as a last line of defence and keeps its
own unit test, but the refusal now happens at the point of choice. `preview()` still reads
`selected_branch`, which no new message writes.

## Technical Context

**Language/Version**: Rust, edition 2021, MSRV 1.97 (workspace-inherited)

**Primary Dependencies**: `iced` 0.14 (`rich_text`/`span` for emphasis, `advanced::Widget` +
`advanced::overlay` for the anchored list) and `micold-core` (`tokens`, `theme`) — both already
dependencies of `micold-client`. **No new dependency**; specifically no fuzzy-matching crate
([R12](./research.md#r12)).

**Storage**: none. Nothing new is persisted; the search text dies with the form
([data-model §Lifetime](./data-model.md#lifetime-and-persistence)).

**Testing**: `cargo test` via `mise run test`; the matching module's own tests under
`crates/micold-core/tests/` (no GUI, `mise run test-core`); reducer tests and the architecture gates
under `crates/micold-client/tests/`; the recorded manual pass in [quickstart.md](./quickstart.md) §B for
render glue only.

**Target Platform**: Linux, macOS, Windows desktop. Nothing here is platform-conditional — matching is
string arithmetic and the widget is stack-portable.

**Performance Goals**: matching and ranking 500 branch names within **16 ms**, one frame at 60 fps
(SC-002), held by a test rather than by inspection; no frames requested at rest, unchanged.

**Constraints**: the form's other fields must not move when the result count changes (FR-001a); no
debounce is available as an escape hatch, because FR-005 requires the visible results to correspond to
the complete current text ([R11](./research.md#r11)); no new colour, type size or spacing value
(FR-011b); feature 016's picker semantics unchanged (FR-012, FR-013).

**Scale/Scope**: one new render-free module (~150 lines plus tests), one component in two halves, one
feature-module rewrite (`branch_picker`), four new reducer messages, one gallery entry, one widened
gate, two documentation pages.

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

- [x] **I. Test-First (NON-NEGOTIABLE)**: PASS. Every rule with a decision in it lands in tested
  render-free code first — matching, ranking, truncation **and the keyboard rule** in `micold-core`
  ([contracts/match-ranking.md](./contracts/match-ranking.md) §2–§4b), state transitions in the reducer
  ([data-model §2](./data-model.md#2-form-state--micold_clientappworktreeform-extended)). The GUI
  exception is claimed only for `src/ui/`'s render glue, whose manual procedure is
  [quickstart.md](./quickstart.md) §B. Deliberately **not** claimed for the widget's keyboard and
  highlight behaviour: the highlighted index lives in the reducer, and the key→intent rule lives in
  `micold-core`, leaving the widget with translation and application only ([R15](./research.md#r15)).
- [x] **II. Multi-Session Support**: PASS. No session state is touched. `WorktreeForm` is per-open-form
  and already transient; the new fields inherit that lifetime.
- [x] **III. Worktree Integration**: PASS. No git operation changes. The candidate list is the one
  `branch_candidates` already produces; this feature only filters what is shown.
- [x] **IV. Local-First Storage (NON-NEGOTIABLE)**: PASS. No file, no directory, no network. The
  existing "remote branches reflect your last fetch" notice stays, because search does not fetch.
- [x] **V. Rust + iced Stack**: PASS. Rust and iced only. Invalid states are narrowed by construction:
  the highlight indexes the match list rather than the candidate list, so it cannot name a filtered-out
  row, and `selected_branch` is written by exactly one message.
- [x] **VI. Cross-Platform Parity**: PASS. No platform branch anywhere. The matching tests and the
  source-scanning gates run on all three platforms in CI as they do today.
- [x] **VII. Documentation First-Class**: PASS. `docs/user-guide/worktrees-and-sessions.md` gains the
  branch-search section and `docs/development/component-library.md` gains the component, both in this
  change ([R17](./research.md#r17)).
- [x] **VIII. Reusable UI Component Foundation**: PASS. The type-ahead is a shared primitive in the
  library, not a picker-local widget — chainable builder terminating in `.into()`
  ([contracts/typeahead-component.md §1](./contracts/typeahead-component.md#1-builder-api)), themed by
  `Roles`, posed in the gallery. `Select` is left intact for its other call site rather than mutated
  into something two callers disagree about.

### Re-check after Phase 1 design

Re-evaluated against the contracts and data model: **all eight still PASS**. The design added two
things worth recording rather than waving through:

- **A third widget-attached overlay delegation.** Not a violation — `tests/one_overlay_implementation.rs`
  exists precisely to make the third one an argued diff rather than an accretion, and
  [R5](./research.md#r5) is that argument. The design strengthens the gate at the same time, so the
  closed list ends the feature stricter than it started.
- **Two files that must not name colours, and one that must not name widgets.** `cdk/typeahead.rs`,
  `material/typeahead.rs` and the rewritten `ui/worktree_form.rs` sit under three different existing
  gates. The split in [R6](./research.md#r6) is what keeps all three satisfiable at once; no gate is
  relaxed and no budget is raised.

No entry in Complexity Tracking.

### Re-check after the `/speckit-analyze` pass

Eleven findings applied; two were Principle I failures this plan had introduced and neither survives:

- **The keyboard rule was untested decision logic in `src/ui/cdk/`** — outside what the GUI exception
  covers. It moves to `micold_core::typeahead::intent_for` (contract §4b), following `keymap.rs`'s
  precedent, and gains its own test file.
- **User Story 1's checkpoint claimed a green suite it could not have** — `showcase_completeness.rs`
  goes red the moment a component exists without a catalogue entry, and the entry was scheduled two
  stories later ([R18](./research.md#r18)).

Also added: a gate holding FR-019 the way every other component rule is held (FR-021a), an explicit
opening trigger for the result list (FR-001b), and coverage for FR-013, SC-001 and SC-006, which had
requirements but no verification. Still **8/8 PASS**, and now honestly so.

## Project Structure

### Documentation (this feature)

```text
specs/021-branch-typeahead-search/
├── plan.md              # This file
├── research.md          # Phase 0 — R1–R17
├── data-model.md        # Phase 1 — matching types, form state, row view model, showcase state
├── quickstart.md        # Phase 1 — §A automated, §B recorded manual pass
├── contracts/
│   ├── match-ranking.md      # the render-free rule: tiers, ranking, truncation, budget
│   └── typeahead-component.md # the component: builder API, behaviour, appearance, gallery entry
├── checklists/
│   └── requirements.md  # spec quality, 16/16
└── tasks.md             # Phase 2 output (/speckit-tasks — NOT created here)
```

### Source code (repository root)

```text
crates/micold-core/
├── src/
│   ├── lib.rs                      # + pub mod typeahead
│   └── typeahead.rs                # NEW — Query, MatchKind, Match, match_one, rank, fit_around
└── tests/
    ├── typeahead_match.rs          # NEW — tiers, offsets, spans
    ├── typeahead_rank.rs           # NEW — order, stability, empty query
    ├── typeahead_fit.rs            # NEW — truncation around a match
    ├── typeahead_budget.rs         # NEW — 500 names inside 16 ms
    ├── typeahead_keys.rs           # NEW — the keyboard rule (FR-017, FR-017a)
    └── typeahead_corpus.rs         # NEW — pinned corpus + query pairs, ≥95% in top five (SC-003, SC-001)

crates/micold-client/
├── src/
│   ├── app.rs                      # WorktreeForm + 4 fields, 4 messages, transitions
│   ├── ui/
│   │   ├── cdk/typeahead.rs        # NEW — the widget: overlay(), key capture, dismissal
│   │   ├── material/typeahead.rs   # NEW — builder, field treatment, menu surface, emphasis
│   │   ├── material/mod.rs         # + re-export
│   │   └── worktree_form.rs        # branch_picker() rewritten onto the component
│   └── showcase/
│       ├── catalogue.rs            # + Entry
│       ├── sections/controls.rs    # + render fn (the input-control section)
│       ├── samples.rs              # + fixed sample rows
│       └── state.rs                # + query/highlight, Message loses Copy
└── tests/
    ├── branch_search_state.rs      # NEW — reducer transitions and invariants
    ├── typeahead_is_generic.rs     # NEW — the component names no branch/worktree/git (FR-021a)
    └── one_overlay_implementation.rs # widened: sees `fn overlay(`, + SANCTIONED entry

docs/
├── user-guide/worktrees-and-sessions.md   # branch search section (FR-022)
└── development/component-library.md       # the new component
```

**Structure Decision**: the existing three-crate workspace, unchanged. The render-free rule goes to
`micold-core` beside `naming`, `overlay` and `worktree`, which is where this codebase already keeps
decision logic. The component goes to `micold-client`'s two library layers — behaviour to `ui/cdk/`,
appearance to `ui/material/` — because `tests/cdk_no_appearance.rs` makes that split structural rather
than stylistic. No new crate, no new binary, no manifest change.

## Delivery order

Sliced by the spec's user-story priorities, each slice independently testable and shippable.

| Slice | Delivers | Gate |
|---|---|---|
| **1 (US1, P1)** | literal matching + ranking, the keyboard rule, the component in both halves, **its gallery entry and cdk exemption**, the picker rewritten, emphasis, match-aware truncation, disabled + selected row treatments, the widened overlay gate, the generic-component gate, user-guide section | `mise run test` green — including `showcase_completeness.rs`; quickstart §B1, §B3–§B5a, §B6–§B7 |
| **2 (US2, P2)** | subsequence and single-edit tiers, the 3-character floor, tier-shaped emphasis, the no-match message, the ranking corpus | quickstart §B2 |
| **3 (US3, P3)** | the gallery entry made **live and typeable**, `Copy` removal, both-scheme posing, component-library docs | `tests/showcase_captions.rs`; quickstart §B8 |

Slice 1 is a complete, useful feature on its own: substring search with emphasis over a long branch
list. It is deliberately the largest slice, because the completeness gate does not let a component
arrive without its catalogue entry ([R18](./research.md#r18)) — that is the price of the gate, and it
is worth paying. Slice 2 is additive inside `micold-core` plus one message for the empty state. Slice
3 touches no application code.

## Risks

| Risk | Handling |
|---|---|
| The hand-written `overlay()` is the least-charted part of the work | It is the *only* mechanism satisfying both halves ([R1](./research.md#r1)–[R4](./research.md#r4)), and four hand-written `Widget` impls already exist in the library to pattern-match against. Slice 1 builds it first, so an unpleasant surprise surfaces before anything else is built on top. |
| Truncation and emphasis interact (FR-011d ↔ FR-010) | Both are pure functions over injected measurement, contracted together in [match-ranking.md §4](./contracts/match-ranking.md#4-truncation) and tested against a monospace stand-in first, then real shaping — the technique `ellipsized.rs` already proves. |
| The 16 ms budget could be missed at 500 branches with the single-edit tier on | The tier is windowed and bounded, and the budget is a test, not an assertion. If it is ever missed the fix is inside `micold-core` with no interface change — the escape hatch is *not* a debounce ([R11](./research.md#r11)). |
| Removing `Copy` from the showcase `Message` ripples | Contained to `showcase/`; the compiler finds every site, and the showcase's gates run in the same suite. |
| FR-012a changes behaviour feature 016 shipped | It is a deliberate, recorded supersession ([R13a](./research.md#r13a)), not a side effect. Feature 016's tests that assert the old refusal path are rewritten rather than deleted, so the invariant they guarded is still guarded — one layer earlier. |
| The SC-003 corpus could be tuned until it passes | The corpus is committed data covering all three tiers, and the assertion is on the rate. Tuning it to pass is a visible diff to a named file, not an invisible threshold change. |

## Complexity Tracking

No constitutional violation requires justification. Left empty deliberately.
