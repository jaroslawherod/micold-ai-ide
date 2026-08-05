# Phase 0 Research: Branch Selector Type-Ahead Search

**Feature**: [spec.md](./spec.md) | **Plan**: [plan.md](./plan.md) | **Date**: 2026-08-04

Every unknown the plan's Technical Context raised, resolved. The through-line: the spec asks for a
field whose result rows carry **per-character emphasis** and whose list **floats without reflowing a
content-sized dialog**. Each existing mechanism in the rendering stack gives one of those and not the
other, which is what forces R5's decision.

---

## R1 — Why the current `Select` cannot carry this

**Decision**: the existing-branch picker stops using `material::Select`; `Select` itself is untouched
and keeps its other call site (the Conventional-Commits type list).

**Rationale**: `Select` wraps `pick_list`, whose dropdown is `iced_widget::overlay::menu::Menu`. That
menu renders each option as a **single `Text`** built from `T: Display` — `overlay/menu.rs` constructs
one `core::text::Text` per row and fills it in one colour. There is no seam at which part of a row
could be emphasised, so FR-009/FR-010 are not reachable through it at any cost short of forking the
widget. `BranchCandidate`'s `Display` impl is literally the rendered row today
(`crates/micold-core/src/worktree.rs:282`), which is why feature 016 needed no widget change and why
this feature needs one.

Secondary: `pick_list` also has no per-item disabling — feature 016 already recorded this (research
R8, `app.rs:207`) and pushed the refusal of a blocked branch to the point of action. That constraint
disappears with the new component, but FR-012/FR-013 require the *behaviour* to stay as it is, so the
plan deliberately keeps refusing at submit rather than quietly changing it.

**Alternatives considered**: teach `Select` a highlight — rejected, the highlight would have to pass
through `pick_list` into a menu type this crate does not own.

---

## R2 — Why not iced's `combo_box`

**Decision**: not used.

**Rationale**: `combo_box` is the rendering stack's own type-ahead — a `text_input` plus a filtered
overlay with keyboard navigation — and it would satisfy FR-001, FR-005 and FR-017 nearly for free. It
fails on the two requirements that define this feature:

1. Its overlay is the **same** `overlay::menu::Menu` as R1's, so no highlighting.
2. Its filtering is `Display`-based substring matching with no ranking and no approximate matching, so
   FR-006 through FR-008 would need a second filter layered on anyway.

Worth stating plainly because `combo_box` is already named in
`tests/one_overlay_implementation.rs`'s `WIDGET_ATTACHED` watch list: it is a known, deliberately
unsanctioned option, not an oversight.

---

## R3 — Why not `Float`

**Decision**: not used.

**Rationale**: iced 0.14's `Float` widget "makes its contents float over other widgets", but its
`layout()` delegates straight to its content
(`iced_widget-0.14.0/src/float.rs`) — the content keeps its layout space and only its *drawing* is
lifted. A result list wrapped in `Float` would still reserve its own height in the dialog, which is
the "inline list, fixed reserved height" shape the clarification session explicitly rejected.

---

## R4 — Why not a window-level `cdk::overlay::Surface`

**Decision**: not used for this surface.

**Rationale**: `cdk::overlay::Anchor` offers `Point`, `TopEnd` and `Center`. Anchoring the result list
under the field needs the field's **window coordinates**, and nothing in the composition knows them:
the dialog is `Anchor::Center` and content-sized, so the field's position exists only after layout.
iced 0.14's `Sensor` reports its content's `Size` on show and resize — not its position — so it cannot
supply them either. This is the same wall `material/select.rs`'s module doc records the hand-rolled
`SelectTrigger`/`SelectOverlay` hitting, and `tests/one_overlay_implementation.rs` already sanctions
`pick_list` for exactly this reason:

> the dropdown must anchor to its trigger inside a content-sized dialog, where a window-level surface
> has nothing to anchor against

Feeding a round-trip position through app state would also put a message on every layout pass, which
is the shape `tests/idle_requests_no_frames.rs` exists to keep out.

---

## R5 — Decision: a hand-written widget that implements `Widget::overlay()`

**Decision**: build the type-ahead as a custom widget whose `overlay()` returns the result list, so the
rendering stack positions it from the field's own on-screen bounds. Strike it onto
`tests/one_overlay_implementation.rs`'s `SANCTIONED` list as the **third** widget-attached delegation,
with its reason recorded there.

**Rationale**: this is the only mechanism that satisfies both halves at once — R1/R2 rule out the
stack's ready-made menus on highlighting, R3/R4 rule out the in-house floating primitives on
anchoring. It is also not novel here: `material/ellipsized.rs`, `material/resize_handle.rs`,
`material/terminal_pane.rs`, `material/navigation_drawer.rs` and four widgets in
`material/animation.rs` are already hand-written `Widget` impls. What is new is only the `overlay()`
half.

**Consequence the plan must carry**: the closed-list gate currently detects *calls* to three named
widgets (`pick_list`, `combo_box`, `tooltip`). A hand-written `fn overlay(` is invisible to it, so a
future fourth delegation could land unargued. The gate is widened to also treat an `fn overlay(`
implementation under `src/ui/` as a delegation requiring an entry — otherwise this feature would
quietly remove the very guarantee it is being held to.

**Alternatives considered**: fork `overlay::menu::Menu` into the library — rejected, it drags in
scrollable-menu internals this feature does not need and creates a second list implementation to keep
in step. Vendor `combo_box` — same objection, larger.

---

## R6 — Which layer each half lives in

**Decision**: split across the two library layers, following feature 017's rule.

| Layer | File | Owns |
|---|---|---|
| Behaviour | `src/ui/cdk/typeahead.rs` | the widget: where the list sits, that it captures keys, when it closes, which row is highlighted |
| Appearance | `src/ui/material/typeahead.rs` | the builder API, the Material field treatment, the menu surface, row metrics, the emphasis treatment |

**Rationale**: `tests/cdk_no_appearance.rs` scans `src/ui/cdk/` and fails on any colour-role or style
name appearing there, so the behaviour half physically cannot resolve its own colours — the material
half hands it already-resolved values, exactly as `cdk::overlay` receives a pre-built scrim. This also
keeps the anchoring/keyboard mechanics reusable by a future picker that wants a different skin
(SC-007).

**Alternatives considered**: one file under `material/` — rejected, it would put positioning and
dismissal in the appearance layer and reopen the split feature 017 closed.

---

## R7 — How a partially-emphasised branch name is drawn

**Decision**: `iced::widget::rich_text` with one `span` per run, emphasised by **colour role plus type
weight** — not by a filled background.

**Rationale**: iced 0.14's `text::Span` carries `color`, `font` (hence weight) and `highlight` (a
background quad). All three are available; the choice is a design one. FR-011c requires the highlight
to distinguish "without obscuring", and a branch name is a path-like string already dense with `/`,
`-` and `_`. Background chips scattered through `feat/JIRA-412_retry-v2` fragment it visually, and
they collide with the row's own hover/selected/keyboard-highlighted background, which is where
Material already spends that channel. Colour + weight leaves the row's state channel free.

**Alternatives considered**: `Span::highlight` background — rejected above. A `row!` of separate
`Text` widgets — rejected: it defeats shaping across the run boundary and makes truncation (R8)
impossible to measure as one string.

---

## R8 — Truncation that follows the match (FR-011d)

**Decision**: a pure `fit_around(content, keep, available, measure) -> (String, Vec<Range>)` — the
longest window of the name that fits and still contains `keep`, plus the highlight ranges rebased onto
it. Ellipsis at whichever end was cut, or both.

**Rationale**: `material/ellipsized.rs` already solves the neighbouring problem — binary-search the
longest prefix that fits, with the measuring function injected so the search is testable against a
monospace stand-in and then re-proved against real shaping (`ellipsized.rs:239`, `:298`). This is the
same search with a different constraint, so it is the same shape of code and the same shape of test.
Keeping it a free function over an injected `measure` is what lets FR-021 hold: the rule is exercisable
without a renderer.

**Alternatives considered**: extend `Ellipsized` itself — rejected, its job is a plain single-colour
label and it has call sites that must not move; this needs rich spans. The two share the technique,
not the type.

---

## R9 — Where matching lives

**Decision**: a new render-free module `micold-core/src/typeahead.rs`, generic over `&str` haystacks.
The client's reducer recomputes matches on each keystroke and stores them on `WorktreeForm`.

**Rationale**: FR-019 forbids the component knowing about branches; FR-021 requires the rule to be
exercisable without rendering; Principle I requires it to be tested first. `micold-core` is the
iced-free crate and is where `naming`, `overlay` and `worktree` already keep exactly this kind of
decision logic. Recomputing in the reducer — rather than in the view — keeps `src/ui/` inside
Principle I's glue exception, which covers only code with *no* decision logic of its own.

**Alternatives considered**: match lazily in the view — rejected, that is decision logic in the
rendering layer and it would recompute per frame rather than per keystroke.

---

## R10 — The matching and ranking algorithm

**Decision**: three tiers, tried in order, first hit wins; results stable-sorted by tier then by match
position, leaving `branch_candidates`' order as the tie-break (FR-007).

| Tier | Test | Emphasis (FR-010) |
|---|---|---|
| Literal | case-folded `find` of the query in the name | the matched run |
| Single edit | some window of the name of length `q-1`, `q` or `q+1` is within Levenshtein distance 1 | the whole window, as one run |
| Subsequence | query characters in order, not necessarily adjacent, greedy leftmost | each corresponding character |

**Corrected during implementation**: the approximate tiers were originally the other way round. A
test caught why they cannot be — a dropped-letter typo is *also* a subsequence, so subsequence-first
claimed every deletion typo and emphasised it as scattered characters, which is the broken word the
clarification session rejected. Single-edit is both tried first and ranked higher: it is the closer
reading of what was typed.

Approximate tiers are skipped entirely below 3 query characters (FR-006a).

**Rationale**: the three tiers *are* the spec's own words (FR-003, FR-006) turned into predicates, and
each yields exactly the highlight shape FR-010 demands for it — which is why the tier is carried on the
match rather than recomputed for display. Greedy-leftmost subsequence is what makes the highlight
deterministic; without a fixed rule, two equally valid alignments would highlight differently between
runs and SC-005 would be unverifiable.

**Alternatives considered**: score-based fuzzy ranking (bonus for word starts, camel humps, contiguity)
— rejected as out of scope: the clarification session deliberately narrowed "close to it" to these two
shapes, and a score is exactly the kind of result "a developer cannot explain" that SC-005 rules out.

---

## R11 — Meeting the 16 ms budget

**Decision**: recompute over the whole candidate list on every keystroke, with no cache and **no
debounce**. Hold the budget with a direct test in `micold-core` over 500 synthetic branch names.

**Rationale**: per candidate the work is bounded — case-folded substring scan O(n), subsequence O(n),
and single-edit over windows O(n·m) with `m` the query length and a hard cap on `m` before the tier is
even attempted. At 500 names of realistic length this is well inside one frame, and the test says so
rather than the plan asserting it. Debouncing is not available as an escape hatch: FR-005 requires the
displayed results to correspond to the complete text currently in the field, and a debounce is
precisely a window in which they do not.

The frame-level half of SC-002 uses the probe and reference scene the repository already carries
(`micold-core/src/frame_probe.rs`, `crates/micold-client/tests/frame_probe_glue.rs`) rather than new
instrumentation.

---

## R12 — No new dependency

**Decision**: no fuzzy-matching crate (`fuzzy-matcher`, `nucleo`, `sublime_fuzzy`, `strsim`).

**Rationale**: three reasons, in order of weight. (1) FR-010 needs highlight positions *keyed to the
tier that matched*, which none of these expose — they return a score and, at best, a flat index list.
(2) FR-006's semantics are narrower and more precisely specified than any crate's default, so the
result would be a crate plus a wrapper enforcing our own rules on top. (3) The whole module is on the
order of a hundred lines with the tiers written out, and it is the part of this feature most worth
having tests read against.

---

## R13 — What the field shows, and where the selection is visible

**Decision**: the field holds the **search text only** — never the selected branch's name. The
selection stays visible in the form's existing derived preview row ("Branch: feat/login").

**Rationale**: FR-014 requires a made selection to survive the search text changing or being cleared,
and the clarified edge case requires it to survive being narrowed out of the visible list. A field that
doubles as the selection display cannot express that state — clearing it would either destroy the
selection or lie about the query. The preview row already renders the selected branch under
`BranchSource::Existing` (`app.rs:182`, `worktree_form.rs:385`), so the selection has a home with no
new surface invented for it.

**Consequence**: the Material pattern realised here is a *search* field with an attached result menu,
not an editable exposed dropdown showing its own value.

**Amended after the post-plan clarification pass.** Keeping the selection out of the field left a gap
this decision originally missed: `material/select.rs` deliberately seeds its open menu's highlighted
row from the current value, so reopening the list marks what is already chosen (feature 013, FR-003).
Dropping that would have been a silent regression, so the component takes a `selected` index beside
its rows and marks that row — distinctly from the keyboard highlight, since both can sit on the same
row at once (FR-014b, contract §4.7).

---

## R13a — Blocked branches: the refusal moves to the point of choice

**Decision**: a blocked branch is listed, visibly disabled, and cannot be picked (FR-012a). Added by
the post-plan clarification pass; it supersedes this plan's original intention to leave feature 016's
behaviour untouched.

**Rationale**: feature 016 made a blocked branch selectable and refused it only at Create, and its own
research says why — `pick_list` has no per-item disabling, and forking a list widget is what the
component-reuse gate rejects (`app.rs:207`). That was a constraint, not a preference, and this feature
removes the constraint. Leaving the workaround in place would also have left the new component's
`enabled` flag decorative for its only consumer, which is the sort of unused seam that rots.

**What this costs**: `can_submit()`'s blocked-branch guard becomes unreachable through the picker. It
is kept, because it is the invariant's last line of defence and costs one comparison; its test moves
from "the form refuses a selected blocked branch" to "a blocked branch cannot become the selection",
with the guard keeping a direct unit test of its own (contract §5).

---

## R14 — Where each empty-state message goes

**Decision**: the two repository-level messages stay exactly where they are, inline under the "Branch"
label; the new no-match message renders **inside the menu surface**.

**Rationale**: FR-015 requires all three to stay distinct. They answer different questions — "this
repository has nothing to offer" and "everything is checked out elsewhere" are facts about the
repository and are true before anything is typed, whereas "nothing matches" is a fact about the query.
Putting the query-scoped one in the list it describes keeps them from reading as variants of each
other, and means the repository-level messages are still the first thing seen on opening the picker.

---

## R15 — Keyboard, and who owns the highlighted row

**Decision**: the highlighted index lives in the **reducer** (`WorktreeForm`), not in widget-tree state.
The widget emits movement messages; the material layer renders the row the reducer names. **Amended by
the analysis pass**: the *rule* deciding which key means what also leaves the widget — it goes to
`micold_core::typeahead::intent_for` (contract §4b).

**Rationale**: `pick_list` keeps its hovered index in widget state, which is fine for a widget nobody
tests. Here the highlight interacts with filtering — FR-005 changes the result set under the highlight
on every keystroke, and what happens to it then is decision logic. Decision logic in widget state is
untestable from `tests/`, which Principle I does not permit and
`tests/logical_state_ownership.rs` is there to catch. In the reducer it is an ordinary state
transition with an ordinary test.

Event order works in our favour: iced delivers events to overlays before the widget tree, so the
overlay consumes Up/Down/Enter/Escape and lets everything else fall through to the field.

**Why the rule moved too.** The original plan left the key→effect mapping inside the widget, on the
reading that a `match` on key codes is glue. It is not: "Down saturates rather than wrapping" and
"Enter on a disabled row does nothing" are business rules, and Principle I's exception explicitly
covers only code "with no decision logic, branching, or business rule of its own", requiring anything
else to "land in tested pure/core logic first". The codebase already had the answer —
`micold-client/src/keymap.rs` is pure key-encoding for the terminal, kept "decoupled from iced so it
is unit-testable (Constitution Principle I)", with the widget translating events in and applying the
result. `intent_for` is the same arrangement.

---

## R18 — The gallery entry cannot be deferred

**Decision**: the `catalogue::Entry` for the material component and the `EXEMPTION` for the behaviour
half both land in **User Story 1**, alongside the component itself — not in User Story 3.

**Rationale**: `tests/showcase_completeness.rs` "fails in **both** directions: a component with no
entry, and an entry naming a component that no longer exists". A `pub struct Typeahead` under
`src/ui/material/` is a component by the shared inventory's definition the moment it converts into an
element, so the suite goes red on the commit that introduces it and stays red until the catalogue
names it. Deferring the entry to a later story would mean a story whose own checkpoint claims a green
suite cannot have one — and would leave the MVP unshippable, which is the opposite of what slicing by
story is for.

The behaviour half takes an exemption rather than an entry, with the reason `cdk/overlay.rs`'s
`Overlay` and `Surface` already use: it is a behaviour-layer wrapper with no appearance, exercised by
the page rather than posed on it.

**What User Story 3 keeps**: the parts that are genuinely about the gallery rather than about the
component existing — the live, typeable example (and the `Copy` removal it forces), posing in both
schemes, and the component-library documentation a future picker author reads.

---

## R16 — The showcase entry costs the showcase's `Copy`

**Decision**: `showcase::state::Message` drops `Copy` (keeping `Clone`), because the gallery's live
type-ahead must carry the typed `String`.

**Rationale**: FR-020 requires a *typeable* example, and `showcase::state::Message` is currently
`#[derive(Debug, Clone, Copy, PartialEq, Eq)]` — every variant is a toggle or an index. A query
message carries a `String`, which is not `Copy`. The alternative — swallowing keystrokes as `NoOp` —
produces a dead example, which is the thing FR-020 exists to prevent. The change is mechanical and
contained: `state.rs`, its tests, and any `Copy`-dependent call site in `gallery.rs`.

Flagged here rather than discovered during implementation because it touches a file the showcase's own
gates read.

---

## R17 — Documentation surfaces

**Decision**: `docs/user-guide/worktrees-and-sessions.md` gains a branch-search section (FR-022);
`docs/development/component-library.md` gains the component alongside the others.

**Rationale**: Principle VII requires user-facing behaviour to ship with its user-guide page in the
same change, and the branch picker is already documented in that page — search belongs beside it
rather than in a page of its own. The development page is where a future picker author looks before
building a second type-ahead, which is what SC-007 is asking for.
