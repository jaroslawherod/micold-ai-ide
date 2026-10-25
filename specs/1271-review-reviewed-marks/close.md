# Close — #1271 Review UI: reviewed marks and progress

Closed 2026-10-25. Every milestone is merged except M5 (#1419, waiting for CI when this was
written); this close PR is stacked on it.

## What shipped

| PR | Milestone | Content |
|---|---|---|
| #1360 | design | spec, plan, tasks, contracts |
| #1374 | M1 | micold-core: change ids (`ChangeId`), whole-change read, `EntryMarks`, row states, progress, review file v4 |
| #1390 | M2 | `SetReviewed` and `ReviewChanged.marks` on the wire; the service keeps, pushes and removes marks; the client holds them |
| #1411 | M3 | row states (○ / ◉ / ✓), **Mark reviewed** control, *n of m reviewed* and bar, user guide |
| #1417 | M4 | changed-since dot, other windows' marks in the view, guide text |
| #1419 | M5 | pull request marks and progress (`pr_change_ids`), guide text |
| this PR | close | redesign note (R4 done), quickstart evidence, this document, spec status |

## Decisions

D1–D9 are in `autopilot.md`. The ones a reader should know:

- **D4 (answered by jaro, 2026-10-25, option A)**: while a toggle hides part of a file's change,
  **Mark reviewed** is unavailable (tooltip names Committed and Uncommitted); unmarking stays
  available.
- D1: a mark holds while the file's whole change is the same, not per head commit.
- D2: a marked file with drafts shows reviewed; the draft count stays.
- D5: milestone cut; M1 kept whole (core only, must land before #1269).
- D6, D7, D8, D9: where the plan's text did not fit the code (id digest inputs, `--no-abbrev`,
  `NotFound` for unknown entries, #1269's `ReviewTabs`, font glyphs, PR file lists).

## Deviations

- milestones.md rule 1 waived for M1 and M2 (D5): no P1 story, tests only.
- M1 and M2 each needed orchestrator CI or rebase fixes after the PR text (ledger, Pull requests).
- `speckit-analyze` and the `visual-pass` skill were not invocable; FR/SC coverage was checked by
  grep and the visual passes were run by the orchestrator by hand (`visual-pass/README.md`).
- T046: `mise` is not installed here. `cargo test -p micold-core --all-targets --no-fail-fast`:
  4349 passed, 4 failed; the four are the root-only permission tests that fail on origin/main too
  (`settings_refuses_save_over_failed_read`, `settings_write_is_logged` family,
  `worktree_leftovers` x2). The full `scripts/gate.sh` is run by the orchestrator.
- T047: no new GUI pass; the per-milestone evidence is collected instead. The two-window check is
  not exercisable in cloud (second client is read-only).
- Not run: quickstart Part B step 9 (restart the app) as a separate step; covered by the store
  tests.

## Follow-ups not done

| # | Follow-up | Issue? |
|---|---|---|
| 1 | The Changes view has no layout below about 900 px session width: at 640x480 with the sidebar open the list pane (~70 px) is narrower than the **Committed** / **Uncommitted** chips, the header takes the whole width leaving the heading 0 px, and the pane overflows onto the status bar without scrolling. Pre-existing on main. | Yes: its own layout issue (touches #1269/#1270 header and drawer). |
| 2 | On a daemon reconnect the client drops the open review and returns to the project page. Predates this branch (`on_connected`, `ReviewTabs`, catalog reconcile). | Yes: bug. |
| 3 | A type change (file to symlink or back) prints two `diff --git` sections for one path in a pull request: two rows, *n of m* counts one, both share the second section's id and mark. Pinned by `a_type_change_lists_two_rows_and_counts_one`. | Yes, low priority: edge case, needs a decision on keying patches by path and section. |
| 4 | With the Comments panel (#1270's SideSheet) open, it covers the diff header's **Mark reviewed** chip. | Yes: small UI issue; or fold into #1270 follow-up work if still open. |
| 5 | The M3 B1 mutation record leaves `wholeload`'s four and `nomarks`' two further failing tests unnamed. | No: record only, costs a build per mutant for no gain. |
| 6 | M1 has no `tdd/cycle-log.md`. | No: process note. |
| 7 | `core.symlinks=false` symlink digest as 100644 (declined in M1 review A r1): git records 120000 regardless. | No: declined with evidence. |
| 8 | Root-only core permission tests fail as root (4 tests). | No: environment, not this feature. |
