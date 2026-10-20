# Quickstart: Review comments on a pull request's diff (#1149)

## Part A: automated

`mise run test-core` (comment, prompt, anchor, store, protocol), then `mise run gate`. The
acceptance rows of spec.md map to the tests in plan.md § Test strategy; none needs a window, a
session or the network (FR-016).

## Part B: visual pass (`visual-pass` skill)

Prerequisites: a worktree with two pull requests listed (open, one merged), `gh` signed in or the
fixture list of the showcase; an agent session on the worktree.

1. Open Review changes, select the first pull request: its diff shows, a gutter click picks a line,
   a range extends with shift. Write a comment: the card hangs under the lines, the panel lists it
   under a heading with the pull request's number and title.
2. Select the second pull request, then the working tree: the first one's card is absent on both;
   the panel still lists it under its own heading.
3. Press Send review on the first pull request: only its comments go; the working-tree drafts and
   the second pull request's stay Draft. The session's input shows the batch (contracts/
   batch-format.md): number, title, head branch, the push request, no mention of posting.
4. Check out another branch in the worktree and send again: the switch-branch paragraph is there.
5. Push a commit to the pull request's branch that moves a commented line, wait for the list
   refresh: the card follows its code; one whose code is gone carries the outdated mark.
6. Merge or close the pull request and send: the preamble says merged or closed.
7. Make the pull request leave the list (close the setting or change branches so it is unlisted):
   its group stays in the panel flagged, Send is unavailable.
8. Restart the app: all comments are back under their pull requests.

Record light and dark theme screenshots of steps 1 and 3 in `visual-pass/`.
