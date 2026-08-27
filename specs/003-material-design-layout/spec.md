# Feature Specification: Material Design Layout & Theming

**Feature Branch**: `003-material-design-layout`

**Created**: 2026-07-15

**Status**: Closed (implemented and shipped; every task in tasks.md is done. The manual quickstart walkthrough ran 2026-08-21 on Linux — evidence: `evidence/T015-T033-manual-walkthrough.md`. §1, §5 and §6 pass; §3 passes but for FR-016; §2 and §4 are partial. One open defect: BUG-002 — a narrow window drops the project name and clips the actions in the known-projects list, failing FR-016 and the spec's own small-window edge case. macOS/Windows parity and a live OS theme change (SC-003) are unrun.)

**Bugfix**: 2026-07-21 — BUG-001 Clarified FR-018/Edge Cases to distinguish a transient OS-theme-detection failure from a genuine, sustained "no preference" reading; added FR-021.

**Input**: User description: "Adopt a Material Design layout and visual language across the Micold AI IDE application shell, with a single design system (color roles, typography scale, spacing, shape), light and dark themes that follow the OS by default and are user-overridable, and all existing surfaces restyled without behavior loss."

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Coherent Material layout across the shell (Priority: P1)

A developer opens the application and sees a structured, professional layout: a top app
bar with the product title and primary actions, a clearly organized main content region,
and distinct surfaces (cards/containers) with consistent elevation, spacing, typography,
and button styling. Every existing screen — active-project header, empty state,
known-projects list, About dialog, project selector, and rename flow — shares the same
visual language.

**Why this priority**: This is the core value of the feature. Without a coherent layout
and shared design system, none of the other outcomes matter. It is also the largest slice
and delivers the visible improvement on its own.

**Independent Test**: Launch the app in its default theme and walk through every existing
screen and dialog. Confirm each renders with consistent typography, spacing, elevation,
and button variants, and that all prior actions (open, rename, reopen, about, select)
still work identically.

**Acceptance Scenarios**:

1. **Given** the app is open with no project, **When** the empty state is shown, **Then**
   it is presented as a Material surface with headline/body typography and a clearly
   styled primary action button, and pressing it still opens the project selector.
2. **Given** the app has an active project, **When** the shell renders, **Then** the top
   app bar shows the title and primary actions, and the active-project header is a Material
   surface preserving the project name, path, and "open another project" action.
3. **Given** one or more known projects exist, **When** the list renders, **Then** each
   entry is a Material list item/card that preserves the active marker, the "git" badge,
   the unavailable state, and the Open / Rename actions with correct enabled/disabled state.
4. **Given** any interactive element, **When** the pointer hovers, focuses, presses, or the
   element is disabled, **Then** it shows the corresponding Material visual state.

---

### User Story 2 - Light and dark themes following the system (Priority: P2)

A developer's operating system is set to dark mode. On first launch the application appears
in its dark theme automatically. When they switch the OS to light mode, the application
updates live to its light theme without a restart. Both themes are fully designed — dark is
not a dimmed afterthought.

**Why this priority**: Theming builds on the design system from Story 1 and is a high-value,
expected behavior for a modern desktop tool, but the app is already usable and coherent with
Story 1 alone.

**Independent Test**: Set the OS to dark, launch, and confirm the app is dark. Change the OS
to light while the app runs and confirm it switches live. Repeat in reverse. Verify every
screen is legible and correctly styled in both themes.

**Acceptance Scenarios**:

1. **Given** no saved theme preference and the OS set to dark, **When** the app launches,
   **Then** it displays the dark theme.
2. **Given** the app is running while following the system, **When** the OS theme preference
   changes, **Then** the app switches to the matching theme live without a restart.
3. **Given** either theme is active, **When** any screen or dialog is displayed, **Then** all
   text meets a legible contrast level against its surface and no element is unstyled.

---

### User Story 3 - User-configurable theme override (Priority: P3)

A developer prefers dark mode even though their OS is in light mode. They open the app's
theme setting and choose "Dark". The app switches to dark and stays dark across restarts,
regardless of the OS. Later they choose "Follow system" and the app resumes tracking the OS
preference.

**Why this priority**: A refinement on top of Story 2. Valuable for user control but the
default system-following behavior already satisfies most users.

**Independent Test**: With the OS in light mode, set the app override to Dark, confirm it
turns dark, restart the app, and confirm it is still dark. Switch the setting to "Follow
system" and confirm it returns to matching the OS.

**Acceptance Scenarios**:

1. **Given** the app is following the system, **When** the user selects a fixed theme (Light
   or Dark), **Then** the app switches to that theme immediately and ignores the OS preference.
2. **Given** a fixed theme override is set, **When** the app is closed and reopened, **Then**
   it launches with the overridden theme.
3. **Given** a fixed theme override is set, **When** the user selects "Follow system", **Then**
   the app resumes matching the OS theme and updates live on subsequent OS changes.

---

### Edge Cases

- What happens when the operating system does not report a theme preference or reporting is
  unavailable? The app falls back to a defined default theme (light) while in "follow system"
  mode. **This is the *sustained* case — the OS genuinely has no preference to report** (see
  FR-021 for the *transient* case below).
- What happens when a single OS theme detection attempt fails or times out (e.g. under CPU load)
  without the OS preference actually changing? The app holds the last-known system theme for
  that poll rather than treating the failure as "no preference" — it does not flash to the
  fallback theme (FR-021). *(Added 2026-07-21 — BUG-001.)*
- How does the app handle a corrupt or unreadable stored theme preference? It ignores the bad
  value and reverts to "follow system".
- What happens to the layout at very small window sizes? Content remains usable — surfaces
  reflow and spacing is preserved rather than clipping or overlapping.
- What happens when the OS theme changes rapidly or repeatedly while the app runs? The app
  settles on the latest reported preference without flicker or stuck intermediate states.
- How is the active-project marker, "git" badge, and "unavailable" state distinguished in both
  light and dark themes? Each remains visually distinct and legible in both.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: The application MUST present a Material Design 3 layout consisting of a top app
  bar (title + primary actions), a structured main content region, and delineated surfaces
  with consistent elevation.
- **FR-002**: The application MUST define a single design system covering color roles (at
  minimum primary, surface, background, error, and their on-* foreground variants), a
  typography scale (display, headline, title, body, label), a spacing scale, and shape/corner
  radii.
- **FR-003**: Every screen and dialog MUST draw its colors, typography, spacing, and shape
  from the shared design system rather than per-widget hard-coded values.
- **FR-004**: The application MUST provide fully designed light and dark themes as equal
  first-class targets.
- **FR-005**: By default the application MUST follow the operating system's light/dark theme
  preference.
- **FR-006**: While following the system, the application MUST update its theme live when the
  operating system's preference changes, without requiring a restart.
- **FR-007**: Users MUST be able to override the system default by selecting a fixed Light or
  Dark theme.
- **FR-008**: Users MUST be able to return to "follow system" after setting a fixed override.
- **FR-009**: The application MUST persist the user's theme preference (follow-system, light,
  or dark) across restarts.
- **FR-010**: The application MUST restyle the top app bar (formerly the toolbar) to the design
  system while preserving its title and existing primary actions.
- **FR-011**: The application MUST restyle the active-project header and the "no project open"
  empty state as Material surfaces, preserving the project name, path, and their existing
  actions.
- **FR-012**: The application MUST render the known-projects list as Material list items/cards
  while preserving the active marker, the "git" badge, the unavailable state, and the
  Open / Rename actions with their existing enabled/disabled behavior.
- **FR-013**: The application MUST restyle the About dialog, project selector, and rename flow
  to match the design system.
- **FR-014**: Interactive elements MUST present consistent Material visual states for hover,
  focus, pressed, and disabled.
- **FR-015**: Buttons MUST use Material button variants (filled, outlined, text) applied
  consistently by role (e.g. primary action vs. secondary action).
- **FR-016**: The layout MUST respond to window resizing and preserve usable spacing at small
  window sizes without clipping or overlapping content.
- **FR-017**: All previously existing behavior (opening, reopening, renaming projects, the
  About flow, and project selection) MUST remain unchanged; this feature changes presentation
  and theme handling only.
- **FR-018**: When the OS theme preference is **sustained-unavailable** (the OS reports no
  preference, or no successful detection has ever occurred) and the app is following the system,
  the application MUST fall back to a defined default theme. *(Clarified 2026-07-21 — BUG-001:
  see FR-021 for a transient detection failure, which is a distinct case.)*
- **FR-019**: When the stored theme preference is missing or invalid, the application MUST
  revert to "follow system".
- **FR-020**: Text MUST remain legible against its surface in both themes, meeting a defined
  minimum contrast level.
- **FR-021**: While following the system, a **transient** failure of a single OS theme detection
  attempt (e.g. a detection call timing out under CPU load) MUST NOT be treated as the OS
  reporting no preference; the application MUST retain the last-known system theme and MUST NOT
  change the displayed theme until a subsequent detection attempt succeeds. Only a sustained
  inability to detect a preference invokes the FR-018 fallback. *(Added 2026-07-21 — BUG-001.)*

### Key Entities *(include if feature involves data)*

- **Theme preference**: The user's persisted choice of how the app selects its theme — one of
  "follow system", "light", or "dark". Stored locally alongside existing application state.
- **Design system tokens**: The named, centralized set of color roles, typography styles,
  spacing steps, and shape values that every surface references. Not user data; a shared
  definition consumed across all screens.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: 100% of existing screens and dialogs render under the Material Design system in
  both light and dark themes with no loss of prior behavior.
- **SC-002**: On first launch with no saved preference, the app matches the OS theme in 100%
  of cases where the OS reports a preference.
- **SC-003**: Changing the OS theme while the app follows the system updates the app's theme
  within 1 second and without a restart.
- **SC-004**: A user-set theme override persists across 100% of app restarts, and selecting
  "follow system" restores OS-tracking behavior.
- **SC-005**: All text meets at least the standard AA contrast ratio (4.5:1 for normal text)
  against its surface in both themes.
- **SC-006**: All pre-existing tests continue to pass, and no existing user-facing behavior
  changes except presentation and theme handling.
- **SC-007**: Design tokens are defined in a single location and referenced by every surface;
  a review finds zero per-widget hard-coded color, spacing, or typography magic numbers in the
  restyled surfaces.

## Assumptions

- The design system targets Material Design 3 (the current Material Design generation) as the
  reference language.
- "Live" theme updates apply to the running application window; no multi-window synchronization
  is required beyond what the app already supports.
- The default fallback theme, when no OS preference is available, is light.
- The AA contrast target (4.5:1 for normal text, 3:1 for large text) is the accessibility bar;
  no higher (AAA) bar is required for this feature.
- The theme preference is stored using the application's existing local storage mechanism; no
  new storage backend is introduced.
- This feature does not add new screens or features; the theme setting is surfaced within an
  existing surface (e.g. the app bar or an existing menu/dialog), and where exactly is a
  planning/implementation detail.
- No third-party Material component library is assumed; whether to build the styling in-house or
  adopt a library is decided during planning.
