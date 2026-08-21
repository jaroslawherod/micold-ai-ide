# Feature Specification: Tabs Are the Only Switcher

**Feature Branch**: `feat/027-tabs-only-switching`

**Created**: 2026-08-21

**Status**: Draft

**Input**: User description: "the ai tab session don't look as I was thinking about it. I wanted to
remove this toggle button and have just a tabs. starting from right AI tab, Terminal tabs, Plus
button to add more terminals. All should be on the right side" — followed by "the user should be
able to freely switch between terminals and AI without toggle".

## Why this exists

Feature 026 put the session's AI CLI process in the tab strip and left the mode toggle beside it,
"which continues to work" (026 FR-008). Two controls, one job. The bar now says the same thing
twice, and the two say it differently: a tab names where it goes, a toggle only says "the other
one".

That was the plan, and the plan was wrong. Seeing it, the user asked for the toggle to go. The
arrangement they asked for is the one a tabbed terminal has: a strip against the bar's trailing
edge, the AI tab at its end, a "+" beside it, and nothing else claiming to switch panes.

**It is also a correctness change, not only a visual one.** Two live defects shipped with 026 and
are invisible for exactly as long as the toggle exists, because the toggle offers a second route
past each of them:

1. `Message::TerminalAiCliSelected` has no handler in the client's message routing, so pressing the
   AI tab moves the strip's mark and the session's mode without telling the daemon which process the
   pane is now attached to. The daemon goes on streaming, and delivering keystrokes to, whichever
   process was attached before. A user who presses the AI tab types into a shell that is no longer
   on screen.
2. Selecting a terminal tab writes the session's active instance but not its mode, and both the
   mark and the attached process are derived from the mode — so from the AI pane, a press on a
   terminal tab is a three-layer no-op indistinguishable from a press that never registered.

Deleting the toggle makes the tabs load-bearing. Both defects have to be fixed in the same change,
which is why this is a feature rather than a deletion.

## Clarifications

### Session 2026-08-21

- Q: Where does the "+" sit in the right-aligned group? → A: Between the tabs and the AI tab, so
  that the "+" and the AI tab both stay anchored at the bar's trailing edge and the tabs grow
  leftward away from them.
- Q: Does anything replace the toggle as a non-tab route to the AI pane? → A: No. Every pane is
  reachable by its own tab; a second vocabulary for the same navigation is what this removes.
- Q: How much process should this carry? → A: A direct change plus amendments to the specs it
  contradicts (010, 011, 012, 026), rather than a full feature cycle. The requirement is one
  arrangement and two defects.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Switching panes with nothing but tabs (Priority: P1)

A developer moves between their shells and the assistant by pressing tabs, in either direction, any
number of times, and the pane always shows what the marked tab says it shows.

**Why this priority**: This is the feature. Removing the toggle without this is removing the only
route that worked.

**Independent Test**: From the AI pane, press a terminal tab — that terminal appears and its tab is
marked. Press the AI tab — the conversation appears and its tab is marked. Repeat; nothing degrades.

**Acceptance Scenarios**:

1. **Given** a session displaying its AI CLI pane with at least one terminal instance open,
   **When** the user presses that instance's tab, **Then** the pane shows that terminal, its tab
   carries the indicator, and the user's keystrokes reach that shell.
2. **Given** a session displaying a terminal instance, **When** the user presses the AI tab,
   **Then** the pane shows the AI CLI conversation, the AI tab carries the indicator, and the
   user's keystrokes reach the AI process.
3. **Given** either of the switches above, **When** it completes, **Then** the daemon is attached to
   the process now on screen — no output continues to arrive from the process that left, and no
   keystroke is delivered to it.
4. **Given** a session displaying its AI CLI pane, **When** the user looks at the bottom bar,
   **Then** there is no mode-toggle control anywhere in it.
5. **Given** a session displaying its AI CLI pane and no terminal instances yet, **When** the user
   presses the "+", **Then** a terminal instance opens, the pane shows it, and its tab is marked.

---

### User Story 2 - The strip sits where a tab strip sits (Priority: P1)

A developer's eye finds the "+" and the AI tab in the same place every time, because they are
against the bar's trailing edge and do not move with the number of open instances.

**Why this priority**: It is what the user asked for in the words they asked for it, and it is what
makes Story 1 pressable without looking.

**Independent Test**: Open and close instances and watch the right-hand end of the bar. The "+" and
the AI tab do not move; the tabs grow and shrink leftward.

**Acceptance Scenarios**:

1. **Given** any session, **When** the user looks at the bottom bar, **Then** its trailing group
   reads, left to right: the terminal tabs, the "+", the AI tab — with the AI tab last.
2. **Given** a session with room to spare in the bar, **When** the user looks at the strip,
   **Then** the last terminal tab finishes against the "+" rather than the tabs starting at the
   left of the region with empty space after them.
3. **Given** a session, **When** the user opens or closes an instance, **Then** the "+" and the AI
   tab do not move.
4. **Given** more instances than the bar can hold, **When** the user looks at the bar, **Then** the
   "+" and the AI tab are still at full size in their places and the tabs scroll (026 FR-002a).

---

### Edge Cases

- **A session with no terminal instances at all.** The "+" is present and the AI tab is marked.
  This is the case the old code got wrong: the "+" was drawn only in Regular mode, which was
  survivable while a toggle could leave the AI pane, and would otherwise strand a session with no
  way to open its first terminal.
- **Pressing the tab that is already marked.** Nothing happens, in either direction (026 FR-007).
- **Closing the last terminal instance.** Feature 012 FR-013 reverts the session to its AI pane and
  the AI tab takes the mark, with no frame in which nothing is marked.
- **A session whose AI CLI process has exited.** Its tab still switches to it, and still carries the
  stopped mark and the restart menu (026 FR-006a). Switching to a dead process is not the same
  action as restarting it.
- **The keyboard chord that opens an instance.** Ctrl+Shift+T works from the AI pane as well as from
  a terminal, for the same reason the "+" does.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: The AI-CLI/Regular mode toggle MUST be removed from the terminal's bottom bar, and no
  control MUST replace it. This **supersedes** feature 010's FR-002 and FR-009 and feature 026's
  FR-008: the tab strip is the only switcher between a session's panes.
- **FR-002**: The bar's trailing group MUST be laid out, in order, as the terminal tabs, then the
  "+", then the AI tab — with the AI tab the bar's **last** child, against its trailing edge. The
  "+" sits between the two so that both it and the AI tab keep a fixed position as the tab count
  changes.
- **FR-003**: The terminal tabs MUST sit against the **trailing** edge of their scrolling region, so
  the last tab finishes beside the "+". This **supersedes** the second half of feature 012's
  FR-002c, which required the opposite for the reason it names: a trailing-aligned strip moves its
  own first tab whenever an instance is opened. That cost is accepted here in exchange for the two
  controls a user reaches for by habit staying still, which is what FR-001 makes load-bearing.
- **FR-004**: The "+" MUST be present in both of a session's panes. It was drawn only in Regular
  mode, which relied on the toggle FR-001 removes and would otherwise leave a session in its AI
  pane with no way to open a terminal. The same applies to the keyboard chord that opens one.
- **FR-005**: Pressing a terminal tab MUST display that instance — it MUST set both the session's
  active instance and the pane it is showing. Setting only the active instance leaves the mark and
  the attached process, which are both derived from the pane, unchanged: the press does nothing a
  user can see.
- **FR-006**: Pressing a tab MUST attach the daemon to the process that tab names, in both
  directions. A switch that changes only the client's view leaves the daemon streaming the previous
  process's output into the pane and delivering the user's keystrokes to it.
- **FR-007**: The bar's child list MUST NOT vary with the session's pane or its instance count
  (feature 023 FR-008a). Every control named here is drawn unconditionally.
- **FR-008**: The terminal tabs MUST share a vertical midline with the "+" and the AI tab beside
  them. Added after the visual pass (T024) found them 4dp above both: the strip's edge-fade box is a
  bar control's full height so the fade spans the whole edge, and a container's default vertical
  alignment is `Start`. Nothing was out of place while the strip lived at the bar's *leading* edge
  with nothing beside it; FR-002 put it against two controls the bar row centres, and the step then
  reads as two rows of controls sharing a bar rather than the one trailing group FR-002 claims.
- **FR-009**: Every strip control that changes which tab is marked MUST ask for that tab to be
  scrolled into view (feature 026 FR-002d) — the "+" and a tab's close control included, not only
  the two tabs. Also from T024: at six instances the strip overflows, and pressing "+" created an
  instance, marked it, and left it behind the trailing edge fade. The reveal machinery was present
  and correct; two of the four arms that move the mark simply never asked it to run. That was
  survivable while the "+" opened instances into a strip with room for them, and FR-002/FR-003 are
  what make it reachable with a single press.

### Key Entities

No new entities. `TerminalMode` keeps its two variants and its meaning; what changes is that only
`Message::TerminalAiCliSelected` and `Message::ShellInstanceSelected` write it, and neither of them
flips — each names where it is going.

## Success Criteria *(mandatory)*

- **SC-001**: A user can reach either pane from the other in one press, from any starting state,
  with no control other than the strip involved.
- **SC-002**: After any switch, the process on screen is the process receiving the user's
  keystrokes — verified from the message the client sends the daemon, not from the pane alone.
- **SC-003**: The "+" and the AI tab occupy the same coordinates at every instance count the bar can
  hold, and at every count past it.
- **SC-004**: No layout node in the bar is laid out at zero width or past the bar's trailing edge at
  any covered instance count (feature 026 SC-008, unchanged and still gated).

## Out of Scope

- Keyboard navigation of the strip. Still out of scope, as in feature 026 — and unchanged by this
  feature: the chord that opens an instance is not strip navigation.
- Reordering tabs by dragging.
- Any change to what the two panes *are*, to process lifecycle, or to how a session remembers its
  mode across restarts (feature 010's FR-005 and FR-010 are untouched).
