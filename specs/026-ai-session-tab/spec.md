# Feature Specification: The AI Session as a Tab

**Feature Branch**: `feat/026-ai-session-tab`

**Created**: 2026-08-16

**Status**: Draft

**Input**: User description: "The terminal's tab strip should include the session's AI CLI process as a tab, always visible while a session is displayed, always at the right side of the strip. That AI tab has no close control — a session has exactly one AI CLI process and closing it is not a user action. The tab strip's active indicator marks whatever the pane is currently showing: the AI tab when the session is in AI CLI mode, otherwise the tab of the active Regular Terminal instance. Selecting the AI tab switches the pane to the AI CLI, the same as the existing mode toggle, which continues to work. The AI tab is labelled with the existing AI CLI icon (the sparkle glyph the mode toggle already uses) rather than text. This supersedes feature 012's FR-005, which hides the switcher until a session has more than one Regular Terminal instance — with the AI tab always present, the strip is always visible. Builds on feature 012 BUG-002's indicator tabs (bare label plus a top-edge accent bar, no containers)."

## Why this exists

Feature 012 gave a session many Regular Terminal instances and a tab strip to switch between them. The
session's AI CLI process — the thing the session is *for* — is not in that strip. It is reached by a
separate icon-button that toggles modes, so the application has two different mental models for "what
this pane is showing": a row of tabs for the shells, and a mode switch for the AI.

That split has a visible cost. The strip's indicator claims to say what you are looking at, and it
lies whenever the AI pane is showing — no tab is marked, because the thing being displayed is not in
the row. A user glancing at the bar cannot tell the difference between "the AI pane is showing" and
"a terminal is showing but nothing is selected".

This feature makes the strip complete: everything the pane can display is a tab, and the indicator
always marks one of them.

## Clarifications

### Session 2026-08-16

- Q: With the AI process always present as a tab, when is the strip visible? → A: Always, while a
  session is displayed. Feature 012's FR-005 (hide below two instances) is superseded.
- Q: What labels the AI tab? → A: The existing AI CLI icon (the sparkle glyph the mode toggle
  already uses), not text — it stays compact beside short numeric labels and carries an association
  the user already has.
- Q: Where does the AI tab sit? → A: At the **right side** of the strip, after the terminal tabs.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - The strip says what the pane is showing, always (Priority: P1)

A developer glances at the bottom bar and can tell, without pressing anything, whether they are
looking at the AI conversation or at one of their shells — and which shell. The AI is a tab like any
other, and exactly one tab is always marked.

**Why this priority**: This is the feature. Without it the indicator is only sometimes meaningful,
which is worse than an indicator that is always meaningful — a user learns to distrust it.

**Independent Test**: Open a session, look at the strip in AI mode (the AI tab is marked), open a
terminal and switch to it (that tab is marked, the AI tab is not), switch back.

**Acceptance Scenarios**:

1. **Given** a session displaying its AI CLI pane, **When** the user looks at the tab strip, **Then**
   the AI tab carries the active indicator and no terminal tab does.
2. **Given** a session displaying a Regular Terminal instance, **When** the user looks at the tab
   strip, **Then** that instance's tab carries the indicator and the AI tab does not.
3. **Given** a session with no Regular Terminal instances open, **When** the user looks at the
   terminal area, **Then** the strip is visible and shows the AI tab alone, marked.
4. **Given** any session state, **When** the user counts the marked tabs, **Then** exactly one tab
   is marked — never zero, never two.

---

### User Story 2 - Reaching the AI CLI by pressing its tab (Priority: P1)

A developer working in a shell presses the AI tab and the pane shows the AI conversation, exactly as
the mode toggle would have done.

**Why this priority**: A tab that shows state but cannot be pressed is a status light, not a tab.
This is what makes Story 1's display honest.

**Independent Test**: From a displayed terminal instance, press the AI tab; the pane shows the AI
CLI. Press a terminal tab; the pane shows that terminal.

**Acceptance Scenarios**:

1. **Given** a session displaying a Regular Terminal instance, **When** the user presses the AI tab,
   **Then** the pane shows the session's AI CLI process and the indicator moves to the AI tab.
2. **Given** a session displaying the AI CLI, **When** the user presses the AI tab again, **Then**
   nothing changes — no process is restarted and no output is disturbed.
3. **Given** a session displaying the AI CLI, **When** the user presses the existing mode toggle,
   **Then** the pane and the indicator move together, exactly as if a tab had been pressed.
4. **Given** any session, **When** the user looks at the AI tab, **Then** it has **no close
   control** — a session has exactly one AI CLI process and closing it is not an available action.

---

### User Story 3 - The AI tab reports the AI process's state (Priority: P3)

A developer whose AI CLI has exited or failed can see that from the strip, the same way a terminal
tab shows its instance is not running.

**Why this priority**: Consistency once the AI is a tab — a row where one tab silently omits the
state every other tab shows is a new inconsistency in place of the one this feature removes. Not
required for the strip to be correct, which is why it is P3.

**Independent Test**: Cause the AI CLI process to exit, and confirm the AI tab reflects a
not-running state without affecting any terminal tab.

**Acceptance Scenarios**:

1. **Given** a session whose AI CLI process has exited, **When** the user looks at the AI tab,
   **Then** it indicates the process is not running.
2. **Given** an AI CLI process that is starting or restarting, **When** the user looks at the AI
   tab, **Then** its state is distinguishable from running.

---

### Edge Cases

- **A session with no Regular Terminal instances.** The strip shows one tab — the AI — and it is
  marked. This is the state feature 012 deliberately rendered nothing in, so it is the case most
  likely to look wrong; it must read as a deliberate strip rather than a stray control.
- **Closing the last Regular Terminal instance.** Feature 012 FR-013 reverts the session to AI CLI
  mode. The indicator must follow to the AI tab in the same step, with no frame in which nothing is
  marked.
- **Opening the first Regular Terminal instance.** A tab joins the strip to the left of the AI tab;
  the AI tab keeps its position at the right edge.
- **A session that is not displayed.** Background sessions have their own tabs and their own
  selection; nothing about one session's strip may reflect another's.
- **Many instances open.** The strip must remain readable as it grows, and the AI tab must remain
  reachable at its right-hand position rather than being pushed out of view.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: The tab strip MUST include the session's AI CLI process as a tab, in addition to one
  tab per open Regular Terminal instance.
- **FR-002**: The AI tab MUST be positioned at the **right-hand end** of the strip, after every
  terminal tab, and MUST keep that position as instances are opened and closed.
- **FR-003**: The tab strip MUST be visible whenever a session is displayed, including when the
  session has zero or one Regular Terminal instances. This **supersedes** feature 012's FR-005,
  which hid the control below two instances.
- **FR-004**: The AI tab MUST NOT offer a close control. A session has exactly one AI CLI process
  (feature 012 FR-016) and terminating it is not an action offered from this control.
- **FR-005**: Exactly one tab MUST carry the active indicator at all times — the AI tab when the
  session's pane is showing the AI CLI, otherwise the tab of the Regular Terminal instance being
  shown. Never zero tabs, never two.
- **FR-006**: Pressing the AI tab MUST show the session's AI CLI process in the pane, and MUST NOT
  start, stop, restart or otherwise disturb any process — neither the AI CLI nor any terminal
  instance.
- **FR-007**: Pressing the AI tab when the AI CLI is already displayed MUST be a no-op with no
  visible change and no effect on the running process.
- **FR-008**: The existing AI-CLI/Regular mode toggle MUST continue to work (feature 012 FR-006),
  and the indicator MUST follow whatever it does. The toggle and the AI tab MUST NOT be able to
  disagree about what is displayed.
- **FR-009**: The AI tab MUST be labelled with the application's existing AI CLI icon — the same
  glyph the mode toggle shows for that mode — rather than with text.
- **FR-010**: The AI tab MUST be visually consistent with the terminal tabs it sits beside: the same
  tab form, the same indicator treatment, and the same behaviour on hover and press (feature 012
  FR-004a/FR-004b).
- **FR-011**: Every session MUST have its own strip and its own marked tab; actions on one session's
  strip MUST have no observable effect on any other session's.
- **FR-012**: The AI tab MUST reflect the AI CLI process's lifecycle state, consistent with how a
  terminal tab reflects its instance's (feature 012 FR-008).

### Key Entities

- **Tab strip** *(existing, extended)*: was a list of Regular Terminal instances; becomes the
  session's complete set of displayable panes — its AI CLI process plus its terminal instances —
  with exactly one marked as displayed.
- **AI tab** *(new)*: the strip's representation of the session's single AI CLI process.
  Unclosable, icon-labelled, right-anchored, and carrying that process's lifecycle state.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: In every observed session state — AI showing, a terminal showing, zero instances, many
  instances — exactly one tab is marked, and it is the one whose content the pane displays.
- **SC-002**: A user can move from a terminal to the AI conversation in **one press**, from the
  strip, without using the mode toggle.
- **SC-003**: Users can tell which pane is displayed from the strip alone, without pressing
  anything, in 100% of observed states — including the states where feature 012 previously showed no
  strip at all.
- **SC-004**: No press on the AI tab ever restarts, interrupts or reorders output in the AI
  conversation or in any terminal instance, in 100% of observed cases.
- **SC-005**: The AI tab is never offered a close affordance in any state, so the single-AI-process
  guarantee cannot be violated through this control.
- **SC-006**: Opening or closing terminal instances leaves the AI tab at the strip's right-hand end
  in every observed case.

## Assumptions

- **"At the right side" means the right end of the tab strip**, after the terminal tabs — not that
  the strip as a whole moves, since it already sits toward the bar's trailing edge. If the intent
  was instead about the strip's own alignment within the bar, FR-002 is the only requirement that
  changes.
- The mode toggle is **kept**, not replaced. It becomes a second route to something the strip now
  also offers, which is safe because both write the same underlying state rather than each holding
  their own (see Dependencies).
- The strip being always visible is acceptable in a session with no terminal instances, where it
  shows a single marked AI tab. This is a deliberate reversal of feature 012's FR-005 and its
  "pixel-identical to the single-instance experience" intent.
- Sessions that are not displayed are unaffected; this is a property of the displayed session's bar.
- Renaming an instance so a tab shows a name rather than an ordinal is **out of scope** and expected
  as a later feature. Feature 012's label sizing already accommodates it, so nothing here needs to
  anticipate it beyond not reintroducing a fixed-width label.

## Dependencies

- **Feature 012 (multiple Regular Terminal instances)** — provides the strip, the instance list and
  the active-instance selection this extends. Its FR-005 is superseded by FR-003 here.
- **Feature 012 BUG-002** — provides the indicator tab form (bare label, top-edge accent bar, no
  container) that FR-010 requires the AI tab to match. This feature should land after it.
- **The existing session state** already distinguishes "showing the AI CLI" from "showing instance
  N" — a session's mode plus its active instance. This feature is a *view* over that state and adds
  no selection of its own, which is what makes FR-008's no-disagreement guarantee structural rather
  than a synchronisation effort.

## Out of scope

- Renaming instances, and any right-click menu on a tab.
- Any change to the number of AI CLI processes per session, which stays exactly one.
- Any change to how the AI CLI process is started, restarted or terminated. This feature displays
  and selects; it does not manage lifecycle.
- Closing the session from the strip.
