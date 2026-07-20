# Feature Specification: Application Shell with Help / About

**Feature Branch**: `001-app-shell-about`

**Created**: 2026-07-13

**Status**: Draft

**Input**: User description: "Basic application window with a Help / About toolbar. When the user launches Micold AI IDE, the app opens a main window with a toolbar across the top. The toolbar contains a \"Help\" entry. Selecting \"Help\" reveals an \"About\" action. Activating \"About\" opens an About dialog showing the application name (Micold AI IDE), the current version, the open-source license, and a one-line description of the app. The user can dismiss the About dialog (via a Close button or the Esc key) to return to the main window."

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Launch to a working application window (Priority: P1)

A user starts Micold AI IDE and is presented with a single main application window that
has a toolbar running across the top. The toolbar shows a "Help" entry. The window is
usable and ready for further interaction.

**Why this priority**: This is the foundational UI shell that every later feature builds
on. Without a window and toolbar there is nothing to hang the About action — or any future
feature — off of. It is the smallest slice that delivers a running, visible product.

**Independent Test**: Launch the application on a clean machine and confirm a single main
window appears with a top toolbar containing a visible "Help" entry, and that no other
toolbar entries are present. Delivers value as a demonstrable, runnable app shell.

**Acceptance Scenarios**:

1. **Given** the application is installed, **When** the user launches it, **Then** a single main window opens with a toolbar across the top.
2. **Given** the main window is open, **When** the user inspects the toolbar, **Then** a "Help" entry is visible and it is the only toolbar entry.

---

### User Story 2 - View application information via Help → About (Priority: P1)

From the open main window, the user selects "Help" in the toolbar, which reveals an "About"
action. Activating "About" opens an About dialog that shows the application name
("Micold AI IDE"), the current version, the project's open-source license, and a one-line
description of the app.

**Why this priority**: Presenting the app's identity, version, and license is the headline
value of this feature and is the primary user story. It also establishes the reusable
dialog/overlay pattern that later features depend on.

**Independent Test**: With the main window open, select Help, activate About, and verify the
dialog displays all four required fields (name, version, license, description) with correct,
non-empty values.

**Acceptance Scenarios**:

1. **Given** the main window is open, **When** the user selects "Help", **Then** an "About" action is revealed.
2. **Given** the "About" action is available, **When** the user activates it, **Then** an About dialog opens.
3. **Given** the About dialog is open, **When** the user reads its contents, **Then** the application name "Micold AI IDE", the current version, the open-source license name, and a one-line description are all visible.
4. **Given** the application was built for release, **When** the About dialog shows the version, **Then** the displayed version matches the build/package metadata (it is not a hardcoded value).

---

### User Story 3 - Dismiss the About dialog and return to the window (Priority: P2)

After viewing the About dialog, the user closes it — either by clicking a "Close" button or
by pressing the Esc key — and is returned to the main window in the same state as before the
dialog opened.

**Why this priority**: Completing the round trip makes the interaction non-trapping and
establishes the dismiss behavior that the reusable overlay pattern will reuse. It builds
directly on Stories 1 and 2 but is a distinct, separately testable capability.

**Independent Test**: With the About dialog open, dismiss it via the Close button in one
test and via the Esc key in another, and confirm the dialog disappears and the main window
is focused and unchanged in both cases.

**Acceptance Scenarios**:

1. **Given** the About dialog is open, **When** the user clicks "Close", **Then** the dialog closes and the main window is shown.
2. **Given** the About dialog is open, **When** the user presses Esc, **Then** the dialog closes and the main window is shown.
3. **Given** the dialog has been dismissed, **When** the user views the main window, **Then** the window and toolbar are in the same state as before the dialog was opened.

---

### Edge Cases

- **Version metadata unavailable**: If the version cannot be read from build/package metadata, the About dialog displays a clearly-labeled fallback (e.g., "unknown") rather than an empty field, a placeholder token, or a crash.
- **Repeated activation**: Activating "About" while the dialog is already open does not open a second dialog; a single instance is maintained.
- **Esc with no dialog open**: Pressing Esc when the About dialog is not open has no effect on the About feature (normal window behavior is unaffected).
- **Backdrop interaction**: While the modal About dialog is open, interaction with the rest of the main window (including the toolbar) is blocked; dismissal is only via the Close button or Esc.
- **Long content**: An unusually long license name or description is displayed without breaking the dialog layout (wraps or fits; no overflow that hides the Close control).
- **Window resize while open**: Resizing the main window while the dialog is open keeps the dialog usable and its Close control reachable.
- **Cross-platform rendering**: The window, toolbar, and dialog appear and behave the same on Linux, macOS, and Windows (no platform-specific divergence in the flow above).

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: On launch, the application MUST open a single main window with a toolbar across the top.
- **FR-002**: ~~The toolbar MUST contain a "Help" entry.~~ (Superseded — spec/code alignment 2026-07-20: the labelled "Help" entry became an unlabelled overflow-menu trigger as later features added toolbar actions.) The toolbar MUST contain an overflow-menu trigger that reveals the application's secondary actions, and MAY contain additional top-level triggers introduced by later features.
- **FR-003**: ~~The toolbar MUST NOT expose any entry other than "Help", and "Help" MUST expose only the "About" action (scope boundary).~~ (Superseded — spec/code alignment 2026-07-20: this scope boundary was intentionally crossed by features 003, 006, and 008, which each added a toolbar surface. It described feature 001's delivery boundary, not a durable product constraint.) The toolbar's overflow menu MUST expose the "About" action; it MAY also expose secondary actions owned by later features (currently the theme-mode toggle from feature 003 and "Settings" from feature 006). The toolbar MAY host additional top-level triggers owned by later features (currently the project switcher from feature 008).
- **FR-004**: Selecting the toolbar's overflow-menu trigger MUST reveal an "About" action.
- **FR-005**: Activating "About" MUST open an About dialog.
- **FR-006**: The About dialog MUST display the application name, exactly "Micold AI IDE".
- **FR-007**: The About dialog MUST display the current application version, sourced from build/package metadata rather than a hardcoded literal.
- **FR-008**: The About dialog MUST display the project's open-source license name (the OSI-approved license the project ships under).
- **FR-009**: The About dialog MUST display a one-line description of the application.
- **FR-010**: The About dialog MUST be dismissible via a "Close" button.
- **FR-011**: The About dialog MUST be dismissible via the Esc key.
- **FR-012**: Dismissing the About dialog MUST return the user to the main window with the window's prior state intact.
- **FR-013**: The About dialog MUST be presented as a modal overlay rendered within the main window (not a separate operating-system window), and MUST block interaction with the rest of the window while open.
- **FR-014**: When the About dialog opens, keyboard focus MUST move into the dialog; when it closes, focus MUST return to the main window.
- **FR-015**: Activating "About" while the dialog is already open MUST NOT create a second dialog instance.
- **FR-016**: When version metadata cannot be determined, the dialog MUST display a clearly-labeled fallback value instead of an empty field or an error.
- **FR-017**: The main window, toolbar, "Help" entry, "About" action, and About dialog MUST render and behave identically on Linux, macOS, and Windows.

### Key Entities *(include if feature involves data)*

- **Application Metadata**: The read-only identity of the running application as shown in the About dialog. Attributes: application name (fixed: "Micold AI IDE"), current version (from build/package metadata), open-source license name, and a one-line description. This feature only reads and displays this data; it does not create or modify it.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: From the open main window, a user can reach the About dialog in at most two interactions (select "Help", then activate "About").
- **SC-002**: 100% of the four required fields — application name, version, license, and description — are visible in the About dialog on first open without scrolling.
- **SC-003**: For every release build, the version shown in the About dialog matches the build's packaged version (zero instances of a hardcoded or stale version across releases).
- **SC-004**: A user can dismiss the About dialog and return to the main window using either the Close button or the Esc key, with the main window returned to its pre-dialog state in every attempt.
- **SC-005**: The full launch → Help → About → dismiss flow passes the same acceptance checks on Linux, macOS, and Windows (feature parity across all three platforms).
- **SC-006**: In an unassisted usability check, at least 95% of first-time users can locate and read the application's version and license without external guidance.

## Assumptions

- The version string is read from build/package metadata and is not hardcoded (FR-007).
- The About dialog is a modal overlay rendered within the main window, not a separate OS-level window (FR-013).
- The license shown is the project's OSI-approved license name as required by the constitution's licensing constraint; the specific license identifier is selected and tracked separately as a constitution follow-up, and the dialog displays whatever the project's chosen license resolves to.
- The window, toolbar, and dialog render and behave identically across Linux, macOS, and Windows (constitution Principle VI, Cross-Platform Parity).
- Exactly one main window exists; multi-window support is out of scope for this feature.
- Window state persistence (size, position, restoring across restarts) is out of scope for this feature.
- The one-line application description is fixed product copy provided with the build.
- No other toolbar or menu entries, and no editing/files/terminals/sessions/worktrees functionality, are included; this feature is strictly the UI shell plus the Help/About flow.

## Dependencies

- Requires the build/packaging process to expose application metadata (at minimum name and version) to the running application (supports FR-007).
- Requires the project's OSI-approved license to be selected and available so its name can be displayed (supports FR-008; tracked as a constitution follow-up TODO).

**Alignment**: 2026-07-20 — Spec/code alignment audit. FR-002 and FR-003 amended: the labelled "Help" toolbar entry became an unlabelled overflow-menu trigger, and FR-003's "no entry other than Help" scope boundary was intentionally crossed by features 003 (theme toggle), 006 (Settings), and 008 (project switcher). FR-004 reworded to match. No behaviour change — the code was correct and the spec had gone stale. Note: `app::toolbar_entries()` / `TOOLBAR_ENTRIES` remain in the code exercised only by `tests/toolbar.rs`; they describe the superseded FR-002 wording and should be removed with that test.
