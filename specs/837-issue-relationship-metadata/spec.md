# Feature Specification: Show a GitHub issue's relationships when a worktree is created for it

**Feature Branch**: `claude/project-thread-tw6sph`

**Created**: 2026-10-16

**Status**: Draft

**Input**: GitHub issue #837, "Show all GitHub issue metadata (blocked by, blocking, parent,
sub-issues…) when a worktree is created". Labels: enhancement, effort:high, flow:feature.

## Background

A worktree can be made for a GitHub issue in two ways: the New worktree form's **GitHub issue**
source (feature 034-github-issue-worktree, 038), where the user picks an open issue and its number,
title and labels fill the ticket, name and type; and the agent tool `create_worktree` with
`github_issue` (feature 550, 532), which reads the issue's number, title, state and labels to name
the worktree. Neither path tells anyone what the issue is tied to. The form shows a picked issue's
title, labels and, on hover, the start of its description; the tool's result reports only the
derived type, ticket, branch and directory. So a user or an agent can start work in a fresh worktree
on an issue that is blocked by another open issue, that is one sub-issue of a larger parent, or that
already has a pull request open against it, and find out only later.

## Clarifications

### Session 2026-10-16

- Q: Does a `create_worktree` call with `github_issue` wait for the details before it returns, or
  may it return without them? → A: It waits, within the 10-second bound, so its result carries
  the details or the reason they could not be read. _(agent-resolved:
  specs/837-issue-relationship-metadata/spec.md#user-story-1, FR-004, FR-007)_
- Q: When does the form read the details: when the user picks the issue, or when Create is pressed?
  → A: When the issue is picked, so they show before Create; pressing Create never waits for them.
  _(agent-resolved: specs/837-issue-relationship-metadata/spec.md#user-story-2, scenarios 1 and 4)_

## User Scenarios & Testing *(mandatory)*

### User Story 1 - An agent learns an issue is blocked when it creates the worktree (Priority: P1)

An agent calls `create_worktree` with `github_issue: N`. The result, besides the worktree it made,
describes issue N: what blocks it, what it blocks, its parent and sub-issues, its labels, assignees,
milestone, project fields, and the pull requests linked to it. The agent sees that N is blocked by
an open issue before it starts work and can tell the user.

**Why this priority**: the agent is the reader that otherwise never looks; the issue's motivation
names it first, and the tool already reads the issue, so the result is the one place every
agent-made worktree for an issue passes through.

**Independent Test**: with GitHub access stubbed, call `create_worktree` with `github_issue` for an
issue that has each kind of relationship, and read the result; repeat for an issue with none.

**Acceptance Scenarios**:

1. **Given** open issue 12 is blocked by open issue 10 and closed issue 9, **When** the agent calls
   `create_worktree` with `github_issue: 12`, **Then** the worktree is created as today and the
   result lists 10 and 9 under "blocked by", each with its number, title and whether it is open or
   closed, and states that 12 is blocked by one open issue.
2. **Given** issue 12 blocks issue 14, has parent issue 3 and sub-issues 15 and 16, **When** the
   call succeeds, **Then** the result lists 14 under "blocking", 3 as the parent and 15 and 16 as
   sub-issues, each with number, title and open or closed, and the sub-issues with how many of them
   are closed out of the total.
3. **Given** issue 12 has labels, assignees, a milestone and a value in a project field, **When** the
   call succeeds, **Then** the result reports the label names, the assignees' logins, the milestone
   title, and each project the issue is in with that project's field values for the issue.
4. **Given** pull request 20 will close issue 12 when merged, and pull request 21 mentions it,
   **When** the call succeeds, **Then** the result lists 20 among the closing pull requests and 21
   among the linked ones, each with number, title and its state (open, draft, merged or closed).
5. **Given** an issue with no blockers, nothing blocked, no parent, no sub-issues, no linked pull
   requests, no assignees, no milestone and no project, **When** the call succeeds, **Then** the
   result carries no field for any of those: only what the issue has is reported.
6. **Given** a call without `github_issue` (a literal `branch`, or `type`/`ticket`/`name`), **When**
   it runs, **Then** the result is exactly as before and nothing is asked of GitHub.

---

### User Story 2 - A user sees an issue's relationships in the app before creating its worktree (Priority: P1)

A user picks an issue in the New worktree form's **GitHub issue** source. Before pressing
**Create**, the form shows the picked issue's relationships and metadata, and makes it plain when
the issue is blocked by an open issue. The user can still create the worktree.

**Why this priority**: the acceptance names the app as well as the tool; the moment a user picks the
issue is the last moment before work starts, which is when "blocked" matters.

**Independent Test**: with GitHub access stubbed, open the form, pick an issue with relationships
and read the form; pick one without and check nothing extra appears.

**Acceptance Scenarios**:

1. **Given** the form with **GitHub issue** chosen, **When** the user picks issue 12 that is blocked
   by open issue 10, **Then** the form shows a "Blocked by" entry naming #10 and its title, marked
   as open and drawn so it reads as a warning, and **Create** stays enabled.
2. **Given** the picked issue has a parent, sub-issues, issues it blocks, assignees, a milestone,
   project fields or linked pull requests, **When** the details arrive, **Then** the form shows each
   kind that is present, with the same content User Story 1 lists for the tool.
3. **Given** the picked issue has no relationships and none of that metadata, **When** the details
   arrive, **Then** the form shows nothing beyond what it shows today.
4. **Given** the details are still being read, **When** the user presses **Create**, **Then** the
   worktree is created without waiting for them.
5. **Given** the user picks another issue, or clears the pick, **When** details for the earlier pick
   arrive, **Then** they are not shown for the new pick.
6. **Given** a worktree created from a picked issue, **When** it appears in the sidebar, **Then**
   its row looks as it does today: the details are shown in the form only. [NEEDS CLARIFICATION:
   should the relationships also stay visible on the created worktree (for example on its sidebar
   row or its issue tag), or is showing them in the form before Create enough? Default: the form
   only, since the sidebar holds no issue data today.]

---

### User Story 3 - The details never block or break creating a worktree (Priority: P2)

Reading relationships is extra. When part of it cannot be read, the worktree is still created and
the rest is shown; nothing new is asked of the user and no worktree is refused because of it.

**Why this priority**: the shipped paths must keep working; a missing permission for one kind of
data must not turn a working call into a failing one.

**Independent Test**: stub GitHub so that project fields (or any one kind) cannot be read, and so
that the whole details read times out; create a worktree both ways each time.

**Acceptance Scenarios**:

1. **Given** the user's GitHub sign-in cannot read the issue's projects, **When** a worktree is
   created through either path, **Then** it is created, every other kind is shown, and projects are
   reported as not readable rather than as absent.
2. **Given** the details cannot be read at all (no answer in time, rate limit, network loss), **When**
   the tool is called with `github_issue`, **Then** the worktree is created when the issue itself was
   read as feature 550 requires, and the result says the relationships could not be read and why.
3. **Given** the same failure in the form, **When** the user has picked the issue, **Then** the form
   says in one line that the issue's details could not be read, and creation works as today.
4. **Given** an issue that is blocked by open issues, **When** a worktree is requested through either
   path, **Then** it is created: being blocked is reported, never enforced.

### Edge Cases

- A related issue in another repository (a cross-repository blocker, parent or pull request): shown
  with its `owner/name#N` reference instead of a bare number.
- A related issue the user cannot see (private repository without access): left out, and the count
  of hidden items is stated so "blocked by" is not silently understated.
- Many relationships (an epic with 200 sub-issues, an issue with dozens of mentions): each list shows
  at most 20 items, followed by "and N more"; the totals (such as closed out of all sub-issues) count
  every item.
- Titles of related items are untrusted text: shown as plain text, with characters that override
  text direction neutralised as the description tooltip does (038).
- A "blocked by" issue that is closed does not count toward "blocked"; only open blockers make the
  issue read as blocked.
- A pull request both closing and mentioning the issue is listed once, as closing.
- A project field with no value for the issue is left out; a project with no field values for it is
  shown by name only.
- An issue picked, then the form closed before the details arrive: the late answer is dropped.
- Explicit `type`, `ticket` or `name` given with `github_issue` (550 FR-006): naming follows them as
  today; the details still describe the issue.
- Two sessions (or a session and the user in a window) create worktrees for the same issue at once:
  each read is independent; the 550 collision rule decides which worktree is created, and the
  refused call reports no details. Several windows open on the project each show only their own
  form's details.
- Platform differences: the details read the same on Linux, macOS and Windows; nothing depends on
  the platform beyond how GitHub access is obtained, which is unchanged.
- The repository uses GitHub Enterprise or a GitHub without issue dependencies or sub-issues: kinds
  the server does not offer are treated as not readable, as in User Story 3.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: When a worktree is created for a GitHub issue, the issue's details MUST be read: by
  the agent tool during a `github_issue` call, and by the form when the user picks the issue in its
  GitHub issue source, before **Create**. The details are: the issues blocking it, the
  issues it blocks, its parent issue, its sub-issues, its labels, assignees and milestone, the
  projects it belongs to with its field values in each, and the pull requests linked to it,
  separated into those that will close it and others.
- **FR-002**: Each related issue MUST be described by its reference (number, or `owner/name#N` for
  another repository), title and open or closed state; each pull request by reference, title and
  state (open, draft, merged, closed).
- **FR-003**: Sub-issues MUST carry how many are closed out of the total; "blocked by" MUST carry how
  many of the blockers are open, and the issue MUST read as blocked exactly when that count is above
  zero.
- **FR-004**: The tool's result for a `github_issue` call MUST include these details, reporting only
  the kinds the issue has; a kind with nothing in it MUST be omitted, so an issue with no
  relationships adds nothing but what is required to identify the issue.
- **FR-005**: The New worktree form MUST show the picked issue's details before **Create**, showing
  only the kinds present, with an open blocker presented as a warning.
- **FR-006**: The details MUST NOT gate creation: a blocked issue, a slow read or a failed read MUST
  NOT refuse, delay or change the worktree that would be created without this feature.
- **FR-007**: A kind that cannot be read (permission, unsupported by the server) MUST be reported as
  not readable, distinct from empty; a failure of the whole read MUST be reported in one plain
  sentence naming the reason, in the tool's result and in the form.
- **FR-008**: Each list MUST show at most 20 items followed by the number not shown; counts MUST
  cover every item, including items left out because the user cannot see them.
- **FR-009**: Details MUST be read with the user's existing GitHub sign-in, as features 550 and
  034-github-issue-worktree read issues; nothing new MUST be asked for, stored or returned about
  credentials. Only the repository's owner and name and the issue number MUST leave the machine.
- **FR-010**: Calls and form uses that do not involve a GitHub issue MUST behave exactly as before
  and make no request to GitHub.
- **FR-011**: The read MUST finish or be abandoned within 10 seconds, the bound feature 550 sets for
  the issue lookup; an abandoned read is a failure under FR-007. The worktree created does not
  depend on the read; a `github_issue` tool call MUST wait for it within that bound before
  returning, so its result carries the details or the FR-007 failure.
- **FR-012**: Details shown for an issue MUST be those of the issue currently picked; an answer for an
  earlier pick or a closed form MUST be dropped.
- **FR-013**: Text taken from GitHub (titles, labels, milestone, field values, logins) MUST be shown
  as plain text with direction-override characters neutralised, as the 038 description tooltip
  does.
- **FR-014**: The tool's published description and the agent-tools page of the user guide MUST say
  that a `github_issue` call reports the issue's relationships; the worktrees-and-sessions page MUST
  describe what the form shows for a picked issue.

### Key Entities

- **Issue details**: what an issue is tied to and tagged with: blockers, blocked issues, parent,
  sub-issues, labels, assignees, milestone, project memberships with field values, linked and
  closing pull requests, plus per-kind "not readable" marks.
- **Related item**: an issue or pull request referenced from the details: reference, title, state.
- **Project membership**: a project's name and the issue's non-empty field values in it.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: For an issue with every kind of relationship, 100% of the kinds appear in both the
  tool's result and the form, with the content FR-002 and FR-003 require.
- **SC-002**: For an issue with no relationships and no optional metadata, the tool's result and the
  form show nothing beyond what they show today, in every tested case.
- **SC-003**: An issue blocked by at least one open issue is reported as blocked in 100% of tested
  cases through both paths; one blocked only by closed issues never is.
- **SC-004**: Every acceptance scenario of features 550 and 532 for `create_worktree`, and of the
  form's GitHub issue source, passes unchanged apart from the added details.
- **SC-005**: With the details read failing or hanging, a worktree is still created through both
  paths, in the form the user waits no longer for **Create** than without this feature, and a tool
  call returns no more than 10 seconds later than it would without this feature.
- **SC-006**: A call or form use without a GitHub issue makes no network request.

## Assumptions

- "Project fields" means GitHub Projects the issue is added to and the issue's values in their
  fields; reading them may need a sign-in permission the user's GitHub CLI lacks, in which case the
  kind is reported not readable (FR-007) rather than asking the user to sign in again.
- "Linked PRs" means pull requests GitHub records as closing the issue, plus pull requests that
  reference it in their timeline.
- The form and the tool use one shared reading of the details, as 550 FR-016 requires for naming,
  so the two cannot report different things for the same issue.
- The session started together with a worktree (`create_worktree` with `prompt` or `ai_cli`) gets no
  injected text: the agent that called the tool reads the details from the result.
- Out of scope: editing relationships, refusing or warning before creating a worktree for a blocked
  issue beyond showing it, showing details for issues listed but not picked, GitLab and Jira
  sources, and keeping details fresh after creation.
