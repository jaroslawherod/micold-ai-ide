# Feature Specification: Agent-created worktrees carry the issue reference

**Feature Branch**: `feat/532_agent-created-worktrees-should-carry-the-issue`

**Created**: 2026-10-12

**Status**: Closed 2026-10-13 — shipped in PRs #781, #792

**Input**: GitHub issue #532, "Agent-created worktrees should carry the issue reference".

## Clarifications

### Session 2026-10-12

- Q: Should a result for a literal `branch` with no issue tag carry a one-line no-ticket hint, or stay unchanged? → A: Stay unchanged (guidance only). _(agent-resolved: specs/532-worktree-issue-reference/spec.md#Requirements, FR-004, SC-003 and Assumptions: existing call shapes produce the same result, this feature is guidance, docs and tests)_

## Background

`create_worktree` on the agent tool server already accepts `type`, `ticket`, `name` and
`github_issue`, and derives the branch (`${type}/${ticket}_${name}`) and directory
(`${type}-${ticket}_${name}`) through the naming code the create-worktree dialog uses (feature
550). The description mentions that form only as the second of two alternatives and never says to
include the issue reference when the worktree is for an issue; the `branch` and `ticket` schema
texts do not steer callers either. So sessions pass a plain literal `branch`, and the worktree
gets no `_` boundary, no issue tag in the sidebar and no hit in the issue filter. On 2026-10-02 four worktrees
(#518, #481, #486, #485) were created that way.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - A session names a worktree after its issue (Priority: P1)

A session asked to "create a worktree for issue #N" reads the tool server's guidance, passes the
ticket as its own input, and the worktree comes out named exactly as the dialog would name it.

**Why this priority**: it is the whole defect; the sidebar tag and the issue filter depend on it.

**Independent Test**: list the tool catalog and read `create_worktree`'s description and
`initialize`'s instructions; call `create_worktree` with `type`, `ticket` and `name` and read back
the row's tags.

**Acceptance Scenarios**:

1. **Given** the tool catalog, **When** a caller reads `create_worktree`'s description, **Then** it
   states the naming convention `${type}/${ticket}_${name}` and tells the caller to include the
   issue reference when the worktree is for an issue or ticket, passed as `ticket` (with `type`
   and `name`) or `github_issue` rather than inside a literal `branch`.
2. **Given** a call with `type: feat`, `ticket: 123`, `name: add thing`, **When** the worktree is
   created, **Then** its branch and directory equal what the dialog derives for the same inputs
   and its sidebar row shows the `#123` issue tag.
3. **Given** such a worktree, **When** the user filters the sidebar by issue 123, **Then** the
   worktree is found.
4. **Given** a call with `github_issue: 123` only, **When** it succeeds, **Then** the worktree
   carries the `#123` tag as today.
5. **Given** a new session, **When** the server answers `initialize`, **Then** its instructions
   state the rule to include the issue reference in the worktree name.
6. **Given** the tool catalog, **When** a caller reads the `ticket` input's schema description,
   **Then** it says the ticket is what makes the sidebar issue tag and the issue filter work.

---

### User Story 2 - Existing calls keep working (Priority: P2)

Callers that pass only `branch`, or no ticket at all, are unchanged.

**Why this priority**: the new guidance must not break current sessions or scripts.

**Independent Test**: call `create_worktree` with only `branch`, and with `type` and `name` but no
ticket; compare with the results before this change.

**Acceptance Scenarios**:

1. **Given** a call that passes only `branch`, **When** it runs, **Then** the worktree is created
   as before, with no issue tag unless the branch itself carries a `_` boundary.
2. **Given** a call with `type` and `name` and no ticket, **When** it runs, **Then** the names have
   no ticket part and the row shows no issue tag.

---

### User Story 3 - The user guide describes the convention (Priority: P3)

A reader of the agent-tools page learns the convention and the `ticket` input, and why the `_`
matters.

**Why this priority**: documentation of a behaviour that already exists in the tool.

**Independent Test**: read `docs/user-guide/agent-tools.md`.

**Acceptance Scenarios**:

1. **Given** the agent-tools page, **When** a reader looks up `create_worktree`, **Then** it states
   the `${type}/${ticket}_${name}` convention, tells agents to include the issue reference, and
   describes the `ticket` input and that a bare `branch` gets no issue tag.
2. **Given** the worktrees-and-sessions naming section, **When** a reader looks for agent-created
   worktrees, **Then** it says they follow the same convention and gain the same issue tag.

### Edge Cases

- A ticket that is a Jira-style key (`ABC-123`) or a `#`-prefixed number: slugified as the dialog
  does, read back as the matching issue tag.
- A ticket that slugifies to empty: treated as no ticket, as the dialog does.
- `ticket` together with a literal `branch`: refused as alternatives, as today.
- Two sessions creating a worktree for the same issue concurrently: the derived-name collision rule
  of feature 550 applies; the second is refused, not overwritten.
- A literal `branch` that already follows `type/ticket_name`: the issue tag is read from it as
  today; nothing is rewritten.
- `ticket` without `type` or without `name`: refused as today, naming the missing input.
- `github_issue` for a closed or missing issue, or with `gh` unavailable: fails as today.
- Platform differences: names are derived by the shared naming code, identical on every platform.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: The `create_worktree` tool description MUST state the naming convention
  `${type}/${ticket}_${name}` (branch) and `${type}-${ticket}_${name}` (directory) as the way to
  name a worktree for an issue, and MUST tell the caller to include the issue reference whenever the worktree is for an issue or ticket.
- **FR-002**: The description MUST tell the caller to pass the reference as `ticket` (with `type` and
  `name`) or as `github_issue`, rather than assembling the `_` form into a literal `branch`.
- **FR-003**: The branch and directory the tool derives from `ticket`, `type` and `name` MUST equal
  those the create-worktree dialog derives for the same inputs.
- **FR-004**: A call that passes only `branch` MUST behave exactly as before.
- **FR-005**: A worktree created without a ticket MUST show no issue tag, as before.
- **FR-006**: The `ticket` input schema description MUST say that it is what makes the issue tag and
  the issue filter work.
- **FR-007**: The server's `initialize` instructions MUST state the rule, so it reaches every
  session at connect time.
- **FR-008**: The user guide's agent-tools page MUST describe the convention, the `ticket` input and
  the consequence of omitting it; the worktrees-and-sessions naming section MUST mention that
  agent-created worktrees follow the same convention.
- **FR-009**: When a literal `branch` is given and the resulting worktree has no issue tag, the
  result MUST stay unchanged: no hint, no warning, no extra field. The rule reaches callers through
  the description, the `ticket` schema text, the `initialize` instructions and the user guide only.

### Key Entities

- **Worktree name pair**: the branch and directory derived from type, ticket and name; the `_`
  separates ticket from description so the issue tag can be read back.
- **Issue tag**: the `#N` or `ABC-123` shown on a worktree's sidebar row and matched by the issue
  filter, read from the directory name.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: 100% of worktrees a session creates through the `ticket` input show the matching
  issue tag and are found by the issue filter.
- **SC-002**: For identical inputs, the names produced via the tool and via the dialog are equal in
  every tested case (type x ticket form x name).
- **SC-003**: Every existing `create_worktree` call shape (literal branch, mode, remote, derived
  without ticket) produces the same result as before this change.
- **SC-004**: The tool description and the `initialize` instructions each contain the convention
  string `${type}/${ticket}_${name}` or the instruction to include the issue reference, checked by
  a test; the user guide states both.

## Assumptions

- Input shape already exists (feature 550); this feature is guidance, docs and tests, not a new
  input or a new naming algorithm.
- Out of scope: renaming or retagging worktrees that already exist (the four from 2026-10-02), the
  dialog itself, and rejecting plain branches.
- The user-facing rule applies to GitHub/GitLab numbers and Jira-style keys alike.
