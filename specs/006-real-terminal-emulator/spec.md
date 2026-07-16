# Feature Specification: Real Terminal Behavior for Embedded Session Terminals

**Feature Branch**: `006-real-terminal-emulator`

**Created**: 2026-07-16

**Status**: Draft

**Input**: User description: "Improve the terminal behavior. It should act as a real terminal and support colors, same as a regular terminal. Propagate the shortcut key events, only when focused on the terminal. Should generally allow for regular work with the claude CLI and ensure that it is displayed in a proper way."

## Clarifications

### Session 2026-07-16

- Q: How does the user move focus out of the terminal back to the app, given that Escape is forwarded to the process? → A: Click outside the terminal pane, or a reserved application keyboard shortcut that is never forwarded to the process (the exact key is chosen during planning).
- Q: Should the terminal forward mouse events to the process, and how does that coexist with text selection/copy? → A: Forward mouse events when the process enables mouse reporting; mouse text selection and copy are still supported — dragging selects when no mouse reporting is active, and holding a modifier (e.g. Shift) selects while mouse reporting is active.
- Q: How is copy/paste triggered by keyboard when the terminal is focused, given that Ctrl+C / Ctrl+V go to the process? → A: Support all common methods — platform-standard terminal chords (Ctrl+Shift+C / Ctrl+Shift+V; Cmd+C / Cmd+V on macOS), auto-copy of the selection with middle-click paste, and a right-click context menu / on-screen controls. The terminal intercepts these gestures and does not forward them to the process.
- Q: What is the scrollback history limit? → A: It is user-configurable, not a fixed constant. A new Settings form — opened from a Settings item in a toolbar dropdown menu — lets the user set the terminal scrollback limit. It ships with a sensible default (e.g. 10,000 lines) and the configured value is persisted locally across restarts.
- Q: What happens to keystrokes typed into a focused terminal while its session's process is not Running (starting, restarting, or failed)? → A: Discard them (no buffering) and show the session status so the user knows why input is not accepted; focus, scrolling, selection, and copy still work.
- Q: What responsiveness guarantee applies under very high output volume? → A: Coalesce rapid output into throttled redraws so input and scrolling stay responsive (≤~100 ms perceived latency); memory stays bounded by the configured scrollback limit; intermediate frames may be coalesced but the final screen state is always correct.

## User Scenarios & Testing *(mandatory)*

The embedded session terminal introduced in feature 005 today shows the running process's output as plain monospace text with all colors and text styles stripped, and accepts input only through a line-buffered box (type a line, press Enter, the whole line is sent). This makes interactive use of the `claude` CLI — an interactive terminal UI — impractical: menus, in-place editing, autocomplete, interruption, and colored output do not work as they do in a standalone terminal. This feature makes the embedded terminal behave like a real terminal emulator.

### User Story 1 - See colored, faithful terminal output (Priority: P1)

A developer runs an interactive `claude` session in the embedded terminal and sees its output rendered with the same colors and text styles they would see in a standalone terminal — colored prompts, highlighted diffs, bold headings, dimmed hints — instead of flat, uniform text. Full-screen (alternate-screen) interfaces redraw cleanly without leftover artifacts.

**Why this priority**: Faithful rendering is the most visible gap and is independently valuable — even before input is reworked, a developer can read `claude`'s output correctly and follow its interface. It is the foundation every other terminal interaction is displayed on.

**Independent Test**: Start a session and drive the terminal with output that uses color and styling (the `claude` TUI, or a command like a colored diff or `ls --color`). Confirm foreground/background colors and styles (bold, dim, italic, underline, reverse) render the same as in a standalone terminal, that default colors follow the app's light/dark theme, and that a full-screen interface redraws without artifacts.

**Acceptance Scenarios**:

1. **Given** a session whose process emits ANSI-colored output, **When** the terminal renders it, **Then** foreground and background colors (standard 16, bright, 256-color, and 24-bit truecolor) appear the same as in a standalone terminal.
2. **Given** a session whose process emits styled text, **When** the terminal renders it, **Then** bold, dim, italic, underline, strikethrough, reverse/inverse, and hidden styles are shown correctly.
3. **Given** the application is in light or dark mode, **When** the terminal renders text with no explicit color, **Then** the default foreground and background follow the active theme, and switching theme updates the terminal accordingly.
4. **Given** a process that uses a full-screen (alternate-screen) interface, **When** it redraws, repositions the cursor, clears the screen, or wraps lines, **Then** the terminal reflects those operations without leftover characters or misplaced content, and the cursor is visible at its current position.

---

### User Story 2 - Work interactively with the claude CLI (Priority: P1)

A developer focuses the embedded terminal and works with the interactive `claude` CLI exactly as in a standalone terminal: typing streams live to the process, arrow keys navigate menus, Tab and slash commands autocomplete, multi-line prompts and in-place line editing work, and Ctrl+C interrupts a running turn. The old "type a line and press Enter to send" box is gone.

**Why this priority**: Live, interactive input is the core purpose of the feature — without it the `claude` TUI cannot be operated. It is equally essential to the MVP as faithful rendering.

**Independent Test**: Focus the terminal, then operate `claude` interactively — navigate a menu with arrow keys, trigger slash-command autocomplete with Tab, edit a line in place, enter a multi-line prompt, and press Ctrl+C to interrupt a running turn. Confirm each behaves as in a standalone terminal and that keystrokes reach the process as they are pressed (no line buffering).

**Acceptance Scenarios**:

1. **Given** the terminal is focused, **When** the user presses a printable key, Enter, Backspace, Delete, Tab, Escape, an arrow key, Home/End/PageUp/PageDown, Insert, or a function key, **Then** the corresponding terminal input is delivered to the process immediately (character by character, not buffered into a line).
2. **Given** the terminal is focused, **When** the user presses a control chord (Ctrl+C, Ctrl+D, Ctrl+Z, Ctrl+R, Ctrl+U, Ctrl+W, and similar), **Then** the matching control input is delivered to the process (e.g. Ctrl+C interrupts a running turn).
3. **Given** the `claude` interactive UI is showing a menu or autocomplete, **When** the user navigates with arrow keys and confirms with Enter or Tab, **Then** navigation and selection behave exactly as in a standalone terminal.
4. **Given** the terminal is focused, **When** the user composes a multi-line prompt and edits earlier text in place, **Then** editing behaves as in a standalone terminal.

---

### User Story 3 - Keys reach the terminal only when it is focused (Priority: P2)

Key events go to the `claude` process only while the terminal is focused. When the terminal is not focused, the same keys drive the surrounding application (its shortcuts and navigation) and never reach the process. Focus is acquired by an explicit action, is clearly indicated, and can always be released without disrupting the session.

**Why this priority**: Without focus gating, application shortcuts would be swallowed whenever a session is open, or process input would leak while the user is navigating the app. Correct gating is what lets the terminal and the app coexist, but it builds on the input behavior of Story 2.

**Independent Test**: With a session open but the terminal unfocused, press application shortcuts and type — confirm the app responds and nothing reaches the process. Click the terminal to focus it (focus is visibly indicated), type, and confirm input now reaches the process. Use the documented focus-out action and confirm focus returns to the app without disrupting the running session.

**Acceptance Scenarios**:

1. **Given** a session is open and the terminal is NOT focused, **When** the user presses keys, **Then** the application handles them (existing shortcuts and navigation) and no input reaches the session's process.
2. **Given** the terminal is not focused, **When** the user performs the explicit focus action (e.g. clicks the terminal), **Then** the terminal gains focus and this is visually indicated.
3. **Given** the terminal is focused, **When** the user performs the documented focus-out action, **Then** focus returns to the application, subsequent keys drive the app again, and the session's process keeps running uninterrupted.
4. **Given** multiple sessions exist with one displayed and focused, **When** the user types, **Then** input reaches only the displayed session's process and never a background session's process.

---

### User Story 4 - Correct sizing, resize, and scrollback (Priority: P3)

The terminal tells the process how many rows and columns are actually visible, so the `claude` UI lays itself out to fit. When the developer resizes the window or the terminal pane, the terminal and the running interface reflow to the new size instead of staying at a fixed size. The developer can scroll back through earlier output.

**Why this priority**: Correct sizing and scrollback make longer, real-world sessions comfortable and prevent misaligned full-screen UIs, but a usable interactive terminal already exists without them, so this is the lowest of the priorities.

**Independent Test**: Start a session, note the `claude` UI fits the visible area, resize the window/pane, and confirm the interface reflows to the new size within a redraw. Produce more output than fits on screen and confirm the user can scroll back to review earlier output.

**Acceptance Scenarios**:

1. **Given** a session is displayed, **When** the terminal renders, **Then** the size reported to the process matches the visible character area (rows × columns), and the process lays out its UI to that size.
2. **Given** a session is displayed, **When** the user resizes the window or the terminal pane, **Then** the terminal and the running interface reflow to the new size, and the process is informed of the new size.
3. **Given** more output has been produced than fits in the visible area, **When** the user scrolls back, **Then** earlier output is shown, up to the configured scrollback limit.

---

### User Story 5 - Configure the terminal via Settings (Priority: P3)

A developer opens application Settings from a dropdown menu in the toolbar and adjusts the terminal scrollback limit. The chosen value is saved and applies to session terminals, and it is remembered across application restarts.

**Why this priority**: The terminal is usable with the default scrollback before any setting is changed, so configuration is an enhancement rather than a prerequisite — but it is the mechanism that makes the scrollback bound (User Story 4) a user choice instead of a hard-coded constant.

**Independent Test**: Open the toolbar dropdown menu, choose Settings, change the terminal scrollback limit, close the form, and confirm the terminal honors the new limit; restart the application and confirm the value is retained.

**Acceptance Scenarios**:

1. **Given** the application is open, **When** the user opens the toolbar dropdown menu, **Then** it includes a Settings item that opens a Settings form.
2. **Given** the Settings form is open, **When** the user views it, **Then** it shows the current terminal scrollback limit and allows changing it.
3. **Given** the user changes the scrollback limit and confirms, **When** the setting is saved, **Then** session terminals honor the new limit and the value persists across application restarts.
4. **Given** the Settings form is shown in light or dark mode, **When** it renders, **Then** it follows the existing Material Design theming and reuses the shared UI components of the app shell.

---

### Edge Cases

- How does the terminal behave when the process requests a color the host display cannot represent exactly (e.g. 24-bit truecolor on a limited backend)? (Assumption: approximate to the nearest available color.)
- What happens to the focus indicator and scroll position when the user switches to a different session and back? (The displayed session changes; input focus and scroll position belong to the currently displayed terminal.)
- What happens if the user presses a key the surrounding application also binds (e.g. Escape) while the terminal is focused? (Resolved: while focused, the key goes to the process; the focus-out action MUST NOT depend on a key the process consumes, so the user is never trapped.)
- What happens when the user pastes multi-line text into the terminal — is it sent as typed input or interpreted line by line? (Assumption: inserted as input; bracketed-paste is honored when the process requests it, so newlines are not auto-submitted.)
- What happens when the window is resized to an extremely small size (fewer rows/columns than the process expects)? (The terminal reports the actual size; the process adapts or applies its own minimum-size handling.)
- What happens to scrollback when it exceeds the bounded history length? (Oldest lines are dropped.)
- What happens when the process switches into and out of a full-screen (alternate-screen) mode? (The main-screen scrollback is preserved and restored on exit.)
- What does the mouse wheel do when the process has enabled mouse reporting — scroll the local scrollback or forward to the process? (Assumption: forwarded to the process while it owns mouse reporting / is on the alternate screen; otherwise it scrolls the local scrollback.)
- What happens to keystrokes typed while the focused session's process is not Running (starting/restarting/failed)? (Resolved: discarded, not buffered; the session status is shown; scrolling, selection, and copy still work.)
- How is a very high output rate (a process printing large volumes quickly) displayed without the UI becoming unresponsive? (Resolved: output is coalesced into throttled redraws; input and scrolling stay responsive (≤~100 ms) and memory stays bounded by the configured scrollback limit; intermediate frames may be coalesced but the final screen state is correct.)

## Requirements *(mandatory)*

### Functional Requirements

#### Rendering

- **FR-001**: The terminal MUST render process output with ANSI foreground and background colors, including the standard 16 colors, the bright 16, the 256-color palette, and 24-bit truecolor.
- **FR-002**: The terminal MUST render text styles emitted by the process: bold, dim/faint, italic, underline, strikethrough, reverse/inverse, and hidden/concealed.
- **FR-003**: When output specifies no explicit color, the terminal's default foreground and background MUST follow the application's active light/dark theme, and MUST update when the theme changes.
- **FR-004**: The terminal MUST display the cursor at its current position and reflect the cursor visibility controlled by the process.
- **FR-005**: The terminal MUST correctly render full-screen (alternate-screen) interfaces — honoring screen clears, cursor positioning, line wrapping, and redraws — without leaving stale characters or misplaced content.
- **FR-005a**: Under sustained high-volume output, the system MUST coalesce rapid updates into throttled redraws so that input and scrolling remain responsive and memory stays bounded by the configured scrollback limit. Intermediate frames MAY be coalesced/skipped, but the final rendered screen state MUST match the process's output.

#### Input & key propagation

- **FR-006**: When the terminal is focused, the system MUST forward key presses to the displayed session's process the way a terminal encodes them: printable characters, Enter/Return, Backspace, Delete, Tab, Escape, arrow keys, Home/End/PageUp/PageDown, Insert, and function keys.
- **FR-007**: When the terminal is focused, the system MUST forward control-key chords (including at least Ctrl+C, Ctrl+D, Ctrl+Z, Ctrl+R, Ctrl+U, Ctrl+W) as their corresponding control input, so that, for example, Ctrl+C interrupts a running turn.
- **FR-008**: The system MUST deliver keystrokes to the process live, as each key is pressed, with no line buffering. The prior line-buffered input box (type a line, press Enter to send) MUST be removed.
- **FR-009**: When the terminal is NOT focused, key events MUST NOT reach any session process and MUST be handled by the surrounding application (existing shortcuts and navigation).
- **FR-010**: The terminal MUST gain focus only through an explicit user action (for example, clicking the terminal), and the focused state MUST be visually indicated.
- **FR-011**: The system MUST let the user move focus out of the terminal back to the application in two ways: by clicking outside the terminal pane, and by a reserved application keyboard shortcut that is never forwarded to the process (so a keyboard-only user is never trapped). Neither action may rely on a key the process itself consumes, and neither may disrupt or terminate the running session.
- **FR-012**: Keystrokes MUST reach only the currently displayed, focused session's process; no input may leak to any background session's process (preserving the session isolation of feature 005, FR-019).
- **FR-012a**: When the focused session's process is not in the Running state (e.g. starting, restarting, or failed per feature 005), the system MUST discard typed keystrokes rather than buffering them for later delivery, and MUST surface the session's current status so the user understands why input is not accepted. Focus, scrolling, selection, and copy MUST remain available in these states.
- **FR-013**: Users MUST be able to copy selected terminal text and paste text into the terminal through all of the following, and the terminal MUST intercept these gestures rather than forwarding them to the process:
  - platform-standard terminal chords — Ctrl+Shift+C / Ctrl+Shift+V on Linux and Windows, Cmd+C / Cmd+V on macOS;
  - auto-copy of the current selection to the clipboard, with middle-click paste;
  - a right-click context menu (and/or on-screen controls) offering copy and paste.
- **FR-013a**: When the terminal is focused and the process has enabled mouse reporting, the system MUST forward mouse events (button clicks, drag/movement as requested by the active reporting mode, and wheel/scroll) to the process encoded the way a terminal would. When the process has not enabled mouse reporting, dragging the mouse MUST select terminal text instead.
- **FR-013b**: Even while the process has mouse reporting enabled, the user MUST be able to select terminal text with the mouse by holding a modifier (e.g. Shift) while dragging, so copy remains available in all cases.

#### Sizing & scrollback

- **FR-014**: The size the terminal reports to the process (rows × columns) MUST match the visible character area, so the process lays out its interface to fit.
- **FR-015**: When the window or terminal pane is resized, the terminal MUST reflow and report the updated size to the process, rather than remaining at a fixed size.
- **FR-016**: The terminal MUST retain a bounded scrollback history and allow the user to scroll back through earlier output. The maximum length MUST be the user-configured scrollback limit (FR-020), defaulting to a sensible value until changed.

#### Terminal settings

- **FR-019**: The system MUST provide a Settings item within a dropdown menu in the toolbar that opens a Settings form.
- **FR-020**: The Settings form MUST display the current terminal scrollback limit and allow the user to change it. The configured limit MUST take effect for session terminals (at minimum for sessions displayed after the change).
- **FR-021**: The scrollback limit MUST have a sensible default when never configured, and the configured value MUST be persisted locally and restored across application restarts (consistent with local-first storage).
- **FR-022**: The Settings form and the toolbar menu MUST follow the application's existing Material Design light/dark theming and reuse the shared UI component library rather than introducing bespoke one-off widgets.

#### Preservation & parity

- **FR-017**: This feature MUST NOT change the session lifecycle, isolation, persistence, or auto-restart behavior established in feature 005 (its FR-012 through FR-023a); it changes only how a session's terminal renders output and accepts input.
- **FR-018**: Colored/styled rendering, key encoding, focus gating, resize, and copy/paste MUST behave equivalently on Linux, macOS, and Windows.

### Key Entities

- **Terminal focus state**: Whether the embedded terminal currently holds input focus. Determines whether key events are forwarded to the displayed session's process or handled by the application. At most one terminal (the displayed session's) holds focus at a time.
- **Styled screen cell**: A single visible character position, carrying its character plus display attributes — foreground color, background color, and style flags (bold, dim, italic, underline, strikethrough, reverse, hidden) — as interpreted from the process's output stream.
- **Scrollback buffer**: The bounded history of prior output lines for a session's terminal that the user can scroll back through, distinct from the currently visible screen. Its maximum length is the configured scrollback limit.
- **Application settings**: User-configurable preferences persisted locally and restored across restarts. For this feature it includes the terminal scrollback limit; it is structured to hold future settings without reworking the surrounding flow.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: A developer can complete an end-to-end interactive `claude` task inside the embedded terminal — navigating a menu with arrow keys, triggering slash-command autocomplete, entering a multi-line prompt, and interrupting a running turn with Ctrl+C — with no step behaving differently from a standalone terminal.
- **SC-002**: For colored, styled output shown side by side with a standalone terminal, 100% of the color categories (16, bright, 256-color, truecolor) and style categories (bold, dim, italic, underline, strikethrough, reverse, hidden) render equivalently.
- **SC-003**: While the terminal is unfocused, 0% of key presses reach the session process and 100% are handled by the application; while focused, 100% of the key set in FR-006/FR-007 reaches the process.
- **SC-004**: After the user resizes the window, the terminal and any running full-screen interface reflow to the new size within one visible redraw, with no truncated or incorrectly wrapped content.
- **SC-005**: In 100% of attempts, the user can move focus out of the terminal back to the application using the documented action, without terminating or disrupting the running session.
- **SC-006**: All of the above behave identically on Linux, macOS, and Windows.
- **SC-007**: A user can open Settings from the toolbar menu, change the terminal scrollback limit, and see the terminal honor the new limit; after an application restart the changed value is still in effect in 100% of attempts.
- **SC-008**: Under sustained high-volume output, input and scrolling remain responsive with ≤~100 ms perceived latency, memory usage stays bounded by the configured scrollback limit, and the final rendered screen matches the process's actual output.

## Assumptions

- The embedded terminal continues to run the `claude` CLI as established in feature 005; this feature does not introduce a general-purpose shell prompt.
- Live keystroke streaming fully replaces the line-buffered input box; there is no separate "compose a line and send" affordance.
- Focus is acquired by clicking the terminal and released either by clicking outside the terminal pane or via a reserved application keyboard shortcut that is never forwarded to the process. Escape is forwarded to the process (the `claude` UI uses it) and is therefore not the focus-out mechanism. The exact reserved shortcut is a planning-phase decision.
- Scrollback is bounded by a user-configurable limit set in the Settings form, with a sensible default (e.g. 10,000 lines) until changed; unlimited history is out of scope. The limit is an in-memory bound on live output and is not itself persisted scrollback content (consistent with feature 005, which does not persist terminal scrollback).
- The toolbar already hosts a dropdown menu and the application already has a local settings store; this feature adds a Settings item/form and the scrollback preference rather than introducing a new persistence mechanism.
- Paste inserts text as terminal input; bracketed-paste mode is honored when the process requests it, so pasted newlines are not auto-submitted.
- 24-bit truecolor is supported; when the display backend cannot represent a color exactly, it is approximated to the nearest available color, which is acceptable.
- The terminal presents an xterm-compatible terminal type to the process (consistent with feature 005), so standard escape sequences for colors, styles, keys, and resize apply.
- Worktree removal and any change to session persistence remain out of scope, consistent with feature 005.
