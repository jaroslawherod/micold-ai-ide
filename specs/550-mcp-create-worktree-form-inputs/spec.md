# Feature Specification: MCP create_worktree accepts the New worktree form's inputs

**Feature Branch**: `feat/550_mcp-create-worktree-should-support-the-same-inputs`

**Created**: 2026-10-08

**Status**: Draft

**Input**: GitHub issue #550 — "MCP create_worktree should support the same inputs as the New worktree form (type, ticket, GitHub issue)". Labels: enhancement.

## Background

The `create_worktree` tool of the daemon's MCP server (feature 034-daemon-mcp-server) takes a literal branch name, an optional directory name, a mode (`new_branch`, `existing_local`, `track_remote`) and a remote. The app's New worktree form instead takes a Conventional-Commits type, an optional ticket and a name, derives the branch (`${type}/${ticket}_${name}`) and directory (`${type}-${ticket}_${name}`) from them, and can fill ticket, name and type from an open GitHub issue (feature 034-github-issue-worktree). An agent that wants a worktree like the ones the app makes must hand-build the branch name and get the convention right alone; nothing validates it, so the sidebar's type and issue tags can come out wrong or missing.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Ask for a worktree by type, ticket and name (Priority: P1)

An agent asks for "type `fix`, ticket `#123`, name `login crash`" and gets the same branch, directory and sidebar tags the New worktree form produces for those inputs.

**Why this priority**: This is the core of the request: parity of the derived convention, so agent-made worktrees are indistinguishable from user-made ones.

**Independent Test**: Call `create_worktree` with `type`, `ticket` and `name` against a scratch repository; compare the result and the sidebar tags with what the form derives for the same inputs.

**Acceptance Scenarios**:

1. **Given** no branch `fix/123_login-crash` exists, **When** the agent calls `create_worktree` with type `fix`, ticket `#123`, name `login crash`, **Then** a worktree is created on new branch `fix/123_login-crash` in directory `.claude/worktrees/fix-123_login-crash`, and the sidebar shows type `fix` and issue tag `123`.
2. **Given** type `feat`, no ticket, name `Dark mode`, **When** the agent calls the tool, **Then** the branch is `feat/dark-mode` and the directory `feat-dark-mode`, with no empty ticket segment and no issue tag.
3. **Given** a successful call, **When** the result is read, **Then** it reports the derived type, ticket (absent when none), branch and directory, as well as the worktree ref and path the tool already returns.

---

### User Story 2 - Reject inputs the form would reject (Priority: P1)

An agent that passes an unknown type, a name that slugifies to nothing, or a missing type gets a clear refusal naming the problem, and nothing is created.

**Why this priority**: Validation is the point of moving derivation into one place; silent wrong tags are the reported problem.

**Independent Test**: Call the tool with each invalid input and check the error and that no branch or directory appeared.

**Acceptance Scenarios**:

1. **Given** type `feature`, **When** the agent calls the tool, **Then** it is refused as invalid input naming the allowed types, and no worktree or branch is created.
2. **Given** name `???`, **When** the agent calls the tool, **Then** it is refused with the form's "enter a name (letters or digits)" reason.
3. **Given** name given but no type, **When** the agent calls the tool, **Then** it is refused with the form's "select a type" reason.
4. **Given** a derived branch that already exists, **When** the agent calls the tool, **Then** it is refused with a reason that names the branch and tells the agent to retry with `branch` and `mode` `existing_local` or `track_remote`; nothing is overwritten.

---

### User Story 3 - Start from a GitHub issue (Priority: P2)

An agent gives `github_issue: 123` and gets a worktree whose ticket is the issue number, whose name is the issue title and whose type comes from the issue's labels, as the form's GitHub issue source does.

**Why this priority**: Completes parity, but needs a GitHub lookup and so depends on the sign-in the form already uses; stories 1 and 2 stand without it.

**Independent Test**: With GitHub access stubbed, call the tool with `github_issue` for an open issue labelled `bug` and compare with the form's pick of the same issue.

**Acceptance Scenarios**:

1. **Given** open issue 123 titled "Login crash" labelled `bug` and the default label mapping, **When** the agent calls the tool with `github_issue: 123`, **Then** the worktree is `fix/123_login-crash`, as the form creates for that pick.
2. **Given** an issue none of whose labels map to a type, **When** the agent passes `github_issue` without `type`, **Then** it is refused with the form's "select a type" reason; with an explicit `type` it succeeds.
3. **Given** the issue is closed, is a pull request, does not exist, or GitHub cannot be reached or the project has no GitHub remote, **When** the agent passes `github_issue`, **Then** it is refused with the plain-language reason the form shows for that case, and nothing is created.
4. **Given** the agent also passes `type`, `ticket` or `name`, **When** the issue resolves, **Then** the explicit value replaces the one taken from the issue (the form lets the user edit what a pick filled in).

---

### User Story 4 - Existing literal-branch calls keep working (Priority: P1)

An agent that calls `create_worktree` with `branch`, `mode`, `remote` and optional `name` exactly as today sees no change.

**Why this priority**: The tool is shipped; breaking callers is not acceptable.

**Independent Test**: Run the existing 034 acceptance scenarios for `create_worktree` unchanged.

**Acceptance Scenarios**:

1. **Given** the call shapes accepted today, **When** they are replayed, **Then** results and errors are unchanged apart from the additional derived fields in the result.
2. **Given** `branch`, `mode` or `remote` together with `type`, `ticket`, `github_issue`, or with `name` meant as a description, **When** the agent calls the tool, **Then** [NEEDS CLARIFICATION: refuse as ambiguous (derived inputs always mean a new branch, `mode`/`remote` rejected), or let `mode` (and `remote` with `track_remote`) apply to the derived branch so the collision hint can be followed with the same inputs?]

### Edge Cases

- Empty or whitespace-only `ticket`: treated as no ticket, as in the form.
- Ticket with symbols (`ABC-123`, `#123`, `gh 7`): slugified as in the form; a ticket that slugifies to nothing is treated as no ticket.
- Very long names and GitHub titles: the same 50-character cut at a word boundary the form applies to an issue title (034-github-issue-worktree FR-010); an explicit `name` is never shortened, as in the form.
- Names that slugify to a Windows-reserved device name get the form's suffix; the result is valid on Linux, macOS and Windows.
- Two agents (or an agent and the app) create the same derived branch at once: exactly one succeeds, the other is refused as in the form; no partial worktree remains.
- Partial failure (for example a submodule fails to initialise): rolled back exactly as the form's creation rolls back; branch and directory are removed.
- Calls from the Default session (project root) remain allowed, as today.
- No type/ticket/name/github_issue and no `branch`: refused as invalid input stating what to provide (`branch` is no longer required by the published schema; the tool enforces this).
- The derived directory already exists while the derived branch does not (a leftover directory, or another branch mapping to the same directory name): refused, nothing created, nothing overwritten.
- `github_issue` that is not a positive integer: refused as invalid input.
- No overwrite mode is offered, with or without the new inputs.
- Several windows open on the project show the new worktree at once with the right tags.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: `create_worktree` MUST accept an optional `type` limited to the form's Conventional-Commits list (`feat`, `fix`, `chore`, `docs`, `refactor`, `test`, `build`, `ci`, `perf`, `style`) and refuse any other value as invalid input that names the allowed values.
- **FR-002**: `create_worktree` MUST accept an optional free-text `ticket`, slugified by the same rules as the form; a blank ticket, or one that slugifies to nothing, means no ticket.
- **FR-003**: When `type`/`ticket`/`name` are used, `name` MUST be the description, and the branch and directory MUST be derived from type, ticket and name with exactly the rules the New worktree form uses, so that the same inputs always give the same branch and directory from either path.
- **FR-004**: Refusals for a missing type, a name that slugifies to nothing and an invalid derived branch MUST carry the same reasons the form shows.
- **FR-005**: `create_worktree` MUST accept an optional `github_issue` (a positive integer) and resolve ticket (the issue number), name (the issue title, cut as the form cuts it) and type (the first entry of the label-to-type mapping matching one of the issue's labels, ignoring case) as the form does, reading the mapping at call time.
- **FR-006**: An explicit `type`, `ticket` or `name` passed with `github_issue` MUST replace the value taken from the issue.
- **FR-007**: A `github_issue` that is not an open issue (closed, a pull request, or missing), or cannot be looked up (no GitHub remote, no tooling, no sign-in, no access, no network, rate limit, no answer within 10 seconds), MUST be refused with the form's plain-language reason and create nothing.
- **FR-008**: The lookup MUST use the user's existing GitHub sign-in and MUST NOT ask for, store or return credentials; it MUST happen only when `github_issue` is passed. Every other call MUST work with no network and no sign-in.
- **FR-009**: Combining the literal `branch`, `mode` or `remote` with the derived inputs MUST follow the rule decided in User Story 4, scenario 2, and the FR-011 hint MUST be actionable under that rule.
- **FR-010**: Calls that use only `branch`, `name`, `mode` and `remote` MUST behave as before.
- **FR-011**: When the derived branch already exists, the call MUST be refused with a reason that names the branch and tells the agent to retry with `branch` and `mode` `existing_local` or `track_remote`; it MUST NOT overwrite or reuse silently. The tool MUST continue to offer no overwrite mode.
- **FR-012**: The result MUST report the derived type, ticket (omitted when none), branch and directory in addition to the fields it returns today.
- **FR-013**: The created worktree MUST carry the type and issue tag in the sidebar that the form would give the same inputs, in every window.
- **FR-014**: Creation MUST go through the same operation as the form, so submodule handling, validation and rollback on partial failure are those of the form; a failed call leaves no branch or directory behind.
- **FR-015**: The tool's published input schema and description MUST list the new inputs, how they combine and the rule for `branch`, so an agent can use them without reading the spec; `branch` is no longer schema-required.
- **FR-016**: Naming derivation and validation MUST live in one shared place used by the form and the tool, so the two cannot drift; a test MUST fail when they disagree.

### Key Entities

- **Worktree request**: either a literal branch (existing modes) or a derived request (type, ticket, name, optional issue); resolves to a type, ticket, branch and directory.
- **Issue resolution**: issue number, title and labels turned into ticket, name and type through the label-to-type mapping.
- **Create result**: worktree ref, path, branch and directory as today, plus type and ticket.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: For every one of the ten types, with and without a ticket, the branch and directory the tool derives equal those the form derives for the same inputs (100% over the full table).
- **SC-002**: Every invalid input listed in the scenarios is refused with a reason naming the problem, and in none of them is a branch or directory left behind.
- **SC-003**: A worktree created by the tool shows the same type and issue tags in the sidebar as one created by the form from the same inputs, in 100% of compared cases.
- **SC-004**: All acceptance scenarios of the shipped `create_worktree` (feature 034-daemon-mcp-server) pass unchanged.
- **SC-005**: A call without `github_issue` makes no network request.

## Assumptions

- The tool keeps its name and its place in the operations table; the change is additive.
- The label-to-type mapping lives in the app-wide settings and the daemon can read it; where it cannot, the default mapping applies (to be settled in planning).
- The GitHub lookup reuses the form's issue source and its failure reasons rather than a second implementation.
- Collisions are refused rather than prompted: an MCP call has no interactive prompt; the agent chooses a mode and calls again.
- The `name` input keeps its meaning (directory name) for literal-`branch` calls and becomes the description when the derived inputs are used.

## Out of Scope

- An overwrite mode, or any prompt back to the agent about a collision.
- Listing or searching GitHub issues through MCP (only resolving one number).
- Changing the form, the label mapping UI or the naming convention itself.
- Configurable naming formats.
