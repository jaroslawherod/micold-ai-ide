# Feature Specification: Filter the GitHub issue selector by search query, label and state

**Feature Branch**: `claude/project-thread-5zc5qk`

**Created**: 2026-10-27

**Status**: Draft

**Input**: User description: "Spec 1400" — issue #1400, *Filter the GitHub issue selector by GitHub search query, label and state*: the GitHub issue selector in the New worktree form only searches open issues by free text; the query is fixed to `repo:{repo} is:issue is:open {text}`. There is no way to hide issues that are blocked or already being worked on, so picking the next issue to start means scrolling past them. Bring the GitHub selector up to what the Jira selector has (#1185 basic filters, #1186 advanced JQL): (1) basic filters — pick from the repo's labels (include and exclude), state (open / closed / all), and optionally assignee, author and milestone, without writing a query; (2) an advanced GitHub search query, e.g. `-label:in-progress -label:blocked no:assignee sort:created-asc`, with `repo:` and `is:issue` fixed so results stay scoped to the project's repo; (3) free-text search still works on top of the active filter; (4) the filter is remembered per project, so a default like "exclude `in-progress` and `blocked`" applies every time the selector opens. Main use case: hide blocked issues and issues already in progress so the list shows only issues ready to start. Acceptance: with exclude-labels `in-progress`, `blocked` set, the list shows no issue carrying either label; a raw query like `-label:in-progress state:open` returns the same issues as the same search on github.com; when the query is invalid, the selector shows GitHub's error message instead of an empty list. Labels at entry: enhancement, flow:feature.

## Context

Feature 034 (`specs/034-github-issue-worktree`) added the GitHub issue source to the New worktree form: it loads up to 1,000 of the repository's open issues, most recently updated first, searches them locally by number, title and label name (034 FR-005), and asks GitHub's search for matches beyond the loaded issues only when the loaded list is incomplete (034 FR-005a). Feature 1102 added the switch that turns GitHub issues on or off. The Jira source of the same form gained basic field filters behind a **Filters** button (1185) and a Basic / Advanced switch with a JQL query remembered per project (1186). This feature gives the GitHub source the same two modes, with GitHub's own filter fields and search syntax, and remembers the filter per project.

## Clarifications

### Session 2026-10-27

- Q: The issue lists "optionally assignee, author and milestone" among the basic filters. Are these three filters part of this feature? → A: Yes, as User Story 6 (P3), built in the last milestone after the P1 and P2 stories; each filter may be left unset. _(provisional: recommended option, awaiting user; autopilot ledger D1)_

## Terms

- **Selector**: the GitHub issue picker in the New worktree form (feature 034): its list of the repository's issues and its search text.
- **Repository**: the GitHub repository the project's default remote names (034 FR-002, FR-004).
- **Basic mode**: the app builds the issue filter from the basic filters (FR-003).
- **Advanced mode**: the user writes the issue filter as a GitHub issue search query (FR-010).
- **Basic filters**: include labels, exclude labels, state, assignee, author and milestone (FR-003).
- **User query**: the GitHub search text typed in advanced mode. While the form is open the input holds the **draft** (what is typed, run or not); the **last run query** is the user query most recently run on this project.
- **Active filter**: the filter the list currently applies: the basic filters in basic mode, the last run query in advanced mode.
- **Default filter**: basic mode with no label, assignee, author or milestone chosen and state open. It lists exactly what the selector lists today (034 FR-004).
- **Scope**: the restriction the app always adds and the user cannot remove: issues (not pull requests) of the repository.
- **Quoted string**: text between a pair of `"`. Qualifiers, parentheses and words inside a quoted string are text, not syntax.
- **Scope qualifier**: a `repo:`, `org:`, `user:` or `owner:` qualifier, with or without a leading `-`, outside any quoted string, matched ignoring letter case.
- **Sort qualifier**: a `sort:` qualifier outside any quoted string, matched ignoring letter case.
- **Remembered selector state**: per project, the mode last used, the basic filters last chosen and the last run query.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Hide blocked and in-progress issues (Priority: P1)

A developer opens the New worktree form with the GitHub source showing. Their repository labels issues `in-progress` while someone works on them and `blocked` while they wait. They open **Filters**, add `in-progress` and `blocked` to the excluded labels, and the list shows only open issues carrying neither label. They type "crash" in the search and the list narrows within those issues. They pick one and create the worktree as before.

**Why this priority**: It is the issue's main use case and its first acceptance criterion; it delivers the mechanism every other story uses (label choices, a filtered request, the active line, clearing).

**Independent Test**: With a scripted GitHub source whose repository holds issues labelled `in-progress`, `blocked`, both and neither, exclude both labels: the list holds exactly the open issues carrying neither, and clearing the filters restores the default list.

**Acceptance Scenarios**:

1. **Given** the GitHub source is showing, **When** the user opens **Filters**, **Then** the label filters offer the repository's labels by name, alphabetically.
2. **Given** the user excluded `in-progress` and `blocked`, **When** the list reloads, **Then** it holds no issue carrying either label, and every open issue of the repository carrying neither label (up to the load cap) is listed, most recently updated first.
3. **Given** the user included `bug`, **When** the list reloads, **Then** every listed issue carries `bug`; **When** they also include `ui`, **Then** every listed issue carries both `bug` and `ui`.
4. **Given** a filter is active, **When** the user types search text, **Then** the results are the filtered issues that match the text by the selector's existing rule (number, title, label name; 034 FR-005).
5. **Given** filters are chosen, **When** the user removes one value, **Then** the list reloads without that restriction and keeps the others; **When** the user activates **Clear filters**, **Then** the default filter applies and the list is today's open-issue listing again.
6. **Given** basic mode and any filter other than the default is active, **When** the list shows, **Then** a line under the search names the active filter ("Excluding labels in-progress, blocked").

---

### User Story 2 - Come back to the same filter (Priority: P1)

The developer who excluded `in-progress` and `blocked` closes the form. The next time they open the New worktree form on that project, in this or a later app run, the GitHub list already excludes those labels, and the active line says so. On another project the selector opens with that project's own filter, or the default.

**Why this priority**: Without it the main use case costs the same clicks on every opening; the issue names remembering per project as part of the proposal.

**Independent Test**: Exclude a label on project A, close the form, reopen it on A: the list loads with the exclusion applied; open the form on project B: B's own filter (or the default) applies; restart the app and reopen on A: the exclusion still applies.

**Acceptance Scenarios**:

1. **Given** the user chose basic filters F on project A, **When** they reopen the form on A (in this or a later app run), **Then** the GitHub list loads once with F applied, and the active line names F.
2. **Given** project A remembers F, **When** the user opens the form on project B, **Then** B's own remembered filter applies, or the default filter when B has none.
3. **Given** the user last used advanced mode on A and last ran query Q, **When** they reopen the form on A, **Then** it opens in advanced mode with Q in the input and runs Q once as soon as the GitHub source is showing.
4. **Given** the user ran Q, then typed Q2 without running it, **When** they close and reopen the form, **Then** the input holds Q, not Q2.
5. **Given** the user activated **Clear filters**, **When** they reopen the form, **Then** the default filter applies.

---

### User Story 3 - Filter with a GitHub search query (Priority: P1)

A developer wants open issues nobody is assigned to and no pull request is linked to, oldest first. They switch the selector to **Advanced**, type `state:open no:assignee -linked:pr -label:blocked sort:created-asc`, press Enter, and the list shows exactly what the same search lists in the repository's issues on github.com, in that order. They type "parser" in the search and the list narrows within those results.

**Why this priority**: It covers every criterion basic filters do not (linked pull requests, dates, reactions, `OR`, sort order) and is the issue's second acceptance criterion.

**Independent Test**: With the scripted GitHub source, switch to Advanced and run a query: the source receives the query within the scope, the list holds the returned issues in the returned order, and search text narrows them locally.

**Acceptance Scenarios**:

1. **Given** the GitHub source is showing in basic mode, **When** the user switches to Advanced, **Then** the **Filters** button is replaced by a single query input, the search text stays, and the list keeps showing what it showed until a query runs.
2. **Given** advanced mode, **When** the user runs `-label:in-progress state:open`, **Then** the list holds the issues GitHub's issue search returns for that query within the repository's issues (the same set github.com's issue search lists for it in that repository), most recently updated first.
3. **Given** advanced mode, **When** the user runs a query with `sort:created-asc`, **Then** the list is in GitHub's order for that sort.
4. **Given** a query whose result includes closed issues, **When** it runs, **Then** they are listed and marked as closed (the query, not the app, decides the state).
5. **Given** a query that matches no issue, **When** it runs, **Then** the selector says no issue matches the query, distinct from an error.
6. **Given** advanced mode, **When** the user switches back to Basic, **Then** the basic filters and their list are as they were when the user left basic mode, and the draft is kept for the next switch to Advanced while the form stays open.

---

### User Story 4 - See why a query failed (Priority: P1)

A developer mistypes a qualifier value. Instead of an empty list, the selector shows GitHub's own message, with the query still in the input so they can fix it.

**Why this priority**: The issue's third acceptance criterion; without it a mistake is indistinguishable from "no matching issue".

**Independent Test**: Script the source to refuse a query with an error message; run it and check that the selector shows that message and keeps the query text.

**Acceptance Scenarios**:

1. **Given** advanced mode, **When** GitHub refuses the query with one or more messages, **Then** the selector shows those messages, in GitHub's order, in place of the list, and the input keeps the query as typed.
2. **Given** the error is showing, **When** the user edits the query and runs it again, **Then** the error is replaced by the new result.
3. **Given** a failure that is not a refusal of the query (no GitHub tooling, not signed in, no access, offline, rate limited, 10-second timeout, other failure), **When** it happens, **Then** the selector shows the same notice with **Retry** as the default list does (034 FR-007), and Retry runs the same query again.
4. **Given** GitHub refuses the query without a message, **When** the result arrives, **Then** the selector says in plain words that GitHub did not accept the search query.
5. **Given** a user query holding a scope qualifier, an unclosed quoted string or unbalanced parentheses (FR-012), **When** the user runs it, **Then** nothing is sent and the selector says why the query was not run.

---

### User Story 5 - Filter by state (Priority: P2)

A developer wants to restart work on a recently closed issue. In **Filters** they set the state to closed (or all), and the list shows the repository's closed (or open and closed) issues, each closed one marked as closed.

**Why this priority**: The issue names state among the basic filters; it is less frequent than hiding labels.

**Independent Test**: Set the state to closed: the list holds exactly the repository's closed issues (up to the cap), marked closed; set it to all: open and closed issues, each marked by its state.

**Acceptance Scenarios**:

1. **Given** basic mode, **When** the user opens **Filters**, **Then** the state filter offers open, closed and all, with open chosen by default.
2. **Given** state closed, **When** the list reloads, **Then** every listed issue is closed and is marked as closed.
3. **Given** a closed issue is listed, **When** the user picks it, **Then** the ticket, name and type are filled exactly as for an open issue (034 FR-009 to FR-014).

---

### User Story 6 - Filter by assignee, author or milestone (Priority: P3)

A developer chooses "no assignee" in the assignee filter (or themselves as the author, or the milestone `v2.4`), and the list shows only the matching issues.

**Why this priority**: The issue names these filters as optional; advanced mode already reaches them. They are in scope and are built last, after the P1 and P2 stories.

**Independent Test**: Choose each value kind in turn — assignee "me", "no assignee", a person; author "me", a person; a milestone, "no milestone" — and check the list holds exactly the matching issues.

**Acceptance Scenarios**:

1. **Given** basic mode, **When** the user opens the assignee filter, **Then** it offers the signed-in user first ("me"), then "no assignee", then the repository's assignable users whose login or name matches the typed text.
2. **Given** the author filter, **When** the user opens it, **Then** it offers "me" first, then the repository's assignable users whose login or name matches the typed text, and accepts any typed GitHub login.
3. **Given** the milestone filter, **When** the user opens it, **Then** it offers "no milestone" and the repository's open milestones by title.
4. **Given** an assignee, author or milestone is chosen, **When** the list reloads, **Then** every listed issue matches it, and it combines with the other filters (AND).

---

### Edge Cases

- **Default filter**: with the default filter active, the list, its requests and its search beyond the loaded issues are exactly today's (034 FR-004, FR-005, FR-005a); no active line is shown.
- **Free text with a filter**: search text never changes the active filter; it narrows the filtered list by 034 FR-005's rule. When the filtered list is incomplete (cap reached), search also asks GitHub for issues matching the active filter and the text, keeping only those that match the text by that rule (034 FR-005a with the filter added).
- **Typed issue number with a filter**: a typed `N` or `#N` matches only among the issues the active filter lists; an issue outside the filter is not listed, and the empty state mentions that a filter is active. When the filtered list is incomplete, the number is not looked up outside the filter, and the empty state says the issue may be outside the filter.
- **More than 1,000 matching issues**: the 1,000 first in the filter's order are listed, and the list says only those are shown and that search also finds the rest (034 FR-004 wording).
- **No labels, milestones or assignable users**: that filter shows as unavailable with a short reason ("This repository has no labels"); the other filters and the list keep working.
- **Choices fail to load**: a failure to load one filter's choices (causes of 034 FR-007) shows the cause and **Retry** on that filter only; the list, the search, the other filters and creating a worktree keep working.
- **Remembered value no longer in the repository** (a label renamed or deleted, a milestone closed): the remembered filter still applies as remembered; the list shows what GitHub returns (possibly nothing, with the empty state and **Clear filters**), and the filter panel shows that value as "not in this repository" so the user can remove it.
- **Same label included and excluded**: the user cannot choose a label for both; choosing it for one removes it from the other.
- **Label names with spaces, quotes, colons or non-Latin letters**: a label is sent so that it matches exactly that label and can never change the query's structure; a label whose name holds a `"`, a `\` or a control character is not offered (GitHub's search cannot express it exactly), and the label filter says how many labels are not offered.
- **More than 1,000 labels**: the first 1,000 alphabetically are offered and the filter says the list is incomplete; advanced mode reaches the rest.
- **Advanced query with a scope qualifier** (`repo:other/x`, `org:acme`, `-repo:...`): refused before sending, with the reason (GitHub would combine repositories rather than narrow). Inside a quoted string (`"repo:x"`) it is text and runs.
- **Advanced query with `is:pr`, `type:pr` or `is:pull-request`**: runs; the scope makes the result empty, and the selector says no issue matches.
- **Advanced query naming the repository's state** (`is:open`, `state:closed`, `is:closed`): honoured; with no state term both open and closed issues are listed.
- **Empty advanced query**: running an empty or whitespace-only query sends nothing: the prompt stands in place of the list, as before any run, and no query counts as run (M5 visual pass: an empty run listed every issue, closed ones too, which no one asks for by pressing Enter on an empty input). This differs from 1186's Jira query, which runs an empty query like any other.
- **Query of only a sort qualifier** (`sort:created-asc`): lists every issue of the repository in that order.
- **Unknown qualifier or a word GitHub reads as text** (`lable:bug`): GitHub decides; when it accepts the query, the selector shows what it returns, as github.com does; only a refusal shows an error.
- **Very long query**: longer than 1,000 characters (after the cleaning below) is refused before sending, with the limit in the message.
- **Control characters or a pasted newline in the query**: newlines and tabs count as spaces; other control characters are removed before the query runs and is remembered.
- **A result outside the scope**: an issue of another repository, or a pull request, in GitHub's answer is dropped from the list whatever the query.
- **Running a new query or changing a filter while a request is in flight**: the earlier result is discarded; only the latest filter's result or error is shown. Switching mode, switching source to Jira, closing the form or turning GitHub issues off discards an in-flight result (034 FR-007a).
- **Rapid filter changes**: several changes in a row produce a correct final list for the last combination; no intermediate result is shown after a later one.
- **Switching source to Jira and back**: the GitHub mode, basic filters and draft (with any unrun edits, FR-002) are kept, and the search text clears (034 behaviour); the list reloads with the active filter (in advanced mode, re-run the last run query, not the draft).
- **GitHub issues switched off (1102)**: no filter control is shown and no request is made; the remembered state is kept, not deleted.
- **Two windows on the same project**: each window's form keeps its own filters, query and list while open; the remembered state is the latest written by either (last write wins). A window whose form is open does not change when the other window writes.
- **Remembered state unreadable or missing**: the form opens in basic mode with the default filter; nothing else is affected.
- **Repository changed** (the default remote now names another repository): the remembered state still applies; values the new repository lacks show as "not in this repository".
- **MCP `create_worktree` with `github_issue`**: unchanged; it names one issue and applies no filter.
- **Linux, macOS and Windows (Principle VI)**: the filters, mode switch, query input, error display and remembered state behave the same on each, including when launched from a desktop launcher (034 FR-026); the remembered state lives in the per-user data location each platform already uses for the app's local state.

## Requirements *(mandatory)*

### Mode

- **FR-001**: With the GitHub source showing, the selector MUST offer a switch between basic mode and advanced mode, and MUST keep the search text in both modes. The Jira source MUST be unchanged.
- **FR-002**: Switching from advanced to basic MUST restore the basic filters and their list as they were when the user left basic mode; switching to advanced MUST show the draft, which starts as the project's last run query when the form opens and keeps unrun edits while the form stays open. Switching within an open form MUST NOT run a query by itself; the list keeps what it showed until a query runs or a filter changes.

### Basic filters

- **FR-003**: Basic mode MUST offer, behind one **Filters** control naming how many filters are chosen, these filters: include labels and exclude labels (each any number of the repository's labels), state (open, closed or all; open by default), and assignee (me, no assignee, or one person), author (me or one GitHub login) and milestone (no milestone, or one of the repository's open milestones).
- **FR-004**: The listed issues MUST be the issues of the repository that satisfy every chosen filter (AND): each carries every included label, carries no excluded label, is in the chosen state, and matches the chosen assignee, author and milestone.
- **FR-005**: Label choices MUST be the repository's labels, alphabetically, up to 1,000, with the incomplete-list notice of the Edge Cases; milestone choices the repository's open milestones; assignee and author people the repository's assignable users matching the typed text. A choice list MUST be requested only when the user first opens the control that needs it in a form showing the GitHub source, and held only in the open form (034 FR-023).
- **FR-006**: Each basic filter value MUST be sent so that it matches exactly that value and can never change the structure of the query; a label whose name holds a `"`, a `\` or a control character MUST NOT be offered; an author login typed by hand MUST be a valid GitHub login (letters, digits and single hyphens, at most 39 characters) before it is used.
- **FR-007**: In basic mode, while any filter other than the default is active, the selector MUST name it on one line under the search, and MUST offer **Clear filters**, which restores the default filter. In advanced mode the query input shows the query, and the active-filter line and **Clear filters** are not shown.
- **FR-008**: Choosing, changing or clearing a basic filter MUST reload the list with the new filter and MUST discard the result of any earlier request.
- **FR-009**: With the default filter active, the selector MUST behave and send exactly as it does today (034 FR-003 to FR-008).

### Advanced query

- **FR-010**: In advanced mode the user MUST be able to type a GitHub issue search query in the syntax github.com's issue search accepts; a query MUST run when the user presses Enter in the query input or activates the run control, and MUST NOT run on each keystroke.
- **FR-011**: The app MUST run the user query within the scope: the request MUST restrict results to the repository's issues whatever the query holds, and MUST honour the user query's terms, including state terms, negations, `no:` and `linked:` qualifiers, boolean operators and parentheses where GitHub's issue search supports them. Without a sort qualifier the list MUST be most recently updated first; with one, it MUST be in GitHub's order for it.
- **FR-012**: Before sending, the app MUST refuse a user query that holds a scope qualifier, has an unclosed quoted string, has parentheses that do not balance outside quoted strings, or is longer than 1,000 characters after cleaning, and MUST show the reason in place of the list. A refused query MUST NOT be sent.
- **FR-013**: When the form opens in advanced mode and a last run query is remembered for the project, the selector MUST run it once as soon as the GitHub source is showing; when no query was ever run on the project it MUST show the empty input with a prompt to type and run a query, and send nothing.

### Listing and search

- **FR-014**: A filtered list MUST hold at most 1,000 issues in the filter's order and MUST say when more match (034 FR-004 wording); each row MUST show the issue as today (number, title, labels, reporter, description tooltip) and MUST mark a closed issue as closed.
- **FR-015**: The selector MUST NOT list a pull request or an issue of another repository, whatever the filter or GitHub's answer.
- **FR-016**: Search text MUST narrow the filtered list by 034 FR-005's rule without contacting GitHub; when the filtered list is incomplete, search MUST also ask GitHub for issues matching the active filter and the text, merged without duplicates under 034 FR-005a's rule, FR-007 and FR-007a.
- **FR-017**: Picking an issue from a filtered list, open or closed, MUST fill the form exactly as picking it from today's list does (034 FR-009 to FR-015).

### Errors

- **FR-018**: When GitHub refuses a query (advanced, or basic with a remembered value GitHub rejects), the selector MUST show GitHub's messages in place of the list, in GitHub's order, each cleaned of control characters and cut to a readable length, and MUST keep the filter or query in place to fix. With no message it MUST say in plain words that GitHub did not accept the search query.
- **FR-019**: Other failures MUST show the notices and **Retry** of 034 FR-007; Retry MUST repeat the same request.
- **FR-020**: An empty result MUST say that no issue matches the filter (or query), distinct from an error, and MUST offer **Clear filters** in basic mode.

### Remembering

- **FR-021**: The app MUST remember, per project, the mode last used, the basic filters last chosen and the last run query, in the user's local per-user data (not in a project file, the repository or `settings.json`), and MUST restore them when the form next opens on that project, in this or a later app run. The mode MUST be remembered at each switch, the basic filters at each change, the query at each run; with two windows the latest write wins. An unrun draft MUST NOT be remembered.
- **FR-022**: The remembered state MUST be kept when GitHub issues are switched off or the source is switched to Jira, and MUST be removed with the project's own local data when the project is removed from the app.

### Privacy, requests and scope

- **FR-023**: Requests MUST follow 034 FR-025, widened by the active filter: a listing or search request may carry the repository's identity, the active filter (basic filter values or the user query) and the typed search text; a choice request carries only the repository's identity and, for a people search, the typed text. The selector's statement of what it sends to GitHub (034 FR-025) MUST say that the filter is sent.
- **FR-024**: The user query and filter values MUST NOT be written to a log line, a notice other than the selector's own, or a session's history.
- **FR-025**: With GitHub issues switched off (1102 FR-002), no filter control is shown and no filter or choice request is made.
- **FR-026**: The filters, the mode switch and the query input MUST be reachable and operable from the keyboard.
- **FR-027**: The user guide MUST document the GitHub filters: the basic filters, advanced mode and its scope, sort and refusal rules, error display, and that the filter is remembered per project.

### Key Entities

- **Basic filter set**: include labels, exclude labels, state, and optionally assignee, author and milestone; the default set lists today's open issues.
- **User query**: the GitHub search text typed in advanced mode; at most 1,000 characters; never holds a scope qualifier.
- **Filter choice**: a label, a milestone, or a person (login and name), loaded from GitHub on demand for the open form, never persisted.
- **Remembered selector state**: per project, the mode last used, the basic filter set and the last run query (never an unrun draft).
- **Query error**: GitHub's messages for a refused query, or the app's own reason for not sending one.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: With exclude labels `in-progress` and `blocked` set, the list shows no issue carrying either label in 100% of tested repositories and states, and every remaining open issue up to the cap is listed.
- **SC-002**: For every advanced query in the test list (state terms, negated labels, `no:assignee`, `linked:pr`, `OR`, parentheses, `sort:`), the listed issues are the set GitHub's issue search returns for that query in the repository, verified against a scripted GitHub source and, by hand, against github.com for `-label:in-progress state:open`.
- **SC-003**: Every invalid query in the test list shows a message naming the problem (GitHub's own message when GitHub refused it); none shows an empty list.
- **SC-004**: 100% of the scope-escape queries in the test list (`repo:`, `org:`, `user:`, unbalanced parentheses, unclosed quotes) are refused before sending, and no test lists a pull request or another repository's issue.
- **SC-005**: Reopening the form on a project restores its mode, basic filters and last run query in 100% of tested cases, including after an app restart, and never shows another project's filter.
- **SC-006**: A user who has set "exclude `in-progress` and `blocked`" once reaches the list of ready issues on every later opening with zero filter interactions.
- **SC-007**: With the default filter, GitHub issues switched off or the Jira source showing, the selector looks and behaves exactly as before this feature.

## Assumptions

- **Mirrors the Jira selector's controls.** Basic filters sit behind one **Filters** button with a count, an active line and **Clear filters** (1185 D15); the Basic / Advanced switch and run-on-Enter query input follow 1186. Unlike 1186, the search text stays in advanced mode, because the issue asks that free text work on top of the active filter.
- **Unlike 1185, filters are remembered.** The issue asks for a per-project default; the basic filters persist like 1186's mode and query.
- **Include labels combine with AND**, as github.com's label filter does when several labels are chosen (`label:a label:b`); "any of" is reachable in advanced mode.
- **Advanced mode adds no state term.** The issue fixes only `repo:` and `is:issue`; a query lists closed issues unless it says `state:open` (as 1186 adds no status filter).
- **Same results as github.com** means the same set of issues for the same query in the same repository; order matches when the query has a sort qualifier, and otherwise is most recently updated first, as the selector lists today. GitHub's search index can lag a few seconds behind an edit; that lag is GitHub's.
- **"Linked to an open PR"** from the issue's use case is reached in advanced mode (`-linked:pr`); no basic filter is added for it.
- **The filtered listing uses GitHub's issue search**, which returns at most 1,000 results, matching the selector's load cap; the default filter keeps today's listing.
- The GitHub sign-in, request path and timeouts of 034 are reused; no new setting is added.
- A 1,000-character query limit is far above any hand-typed query and keeps the request and the remembered state small.

## Out of Scope

- Saved or named filters, or more than one remembered filter per project.
- Search syntax autocomplete, highlighting, or qualifier suggestions.
- Converting basic filters into an advanced query, or a query back into basic filters.
- Filters on other fields in basic mode (type, project board, reactions, dates, linked pull requests).
- Applying the filter to MCP `create_worktree` or to anything outside the New worktree form.
- GitHub Enterprise hosts (034 FR-002).
