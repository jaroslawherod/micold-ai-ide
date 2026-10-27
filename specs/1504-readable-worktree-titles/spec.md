# Feature Specification: Readable worktree titles without a GitHub issue title

**Feature Branch**: `claude/project-thread-t2a600`

**Created**: 2026-10-27

**Status**: Draft

**Input**: User description: "Implement GitHub issue #1504: Worktrees without a GitHub issue title
should still show a readable overview. Follow-up to #1488, which fixes the main cause: a closed
issue's title was rejected by the lookup, so its card fell back to the branch name. What's left:
worktrees whose title can't come from a GitHub issue still show the raw folder or branch name as the
card title: no numeric ticket in the name, or a Jira-style ticket; a ticket only in the branch name,
not in the folder name (`known_title` / `titles_to_fetch` look at the directory name only); a lookup
that failed once (titles are asked once per app run and never retried). Expected: every worktree
shows a human-readable title. Possible fallbacks, in some order: the Jira issue summary, the session
title, the open PR's title, the spec title from `specs/<feature>/spec.md`, the branch's last commit
subject. The folder name stays secondary (tooltip). Owning specs: `specs/813-worktree-overview`
(FR-015), `specs/1120-overview-card-readability` (User Story 2)."

## Terms

- **Location title**: the first line of a worktree's Overview card and of its sidebar row (feature
  1125 shows the same text in both). Today it is the linked GitHub issue's title, else the branch
  name, else the folder name (813 FR-015, 1120 User Story 2).
- **Readable title**: a title written for a person, from one of the sources in FR-001 items 1–6,
  as opposed to a raw name (the branch name or the folder name).
- **Title source**: where a location title came from: GitHub issue, Jira issue, pull request,
  spec, session, commit, branch or folder.
- **Linked ticket** (existing, unchanged): the ticket the naming convention reads from the
  worktree's folder name. The sidebar's issue tag (533), the Jira status chip and the keys sent to
  Jira (1101) keep reading it; this feature does not widen it.
- **Title ticket**: the ticket used only to look up a location title: the linked ticket when the
  folder name has one, else the ticket the same naming convention reads from the branch name. A
  number (GitHub) or a `<PROJECT>-<digits>` key (Jira).
- **Feature spec**: the spec file of the feature folder the Overview already picks for a worktree's
  progress (813, edge case *Several feature folders in one worktree*).

## User Scenarios & Testing *(mandatory)*

### User Story 1 - A ticket in the branch or a Jira ticket still gives the issue's title (Priority: P1)

The developer names some worktrees after a Jira ticket (`feat-ABC-42_login`), and some have a
generic folder name while the branch carries the ticket (folder `project-thread-t2a600`, branch
`feat/1504-readable-titles`). Today both cards read as the raw name. With this change the first
shows the Jira issue's summary and the second the GitHub issue's title, exactly as a worktree whose
folder names a GitHub issue does.

**Why this priority**: These worktrees do have an issue, the most authoritative description there
is; the app only fails to find or read it.

**Independent Test**: With GitHub issues on and a Jira connection set, open the Overview with a
worktree whose folder names a Jira key, and one whose branch (not folder) names a GitHub issue
number; check both titles, and that the sidebar rows show the same text.

**Acceptance Scenarios**:

1. **Given** a worktree whose folder name has no ticket and whose branch is
   `feat/1504-readable-titles`, with GitHub issue #1504 readable, **When** its card is shown,
   **Then** the title is issue #1504's title.
2. **Given** a worktree whose folder names Jira key `ABC-42`, with a Jira connection for the
   project that can read `ABC-42`, **When** its card is shown, **Then** the title is that Jira
   issue's summary.
3. **Given** a worktree whose folder names ticket 813 and whose branch names ticket 900, **When**
   its card is shown, **Then** the title comes from ticket 813: the folder's ticket wins.
4. **Given** a worktree whose Jira issue cannot be read (no connection, unknown key, error),
   **When** its card is shown, **Then** the title falls through to the next source (FR-001).

---

### User Story 2 - A worktree with no readable issue is titled from its own work (Priority: P1)

The developer's worktree has no ticket at all (`claude/project-thread-7041o6`), or its issue cannot
be read. The app still knows what the worktree is about: its branch's open pull request has a
title, its feature spec has a name, its session has a title, its branch has commits. With this
change the card leads with the best of these instead of the branch or folder name, and the folder
name moves to the title's tooltip.

**Why this priority**: It is the case the issue is about: a card titled
`claude/project-thread-7041o6` tells the developer nothing.

**Independent Test**: Open the Overview with worktrees that each have exactly one of: an open pull
request, a feature spec, a titled session, a commit of their own; and one with none of them. Check
each title and its tooltip.

**Acceptance Scenarios**:

1. **Given** a worktree with no readable issue whose branch has an open pull request titled "Add
   readable titles", **When** its card is shown, **Then** the title is "Add readable titles".
2. **Given** a worktree with no readable issue and no open pull request whose feature spec is
   headed `# Feature Specification: Overview cards, readable layout and content`, **When** its card
   is shown, **Then** the title is "Overview cards, readable layout and content".
3. **Given** a worktree with none of the sources above whose main session has the title "Fix the
   flaky resize test", **When** its card is shown, **Then** the title is "Fix the flaky resize
   test".
4. **Given** a worktree with none of the sources above whose branch has commits of its own, the
   latest with subject "fix(overview): wrap long titles", **When** its card is shown, **Then** the
   title is "fix(overview): wrap long titles".
5. **Given** a worktree with none of the sources in FR-001 items 1–6, **When** its card is shown,
   **Then** the title is the branch name, else the folder name, as today.
6. **Given** any worktree card or sidebar row, **When** the user hovers its title, **Then** the
   tooltip shows the folder name and names the title's source (for example *From pull request
   #1520*), unless the title is the folder name itself.
7. **Given** a card titled from a lower source, **When** a higher source becomes known (a pull
   request is opened, an issue title is read), **Then** the title changes to the higher source on
   the next refresh, with no other action from the user.

---

### User Story 3 - A failed issue read is tried again (Priority: P2)

The developer opened the app offline, or GitHub timed out once. Today every title asked for in that
moment stays the branch name until the app restarts. With this change a read that failed is asked
again the next time the app would ask for titles anyway, so the cards recover once the network
does.

**Why this priority**: It turns a lasting wrong title into a temporary one, but with User Story 2
in place a failed read already shows a readable fallback.

**Independent Test**: Make the first title read fail (offline), restore the network, wait past the
retry interval, close and reopen the Overview; the card shows the issue's title.

**Acceptance Scenarios**:

1. **Given** a GitHub issue title read that failed (offline, timeout, service error), **When** the
   app next asks for titles (the Overview opens, the worktree list refreshes, the session service
   reconnects, GitHub issues are switched on) at least the retry interval (FR-009) after the
   failure, **Then** that title is asked for again.
2. **Given** a read that answered that the number is no issue (missing, or a pull request),
   **When** the app next asks for titles in the same run, **Then** that number is not asked for
   again.
3. **Given** a failed read, **When** the app asks for titles again before the retry interval has
   passed, **Then** the number is not asked for again yet.

---

### Edge Cases

- **Project root (Default location)**: keeps today's title (its display name, or branch); this
  feature changes worktree titles only.
- **Branch with no commits of its own** (created from the default branch, nothing committed yet):
  its last commit is the default branch's, which says nothing about the worktree, so the commit
  source gives no title.
- **Detached HEAD or no branch**: the pull request, branch-ticket and commit sources give no title;
  the others still apply.
- **Several pull requests for the branch**: the open one is used; with none open, the pull request
  source gives no title (a merged or closed pull request's title is not used).
- **Spec file missing, unreadable, without a heading, or with the template placeholder
  (`[FEATURE NAME]`)**: the spec source gives no title.
- **Session title pending, or only the placeholder**: the session source gives no title. With
  several sessions, only the location's main session (813 FR-013) is used.
- **A source text that is empty or only whitespace after trimming**: that source gives no title.
- **A very long or multi-line source text**: only its first line is used, trimmed; the card and
  row ellipsize it as they already do for long titles (1120).
- **Both a GitHub number and a Jira key** in the folder or branch: the ticket the naming convention
  reads first is the title ticket; only that one is looked up.
- **GitHub issues switched off, or no Jira connection**: that issue source gives no title and is
  not read (813 FR-019, 1102); the remaining sources still apply.
- **A worktree renamed by the user** (feature 1125 custom name): the custom name still wins in the
  sidebar, as today.
- **Pull request status off or unread**: the pull request source gives no title; no reading is
  started for it.
- **Spec file with CRLF line endings or a byte-order mark before the heading**: the heading is
  still found; the BOM and the trailing `\r` are not part of the title.
- **Windows paths, and branch names or folders with `\` or non-ASCII characters**: the spec and
  commit sources read the worktree at its own path on every platform; a ticket is found in the
  branch name the same way on every platform.
- **git missing, failing or timing out**: the commit source gives no title; the other sources still
  apply and the window is not blocked.
- **A title read (issue, spec, commit) that finishes after a newer refresh or after its worktree is
  removed**: a stale result is dropped, never shown over the newer refresh's title; a removed
  worktree's result is discarded.
- **The main session changes (archived, another becomes main) while titles are computed**: the next
  refresh uses the new main session's title (FR-010).
- **Two windows asking for, or retrying, the same issue's title**: each number is read at most once
  per asking occasion per window, and a pending read is not asked for again until it answers; a
  retry is never sent while a read of the same number is pending.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: A worktree's location title MUST be the first source in this order that gives a
  non-empty text:
  1. the title ticket's GitHub issue title, when the title ticket is a number;
  2. the title ticket's Jira issue summary, when the title ticket is a Jira key;
  3. the title of the open pull request for the worktree's branch;
  4. the title in the worktree's feature spec heading;
  5. the main session's title (recorded by its AI CLI, or derived from the first prompt);
  6. the subject of the latest commit on the branch that is not on the project's default branch;
  7. the branch name;
  8. the folder name (the location's display name).

  [NEEDS CLARIFICATION: should the session title (item 5) rank above the pull request (item 3)
  and spec (item 4) titles, given that it describes the latest conversation rather than the
  worktree's purpose? Default as written: below them.]
- **FR-002**: The title ticket MUST be read from the folder name first and, when the folder name
  has no ticket, from the branch name, with the same naming convention; a ticket in the folder name
  MUST win over a different one in the branch name. The title ticket MUST be used only for the
  location title: the sidebar's issue tag, the Jira status chip and the set of keys whose status is
  read keep using the linked ticket (folder name only), unchanged.
- **FR-003**: A GitHub issue title read for a ticket in the branch name MUST follow the rules
  titles from the folder name already follow: read once per issue unless the read failed (FR-009), seeded from issues the app
  already holds, any state counts (813 FR-015, BUG-1488), none read while GitHub issues are off or
  there is no route to the session service.
- **FR-004**: A Jira issue summary MUST be read only for a valid `<PROJECT>-<digits>` key of the
  project's Jira connection. For a linked ticket it MUST come with the reading the app already makes
  for that key's status (feature 1101), not from a reading of its own. A title ticket that is a
  Jira key only in the branch name MUST have its summary read with the same request, on the same
  occasions and under the same Jira switch and disclosure as the status reading, but its status MUST
  NOT be shown (no chip). A failed or absent reading gives no title.
- **FR-005**: The pull request, session and branch sources MUST use data the app already holds
  (the pull request reading, the session list, the worktree list); building titles MUST NOT start
  any new network reading or polling (813 FR-019).
- **FR-006**: The spec and commit sources MUST be read from the worktree's own files and repository
  on the machine, at most once per worktree list refresh, and MUST NOT block the window while they
  are read.
- **FR-007**: The Overview card and the sidebar row of a worktree MUST show the same location
  title (feature 1125), except where the sidebar shows the user's custom name.
- **FR-008**: The title's tooltip MUST show the folder name and the title source, worded for a
  person (*From GitHub issue #1504*, *From Jira ABC-42*, *From pull request #1520*, *From the
  feature spec*, *From the session title*, *From the latest commit*, *Branch name*); a title that
  is the folder name MUST keep today's tooltip.
- **FR-009**: A GitHub issue title read that failed (offline, timeout, service or tooling error)
  MUST be asked for again on the next occasion the app already asks for titles, once at least
  5 minutes have passed since the failure; a read that answered that the number is no issue MUST
  NOT be asked for again in the same run. While a retry is pending or failing, the card MUST show
  the next source's title (FR-001), never a "loading" text.
- **FR-010**: A location title MUST change to a higher source's text within one refresh of that
  source becoming known, and to the next source's text within one refresh of its source going away
  (the pull request closes, the session is archived).
- **FR-011**: Titles from every source (issue titles and summaries, pull request titles, spec
  headings, session titles, commit subjects) MUST stay on this machine: this feature MUST NOT log
  them or send them to any service (813 FR-020).
- **FR-012**: Only the first line of a source's text, trimmed, MUST be used; a spec heading MUST
  have its `Feature Specification:` prefix removed, and the template placeholder MUST give no
  title.
- **FR-013**: This feature amends 813 FR-015 (task summary: issue title, else branch, else display
  name) and 1120 User Story 2 Acceptance Scenarios 1 and 2 (title is the issue title, else the
  branch; the title's tooltip shows only the display name): FR-001 supersedes their order and FR-008
  their tooltip. Those specs' text MUST be updated to point here when this feature ships.

### Key Entities

- **Title source**: one of the kinds in FR-001, with the text it gives for a worktree (or none) and,
  for the issue and pull request kinds, the number or key it came from.
- **Title lookup** (existing, extended): per issue number, pending, known, no such issue, or
  failed at a time; only failed entries are retried (FR-009).
- **Location title**: the text shown and the source it came from, computed per worktree from the
  sources in FR-001's order.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: For a worktree with any of the sources in FR-001 items 1–6, the card and sidebar row
  title is never the raw branch or folder name; checked on a fixture project with one worktree per
  source and one with none.
- **SC-002**: A worktree whose ticket is only in its branch name shows its issue's title on first
  Overview open, the same as one whose folder names the ticket.
- **SC-003**: After a failed issue read and a restored network, the card shows the issue's title at
  most one title-asking occasion after the retry interval has passed, without restarting the app.
- **SC-004**: Opening the Overview with this feature starts no network reading that it did not
  start before, apart from GitHub issue title reads and Jira summary reads for title tickets found
  only in branch names, and retries of failed GitHub reads (FR-009).
- **SC-005**: No spec, commit or issue read for titles runs on the window's UI thread; with 50
  worktrees, choosing their titles from data already read adds no more than 2 ms to an Overview
  refresh.

## Out of scope

- Refreshing a title that was read successfully (an issue renamed after it was read) within a run.
- Titles for the project root (Default location) and for orphan folders beyond today's behaviour.
- Letting the user choose or reorder title sources.
- Showing titles anywhere other than the Overview card and the sidebar row.

## Assumptions

- The naming convention's ticket parser (folder `<type>-<ticket>-<name>`, branch
  `<type>/<ticket>-<name>`) already recognises both GitHub numbers and Jira keys; applying it to
  the branch name needs no new convention.
- The Jira status reading (feature 1101) can carry each issue's summary in the same request it
  already makes for statuses.
- The pull request reading already holds each branch's pull request titles (feature 813).
- Sessions already carry their title or derived label (feature 032), and the Overview already
  knows each location's main session.
- The Overview already picks one feature folder per worktree for progress; the spec source uses the
  same folder.
- A 5-minute retry interval is short enough to recover from a network blip in the same sitting and
  long enough not to hammer GitHub when it is down.
