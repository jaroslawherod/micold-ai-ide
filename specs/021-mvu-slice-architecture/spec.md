# Feature Specification: Feature-Slice MVU Architecture

**Feature Branch**: `feat/mvp-arch-refactor`

**Created**: 2026-07-28

**Status**: Draft

**Input**: User description: "Refactor the application's internal architecture from a monolithic MVU core into a distributed, component-based MVU with an explicit service layer, without changing any user-visible behavior."

## Context: What This Feature Is

This is an **internal restructuring**. No end user of the application sees anything change — not a
pixel, not a keystroke, not a saved file. The people this feature serves are the **maintainers** of
the codebase, and the value delivered is the cost of the *next* change: how many places you must
edit to add one dropdown, how much you must hold in your head to reason about one feature, and
whether you can test a feature without starting the whole application.

Accordingly, the user stories below are written from the maintainer's perspective, and the success
criteria measure the cost of change rather than the behavior of the product.

### Measured baseline (verified against the codebase on 2026-07-28)

| Concern | Where it lives today | Size |
|---|---|---|
| Application state, messages, reducer | `crates/micold-client/src/app.rs` | 2,245 lines |
| Shell + all remaining I/O | `crates/micold-client/src/main.rs` | 2,914 lines |
| Single flat `State` struct | `app.rs` | 36 fields |
| Single `Message` enum | `app.rs` | 124 variants |
| Modal `Overlay` enum | `app.rs` | 10 variants |
| Ad-hoc popover state fields | `app.rs` | 7 separate fields |

The two files above are the largest and second-largest source files in the repository.

### What already exists (scope reducers, not scope)

Two of the four desired outcomes in the original description are **partly delivered** by earlier
features, and this spec is scoped to the remainder rather than re-specifying them:

- **Overlay rendering, dismissal and stacking are already unified** (feature 017). A shared
  `Layer` / `Surface` / `Trigger` vocabulary and a single dismissal rule live in the render-free
  core; a single floating-surface primitive renders every window-level surface; guard tests
  (`one_overlay_implementation`, `overlay_dismissal_delta`, `overlay_stacking`) hold the line.
  **What remains** is the *state and routing* half: the flat `Overlay` enum, the parallel
  closing-overlay snapshot enum, the central escape-handling match, the seven loose popover
  fields, and the per-overlay fields scattered through `State`.
- **Seven service ports already exist** in the render-free core (version control, project store,
  settings store, folder scanning, terminal backend, terminal handle, AI CLI provider), and
  process/PTY I/O has moved out of the client entirely into the session daemon. **What remains**
  is that the client still constructs concrete implementations inline at the point of use instead
  of receiving them, and three I/O concerns (clipboard, OS theme probe, environment-include
  resolution) have no port at all.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Add an overlay without a scavenger hunt (Priority: P1)

A maintainer adds a new dropdown or dialog. Today this means finding and editing eight unrelated
places: the overlay enum, new state fields, a reducer arm, the escape-handling function, the
keyboard subscription's escape match, the view match, the closing-snapshot match, and the
closing-snapshot enum. Miss one and the overlay opens but will not close, or closes without its
exit animation, and nothing fails until someone tries it by hand.

After this change, the maintainer creates the overlay's own module — its state, its messages, its
reducer, its view — and registers it once. Nothing else in the codebase needs to know it exists.

**Why this priority**: This is the pain the maintainer named first and the one that recurs most
often. It is also the outcome with the sharpest, most falsifiable test: count the files a new
overlay touches. Delivered alone, it removes the largest single source of "I forgot a site" bugs.

**Independent Test**: Add a throwaway overlay end-to-end and count the files changed. It must be
its own module plus at most one registration line. Every existing overlay and popover must still
open, close on Escape, close on outside click, close on scroll-beneath, and animate out exactly as
before — held by the existing overlay test suite, unchanged.

**Acceptance Scenarios**:

1. **Given** a new overlay defined in its own module and registered once, **When** the maintainer
   builds and runs the application, **Then** the overlay opens, dismisses by every dismissal
   trigger appropriate to its kind, and stacks correctly against other surfaces — with no edit to
   any central enum, match statement, or shared state struct.
2. **Given** the existing modals and popovers, **When** the whole overlay suite runs after the
   restructuring, **Then** every test passes unchanged, including dismissal-delta, stacking and
   transition-identity guards.
3. **Given** two overlays open at once, **When** the user presses Escape, **Then** the same one
   closes as before the change, in the same order.
4. **Given** a maintainer who omits the single registration step, **When** the project is built,
   **Then** the omission is caught at build time or by a guard test — not discovered by hand at
   runtime.

---

### User Story 2 - Reason about and test one feature in isolation (Priority: P1)

A maintainer changing worktree behavior must currently read a 36-field state struct and a
124-variant message enum in which worktree concerns are interleaved with sessions, sidebar,
terminal, notifications and settings — then find the relevant arms inside one very long reducer.
Testing that behavior means constructing the entire application state.

After this change, each feature — worktree, session/terminal, project/workspace, sidebar, settings,
notifications, overlays — is a self-contained module owning its own state, messages, reducer and
view-model projection. The maintainer reads one module and tests it against fakes without the rest
of the application present.

**Why this priority**: Equal-first with Story 1. It is the outcome the other three depend on:
without slice boundaries there is nothing for the overlay registry to register into, no unit under
test for the service ports to be injected into, and no boundary for the coordination rule to
protect.

**Independent Test**: Pick one slice; write a test that constructs only that slice's state, sends
only its messages, and asserts on its own state and outcomes — with no reference to any other
slice's types. The test must compile and pass without the application shell.

**Acceptance Scenarios**:

1. **Given** a feature slice, **When** a maintainer writes a test for it, **Then** the test can be
   written against that slice's state and messages alone, with fakes for anything external.
2. **Given** the root state, messages and reducer, **When** a maintainer reads them, **Then** they
   find composition and routing only — no feature's own decision logic.
3. **Given** any one feature slice, **When** a maintainer inspects its type definitions, **Then**
   the slice has no way to express or reach another slice's state.
4. **Given** the full existing test suite, **When** it runs after the restructuring, **Then** every
   test passes with no assertion changed.

---

### User Story 3 - Swap real I/O for fakes at the boundary (Priority: P2)

Deciding what happens on disk, in version control, on the clipboard, or against the operating
system's theme setting is currently entangled with the shell that performs it. Behavior that
depends on I/O is therefore reachable only by running the real thing.

After this change, every I/O concern is a narrow capability declared in the render-free core. The
application depends on the declared capability; the binary supplies the real implementation at
startup; a test supplies a fake. Adding a new I/O concern means declaring one narrow capability,
not widening an existing one.

**Why this priority**: Valuable and clearly separable, but it builds on the slice boundaries from
Story 2 — a port injected into a monolith buys much less than one injected into a slice. Also
genuinely partly done already, so the increment is smaller.

**Independent Test**: For each capability, run the behavior that depends on it against a fake and
assert the outcome, with no real filesystem, repository, clipboard, or OS query involved.

**Acceptance Scenarios**:

1. **Given** any I/O-dependent behavior, **When** a maintainer tests it, **Then** a fake capability
   can be substituted without touching the code under test.
2. **Given** the application's non-shell code, **When** a maintainer inspects its dependencies,
   **Then** it names only declared capabilities, never a concrete implementation.
3. **Given** the binary at startup, **When** it assembles the application, **Then** it is the
   single place where real implementations are chosen and supplied.
4. **Given** a capability with several unrelated operations, **When** a consumer needs one of them,
   **Then** it depends on that operation's capability alone and is not forced to fake the rest.

---

### User Story 4 - Cross-feature effects are visible, not hidden (Priority: P2)

Deleting a worktree today silently reaches into session state and overlay state from inside the
worktree code path. Nothing at the type level prevents this, and nothing makes it visible to a
reader of either the sessions code or the overlay code — so the coupling is discoverable only by
tracing the reducer by hand.

After this change, a slice may mutate only its own state. Anything that must affect another
feature is *returned* as an explicit outcome value that the shell interprets and dispatches.
Cross-feature effects become a readable list at the boundary rather than hidden writes.

**Why this priority**: This is what keeps Story 2's boundaries from eroding back into a monolith
one convenient reach-across at a time. It is P2 rather than P1 only because the boundaries must
exist before they can be enforced.

**Independent Test**: Take the worktree-delete path — the named anti-pattern. Run it in isolation
and assert that it returns outcomes describing the session and overlay consequences, while mutating
only worktree state itself. The existing `worktree_delete` tests must pass unchanged.

**Acceptance Scenarios**:

1. **Given** a worktree is deleted, **When** the operation runs in isolation, **Then** it mutates
   only worktree state and returns explicit outcomes for the session and overlay consequences.
2. **Given** those outcomes, **When** the shell interprets them, **Then** the end-to-end observable
   result is identical to today's, verified by the existing tests.
3. **Given** a maintainer who attempts to write directly to another slice's state from within a
   slice, **When** the project is built, **Then** it does not compile.

---

### Edge Cases

- **Two overlays open at once.** Dismissal priority and stacking order must not change. The
  existing behavior — where a lightweight popover is checked ahead of the modal overlay — must be
  preserved by the generic dispatch, not lost when the special-case match disappears.
- **An overlay mid-exit-animation.** The application currently renders a *snapshot* of an overlay
  whose live state has already been cleared, so it can animate out. Any unified representation must
  preserve this, including the case where the user reopens the same overlay before its exit
  animation finishes.
- **Popovers that must not clear their own data on close.** Closing the sidebar filter panel must
  leave the active filters intact. A generic dismissal path must not "helpfully" reset slice state.
- **Mutual exclusivity between popovers and modals.** Opening a modal currently closes lightweight
  popovers. This must survive as an explicit, testable rule rather than as several hand-written
  field resets that are easy to forget.
- **A slice that legitimately needs another slice's data to render.** View-model projection must be
  able to read across slices at the composition boundary without granting mutation rights.
- **An outcome that triggers a further outcome.** Interpreting one cross-slice effect may produce
  another. The dispatch must terminate and must not depend on the order slices happen to be
  composed in.
- **State restored from disk written by the previous architecture.** Persisted files written before
  this change must load and behave identically after it.
- **A migration step landed on its own.** Every intermediate step must leave the application
  buildable, runnable and green — no step may depend on a later one to compile.

## Requirements *(mandatory)*

### Functional Requirements

#### Feature slices

- **FR-001**: Each named feature — worktree, session/terminal, project/workspace, sidebar,
  settings, notifications, overlays — MUST be a self-contained module owning its own state,
  message type, reducer and view-model projection.
- **FR-002**: The root state, message type and reducer MUST contain composition and routing only,
  and MUST NOT contain any individual feature's decision logic.
- **FR-003**: A slice's types MUST NOT be able to express another slice's state; invalid
  cross-slice states MUST be unrepresentable rather than merely avoided by convention.
- **FR-004**: Every feature slice MUST be unit-testable in isolation, constructing only that
  slice's state and using fakes for anything external.
- **FR-005**: No single source file may concentrate unrelated features. Specifically, the files
  that today hold the monolithic state/reducer and the shell/I/O MUST no longer be the two largest
  source files in the repository, and MUST no longer be where unrelated features live.
- **FR-006**: Feature slices MUST remain render-free and separately testable, consistent with the
  existing core/client split and Constitution Principle I.

#### Overlays as a uniform layer

- **FR-007**: Modals and lightweight popovers MUST share one representation. The four ad-hoc
  popovers (help menu, project switcher, sidebar filter panel, worktree context menu) and the
  remaining loose popover state MUST migrate onto it.
- **FR-008**: The central per-overlay match statements — the overlay enum, the escape-handling
  match, the keyboard subscription's escape match, the view match, the closing-snapshot match and
  the closing-snapshot enum — MUST collapse into a single generic dispatch that knows only the
  shared abstraction and not any specific overlay.
- **FR-009**: Adding a new overlay MUST require changes to that overlay's own module plus at most
  one registration point.
- **FR-010**: Failing to register a new overlay MUST be caught at build time or by a guard test,
  not left to manual discovery at runtime.
- **FR-011**: The unified representation MUST preserve the existing exit-animation snapshot
  behavior, including reopening an overlay while it is animating out.
- **FR-012**: The existing dismissal priority between simultaneously-open surfaces, and the rule
  that opening a modal closes lightweight popovers, MUST be preserved as explicit, tested rules.
- **FR-013**: Dismissing an overlay MUST NOT alter slice state that the dismissal does not own —
  in particular, closing the sidebar filter panel MUST leave the active filters unchanged.
- **FR-014**: This work MUST build on the existing shared floating-surface vocabulary and
  dismissal rule rather than introducing a second, parallel one; the existing overlay guard tests
  MUST continue to pass unchanged.

#### Service layer

- **FR-015**: Every I/O concern MUST be expressed as a narrow, single-purpose capability declared
  in the render-free core: version control, persistence/store, folder scanning, clipboard, OS
  theme probe, and environment-include resolution.
- **FR-016**: Capabilities MUST be narrow enough that a consumer needing one operation is not
  forced to supply or fake unrelated ones.
- **FR-017**: Non-shell code MUST depend only on declared capabilities and MUST NOT reference or
  construct a concrete implementation.
- **FR-018**: The binary MUST be the single place where concrete implementations are chosen and
  supplied to the application.
- **FR-019**: Every capability MUST have a usable fake implementation, and every behavior that
  depends on I/O MUST be testable through it without real filesystem, repository, clipboard or
  operating-system access.

#### Cross-slice coordination

- **FR-020**: A slice MUST mutate only its own state.
- **FR-021**: Any consequence affecting another feature MUST be returned as an explicit outcome
  value rather than applied directly.
- **FR-022**: The shell MUST be the component that interprets outcomes and routes them to the
  slices they concern.
- **FR-023**: The worktree-delete path MUST no longer write directly to session or overlay state,
  and MUST instead return outcomes describing those consequences.
- **FR-024**: Outcome interpretation MUST terminate and MUST NOT depend on the order in which
  slices are composed.

#### Behavior preservation and migration

- **FR-025**: No observable application behavior may change: not visual layout, keybindings,
  dismissal behavior, animation timing, notification behavior, or any user-facing text.
- **FR-026**: The persisted-state format MUST NOT change. Files written before this change MUST
  load and behave identically after it.
- **FR-027**: The entire existing test suite MUST pass with no assertion modified. Tests may be
  *added*; existing expectations may not be relaxed, rewritten or deleted to accommodate the
  restructuring.
- **FR-028**: The migration MUST be expressible as incremental steps that can each ship
  independently, every one leaving the application buildable, runnable and green. (Determining the
  actual sequence is out of scope for this specification.)
- **FR-029**: The architecture MUST remain within the mandated model-view-update shape and MUST
  NOT adopt a retained-view model that works against the GUI framework.
- **FR-030**: Shared user-interface components MUST retain their mandated chainable builder API
  terminating in conversion to an element.

### Key Entities

- **Feature slice**: A self-contained unit of application behavior. Owns its own state, its own
  message vocabulary, the reducer that folds those messages into that state, and a pure projection
  of that state for display. Knows nothing of any other slice.
- **Slice outcome**: An explicit value returned by a slice's reducer describing a consequence that
  falls outside the slice's own state. The only sanctioned channel for cross-feature effects.
- **Layer / floating surface**: The uniform representation of any transient surface drawn over the
  base view — modal or popover alike. Knows how to render itself, what dismisses it, and which
  band of the stacking order it belongs to.
- **Overlay registry**: The single registration point through which a new floating surface becomes
  known to the generic dispatch.
- **Service capability (port)**: A narrow, single-purpose declaration of an I/O need, stated by the
  render-free core and satisfied by either a real implementation supplied by the binary or a fake
  supplied by a test.
- **Composition shell**: The thin layer that owns slice composition, message routing, outcome
  interpretation, and the supply of concrete capabilities. Holds no feature logic of its own.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: Adding a new floating surface — modal or popover — touches exactly one new module
  plus at most one registration line. Verified by performing the addition and counting changed
  files; the count of central match statements a new surface must be added to is **zero**, down
  from six.
- **SC-002**: Adding a new feature slice touches exactly one new module plus at most one
  registration line, with zero edits to any other slice.
- **SC-003**: Neither of the two files that today hold the monolithic state/reducer and the
  shell/I/O remains among the largest source files in the repository, and neither contains logic
  belonging to more than one feature.
- **SC-004**: Every feature slice has at least one test that constructs only that slice's state and
  references no other slice's types.
- **SC-005**: Every declared service capability has a fake implementation and at least one test
  that exercises real behavior through it, with zero real filesystem, repository, clipboard or
  operating-system access.
- **SC-006**: The complete pre-existing test suite passes with zero assertions modified, on all
  three supported platforms.
- **SC-007**: Zero direct writes from one slice into another slice's state remain; the
  worktree-delete path in particular reports its session and overlay consequences as outcomes.
  Attempting such a write does not compile.
- **SC-008**: Application state written by the pre-change version loads and produces identical
  behavior after the change.
- **SC-009**: Every migration step, taken in order, leaves the application building, running and
  passing its tests — verified by the step's own commit, not only by the final state.
- **SC-010**: A maintainer can answer "where does this feature live?" by naming a single module,
  for every feature named in FR-001.

## Assumptions

- **Stakeholder.** The beneficiary is the codebase's maintainers. "No user-visible change" is a
  hard constraint, so all measurable value is expressed as cost-of-change and testability.
- **Baseline is post-daemon-split.** This specification was written against the workspace as it
  stands after the client/core/daemon split. Process and terminal I/O already live in the session
  daemon, so a client-side process-spawn capability is **not** part of FR-015 — the daemon boundary
  already serves that role.
- **Overlay work is a continuation, not a restart.** The shared floating-surface vocabulary,
  dismissal rule, stacking order and single rendering primitive already exist. This feature
  completes the *state and routing* half and migrates the remaining ad-hoc popovers onto the
  existing abstraction rather than designing a new one.
- **Service layer is an extension, not a greenfield.** Seven capabilities already exist in the
  render-free core. The increment is injecting them rather than constructing them at the point of
  use, plus declaring the three that are missing.
- **Test suite is the behavior specification.** With no user-visible change to demonstrate, the
  existing suite — including its architectural guard tests — is the authority on "nothing broke".
  Its assertions are therefore frozen for the duration of this work.
- **Guard tests are the enforcement mechanism.** The codebase already uses executable guards to
  hold architectural lines. New invariants introduced here are expected to be held the same way,
  so that erosion fails a build rather than accumulating unnoticed.
- **Sequencing is deferred.** FR-028 requires that an incremental sequence *exist*; producing it is
  the planning phase's job, not this specification's.
- **Documentation.** This change is not user-facing, so Constitution Principle VII's user-guide
  obligation is satisfied by architectural documentation rather than a user-guide entry.

## Open Questions

Two decisions materially change the size and shape of this work and have no safe default. Both
arise because the original description predates the client/core/daemon split that the codebase has
since adopted.

- **Q1 (scope boundary)**: Does this restructuring cover the session daemon crate as well as the
  client and core? [NEEDS CLARIFICATION: The daemon has its own large files — a 1,218-line server
  and a 1,151-line state module — with concerns arguably as mixed as the client's. Including it
  roughly doubles the work; excluding it leaves a second monolith standing.]
- **Q2 (slice residence)**: Should feature slices live in the render-free core, or in the client
  alongside their views? [NEEDS CLARIFICATION: Core residence makes every slice reachable from the
  fast render-free test suite and maximizes what Constitution Principle I covers, but splits each
  feature across two crates. Client residence keeps a feature in one place but leaves slice logic
  in the crate whose test story is weaker.]
