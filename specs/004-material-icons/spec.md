# Feature Specification: Material Design Icons

**Feature Branch**: `004-material-icons`

**Created**: 2026-07-15

**Status**: Draft

**Input**: User description: "Introduce Material Design icons as a shared, cross-application capability in the Micold AI IDE, using the Material Symbols icon font. Bundle a single embedded Material Symbols (Outlined) font and expose a curated set of named icon constants that map to glyph codepoints, so any surface can render an icon consistently. Icons must integrate with the existing design system: tinted via the established color roles (e.g. on-surface, on-primary, on-surface-variant), sized off the typography scale, and correct in both light and dark themes. Apply icons across all existing surfaces without behavior loss — toolbar/app-bar actions (help, about), primary/secondary action buttons (open project), known-projects list items (open, rename), the git badge, and the active and unavailable markers. Keep the icon-name-to-codepoint mapping in the render-free core so it is unit-testable without iced, and keep font loading and glyph rendering in the GUI layer behind the `gui` feature. Preserve the license posture (Material Symbols is Apache-2.0, matching the repo). No new interactive behavior is added; existing actions must work identically, just with iconography."

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Consistent iconography across every surface (Priority: P1)

A developer opens the application and sees recognizable Material icons paired with the
actions and states throughout the shell: the top app-bar actions carry icons, the primary
"open project" action shows a folder/open icon, each known-project list entry shows
open and rename affordances as icons, the git badge is an icon, and the active and
unavailable states are conveyed with an icon. Every icon is drawn from one shared icon
set, so the same concept always uses the same glyph.

**Why this priority**: This is the core value of the feature and the reason it exists —
a single, coherent icon vocabulary applied consistently across the whole shell. It is the
largest slice and delivers the visible improvement on its own.

**Independent Test**: Launch the app and walk through every existing screen and dialog.
Confirm each named action/state renders its icon, that the same concept uses the same
glyph everywhere it appears, and that every prior action (open, rename, reopen, about,
select) still works identically.

**Acceptance Scenarios**:

1. **Given** the app is open with no project, **When** the empty state is shown, **Then**
   the primary action displays its icon alongside its label, and pressing it still opens
   the project selector.
2. **Given** the app has an active project, **When** the shell renders, **Then** the top
   app-bar actions (help, about) display their icons and remain clickable with unchanged
   behavior.
3. **Given** one or more known projects exist, **When** the list renders, **Then** each
   entry shows the git badge as an icon, the active and unavailable states as icons, and
   the Open / Rename actions as icons with correct enabled/disabled state and unchanged
   behavior.
4. **Given** any surface that shows a given concept (e.g. "open"), **When** that concept
   appears on more than one surface, **Then** it uses the same icon glyph in every place.

---

### User Story 2 - Icons correct in light and dark themes (Priority: P2)

A developer using the application in either the light or the dark theme sees every icon
rendered legibly, with the icon color matching the surrounding text/foreground for its
surface (e.g. an icon on a primary button matches the on-primary foreground; an icon in
body text matches the on-surface foreground). When the theme switches, icon colors update
together with the rest of the UI, with no icon left mismatched or invisible.

**Why this priority**: Theming correctness builds on the existing design system and is
expected for a modern desktop tool, but the icons already deliver value under a single
theme from Story 1.

**Independent Test**: View every screen in the light theme, then the dark theme, and
confirm every icon is legible and its color matches the foreground role of its surface in
both themes. Toggle the theme while the app runs and confirm icon colors switch live with
the rest of the UI.

**Acceptance Scenarios**:

1. **Given** the app is in the dark theme, **When** any surface renders, **Then** every
   icon is legible and tinted to the correct foreground role for its surface.
2. **Given** the app is in the light theme, **When** any surface renders, **Then** every
   icon is legible and tinted to the correct foreground role for its surface.
3. **Given** the app is running, **When** the theme changes, **Then** icon colors update
   in step with the surrounding UI with no mismatched or invisible icon.

---

### User Story 3 - A reusable icon vocabulary for future surfaces (Priority: P3)

A contributor adding a new surface can reference an icon by a stable, human-readable name
(rather than a raw glyph codepoint) and render it at any size and foreground color using
the shared mechanism, so new surfaces stay consistent with the existing ones without
re-deriving how icons work.

**Why this priority**: This generalizes the capability for future work. It is valuable for
maintainability but not required to realize the immediate visible benefit of Stories 1
and 2.

**Independent Test**: From a new call site, request an icon by its name and render it;
confirm it produces the expected glyph, honors a requested size and foreground color, and
that referencing an unknown icon name is caught before or at build time rather than
silently rendering a wrong or missing glyph.

**Acceptance Scenarios**:

1. **Given** the shared icon set, **When** a caller references an icon by its documented
   name, **Then** the correct glyph is produced.
2. **Given** a caller references an icon name that is not in the set, **When** the code is
   built, **Then** the mismatch is surfaced as a build-time error rather than a runtime
   surprise.

---

### Edge Cases

- **Missing glyph / tofu**: If a requested icon has no glyph in the bundled font, the
  system MUST fail closed at build time (unknown named icons are not representable) so a
  blank box ("tofu") can never reach the running UI.
- **Very small or very large sizes**: Icons requested at the smallest label size and the
  largest display size MUST remain recognizable and correctly aligned with adjacent text.
- **Icon-only vs icon+label controls**: Controls that show only an icon (no text label)
  MUST remain identifiable and MUST preserve the accessible meaning of the action they
  previously conveyed via text.
- **Disabled controls**: An icon on a disabled control MUST reflect the disabled visual
  state consistently with the rest of that control.
- **Theme switch mid-session**: A theme change while the app is open MUST not leave any
  icon tinted for the previous theme.
- **Font load failure**: If the bundled icon font cannot be loaded at startup, the failure
  MUST be handled gracefully (the app still runs and text remains legible) rather than
  crashing or rendering unreadable placeholder boxes everywhere.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: The system MUST provide a single, shared icon set that every surface draws
  from, so the same concept is always represented by the same icon.
- **FR-002**: The system MUST expose icons by stable, human-readable names rather than
  requiring callers to use raw glyph codepoints.
- **FR-003**: The mapping from icon name to its glyph MUST live in the render-free core so
  it is unit-testable without the GUI framework, and the set of valid names MUST be closed
  (an unknown name is not representable and is caught at build time).
- **FR-004**: The system MUST render any icon at a caller-specified size drawn from the
  existing typography scale, and MUST tint it with a caller-specified foreground color
  role from the existing design-system color roles.
- **FR-005**: The system MUST apply icons to all existing surfaces without loss of
  behavior: toolbar/app-bar actions (help, about), the primary/secondary action button(s)
  (open project), known-projects list items (open, rename), the git badge, and the active
  and unavailable markers.
- **FR-006**: Every existing action MUST continue to work identically after icons are
  applied; no new interactive behavior is introduced by this feature.
- **FR-007**: Icons MUST render correctly and legibly in both the light and dark themes,
  and their colors MUST update together with the rest of the UI when the theme changes.
- **FR-008**: Icon glyph rendering and font loading MUST reside in the GUI layer behind the
  `gui` feature, so the render-free core (and its `--no-default-features` test run)
  compiles and passes without pulling in the GUI framework.
- **FR-009**: The bundled icon resource MUST be embedded in the application (no runtime
  dependency on a system-installed font or network fetch), consistent with local-first,
  offline operation.
- **FR-010**: The icon resource MUST be distributed under a license compatible with the
  repository's Apache-2.0 posture, and its provenance and license MUST be recorded
  in-repo.
- **FR-011**: Icon-only controls MUST preserve the meaning previously conveyed by their
  text label so the action remains identifiable.
- **FR-012**: The feature MUST behave equivalently on Linux, macOS, and Windows (icons
  render identically across platforms).
- **FR-013**: The feature MUST ship with corresponding user-guide documentation describing
  the shared icon vocabulary and how surfaces use it, updated in the same change.

### Key Entities

- **Icon**: A single named, rendered symbol representing a concept or action. Key
  attributes: a stable human-readable name, the concept it represents, and its glyph in
  the bundled icon set. Rendered with a size (from the typography scale) and a foreground
  color role (from the design-system color roles).
- **Icon Set**: The closed, curated collection of all Icons available to the application —
  the single source of truth every surface references. Backed by one embedded icon font
  resource with recorded license/provenance.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: 100% of the surfaces and states listed in FR-005 render their intended icon.
- **SC-002**: Every concept that appears on more than one surface uses the same icon glyph
  in every location (0 inconsistent representations).
- **SC-003**: 100% of prior actions (open, rename, reopen, about, select) remain functional
  and behave identically after the feature ships (0 behavior regressions).
- **SC-004**: In both the light and the dark theme, every rendered icon is legible and its
  color matches the foreground role of its surface (0 mismatched or invisible icons), and
  a mid-session theme switch leaves 0 icons tinted for the previous theme.
- **SC-005**: No "tofu"/blank-box placeholder ever appears in the running UI; any unknown
  icon name is rejected at build time (0 missing-glyph occurrences at runtime).
- **SC-006**: The render-free core continues to build and pass its test suite without the
  GUI framework (the `--no-default-features` test run stays green), and the icon
  name-to-glyph mapping is covered by tests there.
- **SC-007**: Icons render identically on Linux, macOS, and Windows (0 platform-specific
  rendering differences for the shipped icon set).

## Assumptions

- The existing design-system tokens — typography size scale and foreground color roles
  (e.g. on-surface, on-primary, on-surface-variant) — are reused as-is; this feature does
  not introduce new sizes or color roles, only consumes them for icons.
- The "Outlined" Material Symbols style at a single default weight/fill is sufficient for
  all current surfaces; multiple optical weights/fills are out of scope for this pass.
- ~~A curated subset of icons (only those needed by the surfaces in FR-005, plus a small
  headroom for near-term surfaces) is bundled, not the entire Material Symbols catalog.~~
  (Superseded — spec/code alignment 2026-07-27: feature 009's research R6 replaced the shipped
  font with **full glyph coverage** — every codepoint the upstream font maps — so adding a new
  `Icon` variant never again requires regenerating the font binary, only `src/icons.rs` +
  `tests/icons.rs`. See `assets/fonts/PROVENANCE.md`.) The bundled font ships full Material
  Symbols Outlined glyph coverage; the curated set is expressed in code (the closed `Icon` enum),
  not by subsetting the font file.
- The chosen icon font is available under Apache-2.0 (matching the repository license), so
  no additional licensing negotiation is required; its license text/provenance is vendored
  in-repo.
- No accessibility mechanism beyond preserving each control's existing meaning/label is in
  scope for this pass; screen-reader/labelling enhancements, if any, are handled by the
  surfaces' existing text, not by this feature.
- This feature is additive/visual only: it reuses the existing state model and message
  flow and does not add, remove, or alter any user action.
