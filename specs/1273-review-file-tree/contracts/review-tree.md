# Contract: the review tree (#1273)

Two interfaces: §1 is the core function the client and its tests call; §2 is what the list pane
shows. Types are in [../data-model.md](../data-model.md).

## 1. `micold_core::review::tree::layout`

```text
fn layout(leaves: &[Leaf], options: &LayoutOptions) -> Vec<Row>
```

Pure and deterministic: the same input gives the same rows on every platform.

| # | Given | Rows |
|---|---|---|
| L1 | `src/a.rs`, `src/ui/b.rs`, `README.md`; Path; no filter | `src` (d0), `ui` (d1), `b.rs` (d2), `a.rs` (d1), `README.md` (d0) — scenario 1.1 |
| L2 | only files under `crates/core/src/review/`; `chain: true` | one folder row, label `crates/core/src/review`, `paths` = the four real folders — 1.2 |
| L3 | L1 with `src` in `collapsed` | `src` (open: false), `README.md`; expanding restores `ui` with its own state — 1.3 |
| L4 | chained `a/b/c` with `a/b` collapsed | the chained row shows `open: false` (edge case "chained folder splits or merges") |
| L5 | `x.rs` +40 −10, `y.rs` +1 −1, under one folder | folder row `added: 41`, `removed: 11` — 2.5 |
| L6 | Size; files with totals 5, 100, 10 in one folder; two folders with sums 3 and 50 | folders: 50 then 3; files: 100, 10, 5; equal totals keep Path order — 4.2, SC-004 |
| L7 | UnreviewedFirst; reviewed = {`src/a.rs`} | in `src`: unreviewed files before `a.rs`; a folder with any unreviewed file before one with none — 4.3 |
| L8 | filter `tests` over `src/daemon/tests.rs`, `src/lib.rs`, `docs/tests.md` | `tests.rs` under `src/daemon`, `tests.md` under `docs`; no `lib.rs`; spans on `tests` in each name — 3.1, 3.5 |
| L9 | L8 with `src` in `collapsed` | `src` shown open while the filter holds text — 3.2 |
| L10 | filter matching nothing | empty `Vec` (the client shows the no-match line) — 3.3 |
| L11 | Size + filter | the filtered rows in Size order; totals over listed files — 4.4 |
| L12 | `chain: false`, `DefaultClosed { open: {} }`; also an unsorted input | today's **Files** rows: top-level folders closed, no chaining; files of a level in byte order whatever the input order — FR-017 |

Other functions (`TreeChoices` methods, `sort_options`, `bar`, `bar_scale`, `indent_levels`,
`max_indent_levels`, `hover_text`) follow data-model §2, §4, §5; each has its own unit tests.

## 2. The list pane

Shown only when `list_body` is `ListBody::Files` (FR-016); the loading, failed, empty and
pull-request-unread states keep today's text and look, with no filter box and no sort chips.

Top to bottom, inside the left column #1272 provides:

1. **Filter box**: `TextField`, placeholder `Filter files…`, leading search icon, trailing clear
   action when non-empty (`Msg::TreeFilterCleared`); `Esc` while focused sends the same message through `TextField::on_escape` (research R5).
2. **Sort chips**: `Path`, `Size` (and `Unreviewed first` per `sort_options`), one active.
3. **Rows** (`VirtualRows`, fixed row height, only visible rows built):
   - folder row: disclosure chevron (open / collapsed), label, `+a −r` caption; press →
     `Msg::TreeFolderToggled(paths)`;
   - file row: name (emphasised, ellipsised, highlighted spans), kind tag (Added, Deleted, Renamed,
     Mode; none for Modified), Binary / Not text tag, `ChangeBar` + `+a −r` (none when `bar` is
     `None`; Binary / Not text rows show no counts), unsent-comment tag when non-zero; tooltip
     `hover_text`; press → `Msg::FileSelected(path)`; selected row outlined as today.
4. **No match**: with a filter and no rows, the line `No changed file matches the filter` and a
   **Clear filter** text button (`Msg::TreeFilterCleared`); the diff pane is unchanged (FR-011).

Indentation is `indent_levels(depth, max_indent_levels(width, step, min_name))` steps; the name and
the trailing tags keep their place before indentation grows (edge cases "Very deep paths", "Long file
names").
