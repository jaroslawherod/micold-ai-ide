# Implementation Plan: Review comments on a pull request's diff, sent to the agent to apply and push

**Branch**: `feat/1149_review-comments-on-a-pull-request-s-diff-sent-to` | **Date**: 2026-10-20 | **Spec**: [spec.md](./spec.md)

**Input**: Feature specification from `specs/1149-pr-diff-review-comments/spec.md`

## Summary

Review comments (482, 946) gain a **scope**: the working tree, or one pull request of the entry.
A PR comment is an ordinary `ReviewComment` plus a `PrAnchor { number, head_branch, head_commit }`
(research R1). Everything keyed by entry stays keyed by entry; only the operations that act on "the
drafts" (send, clear sent, discard drafts, counts, the diff's cards, the panel) take a scope
(R2). The client records the head commit of the diff it shows, lets lines be picked on a pull
request's diff, re-reads the diff when the list shows a new head, and re-anchors PR comments
against the diff's hunks (R3, R4). Send review on a pull request sends a `PrSend` context with
`ReviewSend`; the daemon builds a PR review batch (`prompt::build_pr`, contracts/batch-format.md),
reads the entry's checked-out branch and whether the PR head commit is local, and delivers as 482
does (R5, R6). Posting to GitHub is not done anywhere. Protocol 56 to 57, review file 2 to 3 (only with PR comments).

## Technical Context

**Language/Version**: Rust stable, iced GUI.

**Primary Dependencies**: existing only: `micold-core` (`review::{comment, prompt, anchor, store,
diff}`, `protocol::messages`, `git`), `micold-daemon` (`review.rs`, `state.rs`, `server.rs`),
`micold-client` (`features/changes.rs`, `shell/changes.rs`, `ui/changes.rs`). No new crate.

**Storage**: `reviews/<project id>.json`, the file 482 wrote, version 3 (R7). Local only.

**Testing**: `mise run gate`; core unit tests inline in `review/*.rs`; `crates/micold-core/tests/
protocol_roundtrip.rs`; daemon `tests/review_send.rs`, `review_edit.rs`; client reducer tests
`tests/features_changes.rs` (render-free); quickstart §B for the look.

**Target Platform**: Linux, macOS, Windows. Git runs as argument vectors; the batch is pure text.

**Project Type**: desktop app (Rust workspace) with a daemon.

**Performance Goals**: no new read: the PR diff is already in memory (`PrDiffLoad::Ready.patches`);
anchoring parses at most the files that hold comments.

**Constraints**: offline-capable (FR: sending needs no GitHub access); PR title is neither logged
nor stored (R5); deterministic batch bytes (FR-008, SC-005).

**Scale/Scope**: same limits as 482 (comment count, quote elision, 20,000-character texts).

## Constitution Check

- [x] **I. Test-First**: scope filtering, `build_pr`, anchoring on hunks, store round-trip, wire
  and reducer rules are pure core / render-free client logic and are written test-first. Only
  `ui/changes.rs` drawing (a scope heading in the panel, the "no pull request" flag) and `shell/`
  wiring use the quickstart exception, and carry no decision logic.
- [x] **II. Multi-Session**: comments stay per entry and now per pull request within the entry; a
  send of one scope never touches another's drafts; one open send per entry as in 482 (FR-010).
  Daemon integration tests cover two scopes and two windows (Isolation gate).
- [x] **III. Worktree**: the daemon reads the entry's branch and commit in the entry's own
  directory, read-only; nothing creates, changes or removes a worktree. The Default entry shows no
  pull requests and the daemon refuses a PR scope for it.
- [x] **IV. Local-First**: comments and their pull request anchor are plain files; sending needs
  no network; the pull request's title travels only client to daemon to the session, is not stored
  or logged (R5). Nothing is posted anywhere.
- [x] **V. Rust + iced**: `Scope` and `PrAnchor` are types, so a comment cannot be both: a
  `ReviewComment` has `pr: Option<PrAnchor>`, `Scope` is derived from it (R1).
- [x] **VI. Cross-Platform**: the batch is built from the comment data alone, with `\n` only;
  git is called through `GitCli` argument vectors; CI runs all three.
- [x] **VII. Documentation**: `docs/user-guide/reviewing-changes.md` gains commenting on a pull
  request, the PR review batch and the statement that nothing is posted (FR-017); its "The review
  message" section changes with `build_pr`.
- [x] **VIII. Reusable UI**: no new widget: the existing comment card, composer, panel row and
  `Button` are reused; the panel gains a group heading built from the existing `Text` styles.

No violation: Complexity Tracking is empty.

## Requirement map

| FR | Where |
|---|---|
| FR-001 | `features::changes::can_pick` accepts `Source::PullRequest` with a text file; `Msg::GutterPressed`, `composer_saved` build the Add with the shown PR's anchor |
| FR-002 | `ReviewComment.pr: Option<PrAnchor>`; `ReviewEditOp::Add { pr }`; `PullRequestView.head` |
| FR-003 | `Scope`, `ReviewComment::scope()`; `file_comments`, `pending_counts`, `jump_target` filter by `view_scope(view)` |
| FR-004 | `panel_rows` groups: working tree, then pull requests by number; `PanelRow.scope`; `ui/changes.rs` group headings |
| FR-005 | `store.rs` version 3 stores `pr`; discard with the worktree is 482's `forget_worktree`, unchanged |
| FR-006, FR-015 | `EntryReview::{begin_send, clear_sent, discard_pending}(scope)`; `ReviewEditOp::{ClearSent, DiscardPending}{ scope }`; `send_action`, `tidy_actions` count the scope's comments |
| FR-007, FR-008 | `prompt::build_pr`, contracts/batch-format.md |
| FR-009 | `Reviews::begin_send` takes `PrContext`; `AppState::review_send` reads `checked_out_branch`; delivery code unchanged |
| FR-010 | unchanged 482 open send, one per entry; scope does not split it |
| FR-011 | `report` keyed by `CommentRef`, unchanged |
| FR-012 | `anchor::Lines` trait + `HunkLines`; `judge_anchors` for PR comments; `reanchor_effect` allowed for a PR; diff re-read when the listed head changes |
| FR-013 | `panel_rows` flags a scope whose number is not listed; `send_action` is `None` for it |
| FR-014 | `can_pick` unchanged rule on `FileDiff::Text` |
| FR-016 | all of the above are core/render-free; daemon tests use a fake session sink as 482's |
| FR-017 | `docs/user-guide/reviewing-changes.md` |

## Test strategy

| Requirement | Layer and test |
|---|---|
| Scope on the comment, FR-002/003 | core unit `review/comment.rs`: `add` with and without `pr`; `scope()`; `begin_send(Scope)` sends only that scope, `NothingPending` when only other scopes have drafts; `clear_sent`/`discard_pending` per scope |
| FR-005, SC-006 | core unit `store.rs`: v3 round-trips `pr`; a file without PR comments is written as 2; v1 and v2 load as working tree; unknown version (4) still quarantined. The two `store.rs` tests asserting version 3 is quarantined (~lines 192 and 377) change to version 4 |
| FR-007, FR-008, SC-003, SC-005 | core unit `prompt.rs`: `build_pr` exact bytes for the contract example (open, merged, closed; branch checked out or not; head moved; commit unreadable; two recorded commits); no "post" or "GitHub" in any variant; same input twice gives the same bytes; title with control characters is flattened |
| FR-012, SC-004 | core unit `anchor.rs`: `locate` over `SideLines` unchanged, over `HunkLines` finds Same, Moved, Gone; client reducer (incl. the research R4 limitation: a quote outside every hunk is Gone): head change re-reads the diff, comments keep `head_commit`, outdated flag set |
| wire | `protocol_roundtrip.rs`: `Add { pr }`, `ClearSent { scope }`, `DiscardPending { scope }`, `ReviewSend { pr }`, a `ReviewComment` with `pr` |
| FR-006, FR-009, FR-010, SC-002 | daemon `review_send.rs`: 10 comments on 3 files of one PR plus a second PR and working-tree drafts: only the first PR's are delivered, others stay Draft; temp-repo daemon test (real git, as `mutation_semantics.rs`): another branch checked out gives the switch line, a sha absent from the repo gives the "not available" line; delivery failure leaves Draft; two concurrent sends, one `Busy` |
| FR-013 | client reducer: PR not listed gives flagged group, `send_action` `None`, comments kept |
| FR-001, FR-014, SC-001 | client reducer: pick on a PR text file builds an Add with the PR anchor; binary and over-size accept no pick; selecting PR B hides A's comments |
| FR-011 | daemon `review_reports.rs`: a report on a PR comment is shown as for a working-tree one |
| Look | quickstart §B visual pass (`visual-pass` skill) |

## Project Structure

### Documentation (this feature)

```text
specs/1149-pr-diff-review-comments/
├── plan.md  research.md  data-model.md  quickstart.md
├── contracts/batch-format.md       # the PR review batch, byte for byte
├── contracts/review-wire-delta.md  # wire, store and GitCli changes
└── tasks.md                        # /speckit-tasks
```

### Source Code

```text
crates/micold-core/src/
├── review/comment.rs   # + PrAnchor, Scope, ReviewComment.pr, scope(); begin_send/clear_sent/discard_pending take a scope
├── review/prompt.rs    # + PrContext (reuses BranchPrState), build_pr (shared block code with build)
├── review/anchor.rs    # Lines trait (SideLines impl), HunkLines; locate generic over Lines
├── review/diff.rs      # HunkLines::from_diff over FileDiff hunks, per side
├── review/store.rs     # version 3, StoredComment.pr
├── review/git.rs       # inherent GitCli::checked_out_branch, GitCli::has_commit (beside review_base)
└── protocol/{messages,version}.rs  # Add.pr, ClearSent/DiscardPending scope, ReviewSend.pr, PrSend; PROTOCOL_VERSION 57
crates/micold-daemon/src/
├── review.rs           # apply_edit/begin_send take scope; relocate skips PR comments
├── state.rs            # review_send: PrSend validation, branch and commit reads, build_pr
└── server.rs           # pass the new fields
crates/micold-client/src/
├── features/changes.rs # PullRequestView.head, ViewScope, can_pick, judge_anchors, panel_rows, send_action, tidy_actions, Effect::ReviewSend.pr
├── shell/changes.rs    # carries the new Effect fields to the wire
├── ui/changes.rs       # panel group headings, "not in the list" flag
docs/user-guide/reviewing-changes.md
```

**Structure Decision**: decisions in `micold-core` and the render-free `features/changes.rs`; the
daemon only reads git and delivers; `shell/` and `ui/` carry fields and draw.

## Complexity Tracking

None.
