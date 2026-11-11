# Feature Specification: Structured view: Material 3 select and expand animation for model, effort and permissions settings

**Feature Branch**: `feat/1792_feat-structured-view-material-3-select-input-and`

**Created**: 2026-11-11

**Status**: Draft

**Input**: User description: "feat: structured view: Material 3 select input and expand animation for the model, effort and permissions settings (#1792)"

## Clarifications

### Session 2026-11-11

- Q: Does the app have a reduced-motion setting this feature must honour? → A: Yes, `appearance.reduce_motion` (Settings > Appearance > Reduce motion); the OS preference is not read. Added FR-013 and an Out of Scope entry. _(agent-resolved: docs/user-guide/settings.md#Reduce motion)_

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Material 3 select for model, effort and permissions (Priority: P1)

In the structured view, the user changes the model, the effort and the permissions through select inputs. Each select looks like the Material 3 select already shown in the component showcase: container shape, outline or fill, label, trailing arrow, and state layers.

**Why this priority**: The select is the control the user touches every time they change a setting; its look is the core of the request.

**Independent Test**: Open the structured view and the component showcase side by side; put the select in each state and compare.

**Acceptance Scenarios**:

1. **Given** the structured view shows the model select, **When** the user views it at rest, **Then** it shows the Material 3 container shape, outline or fill, label and trailing arrow, as the showcase select does.
2. **Given** a select, **When** the user hovers it, focuses it with the keyboard, or presses it, **Then** it shows the Material 3 hover, focus and pressed state layer respectively.
3. **Given** a select that is disabled, **When** the user views it, **Then** it shows the Material 3 disabled look and does not react to hover, focus or press.
4. **Given** the effort and permissions selects, **When** viewed in any state, **Then** they look identical in style to the model select.

---

### User Story 2 - Material 3 menu for the select options (Priority: P2)

When the user opens a select, the list of options appears as a Material 3 menu.

**Why this priority**: The menu is shown right after the select; a Material 3 select with a non-Material menu would look inconsistent.

**Independent Test**: Open each select and compare the menu with the showcase select's menu.

**Acceptance Scenarios**:

1. **Given** a select in the structured view, **When** the user opens it, **Then** the options appear in a Material 3 menu (container shape, elevation, item layout, selected item emphasis, item state layers).
2. **Given** the open menu, **When** the user hovers or keyboard-focuses an option, **Then** that option shows the Material 3 state layer.

---

### User Story 3 - Expand and collapse animation for the settings (Priority: P2)

When the user opens the model, effort or permissions settings, they grow from their trigger to their full height; when the user closes them, they collapse back the same way. The motion uses the app's existing duration and easing.

**Why this priority**: Removes the abrupt instant open, but the settings work without it.

**Independent Test**: Trigger the open and the close of each setting and watch the height change over time.

**Acceptance Scenarios**:

1. **Given** closed settings, **When** the user opens them, **Then** they grow from the trigger to full height over the app's standard motion duration and easing.
2. **Given** open settings, **When** the user closes them, **Then** they collapse to the trigger with the same duration and easing.
3. **Given** settings that are mid-animation, **When** the user toggles them again, **Then** the motion reverses from the current height without a jump.
4. **Given** the user has turned on the app's Reduce motion setting (Appearance), **When** the settings open or close, **Then** the change is shown in one step without the grow or collapse motion.

---

### Edge Cases

- A select with an empty option list: it does not open a menu and does not crash.
- A select whose current value is no longer in the option list: the label and value area still renders without error.
- A very long option or value text: it is truncated within the container, the trailing arrow stays visible.
- Several sessions open in the structured view at once (multi-session): each session's settings expand and collapse independently, one's animation never moves another's.
- A window too small for the full settings height: the settings stay reachable (clipped or scrolled) and the animation ends at the reachable height.
- Rapid repeated toggling during the animation: no flicker and no stuck half-open state.
- Light and dark theme, and all supported platforms (Linux, macOS, Windows): the same Material 3 tokens apply; no platform-specific look.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: The structured view's select input MUST use the Material 3 container shape, outline or fill, label and trailing arrow, taken from the app's existing Material 3 tokens.
- **FR-002**: The select MUST show distinct Material 3 state layers for hover, focus, pressed and disabled.
- **FR-003**: The model, effort and permissions settings MUST all use this select with the same look.
- **FR-004**: The select's option menu MUST use the Material 3 menu look, including per-option state layers and the selected option's emphasis.
- **FR-005**: Opening the model, effort or permissions settings MUST animate as an expand from the trigger to the full height; closing MUST collapse the same way.
- **FR-006**: The animation duration and easing MUST come from the app's existing motion tokens, not literals.
- **FR-007**: A toggle during an animation MUST reverse it from the current height.
- **FR-008**: Each session's settings animation MUST be independent of other sessions.
- **FR-009**: The structured view's select MUST match the showcase's Material 3 select in every state.
- **FR-010**: Selecting an option MUST keep the existing behaviour: the chosen value is applied to the setting.
- **FR-011**: Edge cases MUST hold: an empty option list opens no menu; a value missing from the options and very long text still render within the container with the arrow visible; a window too small for the full height keeps the settings reachable.
- **FR-012**: The look and animation MUST be verified by the visual pass (quickstart Part B).
- **FR-013**: When the app's Reduce motion setting (`appearance.reduce_motion`) is on, opening and closing the model, effort and permissions settings MUST switch in one step with no animation. The operating system's reduced-motion preference is not read, as for the app's other animations.

### Key Entities

- **Select input**: a control showing a label, the current value and a trailing arrow; states: rest, hover, focus, pressed, disabled, open.
- **Settings group**: the model, effort and permissions settings that expand from and collapse to their trigger.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: In the visual pass, 100% of the select's states (rest, hover, focus, pressed, disabled, open) in the structured view are judged to match the showcase's Material 3 select.
- **SC-002**: Opening and closing each of the three settings takes the standard motion duration, within one frame of it, and never shows a full-height jump.
- **SC-003**: No colour, shape, duration or easing in the new look is a hard-coded value; all come from existing tokens.
- **SC-004**: Changing a setting through the select takes the same number of user actions as before.

## Assumptions

- The component showcase already has a Material 3 select and menu; it is the reference.
- The app already has motion tokens (duration, easing) and Material 3 colour, shape and state tokens.
- The animation applies to the model, effort and permissions settings only; other panels are out of scope.
- #1765 (animate opening the settings) is covered for these three settings by this feature; it is closed as a duplicate or narrowed when this is planned.

## Out of Scope

- Reading the operating system's reduced-motion preference (the app does not read it anywhere).
- Moving the settings icon in the structured view (#1764).
- Animating settings other than model, effort and permissions.
- Changing which options a select offers or what they do.
- New Material 3 tokens or changes to the showcase's select.
