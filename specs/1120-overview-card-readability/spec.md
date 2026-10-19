# Feature Specification: Overview cards, readable layout and content

**Feature Branch**: `claude/project-thread-7041o6`

**Created**: 2026-10-19

**Status**: Draft

**Input**: User description: "Implement GitHub issue #1120: Overview cards: readable layout and
content. Redo what a card shows and how it reads. Tinted header row: state icon and word left,
location type right; neutral body instead of the whole-card state fill. Type scale: title 17 px,
body 14 px, labels 12-13 px minimum. Text uses the on-*-container roles, at least 4.5:1 in every
state in both themes. Card height becomes a minimum; a grid row takes its tallest card. Title =
issue title, else branch, else display name (dir name to the tooltip). Branch shown as
`type/number`. One line "1 session · 1 running" with correct plurals. Rows with nothing to say are
removed: "No task progress", "0 open · 0 merged". Activity line: strip `<agent-message from="…">`
markup, show a slash command as a sentence ("Running autopilot on #1117"), show nothing rather than
markup. Acceptance: five states render with the header; palette tests assert contrast; core unit
tests for title, short branch, counts and the activity sanitiser; no overlap at 280/380/480 px; user
guide and layout snapshot updated." Labels: enhancement, flow:feature. Builds on features 813
(`specs/813-worktree-overview`), 970, 971, 972 and 1024 (`specs/1024-overview-larger-cards`).

## Terms

- **Overview**, **card**, **location**, **location type**, **card state**, **pull request
  counts**, **task summary**, **progress**: as defined by feature 813
  (`specs/813-worktree-overview/spec.md#Terms`). The five card states are Needs action, Needs
  attention, Working, Done and Idle.
- **Header row**: the band across the top of a card that carries the card state (icon and word) at
  its start and the location type (icon and word) at its end.
- **Body**: everything on the card below the header row.
- **Title**: the card's most prominent line, the first line of the body.
- **Short branch**: the branch as a card shows it, `type/number` for a branch that follows the
  project's `<type>/<number>_<slug>` or `<type>-<number>_<slug>` naming (for example
  `fix/1115_container-daemon-fails-to-start` reads `fix/1115`).
- **Sessions line**: the one line saying how many un-archived sessions the location has and how many
  of them run.
- **Activity line**: the line saying what the location's agent is doing now, from its hooks
  (feature 813 FR-018).
- **Markup**: text in a prompt that is meant for the agent, not for a reader: tags in angle brackets
  such as `<agent-message from="…">…</agent-message>`.
- **Slash command**: a prompt that starts with `/` followed by a command name, such as
  `/speckit-autopilot #1117`.
- **Card width**: the width the grid gives one card (feature 1024); the narrowest is 280 px.

## Clarifications

### Session 2026-10-19

- Q: Is FR-012's one generic sentence right for every slash command, or should built-in commands
  (`/clear`, `/compact`, `/model`) read differently or be hidden? → A: One generic sentence for
  every slash command, built-ins included; none is hidden or reworded. _(orchestrator default, the
  recommended option; confirmed by the user 2026-10-19)_

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Read a card's state and type at a glance (Priority: P1)

A developer opens the Overview to see which worktrees need them. Today the whole card is filled
with its state's colour, which makes the text on it hard to read, and the state word competes with
the name in one crowded line. With this change each card has a tinted header row in its state's
colour, with the state icon and word at the start and the location type at the end, and a neutral
body below it where the rest of the card reads like ordinary text.

**Why this priority**: The state is the reason the Overview exists; it must be the first thing read,
and the rest must stay legible in every state.

**Independent Test**: Open the component showcase (or an Overview with one location in each card
state) in the light and in the dark theme, and check that each of the five cards has a header row
tinted for its state with the state icon and word on the left and the type icon and word on the
right, over a body of the same neutral colour on every card.

**Acceptance Scenarios**:

1. **Given** a card in any of the five card states, **When** it is shown, **Then** it has a header
   row in that state's colour carrying the state icon and word at its start and the location type's
   icon and word at its end, and the body below is drawn in one neutral colour, the same for every
   state.
2. **Given** a card in any state and either theme, **When** it is shown, **Then** every piece of text
   on it, in the header row and in the body, has a contrast of at least 4.5:1 against what it sits
   on.
3. **Given** an Idle card beside cards of other states, **When** the user looks at the row, **Then**
   the Idle card is still visibly de-emphasised (feature 813 FR-006), and its text still meets
   4.5:1.
4. **Given** a card, **When** the user hovers the state word, **Then** the reason for the state shows
   in a tooltip, as today; **When** the user hovers the type, **Then** its word shows in a tooltip.
5. **Given** a card targeted by its context menu (feature 972), focused (feature 813 FR-014) or
   pressed (feature 971), **When** it is shown, **Then** the outline, focus ring and ripple still
   cover the whole card, header included.

---

### User Story 2 - The card says what the worktree is for (Priority: P1)

The developer wants each card to lead with what the worktree is about. Today the title is the
worktree's folder name (such as `feat-813_worktree-overview`), the branch repeats the same words in
full, the session footer reads "feature · 1 sessions, 1 running", and rows like "No task progress"
and "0 open · 0 merged" fill the card with nothing. With this change the title is the issue's title
when the app knows it, the branch is shortened to `type/number`, the sessions read "1 session · 1
running", and a row that has nothing to say is not shown.

**Why this priority**: It is half of the issue: a card that reads well but says the wrong things is
still not useful.

**Independent Test**: Open the Overview with a worktree whose issue title is known, one whose branch
has no issue number, the project root, one with no sessions, one with no task files and one with no
pull requests, and check each card's title, branch, sessions line and which rows appear.

**Acceptance Scenarios**:

1. **Given** a worktree whose linked issue's title the app knows, **When** its card is shown,
   **Then** the title is the issue's title, and hovering the title shows the location's display
   name (its folder name).
2. **Given** a worktree with a branch but no known issue title, **When** its card is shown, **Then**
   the title is the branch; **given** a location with neither (an orphan folder, the project root
   without a branch), **then** the title is the display name.
3. **Given** a card with a branch named `fix/1115_container-daemon-fails-to-start` (or
   `feat-813_worktree-overview`), **When** it is shown, **Then** the branch line reads `fix/1115`
   (or `feat/813`), with the full branch name in the line's tooltip.
4. **Given** a card whose branch has no type and number (for example `main` or
   `claude/project-thread-7041o6`), **When** it is shown, **Then** the branch line shows the branch
   as it is, ellipsized if it does not fit, with the full name in the tooltip.
5. **Given** a location with 1 session of which 1 runs, **When** its card is shown, **Then** the
   sessions line reads "1 session · 1 running"; with 3 sessions and 2 running it reads "3 sessions ·
   2 running"; with 2 sessions and none running it reads "2 sessions"; with no sessions it reads "No
   sessions".
6. **Given** a worktree with no phase and no task ratio, **When** its card is shown, **Then** no
   progress row is shown (no "No task progress" and no empty bar). The "No task progress" empty
   state of feature 813 FR-017 is removed on purpose (FR-009).
7. **Given** a card with a branch whose open and merged pull request counts are both 0, **When** it
   is shown, **Then** no pull request row is shown; with 1 open and 0 merged it reads "1 open"; with
   0 open and 2 merged it reads "2 merged"; with both non-zero it reads "1 open · 2 merged"; when
   the counts cannot be read the unknown mark and its tooltip stay (feature 813 FR-004).
8. **Given** a card whose title is the branch, **When** it is shown, **Then** the branch is not
   repeated on a second line.

---

### User Story 3 - The activity line reads as a sentence, never as markup (Priority: P1)

When an agent is driven by another agent or by a slash command, the latest prompt is markup such as
`<agent-message from="orchestrator">Continue M2</agent-message>` or a command such as
`/speckit-autopilot #1117`, and today the card shows it raw. With this change the card strips the
markup and shows the words, turns a slash command into a sentence ("Running autopilot on #1117"),
and shows no activity line at all rather than markup it cannot turn into words.

**Why this priority**: Raw markup on a card is the most visible readability defect the issue names.

**Independent Test**: Feed a location's hooks a prompt wrapped in `<agent-message from="…">`, a
slash command with and without arguments, and a prompt that is only markup, and check the activity
line each time.

**Acceptance Scenarios**:

1. **Given** a latest prompt `<agent-message from="orchestrator">Continue M2</agent-message>`, **When**
   the card is shown, **Then** the activity line reads "Continue M2".
2. **Given** a latest prompt `/speckit-autopilot #1117`, **When** the card is shown, **Then** the
   activity line reads "Running autopilot on #1117".
3. **Given** a latest prompt that is a slash command without arguments (for example `/clear`),
   **When** the card is shown, **Then** the activity line reads a sentence naming the command
   (for example "Running clear").
4. **Given** a latest prompt that holds nothing but markup, or that still holds angle-bracket tags
   after sanitising, **When** the card is shown, **Then** no activity line is shown.
5. **Given** an agent todo list with an in-progress item (feature 813 FR-018), **When** the card is
   shown, **Then** the activity line shows that item and its done / total as today; the item text
   is sanitised the same way.
6. **Given** a prompt that spans several lines, **When** the card is shown, **Then** the activity
   line shows it on one line, ellipsized, with no markup.

---

### User Story 4 - Cards grow to fit, never overlap (Priority: P2)

With a larger type scale and rows that come and go, cards no longer all hold the same content. A
card is never shorter than today's card, grows when its content needs more room, and every card in
a grid row takes the height of the row's tallest card, so rows stay even and nothing is clipped.

**Why this priority**: The larger type must not cause clipping or overlap, but it follows from
Stories 1 to 3.

**Independent Test**: Show cards with every combination of optional rows at card widths of 280,
380 and 480 px and check that no text overlaps another part of its card or another card, and that
the cards of a row share one height.

**Acceptance Scenarios**:

1. **Given** a card with every optional row (branch, activity, progress with bar, pull requests),
   **When** it is shown at a card width of 280, 380 or 480 px, **Then** no text overlaps another
   part of the card or another card, and text that does not fit ends in an ellipsis with the full
   text in a tooltip.
2. **Given** a grid row in which one card has more rows than the others, **When** it is shown,
   **Then** every card in that row is as tall as the tallest card in it.
3. **Given** a card with few rows, **When** it is shown alone in its grid row, **Then** it is no
   shorter than the card's minimum height.

---

### Edge Cases

- **Title equals the display name**: when the title falls back to the display name, the title's
  tooltip still shows the display name; no other line repeats it.
- **Issue title arrives while the Overview is open**: the title changes from the branch to the
  issue's title within feature 813 FR-008's update time; the card may grow or shrink and its row
  re-takes the tallest card's height.
- **Very long issue title**: ellipsized on one line at every card width, full title in the tooltip.
- **Branch with a type but no number** (`feat/make-branch-selector`): shown as it is, ellipsized;
  only `type/number` with both parts is shortened.
- **Branch with a number but a type outside the conventional list** (`scratch/023-docs-gate`): shown
  as it is.
- **Project root**: the header shows the *project root* type; the title is the display name
  (*Default*) unless the root's branch is known, then the branch.
- **Orphan / invalid location**: no branch line, no pull request row; header type reads orphan /
  invalid.
- **Session counts over 99**: shown as the number, no cap; "running" never exceeds sessions.
- **Nested or unbalanced markup** (`<agent-message from="a"><agent-message from="b">Hi`): every tag
  is removed; what remains, when it has words, is shown.
- **Slash command with markup in its arguments**: the markup is removed before the sentence is
  formed.
- **Text that merely contains `<` or `>`** (for example `a < b`): not markup; shown as written.
- **Several sessions in one location** (Principle II): the activity line comes from the location's
  main session only, as feature 813 FR-018 and the main-session rule define (the session marked
  main, else the most recently active). When that session's text sanitises to nothing, the line is
  hidden; it does not fall back to another session's prompt. One location's prompt never appears on
  another location's card.
- **Both themes and every state**: the header tint, the neutral body and all text meet 4.5:1 in
  light and dark.
- **Platforms and display scale** (Principle VI): no overlap and equal row heights hold on Linux,
  macOS and Windows with each platform's default UI font, at display scale 1.0 and 2.0.
- **Pane narrower than one minimum-width card** (under 280 px): one card per row as feature 1024
  defines; text ellipsizes, nothing overlaps.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: Each card MUST have a header row across its full width, filled with its card state's
  container colour (Needs action: error, Needs attention: warning, Working: primary, Done: success,
  Idle: the de-emphasised surface variant), showing the state icon and word at its start and the location
  type's icon and word at its end.
- **FR-002**: The body below the header row MUST use one neutral surface colour, the same in every
  card state; the whole-card state fill is removed.
- **FR-003**: Text and icons in the header row MUST use the matching on-container role of the state's
  colour (Idle, which has no container pair, uses the on-surface-variant role on its surface
  variant); body text MUST use the on-surface roles. Every text colour on the card MUST have a contrast
  of at least 4.5:1 against its background in every card state in both the light and the dark theme.
- **FR-004**: The card's title MUST be drawn at 17 px, body lines at 14 px, and no text on the card
  (labels, captions, counts) smaller than 12 px.
- **FR-005**: Idle cards MUST stay visibly de-emphasised compared with every other state (feature
  813 FR-006), by the header row's colour, without lowering any text below FR-003's contrast.
- **FR-006**: The card's title MUST be the linked issue's title when the app knows it, else the
  branch, else the display name; its tooltip MUST show the location's display name.
- **FR-007**: A card with a branch MUST show the short branch on its own line, unless the title
  already is the branch: `type/number` when the branch is `<type>/<number>`, or `<type>/<number>` or
  `<type>-<number>` followed by `_` or `-` and a slug (so `fix/1115-foo` and `fix/1115_foo` both
  read `fix/1115`), with a conventional branch type (the type list feature
  813 derives the location type from) and a decimal number, else the branch unchanged; the line's tooltip MUST show the full
  branch name.
- **FR-008**: The card MUST show one sessions line: "No sessions" for none; "*n* session" or "*n*
  sessions" (singular for exactly 1); followed by " · *k* running" when *k* > 0 sessions run.
- **FR-009**: The card MUST NOT show a progress row when the location has neither a phase nor a task
  ratio; it MUST NOT draw a progress bar without a ratio. Otherwise the progress row is as feature 813
  FR-016 defines. This replaces feature 813 FR-017's explicit empty state.
- **FR-010**: The card MUST NOT show a pull request row when both counts are 0; when one count is 0
  it MUST show only the other ("1 open", "2 merged"); the unknown mark with its tooltip stays.
- **FR-011**: The activity line MUST remove every angle-bracket tag (such as `<agent-message
  from="…">` and its closing tag) from the prompt or todo item and show the remaining words on one
  line.
- **FR-012**: An activity text that is a slash command MUST be shown as a sentence: "Running
  *name* on *arguments*", or "Running *name*" without arguments, where *name* is the command name
  with the spec-kit prefix (`speckit-`) removed and hyphens read as spaces (for example
  `/speckit-autopilot #1117` reads "Running autopilot on #1117"). The same sentence applies to every
  slash command, Claude's built-in commands (`/clear`, `/compact`, `/model`, …) included: none is
  hidden or worded differently.
- **FR-013**: When nothing readable remains after FR-011 and FR-012 (empty, only whitespace, or still
  holding a tag), the card MUST show no activity line, and MUST NOT show markup.
- **FR-014**: The card's height MUST be a minimum (no shorter than feature 813's card), not a fixed
  height: a card grows to fit its rows, and every card in a grid row MUST be as tall as the row's
  tallest card. This replaces feature 1024 FR-006 (one common height for every card in the grid).
- **FR-015**: Text on the card MUST NOT overlap another part of the card or another card at card
  widths of 280, 380 and 480 px, nor at any other width the grid produces; text that does not fit
  MUST end in an ellipsis with the full text in a tooltip.
- **FR-016**: Card states, card order, activation, keyboard focus, the context menu (feature 972),
  the ripple (feature 971) and the Overview entry's attention badge (feature 813) MUST be unchanged; the target outline, focus ring
  and ripple MUST cover the whole card including the header row.
- **FR-017**: The card MUST be built from the shared UI components with every colour from the theme's
  roles (Principle VIII), and the component showcase MUST show the card in each of the five states
  with its header row.
- **FR-018**: The rules for the title, the short branch, the sessions line, the pull request row and
  the activity sanitiser MUST live in the render-free core so they are unit-tested without the GUI
  (Principle I).
- **FR-019**: The user guide's Overview section MUST describe the new card layout (header row,
  title rule, short branch, sessions line, which rows are hidden when empty, and how the activity
  line reads prompts and slash commands), and the recorded layout snapshot MUST
  gain an Overview state showing the card in the new layout (Principle VII).
- **FR-020**: Prompts, todo items and issue titles MUST stay on this machine (feature 813 FR-020);
  sanitising MUST NOT log them.

### Key Entities

- **Card text**: what a card says, derived from the card model and not stored: title, title tooltip,
  short branch and its tooltip (or none), sessions line, progress row (or none), pull request row (or
  none), activity line (or none).
- **Header palette**: per card state, the header fill and the colour of its text and icons; plus the
  body's neutral fill and text colours.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: In both themes and all five card states, every text colour on the card has a contrast
  ratio of at least 4.5:1 against its background (10 state-theme pairs, header and body).
- **SC-002**: At card widths of 280, 380 and 480 px, a card with every optional row shows no text
  overlapping another element, and every grid row's cards share one height.
- **SC-003**: For the examples in Story 2 and Story 3, the card shows exactly the expected title,
  branch line, sessions line and activity line, and shows no "No task progress", no "0 open · 0
  merged" and no angle-bracket markup.
- **SC-004**: The card title is at least 17 px and no card text is under 12 px.
- **SC-005**: The five card states are each shown in the component showcase with a header row whose
  state word and type word are both visible.

## Out of scope

- New card states, a new order, or new data sources for the title or activity.
- A user setting for the type scale or for hiding rows.
- An LLM summary of the activity.
- Changes to the grid's column rules (feature 1024) other than row height.
- Views other than the Overview.

## Assumptions

- The neutral body is a surface container role of the theme; the header uses the state's
  `*_container` / `on_*_container` pair, as the whole-card fill does today.
- 17 px is not on the Material type scale (16 and 22 are); a card-scoped type role is added, as the
  sidebar's scoped roles are, rather than reusing a scale step.
- "Labels 12-13 px minimum" means no card text below 12 px; the state and type words in the header
  use a 12 or 13 px label role.
- The card's minimum height is today's fixed height (172 px), so a sparse card does not shrink.
- "Running" is not pluralised and is omitted when no session runs; "No sessions" is shown rather
  than hiding the line, because it tells the user activation will offer to start one.
- Only tags in angle brackets whose name starts with a letter count as markup, so `a < b` survives.
- The spec-kit prefix `speckit-` is the only command prefix removed when naming a slash command.
