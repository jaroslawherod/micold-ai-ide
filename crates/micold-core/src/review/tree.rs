//! The changed-file tree (feature 1273): laying out a list of paths as folder and file rows, and
//! the per-view choices that steer it — which folders are collapsed, the filter, the sort.
//!
//! Render-free and iced-free (Principle I): the client draws the [`Row`]s this module returns and
//! decides nothing about grouping, chaining or order itself. The **Files** browser lays out its
//! tree here too, without chaining and with folders closed by default (FR-017).
//!
//! Contract: `specs/1273-review-file-tree/contracts/review-tree.md`; types:
//! `specs/1273-review-file-tree/data-model.md` §1–§3.

use std::collections::{BTreeMap, BTreeSet};
use std::ops::Range;

use crate::typeahead::Query;

/// One file the layout is given (data-model §1). The layout refers back to it by index.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Leaf<'a> {
    /// The `/`-separated path (`RelPath::as_str`).
    pub path: &'a str,
    /// Lines added; 0 where the list has no counts.
    pub added: u32,
    /// Lines removed; 0 where the list has no counts.
    pub removed: u32,
}

/// The order of the rows within each level (research R3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TreeSort {
    /// Byte order of the row's name (scenario 4.1).
    #[default]
    Path,
    /// Largest change first.
    Size,
    /// Files not yet marked reviewed first.
    UnreviewedFirst,
}

/// Which folders are open.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Openness<'a> {
    /// Every folder is open except those in `collapsed` (the changed-file tree, FR-004).
    DefaultOpen {
        /// Real folder paths the reviewer collapsed.
        collapsed: &'a BTreeSet<String>,
    },
    /// Every folder is closed except those in `open` (the **Files** browser).
    DefaultClosed {
        /// Real folder paths the user opened.
        open: &'a BTreeSet<String>,
    },
}

/// How [`layout`] lays the paths out.
pub struct LayoutOptions<'a> {
    /// Merge a folder holding exactly one folder and no file into one row (FR-002).
    pub chain: bool,
    /// Which folders are open.
    pub openness: Openness<'a>,
    /// The order within each level.
    pub sort: TreeSort,
    /// The filter; `None` or empty lists every file.
    pub query: Option<&'a Query>,
    /// Whether a path is marked reviewed; read only by [`TreeSort::UnreviewedFirst`].
    pub reviewed: &'a dyn Fn(&str) -> bool,
}

/// One row of the tree (data-model §3).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Row {
    /// Nesting depth, 0 at the top.
    pub depth: u16,
    /// The file's name, or the folder's name (a chain's names joined with `/`).
    pub label: String,
    /// Byte ranges of `label` to highlight; empty without a filter.
    pub spans: Vec<Range<usize>>,
    /// What the row stands for.
    pub kind: RowKind,
}

/// What a [`Row`] stands for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RowKind {
    /// A folder, or a chain of single-child folders.
    Folder(FolderRow),
    /// A file.
    File {
        /// Its index in the [`Leaf`] slice given to [`layout`].
        leaf: usize,
    },
}

/// A folder row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FolderRow {
    /// Every real folder the row stands for, outermost first; one unless chained.
    pub paths: Vec<String>,
    /// Whether its contents are listed below it.
    pub open: bool,
    /// Lines added over the files listed below it (FR-007).
    pub added: u64,
    /// Lines removed over the files listed below it (FR-007).
    pub removed: u64,
}

/// A folder's contents while the tree is laid out: sub-folders by name, files as (name, leaf).
#[derive(Default)]
struct Node<'a> {
    folders: BTreeMap<&'a str, Node<'a>>,
    files: Vec<(&'a str, usize)>,
}

/// The visible rows of `leaves` laid out as a tree: folders before files at each level, each
/// group in byte order of its name, a folder's contents only while it is open (contract §1).
///
/// Until the totals, filter and sort land (tasks T007–T018), every `sort` orders as
/// [`TreeSort::Path`], `query` is not applied and folder totals are 0.
pub fn layout(leaves: &[Leaf], options: &LayoutOptions) -> Vec<Row> {
    let mut root = Node::default();
    for (index, leaf) in leaves.iter().enumerate() {
        let mut at = &mut root;
        let mut parts = leaf.path.split('/').peekable();
        while let Some(part) = parts.next() {
            if parts.peek().is_some() {
                at = at.folders.entry(part).or_default();
            } else {
                at.files.push((part, index));
            }
        }
    }
    let mut out = Vec::new();
    walk(&root, "", 0, options, &mut out);
    out
}

fn walk(node: &Node, prefix: &str, depth: u16, options: &LayoutOptions, out: &mut Vec<Row>) {
    for (name, mut inner) in &node.folders {
        let mut path = format!("{prefix}{name}");
        let mut label = (*name).to_owned();
        let mut paths = vec![path.clone()];
        while options.chain && inner.files.is_empty() && inner.folders.len() == 1 {
            let Some((next, deeper)) = inner.folders.iter().next() else {
                break;
            };
            path = format!("{path}/{next}");
            label = format!("{label}/{next}");
            paths.push(path.clone());
            inner = deeper;
        }
        let open = match options.openness {
            Openness::DefaultOpen { collapsed } => !paths.iter().any(|p| collapsed.contains(p)),
            Openness::DefaultClosed { open } => paths.iter().all(|p| open.contains(p)),
        };
        out.push(Row {
            depth,
            label,
            spans: Vec::new(),
            kind: RowKind::Folder(FolderRow {
                paths,
                open,
                added: 0,
                removed: 0,
            }),
        });
        if open {
            walk(inner, &format!("{path}/"), depth + 1, options, out);
        }
    }
    let mut files = node.files.clone();
    files.sort_by_key(|&(name, _)| name);
    for (name, leaf) in files {
        out.push(Row {
            depth,
            label: name.to_owned(),
            spans: Vec::new(),
            kind: RowKind::File { leaf },
        });
    }
}

/// One open view's tree choices (data-model §2): kept for as long as the view is, not shared
/// between views (FR-013).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TreeChoices {
    /// The filter text as typed.
    pub filter: String,
    /// The chosen order.
    pub sort: TreeSort,
    /// Real folder paths the reviewer collapsed; never pruned (a folder that leaves the list and
    /// returns keeps its state).
    pub collapsed: BTreeSet<String>,
    /// Folders folded while the filter holds text (research R2).
    pub filter_collapsed: BTreeSet<String>,
}

impl TreeChoices {
    /// Collapse an open folder row (its deepest folder only) or expand a collapsed one (every
    /// folder it stands for), so a chained row opens whichever of its folders was collapsed.
    pub fn toggle(&mut self, row: &FolderRow) {
        if row.open {
            self.collapsed.extend(row.paths.last().cloned());
        } else {
            for path in &row.paths {
                self.collapsed.remove(path);
            }
        }
    }

    /// Open every folder above `path`, so a selected file is listed (FR-008).
    pub fn reveal(&mut self, path: &str) {
        for (end, _) in path.match_indices('/') {
            let ancestor = &path[..end];
            self.collapsed.remove(ancestor);
            self.filter_collapsed.remove(ancestor);
        }
    }

    /// The changed-file tree's layout options: chained, open except the collapsed folders, in the
    /// chosen order. The caller builds `query` from [`TreeChoices::filter`] once per layout.
    pub fn options<'a>(
        &'a self,
        query: &'a Query,
        reviewed: &'a dyn Fn(&str) -> bool,
    ) -> LayoutOptions<'a> {
        LayoutOptions {
            chain: true,
            openness: Openness::DefaultOpen {
                collapsed: &self.collapsed,
            },
            sort: self.sort,
            query: Some(query),
            reviewed,
        }
    }
}
