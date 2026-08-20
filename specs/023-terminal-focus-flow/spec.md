# Feature Specification: Natural Terminal Focus Flow

**Feature Branch**: `feat/improved-focuse-management` (spec `023-terminal-focus-flow`)

**Created**: 2026-08-09

**Status**: Closed

**Input**: User description: "change focuse management for terminal to be more natural and not require double click in some cases . E.g. if terminal is focused and I want to switch to regular I need to click once to release the focuse from terminal and click second time to switch. it should be done in one click. The other case is when I switch between windows or sessions often focuse is not set to terminal and requires a explicit click to gain focuse. That should happen automatically"

## Clarifications

### Session 2026-08-09

- Q: Should a momentary control that takes no keyboard of its own (an icon button such as "toggle
  sidebar", or the pane's restart affordance) take the keyboard away from the terminal at all? →
  A: No — model the whole feature on GNOME Terminal. The displayed terminal is the window's default
  keyboard holder: controls that do not type leave the keyboard where it is, controls that do type
  take it and hand it back when they are dismissed, and pressing inert space changes nothing.
- Q: Is an explicit release remembered per session, or is it one global state — and does navigating
  to a terminal clear it? → A: One global state, cleared by any navigation that displays a terminal
  (FR-011). Deliberately navigating to a terminal is itself a request for it; a release is about the
  present moment, not a property a session carries.
- Q: At application launch, with a previously displayed session restored and shown, does its terminal
  hold the keyboard? → A: Yes — the restored session's terminal holds the keyboard at launch, so the
  user can type straight away. Not persisted from the last run; launch simply applies the
  default-holder rule.
- Q: May the keyboard visibly leave the terminal and come straight back — a release-and-reacquire
  within one press or navigation? → A: No. The holder must never pass through an intermediate state
  the user did not ask for: no focus-ring blink, and no keystroke misrouted mid-transition.
- Q: When the user presses a terminal pane that does not hold the keyboard, does that press also act
  on the terminal, or is it consumed to grant focus? → A: It acts in full — identical to a press on
  an already-focused pane, including being reported to a mouse-aware process. No press is consumed
  solely to grant focus.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - One press does what you pressed (Priority: P1)

A developer is typing in a session's terminal. They want to switch that terminal to the regular
shell, open a different session, or press any other control in the application. They press it once
and it works — exactly as pressing a toolbar button in a terminal emulator works. They never have to
press the same thing twice: once to "get out of" the terminal, once to actually use it.

**Why this priority**: This is the defect the user hits most often and the one that makes the app
feel unresponsive. Every control outside the terminal pane is affected, so fixing it lifts the whole
application; nothing else in this feature matters if a press still has to be repeated.

**Independent Test**: With the terminal focused, press each class of control once — the regular/AI
mode toggle in the terminal's status bar, a Regular Terminal instance tab, a session in the sidebar,
a toolbar action, a menu item, a text field — and confirm each one performs its action on that
single press.

**Acceptance Scenarios**:

1. **Given** a focused terminal in AI CLI mode, **When** the user presses the mode toggle in the
   terminal's status bar once, **Then** the terminal switches to Regular Terminal mode on that press.
2. **Given** a focused terminal, **When** the user presses a different session in the sidebar once,
   **Then** that session becomes the displayed session on that press.
3. **Given** a focused terminal and several open Regular Terminal instances, **When** the user
   presses another instance's tab once, **Then** that instance becomes the attached instance on that
   press.
4. **Given** a focused terminal, **When** the user presses a text entry field (for example the
   sidebar filter or a form field) once, **Then** the field receives the keyboard on that press and
   the next typed characters go into the field, not the terminal.
5. **Given** a focused terminal, **When** the user presses a control that types nothing of its own
   (for example the sidebar toggle), **Then** the action happens and the user can carry on typing
   into the terminal without pressing it again.
6. **Given** a focused terminal, **When** the user presses a control outside the pane once, **Then**
   no keystroke or byte produced by that interaction reaches the terminal's attached process.
7. **Given** a terminal that does not hold the keyboard and a mouse-aware program running in it,
   **When** the user presses inside the pane once, **Then** the terminal takes the keyboard and that
   same press reaches the program at the cell pressed.

---

### User Story 2 - Coming back to the app resumes typing (Priority: P1)

A developer leaves the application for another window — a browser, an editor, a chat client — and
comes back. Whatever they were working in before they left is still what the keyboard drives. If
that was a session terminal, they can simply keep typing; they do not have to hunt for the pane and
click it first.

**Why this priority**: Window switching happens constantly during a working day, and losing the
keyboard on every return is a per-minute tax. It is the second half of the user's report and is
independently valuable even if Story 1 were not addressed.

**Independent Test**: Focus a terminal, switch to another application window, switch back, and type
— the characters must reach the terminal's process without any intervening press. Repeat with the
terminal deliberately unfocused before leaving, and confirm the keyboard is *not* handed to the
terminal on return.

**Acceptance Scenarios**:

1. **Given** a focused terminal, **When** the user switches to another application window and back,
   **Then** typing goes to the terminal's process with no press in between.
2. **Given** a terminal the user deliberately released (via the release shortcut ~~or the release
   affordance in the pane~~ — the shortcut is the only release now, FR-021b), **When** the user
   switches away and back, **Then** the terminal does not take the keyboard, and application
   shortcuts still work.
3. **Given** an open dialog with a text field the user was typing into, **When** the user switches
   away and back, **Then** that field still holds the keyboard and the terminal does not take it.
4. **Given** the application window has no input focus, **When** output arrives in a background
   session, **Then** the focus state is unchanged when the user returns.

---

### User Story 3 - Landing on a session leaves you ready to type (Priority: P2)

A developer selects a session, starts a new one, opens or closes a Regular Terminal instance,
switches terminal mode, or switches to a project whose session is restored. In every case the pane
they land on is ready for input immediately — they type and it goes to the process shown in front
of them.

**Why this priority**: Selecting and starting a session already behaves this way; the gap is the
other navigations that display a terminal (mode switch, instance open/close/switch, project switch)
where the user is left looking at a terminal that ignores the keyboard. Valuable, but a smaller
share of the daily friction than Stories 1–2.

**Independent Test**: From an unfocused state, perform each navigation that changes which terminal
is displayed — select session, start session, toggle mode, open/close/switch a Regular Terminal
instance, switch project — and after each one type a character and confirm it reaches the newly
displayed process.

**Acceptance Scenarios**:

1. **Given** any focus state, **When** the user switches the displayed terminal between AI CLI and
   Regular Terminal mode, **Then** the newly attached terminal holds the keyboard.
2. **Given** any focus state, **When** the user opens a new Regular Terminal instance, **Then** the
   new instance holds the keyboard.
3. **Given** several Regular Terminal instances, **When** the user closes the attached one, **Then**
   the instance that takes its place holds the keyboard.
4. **Given** a project with a previously displayed session, **When** the user switches to that
   project, **Then** the restored session's terminal holds the keyboard.
5. **Given** a terminal that holds the keyboard, **When** the displayed session goes away (closed,
   removed, or its worktree deleted) and no session takes its place, **Then** no terminal holds the
   keyboard and application shortcuts work again.
6. **Given** a session that was displayed when the application last closed, **When** the application
   is launched and restores it, **Then** its terminal holds the keyboard and the user can type
   without pressing anything.

---

### User Story 4 - Focus is never taken while you are typing somewhere else (Priority: P2)

A developer typing into a dialog, a rename field, a filter box, or a branch search is never
interrupted by the terminal grabbing the keyboard mid-word — not by arriving output, not by a
background session changing state, not by a dialog closing behind them.

**Why this priority**: The automatic focusing in Stories 2–3 is only safe if it is bounded. Without
this, the feature trades one class of lost keystrokes for a worse one — keystrokes silently
delivered to a shell process.

**Independent Test**: Open each text-entry surface, type into it, and while doing so trigger the
events that would otherwise focus a terminal (session output, a session reaching Running, another
session's state change) — the typed characters must all land in the field.

**Acceptance Scenarios**:

1. **Given** an open dialog or overlay, **When** any event that would normally focus a terminal
   occurs, **Then** the dialog keeps the keyboard and no character reaches a terminal process.
2. **Given** the user is typing into a filter or search field, **When** a background session
   produces output or changes state, **Then** the field keeps the keyboard.
3. **Given** an open dialog was dismissed, **When** it closes, **Then** the keyboard returns to
   whatever held it before the dialog opened.

---

### Edge Cases

- A press that lands on empty, non-interactive space outside the terminal: nothing happens and the
  terminal keeps the keyboard — inert space is not a way out of the terminal.
- A press on the terminal's own scrollbar, its status bar text, or a right-click for the context
  menu: the terminal keeps the keyboard; these are part of the pane, not a way out of it.
- A press on a control that is disabled: the keyboard is unchanged and nothing is activated, so the
  user is not silently ejected from the terminal by a press that did nothing.
- A menu opened from a control while the terminal was focused: the open menu holds the keyboard so
  it can be driven with arrows and Escape; when it closes — by choosing an item or by dismissing it
  — the keyboard goes back to the terminal.
- A user who explicitly released the terminal (the reserved shortcut ~~or the release affordance~~,
  FR-021b) then
  opens and closes a dialog: the keyboard returns to the application, not to the terminal. An
  explicit release is a decision, and no automatic rule may quietly reverse it.
- Returning to the application by pressing directly on the terminal pane: one press, and the
  terminal has the keyboard — the press must not both restore prior focus and be consumed as a
  focus-granting press with no further effect.
- A displayed session whose process is not running (failed, exited, restarting): the terminal may
  still hold the keyboard, and input is discarded exactly as it is today — automatic focusing must
  not cause keystrokes to be delivered to, or buffered for, a process that is not running.
- The user leaves the application while a terminal is focused and, while away, that session is
  closed or removed from another window: on return, no terminal holds a keyboard it no longer has,
  and the application handles keys normally.
- Rapid alternation (press a control, press back into the terminal, press away again) must settle on
  exactly the state matching the last press, with no lingering or restored focus from an earlier one.

## Requirements *(mandatory)*

### Functional Requirements

#### One press, one outcome

- **FR-001**: A single press on any interactive control outside the terminal pane MUST activate that
  control on that press.
- **FR-002**: The application MUST NOT require a second press on the same control to activate it
  merely because the terminal held the keyboard when the first press was made.
- **FR-003**: A press outside the terminal pane MUST NOT deliver any input to the terminal's attached
  process, whether or not it changes which surface holds the keyboard. (A press *inside* the pane is
  ordinary terminal interaction — see FR-008b.)
- **FR-004**: A single press on a control that accepts typed input — a text field, or a menu or
  dialog that opens on the press — MUST leave that control holding the keyboard, so the characters
  typed next go to it.
- **FR-005**: A press on a control that accepts no typed input of its own — an icon button, a
  toggle, a menu item that performs an action and closes — MUST leave the keyboard where it already
  was, so a user typing in the terminal can carry on typing after pressing it.
- **FR-006**: A press on non-interactive space MUST leave the keyboard where it already was and MUST
  NOT activate anything.
- **FR-007**: A press within the terminal pane's own furniture (its scrollbar, status bar, and
  context menu) MUST leave the terminal holding the keyboard — giving it the keyboard, per FR-008b,
  if it did not already hold it.
- **FR-008**: A press on a disabled control MUST leave the keyboard where it already was.
- **FR-008a**: A press or navigation MUST move the keyboard directly from its old holder to its new
  one. The holder MUST NOT pass through any intermediate state the user did not ask for — in
  particular, a control covered by FR-005 or FR-006 MUST NOT release the terminal and re-acquire it,
  even momentarily. No focus indication may blink, and no keystroke may be routed to a holder that
  exists only mid-transition.
- **FR-008b**: A press on a terminal pane that does not hold the keyboard MUST give it the keyboard
  *and* act exactly as the same press would on a pane that already held it — positioning the cursor,
  starting a selection, hitting the scrollbar, or being reported to a mouse-aware process as the
  case may be. No press may be consumed solely to grant focus.

#### The terminal as the default keyboard holder

- **FR-009**: While a session terminal is displayed, it MUST be the surface that holds the keyboard
  by default — that is, whenever no other surface has taken the keyboard and the user has not
  explicitly released it.
- **FR-010**: When a surface that took the keyboard finishes — a dialog or menu closes, a text field
  is committed or dismissed — the keyboard MUST return to the displayed terminal, unless the user
  had explicitly released the terminal before that surface opened, in which case it returns to the
  application.
- **FR-011**: Every navigation that changes which terminal is displayed — selecting a session,
  starting a session, switching between AI CLI and Regular Terminal mode, opening, closing or
  switching a Regular Terminal instance, and switching to a project with a restored session — MUST
  leave the newly displayed terminal holding the keyboard.
- **FR-012**: When the displayed session goes away and nothing takes its place, no terminal MUST
  hold the keyboard.
- **FR-012a**: At application launch, if a restored session's terminal is displayed, it MUST hold the
  keyboard, so the first keystroke of the session needs no press. No focus state is carried over from
  the previous run — launch applies the default-holder rule to whatever is displayed.

#### Automatic focus on return to the window

- **FR-013**: When the application window regains input focus, the keyboard MUST be restored to
  whatever held it when the window lost input focus, with no press required from the user.
- **FR-014**: If the terminal held the keyboard when the window lost input focus and the displayed
  session still exists on return, the terminal MUST hold the keyboard again on return.
- **FR-015**: If the terminal did not hold the keyboard when the window lost input focus, returning
  MUST NOT give it the keyboard.
- **FR-016**: If the session that held the keyboard no longer exists on return, no terminal MUST
  hold the keyboard and application key handling MUST resume.

#### Bounds on automatic focus

- **FR-017**: The terminal MUST NOT take the keyboard while a dialog or overlay is open.
- **FR-018**: The terminal MUST NOT take the keyboard from a text-entry control that currently holds
  it, for any reason other than a user press or the reserved focus shortcut.
- **FR-019**: Terminal output, a session lifecycle change, or any background session activity MUST
  NOT change which surface holds the keyboard.
- **FR-020**: At most one terminal MUST hold the keyboard at any moment, and only the displayed
  session's terminal MUST be eligible to hold it.

#### Preserved behaviour

- **FR-021**: The existing explicit ways to give and take the terminal's keyboard — pressing the
  pane, the reserved release shortcut, ~~and the release affordance in the pane~~ — MUST continue to
  work unchanged. *(The affordance clause is superseded — bugfix `012-multiple-regular-terminals`
  BUG-001: this feature listed the bottom bar's release-focus button under "Preserved behaviour"
  without re-examining whether it still earned its place after the focus model changed underneath
  it. See FR-021b.)* An explicit release MUST hold until the user gives the keyboard back or
  navigates (FR-021a), and MUST NOT be undone by FR-010's return-to-terminal rule.
- **FR-021b** (bugfix BUG-001): The bottom bar MUST NOT carry a release-focus affordance. This
  feature made every navigation acquire the keyboard automatically (FR-011, FR-021a) and removed the
  click-outside release (FR-005, FR-006), which left that button permanently visible while doing
  nothing a user needs it for: the reserved shortcut covers the keyboard-only user — the "never
  trapped" guarantee `006-real-terminal-emulator` FR-011 actually protects — and navigating anywhere
  covers everyone else. Its removal MUST be **unconditional**: the bar's child list must still not
  vary with focus (FR-008a), so the control must not merely become conditional on
  `terminal_focused()`. `Message::TerminalFocusReleased` itself MUST remain — the reserved shortcut
  dispatches it.
- **FR-021a**: An explicit release MUST be a single application-wide state, not a property remembered
  per session. Any navigation covered by FR-011 MUST clear it, so the terminal the user navigated to
  holds the keyboard regardless of a release that preceded the navigation.
- **FR-022**: While the terminal holds the keyboard, keys MUST continue to be routed to its attached
  process rather than to application shortcuts, exactly as today.
- **FR-023**: While the terminal does not hold the keyboard, no key MUST reach any attached process.
- **FR-024**: Whether the terminal holds the keyboard MUST remain visually apparent at a glance, and
  the state shown MUST always match the state in effect — including immediately after an automatic
  focus change the user did not initiate by pressing.
- **FR-025**: Giving or taking the terminal's keyboard MUST NOT disturb the attached process — no
  restart, no interruption, no lost output, no resize.
- **FR-026**: All of the above MUST behave identically on Linux, macOS, and Windows.

### Key Entities

- **Keyboard holder**: The single surface that receives typed keys at any moment — the displayed
  session's terminal, a text-entry control, an open menu or dialog, or the application itself.
  Exactly one holder at a time.
- **Displayed terminal**: The terminal currently shown in the pane — the displayed session's AI CLI
  process or one of its Regular Terminal instances. The only terminal eligible to hold the keyboard,
  and the default holder whenever nothing else has claimed it.
- **Transient holder**: A surface that takes the keyboard only for as long as it is open or being
  typed into (a menu, a dialog, a search or form field). When it finishes, the keyboard falls back to
  the default holder.
- **Explicit release**: The user's decision to hand the keyboard from the terminal to the
  application. One application-wide state, not a per-session property. It outranks the default-holder
  rule and persists until the user gives the keyboard back or navigates to a terminal.
- **Suspended holder**: The holder recorded when the application window loses input focus, restored
  when it regains input focus.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: Every control outside the terminal pane activates on the first press while the
  terminal holds the keyboard — zero controls require a second press.
- **SC-002**: Switching the displayed terminal between AI CLI and Regular Terminal mode takes exactly
  one press from a focused terminal, down from two.
- **SC-003**: After leaving the application and returning, a user who was typing in the terminal can
  resume typing with zero presses, in 100% of returns where the session still exists.
- **SC-004**: Across the moments that put a terminal in front of the user (application launch with a
  restored session, session select, session start, mode switch, instance open/close/switch, project
  switch), 100% leave the user able to type immediately with zero presses.
- **SC-005**: Zero keystrokes typed into a dialog, filter, or form field are delivered to a terminal
  process during a session in which terminal output, lifecycle changes, and window switches all
  occur.
- **SC-006**: After a menu, dialog, or search field is dismissed, the user can type into the terminal
  with zero presses in 100% of cases where they had not explicitly released it.
- **SC-007**: The visible focus indication matches the actual keyboard holder in 100% of observed
  transitions, including automatic ones, and shows zero intermediate states — no transition produces
  a focus indication the user did not ask for, however briefly.
- **SC-008**: No press outside the terminal pane produces input at the attached process, in any of
  the transitions above.
- **SC-009**: Pressing into a terminal that does not hold the keyboard reaches its content on that
  press — including a mouse-aware program — in 100% of attempts, never needing a second press.

## Assumptions

- The reference behaviour is GNOME Terminal, named by the user: within a focused window the terminal
  widget is what the keyboard drives unless something that types has taken it, buttons and menu items
  act without stealing it, and dismissing a menu or find bar puts it straight back. This spec
  transposes that model onto an application whose window also contains a sidebar, dialogs, and forms.
- "Regular" in the user's report refers to Regular Terminal mode and its instances (features 010 and
  012); the two-press problem is not specific to that control — it applies to every interactive
  control outside the terminal pane, and the requirements are written at that scope.
- "Windows" in the user's report means other applications' windows (the application itself has a
  single window); switching between them is an operating-system window focus change.
- Restoring the previous keyboard holder — rather than always handing the keyboard to the terminal —
  is the correct behaviour on window return, because a user who deliberately released the terminal
  before leaving did so on purpose, and because a half-typed dialog field must survive an
  alt-tab.
- Selecting or starting a session already focuses that session's terminal (bugfix BUG-001 of feature
  006); this feature extends the same rule to the remaining navigations rather than introducing it.
- The existing routing rule stays as it is: the terminal holding the keyboard is what decides whether
  a key reaches the process or the application. This feature changes *when* the terminal holds the
  keyboard, not what holding it means.
- Input to a session that is not running continues to be discarded rather than buffered; automatic
  focusing does not change the write gate.
- Focus behaviour is not user-configurable; there is no setting to turn automatic focusing off.
- Keyboard-only focus traversal (moving focus between controls with Tab) is out of scope for this
  feature except where it already exists; this feature is about pointer presses and automatic focus.

**Bugfix**: 2026-08-14 — `012-multiple-regular-terminals` BUG-001 The bottom bar's release-focus
button is retired. This feature listed it under "Preserved behaviour" (FR-021) without re-examining
whether it still earned its place after the focus model changed underneath it: navigation now
acquires the keyboard automatically (FR-011, FR-021a) and the click-outside release is gone (FR-005,
FR-006), leaving a permanently-visible control — disabled in every state where the terminal does not
hold the keyboard — whose job the reserved `Ctrl+Shift+E` / `Cmd+Shift+E` chord already does.
FR-021's affordance clause struck through and FR-021b added (the bar must not carry the affordance;
removal is unconditional so FR-008a's "child list does not vary with focus" invariant and its gate
`tests/terminal_bar_stability.rs::bar_does_not_branch_on_focus` still hold;
`Message::TerminalFocusReleased` stays, since the chord dispatches it). US1 Scenario 2 and the
edge-case list annotated to match. T013 in `tasks.md` is superseded, not reopened — it was correct
when written. `006-real-terminal-emulator` `contracts/focus-model.md` is amended in the same pass;
006 FR-011 itself is untouched, as it only ever mandated two ways out and the chord satisfies the
"never trapped" guarantee it protects. The fix tasks live in
`specs/012-multiple-regular-terminals/tasks.md` Phase 9, alongside BUG-001's tab-strip work in the
same bar.
