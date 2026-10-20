# Contract: Changes view changes (#1150)

Extends `specs/1003-review-worktree-prs/contracts/changes-view.md` and 775/482's.

## 1. Messages and effects (`features::changes`)
- `Msg::GapToggled(GapKey)`, `Msg::WholeFileToggled`: update the selected file's `Expansion`. For a PR file, selecting it returns `Effect::ReadFile` for its head version (eager, R2); markers appear when `FileRead` answers; toggles before then are ignored.
- `Msg::FileRead { seq, source, path, result }`: the answer to `Effect::ReadFile { seq, project, entry, source: SourceKey, commit, address, path, force_large }`; dropped when `seq` is stale or the source changed.
- `Msg::TreeRead { seq, source, result }` answering `Effect::ReadTree { seq, source }` (both effects: `Effect::{ReadFile, ReadTree}`).
- `Msg::BrowseOpen(RelPath)`, `Msg::ReferenceActivated(RelPath, Option<u32>)`, `Msg::BrowseBack`, `Msg::OpenFileDiff`, `Msg::ShowLargeFile` (extends `ShowLarge`), `Msg::PathPickerChanged(String)`.
- Existing `Msg::DiffRead`/`PullRequestDiffRead`/`ListRead` reconcile expansion (`Expansion::reconcile`) and re-shape; state of ranges that vanished is dropped.
- Effects are performed by `shell/changes.rs` on blocking threads with `app.caps.shared_git().is_some()` as the locality gate only (no `Git` trait method is added: `browse` shells out through `GitCli` and its `no_window` wrapper and uses `std::fs` itself), and ( `issue_tooling().pull_requests` for the PR fallback); results come back as Msgs.

## 2. Core API
- `review::expand::{gaps, shape, Gap, Expansion, Shaped}`; `review::diff::{unified_rows, side_by_side_rows, UnifiedIndex, SideIndex}` accept `&Shaped` (a gap row variant `UnifiedRow::Gap(&Gap)`, `SideRow::Gap(&Gap)`).
- `review::browse::{FileSource, Confined, list_tree, read_file, FileReadFailure}`; `review::reference::find`.
- `diff::plain_file(&SideLines) -> FileDiff`.

## 3. `PullRequestSource` (additions)
```
fn file_at(&self, repo: &GithubRepo, commit: &str, path: &RelPath, now: u64) -> Result<Vec<u8>, FileReadFailure>;
fn tree_at(&self, repo: &GithubRepo, commit: &str, now: u64) -> Result<Vec<RelPath>, FileReadFailure>;
```
One request each, no retry, same `gh` and credentials as `diff`; `diff_args`-style builders are pure and unit-tested; the fake source gets `with_file_answer` / `with_tree_answer`.

## 4. UI
- `DiffView::new(..).gaps(&[Gap]).on_gap(|key| M).references(f).on_reference(|path, line| M)`: a gap is one row "N hidden lines" with an expand control and a collapse control on a revealed range; references underline and are click targets. Both layouts.
- `BrowseBar::new(path, on_back).open_diff(Option<M>).roles(r)`: Back, path, "Open diff" (only when `diff_available`).
- Per-file header: whole-file toggle (hidden for added files, disabled with a tooltip when no text version).
- File tree (`TreeView`) and path picker (`Typeahead`) in the list pane header; both emit `BrowseOpen`.
- Large note with "Show anyway" reuses the 482 large-file row text.

## 5. Picking
`pickable(view, file) -> bool` in `features/changes.rs` is the only gate (R7). Untouched browsed files: `false`.
