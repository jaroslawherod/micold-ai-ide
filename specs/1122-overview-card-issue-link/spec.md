# Feature Specification: Overview cards, a link to the worktree's issue

**Feature Branch**: `claude/project-thread-7041o6`

**Created**: 2026-10-21

**Status**: Draft

**Input**: User description: "Implement GitHub issue #1122: Overview cards: link to the worktree's
issue. Blocked by: #1120 (the branch line the link sits on). 'Issue #N' beside the branch opens the
GitHub issue in the browser; N comes from the worktree name, as the issue-title summary already
does. A Jira key (MIC-87) in the name links to `<Jira base URL>/browse/<key>`, which needs a Jira
base URL setting and shows no link while it is unset. A press on a link does not activate the card.
No link on Default or on a worktree with no issue. Acceptance: both links open the right URL;
Settings has the Jira field; card activation is unaffected; docs updated." Labels: enhancement,
flow:feature. Builds on features 1120 (`specs/1120-overview-card-readability`) and 1121
(`specs/1121-overview-card-full-text`), both shipped, and on features 813, 972, 031, 533 and 1102.

## Terms

- **Overview**, **card**, **location**, **display name**: as defined by feature 813
  (`specs/813-worktree-overview/spec.md#Terms`).
- **Branch line**, **short branch**, **title**, **header row**, **body**, **card width**: as defined
  by feature 1120 (`specs/1120-overview-card-readability/spec.md#Terms`); the branch line is the
  line that shows the short branch.
- **Collapsed card**, **expanded card**: as defined by feature 1121
  (`specs/1121-overview-card-full-text/spec.md#Terms`). The expanded card shows the full branch name
  where the collapsed card shows the short branch (1121 FR-004).
- **Ticket**: the issue reference a worktree's directory name carries before its ticket boundary,
  read the way the card's issue-title lookup already reads it (feature 813, T054): the directory
  `feat-1122_slug` carries `1122`, and the directory `feat-mic-87_slug` (branch `feat/mic-87_slug`)
  carries `mic-87`, read as the Jira key `MIC-87`.
- **GitHub ticket**: a ticket that is all decimal digits. **Jira key**: a ticket of the Jira form,
  a project key (a letter, then letters, digits or `_`) followed by `-` and a decimal number,
  compared ignoring letter case and written upper case (`mic-87` reads `MIC-87`).
- **Issue link**: the text on a card that opens the location's issue in the web browser:
  "Issue #1122" for a GitHub ticket, "Issue MIC-87" for a Jira key.
- **Project's GitHub repository**: the github.com repository the project's remote names, the one the
  create-worktree dialog lists GitHub issues from (feature 034).
- **Jira base URL**: the new setting, the web address issue keys are browsed under
  (`https://example.atlassian.net`); a Jira key links to `<Jira base URL>/browse/<KEY>`.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Open a worktree's GitHub issue from its card (Priority: P1)

A developer looks at the Overview and wants the issue a worktree is for: its description, its
discussion, its checklist. Today they read the number off the branch and look it up by hand. With
this change the card's branch line carries "Issue #1122", and a click on it opens that issue's page
in the browser, without leaving the Overview or switching to the worktree.

**Why this priority**: It is the issue's core: most worktrees in this project are named after a
GitHub issue.

**Independent Test**: In a project whose remote is `github.com/o/r`, show the Overview with a
worktree named `feat-1122_overview-issue-link`; click "Issue #1122" and check that the browser is
asked to open `https://github.com/o/r/issues/1122`, that the Overview stays open and that no
location is selected.

**Acceptance Scenarios**:

1. **Given** a worktree whose name carries GitHub ticket 1122 in a project whose GitHub repository
   is `o/r`, **When** its card is shown, **Then** the branch line shows the issue link
   "Issue #1122" beside the short branch.
2. **Given** that card, collapsed or expanded, **When** the user clicks the issue link, **Then** the
   browser is asked to open `https://github.com/o/r/issues/1122`, and the card is not activated: the
   Overview stays open, no location is selected, and the card shows no ripple.
3. **Given** the pointer over an issue link, **When** it is shown, **Then** the link reads as a link
   (link colour, an underline while the pointer is on it, and the pointing-hand pointer).
4. **Given** the *Default* card, a worktree whose name carries no ticket, or one whose ticket is
   neither a GitHub ticket nor a Jira key, **When** the card is shown, **Then** it shows no issue
   link.
5. **Given** a project with no GitHub repository, **When** a worktree's name carries a GitHub
   ticket, **Then** its card shows no issue link.
6. **Given** the GitHub issues switch (feature 1102) off, **When** a worktree's name carries a
   GitHub ticket, **Then** its card shows no issue link; turning the switch on shows it without a
   restart (FR-001 working assumption, FR-012).

---

### User Story 2 - Open a worktree's Jira issue from its card (Priority: P2)

A developer whose team tracks work in Jira names worktrees after Jira keys (feature 533 names their directories
`feat-mic-87_<slug>`). They enter their Jira base URL in Settings once; from then on each such card
carries "Issue MIC-87", and a click on it opens `<Jira base URL>/browse/MIC-87`.

**Why this priority**: It extends the same link to the project's other tracker; without the setting
it shows nothing, so it costs no one who does not use Jira.

**Independent Test**: With the Jira integration on and the Jira base URL unset, show a card for the
worktree `feat-mic-87_add-login` and check it has no link; set the base URL to
`https://example.atlassian.net/` in Settings, check the card now shows "Issue MIC-87", click it and check the browser is asked to open
`https://example.atlassian.net/browse/MIC-87`.

**Acceptance Scenarios**:

1. **Given** the Jira base URL unset, **When** a worktree's name carries a Jira key, **Then** its
   card shows no issue link.
2. **Given** the Jira base URL set to `https://example.atlassian.net`, **When** a worktree's name
   carries `mic-87`, **Then** its card shows "Issue MIC-87", and a click on it asks the browser to
   open `https://example.atlassian.net/browse/MIC-87`, without activating the card.
3. **Given** Settings with the Jira integration on, **When** the user opens its Jira section,
   **Then** it has a **Jira base URL** field, empty by default, with one line saying it is used for
   issue links on the Overview's cards.
4. **Given** the field, **When** the user saves a value that is not an `https` web address, or has a
   user name, a query or a fragment, **Then** the save is refused, the field is marked with the
   reason, and the stored value is unchanged.
5. **Given** a saved value with a trailing `/` (`https://example.atlassian.net/`) or a path
   (`https://jira.example.com/jira`), **When** a link is built, **Then** it has exactly one `/`
   before `browse` (`https://example.atlassian.net/browse/MIC-87`,
   `https://jira.example.com/jira/browse/MIC-87`).
6. **Given** the Overview open in two windows, **When** the user sets or clears the Jira base URL, or
   turns the Jira integration switch on or off, in one, **Then** Jira links appear or disappear in
   both without a restart.
7. **Given** the Jira base URL set and the Jira integration switch (feature 533) off, **When** a
   worktree's name carries a Jira key, **Then** its card shows no issue link (FR-001 working
   assumption).

---

### User Story 3 - Follow an issue link by keyboard (Priority: P2)

A keyboard user moves through the cards (feature 813 FR-014, feature 1121 Story 3). With this change
Tab reaches a card's issue link right after the card, and Enter opens it, so the link needs no
pointer.

**Why this priority**: A control the pointer can use and the keyboard cannot is a gap the Overview
has so far avoided (1121 SC-001).

**Independent Test**: Without touching the pointer, open the Overview, press Tab until the first
card with an issue link has focus, press Tab once more and Enter, and check that the browser is
asked to open the issue and that the Overview stays open; check what the arrow keys, Escape and
Shift+Tab do from the link.

**Acceptance Scenarios**:

1. **Given** keyboard focus on a card that has an issue link, **When** the user presses Tab, **Then**
   focus moves to that card's issue link, which shows a focus ring, and the card stays expanded (as
   a focused card is, feature 1121 FR-002); a further Tab moves focus to the next card. Shift+Tab
   moves the same way back: from a card to the previous card's issue link when that card has one,
   from an issue link to its own card.
2. **Given** focus on an issue link, **When** the user presses Enter or Space, **Then** the browser
   is asked to open the issue, and the card is not activated.
3. **Given** focus on an issue link, **When** the user presses an arrow key, **Then** focus moves
   to the next or previous card exactly as from the link's own card; the arrow keys never stop on a
   link (feature 813 FR-014, 1121 FR-011 unchanged for arrows).
4. **Given** focus on an issue link of an expanded card, **When** the user presses Escape, **Then**
   the card collapses and focus stays on the link; with no card expanded, Escape closes the Overview
   (feature 1121 FR-005).
5. **Given** focus on an issue link, **When** the user presses the Menu key or Shift+F10, **Then**
   the card's menu opens as from the card (feature 972), and closing it without a choice returns
   focus to the link it was opened from, without re-expanding a card collapsed by Escape (FR-009).
6. **Given** focus on an issue link of an expanded card whose body scrolls (1121 FR-008), **When**
   focus arrives on the link or the user presses Page Up or Page Down, **Then** the body scrolls so
   the link is in view on arrival, and Page Up / Page Down scroll the body as from the card.

---

### User Story 4 - The card behaves as before everywhere else (Priority: P1)

The link is a small target on a large one. Everything else about the card, collapsed and expanded,
must behave as features 1121 and 972 left it.

**Why this priority**: The issue's acceptance names it ("card activation is unaffected"); activating
a card closes the Overview and switches location, so a stray activation is costly.

**Independent Test**: On a card with an issue link, collapsed and expanded, click the card outside
the link, press Enter on the card, open its menu by a secondary click on the link and elsewhere, and
compare with a card without a link.

**Acceptance Scenarios**:

1. **Given** a card with an issue link, collapsed or expanded, **When** the user clicks anywhere on
   it outside the link, **Then** the card activates as today (feature 813 FR-013) with its ripple
   (feature 971).
2. **Given** a collapsed card, **When** the pointer comes to rest on its issue link, **Then** the
   card does not expand, so the link stays where the pointer is; when the pointer moves onto the
   rest of the card and rests there, the card expands as feature 1121 defines.
3. **Given** an expanded card, **When** the pointer moves onto its issue link, **Then** the card
   stays expanded, and the link is the one drawn on the expanded card.
4. **Given** a card with an issue link, **When** the user secondary-clicks the link, **Then** the
   card's menu opens as for a secondary click anywhere on the card (feature 972).
5. **Given** a pointer press that starts on the link and is released outside it (or the reverse),
   **When** the button is released, **Then** neither the link opens nor the card activates.

---

### Edge Cases

- **Title is the branch** (no issue title known, feature 1120 FR-007): the card has no short branch
  to show, so the issue link stands alone on the line where the branch line would be.
- **Orphan or invalid location** (no branch line, 1121 Edge Cases): the link depends only on the
  worktree's name, so it shows on its own line where the branch line would be.
- **Narrow card (280 px)** with a long short branch: the link is never ellipsized or clipped; the
  short branch gives way and ends in an ellipsis first (1120 FR-007, 1121 FR-007). On the expanded
  card the full branch wraps (1121 FR-007) and the link follows it, never cut by the card's edge.
- **Link moves when the card expands**: the expanded card adds lines above the branch line (the
  wrapped title, the display name, 1121 FR-004), so the link is lower on the expanded card. Story 4
  scenario 2 keeps a resting pointer on a collapsed card's link from triggering that move; a press
  goes to whatever is drawn under the pointer at the moment of the press.
- **Content changes under the pointer** (an issue title arrives and the title grows, 1121 Edge
  Cases): the link may move; a press goes to what is drawn at the moment of the press.
- **Browser cannot be opened** (no registered browser, the launch fails): the user sees the
  notification feature 031 FR-015 defines, naming the address and the reason; the Overview stays as
  it was.
- **Window loses focus to the browser**: the expanded card collapses (1121 FR-005); the Overview
  stays open. After a keyboard activation focus is on the link when the window comes back; after a
  click, keyboard focus is where it was before the click (FR-005).
- **Same issue on two worktrees**: each card has its own link to the same address.
- **Jira key with no base URL, GitHub ticket with no GitHub repository**: no link, and nothing on
  the card says one is missing.
- **Jira base URL changes while the Overview is open**: the cards' Jira links follow at once (Story 2
  scenario 6); a link focused when it disappears hands focus to its card.
- **Card goes away while its link has focus** (the worktree is removed): focus moves as feature 972
  FR-011 defines for a focused card that goes away.
- **Multi-session (Principle II)**: the link is per location, whatever sessions the location has;
  opening it touches no session.
- **Full screen** (feature 970): links behave the same; the zoom and its scroll anchor are
  unaffected.
- **Platforms (Principle VI)**: the browser is asked through each platform's opener (feature 031),
  on Linux, macOS and Windows alike; the link reads and behaves the same on all three, at display
  scale 1.0 and 2.0, in both themes.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: A card MUST show an issue link when its location is a worktree whose name carries a
  ticket, and either the ticket is a GitHub ticket and the project has a GitHub repository, or the
  ticket is a Jira key and the Jira base URL is set; otherwise it MUST show none. The *Default* card
  never shows one. [NEEDS CLARIFICATION: Do the links follow the integration switches, so that the
  GitHub link shows only while the GitHub issues switch (feature 1102) is on and the Jira link only
  while the Jira integration switch (feature 533) is on, or do they show whatever the switches say,
  since a link sends nothing from the app? Working assumption: they follow the switches.]
- **FR-002**: The issue link MUST read "Issue #N" for GitHub ticket N and "Issue KEY" for Jira key
  KEY written upper case.
- **FR-003**: The issue link MUST sit on the branch line, after the branch, on the collapsed and the
  expanded card; where the card has no branch line (the title is the branch, or the location is
  orphan or invalid) it MUST sit alone on a line in the branch line's place. It MUST NOT be
  ellipsized or clipped at any card width; the short branch gives way first.
- **FR-004**: Activating a GitHub issue link MUST ask the browser to open
  `https://github.com/<owner>/<repo>/issues/<N>` for the project's GitHub repository; activating a
  Jira issue link MUST ask it to open the Jira base URL, without a trailing `/`, followed by
  `/browse/<KEY>`.
- **FR-005**: A primary click on an issue link (press and release both on it), and Enter or Space
  while it has keyboard focus, MUST activate the link and MUST NOT activate the card: no ripple, no
  location selected, the Overview stays open, full screen stays as it was. A primary press that
  starts on an issue link MUST NOT start a ripple (replacing feature 971 FR-001 for the link's area);
  a press that starts outside the link and is released on it starts the ripple as 971 FR-001
  defines, and it fades as after any press released off its target. A pointer press on an issue
  link MUST NOT move keyboard focus, so a click never expands a collapsed card through FR-008.
- **FR-006**: A press and release on the card outside its issue link, a secondary click anywhere on
  the card including on its link, and every key on the card except Tab and Shift+Tab (FR-008) MUST behave as feature 1121 FR-010 and
  FR-011 define; a press on the link released outside it, or the reverse, MUST do nothing.
- **FR-007**: While the pointer rests on the issue link of a collapsed card, that card MUST NOT
  expand on that rest; a rest on any other part of the card expands it as feature 1121 FR-002
  defines. An expanded card MUST stay expanded while the pointer is on its link. For feature 1121
  FR-005's re-expansion rule, a rest on a card's link counts as a rest on that card: a focused card
  does not expand again while the pointer rests on another card's link, so it never covers that link.
- **FR-008**: The issue link MUST be a keyboard focus stop reached by Tab right after its own card
  and by Shift+Tab right before the next card; the arrow keys MUST move between cards only, from a
  link as from its card. While a card's link has focus the card counts as focused for feature 1121
  (it expands and collapses as a focused card does), and moving focus between a card and its own
  link MUST NOT re-expand a card collapsed by Escape. A focused link MUST show a focus ring; if it
  lies in the scrolled part of an expanded card's body, the body MUST scroll to show it.
- **FR-009**: The Menu key or Shift+F10 on a focused link MUST open the card's menu as on the card
  (feature 972), and closing that menu without a choice MUST return focus to the link, not the card
  (replacing 972 FR-011's hand-back for this case), counting as a move between a card and its own
  link (FR-008); Escape on a focused link MUST do what it does on its card (feature 1121 FR-005),
  leaving focus on the link.
- **FR-010**: Settings MUST have a **Jira base URL** field in its Jira section, empty by default,
  with one line saying it is used to link Jira keys on the Overview's cards, and kept in
  `settings.json`; like the section's other controls it shows only while the Jira integration is on
  (feature 533 FR-001); a settings file without it MUST read
  as unset. [NEEDS CLARIFICATION: Is the Jira base URL one new app-wide field, as the issue says, one field per project like the Jira connection
  (feature 533 FR-007), or no new field at all, the open project's Jira connection site (533 FR-006)
  serving as the base? Working assumption: one new app-wide field.]
- **FR-011**: Saving the Jira base URL MUST accept an `https` web address with a host and an
  optional path, store it with any trailing `/` removed, and MUST refuse, marking the field with
  the reason and storing nothing, a value that is not `https`, names no host, or has a user name, a
  query or a fragment. Clearing the field MUST unset it.
- **FR-012**: A change to the Jira base URL, or to an integration switch FR-001 depends on, made in
  one window MUST show on the Overview's cards in
  every open window without a restart.
- **FR-013**: When the browser cannot be opened, the user MUST see the notification of feature 031
  FR-015, naming the address and the reason, and nothing else MUST change.
- **FR-014**: Showing issue links MUST NOT make any network request, and activating one MUST make
  none from the app other than asking the operating system to open the address; addresses MUST NOT
  be logged (feature 813 FR-020).
- **FR-015**: The issue link MUST be built from the shared UI components with its colours, underline
  and focus ring from the theme's roles (Principle VIII), MUST meet the contrast of feature 1120
  FR-003 in every card state and both themes, and MUST be drawn at the branch line's size (feature
  1120 FR-004); the component showcase MUST show a card with an issue link, collapsed and expanded.
- **FR-016**: The rules that decide whether a card has an issue link, its text and its address, the
  validation of the Jira base URL, and how a press, a rest and a focus move on the link are resolved
  against the card MUST live in render-free code and be unit-tested without the GUI (Principle I).
- **FR-017**: The user guide MUST describe the issue link in the Overview section (what it shows,
  when it shows, pointer and keyboard use) and the Jira base URL field in the Settings section, and
  the recorded layout snapshot MUST include a card with an issue link (Principle VII).

### Key Entities

- **Issue link**: per card, derived per render: the label ("Issue #N" / "Issue KEY") and the
  address; absent when FR-001 says so. Not stored.
- **Jira base URL**: an optional user setting, an `https` address with no trailing `/`, kept in
  `settings.json` with the app's other settings.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: From the Overview, a user reaches a worktree's issue page in one click on its card, or
  in at most two key presses from its focused card (Tab, Enter).
- **SC-002**: For every ticket form the app names worktrees with (GitHub number, Jira key in either
  letter case) the link opens the exact issue address, with no doubled or missing `/`.
- **SC-003**: Across clicks, key presses and secondary clicks on issue links, collapsed and
  expanded, no card is activated by a press on its link, and every press outside a link behaves as
  before this change.
- **SC-004**: At card widths of 280, 380 and 480 px, the issue link is shown whole on every card
  that has one, in both forms and both themes.
- **SC-005**: A change to the Jira base URL or to an integration switch shows on every open window's
  cards within 1 s.
- **SC-006**: A user can set the Jira base URL in Settings, and every invalid value FR-011 lists
  (100% of its cases) is refused with a reason and leaves the stored value unchanged.
- **SC-007**: The user guide's Overview and Settings sections describe the issue link and the Jira
  base URL field, checked by the docs review and the recorded layout snapshot.

## Out of scope

- Fetching anything for the link (issue state, existence check, Jira titles on cards).
- Links on any surface but the Overview's cards (sidebar rows, the issue tag, terminal output).
- Issue trackers other than GitHub and Jira, and GitHub Enterprise hosts.
- A card menu entry for opening the issue.
- Changing how a worktree's ticket is read from its name, or the issue-title lookup of feature 1120.

## Assumptions

- The ticket is read from the worktree's directory name, as the card's issue-title lookup reads it;
  the branch name is not consulted separately (worktrees are named after their branch, feature 034).
- A GitHub ticket links to the project's GitHub repository; a ticket that names another repository
  is not a form the app creates and is not supported.
- `https://github.com/<owner>/<repo>/issues/<N>` reaches a pull request as well as an issue (GitHub
  redirects), so the link needs no check of which N is.
- A Jira base URL may carry a path (a Jira server under a context path); feature 533's Jira Cloud
  restriction concerns requests, and a browse link makes none.
- The link opens in the system's default browser through the opener feature 031 already uses.
- Inserting the link into the Tab order replaces, for Tab and Shift+Tab only, the "focus order
  unchanged" clause of feature 1121 FR-011 and Story 4 scenario 4; the arrow keys keep it.
- A primary press that starts on an issue link starts no ripple: this replaces feature 971 FR-001
  for the link's area only (FR-005). Closing a card menu opened from a link hands focus back to the
  link: this replaces feature 972 FR-011's hand-back for that case only (FR-009).
