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
