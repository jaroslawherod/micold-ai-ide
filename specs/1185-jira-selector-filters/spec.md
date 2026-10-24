# Feature Specification: Filter the Jira issue selector by reporter, label, component and fix version

**Feature Branch**: `claude/project-thread-xwkotj`

**Created**: 2026-10-24

**Status**: Draft

**Input**: User description: "Spec 1185" — issue #1185, *Filter the Jira issue selector by reporter, label, component and fix version*: the Jira issue selector in the New worktree form can only narrow its list with free text (summary, label, or an issue key), always within the project's not-Done issues. In a large project there is no way to narrow the list to the issues I reported, a component, or the next release's fix version. Add basic filters on Jira's built-in fields — reporter, label, component, fix version — each adding a clause ANDed with the project and not-Done clauses and the free-text search. The choices for each filter come from the connected Jira project. Filter values are escaped the same way the search text is, so a value is only ever a JQL string.

## Context

Feature 533 (`specs/533-jira-issue-worktrees`) added a Jira source to the create-worktree dialog's issue picker: it loads up to 1,000 of the connected project's not-Done issues, most recently updated first, searches them locally by key, summary and label (533 FR-012, the GitHub picker's rule of 034 FR-005), and asks Jira for matches beyond the loaded issues only when the loaded list is incomplete. Feature 1101 added each ticket's status. This feature adds four filters to that Jira picker. Issue #1186 (advanced JQL in the same selector) is a separate feature specified in parallel; free-form JQL is out of scope here.

## Clarifications

### Session 2026-10-24

- Q: Which people does the reporter filter offer — users who can browse the project, only the reporters of the loaded issues, or only "me"? → A: The connected account's own user first (Jira's current-user lookup), then the users who can browse the connected project, found by searching Jira with the name the user types (Jira's "users with browse permission for a project" search); the issue names the project's users as the reporter choices. _(agent-resolved: issue #1185 body "choices … come from the connected Jira project (its components, versions, labels, users)"; spec.md#FR-014 already allows a people search with typed text)_
- Q: Where do label choices come from — Jira's site-wide label list or the labels on the project's issues? → A: The labels carried by the project's not-Done issues the picker has loaded in its unfiltered listing (up to 1,000), offered alphabetically with no extra request; Jira has no per-project label list and its site-wide list holds other projects' labels, which would match nothing here. When that listing is incomplete, the label filter says it offers only the labels of the most recently updated issues. _(agent-resolved: issue #1185 body "choices … come from the connected Jira project"; Jira REST `GET /rest/api/3/label` is site-wide only; specs/533-jira-issue-worktrees/spec.md#FR-011 loads labels with each issue)_
- Q: Once a filter replaces the unfiltered listing, where do the label choices come from? → A: From the unfiltered listing as it loaded when the Jira source was shown; the choices are kept for the open dialog and do not shrink to the filtered list. _(agent-resolved: spec.md#FR-005, #FR-007 — the filtered listing replaces the unfiltered one, so label choices are taken at the unfiltered load)_

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Narrow the Jira list by component or fix version (Priority: P1)

A developer in a large Jira project opens the New worktree form with the Jira source showing, picks the component `Payments` and the fix version `2.4`, and the list shows only the project's not-Done issues that belong to that component and are scheduled for that version. They search "refund" within that narrowed list, pick an issue, and create the worktree as before.

**Why this priority**: Component and fix version are the filters a project's own structure defines and that a long list most needs; the choices come entirely from the project, so this slice delivers the filtering mechanism (choice loading, request with filter clauses, clearing) end to end.

**Independent Test**: With a connected project that has components and versions, choose one component and one fix version; the list holds exactly the not-Done issues Jira returns for the project with both, and clearing the filters restores the unfiltered list.

**Acceptance Scenarios**:

1. **Given** the Jira source is showing and the project has components, **When** the user opens the component filter, **Then** it offers the project's components by name.
2. **Given** the user chose component `Payments`, **When** the list reloads, **Then** it holds only not-Done issues of the project in component `Payments`, most recently updated first.
3. **Given** a component is chosen, **When** the user also chooses fix version `2.4`, **Then** the list holds only issues that match both (filters combine with AND).
4. **Given** filters are chosen, **When** the user types search text, **Then** the results are the filtered issues that match the text by the picker's existing rule (key, summary, label).
5. **Given** filters are chosen, **When** the user clears one filter, **Then** the list reloads without that filter's restriction and keeps the others; **When** the user clears all filters, **Then** the list is the unfiltered not-Done listing again.

---

### User Story 2 - Show only the issues I reported (Priority: P2)

A developer wants to start work on one of the tickets they filed. They choose themselves in the reporter filter and the list shows only the project's not-Done issues they reported.

**Why this priority**: Very common need ("my tickets"), but it depends on knowing the people in the project, which is a different kind of choice from the project's own components and versions.

**Independent Test**: Choose a person in the reporter filter; the list holds exactly the not-Done issues of the project that person reported, and each row's reporter shows that person.

**Acceptance Scenarios**:

1. **Given** the Jira source is showing, **When** the user opens the reporter filter, **Then** the connected account's own user is offered first, labelled as the user themselves, followed by the users who can browse the project whose name matches the text the user types into the filter.
2. **Given** the user chose a reporter, **When** the list reloads, **Then** every listed issue was reported by that person.
3. **Given** two people in the project share a display name, **When** the user chooses one of them, **Then** only that person's issues are listed (the filter identifies the person, not the name).

---

### User Story 3 - Narrow by a label (Priority: P3)

A developer chooses the label `backend` from the label filter rather than typing it into the search box, and the list shows only issues carrying that label, while the search box remains free for summary text.

**Why this priority**: Typing a label into the search already reaches labels partly (it matches one label exactly, ORed with the summary), so this filter refines an existing capability.

**Independent Test**: Choose a label; the list holds exactly the not-Done issues of the project carrying that label; then type a summary word and see the results narrow within the labelled issues.

**Acceptance Scenarios**:

1. **Given** the Jira source is showing, **When** the user opens the label filter, **Then** it offers, alphabetically, the labels carried by the project's not-Done issues loaded in the unfiltered listing.
2. **Given** label `backend` is chosen, **When** the user types "login", **Then** the results are issues carrying `backend` whose key, summary or a label matches "login".

---

### Edge Cases

- **Empty choices**: a project with no components (or no versions, no labels) shows that filter as unavailable with a short reason ("This project has no components"); the other filters and the list keep working.
- **Choices fail to load**: offline, timeout (10 seconds), refused credentials or no access for one filter's choices shows the cause in plain words and a **Retry** on that filter only; the list, the search and the other filters keep working, and the ticket, name and type inputs keep working (as 533 FR-018).
- **No user matches the typed name** in the reporter filter: it says no project user matches and keeps the own user offered.
- **Labels beyond the load cap**: a label carried only by not-Done issues older than the 1,000 loaded is not offered; the label filter says only the most recent issues' labels are offered, and typing the label into the search still reaches it (533 FR-012).
- **The connected account's own user cannot be looked up**: the reporter filter shows the cause and a **Retry**, as for any failed choice load; the other filters and the list keep working.
- **No matching issues**: when the chosen filters (with or without search text) match nothing, the list says no not-Done issue matches the filters and offers to clear them; it never shows the unfiltered list as if it were filtered.
- **A chosen value disappears in Jira** (a component deleted, a version archived) while the dialog is open: the next request returns what Jira returns (possibly nothing, or an error Jira reports for an unknown value); the picker shows the empty state or the error with the filter still visible so the user can clear it.
- **Issue key typed while filters are chosen**: the key search is subject to the filters like any other search; an issue outside the filters is not listed, and the empty state mentions that filters are active.
- **More than 1,000 matching issues**: the filtered listing loads the 1,000 most recently updated matching issues and says only those are shown; search beyond them asks Jira with the same filters (533 FR-012 rule, with the filter clauses added).
- **Filter changed while a request is in flight**: the older request's result is discarded and only the result for the current filters and search text is shown (as 533 FR-017 discards the other source's result).
- **Rapid changes**: changing several filters in a row produces a correct final list for the last combination; no intermediate result is shown after a later one.
- **Filter values with special characters or long names**: a component, version or label containing `"`, `\`, edge spaces or non-Latin letters, or longer than 100 characters, matches exactly that value; it can never change the query's structure. A Jira value holding a control character is not offered.
- **Switching source**: switching to GitHub hides the Jira filters (GitHub issues have no such filters in this feature); switching back to Jira starts with no filters chosen and clears the search text (533 FR-017).
- **Concurrency (Principle II, multi-session)**: two create-worktree dialogs open at once (two windows or projects) hold their own filter choices and lists; choosing a filter in one never changes the other's list or choices.
- **Cross-platform (Principle VI)**: the filters, their choices, keyboard use and requests behave identically on Linux, macOS and Windows; nothing depends on the host platform.
- **Jira integration off or no connection**: no filter is shown and no choice request is made (533 FR-020).

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: While the Jira source is showing, the picker MUST offer four filters — reporter, label, component and fix version — each with no value chosen when the source is first shown.
- **FR-002**: Each filter MUST allow choosing at most one value, and MUST allow clearing its value; a single control MUST clear all chosen filters at once.
- **FR-003**: The component filter MUST offer the connected project's components by name; the fix-version filter MUST offer the project's versions that are not archived, unreleased versions before released ones.
- **FR-004**: The reporter filter MUST offer the connected account's own user first, shown as the user themselves, and then the users with permission to browse the connected project whose name matches the text typed into the filter, as Jira's project user search returns them; it MUST identify the chosen person by their Jira account, never by display name alone.
- **FR-005**: The label filter MUST offer, alphabetically and without duplicates, the labels carried by the project's not-Done issues loaded in the unfiltered listing, taken when that listing loads and kept for the open dialog (choosing filters does not shrink them), without a further request to Jira; when that listing is incomplete (the 1,000-issue cap), the filter MUST say it offers only the labels of the most recently updated issues; while the unfiltered listing is loading or has failed, the filter shows that state instead of choices; when a filter is chosen before that listing arrives, its result still supplies the label choices (it is never shown as the list).
- **FR-006**: A filter's choices that need a request (reporter, component, fix version) MUST be requested only when the user first opens that filter in a dialog showing the Jira source, MUST be held only in the open dialog, and MUST NOT be serialised or persisted (as 533 FR-019); diagnostic and log output MUST NOT contain a reporter choice's name or account (as 533 FR-019 redacts a reporter).
- **FR-007**: With one or more filters chosen, the listing MUST hold only the project's not-Done issues that satisfy every chosen filter (AND), most recently updated first, under the same 1,000-issue load cap and "only the most recent are shown" notice as the unfiltered listing.
- **FR-008**: Choosing, changing or clearing a filter MUST reload the listing from Jira with the current filters and MUST discard the result of any earlier request for other filters or search text.
- **FR-009**: Search text MUST combine with the filters: the shown results MUST be issues that satisfy every chosen filter and match the text by the picker's existing rule (533 FR-012). Search beyond the loaded issues MUST carry the same filters.
- **FR-010**: Each filter value MUST be sent to Jira only as a quoted string value with `\` and `"` escaped, so a value can never alter the structure of the query. A value chosen from Jira's own choices MUST be sent whole: never trimmed or cut to the search text's 100-character limit, so a value of any length, or with `"`, `\`, edge spaces or non-Latin characters, matches exactly that value. A choice whose value contains a control character MUST NOT be offered. The reporter value MUST be the chosen person's Jira account identifier, not their display name.
- **FR-011**: When the chosen filters match no not-Done issue, the picker MUST say so and offer to clear the filters.
- **FR-012**: A failure to load one filter's choices MUST show the cause in plain words with a **Retry** on that filter, and MUST NOT block the listing, the search, the other filters or creating a worktree by hand.
- **FR-013**: A failure of a filtered listing or search MUST be shown as the unfiltered one is (533 FR-018), with the filters left in place so the user can retry or clear them.
- **FR-014**: The statement of what the app sends to Jira (533 FR-005b) MUST also name the chosen filter values; requests for filter choices MUST carry only the project key, the credentials and, for a people search, the text the user typed into it, and MUST go only to the connection's site (533 FR-020).
- **FR-015**: Filters MUST be reachable and operable from the keyboard, and the list MUST show which filters are active while any is chosen.
- **FR-016**: Switching the dialog's source away from Jira MUST hide the filters and drop their values; showing the Jira source again starts with no filter chosen. Filter choices MUST NOT be remembered between openings of the dialog.
- **FR-017**: Picking an issue from a filtered list MUST fill the ticket, name and type exactly as an unfiltered pick does (533 FR-013 to FR-015).

### Key Entities

- **Filter**: one of reporter, label, component, fix version; holds at most one chosen value, lives only in the open dialog.
- **Filter choice**: a value offered by a filter — a component name, a version name (with released/unreleased state), a label, or a person (Jira account plus display name); loaded from the connected Jira site on demand, never persisted.
- **Filtered listing**: the project's not-Done issues that satisfy every chosen filter, subject to the load cap; replaces the unfiltered listing while filters are chosen.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: A user in a project with over 1,000 not-Done issues reaches a list of only the issues of one component and one fix version with at most four interactions (open filter, choose, open filter, choose), without typing.
- **SC-002**: For every combination of the four filters (16 combinations, including none), the listed issues are exactly those Jira returns for the project's not-Done issues with those restrictions, verified against a scripted Jira source.
- **SC-003**: For a test corpus of at least 20 filter values containing `"`, `\`, edge spaces, over 100 characters or non-Latin text, choosing each value lists exactly the issues carrying that value: never an error, never other issues; a Jira value containing a control character is never offered in any filter.
- **SC-004**: When a filter's choices fail to load, the user can still search, pick an issue and create a worktree, in 100% of the failure causes listed in FR-012.
- **SC-005**: After any sequence of filter changes, the list shown corresponds to the last chosen combination in 100% of scripted rapid-change runs (no stale result shown).

## Assumptions

- Single value per filter: the issue names one clause per field (`reporter = …`, `labels = …`, `component = …`, `fixVersion = …`), so each filter takes one value; choosing several values of one field is a later enhancement.
- Filters do not persist: they reset on each opening of the dialog, the same way the search text does; remembering them per project can follow if wanted.
- The not-Done restriction stays: filters narrow the existing not-Done listing; showing Done issues is out of scope.
- Jira Cloud identifies people by account, not name, in queries; the reporter filter therefore carries the account identity of the chosen person.
- Fix-version choices exclude archived versions, as Jira's own pickers do.
- The connection and credentials of feature 533 are reused; no new setting is added.

## Out of Scope

- Free-form or advanced JQL in the selector (issue #1186, a separate feature).
- Filters on other fields (assignee, priority, sprint, status, issue type, custom fields).
- Multiple values per filter, OR between filters, and negated filters.
- Filters for the GitHub source.
- Saving or naming filter combinations, or persisting filters between dialog openings.
