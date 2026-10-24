# Contract: reviewed marks on the wire, in the store and from git (#1271)

Amends 482's review-wire, 946's and 1149's `contracts/review-wire-delta.md`. Their rules W1 to W21,
S1 to S3 and G1 stand. Rules here are numbered RM1 to RM9 so they never clash with them.
`PROTOCOL_VERSION` +1 (one bump for the whole feature; 68 to 69 at planning time, the next free
number when the wire milestone merges).

## RM1 SetReviewed

`ClientMsg::ReviewEdit { project, worktree_dir, edit: ReviewEditOp::SetReviewed { scope, path,
change } }`.

- `change: Some(id)` sets the mark of `path` in `scope` to `id` (a new mark, or a re-mark that
  replaces the old one); `None` removes it.
- Refused `InvalidInput`, memory and file unchanged: an unknown entry (W2); `path` not a valid
  entry-relative `/` path (482's `checked_path`); `Scope::PullRequest(0)`; `Scope::PullRequest(_)`
  for the Default entry (`worktree_dir == ""`, as W18).
- `ChangeId` is a type on the wire: a value that is not 40 hex characters fails to decode.
- Accepted while a send of the entry is open: a send freezes comments, not marks.
- Answer: `OperationOk(Ack)` once written.

## RM2 No change, no write, no push

A `SetReviewed` that leaves the entry's marks as they were (setting the id already held, removing
a mark that is not there) is answered `Ack` without a write or a push, as W19. This is what makes a
repeat from another window harmless.

## RM3 Push

After every write, `DaemonMsg::ReviewChanged { project, worktree_dir, comments, sending, marks }`
goes to every client attached to `project` (482's rule), `marks` holding **all** of the entry's
marks of every source, ordered by scope, then path. On attach, one `ReviewChanged` per entry that
has comments **or** marks (`pushes_on_attach`). Once an entry's comments and marks are gone, one
`ReviewChanged` with both lists empty.

## RM4 Last change wins

The daemon applies review edits one at a time under its state lock. Two windows changing the same
file's mark end with the state of the edit applied last, and every window receives that state by
RM3 (FR-011, Edge Cases > Concurrent windows).

## RM5 Kept until unmarked or the entry goes

No operation other than `SetReviewed` changes a mark: not `Add`, `SetText`, `Delete`, `ClearSent`,
`DiscardPending`, `Mark`, `Reanchor`, `ReviewSend`, a report, nor a pull request list or diff
change (FR-007, FR-016). An entry's marks are removed with its comments by 482's W11 rules:
`forget_worktree` (the app removes the worktree), `prune_worktrees` (it was removed outside the
app) and `forget_project` (the project is removed).

## RM6 Comments unaffected

`SetReviewed` never changes `comments` or `sending` in the push; a comment edit never changes
`marks`. A client that ignores `marks` sees exactly the comments it saw before.

## S4 Review file version 4

Written only while some entry has a mark; otherwise version 3 (some PR comment) or 2, as before.
Reads 1, 2, 3 and 4. A stored entry gains `"marks": [ { "scope": ..., "path": ..., "change":
"<40 hex>" } ]`, absent when empty; an entry with marks and no comments is written with
`"comments": []`. Versions 1 to 3 load with no marks and every comment as before (FR-012). A mark
whose `change` is not 40 hex characters, or whose scope is `PullRequest(0)`, makes the file
unreadable like any other malformed field (the file is set aside, 482's rule). Unknown versions
(5 and up) are set aside as today.

## RM7 Older peers

None can attach: the handshake refuses a peer of another `PROTOCOL_VERSION` or `SCHEMA_HASH`
(research R6). No capability flag and no mixed-version path are added.

## G2 Whole change read

`GitCli::whole_change(dir, &ReviewScope) -> io::Result<WholeChange>` and the matching `Git` trait
method; read-only, with the review git reads' deadline:

1. `git diff --no-ext-diff --no-textconv --no-color -z -M --raw --no-abbrev <rev>`, `<rev>` the
   merge-base for a worktree with a base, else `HEAD` (`--no-abbrev`, not `--full-index`: git 2.43
   still abbreviates `--raw` ids under `--full-index`, which widens only a patch's `index` line);
2. for a worktree with a base, `range_paths(<base>, HEAD)` and `range_paths(HEAD)` for the origin;
   otherwise every file is Uncommitted;
3. untracked, not-ignored files (`untracked`), and `git hash-object --stdin-paths` over the paths
   whose new id is all zeros plus the untracked ones, in one process; a path holding `\n` is not
   sent and its file has `id: None`. An untracked file becomes a `WholeFile` of kind added (old
   mode and id all zeros, new mode from disk), the same record git's added file gives, so its id
   does not change when it is staged or committed unaltered.

A failure of step 1 or 2 is the read's error; a failure of step 3 leaves the affected files with
`id: None`.

## Tests

| Rule | Where |
|---|---|
| RM1, RM2 | daemon `tests/review_edit.rs`: set, re-mark, unmark; each refusal; no write and no push on a repeat; accepted while a send is open |
| RM3, RM4 | daemon `tests/review_edit.rs`: two attached clients both receive the push; two edits of the same file end equal in both; attach push for an entry with marks only |
| RM5 | daemon `tests/review_edit.rs` / `review_send.rs`: Clear sent, Discard drafts, a send and a report leave marks; `forget_worktree`, `prune_worktrees`, `forget_project` remove them |
| RM6 | daemon `tests/review_edit.rs`: comments in the push unchanged by `SetReviewed` |
| wire | core `tests/protocol_roundtrip.rs`: `SetReviewed` with `Some` and `None`, `ReviewChanged.marks`, a `ChangeId` that is not hex fails to decode; `tests/schema_hash.rs` pin moves with the bump |
| S4 | core `review/store.rs` unit tests: v4 round-trip; marks-only entry written; no marks writes 3 or 2; v1 to v3 load with no marks; malformed `change` set aside; the "unknown version" tests move from 4 to 5 |
| G2 | core `tests/review_git.rs` against real temp repos: committed, uncommitted and both; untracked; rename; mode only; binary; the Default entry; no base; an unaltered commit keeps the id (US2 s5); untracked → staged → committed keeps the id of a new file; a new file committed unaltered in an entry with a base stays Reviewed; a reverted and redone change gets the same id (US2 s3, s6) |
