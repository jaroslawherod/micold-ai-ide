# Feature Specification: Material Component Architecture

**Feature Branch**: `feat/improve-material-design`

**Created**: 2026-07-27

**Status**: Draft

**Input**: Split from the Material 3 Visual System specification on 2026-07-27, per that spec's clarification session. This feature carries the **structural** half; the visual half is [`specs/018-material3-visual-system/`](../018-material3-visual-system/spec.md), which depends on this one.

**Defining property**: this feature ends with **zero visual change**. The application looks exactly as it does today and the test suite is green. That is what makes it reviewable as a single question — "did anything change?" — rather than a mixture of refactor bugs and design decisions.

## Why this exists

The application's UI is built from a shared component library, but the boundary leaks badly. Thirteen feature modules construct rendering widgets directly and apply styling themselves at 119 call sites; 135 sites select a raw text size. Every one of those is free to render a slightly different button, and many do.

That is why the design system drifts, why press feedback and state layers are applied unevenly, and why five separate floating-surface implementations have grown apart in elevation, corner and dismissal behavior. Restyling on top of that foundation would mean applying every visual decision at 119 places and hoping none is missed.

This feature closes the boundary first. Afterwards, changing how something looks is an edit in one place.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - A developer changes an appearance in one place (Priority: P1)

A developer needs to adjust how buttons look. They edit the button component and every button in the application changes — because no feature module is able to construct or style a button itself.

**Why this priority**: This is the whole point of the feature. Every later visual change depends on it being true.

**Independent Test**: Change a visible property of a shared component (say, its corner radius) and confirm every instance in the application changes, with no feature module edited.

**Acceptance Scenarios**:

1. **Given** the component library, **When** a developer changes a shared component's appearance, **Then** every instance in the application reflects it without any feature module being modified.
2. **Given** a feature module, **When** a developer tries to construct a styled rendering widget directly, **Then** the build fails.
3. **Given** a feature module, **When** a developer tries to apply a style function directly, **Then** the build fails.
4. **Given** the application after this change, **When** a user runs it, **Then** it looks exactly as it did before.

---

### User Story 2 - Adding an animated element costs nothing globally (Priority: P2)

A developer adds a component that animates. They write it and use it. They do not add a variant to a global enumeration, do not add a field to application state, and do not thread a progress value through the view.

**Why this priority**: The current central-animator pattern makes every animated element a change to shared application state, which is why every animated element today costs a change to shared state.

**Independent Test**: Add a trivially animated component to a screen and confirm no application-state or shared-enumeration change was required.

**Acceptance Scenarios**:

1. **Given** a new animated component, **When** it is added to a screen, **Then** no global enumeration and no application-state field is modified.
2. **Given** two instances of the same animated component, **When** one animates, **Then** the other is unaffected.
3. **Given** an animated component, **When** it is removed from the view, **Then** its animation state goes with it and nothing continues to animate.
4. **Given** the application at rest, **When** no interaction occurs, **Then** no frames are requested and no measurable CPU is used.

---

### User Story 3 - Floating surfaces behave alike (Priority: P3)

A developer opens a menu, a context menu, the project-switcher popover, the select dropdown and a dialog. Each dismisses the same way — outside click, Escape — rather than each having its own rules.

**Why this priority**: Five independent implementations are why these already differ. Consolidating them is what lets one later change apply to all of them.

**Independent Test**: Open each floating surface in turn and confirm consistent dismissal, and that only one overlay implementation exists.

**Acceptance Scenarios**:

1. **Given** any non-modal floating surface, **When** the user clicks outside it, **Then** it dismisses.
2. **Given** any non-modal floating surface, **When** the user presses Escape, **Then** it dismisses.
3. **Given** any non-modal floating surface, **When** the content beneath it scrolls, **Then** it dismisses.
4. **Given** a dialog, **When** the user presses Escape or clicks the scrim, **Then** it dismisses — unless it is explicitly non-dismissible because losing its input would destroy work.
5. **Given** two floating surfaces open at once, **When** the user views them, **Then** they stack in a defined order.

---

### Edge Cases

- **A component is removed while animating** — a menu item pressed as its menu closes. Its state must go with it; nothing may continue to animate.
- **The same component appears many times** — dozens of sidebar rows. Each must hold its own state without the count degrading performance or requiring per-instance registration.
- **A dialog that must not be dismissed accidentally** — one holding unsaved input. It must be declarable non-dismissible while every other surface follows the unified rule.
- **A feature module legitimately needs a layout-only widget.** Rows, columns and spacers must remain directly usable; the boundary is appearance, not all widgets.
- **A shared component needs a genuinely new capability** — a caller wants something the wrapper cannot express. The wrapper gains the capability; the call site must not be able to bypass the library instead.
- **State that looks presentational but is persisted.** Sidebar width and expanded tree nodes survive restarts. They must stay with the application even though they feel like view concerns.

## Requirements *(mandatory)*

### Functional Requirements

#### The library wraps the rendering stack

- **FR-001**: The shared component library MUST **wrap** every rendering-stack widget that carries a Material appearance — at minimum buttons, text, text inputs, styled containers and surfaces, checkboxes, scrollables, progress indicators and the select control — and expose each through the established chainable builder API. A feature module MUST NOT construct one of those widgets directly.
- **FR-002**: Feature modules MUST NOT apply styling themselves. The conversion from design tokens to rendering styles MUST be reachable only from inside the component library, so a call site is structurally unable to render a differently-styled variant of a shared component.
- **FR-003**: Pure layout primitives that carry no Material appearance — rows, columns, spacers, stacks and pointer-area wrappers — are exempt and may be used directly. The boundary is appearance: if a widget has a Material specification it is wrapped; if it only positions other widgets it is not.
- **FR-004**: Conformance with FR-001 and FR-002 MUST be verified by an automated test that fails the build when a feature module imports a styled rendering widget or applies a style function, so the boundary cannot erode as new code is written.
- **FR-005**: Every wrapper MUST reproduce the **current** appearance of what it replaces. This feature changes no visual property; it changes only where that property is decided.

#### Layer split

- **FR-006**: The library MUST be split into two layers, mirroring the established separation between a component *development kit* and the *styled component set* built on it. The lower layer provides unstyled behavior primitives — overlay positioning, backdrop handling and dismissal — with no Material appearance. The upper layer applies appearance to them.
- **FR-007**: A behavior primitive MUST NOT hard-code an appearance value, and a styled component MUST NOT re-implement a behavior the lower layer provides.

#### One overlay

- **FR-008**: All floating surfaces MUST be built on **one** overlay primitive owning positioning, backdrop, dismissal and stacking order. The application currently has five independent implementations — the modal, the overflow menu, the context menu, the project-switcher popover and the select dropdown — which is why their behavior has diverged.
- **FR-009**: Dismissal MUST be unified rather than configured per surface. Every non-modal floating surface MUST dismiss on outside click, on Escape, and when the content beneath it scrolls. Modal dialogs MUST dismiss on Escape and on scrim click, unless explicitly declared non-dismissible because losing input would destroy work.
- **FR-010**: Overlapping floating surfaces MUST render in a defined stacking order.

#### Component-owned state

- **FR-011**: A shared component MUST own its own **presentation state** — hover progress, press feedback, expand/collapse progress and any other purely visual transient. It MUST NOT depend on the application holding that state, and MUST NOT require the application to allocate an identity for it.
- **FR-012**: The application retains ownership of **logical** state. The dividing line: if the state would still matter with animation disabled it is the application's; if it exists only to make a transition look right it is the component's.
- **FR-013**: A component's public API MUST NOT expose implementation details — no animation keys, animator handles, progress values, style functions, internal state types or rendering-stack types in its constructor or builder methods. A call site describes *what* it wants, never *how* it is rendered or animated.
- **FR-014**: The presentation state the application currently holds centrally MUST be refactored into the components that own it: the central animator keyed by a global element enumeration (menu fade, sidebar slide, main-view fade, resize-handle hover, overlay fade, filter-panel fade), the per-row hover-fade animator keyed by a hashed row identity, the currently-hovered row, and the resize-handle drag flag.
- **FR-015**: After this refactor, adding a newly animated element MUST NOT require adding a variant to a global enumeration or a field to application state.
- **FR-016**: Logical state MUST remain with the application and MUST NOT move: which worktrees exist, which session is active, which tree nodes are expanded, sidebar visibility and width, open-menu identity, drafts, filters and the theme preference. Several are persisted, and persistence MUST be unchanged.
- **FR-017**: Where a component's behavior involves a genuine decision — a state machine, a queue discipline, a mapping from domain signal to visual emphasis — that decision MUST be a pure function or pure data structure, unit-testable independently of rendering, so component-owned state does not put logic beyond the reach of tests.

#### Tokens move to the render-free core

> *FR-018 and FR-019 (the density scale) were moved to [018](../018-material3-visual-system/spec.md) during the 2026-07-27 analysis pass — applying a density scale assigns heights to components that are content-sized today, which would violate this feature's zero-visual-change property. The numbering gap is deliberate.*

- **FR-020**: The design tokens MUST move to the render-free core module, which declares no rendering dependency and therefore cannot name a rendering type even by accident. Only the conversion of token values into rendering types may live in the client.
- **FR-021**: The move MUST be **mechanical**: the same values, relocated. Introducing new token values is the visual feature's work, not this one's.
- **FR-022**: Token values MUST be exercised by the standard whole-workspace test run, with no renderer present.

#### Constraints

- **FR-023**: The application MUST look **exactly** the same after this feature as before it. Any visible difference is a defect.
- **FR-024**: Every user-visible behavior MUST be unchanged, with a single exception: floating-surface dismissal (FR-009), where unifying five implementations necessarily changes those that differ today. That exception covers only *how* a surface is dismissed — not what it contains, what opening it does, or what any action inside it does.
- **FR-025**: At rest the application MUST request no frames and consume no measurable CPU. Every animation MUST provably settle and release what it held.
- **FR-026**: The result MUST behave the same on Linux, macOS and Windows.
- **FR-027**: The component API contract MUST be published at `contracts/component-api.md` and MUST remain the durable reference the implementation is checked against.
- **FR-028**: Developer-facing documentation MUST be updated in the same change to describe the two layers and the rule that feature modules compose components rather than styling widgets.

### Key Entities

- **Behavior primitive**: An unstyled capability — currently the overlay — with no appearance of its own.
- **Component wrapper**: A styled component exposing a chainable builder, owning its presentation state, hiding its internals.
- **Presentation state**: A purely visual transient held per component instance. Discarded when the instance is.
- **Logical state**: Application-owned state that would still matter with animation disabled.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: Zero feature modules construct a styled rendering widget or apply a style function; the baseline being removed is 13 modules, 119 style applications and 135 raw text-size references. Proven by a test that fails the build on violation.
- **SC-002**: The application is visually identical before and after, verified by walking every screen and dialog in both light and dark schemes.
- **SC-003**: Exactly one overlay implementation exists, down from five, and every floating surface dismisses consistently.
- **SC-004**: Zero components require the application to hold their presentation state or allocate an identity for them, and zero component APIs expose an animation key, progress value, style function or rendering-stack type.
- **SC-005**: The global animated-element enumeration and the central animators are gone from application state; adding a newly animated element requires touching only that element's component, demonstrated by adding one.
- **SC-006**: Every value that survives an application restart survives identically, proven by loading state written by the pre-change build.
- **SC-007**: Every user action available before the change is available after it and produces the same result — with the single exception of floating-surface dismissal.
- **SC-008**: With the application idle, zero frames are requested and CPU use is indistinguishable from the pre-change build over a sustained window; after pressing every interactive element, no animation state remains held.
- **SC-009**: Token values are asserted from the render-free core, which has no rendering dependency, in the standard whole-workspace test run.

## Assumptions

- The rendering stack supports per-instance widget state, so components can own their presentation state without a central store.
- The render-free core cannot name a rendering type, making the token boundary a compile error rather than a convention.
- No new runtime dependency is introduced.
- No persisted data changes shape. The presentation state being moved is not persisted.
- The embedded terminal's rendering is untouched; only its surrounding chrome participates.
- The visual feature ([`018`](../018-material3-visual-system/spec.md)) begins where this one ends.

## Out of Scope

- **Every visual change.** New color values, elevation, typography, shape, state layers, the press ripple, the density scale, component anatomy and motion all belong to [`018-material3-visual-system`](../018-material3-visual-system/spec.md). This feature changes only *where decisions live*.
- Keyboard traversal between elements, and any focus model beyond what exists.
- Changes to what the application does, stores, or how it talks to git, the terminal or the agent process.
- Information-architecture changes.
