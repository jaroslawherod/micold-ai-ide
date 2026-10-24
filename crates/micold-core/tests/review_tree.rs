//! Feature 1273: the changed-file tree's layout and its per-view choices
//! (`specs/1273-review-file-tree/contracts/review-tree.md`, data-model §1–§3).

use std::collections::BTreeSet;

use micold_core::review::tree::{
    layout, FolderRow, LayoutOptions, Leaf, Openness, Row, RowKind, TreeChoices, TreeSort,
};
use micold_core::typeahead::Query;

fn leaves<'a>(paths: &[&'a str]) -> Vec<Leaf<'a>> {
    paths
        .iter()
        .map(|path| Leaf {
            path,
            added: 0,
            removed: 0,
        })
        .collect()
}

fn never(_: &str) -> bool {
    false
}

fn set(paths: &[&str]) -> BTreeSet<String> {
    paths.iter().map(|p| (*p).to_owned()).collect()
}

/// The changed-file tree's options: chained, default-open over `collapsed`, Path order.
fn changes_tree(paths: &[&str], collapsed: &BTreeSet<String>) -> Vec<Row> {
    layout(
        &leaves(paths),
        &LayoutOptions {
            chain: true,
            openness: Openness::DefaultOpen { collapsed },
            sort: TreeSort::Path,
            query: None,
            reviewed: &never,
        },
    )
}

/// The **Files** browser's options: no chaining, default-closed over `open`.
fn files_tree(paths: &[&str], open: &BTreeSet<String>) -> Vec<Row> {
    layout(
        &leaves(paths),
        &LayoutOptions {
            chain: false,
            openness: Openness::DefaultClosed { open },
            sort: TreeSort::Path,
            query: None,
            reviewed: &never,
        },
    )
}

/// `<depth><label>`, with a trailing `/` on a folder and `-` on a closed one.
fn shape(rows: &[Row]) -> Vec<String> {
    rows.iter()
        .map(|row| match &row.kind {
            RowKind::Folder(f) => {
                format!(
                    "{}{}/{}",
                    row.depth,
                    row.label,
                    if f.open { "" } else { "-" }
                )
            }
            RowKind::File { .. } => format!("{}{}", row.depth, row.label),
        })
        .collect()
}

fn folder(rows: &[Row], label: &str) -> FolderRow {
    rows.iter()
        .find_map(|row| match &row.kind {
            RowKind::Folder(f) if row.label == label => Some(f.clone()),
            _ => None,
        })
        .unwrap_or_else(|| panic!("no folder row {label}"))
}

/// Every file row's leaf index names the input path its folder rows and label spell out.
fn assert_leaves_index_input(rows: &[Row], paths: &[&str]) {
    let mut folders: Vec<String> = Vec::new();
    for row in rows {
        folders.truncate(usize::from(row.depth));
        match &row.kind {
            RowKind::Folder(f) => folders.push(f.paths.last().cloned().expect("a folder path")),
            RowKind::File { leaf } => {
                let expected = match folders.last() {
                    Some(parent) => format!("{parent}/{}", row.label),
                    None => row.label.clone(),
                };
                assert_eq!(
                    paths[*leaf], expected,
                    "row {} indexes the wrong leaf",
                    row.label
                );
            }
        }
    }
}

// --- T002: layout under Path, no filter -----------------------------------------------------

/// L1, scenario 1.1: folders before files at each level, each under its folder. (`NOTES.md`
/// stands in for the contract's `README.md`: a literal naming real documentation fails
/// `documentation_is_not_read`.)
#[test]
fn folders_come_before_files_and_hold_their_files() {
    let paths = ["src/a.rs", "src/ui/b.rs", "NOTES.md"];
    let rows = changes_tree(&paths, &BTreeSet::new());
    assert_eq!(
        shape(&rows),
        ["0src/", "1ui/", "2b.rs", "1a.rs", "0NOTES.md"]
    );
    assert!(rows.iter().all(|row| row.spans.is_empty()));
    assert_leaves_index_input(&rows, &paths);
    let paths = ["src/a.rs", "docs/a.rs", "a.rs"];
    let rows = changes_tree(&paths, &BTreeSet::new());
    assert_eq!(shape(&rows), ["0docs/", "1a.rs", "0src/", "1a.rs", "0a.rs"]);
    assert_leaves_index_input(&rows, &paths);
}

/// L2, scenario 1.2: a chain of single-child folders is one row naming every real folder.
#[test]
fn a_chain_of_single_child_folders_is_one_row() {
    let paths = ["crates/core/src/review/a.rs", "crates/core/src/review/b.rs"];
    let rows = changes_tree(&paths, &BTreeSet::new());
    assert_eq!(shape(&rows), ["0crates/core/src/review/", "1a.rs", "1b.rs"]);
    assert_eq!(
        folder(&rows, "crates/core/src/review").paths,
        [
            "crates",
            "crates/core",
            "crates/core/src",
            "crates/core/src/review"
        ]
    );
    assert_leaves_index_input(&rows, &paths);
}

/// A chain stops at a folder that holds a file or two folders.
#[test]
fn a_chain_stops_where_a_folder_branches_or_holds_a_file() {
    let rows = changes_tree(&["a/b/x.rs", "a/b/c/y.rs", "a/b/d/z.rs"], &BTreeSet::new());
    assert_eq!(
        shape(&rows),
        ["0a/b/", "1c/", "2y.rs", "1d/", "2z.rs", "1x.rs"]
    );
}

/// L3, scenario 1.3: a collapsed folder hides everything below it; expanding it restores the
/// sub-folders with their own state.
#[test]
fn collapsing_a_folder_hides_its_rows_and_keeps_inner_state() {
    let paths = ["src/a.rs", "src/ui/b.rs", "NOTES.md"];
    let rows = changes_tree(&paths, &set(&["src", "src/ui"]));
    assert_eq!(shape(&rows), ["0src/-", "0NOTES.md"]);
    let rows = changes_tree(&paths, &set(&["src/ui"]));
    assert_eq!(shape(&rows), ["0src/", "1ui/-", "1a.rs", "0NOTES.md"]);
    let rows = changes_tree(&paths, &BTreeSet::new());
    assert_eq!(
        shape(&rows),
        ["0src/", "1ui/", "2b.rs", "1a.rs", "0NOTES.md"]
    );
}

/// L4: a chained row is closed when any folder it stands for is collapsed.
#[test]
fn a_chained_row_is_closed_when_any_of_its_folders_is_collapsed() {
    let rows = changes_tree(&["a/b/c/x.rs"], &set(&["a/b"]));
    assert_eq!(shape(&rows), ["0a/b/c/-"]);
    assert!(!folder(&rows, "a/b/c").open);
}

/// One file, or files only at the top level: no folder rows.
#[test]
fn top_level_files_have_no_folder_rows() {
    assert_eq!(
        shape(&changes_tree(&["main.rs"], &BTreeSet::new())),
        ["0main.rs"]
    );
    assert_eq!(
        shape(&changes_tree(&["b.rs", "a.rs", "C.md"], &BTreeSet::new())),
        ["0C.md", "0a.rs", "0b.rs"]
    );
}

/// A rename sits under its new path's folder (the leaf is the new path).
#[test]
fn a_renamed_file_sits_under_its_new_folder() {
    let paths = ["new/place/file.rs", "old/other.rs"];
    let rows = changes_tree(&paths, &BTreeSet::new());
    assert_eq!(
        shape(&rows),
        ["0new/place/", "1file.rs", "0old/", "1other.rs"]
    );
    assert_leaves_index_input(&rows, &paths);
}

/// Spec 4.1: siblings by byte order of the name; a chained folder by its first folder's name, so
/// `a/b` lists before `a-x` as the **Files** browser orders them.
#[test]
fn a_chained_folder_orders_by_its_first_folders_name() {
    let rows = changes_tree(&["a-x/d.rs", "a/b/c.rs"], &BTreeSet::new());
    assert_eq!(shape(&rows), ["0a/b/", "1c.rs", "0a-x/", "1d.rs"]);
}

/// SC-002: 30 files in 5 folders; no file row repeats a folder prefix.
#[test]
fn no_file_row_carries_a_folder_prefix() {
    let folders = ["src", "src/ui", "docs", "tests/fixtures", "crates/core/src"];
    let owned: Vec<String> = (0..30)
        .map(|i| format!("{}/file{i}.rs", folders[i % folders.len()]))
        .collect();
    let paths: Vec<&str> = owned.iter().map(String::as_str).collect();
    let rows = changes_tree(&paths, &BTreeSet::new());
    let files: Vec<&Row> = rows
        .iter()
        .filter(|row| matches!(row.kind, RowKind::File { .. }))
        .collect();
    assert_eq!(files.len(), 30);
    assert!(files.iter().all(|row| !row.label.contains('/')));
    assert_leaves_index_input(&rows, &paths);
}

// --- T003: Files parity (L12) ---------------------------------------------------------------

/// `the_tree_shows_a_folders_files_while_it_is_open` in the client's `features_changes.rs`.
#[test]
fn files_tree_lists_a_folders_files_while_it_is_open() {
    let paths = ["NOTES.txt", "docs/a.md", "docs/b.md"];
    assert_eq!(
        shape(&files_tree(&paths, &BTreeSet::new())),
        ["0docs/-", "0NOTES.txt"]
    );
    let rows = files_tree(&paths, &set(&["docs"]));
    assert_eq!(shape(&rows), ["0docs/", "1a.md", "1b.md", "0NOTES.txt"]);
    assert_leaves_index_input(&rows, &paths);
}

/// `a_change_re_reads_the_browsed_file_and_the_tree`: docs open gives docs, its two files, a.rs.
#[test]
fn files_tree_with_an_open_folder_counts_its_rows() {
    let rows = files_tree(&["a.rs", "docs/x.md", "docs/y.md"], &set(&["docs"]));
    assert_eq!(shape(&rows), ["0docs/", "1x.md", "1y.md", "0a.rs"]);
}

/// The **Files** browser never chains, opens a nested folder only by its own path, and orders a
/// level's files by byte order whatever the input order.
#[test]
fn files_tree_does_not_chain_and_sorts_unsorted_input() {
    let paths = ["z/y/b.rs", "z/y/a.rs", "m.rs", "B.rs"];
    assert_eq!(
        shape(&files_tree(&paths, &set(&["z"]))),
        ["0z/", "1y/-", "0B.rs", "0m.rs"]
    );
    let rows = files_tree(&paths, &set(&["z", "z/y"]));
    assert_eq!(
        shape(&rows),
        ["0z/", "1y/", "2a.rs", "2b.rs", "0B.rs", "0m.rs"]
    );
    assert_leaves_index_input(&rows, &paths);
}

/// A folder opened while its parent is closed stays hidden; with chaining, a default-closed
/// chained row opens only when every folder it stands for is open.
#[test]
fn default_closed_hides_an_open_folder_under_a_closed_one_and_opens_chains_whole() {
    let paths = ["z/y/a.rs", "m.rs"];
    assert_eq!(
        shape(&files_tree(&paths, &set(&["z/y"]))),
        ["0z/-", "0m.rs"]
    );
    let chained = |open: &BTreeSet<String>| {
        shape(&layout(
            &leaves(&paths),
            &LayoutOptions {
                chain: true,
                openness: Openness::DefaultClosed { open },
                sort: TreeSort::Path,
                query: None,
                reviewed: &never,
            },
        ))
    };
    assert_eq!(chained(&set(&["z"])), ["0z/y/-", "0m.rs"]);
    assert_eq!(chained(&set(&["z", "z/y"])), ["0z/y/", "1a.rs", "0m.rs"]);
}

// --- T005: TreeChoices ----------------------------------------------------------------------

#[test]
fn choices_start_empty_in_path_order() {
    let choices = TreeChoices::default();
    assert_eq!(choices.filter, "");
    assert_eq!(choices.sort, TreeSort::Path);
    assert_eq!(TreeSort::default(), TreeSort::Path);
    assert!(choices.collapsed.is_empty());
    assert!(choices.filter_collapsed.is_empty());
}

/// Collapsing an open chained row records only its deepest folder; expanding a collapsed one
/// clears every folder it stands for.
#[test]
fn toggle_collapses_the_deepest_folder_and_expands_them_all() {
    let mut choices = TreeChoices::default();
    let paths = ["a/b/c/x.rs"];
    let rows = changes_tree(&paths, &choices.collapsed);
    choices.toggle(&folder(&rows, "a/b/c"));
    assert_eq!(choices.collapsed, set(&["a/b/c"]));

    choices.collapsed = set(&["a", "a/b", "other"]);
    let rows = changes_tree(&paths, &choices.collapsed);
    choices.toggle(&folder(&rows, "a/b/c"));
    assert_eq!(choices.collapsed, set(&["other"]));
    assert_eq!(
        shape(&changes_tree(&paths, &choices.collapsed)),
        ["0a/b/c/", "1x.rs"]
    );
}

/// FR-008: revealing a file opens every folder above it, and only those.
#[test]
fn reveal_opens_every_ancestor_of_the_path() {
    let mut choices = TreeChoices {
        collapsed: set(&["src", "src/ui", "src/uix", "docs", "src/ui/b.rs"]),
        ..TreeChoices::default()
    };
    choices.reveal("src/ui/b.rs");
    assert_eq!(choices.collapsed, set(&["src/uix", "docs", "src/ui/b.rs"]));
}

/// No method prunes a folder that left the list: when it returns it keeps its state.
#[test]
fn a_folder_that_leaves_and_returns_keeps_its_state() {
    let mut choices = TreeChoices::default();
    let rows = changes_tree(&["gone/x.rs", "a.rs"], &choices.collapsed);
    choices.toggle(&folder(&rows, "gone"));
    let rows = changes_tree(&["a.rs", "kept/y.rs"], &choices.collapsed);
    choices.toggle(&folder(&rows, "kept"));
    choices.reveal("a.rs");
    assert_eq!(choices.collapsed, set(&["gone", "kept"]));
    assert_eq!(
        shape(&changes_tree(&["gone/x.rs"], &choices.collapsed)),
        ["0gone/-"]
    );
}

#[test]
fn options_open_by_default_over_collapsed_with_chaining_and_the_sort() {
    let choices = TreeChoices {
        sort: TreeSort::Size,
        collapsed: set(&["src"]),
        ..TreeChoices::default()
    };
    let query = Query::new("x");
    let options = choices.options(&query, &never);
    assert!(options.chain);
    assert_eq!(options.sort, TreeSort::Size);
    assert_eq!(options.query, Some(&query));
    match options.openness {
        Openness::DefaultOpen { collapsed } => assert_eq!(collapsed, &set(&["src"])),
        Openness::DefaultClosed { .. } => panic!("the changed-file tree opens by default"),
    }
    assert!(!(options.reviewed)("src/a.rs"));
}

// --- T007: folder totals (FR-007) -----------------------------------------------------------

/// Leaves with `(path, added, removed)` counts.
fn counted<'a>(files: &[(&'a str, u32, u32)]) -> Vec<Leaf<'a>> {
    files
        .iter()
        .map(|&(path, added, removed)| Leaf {
            path,
            added,
            removed,
        })
        .collect()
}

/// Lay `leaves` out as the changed-file tree with `sort`, `query` and `reviewed`.
fn sorted_tree(
    leaves: &[Leaf],
    collapsed: &BTreeSet<String>,
    sort: TreeSort,
    query: Option<&Query>,
    reviewed: &dyn Fn(&str) -> bool,
) -> Vec<Row> {
    layout(
        leaves,
        &LayoutOptions {
            chain: true,
            openness: Openness::DefaultOpen { collapsed },
            sort,
            query,
            reviewed,
        },
    )
}

/// L5, scenario 2.5: a folder row sums the counts of the files below it.
#[test]
fn a_folder_row_sums_its_files_counts() {
    let leaves = counted(&[("src/x.rs", 40, 10), ("src/y.rs", 1, 1), ("top.rs", 7, 7)]);
    let rows = sorted_tree(&leaves, &BTreeSet::new(), TreeSort::Path, None, &never);
    let src = folder(&rows, "src");
    assert_eq!((src.added, src.removed), (41, 11));
}

/// A chained row's totals cover every file below it, nested folders included; each nested folder
/// sums only its own subtree.
#[test]
fn a_chained_row_sums_every_file_below_it() {
    let leaves = counted(&[
        ("a/b/c/x.rs", 3, 1),
        ("a/b/c/d/y.rs", 5, 2),
        ("a/b/e/z.rs", 10, 0),
    ]);
    let rows = sorted_tree(&leaves, &BTreeSet::new(), TreeSort::Path, None, &never);
    let ab = folder(&rows, "a/b");
    assert_eq!((ab.added, ab.removed), (18, 3));
    let c = folder(&rows, "c");
    assert_eq!((c.added, c.removed), (8, 3));
    let d = folder(&rows, "d");
    assert_eq!((d.added, d.removed), (5, 2));
}

/// Scenario 2.5: a collapsed folder still says how much changed inside it.
#[test]
fn a_collapsed_folder_keeps_its_totals() {
    let leaves = counted(&[("src/x.rs", 40, 10), ("src/ui/y.rs", 1, 1)]);
    let rows = sorted_tree(&leaves, &set(&["src"]), TreeSort::Path, None, &never);
    assert_eq!(shape(&rows), ["0src/-"]);
    let src = folder(&rows, "src");
    assert_eq!((src.added, src.removed), (41, 11));
}

/// Totals are `u64`: thousands of large files do not overflow a `u32` sum.
#[test]
fn folder_totals_do_not_overflow_u32() {
    let leaves = counted(&[("big/a.bin", u32::MAX, 0), ("big/b.bin", u32::MAX, 1)]);
    let rows = sorted_tree(&leaves, &BTreeSet::new(), TreeSort::Path, None, &never);
    let big = folder(&rows, "big");
    assert_eq!(big.added, 2 * u64::from(u32::MAX));
    assert_eq!(big.removed, 1);
}

// --- T011: the filter (research R4) ---------------------------------------------------------

/// Lay `paths` out as the changed-file tree, filtered by `text`, in Path order.
fn filtered(paths: &[&str], text: &str) -> Vec<Row> {
    let query = Query::new(text);
    sorted_tree(
        &leaves(paths),
        &BTreeSet::new(),
        TreeSort::Path,
        Some(&query),
        &never,
    )
}

fn spans_of(rows: &[Row], label: &str) -> Vec<std::ops::Range<usize>> {
    rows.iter()
        .find(|row| row.label == label)
        .unwrap_or_else(|| panic!("no row {label}"))
        .spans
        .clone()
}

/// L8, scenarios 3.1 and 3.5: only matching files are listed, under their folders, with the
/// matched characters of each name highlighted.
#[test]
fn the_filter_lists_matching_files_under_their_folders() {
    let paths = ["src/daemon/tests.rs", "src/lib.rs", "docs/tests.md"];
    let rows = filtered(&paths, "tests");
    assert_eq!(
        shape(&rows),
        ["0docs/", "1tests.md", "0src/daemon/", "1tests.rs"]
    );
    assert_eq!(spans_of(&rows, "tests.rs"), vec![0..5]);
    assert_eq!(spans_of(&rows, "tests.md"), vec![0..5]);
    assert!(spans_of(&rows, "docs").is_empty());
    assert!(spans_of(&rows, "src/daemon").is_empty());
    assert_leaves_index_input(&rows, &paths);
}

/// A match across a folder and a file name lights both labels, each rebased to its own label.
#[test]
fn spans_are_rebased_to_each_rows_label() {
    let rows = filtered(&["src/daemon/tests.rs", "src/lib.rs"], "daemon/t");
    assert_eq!(shape(&rows), ["0src/daemon/", "1tests.rs"]);
    assert_eq!(spans_of(&rows, "src/daemon"), vec![4..10]);
    assert_eq!(spans_of(&rows, "tests.rs"), vec![0..1]);
    let rows = filtered(&["src/a.rs", "src/b.rs", "lib/src.rs"], "src");
    assert_eq!(
        shape(&rows),
        ["0lib/", "1src.rs", "0src/", "1a.rs", "1b.rs"]
    );
    assert_eq!(spans_of(&rows, "src"), vec![0..3]);
    assert_eq!(spans_of(&rows, "src.rs"), vec![0..3]);
    assert!(spans_of(&rows, "lib").is_empty());
    assert!(spans_of(&rows, "a.rs").is_empty());
}

/// A folder row's spans are the union of its listed files' spans inside its label.
#[test]
fn a_folder_rows_spans_are_the_union_of_its_files_spans() {
    use micold_core::typeahead::match_one;
    // `abcde.rs` matches in its own name, `q.rs` within one edit of the folder's name.
    let paths = ["abcdx/abcde.rs", "abcdx/q.rs", "other.rs"];
    let text = "abcde";
    let query = Query::new(text);
    let rows = filtered(&paths, text);
    let label = "abcdx";
    let mut expected: Vec<usize> = Vec::new();
    let mut per_file: Vec<Vec<std::ops::Range<usize>>> = Vec::new();
    for path in paths.iter().filter(|p| p.starts_with("abcdx/")) {
        if let Some(found) = match_one(path, &query) {
            let inside: Vec<_> = found
                .spans
                .iter()
                .map(|s| s.start.min(label.len())..s.end.min(label.len()))
                .filter(|s| !s.is_empty())
                .collect();
            expected.extend(inside.iter().flat_map(Clone::clone));
            per_file.push(inside);
        }
    }
    expected.sort_unstable();
    expected.dedup();
    let got: Vec<usize> = spans_of(&rows, label).into_iter().flatten().collect();
    assert_eq!(got, expected);
    assert!(
        per_file.windows(2).any(|w| w[0] != w[1]),
        "the fixture should give files different spans inside the folder: {per_file:?}"
    );
}

/// L9, scenario 3.2: a collapsed folder holding a match shows open while the filter holds text.
#[test]
fn a_collapsed_folder_shows_open_while_the_filter_holds_text() {
    let paths = ["src/daemon/tests.rs", "src/lib.rs", "docs/tests.md"];
    let choices = TreeChoices {
        filter: "tests".to_owned(),
        collapsed: set(&["src", "docs"]),
        ..TreeChoices::default()
    };
    let query = Query::new(&choices.filter);
    let rows = layout(&leaves(&paths), &choices.options(&query, &never));
    assert_eq!(
        shape(&rows),
        ["0docs/", "1tests.md", "0src/daemon/", "1tests.rs"]
    );
}

/// L10, scenario 3.3: no match gives no rows (the client shows the no-match line).
#[test]
fn a_filter_matching_nothing_gives_no_rows() {
    assert!(filtered(&["src/a.rs", "b.rs"], "zzzz").is_empty());
}

/// Whitespace-only text is no filter.
#[test]
fn whitespace_only_text_is_no_filter() {
    let paths = ["src/a.rs", "b.rs"];
    let rows = filtered(&paths, "   ");
    assert_eq!(shape(&rows), ["0src/", "1a.rs", "0b.rs"]);
    assert!(rows.iter().all(|row| row.spans.is_empty()));
}

/// FR-009: the filter decides which files are listed, never their order.
#[test]
fn the_filter_never_reorders() {
    let rows = filtered(
        &["b/aaa.rs", "c.rs", "a/xa.rs", "q/zzza.rs", "q/ab.rs"],
        "a",
    );
    assert_eq!(
        shape(&rows),
        ["0a/", "1xa.rs", "0b/", "1aaa.rs", "0q/", "1ab.rs", "1zzza.rs"]
    );
}

/// FR-007 with a filter: a folder sums only the files listed below it.
#[test]
fn filtered_folder_totals_cover_listed_files_only() {
    let leaves = counted(&[("src/tests.rs", 4, 1), ("src/lib.rs", 100, 100)]);
    let query = Query::new("tests");
    let rows = sorted_tree(
        &leaves,
        &BTreeSet::new(),
        TreeSort::Path,
        Some(&query),
        &never,
    );
    let src = folder(&rows, "src");
    assert_eq!((src.added, src.removed), (4, 1));
}

/// SC-001: 30 files in 5 folders, `tests.r` lists exactly `daemon/tests.rs`. No other path holds
/// `t e s t s . r` in order or within one edit of it.
#[test]
fn typing_tests_dot_r_lists_exactly_the_one_file() {
    let folders = ["src", "ui", "docs", "daemon", "web"];
    let mut owned: Vec<String> = (0..29)
        .map(|i| format!("{}/file{i}.rs", folders[i % folders.len()]))
        .collect();
    owned.push("daemon/tests.rs".to_owned());
    let paths: Vec<&str> = owned.iter().map(String::as_str).collect();
    let rows = filtered(&paths, "tests.r");
    assert_eq!(shape(&rows), ["0daemon/", "1tests.rs"]);
    assert_leaves_index_input(&rows, &paths);
}

// --- T013: TreeChoices while filtering (research R2) ----------------------------------------

#[test]
fn starting_a_filter_clears_filter_folds_and_keeps_the_text_as_typed() {
    let mut choices = TreeChoices {
        filter_collapsed: set(&["old"]),
        ..TreeChoices::default()
    };
    choices.set_filter("  Te ");
    assert_eq!(choices.filter, "  Te ");
    assert!(choices.filter_collapsed.is_empty());
    choices.filter_collapsed = set(&["src"]);
    choices.set_filter("  Tes ");
    assert_eq!(choices.filter_collapsed, set(&["src"]));
}

/// While the filter holds text a fold writes `filter_collapsed`, never `collapsed`, and the
/// layout reads `filter_collapsed`.
#[test]
fn folds_made_while_filtering_use_the_filter_set() {
    let paths = ["src/daemon/tests.rs", "docs/tests.md"];
    let mut choices = TreeChoices {
        collapsed: set(&["docs"]),
        ..TreeChoices::default()
    };
    choices.set_filter("tests");
    let query = Query::new(&choices.filter);
    let rows = layout(&leaves(&paths), &choices.options(&query, &never));
    choices.toggle(&folder(&rows, "src/daemon"));
    assert_eq!(choices.collapsed, set(&["docs"]));
    assert_eq!(choices.filter_collapsed, set(&["src/daemon"]));
    let rows = layout(&leaves(&paths), &choices.options(&query, &never));
    assert_eq!(shape(&rows), ["0docs/", "1tests.md", "0src/daemon/-"]);
    choices.toggle(&folder(&rows, "src/daemon"));
    assert!(choices.filter_collapsed.is_empty());
    assert_eq!(choices.collapsed, set(&["docs"]));
}

/// FR-010, scenario 3.2: clearing the filter restores the folds from before it.
#[test]
fn clearing_the_filter_restores_the_folds_from_before_it() {
    let mut choices = TreeChoices {
        collapsed: set(&["src"]),
        ..TreeChoices::default()
    };
    choices.set_filter("tests");
    choices.filter_collapsed = set(&["docs"]);
    choices.clear_filter();
    assert_eq!(choices.filter, "");
    assert!(choices.filter_collapsed.is_empty());
    assert_eq!(choices.collapsed, set(&["src"]));
    choices.set_filter("x");
    choices.set_filter("");
    assert_eq!(choices.collapsed, set(&["src"]));
}

/// Whitespace-only text is no filter: a fold writes `collapsed`.
#[test]
fn a_whitespace_filter_folds_into_collapsed() {
    let mut choices = TreeChoices::default();
    choices.set_filter("   ");
    let query = Query::new(&choices.filter);
    let rows = layout(&leaves(&["src/a.rs"]), &choices.options(&query, &never));
    choices.toggle(&folder(&rows, "src"));
    assert_eq!(choices.collapsed, set(&["src"]));
    assert!(choices.filter_collapsed.is_empty());
}

/// FR-008: revealing a file opens its folders in both sets.
#[test]
fn reveal_clears_both_sets() {
    let mut choices = TreeChoices {
        filter: "b".to_owned(),
        collapsed: set(&["src", "docs"]),
        filter_collapsed: set(&["src/ui", "web"]),
        ..TreeChoices::default()
    };
    choices.reveal("src/ui/b.rs");
    assert_eq!(choices.collapsed, set(&["docs"]));
    assert_eq!(choices.filter_collapsed, set(&["web"]));
}

// --- T015: the sorts (research R3) ----------------------------------------------------------

fn size_tree(leaves: &[Leaf]) -> Vec<Row> {
    sorted_tree(leaves, &BTreeSet::new(), TreeSort::Size, None, &never)
}

/// L6, SC-004: folders by their sums, files by their totals, largest first; ties in Path order.
#[test]
fn size_orders_each_level_largest_first() {
    let leaves = counted(&[
        ("a/x.rs", 2, 1),
        ("b/y.rs", 30, 20),
        ("b/s.rs", 0, 1),
        ("b/t.rs", 0, 7),
        ("f5.rs", 5, 0),
        ("f100.rs", 60, 40),
        ("e.rs", 10, 0),
        ("d.rs", 0, 10),
    ]);
    let rows = size_tree(&leaves);
    assert_eq!(
        shape(&rows),
        [
            "0b/", "1y.rs", "1t.rs", "1s.rs", "0a/", "1x.rs", "0f100.rs", "0d.rs", "0e.rs",
            "0f5.rs"
        ]
    );
}

/// SC-004 over a generated change: at every level the largest file is the first file row, files
/// run largest first, and the top level's first folder holds the most changed lines.
#[test]
fn size_puts_the_largest_file_and_folder_first() {
    let owned: Vec<(String, u32, u32)> = (0..60u32)
        .map(|i| {
            let n = i.wrapping_mul(2_654_435_761) >> 7;
            let path = match i % 4 {
                0 => format!("f{i}.rs"),
                1 => format!("d{}/f{i}.rs", i % 5),
                _ => format!("d{}/s{}/f{i}.rs", i % 5, i % 3),
            };
            (path, n % 97, n % 31)
        })
        .collect();
    let leaves: Vec<Leaf> = owned
        .iter()
        .map(|(path, added, removed)| Leaf {
            path,
            added: *added,
            removed: *removed,
        })
        .collect();
    let rows = size_tree(&leaves);
    let total = |row: &Row| match &row.kind {
        RowKind::Folder(f) => f.added + f.removed,
        RowKind::File { leaf } => u64::from(leaves[*leaf].added) + u64::from(leaves[*leaf].removed),
    };
    let mut run: Vec<u64> = Vec::new();
    let mut runs = 0;
    for (i, row) in rows.iter().enumerate() {
        if matches!(row.kind, RowKind::File { .. }) {
            run.push(total(row));
        }
        let ends = rows.get(i + 1).is_none_or(|next| {
            next.depth != row.depth || !matches!(next.kind, RowKind::File { .. })
        });
        if ends && !run.is_empty() {
            assert!(
                run.windows(2).all(|w| w[0] >= w[1]),
                "files not largest first: {run:?}"
            );
            runs += 1;
            run.clear();
        }
    }
    assert!(runs > 3);
    let top: Vec<u64> = rows
        .iter()
        .filter(|row| row.depth == 0 && matches!(row.kind, RowKind::Folder(_)))
        .map(total)
        .collect();
    assert_eq!(top.first(), top.iter().max());
    assert!(top.windows(2).all(|w| w[0] >= w[1]), "{top:?}");
}

/// L7, scenario 4.3: unreviewed files first, then folders holding any unreviewed file before
/// folders holding none; ties in Path order.
#[test]
fn unreviewed_first_orders_by_the_reviewed_set() {
    let reviewed = |path: &str| ["src/a.rs", "done/x.rs", "mixed/p.rs"].contains(&path);
    let paths = [
        "src/a.rs",
        "src/b.rs",
        "src/c.rs",
        "done/x.rs",
        "mixed/p.rs",
        "mixed/q.rs",
        "zz/y.rs",
    ];
    let rows = sorted_tree(
        &leaves(&paths),
        &BTreeSet::new(),
        TreeSort::UnreviewedFirst,
        None,
        &reviewed,
    );
    assert_eq!(
        shape(&rows),
        [
            "0mixed/", "1q.rs", "1p.rs", "0src/", "1b.rs", "1c.rs", "1a.rs", "0zz/", "1y.rs",
            "0done/", "1x.rs"
        ]
    );
}

/// Folders come before files under every sort, and every sort keeps the tree.
#[test]
fn every_sort_keeps_folders_first_and_the_tree() {
    let leaves = counted(&[
        ("huge.rs", 1000, 0),
        ("dir/small.rs", 1, 0),
        ("dir/sub/t.rs", 2, 2),
    ]);
    let reviewed = |path: &str| path.starts_with("dir/");
    let mut path_rows: Vec<(u16, String)> =
        sorted_tree(&leaves, &BTreeSet::new(), TreeSort::Path, None, &reviewed)
            .into_iter()
            .map(|row| (row.depth, row.label))
            .collect();
    path_rows.sort();
    for sort in [TreeSort::Path, TreeSort::Size, TreeSort::UnreviewedFirst] {
        let rows = sorted_tree(&leaves, &BTreeSet::new(), sort, None, &reviewed);
        assert_eq!(rows[0].label, "dir", "{sort:?}");
        assert_eq!(
            rows.last().map(|r| r.label.as_str()),
            Some("huge.rs"),
            "{sort:?}"
        );
        assert_eq!(rows[1].label, "sub", "{sort:?}");
        let mut got: Vec<(u16, String)> = rows.into_iter().map(|r| (r.depth, r.label)).collect();
        got.sort();
        assert_eq!(got, path_rows, "{sort:?}");
    }
}

/// L11, scenario 4.4: a filtered tree is ordered by the sort over the listed files.
#[test]
fn size_with_a_filter_orders_by_the_listed_files() {
    let leaves = counted(&[
        ("src/a_test.rs", 1, 0),
        ("src/b_test.rs", 50, 0),
        ("src/big.rs", 1000, 0),
        ("lib/c_test.rs", 100, 0),
    ]);
    let query = Query::new("test");
    let rows = sorted_tree(
        &leaves,
        &BTreeSet::new(),
        TreeSort::Size,
        Some(&query),
        &never,
    );
    assert_eq!(
        shape(&rows),
        ["0lib/", "1c_test.rs", "0src/", "1b_test.rs", "1a_test.rs"]
    );
    assert_eq!(folder(&rows, "src").added, 51);
}

// --- T018: SC-003 timing (research R9) ------------------------------------------------------

/// 5,000 files in 200 folders: building the leaves and laying them out after a collapse, a filter
/// keystroke and a sort change each stay under 100 ms in release (500 ms in a debug build, so a
/// debug run stays green without hiding a regression). Slowest of repeated runs.
#[test]
fn laying_out_five_thousand_files_stays_under_100_ms() {
    use std::time::{Duration, Instant};
    let owned: Vec<(String, u32, u32)> = (0..5_000u32)
        .map(|i| {
            let folder = i % 200;
            (
                format!(
                    "crates/area{}/module{}/file{i}.rs",
                    folder / 20,
                    folder % 20
                ),
                i % 300,
                i % 70,
            )
        })
        .collect();
    let limit = if cfg!(debug_assertions) {
        Duration::from_millis(500)
    } else {
        Duration::from_millis(100)
    };
    let mut choices = TreeChoices::default();
    let steps: [&dyn Fn(&mut TreeChoices); 3] = [
        &|c| {
            c.collapsed.insert("crates/area3/module7".to_owned());
        },
        &|c| c.set_filter("file4"),
        &|c| c.sort = TreeSort::Size,
    ];
    let mut slowest = Duration::ZERO;
    for _ in 0..5 {
        for step in steps {
            step(&mut choices);
            let started = Instant::now();
            let leaves: Vec<Leaf> = owned
                .iter()
                .map(|(path, added, removed)| Leaf {
                    path,
                    added: *added,
                    removed: *removed,
                })
                .collect();
            let query = Query::new(&choices.filter);
            let rows = layout(&leaves, &choices.options(&query, &never));
            slowest = slowest.max(started.elapsed());
            assert!(!rows.is_empty());
            std::hint::black_box(rows);
        }
        choices = TreeChoices::default();
    }
    assert!(slowest < limit, "slowest layout {slowest:?} (SC-003)");
}
