# Feature Specification: Advanced JQL in the Jira Issue Selector

**Feature Branch**: `claude/project-thread-vf2ck6`

**Created**: 2026-10-24

**Status**: Draft

**Input**: User description: "Support advanced JQL in the Jira issue selector (GitHub issue #1186). The Jira issue selector in the New worktree form only builds its own JQL: the project's not-Done issues, narrowed by free text matched against summary, one label or an issue key. Basic field filters (#1185) cover the common cases, but anything else (custom fields, sprints, epics, assignee, priority, status, date ranges, `OR` combinations, a different sort order) cannot be expressed. Add an advanced mode in the selector where the user types a JQL query directly, as in Jira's own advanced search, and the selector lists the issues it returns. The query is scoped to the connected project: it runs as `project = \"<KEY>\" AND (<user JQL>)`, so it cannot list another project's issues. A trailing `ORDER BY` in the user's JQL is kept; without one, `ORDER BY updated DESC` applies. An invalid query shows Jira's error message in the selector instead of an empty list. Ideally the last query used is remembered per project. Alternative considered: the basic filters of #1185, which cover reporter, label, component and fix version only."

## Terms

- **Selector**: the Jira issue picker in the New worktree form (feature 533): the list of the connected project's issues, its search text and, once #1185 ships, its basic field filters.
- **Basic mode**: the selector as it is today (and as #1185 extends it): the app builds the query from the search text and the basic filters.
- **Advanced mode**: the selector state this feature adds, where the user writes the query (JQL) and the app runs it.
- **User query**: the JQL text the user types in advanced mode, possibly ending in an `ORDER BY` clause. While the form is open the input holds the **draft** (what is typed, run or not); the **last run query** is the user query most recently run on this project, and is what is remembered (FR-013).
- **Quoted string**: text between a pair of `"` or a pair of `'`, in which a backslash escapes the next character (so `"a\"b"` is one string). Quotes and parentheses inside a quoted string are text, not syntax.
- **Sort clause**: the word `ORDER` followed by one or more spaces, tabs or newlines and the word `BY`, outside any quoted string, matched ignoring letter case, and everything after it.
- **Scoped query**: what is sent to Jira: `project = "<KEY>" AND (<user query without its ORDER BY>)` followed by the user's `ORDER BY` clause, or `ORDER BY updated DESC` when the user query has none.
- **Connected project**: the Jira project key of the open project's Jira connection (feature 533 FR-006, FR-007).

## User Scenarios & Testing *(mandatory)*

### User Story 1 - List issues with a JQL query (Priority: P1)

A developer wants a worktree for one of the bugs assigned to them in the current sprint. In the selector they switch to **Advanced**, type `assignee = currentUser() AND sprint in openSprints() AND type = Bug`, press Enter, and the selector lists exactly those issues of the connected project. They pick one and the form fills in the ticket, name and type as it does for any picked issue.

**Why this priority**: It is the feature: any criterion Jira can express becomes reachable without leaving the app.

**Independent Test**: With a connected project served by the fake Jira source, switch to Advanced, run a query, and check that the source received the scoped query and that the selector lists the issues the source returned; pick one and check the form's fields.

**Acceptance Scenarios**:

1. **Given** the Jira source is showing in basic mode, **When** the user switches to Advanced, **Then** the search text box (and the basic filters, when #1185 has shipped) are replaced by a single query input, and the list keeps showing what it showed until a query runs.
2. **Given** advanced mode for project `ABC`, **When** the user runs `assignee = currentUser() AND priority = High`, **Then** Jira is asked for `project = "ABC" AND (assignee = currentUser() AND priority = High) ORDER BY updated DESC` and the selector lists the returned issues in Jira's order.
3. **Given** advanced mode, **When** the user runs `status = "In Review" ORDER BY priority DESC, created ASC`, **Then** Jira is asked for `project = "ABC" AND (status = "In Review") ORDER BY priority DESC, created ASC`.
4. **Given** the results are listed, **When** the user picks issue `ABC-42`, **Then** the ticket, name and type are filled exactly as picking `ABC-42` from the basic list does (533 FR-013).
5. **Given** a query whose result includes issues in a Done status, **When** it runs, **Then** those issues are listed too (the user's query, not the app, decides which statuses appear).
6. **Given** a query that matches no issue, **When** it runs, **Then** the selector says no issue matches the query, which is distinct from the error state of User Story 2.
7. **Given** advanced mode with results listed, **When** the user switches back to Basic, **Then** the basic list, search text and basic filters are as they were before switching to Advanced, and the draft is kept for the next switch to Advanced while the form stays open.

---

### User Story 2 - See why a query failed (Priority: P1)

A developer mistypes a field name. Instead of an empty list, the selector shows Jira's own message, for example *Field 'asignee' does not exist or you do not have permission to view it.*, with the query still in the input so they can fix it.

**Why this priority**: Without it, a typo is indistinguishable from "no matching issue", and JQL is easy to mistype.

**Independent Test**: Script the fake source to refuse a query with Jira's error messages; run a query and check that the selector shows those messages and keeps the query text.

**Acceptance Scenarios**:

1. **Given** advanced mode, **When** Jira answers the scoped query with a bad-request refusal carrying one or more error messages, **Then** the selector shows those messages, in Jira's order, in place of the list, and the query input keeps the text as typed.
2. **Given** the error is showing, **When** the user edits the query and runs it again, **Then** the error is replaced by the new result.
3. **Given** a failure that is not a bad-request refusal (site unreachable, timeout, credentials refused, no access to the project, rate limited, server error), **When** it happens in advanced mode, **Then** the selector shows the same notice with **Retry** as basic mode does (533 FR-018), and Retry runs the same query again.
4. **Given** Jira answers with a bad-request refusal but no message, **When** the result arrives, **Then** the selector says the query is not valid JQL, in plain words.
5. **Given** a user query that would escape the project scope (see FR-007), **When** the user runs it, **Then** nothing is sent to Jira and the selector says why the query was not run.

---

### User Story 3 - Come back to the last query (Priority: P2)

A developer runs the same sprint query every time they start work. When they next open the New worktree form on that project and switch to Advanced, the input already holds their last query, and the form reopens in the mode they last used.

**Why this priority**: The issue asks for it as a nice-to-have; it saves retyping a long query but the feature is usable without it.

**Independent Test**: Run a query on project A, close the form, reopen it on A: the input holds that query; open it on project B: the input holds B's own last query, or is empty.

**Acceptance Scenarios**:

1. **Given** the user ran query Q on project A, **When** they reopen the form on A (in this or a later app run), **Then** the form opens in advanced mode with Q in the input and runs Q once, as soon as the Jira source is showing (FR-004a).
2. **Given** project A remembers Q, **When** the user opens the form on project B, **Then** the input holds B's own last query, or is empty when B has none.
3. **Given** the user last used basic mode on project A, **When** they reopen the form on A, **Then** it opens in basic mode, and switching to Advanced shows Q in the input without running it until the user does.
4. **Given** the user clears the query input and runs it, **When** they reopen the form, **Then** the remembered query is empty.
5. **Given** the user ran Q, then typed Q2 without running it, **When** they close and reopen the form, **Then** the input holds Q, not Q2.

---

### Edge Cases

- **Empty query**: running an empty (or whitespace-only) query runs as `project = "<KEY>" ORDER BY updated DESC`: every issue of the project, Done ones included, most recently updated first, by the same rule as any other advanced query (no status filter is added in advanced mode).
- **Query of only a sort clause** (`ORDER BY created ASC`): runs as `project = "<KEY>" ORDER BY created ASC`, with no empty `AND ()`.
- **`ORDER BY` inside a quoted string** (`summary ~ "order by"`, `summary ~ 'order by'`): is part of the condition, not a sort clause. `order  by` with several spaces, or `ORDER` and `BY` split across a newline, is a sort clause.
- **Quotes and escapes**: `summary ~ 'a)'` and `summary ~ "a\"b)"` are balanced (the parenthesis and the escaped quote are inside a string) and run; `summary ~ "abc` (unclosed) is refused.
- **Attempt to escape the scope**: a query whose parentheses do not balance outside quoted strings (`x = 1) OR project = OTHER OR (x = 1`) or that has an unclosed quote is not sent (FR-007); and any issue Jira returns whose key is not of the connected project is dropped from the list (FR-008).
- **Moved issues**: an issue moved into the connected project carries a key of that project in Jira's answer and is listed; one moved out does not match the scope.
- **A query naming the same project** (`project = ABC AND ...`): runs as written inside the scope; the result is the same.
- **A query naming another project** (`project = XYZ`): runs; the scope makes the result empty, and the selector says no issue matches.
- **Very long query**: longer than 2,000 characters is refused before sending, with the limit in the message.
- **Control characters or a pasted newline**: newlines and tabs count as spaces; other control characters are removed before the query is run and remembered.
- **More than 1,000 results**: the 1,000 first in the query's order are listed, and the selector says the list is incomplete, as basic mode does at its cap (533 edge case).
- **Running a new query while one is in flight**: the earlier result is discarded; only the latest query's result or error is shown. Switching mode, switching source to GitHub, closing the form, turning the integration off or removing the connection discards an in-flight result (533 FR-005, FR-017).
- **Two windows on the same project**: each window's form runs its own query; the remembered query is the one run last in either window (last write wins). A window whose form is open does not change its input when the other window runs a query.
- **Remembered query that Jira now refuses** (a custom field was removed): reopening shows Jira's error (User Story 2), with the query in the input to fix.
- **Remembered data unreadable or missing**: the form opens in basic mode with an empty query; nothing else is affected.
- **Integration off, project not connected, or GitHub source chosen**: no advanced mode control is shown and no query is run; a remembered query is kept, not deleted.
- **Linux, macOS and Windows**: the mode switch, query input, Enter-to-run and error display behave the same on each; the remembered query lives in the per-user data location each platform already uses for the Jira connection.

## Requirements *(mandatory)*

### Mode

- **FR-001**: With the Jira source showing, the selector MUST offer a switch between basic mode and advanced mode. The GitHub source MUST be unchanged and show no such switch.
- **FR-002**: In advanced mode the selector MUST show one query input in place of the search text and the basic field filters, and MUST NOT apply the search text or any basic filter to the query or to the listed issues.
- **FR-003**: Switching from advanced back to basic MUST restore the basic list, search text and basic filters as they were when the user left basic mode; switching to advanced MUST show the draft, which starts as the project's last run query when the form opens and keeps any unrun edits for as long as the form stays open.

### Running a query

- **FR-004**: A query MUST run when the user presses Enter in the query input or activates the selector's run control, and MUST NOT run on each keystroke.
- **FR-004a**: When the form opens in advanced mode (FR-013) with a non-empty last run query, the selector MUST run that query once as soon as the Jira source is showing, as if the user had run it; switching to advanced mode within an open form MUST NOT run anything by itself.
- **FR-005**: The app MUST send Jira the scoped query: `project = "<KEY>" AND (<condition>)` where `<condition>` is the user query up to its first `ORDER BY` outside a quoted string, followed by the user query's `ORDER BY` clause when it has one, else by `ORDER BY updated DESC`. With an empty condition (an empty user query, or one that is only a sort clause) the scoped query MUST be `project = "<KEY>"` followed by the sort clause. Quoted strings and the sort clause are as defined in Terms.
- **FR-006**: The selector MUST list the issues Jira returns in Jira's order, up to 1,000, in the same row form as basic mode (key, status, summary, labels, reporter and description tooltip), and MUST say when more issues match than were loaded.
- **FR-007**: Before sending, the app MUST refuse a user query that has an unclosed quoted string, parentheses that do not balance outside quoted strings, or more than 2,000 characters, and MUST show the reason in the selector instead of a list. A refused query MUST NOT be sent.
- **FR-008**: The selector MUST NOT list an issue whose key is not of the connected project, whatever the query or Jira's answer.
- **FR-009**: Picking an issue from an advanced-mode list MUST fill the form exactly as picking the same issue in basic mode does (533 FR-013, FR-015).

### Errors

- **FR-010**: When Jira answers a scoped query with a bad-request refusal (HTTP 400), the selector MUST show Jira's error messages in place of the list, each cleaned of control characters and cut to a readable length, and MUST keep the query text in the input. With no message from Jira it MUST say in plain words that the query is not valid JQL.
- **FR-011**: Failures other than a bad-request refusal (unreachable, timeout, credentials refused, not found or no access, rate limited, any other status) MUST show the same notices and **Retry** as basic mode (533 FR-018); Retry MUST re-run the same scoped query.
- **FR-012**: An empty result MUST be shown as "no issue matches this query", distinct from an error.

### Remembering

- **FR-013**: The app MUST remember, per project, the last run query and the mode last used, in the user's local per-user data (not in a project file or `settings.json`), and MUST restore both when the form next opens on that project, in this or a later app run. An unrun draft MUST NOT be remembered.
- **FR-014**: A remembered query MUST be kept when the integration is turned off or the source is switched to GitHub, and MUST be removed when the project's Jira connection is removed.

### Privacy and scope of requests

- **FR-015**: Requests MUST follow 533 FR-020, widened by one item: in advanced mode a request carries the scoped query, including the run of FR-004a. The selector's notice of what is sent to the Jira site (533 FR-005b) MUST name the query.
- **FR-016**: The user query MUST NOT be written to a log line, a notice other than the selector's own, or a session's history.
- **FR-017**: The user guide MUST document advanced mode: the switch, how the query is scoped and sorted, the length and balance rules, error display, and that the last query is remembered per project.

### Key Entities

- **User query**: the text typed in advanced mode; at most 2,000 characters; split into a condition and an optional `ORDER BY` clause.
- **Scoped query**: the condition wrapped in the project scope plus the sort clause; the only JQL sent in advanced mode.
- **Remembered selector state**: per project: the mode last used (basic or advanced) and the last run query (never an unrun draft).
- **Query error**: Jira's messages for a refused query, or the app's own reason for refusing to send one.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: A user can list the issues of any JQL criterion Jira supports (custom field, sprint, epic, assignee, priority, status, date range, `OR`, any sort order) and create a worktree from one of them without leaving the app, in one query, one pick and one confirm.
- **SC-002**: 100% of the scope-escape queries in the test list (unbalanced parentheses, unclosed quotes, `OR` across projects) either are refused before sending or list no issue of another project.
- **SC-003**: Every invalid query in the test list shows a message naming the problem; none shows an empty list.
- **SC-004**: Reopening the form on a project restores its last query and mode in 100% of tested cases, including after an app restart, and never shows another project's query.
- **SC-005**: With the integration off or the GitHub source chosen, the selector looks and behaves exactly as before this feature.

## Assumptions

- **Composition with #1185 (basic field filters), specified in parallel.** Advanced mode is Jira's basic/advanced toggle: while it is active, the search text and every basic filter of #1185 are hidden and not applied; switching back restores them untouched. This feature does not implement #1185's filters. If #1185 ships first, FR-002 and FR-003 cover its filters; if this feature ships first, they cover the search text alone and #1185 must leave advanced mode's query untouched.
- **No status filter in advanced mode.** The issue's scope is `project = "<KEY>" AND (<user JQL>)`, so a query can list Done issues; the user adds `statusCategory != Done` when they want it.
- **Basic-to-advanced translation is not done.** Switching to advanced does not convert the current search text or basic filters into JQL (Jira does; it can follow).
- **Jira Cloud only**, through the connection, token and request path of feature 533; this feature adds no setting.
- The app does not parse or validate JQL beyond FR-007; Jira is the judge of a query's validity.
- A 2,000-character limit is far above any hand-typed query and keeps the request and the remembered state small.

## Out of Scope

- The basic field filters (#1185).
- JQL autocomplete, syntax highlighting, field or value suggestions.
- Saved filters or a history of more than the last query; Jira's own saved filters.
- Converting basic-mode criteria into JQL, or JQL back into basic criteria.
- Advanced queries for the GitHub source.
- Writing to Jira; Jira Data Center and Server.
