# Feature Specification: Feature Encapsulation — Own Your Messages, Own Your State

**Feature Branch**: `feat/feature-encapsulation`

**Created**: 2026-08-25

**Status**: Draft

**Input**: User description: "Finish the component migration that feature 021 started but left opt-in. Two maintainer pains remain: (1) a single flat 119-variant `Message` enum in `app.rs` plus a long `State::update`, so changing one widget ripples into `app.rs` and `main.rs` routing; (2) no local state — everything a feature touches lives in the root `State` struct and is plumbed down through views and back up as messages. Give every remaining feature module its own message vocabulary and reducer; promote features whose state is nobody else's business to stateful widgets that own it; make both mandatory with guard tests so the pattern cannot be opted out of the way 021's was."

## Context: What This Feature Is

This is an **internal restructuring**. No end user sees anything change — not a pixel, not a
keystroke, not a saved file. The people it serves are the **maintainers**, and the value delivered
is the cost of the *next* change. Following the precedent of feature 021, the user stories below
are written from the maintainer's perspective and the success criteria measure the cost of change
rather than the behavior of the product.

### The finding this feature exists to act on

Feature 021 set out to turn a monolithic MVU core into a "distributed, component-based MVU". It
delivered a great deal — ten feature modules, an overlay registry, a shell split, and a set of
guard tests this feature will extend rather than replace. But on the two axes a maintainer actually
feels, it stopped short, and **it stopped short by design rather than by accident**:

- **FR-004b explicitly permitted a feature to conclude that a reducer module suffices** — that
  nesting a message vocabulary was optional. Nine of ten features concluded exactly that.
- **Local state was never in scope at all.** Not attempted, not deferred, not measured.

The result is that nesting happened **once**. `features/worktree_form.rs` has a `Msg` enum
(line 590) and a `pub fn update(state, msg) -> Vec<Outcome>` (line 657), and the root sees a single
`Message::WorktreeForm(worktree_form::Msg)` arm (`app.rs:253`) where fourteen variants used to be.
It worked. It is the only one.

**And even that one did not get local state.** `features/worktree_form.rs:11` reads
`use crate::app::{Message, State}` — the form's own reducer takes `&mut` the *root* `State`. It won
a private message vocabulary and nothing more. This is why 021's completion table shows the `State`
struct **growing**, 37 fields to 45, while the `Message` enum fell 130 to 120.

The lesson is not that 021 was wrong. It is that **an optional pattern in a codebase under active
feature pressure is a pattern that does not spread.** Every guard 021 *did* make mandatory — the
overlay registry, write isolation, render-freedom — held perfectly and is still holding today.
That asymmetry is the whole argument for this feature's third scope item.

### Measured baseline (`main`, pinned to `b43c11c`, 2026-08-25)

| Concern | Where it lives | Measure |
|---|---|---|
| Root message vocabulary | `crates/micold-client/src/app.rs:42–502` | **119 variants** |
| Root application state | `crates/micold-client/src/app.rs:506–718` | **44 public fields** |
| Root pure reducer | `app.rs:866–1165` (`State::update`) | **300 lines** |
| Shell effectful reducer | `main.rs:520–708` (`update_inner`) | **52 message arms** |
| Feature modules with their own `Msg` | `src/features/` | **1 of 11** (`worktree_form`) |
| Feature modules with their own reducer | `src/features/` | **1 of 11** (`worktree_form`) |
| Feature modules owning their own state | `src/features/` | **0 of 11** |
| Ownership map entries in the write guard | `tests/feature_write_isolation.rs` | **51 state paths** |
| Existing stateful widgets (the target mechanism) | `src/ui/` | **18 `Widget` impls** |

Approximate attribution of the 119 root variants, to show where the weight sits: session and
terminal ~35, project ~20, worktree ~18, sidebar ~10, connection ~10, window ~5, settings ~7,
help ~3, notifications ~2, cross-cutting and diagnostics ~8, and `WorktreeForm(..)` ~1. Exact
attribution is planning work, not specification work; what matters here is that no feature is small
enough for its share to be noise, and one feature is already down to a single variant.

### What already exists (scope reducers, not scope)

This feature inherits far more machinery than it builds:

- **The `Outcome` vocabulary and its draining interpreter** (`features/mod.rs`, `app::drain`,
  `app::interpret`) already exist, with twelve variants and real emitters. A feature reducer that
  must not write another feature's data already has the sanctioned way to say so. This feature
  extends that vocabulary; it does not invent it.
- **The overlay registry** (feature 017 + 021) already owns surface registration, dismissal and
  stacking uniformly. Popover openness is already derived rather than stored.
- **The write-isolation guard** (`tests/feature_write_isolation.rs`) already resolves every
  `&mut State` reducer to the set of state paths it writes, transitively, and reports cross-feature
  writes. Its `OWNERS` map is a ready-made, tested, per-path ownership table — which is precisely
  the input needed to decide which state can leave the root struct.
- **The registration-cost guard** (`tests/feature_registration_cost.rs`) already asserts that a
  feature is driven only from the root and that no feature module is edited because another feature
  was added.
- **18 `Widget` implementations** in `src/ui/` already own private per-instance state. The
  mechanism this feature needs is not new here; it is unevenly applied.
- **`src/showcase/state.rs`** is a second, independent unit with its own `Message` enum and reducer
  — 021's spec named it "a worked example of the target shape". It stays out of scope and stays the
  reference point.

### Framework constraint, stated up front

Constitution Principle V mandates Rust and iced and forbids introducing an alternative GUI
framework. This feature is scoped **inside** that constraint deliberately. The two pains above were
examined against the alternative of changing frameworks and the alternative was rejected on
evidence, not on the principle alone: the pains are nesting and ownership problems, both of which
the current framework's primitives already solve — as the 18 existing stateful widgets and the one
nested feature demonstrate — while a framework change would discard roughly 45,000 lines of
rendering code and about half the test suite to fix them.

## Clarifications

### Session 2026-08-26

- Q: The spec's baseline figures were inherited from feature 021 and three of them are wrong against the code as it stands, which makes SC-004's target of "eleven of eleven" unsatisfiable — should they be corrected? → A: Yes, all three, to their measured values: ten feature modules (not eleven), twelve outcome variants (not seven), and thirteen uses of component-owned state (not eighteen). SC-004 now reads ten of ten, up from one of ten. The bullets recorded in the 2026-08-25 session are left as written; they are a log of what was answered, not a statement of current fact.

### Session 2026-08-25

- Q: Story 2 asks that single-owner state move into the widget that renders it, but all five fields that qualify are pinned to the application by an existing feature-017 test that FR-021 forbids relaxing — should Story 2 instead give each feature its own state struct on the application, and keep the widget rule only as a guard for future fields? → A: Yes — two tracks. Feature-owned state structs deliver Story 2's outcome; FR-007's widget rule ships as a guard that moves nothing today and catches the first field that genuinely qualifies.
- Q: The root has a message that four tests exercise but that no running code ever sends — leave it in place with its dead status recorded, wire it up, or delete it? → A: Leave it and record it. The guard gains a third verdict — *no owner* — which it reports rather than fails; wiring it would change behavior FR-019 freezes, deleting it would remove assertions FR-021 protects.
- Q: Should SC-001 stay a one-time measurement, or become a fourth guard that fails the build when it stops being true? → A: Measured once, with the three guards recorded as its standing enforcement — a change that touched a second file would have had to add a root variant or a root state path, and one of the three catches that.
- Q: Should the platform-independent test list gain only this feature's three new guards, or the four existing architecture guards alongside them? → A: All seven. The four feature-021 guards join this feature's three, closing an omission 021's own notes recorded twice and left open.
- Q: Regrouping the state renames the paths that ~100 test files read, so nearly every assertion's text changes — should the assertion-freeze check be switched on for this feature? → A: Yes, enforcing, with an adjudications file recording each renamed assertion and the path rename that caused it. A report nobody must act on is the same failure mode as the optional guidance that left feature 021 at one feature in eleven.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Change a feature without touching the root (Priority: P1)

A maintainer adds an interaction to one feature — a new menu entry, a new field, a new keystroke
the feature responds to. Today that means editing the feature module, adding a variant to a
119-variant enum in `app.rs`, adding an arm to `State::update` or to the shell's `update_inner`,
and often adding a field to a 44-field struct. Four files, three of which belong to nobody.

Afterwards it means editing the feature module, and nothing else.

**Why this priority**: It is the pain the maintainer meets on literally every change, it is the one
021 already proved is solvable here (14 variants → 1), and it is a precondition for the second
story: state cannot move out of the root while the messages that write it are still declared there.

**Independent Test**: Add one interaction to any already-nested feature and count the files
changed. Delivers value on its own — each feature converted is a permanent reduction of the root
enum, and the application stays green and shippable after each one.

**Acceptance Scenarios**:

1. **Given** a feature module with its own message vocabulary, **When** a maintainer adds an
   interaction that no other feature observes, **Then** the change is confined to that feature's
   module and its view, and neither the root message vocabulary nor the root reducer is edited.
2. **Given** a feature whose interaction must inform another feature, **When** the maintainer
   implements it, **Then** the feature reports a consequence in the shared outcome vocabulary and
   the root routes it, and the feature does not name the other feature's data.
3. **Given** the whole set of feature modules, **When** the suite runs, **Then** every feature
   module is shown to have its own message vocabulary and its own reducer entry point, with any
   exemption named explicitly and justified in the guard itself.

---

### User Story 2 - State that is nobody else's business lives nowhere else (Priority: P2)

A maintainer reads a feature module and can see everything that feature remembers. Today a
feature's state is scattered across a shared 44-field struct, and telling which fields are "its"
requires consulting a separate ownership map in a test file. Anything a view needs is threaded down
from the root; anything it changes is threaded back up.

Afterwards, everything a feature remembers is declared in that feature's own module as a single
named grouping, which the application holds in one place. State the application genuinely shares
stays shared, and says so. Where a path additionally has no reader outside the feature's view, it
belongs inside the component that renders it — but only where no existing assertion pins it to the
application, because those assertions are the behavior specification this feature is forbidden to
relax.

**Why this priority**: It is the deeper of the two pains and the one never attempted, but it
depends on Story 1 — a feature cannot own its state until it owns the messages that write it. It
also carries more risk per unit of value, because moving state changes *when* things are
remembered, not just where they are declared.

**Independent Test**: Pick one feature that qualifies, move its state into the component, and
confirm the root struct's field count falls by exactly the fields moved with no behavior change
observable in the existing suite. Each feature is independently valuable and independently
revertible.

**Acceptance Scenarios**:

1. **Given** a state path with exactly one writing feature, **When** the migration is complete,
   **Then** that path is declared in that feature's own module and does not appear as a loose field
   of the root application state.
2. **Given** a state path that more than one feature reads, **When** the migration is complete,
   **Then** it remains in the root state, and the reason is recorded where a maintainer will find
   it rather than inferred.
3. **Given** a component that owns state, **When** it is destroyed and recreated as part of normal
   use — a project switch, a session close, a surface dismissal — **Then** the resulting behavior
   matches today's exactly, including whether a draft, a scroll position or a selection survives.
4. **Given** the application at rest, **When** state has moved into components, **Then** no
   additional frames are drawn compared to today.

---

### User Story 3 - The pattern cannot be opted out of (Priority: P1)

A maintainer adds a twelfth feature six months from now, under deadline. Nothing in the codebase
lets them declare it "small enough" to skip the pattern and put its variants in the root enum.

**Why this priority**: This is the story that makes the other two durable, and it is P1 despite
being listed third because *without it this feature is feature 021 again*. 021's optional guidance
reached one feature in ten; every guard it made mandatory still holds today, without exception.
The evidence for this story is already in the repository.

**Independent Test**: Attempt each violation against the guards and confirm each is reported: a new
root variant belonging to one feature, a new root state field with a single owner, a feature module
with no reducer entry point, and a feature reducer reaching for another feature's data.

**Acceptance Scenarios**:

1. **Given** the guards are in place, **When** a maintainer adds a message variant to the root
   vocabulary that only one feature produces and consumes, **Then** the suite fails and names the
   feature that should have declared it.
2. **Given** the guards are in place, **When** a maintainer adds a root state field whose only
   writer is one feature, **Then** the suite fails and names that feature.
3. **Given** a genuine exception exists, **When** it is recorded in the guard's allowlist with a
   written reason, **Then** the suite passes and the exception is visible in one place with its
   justification beside it.

---

### Edge Cases

- **A feature's state is read by its own view only.** The view is not the feature module, so a
  naive "no reader outside the module" rule would misclassify it. The rule must count a feature's
  view as part of the feature.
- **State whose lifetime is shorter than the component that displays it.** A dismissed popover, a
  cancelled dialog, a closed session tab: the component is destroyed and its state with it. Where
  today's behavior is that a draft survives dismissal, moving that draft into the component would
  silently change behavior. Every such case must be identified before it is moved, not after.
- **State written by a background event rather than by an interaction.** Daemon frames, session
  exits, theme changes from the OS arrive from outside any component's event handling. A component
  that owns state a background event writes cannot receive that write directly.
- **The terminal and session cluster.** It is the largest share of the root vocabulary, it is
  written by the daemon at high frequency, and its grid is read by more than one part of the UI. It
  is the case most likely to qualify for Story 1 and least likely to qualify for Story 2, and
  treating those two judgments separately is the point.
- **A feature that legitimately has no state and no messages of its own.** The pattern must be
  satisfiable by such a feature without inventing an empty vocabulary as ceremony.
- **Two features that must act on one interaction.** Escape, a scroll beneath an open surface, and
  a window resize are observed by several features at once. These are cross-cutting by nature and
  belong in the root vocabulary; the guards must not push them into an arbitrary feature.
- **A root message with no producer at all.** A variant can be declared, matched and exercised by
  tests while nothing in the running application sends it. It has no owning feature to be pushed
  into, and both obvious fixes are forbidden here: wiring it up changes behavior, deleting it
  removes assertions. The guard must be able to say *no owner* as a distinct, reported verdict.

- **An exemption granted and then forgotten.** An allowlist entry that outlives its reason is the
  same failure as no guard at all, only quieter.

## Requirements *(mandatory)*

### Functional Requirements

**Message vocabulary (Story 1)**

- **FR-001**: Every feature module MUST declare its own message vocabulary covering the
  interactions that only it produces and only it consumes.
- **FR-002**: Every feature module MUST expose exactly one reducer entry point over that
  vocabulary, so the root has one arm per feature rather than one arm per interaction.
- **FR-003**: The root message vocabulary MUST retain only messages that are genuinely
  cross-cutting — observed by more than one feature, or produced by the environment rather than by
  a feature's own interaction.
- **FR-004**: A feature reducer MUST NOT write another feature's data. Where an interaction has a
  consequence for another feature, the reducer MUST report it in the shared outcome vocabulary and
  the root MUST route it. (This restates the rule 021 established; this feature extends its reach,
  not its meaning.)
- **FR-005**: A feature with no interactions of its own MUST be able to satisfy FR-001 and FR-002
  without declaring an empty vocabulary, and the guard MUST recognise that case rather than
  requiring ceremony.
- **FR-006**: Converting a feature MUST be possible one feature at a time, leaving the application
  buildable, runnable and green after each one.

**State ownership (Story 2)**

- **FR-007**: Every feature's state MUST be declared in that feature's own module as a single named
  grouping, which the root application state holds in one place, rather than as loose paths spread
  across the root.
- **FR-007a**: A state path with exactly one writing feature and no reader outside that feature's
  own module and its view MUST additionally move into the component that renders it — unless an
  existing assertion pins it to the application, in which case the path stays and the guard's
  allowlist MUST record that assertion as the reason. FR-021 makes this exception mandatory rather
  than discretionary: no existing assertion may be relaxed to let a path move.
- **FR-008**: A state path read by more than one feature MUST remain in the root application state,
  and the reason MUST be recorded in the codebase rather than left to be inferred.
- **FR-009**: Moving a state path MUST NOT change when that state is created, retained or
  discarded. Where today's lifetime differs from the owning component's lifetime, the difference
  MUST be identified and preserved explicitly.
- **FR-010**: State moved into a component MUST remain reachable by the existing test suite without
  starting the application, or the behavior it governs MUST be re-covered by a test that does not
  need a window.
- **FR-011**: Moving state MUST NOT cause the application to draw frames while idle.
- **FR-012**: Components owning state MUST be built from the shared component library and expose
  the chainable builder API the project already mandates, rather than becoming bespoke per-feature
  one-offs.

**Enforcement (Story 3)**

- **FR-013**: A guard MUST fail when a message variant in the root vocabulary is produced and
  consumed by exactly one feature, and MUST name that feature. A variant with **no** producer MUST
  be reported rather than failed, and MUST carry a written reason recording that its behavior is
  specified by tests but unreachable in the running application.
- **FR-014**: A guard MUST fail when a root state path has exactly one writing feature and no
  reader outside it, and MUST name that feature.
- **FR-015**: A guard MUST fail when a feature module has no reducer entry point.
- **FR-016**: Each guard MUST accept exceptions only through an explicit allowlist that carries a
  written reason per entry, in the guard itself.
- **FR-017**: Each guard MUST be demonstrated non-vacuous: for each rule, the violation it forbids
  MUST be shown to fail the suite before the guard is relied upon.
- **FR-018**: The guards MUST run in the environment that runs without a window, so they hold on
  every supported platform rather than only where the full suite runs. This covers the three guards
  this feature adds **and** the four architecture guards feature 021 left running on one platform
  only; all seven read source text and start no window.

**Behavior preservation (all stories)**

- **FR-019**: No user-visible behavior may change: not rendering, not keyboard handling, not
  persistence, not what survives a restart.
- **FR-020**: Where a behavior turns out to be undefined or accidental today and the restructuring
  forces a decision, the decision MUST be recorded with its reasoning and pinned by a test, rather
  than being made silently.
- **FR-021**: The existing test suite MUST continue to pass throughout, and no existing assertion
  may be removed to accommodate the restructuring. This MUST be enforced by the repository's
  assertion-freeze check rather than asserted: the check MUST fail this feature's branch, and every
  assertion whose text changes because a state path was renamed MUST be recorded as a reviewed
  rename with the rename that caused it.

### Key Entities

- **Feature module**: One module holding a feature's types together with the operations over them.
  Ten exist. Render-free, and required to stay so.
- **Feature message vocabulary**: The set of interactions one feature produces and consumes,
  declared by that feature. One exists today.
- **Feature reducer**: The single entry point that applies a feature's own vocabulary to state and
  reports consequences. One exists today.
- **Outcome vocabulary**: The shared set of consequences a feature reports for the root to route.
  Twelve variants exist today.
- **Root message vocabulary**: What remains after FR-003 — cross-cutting interactions and
  environmental events.
- **Root application state**: What remains after FR-007 and FR-008 — one grouping per feature, plus
  the paths more than one feature reads, each carrying its recorded reason.
- **Component-owned state**: State held inside the component that renders it, created and destroyed
  with it. The mechanism exists and is used 13 times.
- **Ownership map**: The per-path table of which feature writes which state, already built and
  already tested.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: Adding an interaction that no other feature observes changes **one feature module and
  its view, and no other file** — measured by making such a change and counting, not asserted. No
  fourth guard is added for it: the three guards are its standing enforcement, because a change that
  reached a second file would have had to add a root message variant or a root state path, and
  SC-002 and SC-003 forbid both.
- **SC-002**: The root message vocabulary contains **no variant produced and consumed by exactly
  one feature**, with every exception listed and justified in one place. The rule is the criterion;
  a variant count is reported alongside it as evidence, not as the target.
- **SC-003**: The root application state contains **no loose path with exactly one writing
  feature** — every such path is declared inside its feature's own state grouping — with every
  exception listed and justified in one place.
- **SC-004**: **Every** feature module has its own message vocabulary and reducer entry point —
  ten of ten, up from one of ten, with FR-005's no-interaction case counted as satisfying
  rather than exempt.
- **SC-005**: Each of the three guards is demonstrated non-vacuous by observing its forbidden
  violation fail the suite.
- **SC-006**: The complete pre-existing test suite passes, with no assertion removed, at every
  merged step — not only at the end.
- **SC-007**: A maintainer can name everything a given feature remembers by reading that feature's
  module alone, without consulting the root application state or a test file's ownership map.
- **SC-008**: The application draws no more frames while idle than it does today.
- **SC-009**: Each user story is demonstrated green independently, so the feature delivers value if
  it stops after Story 1.

## Assumptions

Recorded because the description did not settle them and a reasonable default exists. Each is a
decision this spec is making, not a question it is deferring.

- **The qualifying rule for moving state is mechanical, not editorial.** "Nobody else's business"
  is defined as FR-007a states it — one writer, no outside reader — and is computed from the
  ownership data the write-isolation guard already builds. 021's pattern spread to one feature in
  ten because the decision was left to per-feature judgment; this spec removes the judgment.
  FR-007 itself needs no judgment at all: every feature gets its grouping.
- **A feature's view counts as part of the feature** for the purposes of FR-007. Views live beside
  features rather than inside them by existing convention, and treating that convention as
  disqualifying would move nothing.
- **The session and terminal cluster is in scope for Story 1, and for FR-007 like every other
  feature.** It is the largest share of the root vocabulary, so exempting it would forfeit most of
  the benefit. Its grid has multiple readers, so FR-008 is expected to keep parts of it declared as
  shared rather than folded into the session grouping, and that is a correct outcome rather than a
  partial one. Only FR-007a's further move into a component is evaluated case by case.
- **`src/showcase/` is out of scope.** It is a development-only second binary that already has the
  target shape, and 021 reached the same conclusion for the same reason.
- **The daemon, the core crate and the wire protocol are untouched.** This feature lives entirely
  within the client's client-side structure.
- **The framework does not change.** Constitution Principle V governs, and the analysis above
  concluded the same thing on independent grounds.
- **Guards are extended, not duplicated.** The write-isolation and registration-cost guards already
  compute most of what FR-013 through FR-015 need; new guards are expected to build on that
  machinery rather than re-derive it.
- **Success is measured against `b43c11c`.** Other features will land on `main` while this one is
  in flight and will move the counts; following 021's own lesson, the baseline is pinned to a
  revision and the criteria are stated as rules rather than as numbers wherever a rule is available.

## Dependencies

- Feature 017 (Material component architecture) — the shared surface and layer vocabulary.
- Feature 021 (Feature-module MVU architecture) — the ten feature modules, the outcome
  vocabulary and interpreter, the overlay registry, the shell split, and the two guard tests this
  feature extends. **This feature completes 021's Tier 3 rather than revisiting Tiers 1 and 2.**
- Constitution Principles I (test-first), V (Rust + iced), VII (documentation) and VIII (reusable
  component foundation, including the mandatory builder API).
