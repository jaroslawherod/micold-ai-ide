# Feature Specification: Create a Worktree from a GitHub Issue

**Feature Branch**: `feat/allow-to-create-worktree-from-github-issue`

**Created**: 2026-09-29

**Status**: Draft

**Input**: User description: "when creating new worktree add another option github issues. Application should fetch github issues for current project and allow to select one for working with it. The ticket should define an worktree name. The type should be resolved by mapping labels into issues types . The mapping should be global for application for now."

## Clarifications

### Session 2026-09-29

- Q: Which labels does the default label-to-type mapping cover? → A: GitHub's stock type-bearing labels only — `bug` → fix, `enhancement` → feat, `documentation` → docs; the other stock labels (`duplicate`, `good first issue`, `help wanted`, `invalid`, `question`, `wontfix`) say nothing about the kind of work and stay unmapped. _(agent-resolved: this repository's own label set, `gh label list` on origin, is GitHub's stock set; spec.md#FR-021)_
- Q: Without a GitHub sign-in, should a public repository's issues still be listed anonymously? → A: No — without a sign-in the "not signed in" message is shown, for public repositories too. _(decided by user)_
- Q: Which GitHub hosts are supported? → A: github.com only; a remote on any other host (GitHub Enterprise, GitLab, Bitbucket) counts as "no GitHub remote". _(decided by user)_
- Q: Which issues does the picker list? → A: All open issues of the repository, most recently updated first, with search — not filtered to the user's assigned issues, and no closed issues. _(decided by user)_
- Q: Keep an issue-derived name cut to 50 characters, and at most 1,000 issues loaded? → A: Yes, both limits stand as in FR-010 and FR-004. _(decided by user)_
- Q: What does the issue picker's search cover? → A: It searches the repository's open issues: typing filters the loaded list at once by number, title and label names, with the existing-branch picker's type-ahead behaviour and keyboard operation (feature 021); when the loaded list is incomplete (more open issues than the 1,000 cap), the search also asks GitHub for matching open issues beyond the cap, with the same error and offline handling as the initial load. _(decided by user)_
- Q: When the search beyond the load cap returns an issue that matched GitHub's own search only through text the picker does not search (its body or comments), is it shown? → A: No — an issue from that search is shown only when it matches the typed text by number, title or label name, under the same rule as loaded issues, so the results never depend on whether an issue happened to be loaded. _(agent-resolved: spec.md#FR-005; specs/021-branch-typeahead-search/spec.md one matching rule per picker)_

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Start a worktree from an open issue (Priority: P1)

A developer's work is tracked as GitHub issues on the project's repository. Today, to start work on an issue they open the create-worktree form, pick "new branch", and retype the issue number into the ticket field and a shortened issue title into the name field — copying both by hand from the browser. They want a third choice in the form, next to "new branch" and "existing branch": **GitHub issue**. Choosing it lists the project's open issues; picking one fills in the ticket from the issue number and the name from the issue title, so the worktree and branch are named after the issue without any retyping.

**Why this priority**: This is the whole request. Listing issues and naming the worktree after the picked one is a complete, shippable slice even before labels choose the type — the user still picks the type by hand.

**Independent Test**: Can be fully tested on a project whose repository has open issues: open the create-worktree form, choose "GitHub issue", pick issue #42 titled "Crash when opening empty project", pick a type, and confirm the created worktree's ticket segment is `42` and its name segment is derived from the title.

**Acceptance Scenarios**:

1. **Given** the create-worktree form is open on a project whose repository is hosted on GitHub, **When** the user looks at the branch-source choice, **Then** "GitHub issue" is offered alongside "new branch" and "existing branch".
2. **Given** the user chooses "GitHub issue", **When** the issues load, **Then** the repository's open issues are listed, each showing its number and title, most recently updated first.
3. **Given** the issue list is showing, **When** the user types in the list's search field, **Then** the list narrows as each character is typed to issues whose number, title or label names match what was typed, and the user can move through the results with Up and Down and pick one with Enter without leaving the field.
4. **Given** the issue list is showing, **When** the user picks issue #42 titled "Crash when opening empty project", **Then** the ticket is set to `42`, the name is set to the title (shortened as FR-010 describes when it is long), and the directory and branch preview update exactly as if the user had typed those values into the new-branch inputs.
5. **Given** an issue was picked, **When** the user edits the pre-filled name or ticket before submitting, **Then** the edited values are used — the issue supplies a starting point, not a locked value.
6. **Given** an issue was picked and a type is set, **When** the user submits, **Then** the worktree is created exactly as a new-branch worktree with the same type, ticket and name would be, including the existing prompt when the derived branch already exists.
7. **Given** the issues are still loading, **When** the user looks at the list, **Then** a loading indication is shown and the rest of the form stays usable, including switching back to another branch source.
8. **Given** the user typed a ticket and name, or picked another issue earlier, **When** the user picks an issue, **Then** the ticket and name are replaced by the picked issue's number and title.
9. **Given** the user chooses "GitHub issue", **When** the source is shown, **Then** it states that choosing it contacts GitHub and names the repository (`owner/name`) whose issues are read.
10. **Given** the repository has more open issues than the load cap, so the loaded list is incomplete, **When** the user types a search that matches an open issue beyond the cap, **Then** that issue appears in the results — found by asking GitHub — alongside the matching loaded issues, and can be picked like any other.

---

### User Story 2 - The issue's labels choose the worktree type (Priority: P2)

Issues already carry labels that say what kind of work they are — `bug`, `enhancement`, `documentation`. After picking an issue, the developer does not want to pick the type again: the app should translate the issue's labels into a worktree type (feat, fix, chore, docs, refactor, test, build, ci, perf, style) using a label-to-type mapping, and pre-select that type.

**Why this priority**: It removes the last manual step from the issue path, but the feature is already useful without it (Story 1 leaves the type for the user to pick).

**Independent Test**: With the default mapping in place, pick an issue labelled `bug` and confirm the type is pre-selected as `fix`; pick an issue with no mapped label and confirm no type is pre-selected and submitting asks for one.

**Acceptance Scenarios**:

1. **Given** the mapping maps `bug` to `fix`, **When** the user picks an issue labelled `bug`, **Then** the type is pre-selected as `fix`.
2. **Given** the mapping maps `bug` to `fix` and `enhancement` to `feat`, and `bug` is listed before `enhancement` in the mapping, **When** the user picks an issue labelled both `enhancement` and `bug`, **Then** the type is pre-selected as `fix` — the mapping entry listed first wins.
3. **Given** a type is currently selected (chosen by hand or from an earlier pick) and none of an issue's labels appear in the mapping, **When** the user picks that issue, **Then** the type is cleared — no type carries over from earlier input — and submitting without choosing one shows the same "type required" validation the form shows today.
4. **Given** a type was pre-selected from labels, **When** the user chooses a different type, **Then** the user's choice is used.
5. **Given** the mapping lists `Bug` and the issue's label is `bug`, **When** the user picks the issue, **Then** the label matches — label matching ignores letter case.
6. **Given** a type is selected and the picked issue carries a mapped label, **When** the user picks it, **Then** the selected type is replaced by the mapped one.
7. **Given** the issue list is showing, **When** the user looks at an issue row, **Then** the issue's labels are visible, so the user can tell which type it will receive before picking it.

---

### User Story 3 - Edit the label-to-type mapping in Settings (Priority: P3)

Different teams label differently — one uses `type: bug`, another `defect`. The developer wants to edit the label-to-type mapping in the application's Settings: add a label and the type it maps to, change a label's type, remove an entry, and reorder entries to set which wins when an issue carries several mapped labels. The mapping is global: it applies to every project opened in the application.

**Why this priority**: The default mapping covers GitHub's stock labels, so most users get Story 2's value without ever editing it. Editing matters for teams with their own label scheme.

**Independent Test**: In Settings add a mapping entry `defect` → `fix`, save, then in any project pick an issue labelled `defect` and confirm the type is pre-selected as `fix`; restart the application and confirm the entry is still there.

**Acceptance Scenarios**:

1. **Given** Settings is open, **When** the user opens the GitHub issues section, **Then** the current label-to-type mapping is shown as an ordered list of label → type entries.
2. **Given** the mapping is shown, **When** the user adds an entry `defect` → `fix` and saves, **Then** picking an issue labelled `defect` in any project pre-selects `fix`.
3. **Given** the mapping is shown, **When** the user changes, removes, or reorders an entry and saves, **Then** the next issue picked in any project is typed by the updated mapping.
4. **Given** the user edited the mapping, **When** the application restarts, **Then** the edited mapping is still in effect.
5. **Given** the user enters a blank label, or a label already present in the mapping (ignoring case), **When** they try to save, **Then** saving is refused with a message on the offending entry, and the saved mapping is unchanged.
6. **Given** the user never edited the mapping, **When** they open the section, **Then** the default mapping is shown, and a "restore defaults" action returns an edited mapping to it.

---

### Edge Cases

- **Project not hosted on GitHub**: the repository has no remote on GitHub. The "GitHub issue" choice is shown but disabled, with the reason ("this repository has no GitHub remote"), rather than hidden or failing after it is chosen.
- **Several GitHub remotes**: the remote the repository treats as its default (`origin` when present) is the one whose issues are listed; when that remote is not on GitHub but another remote is, the first GitHub remote is used; the list names that repository so the user can see which one it is.
- **Not signed in to GitHub, or no access to the repository**: the list is replaced by a message saying the issues could not be read because GitHub sign-in is missing or lacks access, and what to do about it. Nothing else in the form is affected.
- **Offline or GitHub unreachable**: the list shows that issues could not be loaded and offers a retry. "New branch" and "existing branch" keep working — creating worktrees never depends on the network.
- **Rate-limited by GitHub**: treated as a load failure with the reason given and a retry offered.
- **No open issues**: the list says the repository has no open issues, rather than showing an empty control.
- **Many open issues**: the list stays responsive, and search finds an issue that is not among the first rows shown.
- **Search beyond the load cap**: when the loaded list is complete, search never contacts GitHub. When it is incomplete, the loaded matches show at once and the GitHub search's matches join them when they arrive, with a searching indication meanwhile; an issue in both appears once. If that search fails (offline, not signed in, rate limit, 10-second timeout), the loaded matches stay shown and the list says the search beyond the loaded issues failed and offers a retry, as FR-007 describes for the initial load. A newer keystroke discards an older search's result (FR-007a).
- **Pull requests**: GitHub reports pull requests as issues; they are excluded from the list.
- **Title that slugs to nothing** (for example only emoji or punctuation): the name field is left empty after the pick, and the form's existing "name required" validation applies.
- **Very long title**: the name filled in from it is shortened as FR-010 describes, so a directory named after a long issue stays within path-length limits on every OS (Windows' being the tightest). A long name the user types by hand is not shortened; that behavior is unchanged.
- **Derived branch already exists** (for example work on the same issue started earlier): the existing reuse/overwrite prompt from the existing-branch feature appears, unchanged.
- **Issue closed while the list is open**: picking it still works — the list is a snapshot, and a closed issue is still a valid thing to name work after.
- **User switches source mid-load**: switching away from "GitHub issue" while issues load discards the load's result when it arrives; it does not overwrite anything the user typed.
- **Switching back to "new branch" after a pick**: the form keeps the ticket, name and type the pick filled in, so the user can continue from them by hand.
- **Mapping maps two labels to the same type**: allowed — several labels may map to one type.
- **Retry or re-choose while a load runs**: only the newest load's result is shown; an older response arriving later is discarded.
- **Form closed while issues load**: the result is discarded; reopening the form starts from the source's initial state and loads again only if the user chooses "GitHub issue".
- **Mapping edited in Settings while a form holds a pick**: the mapping is read at the moment of the pick; an already-picked issue's type does not change until the user picks again.
- **GitHub tooling not installed**: when the machine has no GitHub tooling the application can use for the sign-in, the list says so and what to install, distinct from "not signed in".
- **Launched from the desktop rather than a terminal**: tooling that works in the user's terminal is found the same way when the application is launched from the Dock, Start menu or a desktop launcher, on Linux, macOS and Windows alike.
- **Same outcomes on every OS**: the sign-in is stored differently per OS (keychain, Credential Manager, a config file); the list's outcomes — issues, "not signed in", "no access", "tooling not installed" — are the same on Linux, macOS and Windows.
- **Session sandbox**: a project whose sessions run in a sandbox lists issues the same way as one whose sessions do not; the sandbox's network and credential settings neither enable nor block the issue list.
- **Public repository, no sign-in**: the "not signed in" message is shown; the feature does not fall back to anonymous access.

## Requirements *(mandatory)*

### Functional Requirements

**Choosing an issue**

- **FR-001**: The create-worktree form MUST offer "GitHub issue" as a third branch-source choice, alongside "new branch" and "existing branch".
- **FR-002**: When the project's repository has no GitHub remote, the "GitHub issue" choice MUST be shown disabled with the reason stated. Only remotes on github.com (including its SSH-over-HTTPS-port host `ssh.github.com`) count as GitHub remotes; a remote on any other host, GitHub Enterprise included, does not.
- **FR-003**: Issues MUST be requested only when the user chooses "GitHub issue" in the form, or searches in it (FR-005a) — never in the background, on project open, or on application start.
- **FR-004**: The system MUST list all open issues of the repository's default GitHub remote — not filtered by assignee, author or any other criterion — excluding pull requests and closed issues, each with its number, title and labels, ordered most recently updated first. At most the 1,000 most recently updated open issues are loaded; when more exist, the list says that only the most recent 1,000 are shown and that search also finds the rest (FR-005a).
- **FR-005**: The issue list MUST provide a type-ahead search field that searches the repository's open issues by number, title and label names, with the existing-branch picker's behaviour (feature 021): the list narrows as the user types, literal matches first, then approximate ones, and results are moved through and picked from the keyboard without leaving the field (021 FR-017, FR-017a). Filtering the loaded issues MUST NOT contact GitHub.
- **FR-005a**: When the loaded list is incomplete (FR-004's cap was reached), a search MUST also ask GitHub for open issues matching the typed text, excluding pull requests, and merge them into the results without duplicates, keeping only those that match the typed text by FR-005's rule (number, title or label names). This request MUST be subject to FR-007 and FR-007a like the initial load, and its failure MUST leave the loaded matches shown. When the loaded list is complete, search MUST NOT contact GitHub.
- **FR-006**: While issues load, the form MUST show a loading indication and MUST stay usable, including switching to another branch source.
- **FR-007**: When issues cannot be loaded, the list MUST state why in plain language — GitHub tooling not installed, no GitHub sign-in, no access to the repository, no network, rate limit, no answer to any single request within 10 seconds (a large list may take several requests), or other failure — and MUST offer a retry.
- **FR-007a**: When a newer load or search has been started (retry, re-choosing the source, a further keystroke) or the form has been closed, the result of an older load or search MUST be discarded.
- **FR-008**: When the repository has no open issues, the list MUST say so.

**Naming from the issue**

- **FR-009**: Picking an issue MUST set the form's ticket to the issue number, without a `#` prefix.
- **FR-010**: Picking an issue MUST set the form's name to the issue title, which the existing naming rules then turn into the directory and branch name segments. When the title's slugged name segment would exceed 50 characters, the name filled in MUST be cut at the last word boundary that keeps that segment within 50 characters, or at exactly 50 characters when the first word alone is longer. When the title yields an empty name segment, the name is left empty and the ticket is still set.
- **FR-010a**: A pick MUST replace the form's ticket and name, whatever they held before.
- **FR-011**: The ticket and name set by a pick MUST stay editable before submission, and the edited values MUST be used.
- **FR-012**: A worktree created from an issue MUST be created exactly as a new-branch worktree with the same type, ticket and name, including the existing branch-conflict prompt and all existing validation.

**Type from labels**

- **FR-013**: Picking an issue MUST pre-select the type given by the first entry in the label-to-type mapping whose label matches one of the issue's labels, ignoring letter case.
- **FR-014**: When no label of the picked issue matches the mapping, the pick MUST clear any selected type, and the form's existing "type required" validation MUST apply. When a label matches, the pick MUST replace any selected type.
- **FR-014a**: The mapping MUST be read at the moment of a pick; editing it later does not change the type of an issue already picked.
- **FR-015**: A type pre-selected from labels MUST be changeable by the user before submission.

**The mapping**

- **FR-016**: The label-to-type mapping MUST be a single, application-wide setting that applies to every project.
- **FR-017**: The mapping MUST be an ordered list of entries, each pairing one label with one worktree type; order decides which entry wins when an issue carries several mapped labels.
- **FR-018**: Settings MUST let the user view, add, change, remove and reorder mapping entries, and restore the default mapping.
- **FR-019**: Saving the mapping MUST be refused, with the offending entry identified, when an entry's label is blank or duplicates another entry's label ignoring case.
- **FR-020**: The mapping MUST persist on the local filesystem across application restarts.
- **FR-021**: Until the user edits it, the mapping MUST be the default: `bug` → fix, `enhancement` → feat, `documentation` → docs, in that order. GitHub's other stock labels stay unmapped.

**Privacy and offline**

- **FR-022**: The system MUST use the user's existing GitHub sign-in on the machine and MUST NOT ask for, store, or display GitHub credentials itself. Without a sign-in the system MUST NOT fall back to anonymous access, even for a public repository.
- **FR-023**: Loaded issues MUST NOT be persisted beyond the open form; closing the form discards them.
- **FR-024**: Every other way of creating a worktree MUST work with no network and no GitHub sign-in.
- **FR-025**: The "GitHub issue" source MUST state, before and while it loads, that it contacts GitHub, and MUST name the repository (`owner/name`) it reads. Requests MUST carry nothing about the project beyond that repository's identity and, for a search beyond the loaded issues (FR-005a), the text the user typed.
- **FR-026**: The issue list's outcomes MUST be the same on Linux, macOS and Windows, including when the application is launched from a desktop launcher rather than a terminal, and MUST NOT depend on whether the project's sessions run in a sandbox.

### Key Entities

- **Issue**: an open GitHub issue of the project's repository, as shown in the picker — number, title, labels, last-updated time. Read-only, held only while the form is open.
- **Label-to-type mapping**: the application-wide, ordered list of mapping entries, persisted locally, with a default.
- **Mapping entry**: one label (matched ignoring case) and the worktree type it yields.
- **Branch source**: which half of the create-worktree form is active — new branch, existing branch, or GitHub issue.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: A user can go from the open create-worktree form to a created worktree named after a chosen issue in under 20 seconds, without typing the ticket or the name.
- **SC-002**: For an issue carrying a mapped label, the worktree is created with no type chosen by hand in 100% of cases.
- **SC-003**: On a repository with 500 open issues, the user can find a given issue by typing part of its number, title or a label name and see the narrowed list within 1 second of typing.
- **SC-004**: With the network unplugged, "new branch" and "existing branch" creation succeed in 100% of attempts, and the "GitHub issue" choice reports the failure within 10 seconds instead of hanging.
- **SC-005**: A mapping entry saved in Settings takes effect in every open project on the next issue pick, with no restart.
- **SC-006**: No GitHub credential or issue content is written to the application's stored files.

## Assumptions

- "GitHub issues for current project" means issues of the GitHub repository the project's default remote points at; only github.com is supported (FR-002).
- The user is already signed in to GitHub on the machine through the standard GitHub tooling; the application relies on that sign-in and does not implement its own. Signing in is outside this feature. Without a sign-in no issues are listed (FR-022). Which tooling is used is a plan decision.
- Choosing "GitHub issue" is the user's explicit, informed opt-in to contacting GitHub, which satisfies the constitution's local-first principle (IV): nothing is sent unless the user makes that choice.
- "The ticket should define a worktree name" means the issue number becomes the ticket segment and the issue title becomes the name segment, both passed through the existing naming rules; the resulting name format is unchanged.
- The mapping is global "for now": a per-project mapping may follow later, and the design should not rule it out, but it is out of scope here.
- Recording on the worktree which issue it came from (linking back, showing the issue in tooltips, closing the issue) is out of scope.
- The type vocabulary (feat, fix, chore, docs, refactor, test, build, ci, perf, style) is unchanged.
- The issue picker reuses the look and search behavior of the existing-branch picker.
