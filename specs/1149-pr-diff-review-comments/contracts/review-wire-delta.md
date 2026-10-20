# Contract: wire, store and git delta (amends 946's contracts/review-wire-delta.md)

`PROTOCOL_VERSION` 56 to 57 (one bump). 482 and 946 rules W1 to W17 stand unless amended.

## W18 Add with a pull request anchor

`ReviewEditOp::Add { pr: Some(PrAnchor) }` creates a PR comment. Refused `InvalidInput`: for the
Default entry (`worktree_dir == ""`), `number == 0`, empty `head_branch` or one with a control
character, `head_commit` not 40 or 64 hex digits (stored lower-cased). All 482 refusals stand.

## W19 Scoped tidy

`ClearSent { scope }` and `DiscardPending { scope }` touch only comments whose `scope()` equals
`scope`; `Scope::PullRequest(n)` for the Default entry is a no-op `Ack`. No write and no push when
nothing changed.

## W20 ReviewSend with a pull request

`ReviewSend { pr: None }` sends the Drafts of the working tree only; `pr: Some(PrSend)` sends the
Drafts of `Scope::PullRequest(pr.number)` only. Validation as W18 plus `title` any text (flattened,
R5) and `current_head` hex. The daemon, off the state lock:

Reads: `Git::checked_out_branch`/`has_commit` below mean the `GitCli` methods of G1.

1. for `pr: None`: W16 (relocate working-tree Drafts); PR comments are never relocated against the
   working tree;
2. for `pr: Some`: reads `GitCli::checked_out_branch(entry dir)` and `GitCli::has_commit(entry dir, c)` for
   each distinct recorded commit of the scope's Drafts, then builds `PrContext`. A git failure
   counts as `None` / not available, never as a refusal;
3. opens the send for the scope (`Busy` if any send of the entry is open; `NothingPending`
   when the scope has no Draft) and delivers as W6 to W9.

`outdated` ids apply to both sides of PR comments and to the removed side of working-tree ones.
The answer is `ReviewSent` as before. The title is neither logged nor stored; the audit log line
names the pull request number only.

## W21 Reanchor on a pull request

`Reanchor` is accepted for PR comments as for working-tree ones (compare-and-set on `from`). The
client sends it from a pull request's diff (946 R3 no longer excludes it).

## S3 Review file version 3

Written only while some comment has `pr`; otherwise version 2 as before.

`pr` per comment (data-model.md). Unknown versions quarantine as 482. A PR comment never loads as
a working-tree comment.

## G1 Git additions

Inherent `GitCli` methods in `review/git.rs` (not the `Git` trait): `checked_out_branch(&self, dir) ->
Option<String>`; `has_commit(&self, dir, sha) -> bool`. They run `git symbolic-ref --short -q HEAD` and `git cat-file -e <sha>^{commit}` with
the review git reads' deadline; a sha not matching 40 or 64 hex digits returns false without
running git. Core unit tests run them against real temp repos (`tests/support/attach_fixture.rs` helpers).

## Tests

`protocol_roundtrip.rs`: the new fields round-trip, `BranchPrState` (now `Serialize, Deserialize`) in `PrSend` included. `review/store.rs`: S3. `review_edit.rs`: W18
(each refusal), W19 (scope isolation, no write when nothing changed), W21. `review_send.rs`: W20
(scope isolation, branch and commit reads against a real temp repo: another branch checked out, a missing sha, failure leaves Draft, concurrent sends).
