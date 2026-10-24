# Feature Specification: One 44 px title bar with a breadcrumb and a jump-to field

**Feature Branch**: `claude/project-thread-iwrht0`

**Created**: 2026-10-24

**Status**: Draft

**Input**: User description: "Implement GitHub issue #1127: Layout: single 44px title bar with
breadcrumb and jump-to field. Blocked by: #1125 (Overview moves into the sidebar first), #1124
(banners leave the top). One 44 px bar: app mark, project switcher as a breadcrumb (`project /
worktree`), centred jump-to field opening the quick switcher (Ctrl+Shift+P), plan usage, Settings,
overflow menu. The Overview button leaves the bar; Ctrl+Shift+O unchanged. Acceptance: switcher
panel anchors under the breadcrumb; shortcuts unchanged; Overview still reachable from sidebar and
shortcut; docs, layout snapshot and visual pass updated. Mockup:
https://claude.ai/artifact/QXMeLzfh2oEkC9CAwn1M2e" Labels: enhancement, flow:feature. Builds on
features 008 (project switcher), 027 (overflow menu, FR-026e), 039 (unread mark on the switcher),
813 (Overview button and badge), 968 (`specs/968-quick-switcher`), 970 (Overview full screen), 1120
(short branch), 1124 (`specs/1124-bottom-status-bar`) and 1125 (`specs/1125-sidebar-state-groups`).
Part of the layout redesign: #1125 (sidebar), #1126 (context header and session tabs) and #1128
(layout docs and visual pass) are out of scope here.

## Terms

- **Title bar**: the app's own bar across the top of the window, inside the window frame (today
  "the top app bar", 64 px plus a 1 px divider). It is not the operating system's window title bar,
  which stays as it is.
- **App mark**: the small Micold logo at the bar's leading end, in place of today's "Micold" title
  text.
- **Breadcrumb**: the bar's `project / location` path. Its first segment, the **project segment**,
  is the project switcher's trigger (feature 008). Its second segment, the **place segment**, names
  what the main area shows.
- **Jump-to field**: a field-shaped control centred in the bar's free space that opens the quick switcher
  (feature 968). It is pressed, not typed into: typing happens in the quick switcher once it is
  open.
- **Location**, **Overview**, **attention badge**: as defined by feature 813. **Short branch**: as
  defined by feature 1120.
- **Overview row**: the sidebar's first row, added by feature 1125 (its User Story 2).
- **Status bar**: the bottom bar of feature 1124, which holds plan usage and the key hints.

## Clarifications

### Session 2026-10-24

- Q: The issue lists plan usage among the title bar's items, but feature 1124 moved plan usage into
  the status bar. Which place holds it? → A: The status bar, alone. The title bar draws no plan
  usage. _(agent-resolved: feature 1124 is merged and settled this in its own clarification —
  `specs/1124-bottom-status-bar/spec.md#Clarifications` "Is plan usage shown in the status bar or
  left in the title (app) bar? → A: In the status bar", FR-011 — and the mockup's own canvas note
  says the status bar "collects connection, sandbox, CLI and plan usage"; the mockup bar's
  "Plan usage 62%" is the pre-#1124 draft. Drawing it in both places would show one reading twice.)_
- Q: Can the bar rebuild land before #1125 puts the Overview row in the sidebar? → A: Yes. Stories 1
  and 2 do not depend on #1125; only Story 3 (the Overview button leaves the bar) does, and it lands
  after #1125's Overview row is merged. Until then the Overview button stays in the new bar, after
  the breadcrumb. _(agent-resolved: issue #1127 "Blocked by: #1125 (Overview moves into the sidebar
  first)"; specs/1125-sidebar-state-groups/autopilot.md lists the Overview row as its milestone M3,
  not yet merged)_
- Q: Where is the jump-to field centred: on the window's centre line, or in the free space between
  the breadcrumb and the trailing controls? → A: In the free space between the leading group (app
  mark, breadcrumb and, while FR-015 applies, the Overview button) and the trailing group (Settings,
  overflow trigger), so a long breadcrumb never pushes it into the breadcrumb. _(agent-resolved:
  mockup `Main.dc.html` title bar, equal `flex:1` spacers either side of the field)_
- Q: What does the jump-to field do while Settings is open, where the quick switcher's shortcut does
  nothing? → A: It is inert and drawn disabled, as the Overview button is while Settings is open;
  with the Overview open or no project it opens the switcher as the shortcut does; while the
  switcher is open, pressing it closes the switcher, as a press outside it does.
  _(agent-resolved: `specs/968-quick-switcher/spec.md` FR-012, Edge Cases "Settings … open", User
  Story 1 scenario 6; `crates/micold-client/src/ui/toolbar.rs` `overview_press`)_
- Q: Is the app mark a control (pressable, a Tab stop)? → A: No. It is a decorative mark with a
  hover tooltip; it does nothing when pressed and is not in the Tab order. _(agent-resolved: mockup
  `Main.dc.html` draws it as a plain block, not a button; the overflow menu already holds About)_
- Q: Does the new bar keep today's raised look when sidebar content is scrolled under it? → A: Yes,
  unchanged: it is raised while the sidebar is scrolled and flat at rest. _(agent-resolved:
  `crates/micold-client/src/app.rs` `State::app_bar_elevated`, contract §7.1)_
- Q: Does the 4.5:1 / 3:1 contrast rule apply to the jump-to field while it is drawn disabled? → A:
  No. FR-017 applies to the enabled bar; the disabled field uses the shared Material disabled
  treatment (content at 38%), like every other disabled control. _(agent-resolved:
  `specs/003-material-design-layout/contracts/design-tokens.md` "Disabled";
  `docs/development/component-library.md` disabled treatment)_

## User Scenarios & Testing *(mandatory)*

### User Story 1 - One compact bar that says where I am (Priority: P1)

A developer wants the window's chrome to take less height and to read, at a glance, which project
and which worktree they are in. Today the top bar is 64 px tall and names only the project, on a
button at its far end. After this story one 44 px bar holds the app mark, a `project / location`
breadcrumb at its leading end, Settings and the overflow menu at its trailing end. Pressing the
project segment opens the project switcher panel right under it.

**Why this priority**: It is the issue's subject and the frame the other two stories fill in. It
gives back 21 px of height to the work on every view, and the breadcrumb answers "where am I"
without looking at the sidebar.

**Independent Test**: Open a project with two worktrees, select a session in one of them, and check
the bar's height, its items in order, the breadcrumb's text, and where the switcher panel opens when
the project segment is pressed. Repeat with the Overview open, with Settings open and with no
project.

**Acceptance Scenarios**:

1. **Given** a project `micold-ai-ide` is active and a session of the worktree on branch `fix/1117`
   is shown, **When** the window is drawn, **Then** the title bar is 44 px tall and shows, from the
   leading end: the app mark, the project segment `micold-ai-ide` with a drop-down glyph, a `/`
   separator, the place segment `fix/1117`; then the jump-to field in the centre; then a Settings
   button and the overflow-menu trigger at the trailing end.
2. **Given** that bar, **When** the user presses the project segment, **Then** the project
   switcher panel opens with its top edge directly under the bar and its leading edge aligned with
   the project segment's leading edge, and it lists the same projects, counts and actions as today.
3. **Given** the Overview is open, **When** the bar is drawn, **Then** the place segment reads
   `Overview`; **given** Settings is open, **then** it reads `Settings`.
4. **Given** no project is open, **When** the bar is drawn, **Then** the project segment reads
   `Select project` and there is no separator and no place segment.
5. **Given** a project is open and no location is selected, **When** the bar is drawn, **Then** the
   breadcrumb shows the project segment alone.
6. **Given** other projects have 3 unread sessions, **When** the bar is drawn, **Then** the project
   segment carries the unread mark "● 3" with its tooltip, as the switcher button does today
   (feature 039).
7. **Given** the bar, **When** the user presses Settings, **Then** Settings opens, as the overflow
   menu's Settings item does; **When** the user presses the overflow trigger, **Then** the menu
   opens under the trigger, under the bar, with the same items as today.
8. **Given** the bar, **When** the user hovers the app mark, **Then** a tooltip names the
   application (`Micold`).

---

### User Story 2 - Jump anywhere from a field in the bar (Priority: P2)

A developer who does not know, or forgets, the quick switcher's shortcut wants to see that it exists
and open it with the pointer. A field in the centre of the bar reads "Jump to location, session or
project" and shows the shortcut; pressing it opens the quick switcher.

**Why this priority**: The quick switcher (feature 968) is today reachable only by its shortcut and
the status bar's hint. The field makes it discoverable, but the bar of Story 1 is useful without it.

**Independent Test**: Press the jump-to field and check that the quick switcher opens, with its
input focused, exactly as Ctrl+Shift+P opens it; check the field's text and shortcut hint on Linux
or Windows and on macOS.

**Acceptance Scenarios**:

1. **Given** a project is open, **When** the user presses the jump-to field, **Then** the quick
   switcher opens with its query input focused, the same as pressing Ctrl+Shift+P (Cmd+Shift+P on
   macOS).
2. **Given** the field, **When** it is drawn, **Then** it shows a search glyph, the text "Jump to
   location, session or project" and the shortcut `Ctrl+Shift+P` (`Cmd+Shift+P` on macOS), and it
   is centred in the space between the breadcrumb and the trailing controls.
3. **Given** the quick switcher is already open, **When** the user presses the field, **Then** no
   second switcher opens; the open one closes, as it does on a press outside it.
4. **Given** the Overview is open or no project is open, **When** the user presses the field,
   **Then** the quick switcher opens, as the shortcut opens it in that state; **given** Settings is
   open, **then** the field is drawn disabled and pressing it does nothing, as the shortcut does
   nothing (feature 968 FR-012).
5. **Given** keyboard focus is in the bar, **When** the user tabs to the field and presses Enter or
   Space, **Then** the quick switcher opens.

---

### User Story 3 - The Overview leaves the bar for the sidebar (Priority: P3)

Once the sidebar's first row is the Overview (feature 1125), the bar's Overview button is a second
copy of it. The button leaves the bar; the Overview is opened from the sidebar row or with
Ctrl+Shift+O.

**Why this priority**: It completes the redesign but depends on #1125's Overview row being merged;
without that row, removing the button would leave the shortcut as the only way in.

**Independent Test**: With feature 1125's Overview row present, check the bar has no Overview
button, then open and close the Overview with the sidebar row and with Ctrl+Shift+O, in normal and
in full-screen Overview.

**Acceptance Scenarios**:

1. **Given** feature 1125's Overview row is in the sidebar, **When** the bar is drawn, **Then** it
   has no Overview button and no Overview badge.
2. **Given** that bar, **When** the user presses Ctrl+Shift+O (Cmd+Shift+O on macOS), **Then** the
   Overview opens and closes as it does today, also while a terminal has the keyboard.
3. **Given** that bar, **When** the user presses the sidebar's Overview row, **Then** the Overview
   opens (feature 1125).
4. **Given** the Overview is full screen (feature 970), **When** the user presses Ctrl+Shift+O,
   **Then** the Overview closes as it does today; the full-screen control in the Overview's header
   still leaves full screen.

---

### Edge Cases

- **Empty breadcrumb parts**: no project → `Select project` alone; a project with no selected
  location → project segment alone; a location with no branch (detached, or the project root
  outside git) → the place segment shows the location's display name.
- **Long names**: a long project name or branch is shortened with an ellipsis; the full text is in
  the segment's tooltip. The place segment shortens first, then the project segment.
- **Narrowest window (640 × 480, the supported minimum)**: every bar control stays visible, none
  overlaps another, and none is clipped by the bar's 44 px height. The jump-to field gives up width
  first: it drops its text, then its shortcut hint, and keeps at least its search glyph as a
  pressable target. The app mark, the project segment's glyph, Settings and the overflow trigger
  never hide.
- **Failure**: an unavailable active project (folder missing) keeps today's switcher behaviour; a
  quick switcher that has nothing to list opens as it does today. The bar adds no failure state of
  its own and raises no banner.
- **Several sessions and windows (Principle II)**: the place segment follows the location shown in
  this window; when another window, a session ending or the daemon removes that location, the
  segment changes on the same update as the main area (to the next selection, or to the project
  alone). Unread marks on the project segment update as sessions in other projects finish turns,
  without the panel being open.
- **Panels over the bar**: the switcher panel, the overflow menu and tooltips open below the bar's
  bottom edge, never over the trigger that opened them; opening one never changes the bar's height
  or reflows its items.
- **Settings open**: the Settings button shows it is selected; pressing it again changes nothing
  (Settings closes through its own controls, so an unsaved draft is never dropped by the bar).
- **Before Story 3 lands**: the Overview button stays in the bar, after the breadcrumb, with its
  badge, tooltip and inert rules unchanged (feature 813), and the bar is still 44 px.
- **After Story 3, sidebar hidden or Overview full screen**: the Overview row is out of view, so the
  Overview is reached by its shortcut, which the status bar's hint names (feature 1124 FR-008). The
  attention badge is seen again when the sidebar returns.
- **Both themes**: the bar's text (breadcrumb, separator, field text and hint) and its icons meet
  4.5:1 (text) and 3:1 (icons, field outline) against the bar in light and dark.
- **Platforms and display scale (Principle VI)**: the bar's items, order and height are the same on
  Linux, macOS and Windows at display scale 1.0 and 2.0. Shortcut labels read Cmd on macOS and Ctrl
  elsewhere. The operating system's own window frame and title bar are left as they are on every
  platform.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: The title bar MUST be 44 px tall, divider included, in every body branch (session
  view, Overview normal and full screen, Changes, Settings, no project), and no other chrome row MAY
  be drawn between it and the body in the steady state.
- **FR-002**: The bar MUST show, from the leading end: the app mark, the breadcrumb; in its centre
  the jump-to field; at its trailing end a Settings button and the overflow-menu trigger; and, until
  FR-016 applies, the Overview button after the breadcrumb (FR-015). It MUST NOT show the application name as text, nor plan usage.
- **FR-003**: The app mark MUST carry a tooltip naming the application. It is not a control: it
  MUST do nothing when pressed and MUST NOT be a Tab stop.
- **FR-004**: The project segment MUST read the active project's name (or `Select project` with
  none), carry a drop-down glyph and today's unread mark for other projects (feature 039), and open
  and close the project switcher panel when pressed, as the switcher button does today.
- **FR-005**: The switcher panel MUST open with its top edge at the bar's bottom edge and its
  leading edge aligned with the project segment's leading edge, and MUST otherwise keep its rows,
  counts, actions, context menu, scrolling and dismissal unchanged.
- **FR-006**: The place segment MUST read: `Overview` while the Overview is shown; `Settings` while
  Settings is shown; otherwise the short branch of the location the main area shows, or its display
  name when it has no branch. It MUST be absent, with its separator, when there is no project or no
  location selected. It MUST NOT be interactive.
- **FR-007**: A segment whose text does not fit MUST be shortened with an ellipsis and show its full
  text in a tooltip; the place segment MUST shorten before the project segment.
- **FR-008**: The jump-to field MUST be centred in the free space between the bar's leading group
  (app mark, breadcrumb, and the Overview button while FR-015 applies) and its trailing group
  (Settings, overflow trigger), and show a search glyph, the text "Jump
  to location, session or project" and the quick switcher's shortcut as the running platform writes
  it.
- **FR-009**: Pressing the jump-to field, by pointer or by Enter/Space with keyboard focus, MUST
  have the same effect as the quick switcher's shortcut in the same state, including opening the
  switcher with its query input focused. While Settings is open (where the shortcut does nothing,
  feature 968 FR-012) the field MUST be drawn disabled and ignore presses; while the quick switcher
  is open, a press on the field closes it as any press outside it does.
- **FR-010**: As the window narrows, the jump-to field MUST give up width first, dropping its text
  and then its shortcut hint while keeping a pressable search glyph; at the minimum window size
  (640 × 480) every bar control MUST stay visible, unclipped and not overlapping another.
- **FR-011**: The Settings button MUST open Settings, show itself selected while Settings is open,
  and do nothing when pressed while Settings is open.
- **FR-012**: The overflow menu MUST keep its items and their order (Settings, Session service
  diagnostics, About) and open under its trigger, below the bar.
- **FR-013**: Every bar control (the app mark is not one, FR-003) MUST be reachable by Tab in bar order and activated by Enter or
  Space, and MUST show a tooltip with its name, and its shortcut where it has one.
- **FR-014**: Every keyboard shortcut MUST keep its key and its effect; this feature adds and
  removes none.
- **FR-015**: Until feature 1125's Overview row is merged, the bar MUST keep the Overview button,
  after the breadcrumb, with its badge, tooltip and inert rules unchanged (feature 813).
- **FR-016**: Once feature 1125's Overview row is merged, the bar MUST NOT show an Overview button
  or badge, and the Overview MUST stay reachable from the sidebar's Overview row and from
  Ctrl+Shift+O (Cmd+Shift+O on macOS), in normal and full-screen Overview.
- **FR-017**: The bar's text MUST have a contrast of at least 4.5:1, and its icons and the field's
  outline at least 3:1, against the bar in both themes, for every enabled control; a disabled control (the jump-to field
  while Settings is open, FR-009) uses the shared disabled treatment.
- **FR-018**: The user guide (top bar, project switcher, worktrees and sessions, Settings, Overview, icons, and appearance pages), the
  layout snapshot and this feature's visual pass MUST be updated in the same change as the bar they
  describe, and MUST NOT describe an Overview button in the bar after Story 3 lands.
- **FR-019**: The bar MUST keep today's elevation rule: raised while the sidebar's content is
  scrolled under it, flat at rest.

### Key Entities

- **Breadcrumb**: project segment (name, unread count, switcher open or not) and an optional place
  segment (text derived from what the main area shows).
- **Jump-to field**: label, shortcut label, how much of it fits (full, glyph and hint, glyph only).

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: The chrome above the body measures 44 px in all five body branches, 21 px less than
  today's 65 px.
- **SC-002**: 100% of the documented keyboard shortcuts behave as before, checked against the user
  guide's shortcut list.
- **SC-003**: At 640 × 480 and at 1280 × 800, in light and dark, no bar control overlaps another or
  is clipped.
- **SC-004**: The switcher panel opens within 1 px below the bar and within 1 px of the project
  segment's leading edge, in every body branch.
- **SC-005**: In every tested session-view state, the bar's breadcrumb text equals the active
  project's name and the shown location's short branch (or display name), so the bar alone says
  where the user is.
- **SC-006**: After Story 3, the Overview opens in one press from the sidebar row or one shortcut, in
  100% of the tested states (sidebar shown, session view, Changes, full screen via shortcut).

## Out of scope

- The sidebar's Overview row, its badge and its grouping (#1125).
- The context header and session tabs above the terminal (#1126).
- The whole-layout documentation and visual pass of the redesign (#1128); this feature updates only
  what the title bar changes.
- Plan usage in the title bar: it stays in the status bar (feature 1124).
- Drawing into, hiding or replacing the operating system's window frame and title bar.
- The quick switcher's contents, matching and placement (feature 968), and the switcher panel's
  rows.
- New keyboard shortcuts, or a shortcut to focus the bar.

## Assumptions

- The place segment uses the short branch (feature 1120), as the mockup's `micold-ai-ide /
  fix/1117` does, rather than the issue title, which feature 1126's context header shows beside the
  terminal.
- The overflow menu keeps its Settings item although the bar now has a Settings button: one press
  from either place, both opening the same view, so there is still one writer (feature 027 FR-026e).
- The quick switcher opens where it opens today; the jump-to field is a way in, not its anchor.
- The app mark is the existing application icon at a size that fits the 44 px bar.
- Story 3's milestone is scheduled after #1125's Overview row milestone merges; Stories 1 and 2 do
  not wait for it.
