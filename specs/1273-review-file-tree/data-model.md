# Data model: changed-file tree, filter and sort (#1273)

All types in §1–§5 are new, in `crates/micold-core/src/review/tree.rs` (render-free, iced-free).
§6 is the client's view state in `crates/micold-client/src/features/changes.rs`. Names are the
plan's; the tasks unit may refine them without changing the rules.

## 1. Leaf — what the layout is given per file

| Field | Type | Notes |
|---|---|---|
| `path` | `&str` | `/`-separated (`RelPath::as_str`) |
| `added`, `removed` | `u32` | from `ChangedFile`; 0 and 0 for **Files** and for binary / not text / Mode / pure rename |

The layout takes `&[Leaf]` and refers back to files by index, so it never owns or clones a
`ChangedFile`. The reviewed state is not a field: it is the predicate in `LayoutOptions`.

## 2. Choices and openness

```text
enum TreeSort { Path, Size, UnreviewedFirst }        // Default: Path (scenario 4.1)

enum Openness<'a> {
    DefaultOpen   { collapsed: &'a BTreeSet<String> },   // changed-file tree (FR-004)
    DefaultClosed { open: &'a BTreeSet<String> },        // Files browser (today's FilesPane::open_folders)
}

struct LayoutOptions<'a> {
    chain: bool,                          // FR-002; false for Files
    openness: Openness<'a>,
    sort: TreeSort,
    query: Option<&'a Query>,             // None or empty: no filter
    reviewed: &'a dyn Fn(&str) -> bool,   // used only by UnreviewedFirst
}

struct TreeChoices {                      // per open view, not shared (FR-013)
    filter: String,
    sort: TreeSort,
    collapsed: BTreeSet<String>,          // real folder paths, kept across re-reads and sources
    filter_collapsed: BTreeSet<String>,   // folds made while the filter holds text (R2)
}
```

`TreeChoices` transitions (all pure, unit-tested):

| Method | Effect |
|---|---|
| `toggle(&FolderRow)` | row open → insert its deepest path into the active set; row collapsed → remove all its `paths` from the active set. Active set = `filter_collapsed` while filtering, else `collapsed` |
| `set_filter(text)` | empty → non-empty clears `filter_collapsed`; stores `text` |
| `clear_filter()` | `filter` and `filter_collapsed` emptied; `collapsed` untouched (FR-010) |
| `set_sort(sort, offers)` | stores `sort` only if `offers` lists it as `Offer::Shown` |
| `reveal(path)` | removes every ancestor folder of `path` from both sets (FR-008) |
| `options<'a>(&'a self, query: &'a Query, reviewed: &'a dyn Fn(&str) -> bool) -> LayoutOptions<'a>` | `DefaultOpen` over the active set, `chain: true`, `sort`; the caller builds `query` once from `filter` (`Query::new`) and passes the reviewed predicate (always-false until #1271) |

Validation: `filter` is kept as typed; `Query::new` trims it. No method prunes `collapsed`: a folder
that leaves the list and returns keeps its state (spec edge cases).

## 3. Row

```text
struct Row {
    depth: u16,                 // 0 at the top; draw with indent_levels(depth, max)
    label: String,              // file name, or the joined chain `a/b/c`
    spans: Vec<Range<usize>>,   // byte ranges of `label` to highlight; empty without a filter
    kind: RowKind,
}
enum RowKind {
    Folder(FolderRow),
    File { leaf: usize },       // index into the Leaf slice
}
struct FolderRow {
    paths: Vec<String>,         // every real folder the row stands for, outermost first (len 1 unless chained)
    open: bool,                 // false when any of `paths` is in the active collapsed set
    added: u64, removed: u64,   // sums over the files listed below it (FR-007)
}
```

Rules (contracts §1):
- A folder row exists for each folder holding a listed file; with a filter, only folders on a
  matching file's path, all shown open unless folded in `filter_collapsed`.
- Chaining (`chain: true`): a folder with exactly one sub-folder and no listed file merges with it;
  the label joins the names with `/`; `paths` lists each.
- Within a level: folders, then files; each group ordered by `TreeSort` (research R3), ties in Path
  order: byte order of the row's name — a file's name, a chained folder's first folder name — so
  `a/b` sorts before `a-x` as the **Files** browser's per-component `BTreeMap` does (spec 4.1).
- Totals and Size order use the listed (filtered) files.

## 4. Sort offer

```text
enum Offer { Shown, Hidden, Disabled(&'static str) }
const UNREVIEWED_UNTIL_MARKS: Offer = Offer::Hidden;          // the one switch (research R3)
fn sort_options(marks: bool) -> [(TreeSort, Offer); 3]
// marks=false: Path Shown, Size Shown, UnreviewedFirst UNREVIEWED_UNTIL_MARKS
// marks=true:  all Shown
```

## 5. Bar, indent, hover

```text
struct BarParts { added: f32, removed: f32 }      // lengths in px, added + removed <= max_width
fn bar_scale(leaves: &[Leaf]) -> u64               // max added+removed over the whole list (R8)
fn bar(added: u32, removed: u32, scale: u64, max_width: f32) -> Option<BarParts>
const MIN_BAR: f32 = 2.0;
fn max_indent_levels(list_width: f32, step: f32, min_name_width: f32) -> u16
fn indent_levels(depth: u16, max: u16) -> u16
fn hover_text(path: &str, renamed_from: Option<&str>) -> String   // "path" or "path\nRenamed from old"
```

`bar` is `None` when `added + removed == 0` or `scale == 0`.

## 6. Client view state (`OpenView`)

| Field | Type | Notes |
|---|---|---|
| `tree` | `TreeChoices` | new; default at open; untouched by `list_read`, `SourceChosen`, toggles |
| `change_rows` | `Vec<Row>` | cache recomputed by `update` after any message that changes the list, the source, the choices, or the reviewed marks; never in `view` (R9) |

New messages: `TreeFolderToggled(Vec<String>)` (the row's `paths`), `TreeFilterChanged(String)`,
`TreeFilterCleared` (the clear action and the field's `on_escape`, research R5), `TreeSortChosen(TreeSort)`. `FileSelected(path)` also
calls `tree.reveal(&path)`, so every route that selects a changed file through it (`OpenFileDiff`,
and `ReferenceActivated` when it lands on a changed file) reveals it.
`tree_pane(view) -> TreePane::{Hidden, Rows, NoMatch}` (render-free) decides what the list pane
draws: `Hidden` unless `ListBody::Files` (no filter box or chips, FR-016), `NoMatch` with a filter
and no rows (FR-011), else `Rows`. `selected`, `list_offset` and the diff are unchanged by any of them: a
filtered-out selection keeps its diff (scenario 3.4).
