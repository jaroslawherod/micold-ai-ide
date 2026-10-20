# Feature Specification: Review comments on a pull request's diff, sent to the agent to apply and push

**Feature Branch**: `feat/1149_review-comments-on-a-pull-request-s-diff-sent-to`

**Created**: 2026-10-20

**Status**: Draft

**Input**: GitHub issue #1149 "Review comments on a pull request's diff, sent to the agent session to apply and push to the PR". "The Changes view shows a pull request's diff read-only (spec 1003 puts commenting out of scope), so line comments can only be written on the worktree's own changes. Wanted: pick lines and write review comments on a PR's diff in the Changes view, like drafts on the working tree; send them to the agent session on the PR's branch (anchored to the PR head commit); the review message tells the agent to apply the requested changes and push them to the PR's branch. Out of scope: posting comments to GitHub. Relates to #482, #946, #1003."

## Background

Features 482 and 946 let the user pick lines in the Changes view of a worktree's own changes, write review comments, list them in the review panel, and send them as one batch to an agent session of that worktree. Feature 1003 lets the same view show the diff of any pull request the worktree produced, but read-only: no line can be picked and no comment written.

This feature removes that gap for pull requests. It keeps everything 482, 946 and 1003 specify and this spec does not change: how lines are picked, the comment card, size and binary limits, the batch's block format, statuses and replies, the review panel, the section of pull requests.

## Terms

- **PR diff**: the base...head diff of a pull request, as the Changes view shows it since 1003.
- **PR comment**: a review comment written on a PR diff. It records, besides what 482 records, the pull request it belongs to and the **PR head commit** the diff showed when it was written.
- **PR head commit**: the commit the pull request's head branch pointed at in the diff the user commented on.
- **PR branch**: the pull request's head branch.
- **PR review batch**: the one message **Send review** builds from the draft comments written on one pull request.
- **Working-tree comment**: a comment as 482 and 946 define it, on the worktree's own changes.

## Clarifications

### Session 2026-10-20

- Q: Worktree's checked-out branch is not the PR branch on Send review: deliver or refuse? → A: Deliver anyway; the preamble tells the agent to switch to the PR branch first. _(decided by user)_
- Q: Send review for a merged or closed PR: block or allow? → A: Allow, and state "merged" or "closed" in the batch. _(decided by user)_

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Write review comments on a pull request's diff (Priority: P1)

A user opens Review changes, selects a pull request of the worktree, and picks one line or a range of lines in the shown diff exactly as on the working-tree diff. A comment card opens, the user writes the comment, and it is kept as a draft listed in the review panel.

**Why this priority**: Without it nothing else in the issue is reachable.

**Independent Test**: Select a pull request whose diff has a text file, pick lines 10 to 12 on the new side, write "rename this", and check a draft card appears on those lines and a row for it appears in the review panel naming the pull request.

**Acceptance Scenarios**:

1. **Given** a pull request is selected and its text file diff is shown, **When** the user picks a line or range and writes a comment, **Then** a draft comment is created on those lines of that file and side, quoting the code on them, and shown on the diff and in the review panel.
2. **Given** a draft PR comment, **When** the user edits or deletes it, **Then** it behaves as a working-tree draft does (946 FR-002, FR-018).
3. **Given** a pull request's diff shows a binary or non-UTF-8 file, or a file too large to render, **When** the user looks at it, **Then** no lines can be picked on it, as on the working-tree diff (482 FR-008).
4. **Given** PR comments exist on pull request A, **When** the user selects pull request B or returns to the working-tree changes, **Then** A's comments are not shown on B's or the working tree's diff, and each comment remains reachable from the review panel under its own pull request.
5. **Given** draft PR comments, **When** the app restarts, **Then** they are still there, still attached to their pull request and PR head commit (946 FR-008 persistence rules).

---

### User Story 2 - Send the PR comments to the agent on the PR's branch with an apply-and-push request (Priority: P1)

The user presses **Send review** for a pull request. One PR review batch goes to an agent session of the worktree, anchored to the PR head commit. The batch tells the agent to apply the requested changes and push them to the PR branch.

**Why this priority**: The issue's purpose: the comments become changes on the pull request.

**Independent Test**: With two draft comments on a pull request, press Send review and check the delivered message holds both comments, names the pull request, its head branch and its head commit, and asks the agent to apply the changes and push them to that branch.

**Acceptance Scenarios**:

1. **Given** draft comments on a pull request, **When** the user presses Send review, **Then** one PR review batch holding exactly those comments is delivered to a session, and the comments become Sent (946 FR-015).
2. **Given** a PR review batch, **When** the agent reads it, **Then** the preamble names the pull request (number and title), its head branch and the PR head commit the comments were written against, and asks the agent to apply the requested changes (or reply with an explanation, as 946 FR-011), to commit them, and to push them to the PR branch. The preamble never asks the agent to post anything to GitHub.
3. **Given** a PR review batch, **When** it is read, **Then** each comment block keeps the 946 block format, and quotes the code as it was at the PR head commit.
4. **Given** draft comments on the pull request and others on the working tree, **When** the user presses Send review for the pull request, **Then** the batch holds only that pull request's comments and the working-tree drafts stay Draft; and the reverse.
5. **Given** the agent reports on a comment of a PR review batch by its reference, **When** the report arrives, **Then** the comment shows Applied or Answered and the reply as a working-tree comment does (946).
6. **Given** drafts on two different pull requests, **When** the user presses Send review on one, **Then** the batch holds only that pull request's comments.

---

### User Story 3 - The batch reaches a session that can push to the PR's branch (Priority: P1)

The batch must reach an agent working on the PR branch, so a push goes where the pull request is.

**Why this priority**: Delivered to a session on another branch, "push to the PR branch" would push the wrong work.

**Independent Test**: Run a session on the worktree while the worktree is on the pull request's branch and send; then move the worktree to another branch and send again, and check the second case is handled as FR-009 says.

**Acceptance Scenarios**:

1. **Given** the worktree's checked-out branch is the PR branch and a session of the worktree runs, **When** the user presses Send review, **Then** the batch is delivered to the session 482 FR-016 picks.
2. **Given** the worktree's checked-out branch is not the PR branch (the worktree moved on to a later branch), **When** the user presses Send review, **Then** the batch is delivered anyway, and its preamble tells the agent to switch to the PR branch first.
3. **Given** no session of the worktree runs, **When** the user presses Send review, **Then** a session starts and receives the batch as its first input, as 482 FR-016 does.
4. **Given** delivery fails, **When** the user looks at the comments, **Then** they are still Draft and an error is shown (482 FR-017).

---

### User Story 4 - The pull request moves after the comments were written (Priority: P2)

The PR branch can gain commits (the agent's push, a human's push) or be force-pushed while draft comments exist.

**Why this priority**: Comments written against a head commit that is no longer the head must not silently point at the wrong code.

**Independent Test**: Write a comment, push a commit to the PR branch that changes the commented lines, wait for the pull request list refresh, and check the comment is marked outdated and still sent with its recorded quote and PR head commit.

**Acceptance Scenarios**:

1. **Given** a draft PR comment, **When** the pull request's head moves to a new commit, **Then** the comment keeps its recorded PR head commit, is shown on the new diff where its quoted code still is (946 re-anchoring rules), and is marked outdated when it no longer is.
2. **Given** a draft PR comment whose recorded PR head commit is no longer the pull request's head, **When** the user presses Send review, **Then** the batch states both the recorded commit and the current head and each block carries its recorded quote and line range, so the agent can find the code.
3. **Given** the pull request is merged or closed, **When** the user looks at its draft comments, **Then** they stay visible and editable, and Send review stays allowed and states in the batch that the pull request is merged or closed.

---

### Edge Cases

- The pull request leaves the list (1003 FR-008): its comments stay in the review panel under that pull request, flagged as having no pull request in the list; Send review for them is unavailable until it returns. Nothing is deleted.
- Two worktrees whose branches share a name, each with its own comments: comments are scoped to the entry (482 FR-021) and never cross.
- Empty input: a comment with empty text is refused as for working-tree comments.
- Many comments: the 482 size limits and quote elision apply to PR comments identically.
- Concurrency (Principle II): Send review pressed from two windows, or while the pull request refreshes: each draft is delivered at most once (482 FR-018); a head move during a send never changes which commit a comment is anchored to; the preamble names the recorded commit, and the current head too when it differs (FR-007).
- The PR head commit is not available locally (branch not fetched, rewritten or deleted): the batch quotes the code as recorded in the comment and the preamble says the commit could not be read; sending is not refused.
- A PR diff too large for GitHub to diff, or still loading: no lines can be picked until it shows; existing comments stay listed.
- A fork pull request is not listed (1003), so it has no comments.
- The agent session on the branch is a PTY session or a structured session: delivery follows 482/946 for both.
- Windows, macOS and Linux: the batch is byte-identical for the same comments and head commit (Principle VI; 946 FR-012).
- Offline or GitHub unavailable after the diff is shown: drafts stay writable and persist; sending needs no GitHub access.
- A PR comment on a file the working tree also changed: kept separate; a PR comment is never shown on the working-tree diff.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: While a pull request's diff is shown in the Changes view, the user MUST be able to pick one line or a contiguous range on either side of a text file and write a comment, with the same interaction, card, editing and deletion as on the working-tree diff.
- **FR-002**: A PR comment MUST record what 482 FR-012 records, plus the pull request (number and head branch) and the PR head commit of the diff shown when it was written.
- **FR-003**: A PR comment MUST be scoped to its entry and its pull request: it MUST be shown on the diff of that pull request only, never on another pull request's or the working-tree diff, and the reverse for working-tree comments.
- **FR-004**: The review panel MUST list PR comments grouped by pull request, each group naming the pull request, apart from the working-tree comments, with status, reply and edit/delete as 946 FR-016 to FR-018.
- **FR-005**: PR comments MUST be kept on the local filesystem with the entry's other comments and MUST survive an app restart (946 FR-008), and MUST be discarded when the worktree is removed (482 FR-020).
- **FR-006**: Send review MUST act on one pull request at a time: it MUST build one PR review batch from exactly the draft comments of the pull request being shown, and MUST leave the drafts of other pull requests and of the working tree untouched. On the working-tree diff it MUST build the working-tree batch from working-tree drafts only.
- **FR-007**: The PR review batch MUST open with a preamble naming the pull request (number, title, head branch), the PR head commit the comments are anchored to, and, when different, the pull request's current head commit. It MUST ask the agent to apply each requested change (or reply with an explanation, as 946 FR-011), commit the changes, and push them to the pull request's head branch. It MUST NOT ask the agent to post anything to GitHub.
- **FR-008**: The PR review batch's comment blocks MUST keep the 946 block format (reference, file, side, line range, quoted code, text), the quoted code and line numbers being those of the PR head commit's version, and MUST be ordered as 482 FR-015.
- **FR-009**: The batch MUST be delivered to a session of the entry as 482 FR-016 specifies. When the entry's checked-out branch is not the PR branch, the batch MUST still be delivered and its preamble MUST tell the agent to switch to the PR branch first. When the PR branch is the checked-out branch, the batch needs no branch-switch instruction.
- **FR-010**: Comments MUST become Sent only once their batch is delivered; on failure they MUST stay Draft and the user MUST see an error (482 FR-017); a send in progress MUST make Send review unavailable for that entry in every window (482 FR-018).
- **FR-011**: A report from the agent on a PR comment MUST be accepted and shown exactly as for a working-tree comment (946 FR-003 to FR-007).
- **FR-012**: When the pull request's head moves, a PR comment MUST keep its recorded PR head commit, be re-anchored on the new diff by its quoted code, and be marked outdated when its code is gone (946 FR-020, FR-021).
- **FR-013**: PR comments for a pull request that is not in the list (1003 FR-008) MUST be kept and shown in the review panel flagged accordingly; Send review for them MUST be unavailable until the pull request is listed again.
- **FR-014**: Binary, non-UTF-8 and over-size files in a PR diff MUST accept no comments (482 FR-008, FR-009).
- **FR-015**: Clear sent and Discard drafts MUST act on the scope the user is in (the shown pull request, or the working tree) and touch only the current scope's comments and never those of another scope.
- **FR-016**: Building the PR review batch, anchoring PR comments and keeping them per pull request MUST be verifiable by automated tests that run with no window open, no session running and no network.
- **FR-017**: The user guide MUST describe commenting on a pull request's diff, the PR review batch and the apply-and-push request, and MUST state that nothing is posted to GitHub.

### Key Entities

- **PR comment**: a review comment (946) with the extra fields pull request (number, head branch) and PR head commit.
- **PR review batch**: preamble naming the pull request, head branch, PR head commit and the apply-and-push request, then one block per comment.
- **Comment scope**: the working tree, or one pull request of the entry; every comment belongs to exactly one.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: In the acceptance fixtures, a user can go from selecting a pull request to a delivered PR review batch using only in-app actions, typing 0 file paths, 0 line numbers, 0 commit ids and 0 branch names.
- **SC-002**: With 10 comments across 3 files on one pull request, the delivered batch holds all 10, each once, and 0 comments of another pull request or of the working tree.
- **SC-003**: In 100% of PR review batches the preamble names the pull request's head branch and PR head commit and contains the apply-and-push request; in 0% does it ask for anything to be posted to GitHub.
- **SC-004**: After the pull request's head moves, 100% of comments whose quoted code is unchanged keep their place and 100% of those whose code is gone are marked outdated.
- **SC-005**: The same comments on the same pull request and head commit produce a byte-identical batch on Linux, macOS and Windows.
- **SC-006**: Restarting the app loses 0 PR comments.

## Out of Scope

- Posting comments, reviews or replies to GitHub, and importing GitHub review comments.
- Editing, staging, committing or pushing from the Changes view itself: the agent applies and pushes.
- Pull requests that are not in the worktree's list (forks, other worktrees' pull requests).
- Commenting on a diff between arbitrary commits.
- Resolving merge conflicts.

## Assumptions

- The Changes view of 1003 gives the selected pull request's number, head branch and head commit; this feature reads them and does not change how the list is built.
- Line picking, the comment card, the review panel, statuses, replies, re-anchoring and batch delivery are reused from 482 and 946; this feature adds the pull-request scope to them.
- The agent has git access to the PR branch from the worktree; whether the push succeeds is the agent's outcome, reported through the 946 reply, not verified by the app.
- The batch is a request to the agent; the app does not verify that a push happened.
