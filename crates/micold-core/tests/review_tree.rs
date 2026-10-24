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

/// L1, scenario 1.1: folders before files at each level, each under its folder.
#[test]
fn folders_come_before_files_and_hold_their_files() {
    let paths = ["src/a.rs", "src/ui/b.rs", "README.md"];
    let rows = changes_tree(&paths, &BTreeSet::new());
    assert_eq!(
        shape(&rows),
        ["0src/", "1ui/", "2b.rs", "1a.rs", "0README.md"]
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
    let paths = ["src/a.rs", "src/ui/b.rs", "README.md"];
    let rows = changes_tree(&paths, &set(&["src", "src/ui"]));
    assert_eq!(shape(&rows), ["0src/-", "0README.md"]);
    let rows = changes_tree(&paths, &set(&["src/ui"]));
    assert_eq!(shape(&rows), ["0src/", "1ui/-", "1a.rs", "0README.md"]);
    let rows = changes_tree(&paths, &BTreeSet::new());
    assert_eq!(
        shape(&rows),
        ["0src/", "1ui/", "2b.rs", "1a.rs", "0README.md"]
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
