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

## Close unit (T014, T021, T027, T036: quickstart §B steps 1–6), run 2026-10-19

**Run at a display.** The M1–M4 units could not (no `gh` auth, no `xdotool`). The close unit's
container has Xvfb and ImageMagick; `apt-get update && apt-get install -y xdotool
libxkbcommon-x11-0` supplied the rest. Real `gh` is signed out here and there is no GitHub
repository, so the pass used **a stand-in `gh`** (`visual-pass/fake-gh.py`, first on `PATH`): it
answers the list query (`p<i>` aliases) with canned pull requests per head branch and the diff call
(`repos/…/pulls/N`) with `git diff main...<head oid>` of the fixture repo. `visual-pass/seed.sh`
builds the repo (worktree `wt1` with branches a, b, c created in it; `wt2` on d; origin
`github.com/acme/widgets`). What this proves: the client's drawing, stepping, refresh and
unavailable handling against the real binary. What it does not prove: GitHub's real answers (the
parsing is covered by `pull_request_list.rs`).

Setup: client and daemon built together and pinned, `ICED_BACKEND=tiny-skia` on a private Xvfb
(1600×1200), own `XDG_DATA_HOME`/`XDG_RUNTIME_DIR`/`HOME`, "Show pull request status" turned on
through Settings, GitHub (step 1). Evidence crops are in `visual-pass/`.

| Task | Step | Result | Evidence |
|---|---|---|---|
| T014 | 1, 2, 6 | **Pass.** "Review changes" on Wt1 lists #43, #42, #41, newest first, each with its state glyph (open, closed, merged). Wt2 lists none of Wt1's pull requests (weak by itself: the stand-in has no entry for branch `d`; step 6's filtering rests on `pr_list_branch_set_two_worktrees_each_ask_only_their_own` in `main_tests.rs`). | `step2-list.png`, `step5-empty-and-step6-other-worktree.png` |
| T021 | 2 | **Pass.** Check mark on every entry: pending clock (#43), red cross (#42), tick (#41); glyphs are distinct and beside the state glyph without overlap. | `step2-list.png` |
| T027 | 3, 4 | **Pass.** Pressing #42 shows its header (state, check, `#42 Rework widget B`, base line) and its files; previous/next walk #41 → #42 → #43; previous is greyed on the first entry and next on the last; "Working tree" returns to the working-tree changes. A fourth pull request added to the stand-in appeared on top after a worktree-list refresh while #42 stayed shown. | `step3-*.png`, `step4-before.png`, `step4-after.png` |
| T036 | 5 | **Pass.** A worktree without pull requests shows "No pull requests from this worktree yet". With `gh` signed out (stand-in exits 4, "gh auth login"), a freshly opened view shows the muted "Pull requests can't be shown for this worktree", no notice. | `step5-empty-and-step6-other-worktree.png`, `step5-unavailable-signed-out.png` |

Not seen: the "Reading pull requests…" line. It lasts one stand-in call, well under the shot
interval. The view-model tests pin `PrSection::Loading`, but no test or pass shows the drawn text
(`PRS_LOADING`): **that one US4 line is unverified at the display.** The empty and unavailable lines
are verified by the crops. A failed diff read
first showed "The pull request's diff could not be read; select it again to retry" while the
stand-in lacked the diff call (a mistake in the stand-in, then fixed): that line drew correctly with
the header and previous/next still usable (FR-016).

**Base line shown twice (the M3 review's possible duplicate): confirmed, left as is.** With a pull
request shown, the page heading's muted base line (`base_line`, for a pull request it is
`pr_header(..).base`) and the diff area's header caption repeat the same text
(`Pull request #41: main ... a`). Both are specified: data-model.md lists `base_line` in the header
and 775 owns the heading line. Cosmetic, no behaviour; recorded as a follow-up for the issue's owner
rather than changed in a close PR.
