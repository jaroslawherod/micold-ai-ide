# Visual pass — 1003 review-worktree-prs

## M1 (T014: quickstart §B steps 1, 2, 6, without the check marks)

**Not run at a display in M1. T014 is unticked and moved to the close unit, beside T037 (quickstart §B in full).** Steps 1, 2 and 6
need `gh` signed in to a GitHub repository whose worktree has pull requests from several branches.
The M1 unit ran in a container with `gh` unauthenticated and no `xdotool`, so neither the client nor
GitHub could be driven.

What M1 changes on screen, and how it is covered instead:

- **More rows in the existing Pull requests section.** The section's drawing is unchanged since
  feature 775, which already draws one row per pull request of a branch (a re-opened branch gives
  several). M1 only feeds it the pull requests of every branch of the set. Covered by
  `pr_list_branch_set_asks_every_branch_created_in_the_worktree` (three branches, three rows in
  order) and `pr_list_branch_set_two_worktrees_each_ask_only_their_own` (step 6) in
  `crates/micold-client/src/main_tests.rs`, against real git reflogs.
- **The base line's head names the shown pull request's own branch** (research R6). Text only;
  covered by `the_base_line_names_the_shown_pull_requests_own_head_branch` in
  `crates/micold-client/tests/features_pr_changes.rs`.
