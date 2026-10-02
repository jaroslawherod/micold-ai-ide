# Feature Specification: Pull Request and Check Status for Each Worktree

**Feature Branch**: `feat/worktree-pr-ci-status`

**Created**: 2026-10-02

**Status**: Draft

**Input**: User description: "Implement GitHub issue #486 (https://github.com/jaroslawherod/micold-ai-ide/issues/486): Show pull request and CI status for each worktree. For each worktree whose branch has a pull request, show a small PR indicator in the sidebar: open, draft, merged or closed, plus a combined check status (pending, passing, failing). The worktree tooltip shows the PR number, title, review state and a link that opens it in the browser. A merged PR suggests removing the worktree. Data comes from the `gh` CLI, refreshed on a modest interval and on demand, and only when `gh` is installed and signed in. Acceptance criteria: no indicator and no error when `gh` is missing or the repository has no GitHub remote; refreshing never blocks the UI and respects GitHub rate limits; status parsing lives in the render-free core with tests against recorded `gh` output."

## Clarifications

### Session 2026-10-02

- Q: Is the Settings switch for pull request status off until the user turns it on, or on from the start whenever the GitHub tooling is installed and signed in? → A: Off until the user turns it on. Being signed in to the GitHub tooling is a precondition, not consent; the switch, which states what is read, how often and what is sent (FR-029), is the explicit, informed opt-in. Turning it on reads the status of every project open in a window at once. _(agent-resolved: .specify/memory/constitution.md#IV. Local-First Storage — "Nothing is transmitted off-device without the user's explicit, informed opt-in"; specs/034-github-issue-worktree/spec.md#Assumptions — a choice the user makes in the application is what counts as the opt-in, and FR-003 forbids contacting GitHub in the background without one)_
- Q: How does the user act on the removal suggestion? → A: With the worktree's existing **Delete** action in the row's right-click menu. The suggestion is a passive mark and a tooltip line; it adds no button, menu entry or dialog of its own. _(agent-resolved: specs/040-worktree-pr-ci-status/spec.md#Assumptions — "a passive mark and a tooltip line that lead to the existing delete confirmation"; specs/008-worktree-sidebar-refinement/spec.md#User Story 2 — Delete lives in the worktree's right-click menu)_

## User Scenarios & Testing *(mandatory)*

### User Story 1 - See at a glance which worktrees have a pull request, and whether its checks pass (Priority: P1)

A developer has several worktrees open in the sidebar, some started from GitHub issues (feature 034). For three of them a pull request already exists. Today the sidebar says nothing about that, so they switch to the browser to learn whether a pull request exists, whether its checks are green, and whether it was merged. They want each worktree row to carry a small indicator that answers this: the pull request's state — open, draft, merged or closed — and, for a pull request that is still open or a draft, one combined result of its checks — pending, passing or failing. A worktree without a pull request looks exactly as it does today, and so does every worktree when GitHub cannot be read.

**Why this priority**: It is the core of the request and stands alone: the indicator alone removes most trips to the browser. Everything else in this feature adds detail to it.

**Independent Test**: With pull request status turned on in Settings, in a project whose repository is on GitHub, with one worktree whose branch has an open pull request with failing checks, one whose branch has a merged pull request, and one whose branch has no pull request, open the project and confirm the first row shows "open" with "failing", the second shows "merged", and the third shows no indicator.

**Acceptance Scenarios**:

1. **Given** a worktree's branch has an open pull request, **When** the pull request status has been read, **Then** the worktree's row shows a pull request indicator in the "open" state.
2. **Given** a worktree's branch has a draft pull request, **When** the status has been read, **Then** the indicator shows "draft", distinguishable from "open".
3. **Given** a worktree's branch has a merged pull request and no open one, **When** the status has been read, **Then** the indicator shows "merged".
4. **Given** a worktree's branch has a pull request that was closed without merging and no open one, **When** the status has been read, **Then** the indicator shows "closed".
5. **Given** an open or draft pull request has at least one check that failed, **When** its row is shown, **Then** the indicator shows the check status "failing", whatever the other checks report.
6. **Given** an open or draft pull request has no failed check and at least one check still queued or running, **When** its row is shown, **Then** the indicator shows "pending".
7. **Given** an open or draft pull request whose checks have all finished without a failure, **When** its row is shown, **Then** the indicator shows "passing".
8. **Given** an open or draft pull request with no checks at all, **When** its row is shown, **Then** the indicator shows the pull request state alone, with no check status.
9. **Given** a worktree's branch has no pull request, **When** the status has been read, **Then** its row shows no indicator and looks as it does today.
10. **Given** the GitHub tooling is not installed, or the user is not signed in to GitHub, **When** the project is open, **Then** no row shows an indicator, and no error, notice or dialog about pull requests appears.
11. **Given** the project's repository has no GitHub remote, **When** the project is open, **Then** no row shows an indicator, no error appears, and nothing is sent to GitHub.
12. **Given** the status is being read, **When** the user clicks, types, scrolls or switches sessions, **Then** the application responds as it does when nothing is being read.
13. **Given** the indicator's states, **When** the user sees them in the light and the dark theme, **Then** each pull request state and each check status can be told apart by its shape or symbol, not by colour alone.

---

### User Story 2 - Read the pull request's details and open it in the browser (Priority: P2)

The developer sees "failing" on a worktree and wants to know which pull request it is and whether it has been reviewed. They hover the row: the worktree's tooltip (feature 029-worktree-tooltip-details) now also names the pull request — its number and title, its state and check status in words, and its review state — and they can open the pull request in their browser without typing its address.

**Why this priority**: The indicator says that something needs attention; the tooltip and the way to the browser say what, and get the user there. It depends on story 1's data and adds no new reading of GitHub.

**Independent Test**: Hover the row of a worktree whose branch has an approved open pull request, confirm the tooltip names its number, title, state, check status and "approved", then open the pull request from the application and confirm the browser shows that pull request.

**Acceptance Scenarios**:

1. **Given** a worktree's row shows a pull request indicator, **When** the user hovers the row, **Then** the tooltip shows, each on its own labelled line beside the lines it already has, the pull request's number and title, its state (open, draft, merged or closed), and its check status when it has one.
2. **Given** GitHub reports a review decision for the pull request — approved, changes requested, or review required — **When** the tooltip is shown, **Then** it states that review state in words.
3. **Given** GitHub reports no review decision for the pull request, **When** the tooltip is shown, **Then** it has no review line.
4. **Given** a worktree has no pull request indicator, **When** the user hovers its row, **Then** the tooltip is exactly as it is today, with no pull request lines and no placeholder.
5. **Given** a worktree's row shows a pull request indicator, **When** the user opens the pull request from the application, **Then** the pull request's page opens in the user's default browser, and the application's own state — selection, sessions, the sidebar — is unchanged.
6. **Given** a pull request title longer than the tooltip is wide, **When** the tooltip is shown, **Then** the title wraps or is shortened within the tooltip's bounded width (029 FR-009), and the number stays readable.
7. **Given** the user hovers a row, **When** the tooltip opens, **Then** nothing is sent to GitHub and the disk is not read: the tooltip shows what the last reading found (029 FR-012).
8. **Given** the shown status was read longer ago than two refresh intervals, **When** the tooltip is shown, **Then** it states how long ago the status was read.

---

### User Story 3 - A merged pull request suggests removing the worktree (Priority: P3)

The developer's pull request was merged. The worktree that produced it is now clutter, but they only notice weeks later. They want the application to point it out: the row of a worktree whose pull request was merged, and which holds no newer work, says that the worktree can be removed, and removing it takes the route the application already offers.

**Why this priority**: Housekeeping on top of the first two stories. It saves a later clean-up but nothing is blocked without it.

**Independent Test**: With a worktree whose branch's pull request was merged and whose branch has no commits after the merged ones, confirm the row and its tooltip suggest removing the worktree, choose **Delete** from the row's right-click menu, and confirm the existing delete confirmation appears and nothing is removed until it is confirmed.

**Acceptance Scenarios**:

1. **Given** a worktree's pull request is merged and the worktree's branch has no commits beyond those the pull request merged, **When** the row is shown, **Then** the row is marked as removable and its tooltip states that the pull request was merged and the worktree can be removed.
2. **Given** such a suggestion is shown, **When** the user chooses **Delete** from the row's right-click menu, **Then** the application's existing delete confirmation opens, as it does today, and nothing is removed unless the user confirms it.
3. **Given** a worktree's pull request is merged, **When** any amount of time passes, **Then** the application never removes the worktree, its sessions or its branch on its own.
4. **Given** a worktree's pull request is merged but its branch has commits made after the merged ones, **When** the row is shown, **Then** the indicator shows "merged" and no removal is suggested.
5. **Given** a worktree's pull request was closed without merging, **When** the row is shown, **Then** no removal is suggested.
6. **Given** a worktree's branch has a merged pull request and a newer open one, **When** the row is shown, **Then** the indicator shows the open pull request and no removal is suggested.

---

### User Story 4 - The status stays current without asking, and can be refreshed when wanted (Priority: P4)

The developer pushes a fix and goes back to work. Without doing anything, a few minutes later the worktree's indicator turns from "failing" to "pending" and then "passing". When they do not want to wait, they press the sidebar's existing **refresh** button and the indicators catch up at once. Through all of it the application stays well inside GitHub's request limits.

**Why this priority**: The first reading on opening a project already delivers stories 1 to 3. Keeping it current is what makes the indicator trustworthy over a working day, and it carries the feature's limits.

**Independent Test**: With a pull request whose checks are running, leave the application untouched until the checks finish and confirm the indicator changes to "passing" within 6 minutes; then change the pull request on GitHub, press refresh, and confirm the indicator follows within 10 seconds.

**Acceptance Scenarios**:

1. **Given** a project is open in a window and pull request status is enabled, **When** the project is opened, **Then** the status is read once without the user asking.
2. **Given** a project stays open in a window, **When** 5 minutes have passed since the last reading, **Then** the status is read again without the user asking.
3. **Given** a pull request's state or checks changed on GitHub, **When** the next reading arrives, **Then** the row's indicator and tooltip show the new status, and the sidebar's selection, expansion and scroll position are unchanged.
4. **Given** the user presses the sidebar's refresh button, **When** the worktree list is refreshed, **Then** the pull request status of that project is read again as part of it.
5. **Given** a reading is already under way, **When** the interval elapses or the user presses refresh, **Then** no second reading is started alongside it; a refresh pressed meanwhile is answered by one further reading that starts when the first ends, unless the request limit holds readings back (scenario 7).
6. **Given** the user presses refresh, **When** the worktree list has been refreshed, **Then** the refresh control returns to idle and its "Worktree list refreshed." notice appears as today, without waiting for the pull request reading.
7. **Given** GitHub answers that the request limit was reached, **When** the next interval elapses, **Then** no automatic reading is made until the time GitHub gives for the limit to reset, and the indicators keep showing the last status read.
8. **Given** a reading fails for a passing reason — no network, no answer within 10 seconds, GitHub's request limit, or an answer that cannot be understood — **When** it fails, **Then** the rows keep the last status read, no error, notice or dialog appears, and the next reading is tried at the next interval.
9. **Given** a reading finds that pull requests cannot be read at all — the GitHub tooling is no longer installed, the user is no longer signed in, the sign-in cannot see the repository, or the repository no longer has a GitHub remote — **When** it finds this, **Then** every indicator, pull request tooltip line and removal suggestion of that project is removed, no error, notice or dialog appears, and the next reading is tried at the next interval.
10. **Given** no window shows a project, **When** any amount of time passes, **Then** no pull request status is read for that project.
11. **Given** pull request status is off — as it is until the user turns it on in Settings, and after they turn it off again — **When** a project is open, **Then** no indicator is shown and nothing is sent to GitHub for it.
12. **Given** pull request status is off and a project is open in a window, **When** the user turns it on in Settings, **Then** the status of that project is read at once, without waiting for the interval.

---

### Edge Cases

- **No pull request, no remote, no tooling, no sign-in**: no indicator and no error (story 1, scenarios 9 to 11). A project with no worktrees makes no request.
- **Worktree with no branch** (a detached checkout): no indicator; it is not looked up.
- **Branch never pushed**: no pull request can exist for it; no indicator.
- **Several pull requests for one branch**: one is shown. An open (or draft) one wins over merged and closed ones; among several of the same kind, the most recently created wins (FR-004).
- **Pull request reopened, or a merged branch reused for a new pull request**: the row follows the rule above at the next reading — it shows the open one, and a removal suggestion disappears.
- **Same branch name in someone else's fork**: a pull request whose branch merely has the same name but lives in another repository is not the worktree's pull request (FR-005).
- **Branch renamed, or the worktree switched to another branch, outside the application**: the row keeps the branch the sidebar last listed, and its pull request, until the worktree list is refreshed (the listing stays manual, 029-refresh-worktrees-list FR-012). The refresh control updates the listing and then reads pull request status for the branches it now shows, so the row and its tooltip's branch line never name one branch while the indicator describes another (FR-018a). A listed branch that no longer exists has no indicator.
- **Worktree removed or project closed while a reading is under way**: the answer is dropped for rows that no longer exist; nothing reappears and no error is shown.
- **Worktree created between readings**: one created in the application is listed at once and gains its indicator at the next reading, automatic or on demand. One created by an agent or by hand is not in the sidebar until the list is refreshed; that refresh also reads its pull request status.
- **Checks that are skipped, neutral or cancelled**: skipped and neutral checks count as finished without failure; a cancelled, timed-out or action-required check counts as failed (FR-008).
- **Hundreds of checks, or a very long title**: the row shows one combined status however many checks there are; the tooltip stays within its bounded width.
- **Very many worktrees**: every listed worktree with a branch is covered, however many there are. The request budget of SC-006 is promised for up to 50 worktrees in a project; beyond 50 the coverage stays complete and the number of requests may grow.
- **GitHub does not answer**: the reading is abandoned after 10 seconds; the application stays responsive throughout and the rows keep what they had (story 4, scenario 8).
- **An answer the application cannot understand** (the tooling changed its output, or the answer is cut short): treated as a passing failure — the rows keep what they had, and nothing half-read is shown.
- **Rate limit reached**: automatic readings pause until the limit resets; nothing is shown as an error. A reading the user asks for with the refresh button during the pause is not sent either.
- **Stale status**: when the last successful reading is older than two intervals, the indicator is drawn in a lower-emphasis form and the tooltip says how old it is, so an out-of-date "passing" is not mistaken for a current one (FR-019).
- **Sign-in lost, tooling removed, access withdrawn or GitHub remote removed while the application runs**: the next reading finds that pull requests cannot be read at all, and the indicators disappear, as if the project had been opened that way; no error is shown (FR-025). This differs from a passing failure — no network, no answer, the request limit — which keeps the last status and lets it age to stale (FR-019).
- **First start, or the switch never touched**: pull request status is off; no indicator appears and nothing is sent to GitHub, even when the GitHub tooling is installed and signed in (FR-030).
- **Offline start**: every existing function works; no indicator appears until a reading succeeds (Principle IV).
- **Application restarted**: status is not stored; rows start without indicators and gain them at the first reading.
- **Several windows** (Principle II): windows showing the same project show the same status, and one reading serves them all — a second window adds no requests. Windows showing different projects read and show their own project's status only; pressing refresh in one window reads that window's project only.
- **Several sessions in one worktree**: the indicator belongs to the worktree row, not to its sessions; session rows are unchanged.
- **The "Default" entry**: it is not a worktree and shows no indicator, no pull request lines and no removal suggestion; its tooltip keeps its fixed wording (029 FR-011).
- **Worktrees made by agents and worktrees included from elsewhere**: they are worktrees with branches and get the indicator like any other.
- **Narrow sidebar**: the indicator keeps its size and place; the worktree's name gives way, as it does today for the row's other marks.
- **Same on every OS** (Principle VI): the indicator, the tooltip lines, opening the browser and the refresh behaviour are the same on Linux, macOS and Windows.

## Requirements *(mandatory)*

### Functional Requirements

**Indicator**

- **FR-001**: The row of every worktree whose branch has a pull request (as FR-005 and FR-006 define it) MUST show a pull request indicator. The row of a worktree without one MUST look as it does today.
- **FR-002**: The indicator MUST show exactly one pull request state: open, draft, merged or closed (closed meaning closed without merging).
- **FR-003**: For an open or draft pull request that has checks, the indicator MUST also show one combined check status: failing, pending or passing. For a merged or closed pull request, and for one with no checks, no check status is shown.
- **FR-004**: When a branch has several pull requests, the one shown MUST be an open or draft one if any exists, otherwise the most recently created; among several open ones, the most recently created.
- **FR-005**: The project's GitHub repository MUST be determined by the rule feature 034 uses for issues (034 FR-002: github.com only). A pull request counts as a branch's pull request only when its source branch has the worktree's branch name and lives in that repository.
- **FR-006**: Pull requests opened from a fork: [NEEDS CLARIFICATION: When the project's GitHub remote is the user's fork and the pull request lives in the upstream repository — the usual open-source arrangement — must the worktree show that pull request too? Doing so means finding the upstream repository and reading pull requests there, matched by the fork owner and branch; not doing so means such worktrees show no indicator.]
- **FR-007**: A worktree with no branch, and the "Default" entry, MUST show no indicator.
- **FR-008**: The combined check status MUST be: failing when any check failed, was cancelled, timed out or needs action; otherwise pending when any check is queued or running; otherwise passing. Skipped and neutral checks count as finished without failure.
- **FR-009**: Each pull request state and each check status MUST be distinguishable without relying on colour alone, in the light and the dark theme, and the indicator MUST keep its size at every sidebar width the application allows.

**Tooltip and opening the pull request**

- **FR-010**: The tooltip of a worktree that shows an indicator MUST add, each on its own labelled line (029 FR-008) and after the lines it has today: the pull request's number and title; its state; its check status, when it has one; and its review state — approved, changes requested or review required — when GitHub reports one.
- **FR-011**: The tooltip of a worktree without an indicator MUST be unchanged, and the pull request lines MUST respect the tooltip's bounded width (029 FR-009) and MUST NOT make it cover its row (029 FR-013).
- **FR-012**: Showing the tooltip MUST NOT contact GitHub or read the disk (029 FR-012).
- **FR-013**: The user MUST be able to open the shown pull request's page in their default browser from the application. How: [NEEDS CLARIFICATION: The issue asks for a link in the tooltip, but a worktree tooltip closes when the cursor leaves the row (029 FR-010), so a link inside it cannot be reached. Should the pull request open by clicking the indicator on the row, by an entry in the worktree's right-click menu, by both, or should the tooltip be changed to stay open while the cursor moves into it?]
- **FR-014**: Opening the pull request MUST NOT change the application's selection, sessions or sidebar, and the address opened MUST be the one GitHub reported for that pull request.

**Merged pull requests**

- **FR-015**: When the shown pull request is merged and the worktree's branch has no commits beyond those the pull request merged, the row MUST be marked as removable and the tooltip MUST state that the pull request was merged and the worktree can be removed.
- **FR-016**: The suggestion MUST NOT add a control of its own: the user acts on it with the worktree's existing **Delete** action in the row's right-click menu, which MUST open the application's existing delete confirmation for that worktree, with its existing choices and wording. The application MUST NOT remove a worktree, a session or a branch because a pull request was merged without that confirmation.
- **FR-017**: No removal MUST be suggested for a closed pull request, for a merged one whose branch has later commits, or while the branch has an open pull request.

**Refreshing**

- **FR-018**: While pull request status is enabled, the status of a project MUST be read: once when the project is opened in a window; once when the switch is turned on, for every project a window shows (FR-030); again every 5 minutes while at least one window shows the project; and whenever the user refreshes the worktree list with the sidebar's refresh control (029-refresh-worktrees-list). It MUST NOT be read for a project no window shows.
- **FR-018a**: A reading MUST cover the worktrees the sidebar lists for the project, each by the branch the listing shows for it; it MUST NOT change the listing. A refresh with the sidebar's control MUST update the listing first and read pull request status for the branches the updated listing shows. Whether a branch has commits beyond its merged pull request (FR-015) MUST be worked out anew at every reading, from that branch as the repository holds it then.
- **FR-019**: A row MUST keep the last status read until a newer reading replaces it or FR-025 removes it. A reading that fails for a passing reason — no network, no answer within 10 seconds, the request limit, or an answer that cannot be understood — MUST leave every row's status as it was. When the last successful reading is older than two intervals (10 minutes), the indicator MUST be drawn in a lower-emphasis form and the tooltip MUST state how long ago the status was read.
- **FR-020**: A new reading MUST NOT disturb the sidebar: selection, expansion, scroll position, filters and running sessions stay as they are; only indicators, tooltip lines and removal suggestions change.
- **FR-021**: The application MUST stay responsive while a reading is under way, including when GitHub is slow or does not answer. A reading with no answer within 10 seconds MUST be abandoned.
- **FR-022**: At most one reading per project MUST be under way at a time, however many windows show the project and however often refresh is pressed. Refreshes pressed while a reading is under way MUST together cause exactly one further reading, started when that one ends, unless FR-024 holds readings back.
- **FR-023**: A reading MUST cover every listed worktree that has a branch, however many the project has. The number of requests a reading sends to GitHub MUST NOT grow with the number of checks, and automatic readings MUST stay within the budget of SC-006 for up to 50 worktrees in a project.
- **FR-024**: When GitHub answers that the request limit was reached, no further reading — automatic or on demand — MUST be sent for that sign-in until the time GitHub gives for the limit to reset.

**Staying quiet**

- **FR-025**: When a reading finds that pull requests cannot be read at all — the GitHub tooling is not installed, the user is not signed in, the sign-in cannot see the repository, or the repository has no GitHub remote — the project MUST show no indicator, no pull request tooltip line and no removal suggestion, including any shown before that reading. Neither this nor a passing failure (FR-019) MUST produce an error, notice or dialog about pull requests, and every other function of the application MUST work as it does today.
- **FR-026**: When the repository has no GitHub remote, or pull request status is turned off, nothing MUST be sent to GitHub by this feature.
- **FR-027**: A failed on-demand refresh of the worktree list keeps its own existing error reporting (029-refresh-worktrees-list FR-008); a failed pull request reading MUST NOT add to it, and MUST NOT make an otherwise successful list refresh report a failure. The refresh control's busy state and its notice MUST follow the list refresh alone and MUST NOT wait for the pull request reading.

**Consent, privacy and storage**

- **FR-028**: The application MUST use the user's existing GitHub sign-in on the machine, MUST NOT ask for, store or display GitHub credentials, and MUST NOT fall back to anonymous access (034 FR-022).
- **FR-029**: Settings MUST offer one switch that turns pull request status on or off for the application, stating that it reads pull requests of the open project's repository from GitHub in the background, how often, and what is sent. Turning it off MUST remove every indicator at once and stop all readings.
- **FR-030**: The switch MUST be off until the user turns it on; the GitHub tooling being installed and signed in MUST NOT turn it on. While it is off, the application MUST show no indicator and send nothing to GitHub for this feature (FR-026). Turning it on MUST start a reading at once for every project a window shows. The user's choice MUST be kept across restarts.
- **FR-031**: Requests MUST carry nothing about the project beyond its GitHub repository's identity and the names of its worktrees' branches.
- **FR-032**: Pull request status MUST be held only while the application runs, MUST NOT be written to the application's stored files, and pull request titles and addresses MUST NOT be written to its logs.

**Consistency, platforms and documentation**

- **FR-033**: The indicator MUST be provided by the shared component library and used from there by the sidebar, and the component showcase MUST show it in every pull request state and check status, current and stale (Principle VIII).
- **FR-034**: All of the above MUST behave the same on Linux, macOS and Windows (Principle VI).
- **FR-035**: The user guide's worktree chapter MUST describe the indicator and its states, the tooltip lines, opening the pull request, the removal suggestion, when GitHub is contacted and what is sent, and the Settings switch, in the same change that ships each (Principle VII).

### Key Entities

- **Pull request status**: what the application knows about one worktree's pull request — its number, title, address, state (open, draft, merged, closed), combined check status (none, pending, passing, failing), review state (none, approved, changes requested, review required), whether the worktree's branch has commits beyond it, and when it was read. Read-only, held in memory only.
- **Pull request indicator**: the small mark on a worktree row that shows the state and the check status, in a current or a stale form.
- **Reading**: one refresh of a project's pull request status — started by opening the project, by the interval or by the refresh control — that succeeds, fails quietly, or is held back by the request limit.
- **Removal suggestion**: the mark and tooltip line on a worktree whose merged pull request holds all of the branch's work; it leads to the existing delete confirmation.
- **Pull request status setting**: the one application-wide switch that allows or forbids readings. Off until the user turns it on; the choice is stored, unlike the status itself.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: On a working connection with pull request status turned on, within 10 seconds of opening a project, 100% of its worktrees whose branch has a pull request show an indicator, and 0 worktrees without one do.
- **SC-002**: For every combination of pull request state (open, draft, merged, closed) and check outcome (none, pending, passing, failing), and for each review state, the row and tooltip show, in 100% of cases, the state, the check status by the rule of FR-008 and the review decision that GitHub reports for the pull request FR-004 selects.
- **SC-003**: On a working connection with no rate-limit pause in force, a change on GitHub — checks finishing, a merge, a close — is visible in the sidebar within 6 minutes with no user action, and within 10 seconds of pressing refresh (counted from the end of a reading already under way, when there is one).
- **SC-004**: With the GitHub tooling missing, with no sign-in, with no GitHub remote, and with no network, the application shows 0 indicators and 0 errors, notices or dialogs about pull requests, and every worktree and session function works as before.
- **SC-005**: While a reading is under way — including one GitHub never answers — the application responds to every click and key press within 100 milliseconds, as it does when idle.
- **SC-006**: Automatic readings send at most 30 requests to GitHub per hour for one open project with up to 50 worktrees; 0 for a project no window shows; 0 while the switch is off, as it is on a first start; and 0 between a rate-limit answer and the limit's reset.
- **SC-007**: A second window on the same project adds 0 requests.
- **SC-008**: Hovering worktree rows causes 0 requests to GitHub and 0 disk reads.
- **SC-009**: From seeing a "failing" indicator, a user has the pull request open in their browser in at most 2 actions, without typing.
- **SC-010**: 0 worktrees, sessions or branches are removed without the user confirming the existing delete confirmation.
- **SC-011**: After the application has run with pull request status on, its stored files contain 0 pull request data and its logs contain 0 pull request titles or addresses.

## Assumptions

- "Each worktree" means the worktree rows of the sidebar. The "Default" entry is not a worktree (Principle III) and gets no indicator; showing a pull request for the branch checked out in the project root is out of scope.
- "Combined check status" is one value for all of a pull request's checks, by the rule of FR-008 — the same reduction GitHub shows beside a pull request. Listing individual checks, their names or their logs is out of scope.
- Check status is shown for open and draft pull requests only; for a merged or closed one it no longer calls for action.
- "Review state" is GitHub's overall review decision for the pull request. Individual reviewers, comments and requested reviewers are out of scope.
- "A modest interval" is 5 minutes, fixed; making it configurable is out of scope. "On demand" is the sidebar's existing refresh control — no new button is added.
- "Suggests removing" is a passive mark and a tooltip line that lead to the existing delete confirmation through the row's existing **Delete** action (FR-016). It is never a dialog that interrupts, and never an automatic removal. Whether the branch is deleted too is the existing confirmation's choice.
- A worktree counts as holding no newer work when its branch has no commits beyond those the pull request merged. This is worked out at each reading (FR-018a), so a commit made after a reading withdraws the suggestion at the next one, not at once; the existing delete confirmation remains the safeguard in between. Uncommitted changes in the worktree are not examined for the suggestion; the existing delete confirmation already speaks to what is removed.
- Only github.com is supported, as in 034. GitHub Enterprise, GitLab and other hosts show no indicator.
- The user is signed in to GitHub through the same GitHub tooling feature 034 relies on (the GitHub CLI, `gh`); signing in is outside this feature. Where that tooling runs when the session service runs in a container is a plan decision; the behaviour above holds either way.
- Acting on a pull request from the application — merging, closing, approving, re-running checks, creating one — is out of scope. So are notifications (sound, system notification) when a status changes.
- The exact shape, size, colours and position of the indicator, and the wording of the tooltip lines, are plan decisions within FR-009 to FR-011.
- **Constraint carried from the issue, for the plan**: turning the GitHub tooling's output into pull request status belongs in the application's render-free core and is tested against recorded output of that tooling (Principle I).
- **Relation to other features**: builds on 034-github-issue-worktree (GitHub remote and sign-in rules), 029-worktree-tooltip-details (tooltip lines and bounds) and 029-refresh-worktrees-list (the refresh control). 029-refresh-worktrees-list FR-012 forbids automatic refreshing of the worktree *listing*; this feature leaves that listing manual and refreshes only pull request status on the interval.
