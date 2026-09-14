# Feature Specification: Real Terminal Behavior for Embedded Session Terminals

**Feature Branch**: `006-real-terminal-emulator`

**Created**: 2026-07-16

**Status**: Closed (implemented and shipped; every task through Phase 13, including T042, is done). The manual quickstart pass is **fully run** — [evidence](./evidence/gui-pass-2026-08-25.md), 2026-08-25, superseding the [partial pass](./evidence/partial-gui-pass.md) of 2026-08-20. Most steps pass. The three clauses that did not were filed as bugs and are **fixed** (2026-09-13, Phase 13): the focused terminal now draws the 018 focus ring ([BUG-005](./bugs/BUG-005.md), FR-010/FR-010b), paste is bracketed when the process asks for it ([BUG-006](./bugs/BUG-006.md), FR-013d), and a copy chord with nothing selected leaves the clipboard alone ([BUG-004](./bugs/BUG-004.md), FR-013c). [BUG-007](./bugs/BUG-007.md) (FR-013e: a single click selected and copied the clicked cell) was patched 2026-09-14; its Phase 14 (T074–T076) is open. The pass also surfaced [010 BUG-014](../010-daemon-session-persistence/bugs/BUG-014.md). Four claims remain out of reach of a headless pass and are recorded as unrun, not passed: SC-008's perceived latency on a software rasteriser, sub-line touchpad flicks, interrupting a running `claude` turn, and SC-006's non-Linux platforms (carried by T041's CI matrix).

**Input**: User description: "Improve the terminal behavior. It should act as a real terminal and support colors, same as a regular terminal. Propagate the shortcut key events, only when focused on the terminal. Should generally allow for regular work with the claude CLI and ensure that it is displayed in a proper way."

## Clarifications

### Session 2026-07-16

- Q: How does the user move focus out of the terminal back to the app, given that Escape is forwarded to the process? → A: Click outside the terminal pane, or a reserved application keyboard shortcut that is never forwarded to the process (the exact key is chosen during planning).
- Q: Should the terminal forward mouse events to the process, and how does that coexist with text selection/copy? → A: Forward mouse events when the process enables mouse reporting; mouse text selection and copy are still supported — dragging selects when no mouse reporting is active, and holding a modifier (e.g. Shift) selects while mouse reporting is active.
- Q: How is copy/paste triggered by keyboard when the terminal is focused, given that Ctrl+C / Ctrl+V go to the process? → A: Support all common methods — platform-standard terminal chords (Ctrl+Shift+C / Ctrl+Shift+V; Cmd+C / Cmd+V on macOS), auto-copy of the selection with middle-click paste, and a right-click context menu / on-screen controls. The terminal intercepts these gestures and does not forward them to the process.
- Q: What is the scrollback history limit? → A: It is user-configurable, not a fixed constant. A new Settings form — opened from a Settings item in a toolbar dropdown menu — lets the user set the terminal scrollback limit. It ships with a sensible default (e.g. 10,000 lines) and the configured value is persisted locally across restarts.
- Q: What happens to keystrokes typed into a focused terminal while its session's process is not Running (starting, restarting, or failed)? → A: Discard them (no buffering) and show the session status so the user knows why input is not accepted; focus, scrolling, selection, and copy still work.
- Q: What responsiveness guarantee applies under very high output volume? → A: Coalesce rapid output into throttled redraws so input and scrolling stay responsive (≤~100 ms perceived latency); memory stays bounded by the configured scrollback limit; intermediate frames may be coalesced but the final screen state is always correct.

### Session 2026-07-17 (bugfix BUG-001)

- Q: Selecting a session shows its terminal but leaves it unfocused, so the user must click the terminal before they can type. What is expected? → A: Making a session the displayed session — starting it or selecting it in the sidebar — MUST automatically focus that session's terminal so the user can interact with `claude` immediately. This supersedes the earlier rule that focus is acquired only by an explicit click and that `SessionSelected` clears focus. Explicit focus/release still work, only the displayed session's terminal is ever focused, and the write-gate (bytes only while `Running`) is unchanged.
- Q: If selecting a session auto-focuses the terminal, what happens to the application's own keyboard shortcuts (which are gated to the unfocused state, FR-009)? → A: While a freshly-selected terminal holds focus, the app's global shortcuts are owned by the terminal (as when focus is acquired by click). The user regains the app shortcuts by releasing focus (click outside the pane or the reserved `Ctrl+Shift+E` / `Cmd+Shift+E` chord, FR-011); this trade-off is accepted so that interaction is immediate.

### Session 2026-07-20 (bugfix BUG-002)

- Q: FR-016 required scrolling to be driven "at least by the mouse wheel". Does that cover a touchpad, whose scroll travel arrives continuously in units smaller than one text line? → A: No — the wording named only the wheel and said nothing about how travel maps to lines, so quantizing each event on its own and discarding anything under a line conformed to the spec while leaving touchpad scrolling completely non-functional. Scrolling MUST be driven by continuous, high-resolution sources as well as a discrete wheel, and sub-line travel MUST accumulate across events until it reaches a whole line instead of being discarded (FR-016b).
- Q: Does this distinction matter for the platform-parity requirement? → A: Yes. The same code behaves differently by windowing system, not by operating system: X11 reports touchpad scroll as discrete line events (so it worked), Wayland reports high-resolution pixel travel (so it did not). Because parity is judged per platform, a defect that reproduces only under one windowing system passed unnoticed. FR-016b therefore binds on every supported platform regardless of how that platform reports scroll travel.

### Session 2026-08-09 (bugfix BUG-003)

- Q: FR-014 requires the reported size to match the visible area and FR-015 requires a report when that area is *resized*. Which event gives a newly started session its size? → A: None was named, and none existed. Since feature 010 the process is spawned by a separate long-lived service that has never seen the terminal area, so a session started while the visible size had not changed was spawned at the service's own default and stayed there until an unrelated window resize. FR-014a now makes the *start* of a session a size-conveying event in its own right, independent of any change.
- Q: What must happen to a size reported for a session whose process does not exist yet (it has not been started, or its start is still in flight)? → A: It must be retained and applied when that process is spawned — a size is state belonging to the session, not a command that requires a live process to receive it. Discarding it for want of a process is what made the defect routine rather than rare, because starting a session is asynchronous and a client's size report legitimately arrives before the process does.
- Q: Once the start conveys a size, what does a *freshly launched* application convey — it has displayed no session, so what has it measured? → A: Nothing, as built: the measurement lived in the widget that renders a session's output, which is mounted only after that session's first frame arrives. So the one path FR-014a exists for — the first session clicked after launch — was the one path still unable to state a size. FR-014b moves the measurement to the terminal *area*, which exists as soon as a session is displayed and therefore before its process is spawned. The residual case (a spawn that completes before the application has drawn a single frame) is covered by FR-014a's retention rule rather than by ordering.

### Session 2026-09-13 (bugfix BUG-004, BUG-005, BUG-006)

- Q: FR-013 says users can copy *selected* text. What does a copy gesture do when nothing is selected? → A: Nothing. The chord wrote an empty string and destroyed whatever the user had copied elsewhere, while the auto-copy and context-menu paths already declined to write. FR-013c states the no-op for all three surfaces, so none of them is correct by accident.
- Q: FR-010 requires the focused state to be visually indicated but never said how. What does the indicator look like, and may it cost the terminal any space? → A: The design system's own focus indicator (018 FR-022: a 3dp `secondary` outline), drawn in a gutter the pane reserves whether or not it is focused. Reserving the gutter only while focused would resize the process every time focus moved; drawing the outline over the character area would clip the first column's glyphs. The gutter costs less than one column and one row (FR-010b).
- Q: The bracketed-paste rule lived only in an assumption and in the quickstart. Where does it bind, and how is a pasted end marker handled? → A: FR-013d makes it a requirement on every paste gesture. End markers inside the pasted text are removed — repeatedly, until none remain, so text built to reassemble a marker once one is removed cannot close the block either. Other bytes, including other escape sequences, are left as pasted: the process receives them inside the block and is the one that decides what pasted text means.

### Session 2026-09-14 (bugfix BUG-007)

- Q: FR-003 makes the default colours follow the theme. Does that bind only what the terminal draws, or also what it tells a program that asks? → A: Both. A program that adapts to its background (`claude`'s `auto` theme, `vim`, `bat`) learns the background by asking the terminal, and chooses its palette from the answer. A pane drawn dark that answers "light" gets black text drawn on it — which is what happened, because since feature 010 the question reaches the session service, and it answered from a fixed table whose fallback is light grey. FR-003a makes the answer the colours the pane is actually painted with.
- Q: The session service does not render and has never been told the theme. Should it forward the question to the window and wait? → A: No. Terminal-internal replies stay in the service (010 `contracts/protocol.md` §8): a round trip to a window would stall the program on every query, and would have no answer at all while no window is connected. The window instead **tells** the service its resolved scheme — on every connection, before it attaches a project or starts a session, and again whenever the scheme changes — and the service answers from the last scheme it was told. Both sides derive the colours from one definition in the shared core, so the answer cannot drift from the drawing.
- Q: What does the service answer before any window has told it a scheme, and which window wins when several have? → A: Before any report it answers for the light scheme, the scheme the application resolves when it knows nothing about the OS preference (`003-material-design-layout` `contracts/theme-behavior.md`, FR-018 fallback). The last report wins. Windows share one theme preference and one OS, so two windows reporting different schemes is not a state the application produces; last-writer-wins is sufficient.
- Q: FR-013a says *dragging* selects text. What does a left click that never leaves the cell it pressed select? → A: Nothing. As built, the press started a character selection whose two inclusive ends were the pressed cell, so a click highlighted that cell until the next press and auto-copied its character over whatever the clipboard held. A click clears the previous selection and selects nothing (FR-013e). Once a drag has entered another cell the selection is real, and dragging back onto the pressed cell selects that single cell, so one-character selections stay reachable. Double- and triple-click are unchanged: their word and line are selected from the press.

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
- What does a program that is already running see when the user switches theme? (Bugfix BUG-007: every query it makes afterwards gets the new colours — FR-003a — but the terminal does not *notify* it of the change. `claude` learns of a change only through DEC private mode 2031's `CSI ? 997 ; 1|2 n` report, which the terminal does not implement, so a `claude` started before the switch keeps the palette it chose until it is restarted. That matches most standalone terminals, which also lack mode 2031; supporting it is a follow-up, not part of this fix.)
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
- **FR-003**: When output specifies no explicit color, the terminal's default foreground and background MUST follow the application's active light/dark theme, and MUST update when the theme changes. *(Bugfix BUG-007: this binds what programs are told, too — FR-003a.)*
- **FR-003a** (bugfix BUG-007): A program that asks the terminal for its default foreground, background or cursor colour (`OSC 10`, `OSC 11`, `OSC 12` with `?`) MUST receive the colour the terminal paints for that role under the application's active scheme — the default foreground, the default background, and the default foreground for the cursor block. After the scheme changes, every later query MUST receive the new scheme's colours. The reply MUST be written by the session service without waiting on a window (010 `contracts/protocol.md` §8), from the scheme a window last reported; each window MUST report its resolved scheme on every connection before it attaches a project or starts a session, and again on every change. Until any window has reported, the reply is for the light scheme. Palette queries (`OSC 4`) are unchanged by this requirement.
- **FR-004**: The terminal MUST display the cursor at its current position and reflect the cursor visibility controlled by the process.
- **FR-005**: The terminal MUST correctly render full-screen (alternate-screen) interfaces — honoring screen clears, cursor positioning, line wrapping, and redraws — without leaving stale characters or misplaced content.
- **FR-005a**: Under sustained high-volume output, the system MUST coalesce rapid updates into throttled redraws so that input and scrolling remain responsive and memory stays bounded by the configured scrollback limit. Intermediate frames MAY be coalesced/skipped, but the final rendered screen state MUST match the process's output.

#### Input & key propagation

- **FR-006**: When the terminal is focused, the system MUST forward key presses to the displayed session's process the way a terminal encodes them: printable characters, Enter/Return, Backspace, Delete, Tab, Escape, arrow keys, Home/End/PageUp/PageDown, Insert, and function keys.
- **FR-007**: When the terminal is focused, the system MUST forward control-key chords (including at least Ctrl+C, Ctrl+D, Ctrl+Z, Ctrl+R, Ctrl+U, Ctrl+W) as their corresponding control input, so that, for example, Ctrl+C interrupts a running turn.
- **FR-008**: The system MUST deliver keystrokes to the process live, as each key is pressed, with no line buffering. The prior line-buffered input box (type a line, press Enter to send) MUST be removed.
- **FR-009**: When the terminal is NOT focused, key events MUST NOT reach any session process and MUST be handled by the surrounding application (existing shortcuts and navigation).
- **FR-010**: ~~The terminal MUST gain focus only through an explicit user action (for example, clicking the terminal), and the focused state MUST be visually indicated.~~ (Superseded — bugfix BUG-001: the "only through an explicit user action" clause forbade auto-focusing a session's terminal on selection.) The terminal MUST gain focus **both** (a) automatically when a session becomes the displayed session — when the user starts a session or selects one in the sidebar (feature 005 FR-010/FR-015) — so the user can interact with the AI CLI immediately, **and** (b) through an explicit user action such as clicking the terminal pane. The focused state MUST be visually indicated. At most one terminal — the displayed session's — is focused at a time, and focus MUST remain releasable at any time per FR-011. *(Bugfix BUG-005: the clause "the focused state MUST be visually indicated" was never drawn — see FR-010b for the form it takes.)*
- **FR-010b** (bugfix BUG-005): The indication FR-010 requires MUST be persistent while the terminal holds focus and absent while it does not, and MUST follow the design system's focus indicator for an element that holds keyboard focus (`018-material3-visual-system` FR-022, contract §5: a 3dp outline in the `secondary` role) in both light and dark schemes. It MUST NOT change the terminal's geometry: the pane's layout and the character area reported to the process (FR-014) MUST be the same whether it is focused or not, so gaining or releasing focus never resizes the process. The indicator MUST NOT cover terminal content.
- **FR-010a** (bugfix BUG-001): Auto-focus on select/start MUST NOT weaken any other focus guarantee. Only the displayed session's terminal is focused (never a background session, FR-012); keystrokes are still delivered to the process only while it is `Running` (FR-012a); and while the auto-focused terminal holds focus the application's global shortcuts are owned by the terminal (as with click-acquired focus, FR-009) until the user releases focus (FR-011). Selecting a session in the sidebar (a click outside the pane, which by FR-011 would release focus) MUST result in the newly-selected session's terminal being focused — the auto-focus of the selected session takes precedence over the click-outside release.
- **FR-011**: The system MUST let the user move focus out of the terminal back to the application in two ways: by clicking outside the terminal pane, and by a reserved application keyboard shortcut that is never forwarded to the process (so a keyboard-only user is never trapped). Neither action may rely on a key the process itself consumes, and neither may disrupt or terminate the running session.
- **FR-012**: Keystrokes MUST reach only the currently displayed, focused session's process; no input may leak to any background session's process (preserving the session isolation of feature 005, FR-019).
- **FR-012a**: When the focused session's process is not in the Running state (e.g. starting, restarting, or failed per feature 005), the system MUST discard typed keystrokes rather than buffering them for later delivery, and MUST surface the session's current status so the user understands why input is not accepted. Focus, scrolling, selection, and copy MUST remain available in these states.
- **FR-013**: Users MUST be able to copy selected terminal text and paste text into the terminal through all of the following, and the terminal MUST intercept these gestures rather than forwarding them to the process:
  - platform-standard terminal chords — Ctrl+Shift+C / Ctrl+Shift+V on Linux and Windows, Cmd+C / Cmd+V on macOS;
  - auto-copy of the current selection to the clipboard, with middle-click paste;
  - a right-click context menu (and/or on-screen controls) offering copy and paste.
- **FR-013a**: When the terminal is focused and the process has enabled mouse reporting, the system MUST forward mouse events (button clicks, drag/movement as requested by the active reporting mode, and wheel/scroll) to the process encoded the way a terminal would. When the process has not enabled mouse reporting, dragging the mouse MUST select terminal text instead.
- **FR-013b**: Even while the process has mouse reporting enabled, the user MUST be able to select terminal text with the mouse by holding a modifier (e.g. Shift) while dragging, so copy remains available in all cases.
- **FR-013c** (bugfix BUG-004): A copy gesture made while nothing is selected — or while the selection holds no text — MUST leave the clipboard's existing contents untouched. This binds all three copy surfaces in FR-013 (the chord, auto-copy on release, and the context menu's Copy); "copy nothing" is a no-op, never a write of empty text.
- **FR-013d** (bugfix BUG-006): When the process has enabled bracketed-paste mode, text pasted through any of FR-013's paste gestures (the chord, middle-click, and the context menu's Paste) MUST be delivered to it as one bracketed block — the bracketed-paste start marker, the text, the end marker — so the process receives it as pasted text rather than as typed keys, and embedded newlines are not acted on as Enter. Any end-marker sequence contained in the pasted text itself MUST be removed before delivery, so pasted content cannot close the block early and have its remainder run as keystrokes. When the process has not enabled the mode, the text MUST be delivered unchanged.
- **FR-013e** (bugfix BUG-007): A left-button press that is released before the pointer has entered any other cell — a click, as opposed to a drag — MUST select nothing: no cell is shown as selected, and nothing is copied, so the clipboard's existing contents are untouched (FR-013c). The click still clears any existing selection. Once a drag has entered another cell, the cells it spans are selected as FR-013a describes, including the pressed cell alone when the drag returns to it. Double- and triple-click (word and line selection) are unaffected.

#### Sizing & scrollback

- **FR-014**: The size the terminal reports to the process (rows × columns) MUST match the visible character area, so the process lays out its interface to fit. *(Bugfix BUG-003: this holds from the process's **first** output, not only after a resize — see FR-014a for the event that establishes it.)*
- **FR-014a** (bugfix BUG-003): A session's process MUST be started at the size of the terminal area it will be displayed in, so its first rendered output already fits. Starting, resuming, selecting, or creating a session MUST convey the current visible size to whatever spawns that process, and a size conveyed for a session whose process does not exist yet MUST be retained and applied when it is spawned, rather than discarded for want of a running process. A session that is not currently displayed at a known size (none has ever been reported) MUST still start at a defined default. This requirement binds independently of FR-015: a session started while the visible size has not *changed* must still be started at that size.
- **FR-014b** (bugfix BUG-003, second finding): The visible size MUST be measured from the terminal **area** — the region a session's output will occupy — and MUST be available from the moment a session becomes the displayed one, not from the moment its first output arrives. Measuring it from the rendered output instead means no size exists during the interval between requesting a session's start and receiving its first frame, which is precisely the interval in which the process is spawned; a freshly-launched application, which has displayed no session at all, then has nothing to state under FR-014a. Where the size cannot be established before the start is requested, the retention rule in FR-014a is what carries it — a size reported while the spawn is still in flight MUST still reach that spawn.
- **FR-015**: When the window or terminal pane is resized, the terminal MUST reflow and report the updated size to the process, rather than remaining at a fixed size. *(Bugfix BUG-003: a size change is one trigger, not the only one — it MUST NOT be the sole path by which a process ever learns its size, see FR-014a.)*
- **FR-016**: The terminal MUST retain a bounded scrollback history and allow the user to scroll back through earlier output. Scrolling back MUST reposition the visible viewport over the scrollback — revealing earlier lines at the top and shifting the current content down — rather than leaving the visible text in place. ~~Scrolling MUST be driven at least by the mouse wheel.~~ (superseded by FR-016b — BUG-002: naming only the wheel left continuous scroll sources unspecified.) Scrolling MUST be driven by both discrete and continuous pointer scroll sources, per FR-016b. The maximum length MUST be the user-configured scrollback limit (FR-020), defaulting to a sensible value until changed.
- **FR-016b**: Scrolling MUST be driven by a discrete mouse wheel *and* by continuous, high-resolution scroll sources (touchpads and precision pointing devices). Where the platform reports scroll travel in a unit finer than one text line, the system MUST accumulate travel smaller than a line across successive events until it amounts to at least one line, rather than discarding it. A gesture composed entirely of sub-line increments MUST therefore still scroll. Reversing scroll direction MUST NOT allow travel banked before the reversal to offset travel after it. This requirement applies both when scrolling the local scrollback and when forwarding wheel events to a mouse-reporting process (FR-013a), and it MUST hold on every supported platform regardless of how that platform reports scroll travel.
- **FR-016a**: While the user is scrolled back into the history, the terminal MUST display a scrollbar on the pane indicating the viewport's position and size within the scrollback. The scrollbar MUST be draggable to reposition the view and MUST support clicking the track to page through the history. It MUST be hidden while the terminal is parked at the live bottom (nothing scrolled back), and MUST follow the application's Material Design light/dark theming (FR-022).

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
- **SC-009** (bugfix BUG-001): In 100% of session starts and sidebar selections, the newly-displayed session's terminal is focused immediately afterward, so the user's next keystroke reaches that session's `claude` process (when it is `Running`) with no intervening click; releasing focus (click-outside / reserved chord) still returns keys to the application in 100% of attempts.
- **SC-010** (bugfix BUG-002): A user scrolling the terminal with a touchpad moves the viewport through the scrollback in 100% of attempts, on every supported platform and windowing system — including those reporting scroll travel continuously in units smaller than one text line. No quantity of sub-line scroll travel is discarded without eventually moving the viewport.
- **SC-011** (bugfix BUG-003): In 100% of session starts, resumes, selections, and creations — **including the first session started after the application launches** — the session's process lays out its interface for the full visible terminal area from its first output — with no window or pane resize performed by the user, and regardless of whether the visible size changed since the previous session. A session whose process is respawned (crash restart) or joined by an additional terminal instance comes back at that same size, not at a default.
- **SC-012** (bugfix BUG-004, BUG-006): In 100% of copy gestures made with nothing selected, the clipboard holds afterwards exactly what it held before; and in 100% of multi-line pastes into a process that has enabled bracketed paste, **no** pasted line is executed by the paste itself — the whole block waits at the process's input until the user submits it.
- **SC-013** (bugfix BUG-005): A user can tell from the screen alone, in both light and dark schemes, whether the terminal holds the keyboard: a focused and an unfocused pane differ visibly in 100% of observations, and the terminal's reported size is identical in both.
- **SC-014** (bugfix BUG-007): In both schemes, 100% of default-colour queries (`OSC 10/11/12`) are answered with the colours the pane is painted with, including queries made after a theme change; and `claude` on its default `auto` theme, started in a dark-scheme terminal, draws its body text legibly on the pane in 100% of observations.
- **SC-014** (bugfix BUG-007): In 100% of single left clicks on terminal text without a drag, no cell is shown as selected afterwards and the clipboard holds exactly what it held before the click.

## Assumptions

- The embedded terminal continues to run the `claude` CLI as established in feature 005; this feature does not introduce a general-purpose shell prompt.
- Live keystroke streaming fully replaces the line-buffered input box; there is no separate "compose a line and send" affordance.
- Focus is acquired by clicking the terminal and released either by clicking outside the terminal pane or via a reserved application keyboard shortcut that is never forwarded to the process. Escape is forwarded to the process (the `claude` UI uses it) and is therefore not the focus-out mechanism. The exact reserved shortcut is a planning-phase decision.
- Scrollback is bounded by a user-configurable limit set in the Settings form, with a sensible default (e.g. 10,000 lines) until changed; unlimited history is out of scope. The limit is an in-memory bound on live output and is not itself persisted scrollback content (consistent with feature 005, which does not persist terminal scrollback).
- The toolbar already hosts a dropdown menu and the application already has a local settings store; this feature adds a Settings item/form and the scrollback preference rather than introducing a new persistence mechanism.
- Paste inserts text as terminal input; bracketed-paste mode is honored when the process requests it, so pasted newlines are not auto-submitted. *(Bugfix BUG-006: this assumption was the only place the obligation was stated, so no task was ever generated for it — it is now a requirement, FR-013d.)*
- 24-bit truecolor is supported; when the display backend cannot represent a color exactly, it is approximated to the nearest available color, which is acceptable.
- The terminal presents an xterm-compatible terminal type to the process (consistent with feature 005), so standard escape sequences for colors, styles, keys, and resize apply.
- Worktree removal and any change to session persistence remain out of scope, consistent with feature 005.
- Auto-focusing the displayed session's terminal on select/start (bugfix BUG-001) is the accepted trade-off for immediate interaction: while that terminal holds focus the app's global shortcuts are owned by the terminal until the user releases focus (FR-011). The release mechanisms and the `Running`-only write-gate guarantee the user is never trapped and no keys leak to a non-`Running` or background process.

**Bugfix**: 2026-07-17 — BUG-001 Selecting or starting a session now auto-focuses its terminal so the user can interact with the AI CLI immediately. FR-010 amended (auto-focus on select/start), FR-010a added (guarantees preserved + select-precedence over click-outside release), SC-009 added, plus a Clarifications entry and an assumption. `contracts/focus-model.md` transition table updated accordingly.

**Bugfix**: 2026-07-20 — BUG-002 Touchpad scrolling was completely non-functional wherever the platform reports scroll travel in units finer than one text line, because each event was quantized to whole lines on its own and anything under a line was discarded. FR-016's "at least by the mouse wheel" clause superseded (struck through) and FR-016b added, requiring continuous/high-resolution scroll sources and mandating that sub-line travel accumulate across events rather than being discarded — on both the local-scrollback and mouse-reporting (FR-013a) paths. SC-010 added, plus a Clarifications entry recording the discrete-vs-continuous distinction and why it escaped platform-parity review. `contracts/terminal-render-input.md` wheel rule updated accordingly.

**Bugfix**: 2026-08-09 — BUG-003 A session's process was spawned at a fixed default and only ever learned the real terminal size from a *change* to it, so any session started while the window sat still ran at 100×30 inside a much larger pane until the user resized the window. FR-014a added (the start of a session conveys the visible size; a size reported for a session with no process is retained and applied at spawn; a default covers "never reported"), FR-014 and FR-015 annotated to point at it, SC-011 added (covering respawn and additional terminal instances too), plus a Clarifications entry recording why the invariant held for free before feature 010 moved spawning behind the session service. `contracts/terminal-render-input.md` auto-resize rule updated accordingly. The size's service-side half is specified in `010-daemon-session-persistence` FR-020a. **Extended the same day** with FR-014b, after the first fix left the cold-start case open: the measurement lived in the widget that draws a session's output, which is not mounted until that output exists, so a freshly-launched application had measured nothing at the moment it started its first session. The measurement now wraps the terminal area itself and exists from the frame a session is displayed.

**Bugfix**: 2026-09-13 — BUG-004, BUG-005, BUG-006 Three clauses the 2026-08-25 quickstart pass found unmet. FR-013c added (a copy gesture with nothing selected leaves the clipboard untouched, on all three copy surfaces — BUG-004). FR-010 annotated and FR-010b added (the focus indication FR-010 already required is the design system's 3dp `secondary` focus outline, drawn in a gutter reserved at every focus state so focus never resizes the process and the outline never covers content — BUG-005). FR-013d added (paste into a process that enabled bracketed paste is delivered as one bracketed block, with pasted end markers removed; the obligation previously existed only as an assumption — BUG-006). SC-012 and SC-013 added, plus a Clarifications entry for each decision.

**Bugfix**: 2026-09-14 — BUG-007 A program asking the terminal for its background was told light grey in a dark pane, so `claude`'s `auto` theme drew black text on black. FR-003 annotated and FR-003a added (default-colour queries are answered with the colours the pane paints under the active scheme, by the session service, from the scheme a window last reported on connection and on change; light until any report). SC-014 added, plus a Clarifications entry. The wire half is specified in `010-daemon-session-persistence` `contracts/protocol.md` §8 and `contracts/messages.md` (`TerminalColorScheme`).

**Bugfix**: 2026-09-14 — BUG-007 A single left click selected the clicked cell: it stayed highlighted until the next press, and over text its release auto-copied that one character over the clipboard. FR-013e added (a click without a drag selects nothing and copies nothing; a drag that returns to the pressed cell still selects it; double- and triple-click unchanged), SC-014 added, plus a Clarifications entry.
