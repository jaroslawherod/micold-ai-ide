# Contract: the Changes view's hold (client reducer, M3)

`features::changes::update` behaviour, tested in `crates/micold-client/tests/features_changes.rs`
and the reducer's own tests. "Watch read" = a `ListRead` answering a read started by `Msg::Changed`
with `apply_next` unset. "List on screen" = `view.list` is `Ready` with at least one row (or
`Loading { last: Some(non-empty) }`). Types: [data-model.md](../data-model.md) §10–§13. Builds on
482's contracts (`specs/482-worktree-changes-review/contracts/changes-view.md`).

## Holding

- **H1** A watch read with a list on screen that differs (`observe` → `Held`): `view.list`,
  `selected`, `diff`, `diff_offset`, `pick`, `composer` and `anchors` are unchanged; `view.held`
  holds the latest read; no `ReadDiff` is issued (FR-005, FR-006, FR-014, AS1).
- **H2** A watch read that does not differ (`Same`): the latest list replaces `view.list`, `held`
  stays `None`, and no `ReadDiff` is issued (R4).
- **H3** A user read (Committed/Uncommitted toggled, source chosen, view reopened) sets
  `apply_next`; when it ends, the list is replaced as today, `held` is `None` (AS7). A user read
  queued behind a watch read (`again`) still applies.
- **H4** A watch read while held: `held` is re-observed against the shown list: the set grows,
  shrinks (AS4) or empties (→ H2).

## Notice and dots

- **H5** `stale_notice` is `None` with nothing held, *1 file changed since you looked* for one, *n
  files changed since you looked* for n > 1 (FR-007).
- **H6** `changed_since_you_looked(view, p)` is true exactly for listed paths in the set; a path
  only in the latest read counts in n with no row (FR-008).

## Selecting under a hold (FR-009)

- **H7** Selecting a file not in the set issues `ReadDiff` under the shown list's scope, as today
  (AS9).
- **H8** Selecting a file in the set that the latest read lists issues `ReadDiff` under the latest
  read's scope and row (`from` of a rename); its answer's `row` is adopted (`Held::adopt`): the row
  takes the new counts, loses its dot, n drops by one, other held files stay held (AS10).
- **H9** Selecting a file in the set that the latest read no longer lists issues no read;
  `no_longer_changed` is true and the diff area says the file has no changes any more; the row
  keeps its dot (AS11).
- **H10** With nothing held, a working-tree `DiffRead` with `DiffRow::Row` writes it into the shown
  list, so the next watch read compares against it (Edge *Selected between reads*).
- **H10a** `DiffRow::NotRead` (a non-working-tree read, or a failed `change_list_of`) adopts
  nothing: the diff shows, the file keeps its dot, the set is unchanged. `DiffRow::Unlisted` under a
  hold sets `no_longer_changed` as H9; with nothing held it leaves the shown list unchanged.
- **H11** Pull request: a head refresh that differs sets `pr.held` and leaves `pr.diff`,
  `pr.selected`, `pr.file` unchanged (AS8). Selecting a file in `pr.held`'s set adopts its row and
  patch from `held.latest` and shows that patch; another file shows the held snapshot's patch.

## Applying

- **H12** `Msg::StaleApplied`: `Held::apply`; `held` is `None`; the selected file stays selected
  while listed (AS2); its diff is read again keeping the old one on screen meanwhile (the refresh
  path `request_diff(state, true)`); anchors are re-judged; for a pull request `pr.diff` becomes
  `held.latest` and `pr.file` is re-parsed.
- **H13** Applying keeps the composer's text, and keeps picked lines where today's refresh keeps
  them (C4, `composer_placed`); an unsaved new comment on a file that leaves the list is closed as
  today (Edge *Open file removed*) (FR-010, AS6).
- **H14** While held, `ComposerEdited`, `ChipPressed` and `ComposerSaved` behave as without a hold;
  a saved comment quotes the lines shown (FR-011, Edge *Comment written while stale*).

## No list on screen

- **H15** A watch read with no list on screen (empty list, `Idle`, first read, `Failed`) replaces
  it at once, as today (FR-012). A failed watch read with a list on screen keeps the list and the
  hold as they were and shows no error (Edge *Re-read fails*).

## Reviewed marks (M6, after #1271)

- **H16** A file in the held set that #1271 marks reviewed is shown unseen with the dot, and *n of m
  reviewed* counts it as not reviewed (FR-013); nothing is written to the service by the hold.
