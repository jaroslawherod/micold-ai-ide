# Feature Specification: Component Showcase Gallery

**Feature Branch**: `feat/component-showcase-gallery`

**Created**: 2026-07-28

**Status**: Closed

**Input**: User description: "A component showcase gallery for the shared UI library, shipped as a second binary in the micold-client crate. Every visual acceptance criterion in this project is verified by a human walking the running IDE, and those walkthroughs get skipped. The showcase renders every component the shared library provides, across every interaction state, variant and density, in both schemes, on one screen — with no daemon, no git repository and no application state required. Completeness must be enforced rather than trusted. Zero change to the IDE binary. Lands before feature 018's implementation begins."

## Clarifications

### Session 2026-07-28
- Q: FR-003 requires an instance at each density step a component honours, but no density scale exists yet — it is [feature 018](../018-material3-visual-system/spec.md)'s FR-026b, and this feature lands first. What does the clause mean at delivery? → A: **Keep it, marked explicitly dormant.** No component honours a density step today, so the clause is satisfied vacuously at delivery; it is stated now because the gallery is precisely where a density scale gets reviewed once it exists. 018 carries a recorded obligation to add a density row per component when it introduces the axis, and the completeness check can enforce the clause the moment the axis exists. Introducing the axis here was rejected as scope creep against FR-019.
- Q: The spec never mentions motion, and the completeness check's inherited definition of a component ("converts into an element") structurally excludes the library's animation helpers, which are free functions — so the gallery could omit motion entirely without failing. Is motion covered? → A: **Yes, as a named second category.** The gallery gets a motion section demonstrating each animation helper on a replayable trigger, and the completeness check covers animation helpers under their own two-way rule rather than inheriting a definition that happens to exclude them. A gallery silently missing a whole category is the failure FR-012 exists to prevent, and 018's Motion walkthrough is one of the passes this feature is meant to make cheap.
- Q: FR-023 requires no frames at rest, but 018 introduces an indeterminate indicator that runs continuously while visible — and 018's FR-039d calls such an indicator a defect when no operation is in flight, which is always true in a gallery. How is it shown? → A: **Behind an explicit run control.** Any component whose appearance runs continuously is stopped at rest and requests no frames; the developer starts it and watches for as long as they want. The developer's request is the operation in flight, so 018's FR-039d holds with no exemption and FR-023 stays literally true. This generalises the replay control FR-007b already establishes for motion, and avoids the static approximation FR-004 refuses elsewhere.
- Q: SC-008 — that the installable package contains no showcase — was assigned to manual inspection, in a feature whose premise is that manual passes get skipped. Gate or inspection? → A: **Gate it.** A check reads the packaging manifest and the desktop entry and fails if either names the showcase, in the same source-scanning shape as 017's existing gates. SC-008 moves to the automated list. This is the requirement with the worst failure mode in the feature — getting it wrong ships a developer tool to end users — and the artifacts involved are declarative text nobody re-reads, so it is both the cheapest thing to automate and the least safe thing to leave to a human.
- Q: The Assumptions claim the showcase must compile *and run* on all three platforms, but nothing verifies the running half — CI cannot launch a GUI headlessly — and Principle VI makes this the feature's definition of done. What is actually required? → A: **Compilation on all three, gated; no per-platform appearance claim.** The existing workspace build already enforces the compile, and the feature's checks run on all three. The showcase is a development tool no user installs, so its appearance carries no user-facing parity obligation; parity of the components it displays belongs to the features that own them. Requiring a recorded launch on three machines would add a ritual that protects nothing.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Every component is visible without running the IDE (Priority: P1)

A developer wants to see what a component looks like. Today that means launching the application, letting it spawn a session daemon, opening a project, creating or selecting a worktree, and then finding a screen that happens to use the component. If the component only appears in an error state or an empty state, they must first produce that state.

With the showcase, they launch one command and see the whole library on a single scrolling page — every component, grouped and labelled, rendered against the same design tokens the application uses. No daemon, no git repository, no saved application state, no project.

**Why this priority**: This is the entire premise. Everything else in this feature refines a gallery that must first exist, and it is the change that converts "look at the component" from a multi-minute setup into a single command.

**Independent Test**: On a machine with no configuration for this application and no git repository present, launch the showcase. Confirm it opens, renders, and that no session daemon process was started.

**Acceptance Scenarios**:

1. **Given** a machine with no saved application state and no project configured, **When** the developer launches the showcase, **Then** it opens and renders the full component catalogue without error.
2. **Given** the showcase is running, **When** the developer inspects the running processes, **Then** no session daemon has been started and no terminal session exists.
3. **Given** the showcase is running, **When** the developer scrolls the page, **Then** every component in the shared library appears, each under a heading naming it.
4. **Given** a component that the application only shows in a rare state (an error notice, an empty list, a disconnected banner), **When** the developer views the showcase, **Then** that component is present without the developer having to reproduce the state that triggers it.

---

### User Story 2 - Every state a component can be put into is shown side by side (Priority: P2)

A developer checking that a component looks right in all its states does not want to hunt for one instance of each. The showcase places a component's configurable states next to each other on the same row — every variant, every density step, enabled and disabled, selected and unselected — so a difference between them is visible by comparison rather than by memory.

The states a pointer produces — hover, pressed — and the state the keyboard produces — focus — cannot be posed. They are exercised live: the developer moves the pointer along a row of real, interactive components and watches each respond.

**Why this priority**: This is what makes a visual acceptance criterion cheap to check. Feature 018 asks for a hover and a pressed state on *100% of interactive elements*; a row of every interactive component in the library is the only place that can honestly be confirmed in one pass.

**Independent Test**: Pick any component with more than one variant. Confirm all its variants render on one screen simultaneously. Move the pointer across the row and confirm each responds; press and hold each and confirm a stronger response.

**Acceptance Scenarios**:

1. **Given** a component with multiple variants, **When** the developer views its section, **Then** every variant is rendered simultaneously and each is labelled.
2. **Given** a component that can be disabled, **When** the developer views its section, **Then** both an enabled and a disabled instance are rendered side by side.
3. **Given** a component that can be selected, **When** the developer views its section, **Then** both a selected and an unselected instance are rendered side by side.
4. **Given** a component that honours the density scale, **When** the developer views its section, **Then** an instance is rendered at each density step it supports.
5. **Given** any interactive component in the gallery, **When** the developer moves the pointer onto it, **Then** it responds exactly as the same component responds inside the application, because it is the same component.
6. **Given** a component section, **When** the developer reads its label, **Then** the label names which states are posed as separate instances and which must be exercised with the pointer or keyboard, so nothing is assumed to be missing when it is merely live.

---

### User Story 3 - Light and dark are comparable without a restart (Priority: P3)

A developer checking a colour decision switches the showcase between the light and the dark scheme from within the showcase itself and watches every component re-render. They do not restart it, and they do not change their operating system's theme setting to do it.

**Why this priority**: Scheme parity is a standing obligation on every component, and a scheme bug is far easier to see when the two renderings are seconds rather than minutes apart. It sits after states because a component must first be on screen in all its configurations before comparing two schemes of it means anything.

**Independent Test**: Launch the showcase, note the appearance of several components, switch the scheme from the showcase's own control, and confirm every component on the page re-renders in the other scheme with no restart.

**Acceptance Scenarios**:

1. **Given** the showcase is running in one scheme, **When** the developer activates the scheme control, **Then** every component on the page re-renders in the other scheme without a restart.
2. **Given** the showcase has been switched to the other scheme, **When** the developer scrolls to a section that was off screen at the time of the switch, **Then** it is also rendered in the new scheme.
3. **Given** the showcase is running, **When** the developer inspects the colours a component resolves, **Then** they are the same colours that component resolves inside the application in that scheme.

---

### User Story 4 - The gallery cannot fall out of date unnoticed (Priority: P4)

A developer adds a new component to the shared library and forgets to add it to the showcase. The build fails and tells them which component is missing. Another developer deletes a component; the build fails and tells them the gallery still lists something that no longer exists.

**Why this priority**: A catalogue that silently omits things is worse than no catalogue, because it is consulted as though it were complete. This is what makes the showcase trustworthy enough to verify against, and it is scoped last only because there must be a gallery before there is anything to hold complete.

**Independent Test**: Add a component to the shared library without adding it to the gallery and confirm the build fails naming it. Separately, remove a component that the gallery lists and confirm the build fails naming it.

**Acceptance Scenarios**:

1. **Given** a component exists in the shared library, **When** it has no entry in the gallery, **Then** the build fails and names the missing component.
2. **Given** the gallery lists an entry, **When** the component it names no longer exists in the library, **Then** the build fails and names the stale entry.
3. **Given** a component exposes named variants, **When** any variant has no instance in the gallery, **Then** the build fails and names the missing variant.
4. **Given** the shared library is moved or renamed, **When** the completeness check runs, **Then** it fails rather than reporting success over an empty set.
5. **Given** an animation exists in the shared library, **When** it has no entry in the motion section, **Then** the build fails and names it — and likewise when a motion entry names an animation that no longer exists.

---

### Edge Cases

- **A component cannot be rendered without data the showcase has no access to** — one that displays a live terminal grid, or a list of real worktrees. The showcase must render it with fixed, invented sample content rather than omitting it, and the sample content must be part of the gallery rather than something the developer supplies at launch.
- **A component is a floating surface** — a dialog, a menu, a popover — that covers the page when open. It must be presented in a way that lets the rest of the gallery stay reachable: openable from its section and dismissible without leaving the page.
- **Two floating components could be opened at once.** The gallery must not deadlock itself into a state where a surface cannot be dismissed and the page cannot be scrolled.
- **A component's natural size is far larger than its neighbours** — a full-width banner beside a small chip. The layout must not let one oversized component push the rest off screen or force horizontal scrolling of the whole page.
- **The window is resized very narrow.** The gallery must reflow or scroll rather than clipping component instances out of view, since a clipped instance reads as a missing one.
- **A component is added to the library that has no visible appearance of its own** — a layout or behaviour helper with nothing to look at. The completeness check must have a recorded way to mark it as having no gallery entry, and that exemption list must fail when an entry on it no longer exists, in the same way the gallery itself does.
- **The showcase and the application disagree.** If a component looks different in the two, that is a defect in the showcase, never a licence to style the gallery's copy differently — the two must resolve the same component and the same tokens.

## Requirements *(mandatory)*

### Functional Requirements

#### What the gallery contains

- **FR-001**: The showcase MUST present every component the shared UI component library provides, each under a heading naming it.
- **FR-002**: Each component MUST be rendered as a live, interactive instance — the same component the application renders, resolving the same design tokens — never as a picture, a mock-up, or a gallery-local copy.
- **FR-003**: For each component, every state that can be **posed** — that is, set through the component's own configuration — MUST be rendered as a separate instance, with all of a component's posed instances visible together. This covers at minimum: each named variant, enabled and disabled, and selected and unselected, wherever the component admits that state.
- **FR-003a**: Each density step a component honours MUST likewise be rendered as a separate instance. **This clause is dormant at delivery**: no component honours a density step today, because the density scale is introduced by [feature 018](../018-material3-visual-system/spec.md) (its FR-026b) and this feature lands first. It is stated now because the gallery is where such a scale would be reviewed, and because the completeness check should enforce it from the moment the axis exists rather than waiting for someone to remember. The sidebar's existing reduced-density type roles are a separate, older decision and are not density steps in this sense.
- **FR-004**: The states that cannot be posed — hover and pressed, which follow the pointer, and focus, which follows the keyboard — MUST be exercisable directly on the rendered instances. The gallery MUST NOT fake them with static approximations, because an approximation that drifts from the real state layer is worse than no swatch at all.
- **FR-005**: Each component's section MUST state which of its states are posed as separate instances and which must be exercised live, so that a state absent from the page is understood as live rather than missing.
- **FR-006**: Components that require content the showcase cannot obtain — live session output, real repository data — MUST be rendered with fixed sample content defined inside the gallery. No component may be omitted on the grounds that it needs data.
- **FR-007**: Floating components — dialogs, menus, popovers, and any other surface that covers what is beneath it — MUST be openable from their own section and dismissible without leaving or reloading the page.

#### Motion

- **FR-007a**: The showcase MUST include a **motion section** presenting every animation the shared library provides — the transition helpers a feature module wraps content in, and any component whose appearance is itself an animation.
- **FR-007b**: Each animation MUST be **replayable on demand** from its own entry, as many times as the developer wants, without reloading the page or waiting for an unrelated state change. An animation that can only be seen by catching it once is not meaningfully reviewable.
- **FR-007c**: Each animation's entry MUST name the animation, so that a developer comparing against a motion specification knows which one they are watching.

#### Theming

- **FR-008**: The showcase MUST offer a control that switches between the light and the dark scheme, applying to every component on the page.
- **FR-009**: Switching the scheme MUST NOT require a restart, and MUST NOT require changing the host system's theme preference.
- **FR-010**: The showcase MUST resolve colours through the same design tokens the application resolves. It MUST NOT define a palette, a role, or any styling value of its own.

#### Completeness

- **FR-011**: A build-time check MUST fail when a component exists in the shared library and has no instance in the gallery, and the failure MUST name the component.
- **FR-012**: The same check MUST fail when the gallery names a component that no longer exists in the library, and MUST name the stale entry. A catalogue that outlives its contents misleads in the opposite direction and must fail just as loudly.
- **FR-013**: The check MUST fail when a component's named variant has no instance in the gallery, and MUST name the missing variant.
- **FR-013a**: The check MUST cover the library's **animation helpers as a named second category**, under the same two-way rule: an animation with no entry in the motion section fails the build, and a motion entry naming an animation that no longer exists fails the build. This category MUST be enumerated deliberately rather than inherited, because the component definition of FR-014 recognises things that convert into an element and therefore does not see a helper offered as a plain function — leaving motion outside the check by accident, which is the silent omission FR-012 exists to prevent.
- **FR-014**: For components, the check MUST use the **same definition of "a component"** that the existing component-API gate uses, so the two cannot disagree about what the library contains. A change to that definition MUST take effect in both at once. What that definition does *not* reach is covered by FR-013a rather than left unstated.
- **FR-015**: Components with no visible appearance of their own MAY be exempted from FR-011 through a recorded exemption list. Each entry MUST carry the reason it cannot be shown. The list MUST fail when an entry names something that no longer exists, on the same reasoning as FR-012.
- **FR-016**: The check MUST fail rather than pass if it finds no components at all — for example because the library moved — so that a relocation cannot be mistaken for a clean result.

#### Isolation from the application

- **FR-017**: The showcase MUST be a separate program from the application. Launching one MUST NOT launch the other.
- **FR-018**: The showcase MUST NOT be included in the installable package, the desktop entry, or the installed launcher. It is a development tool and MUST NOT reach an end user through a normal installation.
- **FR-018a**: FR-018 MUST be enforced by a build-time check rather than by inspection: the check reads the packaging manifest and the desktop entry and fails when either names the showcase. Both are declarative files that no one re-reads once written, and shipping a development tool to end users is the worst outcome this feature can produce, so it is the last thing that should depend on somebody remembering to look.
- **FR-019**: The application's appearance and behaviour MUST be unchanged by this feature. Any visible or behavioural difference in the application is a defect.
- **FR-020**: The showcase MUST run without a session daemon, without a git repository, and without any saved application state, and MUST NOT create, read or modify any of them.
- **FR-021**: The showcase MUST NOT become a second implementation of anything. It composes existing components and supplies sample content; it MUST NOT contain styling, layout rules, or interaction behaviour that belongs in the component library. Where the gallery reveals that something is missing from the library, the fix is to add it to the library.

#### Determinism

- **FR-022**: The gallery's content MUST be fixed: the same components, the same sample data, and the same ordering on every launch. Nothing may vary with the clock, with random values, or with the machine it runs on.
- **FR-023**: At rest the showcase MUST request no frames and consume no measurable CPU, on the same terms as the application, and MUST honour the same single sanctioned frame-request path. It is not exempt from the guarantees the library already carries.
- **FR-023a**: A component whose appearance **runs continuously** rather than settling — an indeterminate progress indication being the case this feature will meet first — MUST be posed behind an explicit **run control**. At rest it is stopped and requests no frames; the developer starts it and it runs for as long as they watch. The developer's request is what stands in for the operation such an indication normally reports on, so the component is never displayed running with nothing running, and FR-023 needs no exemption for it. This is the same mechanism as FR-007b's replay trigger, applied to appearance rather than to transitions.

#### Documentation

- **FR-024**: The showcase MUST be documented for developers — what it is for, how to launch it, and how to add a component to it — in the same change. The user guide MUST NOT be extended to describe it, because it is not a user-facing capability.

### Key Entities

- **Gallery section**: One component's place in the showcase — its name, its posed instances, the sample content it needs, and the note recording which of its states are live rather than posed.
- **Posed instance**: One rendering of a component in a specific configuration — a named variant, a density step, disabled, or selected — shown alongside its siblings for comparison.
- **Sample content**: Fixed, invented data standing in for the real content a component would display in the application. Belongs to the gallery, not to the component.
- **Motion entry**: One animation's place in the motion section — its name and the trigger that replays it on demand.
- **Exemption entry**: A recorded statement that a named library component has no gallery instance, together with the reason. Valid only while the component it names exists.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: A developer with a clean machine — no configuration for this application, no project, no repository — can go from launching the showcase to looking at any named component in under 30 seconds, with one command and no setup steps.
- **SC-002**: 100% of the components in the shared library appear in the gallery, or appear on the exemption list with a recorded reason. The count is proven by a check that fails the build, not by inspection.
- **SC-003**: 100% of components' named variants have an instance in the gallery, proven by the same check.
- **SC-003a**: 100% of the animations the shared library provides have an entry in the motion section, each replayable on demand, proven by the same check under its own category (FR-013a).
- **SC-004**: Adding a component to the library without adding it to the gallery fails the build, and the failure message names the component. Deleting a component the gallery lists fails the build, and the message names the entry. Both directions are demonstrated by deliberately introducing each failure and observing it.
- **SC-005**: Every interactive component in the library can be hovered and pressed within one scrolling page, so that confirming a hover and a pressed state across the whole library is a single pass rather than a search.
- **SC-006**: Switching the scheme re-renders every component on the page with no restart, and the resulting colours match what the same component resolves in the application in that scheme.
- **SC-007**: The application binary is byte-for-byte unaffected in appearance and behaviour: its existing test suite passes unchanged, and its style parity snapshot is unchanged.
- **SC-008**: The installable package contains no showcase binary, no showcase desktop entry, and no showcase launcher entry, proven by a check that fails the build when the packaging manifest or the desktop entry names the showcase (FR-018a).
- **SC-009**: With the showcase open and idle — every replay and run control stopped — zero frames are requested and CPU use is not measurably above zero over a sustained observation window. This holds with a continuously-running component's section on screen, because such a component is stopped until started (FR-023a).
- **SC-010**: Two consecutive launches of the showcase render the same content in the same order, with no differences attributable to time, randomness, or the host machine.

## Assumptions

- The showcase's audience is developers working on this repository, not end users. "User value" throughout this specification means value to that audience, and Principle VII's user-guide obligation is met by developer documentation rather than by a user-guide chapter (FR-024).
- The shared component library is already consumable from outside the application's own entry point — the existing test suite drives components directly, which is the same access the showcase needs. No extraction of the library into a separate package is required, and none is in scope.
- The existing component-API gate already defines what counts as a component in the library. FR-014 reuses that definition rather than introducing a second one that could drift from it.
- The gallery is built against whatever the component library looks like at the time. It is deliberately built *before* the Material 3 visual system lands, so it renders the pre-change appearance first and becomes a before-and-after reference at no extra cost.
- Verification splits by what can be asserted without a human judging pixels. **Automated**: SC-002, SC-003, SC-003a, SC-004, SC-007, SC-008, SC-009 and SC-010 — all structural or behavioural checks, including the packaging exclusion, which is a scan of two declarative files rather than an inspection (FR-018a). **Recorded manual walkthrough**: SC-001, SC-005 and SC-006 — a timing measurement and two visual comparisons, which are the only three that need a person looking at the screen.
- Cross-platform parity (Principle VI) applies to the showcase **as a build target only**: it MUST compile on Linux, macOS and Windows, which the existing whole-workspace build already enforces in CI, and this feature's own checks run on all three. No claim is made about its *appearance* on any given platform, and no recorded launch on each platform is required to call the feature done — the showcase is a development tool that no user installs, and parity of the components it displays is owned by the features that introduce them.
- No new dependency is expected. The showcase composes components that already exist using the framework already in use.

## Relationship to other features

**[Feature 017 — Material Component Architecture](../017-material-component-architecture/spec.md)** made this feature possible and constrains it. 017 drew the boundary between the component library and the code that uses it, and enforces that boundary with source-scanning gates. The showcase depends on that boundary being real: it consumes the library exactly as a feature module does. FR-014's completeness check is deliberately built in the shape of 017's existing gates, and FR-023 holds the showcase to 017's single sanctioned frame-request path rather than exempting it.

**[Feature 018 — Material 3 Visual System](../018-material3-visual-system/spec.md)** is the reason for the timing. 018 carries a large recorded manual walkthrough — its SC-002, SC-004, SC-005, SC-006 and SC-007 — because visual criteria cannot be asserted automatically. Several of those ask for a property of *every* interactive element, which is exactly the kind of exhaustive manual pass that gets skipped; 017's own convergence pass found that its equivalent check had never been run. This feature should land **before** 018's implementation begins, so that walkthrough is done against one page rather than by navigating the application, and so the pre-change appearance is on record before any token value changes.

That ordering leaves 018 holding one obligation back to this feature: 018 introduces the density scale (its FR-026b), and FR-003a here is dormant until it does. When 018 adds the axis, it MUST add a density row per honouring component to the gallery in the same change, at which point FR-003a stops being vacuous and the completeness check can hold it.

**[Feature 019 — Layout Snapshot Parity](../019-layout-snapshot-parity/spec.md)** is expected to benefit but is not delivered here. 019 pins resolved widget bounds as a committed fixture; the application's layout depends on how many worktrees exist and what is open, whereas the gallery's content is fixed by FR-022. That makes the gallery a better subject for such a fixture. Building that fixture is 019's work, not this feature's — this feature only guarantees the determinism that would make it possible.

## Out of Scope

- Extracting the component library into a separate package. The library is already consumable as it stands; a package boundary would break the path-based gates 017 relies on and delivers nothing this feature needs.
- Any change to the application's appearance or behaviour.
- Shipping the showcase to end users, in any package or installer.
- Automated visual comparison of rendered output — image diffing, screenshot baselines, or perceptual comparison. The showcase makes human comparison cheap; it does not replace it.
- Building feature 019's layout snapshot fixture, even though this feature is intended to make it easier.
- Documenting the component library's API. The showcase demonstrates appearance and behaviour; reference documentation is a separate concern.
- Any editing, theming, or configuration capability in the showcase beyond switching the colour scheme. It is a catalogue, not a design tool.
