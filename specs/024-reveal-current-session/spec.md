# Feature Specification: Reveal the current session in the sidebar

**Feature Branch**: `feat/expand-a-current-session`

**Created**: 2026-08-09

**Status**: Closed

**Input**: User description: "when I switch between project I would like to know at which session I'm currently. So the session to which I'm switched should be expanded in side panel and highlighted"

## Clarifications

### Session 2026-08-09

- Q: The spec listed a reveal trigger for "the current session ends and another takes over", but no such hand-over exists — closing or removing the current session leaves none current. Add hand-over, or drop the trigger? → A: Drop it. This feature reveals where you are; it does not change which session you are on. Closing/removing the current session leaves nothing current and reveals nothing.
- Q: Locations are discovered asynchronously and the list is replaced wholesale on refresh. What must the user observe when the location list arrives after the switch, or is replaced while the current session is unchanged? → A: Revealed-ness is derived from which session is current, not stored — the row opens as soon as its location is known and survives any refresh; only the user's own close closes it.
- Q: How should the one location that escapes the active filters (FR-011) present itself? → A: In its normal sort position, carrying a cue on the row stating it is shown because it holds the current session — reusing the row's existing tag/chip slot rather than pinning it or leaving it unexplained.
- Q: May the current-session mark rely on colour alone (the existing selected-row tint), given hover is also a tint change and the row already carries lifecycle colour and an activity dot? → A: No. The mark pairs the selected tint with at least one non-colour cue, so the current row is identifiable without relying on colour.
- Q: Does the reveal animate, and does that contradict SC-002's "complete by the first draw"? → A: Instant where the panel's contents are being replaced anyway (project switch, launch restore) — drawn already open, marked and scrolled; animated only when the reveal happens in a panel the user is already looking at (starting a new session).

### Session 2026-08-10 — corrections found during planning

Two statements in the requirements above described behaviour this application does not have. Both
were found by reading the code during `/speckit-plan`, and both are corrected in place rather than
left for implementation to discover.

- **"Restore at launch" was never a trigger.** FR-001, SC-002, SC-004 and US3 all named restoring a
  session at launch as one of the paths that makes a session current. No such path exists: the app
  starts with no current session and keeps none until the user picks or starts one. Rather than add
  the behaviour — which is a different feature, and one nobody asked for — FR-001 is restated as a
  rule over *every* path that makes a session current instead of a list of three, so it stays true
  as paths are added; FR-001d states the negative; SC-004 is measured by the absence of
  path-specific behaviour rather than by counting paths; and US3 scenario 1 now asserts what a cold
  start actually does. The reveal will cover a launch restore the day one exists, without this spec
  changing again.
- **FR-010a asked for an animation that does not exist.** It required the in-place reveal to be
  animated "using the same motion a user-initiated expand and scroll use" — but a user-initiated
  expand in this application is instant, and a user-initiated *scroll* is a drag with no motion to
  borrow. Read literally the requirement was unsatisfiable; read as intended it was already
  satisfied. It now says what it was for: the app's own reveal must never be a different experience
  from the user's own expand of the same row, whatever that experience is. If expansion later gains
  motion, the reveal inherits it — which is the guarantee the original clause was reaching for.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - See which session I landed on after switching projects (Priority: P1)

I keep several projects open and move between them all day. Each project has its own set of
locations (the project root and any number of worktrees), and each location can hold sessions.
When I switch to a project, the app puts one of its sessions in front of me — but the side panel
shows every location collapsed, so the session I am now looking at is not listed anywhere. I have
to guess which location holds it and open rows one by one to find out where I am.

After switching, I want the side panel to already have opened the location that holds the session
I have been dropped into, with that session's row marked as the one I am currently on.

**Why this priority**: This is the reported problem, and it is the whole point of the feature.
Without it the panel actively contradicts the main area — the main area shows a session, the panel
shows nothing selected. Delivered alone it is already a complete improvement.

**Independent Test**: Open two projects, each with a running session in a worktree. Switch from one
to the other and confirm, without any further clicks, that the incoming project's row holding the
current session is open and that session's row is marked as current.

**Acceptance Scenarios**:

1. **Given** project A is active and project B has a running session in a worktree,
   **When** I switch to project B,
   **Then** the side panel shows that worktree's row already opened and the session it holds marked
   as the current one.
2. **Given** the incoming project's current session lives in the project root location rather than
   a worktree,
   **When** I switch to that project,
   **Then** the project-root row is already opened and its current session is marked.
3. **Given** the incoming project has locations other than the one holding the current session,
   **When** I switch to it,
   **Then** those other rows are left as they were — only the row that holds the current session is
   opened on my behalf.
4. **Given** the incoming project has no session the app can put in front of me,
   **When** I switch to it,
   **Then** no row is opened and no session is marked as current, and the panel does not claim
   otherwise.
5. **Given** the row holding the current session was already open,
   **When** I switch to that project,
   **Then** it stays open — nothing flickers shut and back.

---

### User Story 2 - The current session is actually on screen (Priority: P2)

Some of my projects have many worktrees. Opening the right row is not enough if that row sits far
enough down the list that I cannot see it — I still do not know where I am until I scroll and hunt.
When the app reveals the current session, I want the panel scrolled so I can see it.

**Why this priority**: It completes the promise of US1 for the projects where it matters most, but
US1 is useful on its own for the common case of a short list that fits without scrolling.

**Independent Test**: Give a project enough locations that its list overflows the panel, with the
current session's location near the bottom. Switch to that project and confirm the current
session's row is visible without scrolling.

**Acceptance Scenarios**:

1. **Given** a project whose location list is longer than the side panel is tall, with the current
   session's location below the visible area,
   **When** I switch to that project,
   **Then** the panel is scrolled so the current session's row is visible.
2. **Given** the current session's row is already visible,
   **When** the app reveals it,
   **Then** the panel does not scroll — the list does not move under me for no reason.
3. **Given** the panel scrolled to reveal the current session,
   **When** I then scroll the panel myself,
   **Then** my scrolling is respected and the panel does not snap back.

---

### User Story 3 - Reveal wherever the app moves me, not just on a project switch (Priority: P2)

The app decides which session is in front of me in more situations than a project switch — starting
a new session is the other one today. That leaves me in the same position as a switch does — a
session in the main area with nothing marked in the panel. I want the same reveal wherever the app
moves me, including in any path that gains the ability to move me later.

**Why this priority**: Same value as US1 and the same mechanism, applied to the remaining paths.
Separated because US1 is the reported case and can ship first.

**Independent Test**: Start a session and confirm the panel ends with it marked and its location
open, with no clicks in the panel. Then confirm the paths that must *not* reveal — clicking a
session, and closing the one I am on — leave the panel alone.

**Acceptance Scenarios**:

1. **Given** the app has just started and has not put any session in front of me,
   **When** the panel first appears,
   **Then** no session is marked and no row is opened on my behalf — and if the app later gains the
   ability to restore a session at launch, that session is revealed exactly as a switch reveals one.
2. **Given** I start a new session in some location,
   **When** it becomes the session in front of me,
   **Then** that location is open and the new session is marked as current.
3. **Given** I close or remove the session I am on,
   **When** nothing becomes current in its place,
   **Then** no session carries the current mark and no row is opened or closed on my behalf.
4. **Given** I click a session in the panel,
   **When** it becomes current,
   **Then** it is marked as current and nothing is opened or scrolled on my behalf — it was already
   in view.

---

### User Story 4 - Reveal it even when my filters would hide it (Priority: P3)

I use the panel's tag filters and I keep agent worktrees hidden. When the app drops me into a
session that lives in a location my filters exclude, the panel has nothing to open — so I am back
to not knowing where I am, in exactly the situation I set the filters up for. I want the location
holding my current session to appear regardless, so the panel never lies about where I am.

**Why this priority**: A narrower case than US1-US3 — it needs a filter to be on — and it can only
be built once the reveal itself exists.

**Independent Test**: Turn on a tag filter that excludes the location holding the current session,
then switch away and back. Confirm that location appears, opened, with the current session marked
and the row saying why it is shown, while every other excluded location stays hidden.

**Acceptance Scenarios**:

1. **Given** a tag filter is on that excludes the location holding the current session,
   **When** the app reveals the current session,
   **Then** that one location appears in the panel, opened, with the current session marked, and
   all other filtered-out locations remain hidden.
2. **Given** a location is listed only because it holds the current session,
   **When** I look at its row,
   **Then** it sits where it would sit unfiltered and says on the row that it is shown because it
   holds the current session — a location the filter admits on its own says nothing of the kind.
3. **Given** agent worktrees are hidden and the current session lives in one,
   **When** the app reveals the current session,
   **Then** that agent worktree appears, opened, with the current session marked, and other agent
   worktrees remain hidden.
4. **Given** a location is showing only because it holds the current session,
   **When** the current session moves to a location the filters do allow,
   **Then** the previously-revealed location returns to being hidden.

---

### Edge Cases

- **The list of locations arrives after the switch, or is replaced while the current session is
  unchanged** (a worktree was created, deleted, or re-discovered): the current session's row opens as
  soon as its location is known and stays open across the replacement; a row the user closed stays
  closed.
- **The location holding the current session no longer exists** (its worktree was removed or has
  gone missing while the project was inactive): nothing is opened for it; the app must not open an
  unrelated row or mark an unrelated session.
- **The user closes the revealed row.** Their collapse stands. The app does not re-open it until
  the next time it moves the current session for them.
- **The current session's row is marked while its terminal does not have keyboard focus.** Being
  the current session and having typing focus are separate things (a project switch deliberately
  does not carry focus across); the mark reflects the former.
- **Switching to a project and straight back.** Whatever was revealed on the way out is revealed
  again on the way back; the panel does not accumulate opened rows from every project visited.
- **A project with exactly one location and no sessions.** Nothing is opened, nothing is marked.
- **Two sessions in the same location, one current.** Only one row carries the current mark.
- **The panel is not wide enough to show the full session name.** The current mark must still be
  unambiguous — it cannot depend on reading the whole label.
- **The current session's row is hovered.** It reads as both current and hovered without either
  signal cancelling the other, and it is still tellable from a hovered row that is not current.
- **Colour is unavailable or indistinguishable** (a colour-vision deficit, a greyscale rendering):
  the current row is still identifiable (FR-003a).

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: Whenever the app itself makes a session the current one, the side panel MUST open the
  location that holds that session so its row is listed. This MUST hold for every such path rather
  than for an enumerated set: today those paths are a project switch and a newly started session,
  and any path added later MUST reveal without needing this requirement restated.
- **FR-001d**: The app making *no* session current is not one of those paths. Nothing is opened,
  marked, or scrolled when the current session merely goes away — see FR-001a — or when the app
  starts with none, which is what it does today (there is no restore-a-session-at-launch behaviour
  to reveal for).
- **FR-001a**: Closing or removing the current session MUST leave no session current, and MUST NOT
  open, close, or scroll anything on the user's behalf. This feature does not promote another
  session in its place.
- **FR-001b**: The location holding the current session MUST be shown open for as long as that
  session is current and its location is known, however late that location becomes known and however
  many times the list of locations is replaced. Only the user closing it (FR-005) closes it.
- **FR-001c**: When a session stops being current, its location MUST NOT be closed on the user's
  behalf — it stays open until the user closes it, the app reveals a different location, or the
  active project changes. Ceasing to be current takes the mark away (FR-002), never the open row.
- **FR-002**: The side panel MUST mark exactly one session row as the current one whenever there is
  a current session, and MUST mark none when there is not.
- **FR-003**: The current-session mark MUST be distinguishable from a merely hovered row and from an
  ordinary row at a glance, without relying on reading the session's name.
- **FR-003a**: The current-session mark MUST NOT be carried by colour alone: it MUST include at least
  one non-colour cue, so the current row remains identifiable when colour differences cannot be
  perceived. It MUST NOT weaken or displace the signals a session row already carries — its
  lifecycle-tinted name and its activity indicator.
- **FR-004**: Opening a location on the user's behalf MUST NOT change whether any other location is
  open or closed.
- **FR-005**: A location the user has closed MUST stay closed until the app next makes a session
  current for them; closing a revealed row MUST NOT be undone while the current session is unchanged.
- **FR-006**: When the user makes a session current by choosing it in the side panel, the app MUST
  mark it as current and MUST NOT open or scroll anything on their behalf.
- **FR-007**: Open/closed state revealed for one project MUST NOT carry into another project — after
  a switch, the only location opened on the user's behalf is the one holding the incoming project's
  current session.
- **FR-008**: When the app reveals the current session and its row is outside the visible part of the
  panel, the panel MUST scroll so that row is visible.
- **FR-009**: When the current session's row is already visible, revealing it MUST NOT scroll the
  panel.
- **FR-010**: After the panel has been scrolled to reveal the current session, the user's own
  scrolling MUST take effect and MUST NOT be overridden until the app next makes a session current.
- **FR-010a**: When the reveal accompanies a wholesale change of the panel's contents (a project
  switch), the panel MUST be drawn already open, marked, and scrolled — no transition into that
  state is shown. When the reveal happens in a panel whose contents the user is already looking at
  (starting a new session), the opening MUST behave exactly as a user-initiated expand of the same
  row behaves — whatever motion that is, including none. The requirement is that the app's own
  reveal is never a *different* experience from the user's own expand, not that either is animated.
- **FR-011**: The location holding the current session MUST be listed in the panel even when the
  active tag filters or the hidden-agent-worktree setting would exclude it.
- **FR-012**: An exemption under FR-011 MUST apply only to the location holding the current session;
  every other excluded location MUST stay hidden, and the exemption MUST end as soon as that
  location no longer holds the current session.
- **FR-012a**: A location listed only by the FR-011 exemption MUST appear in the same position it
  would occupy unfiltered, and MUST state on the row that it is shown because it holds the current
  session, so its presence despite the filter is self-explaining. A location the filters admit on
  their own MUST NOT carry that cue.
- **FR-013**: When there is no current session, or the location that held it no longer exists, the
  app MUST leave the panel's open/closed state and scroll position alone and MUST mark no session.
- **FR-014**: The current-session mark MUST be independent of whether that session's terminal holds
  keyboard focus.
- **FR-015**: The current-session mark MUST be independent of the session's run state — a stopped,
  failed, or interrupted session that is the current one MUST still carry the mark, alongside
  whatever its state already communicates.

### Key Entities

- **Current session**: the one session of the active project that the main area is showing. At most
  one exists at a time, and it may be absent.
- **Location**: a place within a project that can hold sessions — the project root, or one of the
  project's worktrees. Each location's row in the panel is either open (listing its sessions) or
  closed.
- **Reveal**: the act of opening the current session's location, marking that session's row, and
  bringing it into view. Triggered by the app moving the current session, never by the user
  operating the panel directly.
- **Side panel**: the sidebar listing the active project's locations and, for each open location, its
  sessions. Referred to throughout as the side panel; "sidebar" means the same thing.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: After switching projects, the user can tell which session they are on by looking at
  the side panel, with zero clicks and no scrolling.
- **SC-002**: On a project switch, the reveal is complete by the time that project's panel is first
  drawn — the user never sees an intermediate state where the current session is unmarked or hidden,
  and no transition into the revealed state is visible.
- **SC-003**: In a project with 30 locations, the current session's row is visible in the panel
  immediately after a switch, in every case where that session exists.
- **SC-004**: Across every path where the app makes a session current — a project switch and a newly
  started session today — the panel ends in the same revealed state, and no path is an exception.
  Measured by there being no path-specific reveal behaviour to find, rather than by counting paths:
  a path added later that did not reveal would fail this criterion.
- **SC-005**: Turning on a filter that excludes the current session's location does not make the
  panel stop showing where the user is, and does not reveal any other excluded location.
- **SC-006**: A location the user closes stays closed for the whole time they remain on the same
  session — it never re-opens by itself.
- **SC-007**: Panel scroll position is never moved while the current session is unchanged.
- **SC-008**: Creating, deleting, or re-discovering worktrees never closes the current session's row
  and never re-opens a row the user closed.

## Assumptions

- "The session I'm switched to" means the current session as the app already chooses it on a switch
  (the project's remembered foreground session, else its first running one, else none). This feature
  changes only how that choice is presented, not how it is made.
- The mark on the current session's row builds on the existing selected-row treatment for session
  rows rather than replacing it; FR-003a adds a non-colour cue to it. That cue is one addition, not a
  second competing indicator, and it is not the activity indicator the row already has.
- Marking the containing location's row when it is closed is out of scope: the app opens it, so the
  session row itself carries the signal. A user who closes the row has chosen to give that up.
- Nothing about this reveal is remembered across restarts. Which rows are open is derived fresh from
  the current session each time the app moves it, so there is no new stored preference.
- Scroll-into-view means the row is fully visible; it does not require the row to be centred or
  pinned to a particular edge of the panel.
- Keyboard focus is unaffected. Revealing a session does not put the cursor in its terminal — that
  remains a separate, explicit action.
- Sessions belonging to projects that are not active are out of scope; the panel shows the active
  project's locations only, as it does today.
