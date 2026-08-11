# Feature Specification: Feature-Module MVU Architecture

**Feature Branch**: `feat/mvp-arch-refactor`

**Created**: 2026-07-28

**Status**: Merged (PR #47) and since amended — five inconsistencies found by the planning phase's
cross-artifact analysis are corrected in place, adding FR-015a and a daemon-connection feature. See
the checklist's iteration-5 findings for what changed and why.

**Input**: User description: "Refactor the application's internal architecture from a monolithic MVU core into a distributed, component-based MVU with an explicit service layer, without changing any user-visible behavior."

## Context: What This Feature Is

This is an **internal restructuring**. No end user of the application sees anything change — not a
pixel, not a keystroke, not a saved file. The people this feature serves are the **maintainers** of
the codebase, and the value delivered is the cost of the *next* change: how many places you must
edit to add one dropdown, how much you must hold in your head to reason about one feature, and
whether you can test a feature without starting the whole application.

Accordingly, the user stories below are written from the maintainer's perspective, and the success
criteria measure the cost of change rather than the behavior of the product.

### Measured baseline (re-verified against `main` on 2026-08-07)

| Concern | Where it lives today | Size | Was, 2026-07-28 |
|---|---|---|---|
| Application state, messages, reducer | `crates/micold-client/src/app.rs` | 2,434 lines | 2,245 |
| Shell + all remaining I/O | `crates/micold-client/src/main.rs` | 3,567 lines | 2,914 |
| Single flat `State` struct | `app.rs` | 37 fields | 36 |
| Single `Message` enum | `app.rs` | 130 variants | 124 |
| Modal `Overlay` enum | `app.rs` | 10 variants | 10 |
| Closing-snapshot `ClosingOverlay` enum | `app.rs` | 9 variants | — |
| Ad-hoc popover state fields | `app.rs` | 7 separate fields | 7 |

The two files above remain the largest and second-largest source files in the repository.

**The drift is itself evidence.** In the ten days between the two measurements the shell file grew
by 22% and the state file by 8%; the message enum gained six variants and the state struct a field,
without anyone setting out to enlarge them. The figures were taken twice during that window, on
2026-08-06 and again on 2026-08-07 after this branch was rebased onto `main`, and both files had
grown between those two readings alone. Nothing in the current structure resists accretion into
these two files; that is the cost this feature exists to remove, and the reason SC-003 states an
absolute line-count target rather than a relative improvement.

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
  resolution) have no port at all. Re-verified on 2026-08-07: still exactly seven ports, and the
  three named gaps are still unfilled.

### The second binary, and why it is out of scope

Feature 020 added a development-only component gallery as a second binary in the same crate,
excluded from packaging and held there by a guard test. It has its own state module, its own
message vocabulary and its own reducer, in under 300 lines. It is **out of scope** for this
feature — but it is worth naming for two reasons:

- It is a worked example of the target shape. A separate screen with an independent lifecycle got
  its own nested unit, which is exactly the condition FR-003 sets for nesting. It is the reference
  point for judging whether any feature of the main application meets the same bar.
- Its isolation from application state is already enforced by a guard test rather than by making
  the coupling unrepresentable. That is the same enforcement mechanism FR-024a specifies, already
  proven in this codebase.

### Structural stance: type-first modules, nesting only where a lifecycle demands it

The original description asked for a "distributed, component-based MVU". Established guidance on
structuring model-view-update applications makes two claims that narrow what that should mean here:

- A module is **a type together with the helper functions that operate on it**. Splitting one
  feature across parallel state / update / view files is an anti-pattern — it creates unanswerable
  questions about where a given function belongs, and separates a type from its own operations.
- Nesting a full model-update-view unit **per visual component** is the habit to unlearn. Nesting
  earns its cost at the granularity of a *page* — a screen with an independent lifecycle — not of a
  widget, panel or dialog that shares the surrounding screen's lifecycle.

This application is a single screen. It has no pages. Giving *every* feature its own state, message
vocabulary, reducer and outcome channel would therefore buy one message-wrapping layer per feature
and a cross-feature effect protocol, to pay for isolation the screen does not need — and would not
retire the "which feature owns this?" question, only move it up a level and make the answer
compiler-enforced. (The original description assumed seven such features; planning measurement
found eight. The count is not what makes the argument — the absence of pages is.)

Accordingly this feature pursues the same outcomes — one module per feature, a routing-only root,
tests that need no application shell, and an overlay that costs one file — through **type-first
extraction first, and message nesting only where a feature is shown to need it**. The work is
specified in three tiers plus a separate shell split:

| Tier | What moves | Nesting introduced |
|---|---|---|
| **1** | Every custom type in the monolithic state file, with its helper functions, into a module named for that type | None |
| **2** | The overlay enum, its closing-snapshot twin and their six central match statements, onto one uniform floating-surface type behind a registry | None — one shared type replaces two |
| **3** | The single long reducer, split into per-feature reducer modules over the shared state | None by default; a nested state + message + reducer unit only for a feature demonstrated to have an independent lifecycle |
| **Shell** | The shell/I/O file, split by the external system each part talks to, behind declared capabilities | Not applicable |

The tiers are ordered by value per unit of architectural risk, not by dependency. Tier 1 is the
largest reduction of the monolith for the least commitment. Tier 3 is deliberately last, because
the evidence for *which* features deserve their own message vocabulary only exists once Tiers 1 and
2 have landed — and if none do, Tier 3 stops at per-feature reducer modules and the feature is
still complete.

## Clarifications

### Session 2026-08-07

- Q: Does FR-025's "no observable behavior change" constrain runtime cost, and how should injected capabilities be dispatched? → A: Dynamic dispatch, no performance budget — FR-025 governs user-visible behavior only
- Q: When SC-003's 500-line proxy and FR-005's actual requirement disagree, which governs acceptance? → A: FR-005 governs; the 500-line figure is indicative, not a gate
- Q: Where should the capability fakes required by FR-019 live? → A: In the render-free core as ordinary public items, following the existing fake-implementation precedent
- Q: Should SC-001/SC-002's "count the changed files" verification become a permanent automated guard, or stay a one-time manual measurement? → A: A permanent guard test, replacing the one-time count
- Q: If the restructuring reveals that an existing test asserts genuinely wrong behavior, what happens? → A: Preserve the behavior and its assertion; file the bug separately and fix it after this feature

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

A maintainer changing worktree behavior must currently read a 37-field state struct and a
130-variant message enum in which worktree concerns are interleaved with sessions, sidebar,
terminal, notifications and settings — then find the relevant arms inside one very long reducer.
Testing that behavior means constructing the entire application state.

After this change, each feature — worktree, session/terminal, project/workspace, sidebar, settings,
notifications, daemon connection, overlays — is one module holding that feature's types and the
operations on them
(Tier 1), with its share of the reducer in a module of its own (Tier 3). The maintainer reads one
module and tests it against fakes without the rest of the application present.

**Why this priority**: Equal-first with Story 1. It is the outcome the other three depend on:
without per-feature boundaries there is nothing for the overlay registry to register into, no unit
under test for the service ports to be injected into, and no boundary for the coordination rule to
protect.

**Independent Test**: Pick one feature; write a test that constructs only that feature's types,
exercises only its own operations and reducer, and asserts on the result — with no reference to any
unrelated feature's types. The test must compile and pass without the application shell.

**Acceptance Scenarios**:

1. **Given** a feature module, **When** a maintainer writes a test for it, **Then** the test can be
   written against that feature's types and operations alone, with fakes for anything external.
2. **Given** the root state, messages and reducer, **When** a maintainer reads them, **Then** they
   find composition and routing only — no feature's own decision logic.
3. **Given** any one feature, **When** a maintainer asks where its data and its operations live,
   **Then** the answer is a single module, and that module does not split the feature's type from
   the functions over it.
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

**Why this priority**: Valuable and clearly separable, but it builds on the module boundaries from
Story 2 — a port injected into a monolith buys much less than one injected into a feature. Also
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
worktree code path. Nothing makes that visible to a reader of either the sessions code or the
overlay code — so the coupling is discoverable only by tracing the reducer by hand.

After this change, a feature's reducer module that must affect another feature *returns* an
explicit outcome value that the root interprets and dispatches, rather than reaching across and
writing. Cross-feature effects become a readable list at the boundary rather than hidden writes.

**Why this priority**: This is what keeps Story 2's boundaries from eroding back into a monolith
one convenient reach-across at a time. It is P2 rather than P1 only because the boundaries must
exist before they can be enforced.

Note the deliberate limit: the outcome channel is required *where a cross-feature consequence
exists*, not as blanket plumbing on every reducer. Per the structural stance, a feature reducer
that only touches its own data needs no outcome vocabulary, and the shared state struct is not
partitioned to make cross-feature writes unrepresentable — the requirement is that the remaining
real ones are named and routed, and held there by a guard test.

**Independent Test**: Take the worktree-delete path — the named anti-pattern. Run it in isolation
and assert that it returns outcomes describing the session and overlay consequences, while touching
only worktree data itself. The existing `worktree_delete` tests must pass unchanged.

**Acceptance Scenarios**:

1. **Given** a worktree is deleted, **When** the operation runs in isolation, **Then** it mutates
   only worktree data and returns explicit outcomes for the session and overlay consequences.
2. **Given** those outcomes, **When** the root interprets them, **Then** the end-to-end observable
   result is identical to today's, verified by the existing tests.
3. **Given** a maintainer who adds a new direct cross-feature write from inside a feature reducer,
   **When** the test suite runs, **Then** a guard test fails and names the offending path.

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
  leave the active filters intact. A generic dismissal path must not "helpfully" reset a feature's
  own state.
- **Mutual exclusivity between popovers and modals.** Opening a modal currently closes lightweight
  popovers. This must survive as an explicit, testable rule rather than as several hand-written
  field resets that are easy to forget.
- **A view that legitimately needs two features' data to render.** Reading across features to build
  a view must stay possible and cheap — the sidebar reads session data today. This is the specific
  case that rules out partitioning the state struct into mutually invisible halves.
- **An outcome that triggers a further outcome.** Interpreting one cross-feature effect may produce
  another. The dispatch must terminate and must not depend on the order feature modules happen to
  be composed in.
- **A feature that turns out not to need its own message vocabulary.** Tier 3 must be allowed to
  conclude, for a given feature, that a reducer module over the shared state is the right answer
  and no nested unit is warranted. That conclusion is a valid completion of the tier, not a
  shortfall.
- **State restored from disk written by the previous architecture.** Persisted files written before
  this change must load and behave identically after it.
- **A migration step landed on its own.** Every intermediate step must leave the application
  buildable, runnable and green — no step may depend on a later one to compile.
- **An existing test turns out to assert a latent bug.** The restructuring may surface behavior that
  is wrong but faithfully asserted by the frozen suite. The bug and its assertion MUST both be
  preserved, and the defect recorded as a separate bug report to be fixed in its own change after
  this feature. FR-027 admits no exception here: its whole value is that a red suite unambiguously
  means the restructuring broke something. A single "justified" assertion edit destroys that
  signal for every step that follows it.

## Requirements *(mandatory)*

### Functional Requirements

#### Feature modules and reducer split (Tiers 1 and 3)

- **FR-001**: Every custom type that today lives in the monolithic state file MUST move into a
  module named for that type or for the feature it serves, **together with** the helper functions
  that operate on it. For each named feature — worktree, session/terminal, project/workspace,
  sidebar, settings, notifications, daemon connection, overlays — a maintainer MUST be able to name
  the single module holding its data and its operations. (The daemon-connection feature was added
  to this list during planning: measurement found nine message variants, their own state fields and
  their own status projection, all of which meet every test this requirement applies. It concerns
  the *client's* handling of its connection, which has always been client code, and so does not
  reopen Q1's exclusion of the daemon process itself.)
- **FR-001a**: A feature MUST NOT be split across parallel state / update / view files. A type and
  the functions over it MUST NOT be separated by module boundary.
- **FR-002**: The root state, message type and reducer MUST contain composition and routing only,
  and MUST NOT contain any individual feature's decision logic.
- **FR-003**: A feature MUST introduce its own message type and nested reducer **only** where it
  has an independent lifecycle — it is opened, edited and dismissed as a unit whose intermediate
  state no other feature reads. Every other feature MUST be expressed as a feature module (FR-001)
  plus a per-feature reducer module (FR-004a), with no nested message vocabulary. The plan MUST
  record, per feature, which of the two applies and the evidence for it.
- **FR-003a**: The shared state struct MUST NOT be partitioned such that a view can no longer read
  across features; cross-feature *reads* for display remain permitted and cheap. Isolation is
  enforced on writes (FR-020) by guard test, not on reads by type.
- **FR-004**: Every feature module MUST be unit-testable in isolation, constructing only that
  feature's types and using fakes for anything external.
- **FR-004a**: The reducer MUST be split into per-feature reducer modules, each handling the message
  variants belonging to one feature. The root reducer MUST retain routing only, per FR-002. This
  applies to the reducer **wherever its arms live**: measurement during planning found not one long
  reducer but two over the same message enum — a pure one in the monolithic state file and a larger,
  effectful one in the shell file, split from each other by purity rather than by feature. Both are
  in scope. A feature's pure and effectful arms MUST end up on the same feature boundary: the pure
  arms in that feature's reducer module, the effectful arms in the shell module for the external
  system they address (FR-019a).
- **FR-004b**: Tier 3 MUST be able to conclude, for any given feature, that a reducer module over
  the shared state is sufficient and no nested state/message/reducer unit is warranted. Reaching
  that conclusion for every feature is a valid completion of the tier.
- **FR-004c**: Each tier MUST be independently shippable: Tiers 1, 2 and the shell split MUST each
  leave the application buildable, runnable and green without any part of Tier 3 having landed, and
  Tier 3 MUST NOT be a precondition for the overlay registry (FR-009) or the service layer
  (FR-017).
- **FR-005**: No single source file may concentrate unrelated features. Specifically, the files
  that today hold the monolithic state/reducer and the shell/I/O MUST no longer be the two largest
  source files in the repository, and MUST no longer be where unrelated features live.
- **FR-006**: Feature modules MUST remain render-free and separately testable, consistent with the
  existing core/client split and Constitution Principle I.

#### Overlays as a uniform layer (Tier 2)

- **FR-007**: Modals and lightweight popovers MUST share one representation. All seven ad-hoc
  popovers — help menu, project switcher, sidebar filter panel, worktree context menu, project
  context menu, terminal context menu and session context menu — and the remaining loose popover
  state MUST migrate onto it. (Re-verified 2026-08-07: the count is seven, up from the four named
  when this requirement was first written; two context menus and the session menu were added in
  the interim, each as another loose field, which is the accretion FR-009 exists to stop.)
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
- **FR-013**: Dismissing an overlay MUST NOT alter feature state that the dismissal does not own —
  in particular, closing the sidebar filter panel MUST leave the active filters unchanged.
- **FR-014**: This work MUST build on the existing shared floating-surface vocabulary and
  dismissal rule rather than introducing a second, parallel one; the existing overlay guard tests
  MUST continue to pass unchanged.

#### Service layer and shell split

- **FR-015**: Every I/O concern MUST be expressed as a narrow, single-purpose capability declared
  in the render-free core. The concerns this feature must account for are: version control,
  persistence/store, folder scanning, clipboard, OS theme probe, and environment-include
  resolution. **This list is not the full inventory of capabilities.** Three further ports already
  exist and are already satisfactory — terminal backend, terminal handle and AI CLI provider — and
  are listed here only to be explicit that they are in the inventory that SC-005 measures, while
  requiring no work under this requirement.
- **FR-015a**: Where the GUI framework makes a synchronous capability impossible — the operation
  returns a deferred task rather than a value — the I/O concern MAY instead be expressed as an
  explicit effect request in the outcome vocabulary (FR-021), interpreted by the shell. Such a
  concern is still subject to FR-017 (non-shell code MUST NOT reach the framework directly) and to
  FR-019/SC-005 (the request MUST be assertable in a test with zero real I/O). Clipboard access is
  the known instance: all three of its call sites return a deferred task, so a synchronous port
  cannot wrap them without blocking.
- **FR-016**: Capabilities MUST be narrow enough that a consumer needing one operation is not
  forced to supply or fake unrelated ones.
- **FR-017**: Non-shell code MUST depend only on declared capabilities and MUST NOT reference or
  construct a concrete implementation.
- **FR-018**: The binary MUST be the single place where concrete implementations are chosen and
  supplied to the application.
- **FR-019**: Every capability MUST have a usable fake implementation, and every behavior that
  depends on I/O MUST be testable through it without real filesystem, repository, clipboard or
  operating-system access. Fakes MUST live in the render-free core beside the capability they
  satisfy, as ordinary public items — following the precedent already set by the existing fake
  version-control implementation. They MUST NOT be hidden behind a compilation flag or moved to a
  separate crate: a fake that any crate's tests can reach without configuration is worth more than
  the dead code it costs, and the existing precedent should not be made inconsistent for one
  feature's convenience.
- **FR-019a**: The shell/I/O file MUST be split by the **external system each part addresses** —
  startup assembly, persistence, daemon synchronisation, subscriptions, environment-include
  resolution, operating-system theme — and MUST NOT be split by feature. This split is orthogonal
  to Tiers 1–3 and MUST be shippable independently of them.
- **FR-019b**: Capabilities MAY be supplied by dynamic dispatch. Dispatch cost is explicitly **not**
  constrained: every capability call is already gated behind real I/O — disk, a git subprocess, an
  operating-system query — whose cost exceeds an indirect call by orders of magnitude, and no
  capability is reachable from the rendering path. Threading capabilities as generic type parameters
  to preserve static dispatch is therefore NOT required, and MUST NOT be adopted at the cost of
  making the single assembly point of FR-018 harder to express.

#### Cross-feature coordination (Tier 3)

- **FR-020**: A feature's reducer module MUST mutate only its own feature's data.
- **FR-021**: Any consequence affecting another feature MUST be returned as an explicit outcome
  value rather than applied directly. Feature reducers with no cross-feature consequence MUST NOT
  be required to carry an outcome vocabulary.
- **FR-022**: The root reducer MUST be the component that interprets outcomes and routes them to
  the features they concern.
- **FR-023**: The worktree-delete path MUST no longer write directly to session or overlay state,
  and MUST instead return outcomes describing those consequences.
- **FR-024**: Outcome interpretation MUST terminate and MUST NOT depend on the order in which
  feature modules are composed.
- **FR-024a**: FR-020 MUST be enforced by a guard test that names the offending path on failure,
  not by making cross-feature state unrepresentable — cross-feature *reads* for display must remain
  available (FR-003a).

#### Behavior preservation and migration

- **FR-025**: No observable application behavior may change: not visual layout, keybindings,
  dismissal behavior, animation timing, notification behavior, or any user-facing text. "Observable"
  means **user-visible behavior**, not runtime cost: this requirement sets no performance budget and
  imposes no measurement obligation (see FR-019b). Animation *timing* is named above because it is
  visible, and remains in scope.
- **FR-026**: The persisted-state format MUST NOT change. Files written before this change MUST
  load and behave identically after it.
- **FR-027**: The entire existing test suite MUST pass with no assertion modified. Tests may be
  *added*; existing expectations may not be relaxed, rewritten or deleted to accommodate the
  restructuring. Tests MAY be **relocated** — moved to a different file, including out of an inline
  test module and alongside the code they cover — provided each relocated assertion arrives
  unchanged. Relocation is not modification. This is not a loophole but a necessity: a quarter of
  the shell file is an inline test module, and those tests must travel with their subjects for the
  file to be split at all. Relocation is the **only** admitted exception: an assertion that turns
  out to encode a latent bug is still frozen (see Edge Cases), because a rule with one justified
  exception no longer supports the inference this feature depends on — that a red suite means the
  restructuring broke something. **Mechanism renames are not a further exception** — see Q3 under
  Resolved Decisions, which refuses one and says why. FR-027 binds changes made *for this feature*;
  it does not freeze the suite against every other feature shipping concurrently.
- **FR-028**: The migration MUST be expressible as incremental steps that can each ship
  independently, every one leaving the application buildable, runnable and green. (Determining the
  actual sequence is out of scope for this specification.)
- **FR-029**: The architecture MUST remain within the mandated model-view-update shape and MUST
  NOT adopt a retained-view model that works against the GUI framework.
- **FR-030**: Shared user-interface components MUST retain their mandated chainable builder API
  terminating in conversion to an element.

### Key Entities

- **Feature module** (Tier 1): One module holding a feature's type or types together with every
  helper function that operates on them. The default unit of organization. Has no message
  vocabulary of its own.
- **Feature reducer module** (Tier 3): The arms of the root reducer belonging to one feature,
  extracted into their own module and operating on the shared state. Does not imply a nested
  state or message type.
- **Nested unit** (Tier 3, conditional): A feature module that additionally owns its own state,
  message vocabulary and reducer, warranted only for a feature with an independent lifecycle. Zero
  such units is an acceptable outcome.
- **Feature outcome**: An explicit value returned by a feature reducer module describing a
  consequence that falls outside that feature's own data. The only sanctioned channel for
  cross-feature *writes*; cross-feature reads for display do not use it.
- **Layer / floating surface**: The uniform representation of any transient surface drawn over the
  base view — modal or popover alike. Knows how to render itself, what dismisses it, and which
  band of the stacking order it belongs to.
- **Overlay registry** (Tier 2): The single registration point through which a new floating surface
  becomes known to the generic dispatch.
- **Service capability (port)**: A narrow, single-purpose declaration of an I/O need, stated by the
  render-free core and satisfied by either a real implementation supplied by the binary or a fake
  supplied by a test.
- **Composition shell**: The thin layer that owns composition, message routing, outcome
  interpretation, and the supply of concrete capabilities. Holds no feature logic of its own, and
  is itself divided by external system rather than by feature (FR-019a).

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: Adding a new floating surface — modal or popover — touches exactly one new module
  plus at most one registration line, and the count of central match statements a new surface must
  be added to is **zero**, down from six. Verified by a **permanent guard test**, not by a one-time
  file count: the guard MUST fail if any registered surface becomes reachable from anywhere beyond
  its own module and the single registration point.
- **SC-002**: Adding a new feature touches exactly one new module plus at most one registration
  line, with zero edits to any other feature's module. Verified by the same permanent guard
  mechanism as SC-001.
- **SC-002a**: The guards behind SC-001 and SC-002 MUST remain in the suite after this feature
  ships. A one-time measurement proves the property on the day it is taken; only an executable
  guard keeps it true. This matches how every other architectural line in this codebase is held.
- **SC-003**: Neither of the two files that today hold the monolithic state/reducer and the
  shell/I/O remains among the largest source files in the repository, and neither contains logic
  belonging to more than one feature. **This, per FR-005, is the criterion.** As an indicative
  figure, both files are expected to land below roughly 500 lines — but the line count is a
  progress signal, not a gate. A file that contains exactly one feature and is no longer among the
  largest satisfies this criterion at any length. A module MUST NOT be split into arbitrary halves
  to cross a numeric threshold: doing so would make the codebase worse while scoring the criterion
  green, which is the opposite of what this feature is for.
- **SC-004**: Every feature module has at least one test that constructs only that feature's types
  and exercises only its own operations.
- **SC-004a**: For every feature named in FR-001, the plan records whether it became a feature
  module plus reducer module or a nested unit, with the lifecycle evidence for the choice. A count
  of zero nested units satisfies this criterion.
- **SC-004b**: Tiers 1, 2 and the shell split are each demonstrated green with no part of Tier 3
  merged.
- **SC-005**: Every declared service capability has a fake implementation and at least one test
  that exercises real behavior through it, with zero real filesystem, repository, clipboard or
  operating-system access.
- **SC-006**: The complete pre-existing test suite passes with zero assertions modified, on all
  three supported platforms.
- **SC-007**: Zero direct writes from one feature's reducer into another feature's data remain; the
  worktree-delete path in particular reports its session and overlay consequences as outcomes.
  Adding such a write fails a guard test that names the path.
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
- **One screen, no pages.** The application presents a single screen. This is the load-bearing
  premise behind the structural stance: it is why nesting is the exception rather than the default,
  and why the isolation requirements are enforced on writes by guard test rather than on reads by
  type. Should the application later grow genuine pages, the Tier 3 conditional exists to promote
  them without re-specifying this feature.
- **Sequencing within tiers is deferred.** FR-028 requires that an incremental sequence *exist*;
  the tier ordering above fixes the coarse shape, but producing the step-by-step sequence is the
  planning phase's job, not this specification's.
- **Documentation.** This change is not user-facing, so Constitution Principle VII's user-guide
  obligation is satisfied by architectural documentation rather than a user-guide entry.

## Resolved Decisions

Two decisions materially change the size and shape of this work. Both arose because the original
description predates the client/core/daemon split that the codebase has since adopted. Both are
now resolved. A third (Q3) arose later, from the assertion-freeze gate, and is resolved below.

- **Q1 (scope boundary) — Resolved: the session daemon is out of scope.** The daemon has its own
  large files (a 1,483-line server and a 1,317-line state module as of 2026-08-07, both still
  growing — the server gained 166 lines in the nine days since the first reading), but it is not a
  model-view-update application, so neither the structural stance nor any
  of FR-001 through FR-024a describes it. Restructuring it is a separate feature with separate
  reasoning; including it here would roughly double the work while sharing none of the criteria.
  This feature covers the client and the render-free core only.
- **Q2 (residence) — Resolved: feature modules live in the client, alongside their views.** The
  render-free obligation is already met — as of 2026-08-07 the monolithic state file names the
  rendering framework in four places, all of them comments and none of them code, and is exercised
  by a 71-file, feature-named client test suite. Moving feature modules into the core would split each feature across two
  crates for a test-speed gain that is largely already realized, and would separate a type from its
  own operations, which FR-001a forbids. The core keeps the **domain** model (worktree, session,
  workspace, settings, protocol) and the declared service capabilities of FR-015; the client keeps
  application state, feature modules and views.
- **Q3 (mechanism renames) — Resolved: FR-027 does not admit them, and the question rested on a
  false premise.** Raised 2026-08-10 by issue #146 and left open there deliberately, since the
  change that prompted it was the asker's own. Asked: should the freeze treat `x.field` →
  `x.field()` as mechanism rather than expectation, the way `norm()` in
  `scripts/check-assertions-frozen.sh` already strips module paths? **No**, for three reasons, the
  first of which is sufficient on its own.

  1. **`()` is not mechanism.** A module path can be stripped safely because it carries no truth
     value: `app::State::foo` and `State::foo` denote the same thing by construction. Adding `()`
     swaps a stored fact for a computed one, and the computation is arbitrary — `assert!(s.ready)`
     → `assert!(s.ready())` reads as a rename whether `ready()` is a faithful predicate or `true`.
     The waiver cannot distinguish them, so it would readmit exactly the class of defect #146 was
     filed to close: an expectation change wearing an edit that obviously isn't one.
  2. **The motivating change is its own counterexample.** Feature 023 replaced a stored
     `terminal_focused: bool` with a four-clause derived predicate that additionally requires
     `focused_field.is_none()` and that no surface takes the keyboard. Field and method are *not*
     the same proposition; they agreed at ten of the twelve affected sites, and that agreement is
     the **result** 023 had to establish, not a fact recoverable from the text. At the other two
     they disagree — which is the point of the feature. If `()` were mechanism, 023 would have been
     a no-op refactor. It was a behavior change with its own specification.
  3. **The gate was right and the report was already adjudicable.** Run from a base predating 023,
     the whole-file check reports twelve losses: ten with a 96–99% surviving counterpart printed
     beside them (a reader settles each at a glance) and two flagged "no near match survives" —
     precisely the two deliberate reversals. A waiver would have auto-passed the ten *and* said
     nothing about the two. Noise suppression that also suppresses the signal is not an improvement.

  **The real defect is scope, not content.** FR-027 freezes the suite for the duration of *this*
  feature. Feature 023 is a different feature with its own spec and its own right to change what
  its expectations say, so those twelve were never FR-027 violations — the gate flagged them
  because `.github/workflows/ci.yml` runs it on every non-docs-only change on every branch, while
  the rule it enforces is scoped to one feature. That mismatch is harmless while the job is
  advisory and becomes a merge blocker for every concurrent feature the moment it is promoted; see
  T074, which is now conditioned on fixing it.
