# Quickstart: full file and repo browsing in the Changes view (#1150)

## Part A: automated

```
mise run gate
```
Expect green. Key tests: `review::expand` (gaps; shape properties: whole-file covers every line once, numbering continuous, changed lines kept, collapse restores input; reconcile after refresh), `review::reference` (resolves to listed files only, relative and extension probing, line suffixes), `review::browse` + `tests/review_browse.rs` (real git: PR-head read equals the commit's blob while the worktree differs, `..` and symlink escape refused, tree of worktree vs head, untracked/ignored only for worktree), `pull_request` (`file_at`/`tree_at` args and failure mapping), `features_changes` / `features_pr_changes` (toggle, whole, per-source state, refresh, stale-answer drop, back stack restore, large "Show anyway", pick gate false for untouched files), geometry gates in `diff_view.rs` (gap row height, both layouts).

## Part B: visual pass (the real GUI; a worktree with a PR whose head differs from the worktree)

1. Open Review changes on a file with two distant hunks: gap markers say "N hidden lines". Expand the middle gap: lines appear numbered continuously with the diff's colouring, in unified and side-by-side. Collapse it.
2. Expand the leading and trailing gaps. Toggle whole-file: every line once, removed lines in place; toggle off.
3. Added file: no markers, no toggle. Deleted file and a binary: no expansion offered.
4. Select the PR: expand a gap; lines equal the PR head, not the edited worktree file. Switch back to the working tree: the gap is closed again (state per source).
5. Edit the file in the worktree while a gap is open: the view refreshes, open gap kept, numbers consistent.
6. Open the file tree and the path picker; open an untouched file: line numbers and colouring. Click a path/import in a diff line: that file opens at the line. Back twice: same file, scroll and expansion.
7. A file over 2 MB: note with "Show anyway"; shown after pressing; resets on closing the view.
8. Browse a missing path under the PR and a binary: message, Back works.
8b. Pick a revealed line where picking works; an untouched file is not pickable.
9. Worktree changes with the network off: expansion and browsing work. Offline PR with the head commit absent locally: message on that file, diff usable.
10. Light and dark themes.
11. A 5,000-line source file: expand a gap and toggle whole-file; lines show in under 1 s and scrolling stays smooth.
