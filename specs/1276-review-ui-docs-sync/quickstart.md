# Quickstart: validating Review UI docs, snapshot, visual pass and mockup sync

Prerequisites: #1269–#1275 and #1128 merged; branch rebased on `origin/main`; `mise trust` done
(or, where `mise` is missing, the steps of `scripts/gate.sh` run directly).

## A. Docs and tests

1. **Key table** — `mise run test-core` (or `cargo test -p micold-core --test review_keys_doc`):
   passes. Change one `Does` cell in the marker table: the test fails listing both rows; revert.
2. **No pre-redesign statements** (FR-002, FR-014) — each of these returns nothing from
   `docs/user-guide/reviewing-changes.md`, `terminal-panes.md`, `worktrees-and-sessions.md`,
   `settings.md`, `agent-tools.md`:
   `grep -n -i -E 'three columns|review panel|\*\*Close\*\*|within 2 s|replaces the terminal|preceded by a \*\*Pull requests\*\* section|Changes view'`; and
   no *Pull requests* / *Committed* / *Uncommitted* toggle described in the file list. Read each hit
   of `grep -n -i 'committed\|pull requests'` and confirm it describes the source control.
3. **Other pages** (FR-003) — `terminal-panes.md` lists the Review tab with badge and mark;
   `worktrees-and-sessions.md` row menu says **Review changes** opens or focuses the tab.
4. **Layout doc** (FR-008) — `docs/development/layout-snapshot.md` names every state of
   [contracts/review-states.md](contracts/review-states.md).
5. **Record** (FR-013) — `review-redesign.md` status built; R0–R7 rows with PRs; questions 0–5
   answered; differences and follow-ups listed.
6. **Docs CI** (FR-014) — `site/checks/links.sh --sources` passes; `scripts/tests/documentation-set.test.sh`
   passes; `cargo test -p micold-core --test documentation_is_not_read` passes.
7. **Layout** (FR-005–FR-007) — `cargo test -p micold-client --test layout_snapshot` and
   `--test layout_text_overflow` pass; +1 px on the review bar's padding fails naming `review.bar`;
   revert. CI green on Linux, macOS, Windows.
8. **Gate** — `mise run gate` green before push.

## B. Visual pass (FR-009, FR-010)

Invoke the `visual-pass` skill with: the worktree path; the checks of
[contracts/visual-pass-checks.md](contracts/visual-pass-checks.md); the world of
[data-model.md](data-model.md) § Visual-pass world; output to
`specs/1276-review-ui-docs-sync/visual-pass/`. Expected: two crops per check, `results.md` with a
verdict per check, every FAIL with a fix or an issue link.

## C. Mockup (FR-011, FR-012)

1. Read `https://claude.ai/artifact/MviqD4uqBNaZtsCKH7yuTu` with the Artifact tool.
2. For each board, compare with its crop from §B: labels, order, presence of controls, count formats,
   state colours; Overview board and `OverviewCard` per FR-011.
3. With *writer* access: fix or label each difference (FR-012) and republish to the same URL.
   Without: hand the list to the owner and note it in the spec status.
4. `visual-pass/mockup-sync.md` lists board, crop, differences, action. Expected: 0 unlabelled
   differences (SC-004).

## Results (close unit, 2026-10-27, `main` at 09ee4cb1)

- **A1** `review_keys_doc` 3 passed; with one `Does` cell changed it fails
  (`the_guides_table_is_the_review_key_table`), restored green.
- **A2** the grep's remaining hits are other dialogs' **Close** buttons (the review's **Show all**
  reply dialog at `reviewing-changes.md:799`, setup output, session row, daemon details) and settings'
  "within 2 seconds" path check: none describes the pre-redesign view.
- **A3** `terminal-panes.md:44` lists the Review tab with its badge and changed dot;
  `worktrees-and-sessions.md:1187` says **Review changes** opens or shows it.
- **A4** `layout-snapshot.md` names every covered state of `review_tab_fits`'s `REVIEW_STATES`
  (the contract's names map onto the slices' state names there).
- **A5** `review-redesign.md` status "built (epic #1277)"; checked by M5 review B.
- **A6** `documentation-set.test.sh` passed; `documentation_is_not_read` 3 passed;
  `site/checks/links.sh --sources` not run (no lychee here), CI's `docs check` green on #1545.
- **A7** `layout_snapshot` 162 passed, `layout_text_overflow` 3 passed; +1 px on the review bar's
  strip padding fails naming `path 0/0/0/1/1/2/2/0/1/0, inside review.bar`, restored green. CI
  green on Linux, macOS and Windows for #1495.
- **A8** the close PR's `mise run gate` (recorded in its body).
- **B** not re-run: `visual-pass/results.md` (M3) holds 34 crops with verdicts; its FAILs are fixed
  or filed (#1519, #1539, #1540).
- **C** not re-run: `visual-pass/mockup-sync.md` (M4) has an action on every row; M4 review B
  round 2 re-read canvas version 11.
