# Feature Specification: Reporter, Labels and a Description Tooltip in the Issue List

**Feature Branch**: `feat/issue-list-reporter-labels-tooltip`

**Created**: 2026-10-02

**Status**: Draft

**Input**: User description: "Implement GitHub issue #518 (https://github.com/jaroslawherod/micold-ai-ide/issues/518): in the new-worktree form's issue list, show the reporter and labels on a second, wrapping line, and show a truncated description in a tooltip after the cursor rests on a row for 3 seconds. Read the issue with `gh issue view 518` for the acceptance criteria and code pointers."

## Clarifications

### Session 2026-10-02

- Q: When the repository has more open issues than the 1,000 loaded, must typing a reporter's login also find that reporter's issues beyond the loaded ones? → A: No. The reporter is searched like a label name is today: among the loaded issues and among the issues the existing search beyond the cap returns for the typed text. No request filtered by author is added, and the existing request is unchanged. _(agent-resolved: specs/034-github-issue-worktree/spec.md#FR-005a and #FR-025; `crates/micold-core/src/github.rs` `search_args` sends only `repo:… is:issue is:open <typed text>`, with no label filter, so label names already behave this way; GitHub issue #518 asks for the reporter to match "as number, title and label already do")_

## User Scenarios & Testing *(mandatory)*

### User Story 1 - See who reported an issue, and its labels, without anything cut off (Priority: P1)

A developer opens the create-worktree form, chooses **GitHub issue** (feature 034) and scans the list for the issue to start on. Today each issue is one line — number, title, then the labels — so a long title pushes the labels out of view, and the row never says who reported the issue. They want each issue to take two lines: the number and title on the first, the reporter and the labels on the second, with any text too long for the row wrapping onto a further line instead of being cut off.

**Why this priority**: It is the core of the request and stands alone: the reporter and the labels are what tell two similarly titled issues apart, and a row that hides its labels makes the label-chosen worktree type (034) a surprise.

**Independent Test**: On a repository with an open issue that has a title longer than the list is wide and three labels, open the create-worktree form, choose "GitHub issue", and confirm the row shows the whole title over several lines and, below it, the reporter's login followed by all three labels.

**Acceptance Scenarios**:

1. **Given** the issue list has loaded, **When** the user looks at any row, **Then** its first line reads `#<number> <title>` and the line below shows the login of the issue's reporter.
2. **Given** an issue has labels, **When** its row is shown, **Then** the labels appear on the second line, after the reporter and visibly separated from it.
3. **Given** an issue has no labels, **When** its row is shown, **Then** the second line shows the reporter alone, with no separator or empty label area.
4. **Given** an issue's title is longer than the row is wide, **When** its row is shown, **Then** the title continues on further lines; no part of it is clipped, replaced by an ellipsis, or drawn outside the list.
5. **Given** an issue's reporter and labels together are longer than the row is wide, **When** its row is shown, **Then** they continue on further lines and every label can be read.
6. **Given** the list holds rows of differing height, **When** the user moves the highlight with Up and Down, **Then** each press moves it by exactly one issue, the highlighted row is wholly visible in the list, and Enter picks it.
7. **Given** the user picked an issue, **When** the list is shown again, **Then** the picked issue's row carries the picked-row marker, whatever its height.
8. **Given** an issue was found by the search beyond the load cap or by typing its number (034 FR-005a), **When** its row is shown, **Then** it has the same two lines — number and title, reporter and labels — as a row from the initial listing.
9. **Given** the form is resized or shown at a different width, **When** the list is redrawn, **Then** each row wraps to the new width and still shows all of its text.

---

### User Story 2 - Find issues by who reported them (Priority: P2)

The developer remembers that a colleague filed the issue but not its title. They type the colleague's GitHub login into the issue search field and the list narrows to that reporter's issues, with the typed text emphasised in the reporter's login the same way it is emphasised in a number, a title or a label.

**Why this priority**: Once the reporter is visible, users expect to search by it; without this the new text in the row is the only text the search ignores. It depends on story 1 and adds to it.

**Independent Test**: On a repository where one login reported two of ten open issues, type that login into the search field and confirm exactly those two issues remain (plus any whose number, title or label contains the text), with the typed text emphasised in the reporter on each.

**Acceptance Scenarios**:

1. **Given** the issue list has loaded, **When** the user types a reporter's login, or part of it, into the search field, **Then** the list narrows to the issues that reporter filed, alongside any issue whose number, title or label matches the same text.
2. **Given** a row is listed because the typed text matches its reporter, **When** the row is shown, **Then** the matching part of the reporter's login is emphasised as matched text is in the title and labels.
3. **Given** the typed text matches a row's title and its reporter, **When** the row is shown, **Then** both matches are emphasised.
4. **Given** the user types the login in a different letter case, **When** the list narrows, **Then** the reporter's issues still match.
5. **Given** the search field is empty, **When** the user reads its hint, **Then** the hint names the reporter among the things the search covers, alongside number, title and label.
6. **Given** the repository has more open issues than the load cap, **When** the user types a reporter's login, **Then** the request to GitHub is the same one any other typed text causes — one search for that text, with no filter by author — and the list shows that reporter's loaded issues plus any returned issue that matches by number, title, label or reporter.

---

### User Story 3 - Peek at an issue's description (Priority: P3)

Two issues have near-identical titles. Rather than opening the browser, the developer rests the cursor on a row; after 3 seconds a small tooltip shows the start of that issue's description. Moving on closes it. While they sweep the cursor down the list or keep it moving, nothing pops up.

**Why this priority**: A convenience on top of a list that is already complete without it. It may also build on the shared tooltip's show delay (feature 036, GitHub issue #430; see Assumptions), which the first two stories do not need.

**Independent Test**: Rest the cursor on the row of an issue with a long description, wait 3 seconds, and confirm a tooltip opens showing only the start of the description, at most three lines, ending in an ellipsis; move the cursor to another row and confirm it closes.

**Acceptance Scenarios**:

1. **Given** the cursor is over a row whose issue has a description, **When** it stays still for 3 seconds, **Then** a tooltip for that row opens.
2. **Given** the cursor is over a row, **When** it has been still for less than 3 seconds, **Then** no tooltip is open.
3. **Given** the cursor keeps moving over the list, within one row or across rows, **When** any amount of time passes, **Then** no tooltip opens; each movement beyond the rest tolerance (FR-016) starts the 3 seconds again.
4. **Given** a row's tooltip is open, **When** the cursor moves to another row, **Then** the tooltip closes, and the other row's tooltip opens only after the cursor has rested on that row for 3 seconds.
5. **Given** a row's tooltip is open, **When** the cursor leaves the list, **Then** the tooltip closes.
6. **Given** a row's tooltip is open, **When** the user reads it, **Then** it holds the description text and nothing else — no number, title, reporter, labels or heading.
7. **Given** an issue's description is empty or only whitespace, **When** the cursor rests on its row for 3 seconds or longer, **Then** no tooltip opens.
8. **Given** an issue's description is longer than fits in three lines of the tooltip, **When** its tooltip opens, **Then** the tooltip shows the start of the description, is at most three lines tall, and ends with an ellipsis.
9. **Given** an issue's description fits in three lines of the tooltip, **When** its tooltip opens, **Then** the whole description is shown with no ellipsis.
10. **Given** a row's tooltip is open, **When** the user clicks the row, **Then** the issue is picked exactly as without a tooltip, and the tooltip closes.
11. **Given** an issue found by the search beyond the load cap or by its typed number, **When** the cursor rests on its row for 3 seconds, **Then** its tooltip opens as for a row from the initial listing.

---

### Edge Cases

- **No labels**: the second line is the reporter alone (story 1, scenario 3).
- **Many labels** (up to the 20 per issue that 034 reads): the second line wraps onto as many lines as it needs; no label is dropped to keep the row short.
- **A title or label with no spaces** (a long identifier, a URL): it breaks inside the word rather than running off the row.
- **Reporter no longer exists**: GitHub reports no author for an issue whose account was deleted. The row shows `ghost`, the name GitHub itself shows for such issues, and typing `ghost` matches it.
- **Reporter is an app or bot**: the row shows the login GitHub reports for it; nothing marks it as a bot.
- **Reporter's issue beyond the load cap**: on a repository with more open issues than the 1,000 loaded, typing a login lists that reporter's loaded issues; one of their issues beyond the cap appears only when the existing search for the typed text returns it (FR-013). The list's notice that only the most recent issues are loaded (034 FR-004) already tells the user the list is incomplete.
- **A label spelled like a login**: the reporter is always first on the second line and separated from the labels, so the two cannot be confused.
- **Empty or whitespace-only description**: no tooltip, however long the cursor rests (story 3, scenario 7).
- **Description that starts with blank lines**: leading blank space is skipped; the tooltip starts at the first text.
- **Very long description**: only its start is ever shown; a description of GitHub's maximum length does not make the tooltip larger than three lines (SC-005), and reading descriptions keeps the list's loading time within SC-008.
- **The list changes under a still cursor** — the user types and the list narrows, a search beyond the cap adds rows, or the list is scrolled by wheel or keyboard: when a different row, or no row, ends up under the cursor, an open tooltip closes and the 3 seconds start again for the row now under the cursor.
- **Cursor resting on the list while the user navigates by keyboard**: the tooltip belongs to the row under the cursor, not to the keyboard-highlighted row. Moving the highlight does not open a tooltip.
- **Row near the edge of the window**: the tooltip opens whole inside the window and does not cover the row it describes (FR-023).
- **No cursor at all** (touch input, keyboard-only use): no tooltip appears; every other part of the row works. The description is not reachable without a pointer in this feature.
- **Form closed, source switched, or a newer load started while the cursor rests on a row**: the wait is abandoned and no tooltip opens for a list that is no longer shown.
- **Loading failure**: an issue list that fails to load shows the failure exactly as 034 describes; this feature adds no new failure, and a response that lacks a reporter or a description for one issue still lists that issue (with `ghost`, and with no tooltip).
- **Several windows** (Principle II): each window's form holds its own list, highlight and tooltip state. Resting the cursor in one window opens nothing in another, and closing one form does not disturb another's list.
- **Same on every OS** (Principle VI): the two lines, the wrapping, the search and the 3-second tooltip behave the same on Linux, macOS and Windows.
- **Existing-branch picker**: it shares the list's look with the issue picker (034) and is unchanged — one line per branch, no tooltip.

## Requirements *(mandatory)*

### Functional Requirements

**Two-line rows**

- **FR-001**: Every row of the issue list MUST show `#<number> <title>` on its first line.
- **FR-002**: Every row MUST show, below the title, the login of the issue's reporter as GitHub reports it. An issue for which GitHub reports no author MUST show `ghost`.
- **FR-003**: Every row MUST show the issue's labels on the same line as the reporter, after it and visibly separated from it. An issue with no labels MUST show the reporter alone, with no separator.
- **FR-004**: Text wider than the row MUST wrap onto further lines within the row — the title under the title, the reporter and labels under the reporter and labels. No text of a row may be clipped, replaced by an ellipsis, or drawn outside the list, at any width the form can take. A word wider than the row MUST break inside the word.
- **FR-005**: The reporter-and-labels line MUST be drawn in smaller text or a lower-emphasis colour than the title line, or both, in the light and the dark theme alike, so the title remains what the eye finds first.
- **FR-006**: Rows from the initial listing, from the search beyond the load cap, and from a typed issue number (034 FR-005a) MUST show the same two lines with the same content rules.
- **FR-007**: With rows of differing height, the list's existing behaviour MUST hold: Up and Down move the highlight by one issue, the highlighted row is scrolled wholly into view, Enter and a click pick the row, and the picked issue's row carries the picked-row marker.
- **FR-008**: Everything a pick does (034 FR-009 to FR-015: ticket, name, type) MUST be unchanged.

**Search by reporter**

- **FR-009**: Typing in the issue search field MUST match an issue when the typed text matches its reporter's login, under the same matching rule (including letter case) the field already applies to number, title and label names.
- **FR-010**: The matched part of a reporter's login MUST be emphasised in the row, as matched text in the number, title and labels is.
- **FR-011**: The search field's hint MUST name the reporter among the things the search covers.
- **FR-012**: An issue returned by the search beyond the load cap MUST be shown when it matches the typed text by number, title, label name or reporter — the rule for loaded issues — and not otherwise (034's "one matching rule" clarification, extended to the reporter).
- **FR-013**: Searching by reporter MUST NOT add a request to GitHub or change the one the search beyond the load cap already makes (034 FR-005a): that request carries the typed text as it does today, with no filter by author. The reporter is therefore matched among the loaded issues and the issues that request returns (FR-012). An issue beyond the load cap that matches the typed text only by its reporter, and that the request does not return, is not listed — as is already the case for an issue beyond the cap that matches only by a label name.
- **FR-014**: The description MUST NOT be searched: typing text that appears only in an issue's description does not match that issue.

**Description tooltip**

- **FR-015**: When the cursor has stayed still over a row for 3 seconds, and the row's issue has a description, a tooltip for that row MUST open. It MUST NOT open sooner.
- **FR-016**: Cursor movement over the list before the tooltip opens MUST start the 3 seconds again, so no tooltip opens while the cursor keeps moving. Only movement within a small rest tolerance — a few pixels from where the cursor came to rest, its exact size a plan decision — counts as staying still; movement to another row always starts the 3 seconds again, however small.
- **FR-017**: An open tooltip MUST close when the cursor moves to another row or leaves the list, when a different row or no row comes to lie under the cursor because the list changed or scrolled, and when the row is picked.
- **FR-018**: The tooltip MUST open after its 3 seconds even when nothing else happens in the application meanwhile, and waiting for it MUST NOT make the window redraw continuously.
- **FR-019**: The tooltip MUST show the description and nothing else.
- **FR-020**: The description shown MUST be taken from the start of the issue's body, skipping leading blank space. An issue whose body is empty or only whitespace MUST have no tooltip.
- **FR-021**: The tooltip MUST be at most three lines of text tall at the shared tooltip's standard width. A description that does not fit MUST be cut and end with an ellipsis; one that fits MUST be shown whole, without an ellipsis.
- **FR-022**: How the body becomes the description text: [NEEDS CLARIFICATION: Issue bodies are written in Markdown and often open with a heading or a template (`## Problem`, checkboxes, links, HTML comments). Should the tooltip show the body's start as readable plain text — markup characters and hidden comments removed, line breaks folded into one flowing paragraph — or the body's first characters exactly as written?]
- **FR-023**: The tooltip MUST NOT cover the row it describes (the rule 029-worktree-tooltip-details FR-013 sets for worktree rows), MUST lie whole inside the window whenever it fits in the window at all, and MUST NOT take keyboard focus or stop a click on the row from picking the issue.
- **FR-024**: Opening a tooltip MUST NOT contact GitHub: the description arrives with the issue, in the requests the list already makes (034 FR-003, FR-005a), and no request is made because a cursor rests on a row.

**Data, privacy and limits**

- **FR-025**: Reporters and descriptions MUST be held only while the form is open and MUST NOT be written to the application's stored files (034 FR-023, SC-006). They MUST NOT be written to the application's logs either — a rule new in this feature.
- **FR-026**: The requests that read issues MUST stay within 034's limits — the same occasions (FR-003), the same load cap (FR-004) and the same time limit for an answer (FR-007) — and MUST carry nothing more about the project than they do today (034 FR-025).
- **FR-027**: 034's loading, empty, failure and retry states MUST be unchanged.

**Consistency, platforms and documentation**

- **FR-028**: The two-line wrapping row and the rest-delay tooltip MUST be provided by the shared component library and used from there by the issue list (Principle VIII); the component showcase MUST show a list with two-line rows of differing height.
- **FR-029**: The existing-branch picker MUST look and behave as it does today.
- **FR-030**: All of the above MUST behave the same on Linux, macOS and Windows.
- **FR-031**: The user guide's "From a GitHub issue" section MUST describe the two lines, searching by reporter, and the description tooltip, in the same change that ships each (Principle VII).

### Key Entities

- **Issue** (034): gains a **reporter** — the login of the account that opened it, or `ghost` when GitHub reports none — and a **description** — the truncated start of its body, possibly empty. Both are read-only and held only while the form is open.
- **Issue row**: one issue as shown in the list — a title line (`#<number> <title>`) and a details line (reporter, then labels), each wrapping, with emphasis on the text the search matched.
- **Description tooltip**: the small panel tied to one row, holding that issue's description; open only while the cursor has rested, and remains, on that row.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: In a list that includes issues with titles of 256 characters (GitHub's maximum) and with 20 labels, 100% of rows show their whole title, their reporter and every label, with no text clipped, at the form's default width and at its narrowest.
- **SC-002**: A user who knows only who reported an issue finds it by typing that login, and on a repository with 500 open issues sees the narrowed list within 1 second of typing (034 SC-003 still holds).
- **SC-003**: A tooltip opens between 3.0 and 3.5 seconds after the cursor comes to rest on a row with a description, and in 0 of 20 trials before 3 seconds.
- **SC-004**: Sweeping the cursor over the list continuously for 10 seconds opens 0 tooltips.
- **SC-005**: No tooltip is ever taller than three lines of text, for descriptions of any length up to GitHub's maximum.
- **SC-006**: Resting the cursor on rows causes 0 requests to GitHub.
- **SC-007**: With rows of one to five lines mixed in the list, every issue can be reached with Up and Down alone, and the highlighted row is wholly visible in 100% of steps.
- **SC-008**: On a repository with 1,000 open issues, the issue list appears in at most 1.5 times the time it takes before this feature, measured on the same repository and connection, and every request for issues is answered within the 10 seconds 034 FR-007 allows.

## Assumptions

- "Reporter" is the GitHub account that opened the issue, shown by its login — not its display name, and not the assignee.
- "A few lines" of description means at most three lines at the shared tooltip's standard width.
- "Stays still" allows the small rest tolerance of FR-016, so that an ordinary hand resting on a mouse can open the tooltip; the tolerance's size is a plan decision.
- Moving the cursor within the row after its tooltip opened leaves the tooltip open; only another row, leaving the list, a pick, or a list change under the cursor closes it (FR-017).
- The description is reachable by pointer only. A keyboard way to read it is out of scope, as are a link to open the issue in a browser, and showing assignees, milestones, comment counts or the issue's age.
- The description is not searched (FR-014), as 034 decided for the issue body.
- The row's exact separator, text sizes and colours are plan decisions within FR-003 and FR-005.
- The existing-branch picker keeps single-line rows; wrapping long branch names there is a separate request.
- **Dependency — feature 036 (GitHub issue #430)**: the shared tooltip gains an optional show delay there. That delay counts time since the cursor entered the control; this feature needs time since the cursor last moved (FR-015, FR-016). The plan decides whether the rest-delay is built on 036's delay once 036 has merged or is added here; stories 1 and 2 do not depend on 036. 036 is being specified in another flow at the time of writing and is not on `main`.
- Reading the reporter and the description adds no new consent: choosing "GitHub issue" already is the user's opt-in to reading the repository's issues from GitHub (034, Principle IV).
