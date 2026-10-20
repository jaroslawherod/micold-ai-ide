# Contract: the PR review batch (amends 946's contracts/batch-format.md)

The working-tree batch is unchanged. A PR review batch is blocks joined by one blank line, `\n`
only, as in 946: preamble 1, preamble 2, optional notes, then one block per comment.

## P-1 Preamble 1

```
Please work through these review comments on pull request #<number> "<title>" (head branch <head_branch>, head commit <current_head>)<state>. Each block below is one comment: its reference, the file (relative to the worktree root), the lines it is about, the code on those lines at the pull request's head commit when the comment was written, and the comment itself.
```

`<state>` is empty for open and draft, `, which is merged` or `, which is closed`. `<title>` is the
title flattened to one line (control characters to spaces, runs of spaces kept, at most 200
characters, cut on a character boundary).

`<current_head>` is the pull request's head commit now (full sha). The commits the comments were
recorded at are named in note 2 when any differs from it, and in the block line when they differ
among themselves.

## P-2 Preamble 2: what to do

```
For each comment, either make the change it asks for, or explain why you are not making it. Commit the changes you make and push them to the branch <head_branch>. Then report each outcome by the comment's reference with the report_review_comment tool of the micold tool server: outcome "applied" when you made the change (a reply is optional), or "answered" with your explanation as the reply. If you cannot use that tool, answer each comment by its reference in your reply instead.
```

The batch never asks for anything to be posted, and never contains the words "post" or "GitHub"
(SC-003; tested on every variant).

## P-3 Notes (each only when it applies, in this order, each its own paragraph)

1. Branch: when `checked_out != Some(head_branch)`:
   `This worktree does not have <head_branch> checked out. Switch to <head_branch> before changing anything.`
2. Head moved: when any recorded commit differs from `current_head`:
   `The comments were written against commit <c1>[, <c2>...]; the pull request's head is now <current_head>, so the code may have moved. Find it by the quoted code.`
   (recorded commits sorted, without repeats).
3. Unreadable: when `unreadable` is not empty:
   `Commit <c1>[, <c2>...] is not available in this worktree; the code is quoted as recorded.`

## P-4 Blocks

946's block, byte for byte (heading `### Comment C-xxxxxxxx: <path>, line N (current)`, outdated
note, fence, quote with 482 elision, text), ordered by path, start line, side, created, id. When
the batch's comments hold more than one recorded commit, each block's heading is followed by the
line `Written against commit <head_commit>.` (before the outdated note).

`line N` / `lines N-M` is the comment's current place (`range`, which a re-anchor may have moved);
the recorded commit applies to the quoted code only.

## P-5 Example (open pull request, other branch checked out, head moved)

~~~~
Please work through these review comments on pull request #12 "Add retry" (head branch feat/retry, head commit <current_head>). Each block below is one comment: ...

For each comment, ... push them to the branch feat/retry. Then report ...

This worktree does not have feat/retry checked out. Switch to feat/retry before changing anything.

The comments were written against commit 1111...1111; the pull request's head is now 2222...2222, so the code may have moved. Find it by the quoted code.

### Comment C-0000000a: src/a.rs, line 7 (current)
```
let x = 1;
```
Name this.
~~~~

(the full bytes are fixed by the test `p1_pr_example_gives_the_exact_bytes`, which the user guide's
"The review message" section quotes.)

## Tests

`prompt.rs` units: P1 example; open, draft, merged, closed; branch checked out (no note), another,
detached; head equal (no note), moved; unreadable; two recorded commits (per-block line); title
with `\n`, `\x1b`, 300 characters; forbidden words absent; byte-identical on two calls.
