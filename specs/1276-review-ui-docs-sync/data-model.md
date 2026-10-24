# Data model: Review UI — docs, layout snapshot, visual pass and mockup sync

No production type is added. These are the test-side and record-side entities of the spec's Key
Entities.

## Review key binding (read, not added)

Owned by #1274 in `micold-core` (plan name `micold_core::review::keys::REVIEW_KEYS`; the merged name
wins).

| Field | Meaning | Used by |
|---|---|---|
| key | the chord as bound (`j`, `]`, `?`, `Ctrl+Shift+Enter`…) with its modifier set | guide `Key` cell |
| primary modifier | whether `Ctrl` becomes `Cmd` on macOS | guide `(… on macOS)` suffix |
| sheet text | what the `?` sheet says the key does | guide `Does` cell |

Order is the table's order. Validation: [contracts/guide-shortcut-table.md](contracts/guide-shortcut-table.md),
including what the test does when the table lacks sheet text or a primary-modifier flag.

## Guide shortcut row

`(key cell, does cell)` parsed from the body rows between `<!-- review-shortcuts:start -->` and
`<!-- review-shortcuts:end -->` in `docs/user-guide/reviewing-changes.md`; header and rule rows
skipped; exactly two cells per row. Equal to the rows rendered from the bindings, in order.

## Review state under test

An entry of `crates/micold-client/tests/support/covered_states.rs` (`CoveredState { name, build,
anchors }`).

| Field | Rule |
|---|---|
| `name` | `review-<what>` for new states; `session-area-changes-open` kept by name (research R5) |
| `build` | a `StateUnderTest` from fixed invented data; a press (`pressing`) when the state is opened by a control the app keeps private (e.g. the source dropdown) |
| `anchors` | every element the state introduces, named `review.<part>` ([contracts/review-states.md](contracts/review-states.md)); a conditional child gets an anchor so its absence fails |

Lifecycle: registered (fixture mismatch fails) → fixture re-recorded once (M2) → gated by
`review_tab_fits.rs` at two windows × two themes.

## Visual-pass check

| Field | Rule |
|---|---|
| id | `NN` two-digit order in [contracts/visual-pass-checks.md](contracts/visual-pass-checks.md) |
| crops | `visual-pass/NN-<slug>-light.png`, `visual-pass/NN-<slug>-dark.png`, cropped, 1280×800 frame |
| verdict | PASS or FAIL with one line of what was looked at |
| on FAIL | fix commit (styling / doc only) or issue link (FR-010) |

Recorded in `visual-pass/results.md` with date, environment and checks not run (with reason).

### Visual-pass world

A private data directory with: one project; worktree **A** with changes in ≥ 12 files across nested
directories (one deep path, one long name, one binary), 2 drafts, 2 sent comments of which 1 has a
reply, 3 files marked reviewed and 1 marked then changed; worktree **B** with its own open Review
tab and a different file selected; a worktree **C** with no changes; a stub `gh` first on the
daemon's `PATH` (a `sh` script, on the pattern of `crates/micold-core/tests/github_gh_cli.rs`) that
lists one pull request for worktree A, with a long title, and exits non-zero on its diff request, so
the PR source is listed and selecting it shows the "cannot be read" error (check 15). All invented:
nothing from the author's machine.

## Mockup board

| Field | Rule |
|---|---|
| board | one canvas page per visual-pass check group (tab, header + source, tree, diff + cards, bar states, drawer, sheet, stale, composer, empty, Overview) |
| components | shared: `TitleBar`, `Sidebar`, `StatusBar`, tab strip, review bar, drawer, source control, file tree row, comment card, shortcut sheet, `OverviewCard`; on `tokens.css` |
| in sync | every difference to its crop removed, or labelled "follow-up, not built" when the redesign record lists it as a follow-up (FR-012) |

Comparison recorded in `visual-pass/mockup-sync.md`: board, crop, differences found, action taken.

## Redesign record

`docs/development/review-redesign.md` sections after the change: status (built, epic #1277), Tasks
table with a PR column, open questions 0–5 each with its answer, *Differences from the proposal*,
*Follow-ups not built*.
