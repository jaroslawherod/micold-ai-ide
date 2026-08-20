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

### Session 2026-08-19

- Q: How wide is the AI tab, given it carries no close control? → A: The same fixed width as every
  terminal tab; the icon sits on the tab's midline and the close control's slot stays empty.
- Q: Does a secondary (right) press on the AI tab open a menu? → A: Yes — whatever a terminal tab's
  menu offers, minus Close, including items added to that menu later.
- Q: What happens on a secondary press when that menu would be empty? → A: No menu opens; the press
  does nothing.
- Q: What does "reflects lifecycle" mean, given a terminal tab now shows none? → A: Both kinds of
  tab gain a visible not-running cue, and the AI tab follows the same rule.
- Q: What happens when the tabs outgrow the bar? → A: The terminal tabs scroll horizontally; the AI
  tab is pinned outside the scrolling region.

### Session 2026-08-19 (second pass)

- Q: What form does the not-running cue take? → A: A small state mark beside the label, in an error
  or warning role, drawn in the leading spacer the tab already reserves.
- Q: What holds SC-003 when the marked tab is scrolled out of view? → A: Auto-scroll it into view on
  selection, plus an edge indicator pointing to it whenever it is off-screen.
- Q: How is the strip scrolled, and how is overflow announced? → A: Mouse wheel over the strip, plus
  a persistent edge fade on whichever side holds more tabs.
- Q: Does a starting process wear the not-running mark? → A: No — the mark means actionable, shown
  for not-started and exited only; starting keeps the existing in-progress treatment.
- Q: Is keyboard access to the strip in scope? → A: No — explicitly out of scope; the strip is
  pointer-driven and the existing mode toggle stays the non-tab route to the AI pane.

### Session 2026-08-19 (third pass)

- Q: Should the gallery cover the tab strip? → A: Yes, and in **both** indicator orientations — the
  accent bar on the tab's top edge and on its bottom edge, posed beside each other.
- Q: What shape is a tab's highlight? → A: A Material **tab** state layer — rectangular, filling the
  tab — not the rounded pill a text button draws. An unhighlighted tab still draws nothing.

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
5. **Given** more open instances than the bar can show at once, **When** the user looks at the
   strip, **Then** the AI tab is still at the right-hand end at full size, the "+" and the mode
   toggle are still present at full size, and the edge the hidden tabs lie beyond is faded.
6. **Given** that same strip, **When** the user turns the mouse wheel over it, **Then** the terminal
   tabs scroll at their own width — none is shrunk, ellipsised or dropped — and the AI tab does not
   move.
7. **Given** the user has scrolled the marked tab out of view, **When** they look at the strip,
   **Then** the edge it lies beyond carries the indicator's accent rather than the neutral fade;
   **and when** they then select any tab, **Then** the newly marked tab is scrolled back into view.
8. **Given** any tab, **When** the pointer rests on it, **Then** it draws a rectangular state layer
   filling the tab — not a rounded pill — and **when** the pointer leaves, **Then** it draws no
   shape at all.

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
5. **Given** a session whose AI CLI process has exited, **When** the user presses the AI tab with
   the secondary (right) button, **Then** a menu opens offering restart and **not** offering close.
6. **Given** a session whose AI CLI process is running, **When** the user presses the AI tab with
   the secondary button, **Then** no menu opens and nothing changes.
7. **Given** a session displaying a terminal instance, **When** the user restarts the AI CLI from
   the AI tab's menu, **Then** the pane keeps showing that terminal instance — acting on a tab from
   its menu does not select it.

---

### User Story 3 - The strip reports which processes are not running (Priority: P2)

A developer whose AI CLI or whose backgrounded shell has exited can see that from the strip, without
selecting anything — and therefore knows which tab to open a menu on to restart it.

**Why this priority**: Raised from P3 by the 2026-08-19 clarifications. It was scoped as "the AI tab
should not silently omit what the terminal tabs show", which read as pure consistency polish. Since
feature 012's BUG-005 moved the restart affordance off the tab and into a menu, **no** tab shows
lifecycle, and the strip's only action is behind a press that does nothing on a running tab
(FR-006b). The cue is now what makes that action findable at all, not a finishing touch. Still not
P1: the strip is correct about *what is displayed* without it, which is Story 1's claim.

**Independent Test**: Cause a background terminal instance and the AI CLI process to exit, and
confirm each tab wears the stopped mark, distinguishably from the active indicator, without
either affecting the other.

**Acceptance Scenarios**:

1. **Given** a session whose AI CLI process has exited, **When** the user looks at the AI tab,
   **Then** it indicates the process is not running.
2. **Given** a session with a backgrounded terminal instance whose shell has exited, **When** the
   user looks at that instance's tab, **Then** it indicates the process is not running, by the same
   cue the AI tab uses.
3. **Given** a tab that is both the active one and not running, **When** the user looks at the
   strip, **Then** the active indicator and the stopped mark are both legible and are not
   mistaken for each other.
4. **Given** an AI CLI process that is starting or restarting, **When** the user looks at the AI
   tab, **Then** its state is distinguishable from running, **and** it does not wear the stopped
   mark — the mark says the tab can be acted on, and a starting process cannot.
5. **Given** a stopped instance the user restarts from its tab's menu, **When** the process comes
   up, **Then** the stopped mark clears from that tab and no other tab changes.

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
- **Many instances open.** The terminal tabs scroll (FR-002a) while the AI tab holds its position
  outside the scrolling region (FR-002b), and the bar's other controls are untouched (FR-002c). The
  wall is nearer than it looks: at a 136dp tab on a 144dp pitch, roughly five tabs exhaust a bar
  that also carries a title, a status, the "+" and the mode toggle.
- **Tabs beyond an edge, marked tab still visible.** The edge fades in the neutral surface tint,
  saying there is more that way without claiming the marked tab is out there (FR-002e).
- **The marked tab scrolled out of view.** It keeps the indicator, and the scrolling region's edge
  says which way it lies (FR-002e). Pressing the AI tab, or selecting any instance, scrolls the new
  marked tab back into view (FR-002d).
- **A secondary press on a running AI tab.** Nothing happens, by FR-006b — the menu would be empty.
  The same press on a *stopped* AI tab opens a menu with restart in it, so the affordance appears
  exactly when it can be acted on.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: The tab strip MUST include the session's AI CLI process as a tab, in addition to one
  tab per open Regular Terminal instance.
- **FR-002**: The AI tab MUST be positioned at the **right-hand end** of the strip, after every
  terminal tab, and MUST keep that position as instances are opened and closed.
- **FR-002a**: When the terminal tabs together need more width than the bar can give them, they
  MUST **scroll horizontally** within the strip, at their fixed width (FR-010a, feature 012
  FR-004c). Tabs MUST NOT be shrunk, ellipsised or dropped to make them fit.
- **FR-002b**: The AI tab MUST sit **outside** that scrolling region, so it keeps its right-hand
  position and stays reachable in one press no matter how many terminal instances are open. FR-002
  is only a meaningful requirement under overflow; this is what it means there.
- **FR-002c**: No control in the bar — the "+", the mode toggle, the session title or the status —
  MUST be shrunk or displaced by the strip's growth. Today the bar lays its controls out in one row
  with no bound on the strip, so the controls at its trailing end absorb any shortfall silently —
  drawn narrower, or not at all, with nothing reported. That is the failure mode feature 012's
  BUG-005 was filed for, one level out, and making the strip always visible (FR-003) brings the bar
  to it sooner.
- **FR-002d**: Whenever the marked tab (FR-005) changes, the strip MUST scroll it into view. A user
  MAY then scroll away from it by hand.
- **FR-002e**: Whenever tabs lie beyond either edge of the scrolling region, that edge MUST carry a
  persistent fade saying so, and when the tab beyond it is the **marked** one that fade MUST be
  drawn in the **same accent role the active indicator wears** rather than the neutral surface tint
  it carries otherwise. Two states of one cue, differing only in role: the accent is already the
  application's word for "this is the one you are looking at", so an accent-tinted edge says which
  way that tab lies without introducing a second vocabulary, and it survives both schemes because
  the indicator itself has to. Without this the strip can show
  **nothing marked** — which is the exact defect this feature exists to remove (User Story 1),
  arriving by scrolling instead of by the AI pane, and it would hollow out SC-003's claim rather
  than narrow it. The AI tab is unaffected: FR-002b keeps it outside the scrolling region, so it is
  never the tab that goes missing.
- **FR-002f**: The strip MUST scroll to the mouse wheel while the pointer is over it. It MUST NOT
  add scroll-arrow controls: they would spend an interactive target's width at each end of the bar
  FR-002c exists to keep uncrowded, and the wheel over a scrollable region is how this application
  already scrolls its sidebar and its terminal scrollback. The edge fade of FR-002e is what tells
  the user there is anything to scroll to.
- **FR-003**: The tab strip MUST be visible whenever a session is displayed, including when the
  session has zero or one Regular Terminal instances. This **supersedes** feature 012's FR-005,
  which hid the control below two instances.
- **FR-004**: The AI tab MUST NOT offer a close control. A session has exactly one AI CLI process
  (feature 012 FR-016) and terminating it is not an action offered from this control.
- **FR-005**: Exactly one tab MUST carry the active indicator at all times — the AI tab when the
  session's pane is showing the AI CLI, otherwise the tab of the Regular Terminal instance being
  shown. Never zero tabs, never two. Under overflow the marked tab may be scrolled out of view; it
  still carries the indicator, and FR-002d/FR-002e are what keep that fact reachable.
- **FR-006**: A **primary** press on the AI tab MUST show the session's AI CLI process in the pane,
  and MUST NOT start, stop, restart or otherwise disturb any process — neither the AI CLI nor any
  terminal instance. Selecting is all a primary press does; acting on the process is FR-006a's
  menu, reached by a different press.
- **FR-006a**: A **secondary** (right) press on the AI tab MUST open a context menu carrying the
  same items a terminal tab's menu offers, **except Close** — restart for a process whose lifecycle
  offers it today, and whatever is added to that menu in future without this requirement being
  revisited. Stated as "the terminal tab's menu minus Close" rather than as a list, so the two tabs
  cannot drift into offering different actions for the same reason FR-010 asks them to look alike.
  Close is excluded by FR-004, which holds for every press and not only for the primary one.
- **FR-006b**: When that menu would carry **no items**, no menu MUST open and the secondary press
  MUST do nothing. With restart the only item and Close excluded, this is the state whenever the AI
  CLI is running, which is most of the time. An empty panel is a defect everywhere else in the
  application, and a panel whose entire content is inert is one too — so the offer is absent rather
  than present-and-useless. This also keeps the strip agreeing with the bar beside it, which already
  shows a restart control only for a process that is not running.
- **FR-007**: Pressing the AI tab when the AI CLI is already displayed MUST be a no-op with no
  visible change and no effect on the running process.
- **FR-008**: The existing AI-CLI/Regular mode toggle MUST continue to work (feature 012 FR-006),
  and the indicator MUST follow whatever it does. The toggle and the AI tab MUST NOT be able to
  disagree about what is displayed.
- **FR-009**: The AI tab MUST be labelled with the application's existing AI CLI icon — the same
  glyph the mode toggle shows for that mode — rather than with text.
- **FR-010**: The AI tab MUST be visually consistent with the terminal tabs it sits beside: the same
  tab form, the same indicator treatment, and the same answer to the same gestures — hover, primary
  press, secondary press (feature 012 FR-004a/FR-004b/FR-010b). What the secondary press's menu
  *holds* is FR-006a's, which deliberately differs by one item; the gesture, and the fact that it
  opens a menu at all, do not.
- **FR-010a**: The AI tab MUST measure the **same fixed width** as a terminal tab (feature 012
  FR-004c), and its icon MUST sit on the tab's own midline. Having no close control (FR-004) MUST
  NOT make it narrower than its neighbours: the trailing slot is left empty rather than reclaimed.
  A strip whose tabs are not all one size reads as a control among controls rather than as a strip,
  which is the defect feature 012's BUG-001 was filed for, and a differently-derived width would
  put this tab outside the guarantee FR-004c's derivation exists to give.
- **FR-011**: Every session MUST have its own strip and its own marked tab; actions on one session's
  strip MUST have no observable effect on any other session's.
- **FR-012**: A tab whose process is **stopped** — not started, or exited — MUST be visually
  distinct from one whose process is running, and the distinction MUST be the same for the AI tab
  and for a terminal tab. This applies whether or not the tab is the active one: a background
  instance that has died is the case feature 012's FR-010a exists for.
- **FR-012a**: The stopped mark MUST be carried in addition to, and distinguishably from, the
  active indicator (FR-005) — a tab can be active *and* not running, and the strip must not read
  those two states as one.
- **FR-012b** *(rationale for FR-012, not separately testable — no task implements it)*: This cue is
  what makes FR-006a's menu findable. Without it a user must open menus at random to learn which
  instance is stopped, and by FR-006b every running tab answers that press with silence — so the
  strip would be hiding the one action it offers. The testable form of this intent is FR-012d, which
  ties the mark to exactly the states the menu can act on.
- **FR-012c**: The mark MUST be a **small state mark beside the label**, in the palette's error or
  warning role, drawn in the **leading spacer** every tab already reserves — not a third tone on
  the label. A tone-only cue has to be legible against both the accent an active tab wears and the
  muted tint an inactive one wears, in both schemes, which is the distinction FR-012a asks for and
  the one a third grey is worst at. Placing it in the leading spacer also costs no width: that
  space exists only to balance the trailing close control (feature 012 FR-004a) and is empty today,
  so no tab grows and the derived width (FR-010a) is untouched. The mark MUST sit in the same place
  on the AI tab as on a terminal tab, and MUST NOT displace the label from the tab's midline.
- **FR-012d**: The mark MUST appear for exactly the lifecycle states the tab's menu can act on —
  `NotStarted` and `Exited` — and MUST NOT appear for `Starting`. Its meaning is "there is something
  you can do here", which is what FR-012b makes it for; a mark on a state nobody can act on sends a
  user to a press that does nothing (FR-006b), which is the dead end the mark exists to prevent.
- **FR-012e**: A `Starting` process MUST still be distinguishable from a running one, by the
  application's existing in-progress treatment rather than by FR-012c's mark. Feature 012's BUG-003
  was this state being unobservable; nothing here may return it to that.

- **FR-013**: A tab and the strip that holds them MUST be **shared components** in the client's
  component library, exposed through the chainable builder API the constitution mandates (Principle
  VIII), rather than assembled privately at the call site as they are today. This
  feature is what makes that necessary rather than tidy: one tab shape now serves two different
  kinds of member (FR-001), and it is about to carry a scrolling viewport (FR-002a), a state mark
  (FR-012c) and a state layer (FR-015). A call-site assembly cannot be posed in the gallery either,
  which is what FR-014 requires.
- **FR-014**: The gallery MUST pose the tab strip in **both** indicator orientations — the accent
  bar on the tab's **top** edge and on its **bottom** edge — as separate instances beside each
  other. This application puts the indicator at the top because its strip is anchored to the
  window's bottom and the pane a tab selects is above it (feature 012 FR-004b), which is the
  opposite of Material's default placement. A deliberate inversion that is never shown next to the
  thing it inverts reads as a mistake to the next person, and the two are exactly the kind of
  difference the gallery exists to make visible by comparison rather than by memory.
- **FR-015**: A tab's **highlight** — its hover and press state layer — MUST be a Material *tab*
  state layer: **rectangular, spanning the tab's full width and height**. It MUST NOT be the fully
  rounded pill a text button draws, which is what a tab inherits today by being built as one. An **unhighlighted** tab MUST continue to draw nothing at all — no background, no outline,
  no pill (feature 012 FR-004b). The highlight is the one state in which a tab has a shape, and that
  shape is a tab's.

### Key Entities

- **Tab** *(promoted)*: was an inline assembly at the call site; becomes a shared component — a
  label or icon, a reserved leading slot, a reserved trailing slot, an indicator edge and a
  rectangular state layer. One component, two kinds of member.
- **Tab strip** *(existing, extended, promoted)*: was a list of Regular Terminal instances; becomes
  the session's complete set of displayable panes — its AI CLI process plus its terminal instances —
  with exactly one marked as displayed.
- **AI tab** *(new)*: the strip's representation of the session's single AI CLI process.
  Unclosable, icon-labelled, right-anchored, the same width as a terminal tab, and carrying that
  process's lifecycle state as a leading state mark.
- **Stopped mark** *(new)*: a small state mark in the palette's error or warning role, occupying the
  leading spacer of any tab — AI or terminal — whose process is `NotStarted` or `Exited`. Independent
  of the active indicator, so a tab can carry both, and absent for `Starting`, which wears the
  in-progress treatment instead.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: In every observed session state — AI showing, a terminal showing, zero instances, many
  instances — exactly one tab is marked, and it is the one whose content the pane displays.
- **SC-002**: A user can move from a terminal to the AI conversation in **one press**, from the
  strip, without using the mode toggle.
- **SC-003**: Users can tell which pane is displayed from the strip alone, without pressing
  anything, in 100% of observed states — including the states where feature 012 previously showed no
  strip at all, and including states where the marked tab has been scrolled out of view, where the
  edge fade takes the indicator's own accent to say which way it went (FR-002e).
- **SC-004**: No press on the AI tab ever restarts, interrupts or reorders output in the AI
  conversation or in any terminal instance, in 100% of observed cases.
- **SC-005**: The AI tab is never offered a close affordance in any state, so the single-AI-process
  guarantee cannot be violated through this control.
- **SC-006**: Opening or closing terminal instances leaves the AI tab at the strip's right-hand end
  in every observed case.
- **SC-007**: A user can tell which tabs' processes are not running from the strip alone, without
  selecting or pressing anything, in 100% of observed states — including a background terminal
  instance whose shell has exited, which no control showed before this feature.
- **SC-008**: At any number of open terminal instances, the AI tab, the "+", the mode toggle and
  the session status are all present at their full size, and the AI tab is reachable in one press.
- **SC-009**: Whenever any tab is off-screen, the strip says so at the edge it lies beyond, so a
  user never has to scroll to discover whether there is anything to scroll to.
- **SC-010**: A tab draws a shape in exactly one state — highlighted — and that shape is
  rectangular and fills the tab. In no other state does any tab draw a background, an outline or a
  pill.
- **SC-011**: Both indicator orientations are visible side by side in the gallery, so the
  application's top-edge choice can be compared with Material's bottom-edge default rather than
  taken on trust.

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
- Scroll position within the strip is presentation, not state to persist: FR-002d scrolls the marked
  tab into view on selection, and the position is not remembered across sessions or restarts.
- Renaming an instance so a tab shows a name rather than an ordinal is **out of scope** and expected
  as a later feature. Feature 012's label sizing already accommodates it, so nothing here needs to
  anticipate it beyond not reintroducing a fixed-width label.

## Dependencies

- **Feature 012 (multiple Regular Terminal instances)** — provides the strip, the instance list and
  the active-instance selection this extends. Its FR-005 is superseded by FR-003 here.
- **Feature 012 BUG-002** — provides the indicator tab form (bare label, top-edge accent bar, no
  container) that FR-010 requires the AI tab to match. This feature should land after it.
- **Feature 012 BUG-005** — provides the terminal tab's own context menu, which FR-006a extends to
  this tab, and the derived fixed tab width that FR-010a adopts. Both landed 2026-08-19, after this
  spec was written.
- **The existing session state** already distinguishes "showing the AI CLI" from "showing instance
  N" — a session's mode plus its active instance. This feature is a *view* over that state and adds
  no selection of its own, which is what makes FR-008's no-disagreement guarantee structural rather
  than a synchronisation effort.

## Out of scope

- Renaming instances, and any menu item that would rename one.
- **Keyboard access to the strip** — cycling between tabs, selecting one by ordinal, or moving focus
  into the strip. Decided, not overlooked: this feature is a view over state the application already
  holds and adds no selection of its own, while a keymap is a separate interaction surface with a
  mode-gating problem of its own (feature 012's FR-019 had to gate its chord because the terminal
  pane swallows keystrokes). Nothing becomes unreachable — the mode toggle is the keyboard-
  independent route to the AI pane and FR-008 keeps it working.
- Any change to a tab's **content** beyond what FR-009 and FR-012c add. Promoting the tab into the
  component library (FR-013) is a move, not a redesign: the tab that comes out of it draws what
  feature 012's tab drew, plus this feature's own additions.
- Any change to *how* lifecycle is tracked or reported. FR-012 presents state the application
  already holds (feature 012 FR-008, and the daemon liveness its BUG-003 settled); it introduces no
  new state and no new source for it.
- Any change to the number of AI CLI processes per session, which stays exactly one.
- Any change to **how** the AI CLI process is started, restarted or terminated. FR-006a adds a
  *route* to the restart the application already performs; it introduces no new lifecycle action
  and changes none of the existing ones. *(This line previously read "this feature displays and
  selects; it does not manage lifecycle", and also excluded "any right-click menu on a tab". Both
  were written on 2026-08-16, before feature 012's BUG-005 gave terminal tabs a menu — the
  exclusion had since changed meaning from "like its neighbours" to "unlike them".)*
- Closing the session from the strip, and closing the AI CLI process by any route (FR-004).
