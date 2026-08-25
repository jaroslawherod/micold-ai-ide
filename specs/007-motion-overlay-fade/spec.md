# Feature Specification: Generic Motion Library & Overlay Fade In/Out

**Feature Branch**: `007-motion-overlay-fade`

**Created**: 2026-07-17

**Status**: Reopened 2026-08-25 — implemented and shipped, and the manual quickstart pass (unrun until now) found the headline claim half-broken. Overlays **enter** with a visible transition; every overlay **exit** renders in a single frame, on all three dismissal paths and for the overflow menu and main-view switch too, so FR-002 and FR-003 are not met. Measured at 60 fps — `evidence/T024-quickstart-pass.md`, filed as `bugs/BUG-001.md`. All tasks are ticked as *run*; the feature closes again when that bug is fixed and the two clauses it makes unanswerable (reveal-beneath, reopen-during-exit) can be checked.

**Input**: User description: "Generic, reusable UI animation library plus fade in/out for modal overlays. Overlays currently appear/disappear instantly; existing animations are bespoke and duplicated. Want one reusable animation mechanism any widget can use (not a static per-animation list), overlay fade in AND out (revealing the app beneath), and clearly perceptible timing (~300ms in / ~240ms out) — the current ~90ms is too fast to notice."

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Modal overlays ease in and out (Priority: P1)

When a user opens a modal dialog (About, the project selector, rename project, add worktree, or Settings), it fades smoothly into view rather than snapping on. When they dismiss it — by Cancel, by pressing Esc, or by a successful submit — it fades smoothly away, and the application content behind it becomes progressively visible again as it leaves.

**Why this priority**: This is the directly requested, user-visible improvement. It removes the abrupt "blink" of every dialog and makes the app feel considered and polished. It delivers value on its own regardless of any internal refactor.

**Independent Test**: Open and close each of the five overlays through every dismissal path (Cancel, Esc, successful submit) and confirm each one visibly fades in on open and fades out on close, with the app behind it re-appearing during the exit.

**Acceptance Scenarios**:

1. **Given** no overlay is open, **When** the user opens any of the five overlays, **Then** the overlay fades in from fully transparent to fully visible.
2. **Given** an overlay is open, **When** the user cancels it or presses Esc, **Then** the overlay fades out and the underlying app content progressively reappears until the overlay is gone.
3. **Given** an overlay with a form (rename, add worktree, Settings), **When** the user submits successfully, **Then** the overlay fades out (the same exit as Cancel), not disappears instantly.
4. **Given** a form overlay with invalid input, **When** the user submits, **Then** the overlay stays open (no exit animation) and shows the validation error, exactly as today.
5. **Given** an overlay is fading out, **When** the exit animation is mid-flight, **Then** the dialog's displayed content stays consistent with its last shown state (no abrupt content change while leaving).

---

### User Story 2 - One reusable, extractable animation library (Priority: P2)

The app's motion is driven by a single shared animation mechanism that any widget or component can use. Adding a new animated element costs only: name it, set its target, and read its progress — no element-specific state threaded through multiple files. The four existing animations (overflow-menu fade, sidebar slide, main-view fade, resize-handle hover) are moved onto this shared mechanism and keep their current feel. Crucially, the mechanism is **self-contained and free of any dependency on this application's domain**, so it can be extracted into a standalone, reusable library and used by other projects unchanged.

**Why this priority**: This is the core of the request ("make animations generic … a reusable lib cross widgets/components", and it should be extractable for use outside this project). It removes today's duplicated per-animation boilerplate, so the overlay fades (and any future motion) are cheap to add and consistent, and it turns the motion engine into a portable asset rather than app-local plumbing. It enables User Story 1 to be added without multiplying plumbing.

**Independent Test**: Review that all animations (the four existing plus the overlay fades) read from one shared mechanism; demonstrate that registering one additional animated value requires a change in a single place; and confirm the mechanism's core carries no references to application-specific types (it could be compiled/used in a different project as-is). Confirm the four existing animations look and behave as they did before.

**Acceptance Scenarios**:

1. **Given** the shared animation mechanism, **When** a developer adds a new animated element, **Then** they register a name, set a target, and read progress — with no new element-specific state added in multiple locations.
2. **Given** the migration is complete, **When** the app runs, **Then** the overflow-menu fade, sidebar slide, main-view fade, and resize-handle hover behave identically to before.
3. **Given** the shared mechanism, **When** nothing is animating, **Then** no animation work runs (idle cost is unchanged from today).
4. **Given** the animation library's core, **When** it is inspected (or lifted into another project), **Then** it references no application-specific domain types and depends only on its own generic primitives, so it is reusable outside this project without modification.

---

### User Story 3 - Perceptible, consistent, tunable timing (Priority: P3)

Overlay animations are slow enough to be clearly noticed (the previous micro-animations were too fast to perceive). The animation timing is expressed in human-legible durations so it can be reviewed and tuned, rather than as opaque numeric step values.

**Why this priority**: The user explicitly reported that a ~90ms fade "cannot be noticed." Perceptibility is what makes the feature feel real; legible timing keeps it maintainable. It refines Stories 1–2 rather than standing fully alone.

**Independent Test**: Observe that each overlay's open/close animation is plainly visible (not a single-frame flash) and lasts roughly the target durations; confirm the timing values are stated as durations that a reviewer can read and change in one place.

**Acceptance Scenarios**:

1. **Given** an overlay open action, **When** it fades in, **Then** the animation lasts approximately 300 ms and is clearly perceptible.
2. **Given** an overlay close action, **When** it fades out, **Then** the animation lasts approximately 240 ms and is clearly perceptible.
3. **Given** the timing configuration, **When** a reviewer reads it, **Then** durations are expressed in time units (e.g. milliseconds), not opaque per-frame step numbers.

---

### Edge Cases

- **Reopen during exit**: A user reopens an overlay (or opens a different overlay) while a previous overlay is still fading out — the newly opened overlay fades in cleanly and no stale/leftover overlay is shown.
- **Rapid toggle**: Quickly opening and closing an overlay repeatedly does not leave a dialog stuck partially visible or double-drawn.
- **Successful submit vs. validation failure**: Only a successful submit triggers the exit animation; a validation failure keeps the overlay open with its error, with no exit animation.
- **Quit mid-animation**: Closing the application while an overlay is mid-animation shuts down cleanly with no error.
- **Interaction during exit**: A dialog that is fading out does not accept further interaction (it is leaving); input goes to the app beneath once the exit completes.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: Each modal overlay (About, project selector, rename project, add worktree, Settings) MUST animate a fade-in when it opens.
- **FR-002**: Each modal overlay MUST animate a fade-out when it is dismissed, for every dismissal path: Cancel, Esc, and successful submit/confirm.
- **FR-003**: During an overlay's fade-out, the underlying application content MUST become progressively visible; the overlay MUST NOT disappear in a single frame.
- **FR-004**: Overlay open and close animations MUST be clearly perceptible to a user (not an imperceptible flash).
- **FR-005**: Overlay fade-in SHOULD last approximately 250 ms and fade-out approximately 200 ms (informed defaults, tunable; originally specified as 300 ms / 240 ms and tuned down during implementation — both sets sit inside SC-002's 0.15–0.5 s band).
- **FR-006**: While an overlay animates out, its displayed content MUST remain visually consistent with its last shown state (no abrupt content change during exit).
- **FR-007**: All UI animations — the existing overflow-menu fade, sidebar slide, main-view fade, and resize-handle hover, plus the new overlay fades — MUST be driven by a single shared animation mechanism.
- **FR-008**: Registering a new animated element MUST NOT require adding element-specific state or plumbing in multiple places; it MUST require only naming the element, setting a target, and reading its progress.
- **FR-009**: After migration to the shared mechanism, the four pre-existing animations MUST retain their current visual behavior (timing and feel).
- **FR-010**: The shared animation mechanism's core logic MUST be verifiable by automated tests that run without rendering the UI (Constitution Principle I).
- **FR-011**: The overlay enter/exit treatment MUST be provided as one shared, reusable component used by every overlay — no per-overlay bespoke transition (Constitution Principle VIII).
- **FR-012**: The application's overlay lifecycle, form/draft state, and persistence behavior MUST remain functionally unchanged; the animations are presentation-only.
- **FR-013**: Animation timing MUST be expressed as human-legible durations (time units), not opaque per-frame step values.
- **FR-014**: The app MUST NOT perform ongoing animation work while nothing is animating (idle cost unchanged from today).
- **FR-015**: The reusable animation mechanism MUST be self-contained and free of dependencies on this application's domain (its core logic MUST NOT reference application-specific types, state, or modules), so it can be extracted into a standalone library and reused by other projects without modification.
- **FR-016**: Application-specific concerns (which elements animate, their identifiers, and their timings) MUST live in the consuming application, not inside the reusable mechanism, so the mechanism embeds no app-specific assumptions.
- **FR-017**: The reusable animation library MUST expose a documented public interface (its animatable-value operations) suitable for consumers outside this project.

### Key Entities

- **Animation track**: A single named, animatable value that moves from its current value toward a target over time, exposing a progress reading (fully hidden → fully shown). The collection of tracks is the shared mechanism every component draws from.
- **Overlay transition**: The shared visual treatment applied to any modal overlay as it enters and exits, parameterized by a single progress value.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: 100% of the five modal overlays visibly animate on both open and close, across all three dismissal paths (Cancel, Esc, successful submit).
- **SC-002**: The overlay open animation completes in roughly 0.3 s and the close animation in roughly 0.24 s (each within a 0.15 s–0.5 s perceptible band).
- **SC-003**: No overlay appears or disappears within a single rendered frame — every open/close is a visible transition.
- **SC-004**: Adding one new animated element requires a change in a single location to register it, versus the current pattern that requires edits in multiple locations.
- **SC-005**: All four pre-existing animations behave identically to before the change (no visual regression), confirmed by running the app.
- **SC-006**: Idle behavior is unchanged — no animation processing occurs while nothing is animating.
- **SC-007**: All existing automated tests continue to pass, and the shared animation mechanism is covered by new non-rendering tests.
- **SC-008**: The animation library's core has zero dependencies on this application's domain modules — verifiable because it compiles and its tests run without any of the application's app-specific code — so it can be lifted into another project unchanged.

## Assumptions

- The target durations (≈300 ms in / ≈240 ms out) are informed defaults chosen because the prior ~90 ms motion was imperceptible; they may be tuned during implementation and review.
- Reduced-motion / accessibility motion-reduction preferences are out of scope for this feature (the application exposes no such setting today).
- The rendering environment provides no general per-element transparency, so the "reveal the app beneath" fade is achieved with an animated dimming layer plus a dialog transform; this is a known technical constraint recorded for the planning phase, not a user-facing requirement.
- Overlay dialogs are non-interactive while animating out (they are leaving the screen).
- At most one modal overlay is shown at a time (existing application behavior).
- The fade-out is handled entirely in the rendering layer; the pure application core (overlay lifecycle, drafts, persistence) is not modified.
- The extraction requirement is met by making the animation mechanism's **core** self-contained, framework-agnostic, and app-agnostic, with a documented public API. Whether that core is physically split into a separately published package now or kept as a self-contained internal module prepared for extraction is a delivery decision recorded in the plan (see `plan.md` / `research.md`).
- The animation library has two layers: a **framework-agnostic core** (the animatable-value engine — reusable in any project) and a thin set of **rendering helpers** that necessarily depend on the GUI framework (reusable by other applications built on the same GUI framework). Only the core is required to be free of GUI-framework coupling; the rendering helpers are a separate, optional layer.

**Alignment**: 2026-07-20 — Spec/code alignment audit. FR-005's informed defaults updated from 300 ms / 240 ms to the implemented 250 ms / 200 ms. Both sets sit inside SC-002's 0.15–0.5 s band and FR-005 is a SHOULD with an explicit "tunable" allowance, so this records the tuning rather than changing a requirement. Feature 007 is otherwise fully conformant — all 17 FRs satisfied with no wiring gaps.
