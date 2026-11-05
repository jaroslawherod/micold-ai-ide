# Feature Specification: Highlighted user messages in the structured view

**Feature Branch**: `feat/1663_structured-view-messages-sent-by-the-user-are-not`

**Created**: 2026-11-05

**Status**: Closed 2026-11-05 — shipped in PRs #1695, #1701

**Input**: Issue #1663 — "Structured view: messages sent by the user are not visually distinct from the agent's output". In a long transcript the text the user sent blends into assistant text, thinking and tool cards; a user turn should stand apart through a tinted surface and a left accent, using existing theme tokens, in light and dark.

## Terms

- **User turn**: one transcript entry of the user speaking: a prompt typed in the composer, a queued message once it is delivered to the agent, or a slash command invocation (shown as its command card).
- **Turn label**: the small caption above a user turn that names who spoke.

## Clarifications

### Session 2026-11-05

- Q: Is any requirement ambiguous enough to change the design or tests? → A: No critical ambiguity found: scope, forms of user turn, selection/copy, theming, edge cases and exclusions are all stated; token choice is deferred to the plan by the first Assumption. _(agent-resolved: specs/1663-user-message-highlight/spec.md#Assumptions)_

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Find where each turn began (Priority: P1)

A user scrolls a long structured-view transcript. Each message they sent is drawn on a tinted surface with a left accent bar, so every turn start is found at a glance without reading the text.

**Why this priority**: This is the whole problem; without it nothing else in the issue is met.

**Independent Test**: Open a session whose transcript holds a prompt followed by assistant text, thinking and tool cards. The prompt is drawn on its own tinted surface with an accent edge; none of the other entries are.

**Acceptance Scenarios**:

1. **Given** a transcript with a user prompt and assistant output, **When** it is shown, **Then** the user prompt sits on a surface whose colour differs from the transcript background and from assistant text, with a left accent edge, and assistant text, thinking and tool cards carry neither.
2. **Given** a long transcript scrolled so that several turns are on screen, **When** the user looks at it, **Then** every user turn is told apart from the entries around it without reading them.
3. **Given** the app is in light theme and then in dark theme, **When** the same transcript is shown, **Then** the highlight is present and legible in both.

---

### User Story 2 - Every kind of user turn is highlighted alike (Priority: P1)

Prompts typed in the composer, queued messages once delivered, and slash commands all get the same highlight, so the rule "tinted with an accent edge is something I said" holds without exception.

**Why this priority**: A highlight that covers only some of the user's turns would mislead more than none.

**Independent Test**: Send a typed prompt, queue a message while the agent works and let it be delivered, and run a slash command. All three show the same treatment.

**Acceptance Scenarios**:

1. **Given** a prompt typed in the composer and sent, **When** it appears in the transcript, **Then** it has the highlight.
2. **Given** a message queued during a running turn, **When** it is delivered and appears as a user turn, **Then** it has the highlight.
3. **Given** a slash command run from the composer, **When** its command card appears, **Then** the card sits inside the same highlight and keeps its fold/unfold behaviour.
4. **Given** a user turn with attachments, **When** it is shown, **Then** the attachments line is inside the highlight.

---

### User Story 3 - Text stays readable and selectable (Priority: P2)

The highlighted text is as readable as other transcript text, and the user can still select and copy it as before (#1550).

**Why this priority**: The highlight must not cost the legibility or the copy behaviour that already exist.

**Independent Test**: Drag-select across a user turn and the assistant text after it, and copy; the copied text is the same as before this feature. Run the contrast gate over the new surface.

**Acceptance Scenarios**:

1. **Given** a highlighted user turn, **When** the user drags across it, **Then** the selection covers the text and copies exactly the text of the turn, with no label or decoration text added.
2. **Given** a selection spanning an assistant entry and a user turn, **When** it is copied, **Then** the result is the same as it was before this feature.
3. **Given** the highlight in light and dark theme, **When** text contrast is measured, **Then** it meets the same bar as the other transcript surfaces.

### Edge Cases

- Empty or whitespace-only user text: the turn still shows its highlight with its label, at least the label's height plus padding; nothing crashes.
- Very long single-line or multi-paragraph user text: it wraps inside the highlight and never overflows the pane.
- Narrow pane: the accent edge and padding stay; the text wraps.
- Several user turns in a row (queued messages delivered back to back): each is its own highlighted block with visible separation.
- Sub-agent transcripts: a user turn inside a sub-agent scope (the prompt the parent sent it) is highlighted like a main-scope one.
- Streaming and live updates (Principle II, multiple sessions): a user turn appearing while another session streams is highlighted the moment it is shown; no per-session state is shared.
- Theme change at runtime and an accent colour set by the user: the highlight follows the active theme and accent with no restart.
- Platform differences (Principle VI): the highlight uses theme tokens only, so Linux, macOS and Windows draw it alike; it does not depend on a platform font or shadow.
- A new entry arriving while a selection spans a user turn, or while a command card is unfolded: the selection and fold state survive.
- Selected text inside the highlight: the selection colour stays visible on the tinted surface.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: Each user turn MUST be drawn on a surface tinted differently from the transcript background and from every assistant-side entry (assistant text, thinking, tool cards, command output).
- **FR-001a**: In light and dark theme the highlight's tint MUST differ from the transcript background (the two tokens are not equal).
- **FR-002**: Each user turn MUST show a left accent edge in addition to the tint, so it is told apart even where tint differences are slight.
- **FR-003**: The highlight MUST apply to typed prompts, delivered queued messages and slash command turns, in main and sub-agent scopes.
- **FR-004**: The highlight MUST contain the whole user turn: its label, its text or command card, and its attachments line.
- **FR-005**: Assistant text, thinking, tool cards and command output MUST NOT receive the user-turn highlight.
- **FR-006**: The highlight colours MUST come from existing theme tokens, with no new fixed colour values, and MUST follow light/dark theme and the user's accent without a restart.
- **FR-007**: Text on the highlight MUST meet the same contrast bar as the other transcript surfaces, in light and dark.
- **FR-008**: Selecting and copying text from a user turn MUST work as before (#1550): the same selectable units, and copied text without added label or decoration text.
- **FR-009**: A slash command card inside the highlight MUST keep its existing fold/unfold behaviour and keyboard access.
- **FR-010**: The highlight MUST NOT change what entries are shown, their order, or the transcript's scroll behaviour; only how a user turn is drawn changes.
- **FR-011**: A visual pass of the result in light and dark MUST be recorded, because colour and elevation are not covered by the geometry gates.

### Key Entities

- **User turn**: a transcript entry spoken by the user, in the three forms of the Terms. Carries text or a command, optional attachments and a label.
- **Highlight**: the tinted surface plus left accent that wraps one user turn.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: In a recorded visual pass of a long transcript in light and dark, every user turn is identified by a reviewer on a first look, and no assistant entry is mistaken for one.
- **SC-002**: The contrast of body text on the highlight surface is at least that of the other transcript surfaces, as measured by the existing contrast gate, in both themes.
- **SC-003**: Copying a selection over a user turn yields byte-identical text to the behaviour before this feature, over the existing selection tests.
- **SC-004**: All three kinds of user turn show the highlight in 100% of the cases covered by the tests.
- **SC-005**: Existing geometry and layout gates pass unchanged apart from the extra padding the highlight adds to a user turn.

## Assumptions

- "Existing theme tokens" means the design-system surface and accent roles already used by transcript cards; if none gives enough separation in both themes, the plan picks the nearest role, not a new colour.
- The turn label stays; the highlight wraps it rather than replacing it.
- Right-side alignment (a chat-bubble layout) is out of scope: the issue offers "tint and left accent *or* alignment", and the tint plus accent is chosen as it keeps the full-width reading column.

## Out of Scope

- Restyling assistant text, thinking or tool cards.
- Changing the composer, the queued-message list before delivery, or the terminal view.
- A user setting to turn the highlight off or to pick its colour.
- Changing how selection works (#1550).
