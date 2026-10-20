---

description: "Task list for feature 1149: review comments on a pull request's diff, sent to the agent to apply and push"
---

# Tasks: Review Comments on a Pull Request's Diff

**Input**: Design documents from `specs/1149-pr-diff-review-comments/`

**Prerequisites**: plan.md, spec.md, research.md, data-model.md, contracts/ (batch-format, review-wire-delta), quickstart.md. Extends 482 and 946.

**Tests**: Mandatory (Constitution I). Every test task comes before the implementation it covers and must fail first for the right reason.

**Documentation**: `docs/user-guide/reviewing-changes.md` grows with each story, in the same milestone as the behaviour it describes (Constitution VII, CI user-guide gate).

**Cross-platform**: the batch, scope rules and anchoring are core logic with `\n` only (Constitution VI).

**Wire**: every wire-visible change of this feature is made in **one** edit (T005), so `PROTOCOL_VERSION` moves 56 to 57 once; a second bump fails `crates/micold-core/tests/schema_hash.rs`. Until M2 the daemon refuses `ReviewSend { pr: Some }` with `InvalidInput` and the client does not offer Send review on a pull request.

**Format**: `- [ ] T### [P?] [US?] description with path`. Paths are under `crates/` unless stated.

## Phase 1: Setup

- [x] T001 Add `Serialize, Deserialize` (lower-case variant names on the wire) to `BranchPrState` in `micold-core/src/pull_request.rs` (data-model.md, `PrContext`); add a serde round-trip unit test beside it.

## Phase 2: Foundational (core scope model, wire, store)

- [x] T002 [P] Tests first in `micold-core/src/review/comment.rs`: `PrAnchor { number: u64 (> 0), head_branch: String (non-empty, no control characters), head_commit: String (40 or 64 lowercase hex digits) }`; `Scope::{WorkingTree, PullRequest(u64)}`; `add` with and without `pr` stores it; `scope()` derived from `pr`; `clear_sent(scope)` and `discard_pending(scope)` touch only that scope; `begin_send(entry, scope, ..)` snapshots only that scope's Drafts and gives `NothingPending` when only other scopes have drafts; `Busy` while any send is open.
- [x] T003 [P] Tests first in `micold-core/src/review/store.rs`: version 3 round-trips `pr`; a file without PR comments is written as version 2; versions 1 and 2 load with `pr` null; version 4 is quarantined. Change the two existing tests asserting version 3 is quarantined (~lines 192 and 377) to version 4.
- [x] T004 [P] Tests first in `micold-core/tests/protocol_roundtrip.rs`: `ReviewEditOp::Add { pr }`, `ClearSent { scope }`, `DiscardPending { scope }`, `ClientMsg::ReviewSend { pr: Option<PrSend> }` with `PrSend { number, title, head_branch, state: BranchPrState, current_head }`, and a `ReviewComment` with `pr`.
- [x] T005 Implement T002 to T004 in one edit: `PrAnchor`, `Scope`, `ReviewComment.pr`, `Draft.pr`, scoped `begin_send/clear_sent/discard_pending` in `micold-core/src/review/comment.rs`; version 3 in `micold-core/src/review/store.rs` (`StoredComment.pr`, written only when a comment has `pr`, else 2); the wire types and `PROTOCOL_VERSION` 57 in `micold-core/src/protocol/messages.rs` and `micold-core/src/protocol/version.rs`; update the schema hash test expectation. `begin_send` takes `pr: Option<&PrContext>` as in data-model.md: define `PrContext` in `micold-core/src/review/prompt.rs` as a type only (no `build_pr` yet). Depends on T001.
- [x] T006 [P] Tests first in `micold-daemon/tests/review_edit.rs`: W18 (each `InvalidInput` refusal: Default entry `worktree_dir == ""`, `number == 0`, empty or control-character `head_branch`, bad `head_commit`; commit stored lower-cased), W19 (scope isolation of `ClearSent`/`DiscardPending`, `Scope::PullRequest(n)` on the Default entry is a no-op `Ack`, no write and no push when nothing changed), `Edit`/`Delete` of a PR comment as of a working-tree one (US1 s2), `forget_worktree` discarding a PR comment with the worktree (FR-005), and W21 (`Reanchor` on a PR comment accepted with compare-and-set on `from`, a stale `from` refused).
- [x] T007 Implement W18 and W19 (and accept `Reanchor` on a PR comment as on a working-tree one, W21, so the wire is complete; the client sends it only from M3) in `micold-daemon/src/review.rs` (`apply_edit` takes the scope; `relocate` skips PR comments) and pass the new fields in `micold-daemon/src/server.rs`; `micold-daemon/src/state.rs` `review_send` passes `Scope::WorkingTree` for `pr: None` and answers `InvalidInput` for `pr: Some` until M2. Existing 482/946 daemon tests keep passing. Depends on T005, T006.

**Checkpoint**: wire, store and daemon edits accept PR comments; nothing in the client creates one yet.

## Phase 3: User Story 1 - Write review comments on a pull request's diff (P1) MVP

**Goal**: pick lines on a pull request's diff, write a comment, see it on the diff and in the panel, keep it across restarts.

**Independent test**: select a pull request, pick lines 10 to 12 on the new side, write "rename this": a draft card shows and a panel row appears under a heading naming the pull request.

- [x] T008 [P] [US1] Tests first in `micold-client/tests/features_changes.rs` (render-free, spec scenarios US1 1-4, FR-001/003/004/013/014/015): `can_pick` true for a text file of a ready PR diff, false for binary, non-UTF-8, over-size, and a diff still loading or too large; a pick on a PR text file builds `Add` with the shown PR's anchor (`number`, `head_branch`, `head` as `head_commit`); `file_comments`, `pending_counts`, `jump_target` filter by `view_scope(view)` (selecting PR B or the working tree hides A's cards); `panel_rows` groups working tree first, then pull requests by number, each `PanelRow.scope`, `PanelGroup { scope, title, listed }`; a scope whose number is not listed is flagged `listed: false` and its `send_action` is `None`, comments kept; `tidy_actions` (Clear sent, Discard drafts) emit the shown scope.
- [x] T009 [US1] Implement in `micold-client/src/features/changes.rs`: `PullRequestView.head` (the listed head when the diff read started, R3), `ViewScope`/`view_scope`, `can_pick` for `Source::PullRequest`, `Msg::GutterPressed` and `composer_saved` building `Add { pr }`, scoped `file_comments`, `pending_counts`, `status_counts`, `review_toolbar_label`, `jump_target`, `panel_rows`/`PanelRow.scope`/`PanelGroup`, `tidy_actions`, `send_action` (always `None` on a pull request until M2). Carry the new fields to the wire in `micold-client/src/shell/changes.rs`. Depends on T005, T008.
- [x] T010 [US1] Draw in `micold-client/src/ui/changes.rs`: a group heading per pull request in the review panel (number and title from existing `Text` styles; "not in the list" flag when `listed` is false); no new widget (plan, Constitution VIII). Depends on T009.
- [x] T011 [P] [US1] User guide: add "Commenting on a pull request" (picking lines, the panel group heading, comments kept per pull request and across restarts, the "not in the list" flag, nothing is posted to GitHub) to `docs/user-guide/reviewing-changes.md` (FR-017).

**Checkpoint (M1)**: comments can be written, listed, edited, deleted and tidied per scope on a pull request's diff; Send review for a pull request is not yet offered.

## Phase 4: User Story 2 - Send the PR comments with an apply-and-push request (P1)

**Goal**: Send review on a pull request delivers one PR review batch of exactly that pull request's drafts.

**Independent test**: two draft comments on a pull request, press Send review: one message holds both, names the pull request, head branch and head commit, asks to apply, commit and push, never says "post" or "GitHub".

- [x] T012 [P] [US2] Tests first in `micold-core/src/review/prompt.rs` (contracts/batch-format.md): `p1_pr_example_gives_the_exact_bytes`; open, draft, merged, closed; head equal (no note) and moved; two recorded commits (per-block `Written against commit` line); title with `\n`, `\x1b` and 300 characters flattened (control characters to spaces, at most 200 characters, character boundary); no "post" or "GitHub" in any variant; byte-identical on two calls.
- [x] T013 [US2] Implement `build_pr(entry, &PrContext, comments, outdated)` in `micold-core/src/review/prompt.rs`, sharing the block code with `build`; `begin_send` calls it when given a `PrContext`. Depends on T012.
- [x] T014 [P] [US2] Tests first in `micold-daemon/tests/review_send.rs` (W20, SC-002, FR-006/010): 10 comments on 3 files of one PR plus a second PR and working-tree drafts: only the first PR's are delivered, the rest stay Draft and `ReviewSend { pr: None }` sends the working tree only; delivery failure leaves Draft; two concurrent sends, one `Busy`; a head move while a send is open leaves each comment's recorded `head_commit` unchanged; `NothingPending` when the scope has no Draft; refusals as W18 plus bad `current_head`, Default entry.
- [x] T015 [P] [US2] Regression test, run after T016 (characterization: reports are keyed by `CommentRef` and unchanged, so it may pass on arrival; not a red-first gate) in `micold-daemon/tests/review_reports.rs` (FR-011): a report on a PR comment by its reference shows Applied or Answered with the reply as for a working-tree comment.
- [x] T016 [US2] Implement W20 in `micold-daemon/src/state.rs` (`review_send` validates `PrSend`, flattens the title, never logs or stores it, builds `PrContext`, opens the send for the scope, delivers as W6 to W9) with the audit log line naming the number only; pass the field in `micold-daemon/src/server.rs`. Until T021 `PrContext.checked_out` is the PR branch and `unreadable` is empty (T016 and T021 land in the same M2 PR). Depends on T013, T014.
- [x] T017 [P] [US2] Tests first in `micold-client/tests/features_changes.rs`: `send_action` on a listed pull request is `Some` and `Effect::ReviewSend.pr` is `Some(PrSend)` from the list (number, title, head branch, state, head); `None` on the working tree; `None` while the PR is not listed (FR-013).
- [x] T018 [US2] Implement in `micold-client/src/features/changes.rs` and carry `Effect::ReviewSend.pr` in `micold-client/src/shell/changes.rs`. Depends on T016, T017.

## Phase 5: User Story 3 - The batch reaches a session that can push to the PR's branch (P1)

**Goal**: the daemon reads the checkout and tells the agent to switch branch or that a commit is unavailable.

**Independent test**: send while the PR branch is checked out (no switch line), then with another branch checked out (switch line).

- [x] T019 [P] [US3] Tests first, real temp repos (helpers of `micold-core/tests/support/attach_fixture.rs`) in `micold-core/src/review/git.rs`: `GitCli::checked_out_branch` (branch, detached gives `None`) and `GitCli::has_commit` (present, missing, a sha not 40 or 64 hex digits returns false without running git); in `micold-core/src/review/prompt.rs`: P-3 note 1 (checked out is the head branch gives none, another branch, detached) and note 3 (unreadable commits).
- [x] T020 [P] [US3] Tests first in `micold-daemon/tests/review_send.rs` (real git, as `mutation_semantics.rs`; red before T021): another branch checked out gives the switch line, a sha absent from the repo gives the "not available" line, delivery is not refused; no running session starts one that receives the batch as first input (482 FR-016). Depends on T016, T019.
- [x] T021 [US3] Implement inherent `GitCli::checked_out_branch(&self, dir) -> Option<String>` (`git symbolic-ref --short -q HEAD`) and `has_commit(&self, dir, sha) -> bool` (`git cat-file -e <sha>^{commit}`) in `micold-core/src/review/git.rs` with the review reads' deadline; `build_pr` notes 1 and 3; `review_send` in `micold-daemon/src/state.rs` reads both off the state lock for each distinct recorded commit (a git failure counts as `None`/not available, never a refusal). Depends on T019, T020, T016.
- [x] T022 [P] [US3] User guide: add "Sending a pull request's comments" to `docs/user-guide/reviewing-changes.md` (one pull request at a time, the apply, commit and push request, the switch-branch paragraph, merged or closed stated, nothing posted) and update "The review message" for `build_pr`.

**Checkpoint (M2)**: Send review on a pull request delivers the PR review batch to the right session.

## Phase 6: User Story 4 - The pull request moves after the comments were written (P2)

**Goal**: comments follow the code when the head moves and are marked outdated when it is gone; the batch names both commits.

**Independent test**: write a comment, push a commit that changes its lines, refresh the list: the comment is marked outdated and still sent with its recorded quote and commit.

- [x] T023 [P] [US4] Tests first in `micold-core/src/review/anchor.rs` and `micold-core/src/review/diff.rs` (FR-012, SC-004, research R4): `locate` over `SideLines` unchanged; over `HunkLines::new(&FileDiff, Side)` finds Same, Moved, Gone; `line(n)` is `None` outside the hunks; a quote outside every hunk is Gone (the recorded limitation pinned by a test).
- [x] T024 [US4] Implement the `Lines` trait (`line(number) -> Option<&str>`, `len()`) with `SideLines` impl, `locate` generic over it, and `HunkLines` in `micold-core/src/review/anchor.rs` and `micold-core/src/review/diff.rs`. Depends on T023.
- [x] T025 [P] [US4] Tests first in `micold-client/tests/features_changes.rs` and `micold-client/tests/features_pr_changes.rs`: a list read showing a new head for the shown number re-reads the diff keeping the selected file and sets `PullRequestView.head` when ready; comments keep their `head_commit`; `judge_anchors` locates PR comments in `HunkLines` of their file's patch for every file with comments and sets the outdated flag; `reanchor_effect` allows a PR comment (W21) and a move sends `Reanchor`; merged or closed pull request keeps comments visible and editable and `send_action` stays `Some`.
- [x] T026 [US4] Implement in `micold-client/src/features/changes.rs` (`judge_anchors`, `reanchor_effect`, head-change re-read) and shell wiring in `micold-client/src/shell/changes.rs`. Depends on T024, T025.
- [x] T027 [P] [US4] User guide: add "When the pull request moves" (comments follow their code, outdated mark, the batch names the recorded and the current head, the quoted code outside the diff's hunks reads as outdated) to `docs/user-guide/reviewing-changes.md`.

**Checkpoint (M3)**: comments follow the pull request's head.

## Dependencies and order

- Phase 2 before every story; T005 is the single wire edit.
- US1 (M1) before US2 and US3 (M2); US4 (M3) after M1 and M2 (needs the batch's two-commit note, T013).
- Tests precede their implementation in every pair above.
- Parallel: T002, T003, T004, T006 (different files); T008 with T011; T012, T014, T015, T017 (different test files); T023 with T025.

## Implementation Strategy

MVP is M1: pick, write, list and tidy PR comments; the wire change lands whole in M1. M2 adds Send review and the batch (US2 and US3 together: `PrContext` carries both the batch facts and the checkout facts, and the batch contract tests are one set). M3 adds anchoring on hunks and the head-follow. Quickstart part B (visual pass) is left to the close unit.

## Milestones

Each milestone merges to `main` on its own, through one PR (speckit-autopilot).

### M1 — Write review comments on a pull request's diff 🎯 MVP

- **Tasks**: T001–T011
- **Deliverable**: in Review changes a pull request's text files take picked lines and comments, which show on that pull request's diff only, are listed in the panel under a heading naming the pull request, are edited, deleted, tidied per scope and survive a restart; a pull request not in the list keeps its flagged group.
- **Satisfies**: US1 acceptance scenarios 1–5; FR-001, FR-002, FR-003, FR-004, FR-005, FR-013 (display), FR-014, FR-015
- **Verify**: `mise run test-core`, then `cargo test -p micold-client --test features_changes` and `cargo test -p micold-daemon --test review_edit` via `mise run gate`.
- **Depends on**: —
- **Tier**: full

### M2 — Send a pull request's review to the agent

- **Tasks**: T012–T022
- **Deliverable**: Send review on a listed pull request delivers one PR review batch of exactly that pull request's drafts, naming the pull request, head branch and head commit and asking the agent to apply, commit and push (never to post), with a switch-branch line when another branch is checked out; the comments become Sent and agent reports show on them. Split not possible along an acceptance scenario: US2 and US3 share `PrContext` and one batch contract.
- **Satisfies**: US2 acceptance scenarios 1–6; US3 acceptance scenarios 1–4; FR-006, FR-007, FR-008, FR-009, FR-010, FR-011, FR-013 (send unavailable), FR-016, FR-017
- **Verify**: `cargo test -p micold-core prompt`, `cargo test -p micold-core review::git`, `cargo test -p micold-daemon --test review_send --test review_reports`, `cargo test -p micold-client --test features_changes`.
- **Depends on**: M1
- **Tier**: full

### M3 — Comments follow the pull request's head

- **Tasks**: T023–T027
- **Deliverable**: when a pull request's head moves, the diff re-reads, comments keep their recorded commit, follow their quoted code and are marked outdated when it is gone; a merged or closed pull request keeps its comments editable and sendable.
- **Satisfies**: US4 acceptance scenarios 1–3; FR-012, SC-004
- **Verify**: `cargo test -p micold-core anchor`, `cargo test -p micold-client --test features_changes --test features_pr_changes`, `cargo test -p micold-daemon --test review_edit`.
- **Depends on**: M1, M2
- **Tier**: full
