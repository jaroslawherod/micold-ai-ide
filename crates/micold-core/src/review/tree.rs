//! The changed-file tree (feature 1273): laying out a list of paths as folder and file rows, and
//! the per-view choices that steer it — which folders are collapsed, the filter, the sort.
//!
//! Render-free and iced-free (Principle I): the client draws the [`Row`]s this module returns and
//! decides nothing about grouping, chaining or order itself. The **Files** browser lays out its
//! tree here too, without chaining and with folders closed by default (FR-017).
//!
//! Contract: `specs/1273-review-file-tree/contracts/review-tree.md`; types:
//! `specs/1273-review-file-tree/data-model.md` §1–§3.

use std::cmp::Reverse;
use std::collections::{BTreeMap, BTreeSet};
use std::ops::Range;

use crate::typeahead::{self, Query};

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

/// A folder's contents while the tree is laid out: sub-folders by name, files as (name, leaf),
/// and what the files below it add up to.
#[derive(Default)]
struct Node<'a> {
    folders: BTreeMap<&'a str, Node<'a>>,
    files: Vec<(&'a str, usize)>,
    /// Lines added over the listed files below it.
    added: u64,
    /// Lines removed over the listed files below it.
    removed: u64,
    /// Whether a listed file below it is unreviewed; kept only under `UnreviewedFirst`.
    unreviewed: bool,
    /// Matched byte ranges of the listed files below it that fall in this folder's name and the
    /// `/` after it, relative to the name's start.
    spans: Vec<Range<usize>>,
}

/// What one listed file brings to the layout.
struct Listed {
    /// The matched byte ranges of its path; empty without a filter.
    spans: Vec<Range<usize>>,
    /// Whether it is unreviewed (read only under `UnreviewedFirst`).
    unreviewed: bool,
}

/// The visible rows of `leaves` laid out as a tree (contract §1): only the files `query` matches,
/// folders before files at each level, each group in `sort` order with ties in byte order of the
/// name, a folder's contents only while it is open, and each folder summing its listed files.
pub fn layout(leaves: &[Leaf], options: &LayoutOptions) -> Vec<Row> {
    let query = options.query.filter(|query| !query.is_empty());
    let unreviewed_first = options.sort == TreeSort::UnreviewedFirst;
    let mut listed: Vec<Option<Listed>> = Vec::with_capacity(leaves.len());
    let mut root = Node::default();
    for (index, leaf) in leaves.iter().enumerate() {
        let spans = match query {
            Some(query) => match typeahead::match_one(leaf.path, query) {
                Some(found) => found.spans,
                None => {
                    listed.push(None);
                    continue;
                }
            },
            None => Vec::new(),
        };
        let unreviewed = unreviewed_first && !(options.reviewed)(leaf.path);
        let mut at = &mut root;
        let mut start = 0;
        let mut parts = leaf.path.split('/').peekable();
        while let Some(part) = parts.next() {
            let end = start + part.len();
            if parts.peek().is_some() {
                at = at.folders.entry(part).or_default();
                at.added += u64::from(leaf.added);
                at.removed += u64::from(leaf.removed);
                at.unreviewed |= unreviewed;
                // The name and its `/`, so a span across a chain's `/` stays whole.
                at.spans.extend(clip(&spans, start..end + 1));
            } else {
                at.files.push((part, index));
            }
            start = end + 1;
        }
        listed.push(Some(Listed { spans, unreviewed }));
    }
    let mut out = Vec::new();
    let walker = Walker {
        leaves,
        listed: &listed,
        options,
    };
    walker.walk(&root, "", 0, &mut out);
    out
}

/// The parts of `spans` inside `range`, relative to its start.
fn clip(spans: &[Range<usize>], range: Range<usize>) -> impl Iterator<Item = Range<usize>> + '_ {
    spans.iter().filter_map(move |span| {
        let start = span.start.max(range.start);
        let end = span.end.min(range.end);
        (start < end).then(|| start - range.start..end - range.start)
    })
}

/// `spans` sorted, with overlapping and touching ranges merged.
fn merged(mut spans: Vec<Range<usize>>) -> Vec<Range<usize>> {
    spans.sort_by_key(|span| (span.start, span.end));
    let mut out: Vec<Range<usize>> = Vec::with_capacity(spans.len());
    for span in spans {
        match out.last_mut() {
            Some(last) if span.start <= last.end => last.end = last.end.max(span.end),
            _ => out.push(span),
        }
    }
    out
}

/// What [`layout`] walks the tree with.
struct Walker<'a, 'o> {
    leaves: &'a [Leaf<'a>],
    listed: &'a [Option<Listed>],
    options: &'a LayoutOptions<'o>,
}

impl Walker<'_, '_> {
    fn walk(&self, node: &Node, prefix: &str, depth: u16, out: &mut Vec<Row>) {
        let options = self.options;
        let mut folders: Vec<(&str, &Node)> = node.folders.iter().map(|(n, f)| (*n, f)).collect();
        match options.sort {
            TreeSort::Path => {}
            TreeSort::Size => folders.sort_by_key(|(_, f)| Reverse(f.added + f.removed)),
            TreeSort::UnreviewedFirst => folders.sort_by_key(|(_, f)| !f.unreviewed),
        }
        for (name, mut inner) in folders {
            let mut path = format!("{prefix}{name}");
            let mut label = name.to_owned();
            let mut paths = vec![path.clone()];
            let mut spans: Vec<Range<usize>> = inner.spans.clone();
            while options.chain && inner.files.is_empty() && inner.folders.len() == 1 {
                let Some((next, deeper)) = inner.folders.iter().next() else {
                    break;
                };
                let offset = label.len() + 1;
                spans.extend(
                    deeper
                        .spans
                        .iter()
                        .map(|s| s.start + offset..s.end + offset),
                );
                path = format!("{path}/{next}");
                label = format!("{label}/{next}");
                paths.push(path.clone());
                inner = deeper;
            }
            let spans = merged(clip(&spans, 0..label.len()).collect());
            let open = match options.openness {
                Openness::DefaultOpen { collapsed } => !paths.iter().any(|p| collapsed.contains(p)),
                Openness::DefaultClosed { open } => paths.iter().all(|p| open.contains(p)),
            };
            out.push(Row {
                depth,
                label,
                spans,
                kind: RowKind::Folder(FolderRow {
                    paths,
                    open,
                    added: inner.added,
                    removed: inner.removed,
                }),
            });
            if open {
                self.walk(inner, &format!("{path}/"), depth + 1, out);
            }
        }
        let mut files = node.files.clone();
        files.sort_by_key(|&(name, _)| name);
        match options.sort {
            TreeSort::Path => {}
            TreeSort::Size => files.sort_by_key(|&(_, leaf)| {
                let leaf = &self.leaves[leaf];
                Reverse(u64::from(leaf.added) + u64::from(leaf.removed))
            }),
            TreeSort::UnreviewedFirst => files.sort_by_key(|&(_, leaf)| {
                !self.listed[leaf].as_ref().is_some_and(|l| l.unreviewed)
            }),
        }
        for (name, leaf) in files {
            let path = self.leaves[leaf].path;
            let spans = self.listed[leaf]
                .as_ref()
                .map(|l| clip(&l.spans, path.len() - name.len()..path.len()).collect())
                .unwrap_or_default();
            out.push(Row {
                depth,
                label: name.to_owned(),
                spans,
                kind: RowKind::File { leaf },
            });
        }
    }
}

/// The shortest bar drawn for a counted change, in px, so a one-line change stays visible.
pub const MIN_BAR: f32 = 2.0;

/// A file's change bar: the lengths of its added and removed parts, in px (data-model §5).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BarParts {
    /// The added (green) part.
    pub added: f32,
    /// The removed (red) part.
    pub removed: f32,
}

/// The bar scale: the largest `added + removed` over the **whole** change list, not the filtered
/// rows, so bars do not jump while the filter is typed (research R8).
pub fn bar_scale(leaves: &[Leaf]) -> u64 {
    leaves
        .iter()
        .map(|leaf| u64::from(leaf.added) + u64::from(leaf.removed))
        .max()
        .unwrap_or(0)
}

/// A file's bar against `scale`: its length grows with `added + removed`, is at least
/// [`MIN_BAR`] and never more than `max_width`, split into the two parts in exact proportion
/// (FR-006). `None` when the file has no counted lines (binary, not text, Mode only, pure rename)
/// or the scale is 0.
pub fn bar(added: u32, removed: u32, scale: u64, max_width: f32) -> Option<BarParts> {
    let total = u64::from(added) + u64::from(removed);
    if total == 0 || scale == 0 {
        return None;
    }
    let ratio = (total as f64 / scale as f64).min(1.0);
    let length = ((f64::from(max_width) * ratio) as f32)
        .max(MIN_BAR)
        .min(max_width);
    let added = (f64::from(length) * f64::from(added) / total as f64) as f32;
    Some(BarParts {
        added,
        removed: (length - added).max(0.0),
    })
}

/// How many indentation steps a row may take in a list `list_width` wide so its name keeps
/// `min_name_width` (edge case "Very deep paths").
pub fn max_indent_levels(list_width: f32, step: f32, min_name_width: f32) -> u16 {
    if step <= 0.0 {
        return u16::MAX;
    }
    // A float-to-int `as` saturates: negative room is 0 levels.
    ((list_width - min_name_width) / step).floor() as u16
}

/// The indentation steps a row at `depth` draws with: its depth, capped at `max`.
pub fn indent_levels(depth: u16, max: u16) -> u16 {
    depth.min(max)
}

/// A file row's tooltip: its full path, and the old path of a rename (FR-005).
pub fn hover_text(path: &str, renamed_from: Option<&str>) -> String {
    match renamed_from {
        Some(old) => format!("{path}\nRenamed from {old}"),
        None => path.to_owned(),
    }
}

/// Whether `text` filters anything: [`Query::new`] trims it, so whitespace alone does not.
fn is_filter(text: &str) -> bool {
    !text.trim().is_empty()
}

/// Whether the sort control offers a sort (data-model §4).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Offer {
    /// Offered and choosable.
    Shown,
    /// Not offered.
    Hidden,
    /// Offered but not choosable, with the reason shown on it.
    Disabled(&'static str),
}

/// What the control offers for **Unreviewed first** until #1271's reviewed marks exist — the one
/// switch for spec 4.3 (research R3).
pub const UNREVIEWED_UNTIL_MARKS: Offer = Offer::Hidden;

/// The sorts the control offers, in its order; `marks` is whether reviewed marks exist (FR-012).
pub fn sort_options(marks: bool) -> [(TreeSort, Offer); 3] {
    [
        (TreeSort::Path, Offer::Shown),
        (TreeSort::Size, Offer::Shown),
        (
            TreeSort::UnreviewedFirst,
            if marks {
                Offer::Shown
            } else {
                UNREVIEWED_UNTIL_MARKS
            },
        ),
    ]
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
    ///
    /// While the filter holds text the fold goes to [`TreeChoices::filter_collapsed`], so it never
    /// changes the tree restored when the filter is cleared (FR-010).
    pub fn toggle(&mut self, row: &FolderRow) {
        let active = self.active_mut();
        if row.open {
            active.extend(row.paths.last().cloned());
        } else {
            for path in &row.paths {
                active.remove(path);
            }
        }
    }

    /// Store the filter text as typed; starting a filter (from no text) drops the folds of any
    /// earlier filter.
    pub fn set_filter(&mut self, text: &str) {
        if !self.filtering() && is_filter(text) {
            self.filter_collapsed.clear();
        }
        text.clone_into(&mut self.filter);
    }

    /// Empty the filter and its folds; the folds made before it hold again (FR-010).
    pub fn clear_filter(&mut self) {
        self.filter.clear();
        self.filter_collapsed.clear();
    }

    /// Choose `sort` if `offers` shows it; a sort the control does not show is never stored
    /// (FR-012).
    pub fn set_sort(&mut self, sort: TreeSort, offers: &[(TreeSort, Offer)]) {
        if offers.contains(&(sort, Offer::Shown)) {
            self.sort = sort;
        }
    }

    /// Whether the filter holds text; whitespace alone is no filter.
    fn filtering(&self) -> bool {
        is_filter(&self.filter)
    }

    /// The folds the tree reads now: the filter's while it holds text, else the reviewer's.
    fn active(&self) -> &BTreeSet<String> {
        if self.filtering() {
            &self.filter_collapsed
        } else {
            &self.collapsed
        }
    }

    fn active_mut(&mut self) -> &mut BTreeSet<String> {
        if self.filtering() {
            &mut self.filter_collapsed
        } else {
            &mut self.collapsed
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

    /// The changed-file tree's layout options: chained, open except the active set's folders, in
    /// the chosen order. The caller builds `query` from [`TreeChoices::filter`] once per layout.
    pub fn options<'a>(
        &'a self,
        query: &'a Query,
        reviewed: &'a dyn Fn(&str) -> bool,
    ) -> LayoutOptions<'a> {
        LayoutOptions {
            chain: true,
            openness: Openness::DefaultOpen {
                collapsed: self.active(),
            },
            sort: self.sort,
            query: Some(query),
            reviewed,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn leaf(path: &str, added: u32, removed: u32) -> Leaf<'_> {
        Leaf {
            path,
            added,
            removed,
        }
    }

    // --- T008: bar maths (data-model §5, research R8) ---------------------------------------

    #[test]
    fn bar_scale_is_the_largest_change_over_the_whole_list() {
        let leaves = [leaf("a", 40, 10), leaf("b", 100, 0), leaf("c", 3, 90)];
        assert_eq!(bar_scale(&leaves), 100);
        assert_eq!(bar_scale(&[]), 0);
        assert_eq!(
            bar_scale(&[leaf("big", u32::MAX, u32::MAX)]),
            2 * u64::from(u32::MAX)
        );
    }

    /// Scenario 2.1: 40 added and 10 removed split 4:1.
    #[test]
    fn a_bar_splits_added_and_removed_in_proportion() {
        let parts = bar(40, 10, 100, 50.0).expect("a bar");
        assert!(
            (parts.added - 4.0 * parts.removed).abs() < 1e-4,
            "{parts:?}"
        );
        assert!(
            (parts.added + parts.removed - 25.0).abs() < 1e-4,
            "{parts:?}"
        );
    }

    /// Scenario 2.2: a larger change draws a longer bar, and none exceeds the maximum width.
    #[test]
    fn a_larger_change_is_longer_and_no_bar_exceeds_the_width() {
        let long = bar(60, 40, 100, 48.0).expect("a bar");
        let short = bar(7, 3, 100, 48.0).expect("a bar");
        assert!(long.added + long.removed > short.added + short.removed);
        assert!((long.added + long.removed - 48.0).abs() < 1e-4);
        for (added, removed) in [(100, 0), (0, 100), (50, 50), (1, 0), (99, 1)] {
            let parts = bar(added, removed, 100, 48.0).expect("a bar");
            assert!(parts.added + parts.removed <= 48.0 + 1e-4, "{parts:?}");
            assert!(parts.added >= 0.0 && parts.removed >= 0.0, "{parts:?}");
        }
        let over = bar(500, 0, 100, 48.0).expect("a bar");
        assert!(over.added + over.removed <= 48.0 + 1e-4, "{over:?}");
    }

    /// Scenarios 2.3, 2.4 and a pure rename: no counted lines, no bar; nor with no scale.
    #[test]
    fn no_counted_lines_or_no_scale_draws_no_bar() {
        assert_eq!(bar(0, 0, 100, 48.0), None);
        assert_eq!(bar(5, 5, 0, 48.0), None);
    }

    /// A one-line change in a list with a huge one stays visible.
    #[test]
    fn a_one_line_change_is_at_least_the_minimum_bar() {
        let parts = bar(1, 0, 100_000, 48.0).expect("a bar");
        assert!(parts.added + parts.removed >= MIN_BAR - 1e-6, "{parts:?}");
        assert_eq!(MIN_BAR, 2.0);
        let parts = bar(0, 1, 100_000, 48.0).expect("a bar");
        assert!(
            parts.removed >= MIN_BAR - 1e-6 && parts.added == 0.0,
            "{parts:?}"
        );
    }

    // --- T016: the sort offer (FR-012, research R3) ------------------------------------------

    #[test]
    fn unreviewed_first_is_hidden_until_marks_exist() {
        assert_eq!(UNREVIEWED_UNTIL_MARKS, Offer::Hidden);
        assert_eq!(
            sort_options(false),
            [
                (TreeSort::Path, Offer::Shown),
                (TreeSort::Size, Offer::Shown),
                (TreeSort::UnreviewedFirst, UNREVIEWED_UNTIL_MARKS),
            ]
        );
        assert_eq!(
            sort_options(true),
            [
                (TreeSort::Path, Offer::Shown),
                (TreeSort::Size, Offer::Shown),
                (TreeSort::UnreviewedFirst, Offer::Shown),
            ]
        );
    }

    /// A sort the control does not show is never stored.
    #[test]
    fn set_sort_stores_only_a_shown_sort() {
        let mut choices = TreeChoices::default();
        choices.set_sort(TreeSort::UnreviewedFirst, &sort_options(false));
        assert_eq!(choices.sort, TreeSort::Path);
        choices.set_sort(TreeSort::Size, &sort_options(false));
        assert_eq!(choices.sort, TreeSort::Size);
        let disabled = [(TreeSort::Path, Offer::Disabled("not yet"))];
        choices.set_sort(TreeSort::Path, &disabled);
        assert_eq!(choices.sort, TreeSort::Size);
        choices.set_sort(TreeSort::Path, &[]);
        assert_eq!(choices.sort, TreeSort::Size);
        choices.set_sort(TreeSort::UnreviewedFirst, &sort_options(true));
        assert_eq!(choices.sort, TreeSort::UnreviewedFirst);
        choices.set_sort(TreeSort::Path, &sort_options(true));
        assert_eq!(choices.sort, TreeSort::Path);
    }

    // --- T009: indentation cap and hover text (FR-005) ----------------------------------------

    /// The name keeps `min_name_width` before indentation grows.
    #[test]
    fn max_indent_levels_leaves_room_for_the_name() {
        assert_eq!(max_indent_levels(300.0, 16.0, 120.0), 11);
        assert_eq!(max_indent_levels(120.0, 16.0, 120.0), 0);
        assert_eq!(max_indent_levels(100.0, 16.0, 120.0), 0);
        assert_eq!(max_indent_levels(136.0, 16.0, 120.0), 1);
    }

    /// Edge case "Very deep paths": depth beyond the cap draws at the cap.
    #[test]
    fn indent_levels_is_depth_capped_at_the_maximum() {
        assert_eq!(indent_levels(3, 10), 3);
        assert_eq!(indent_levels(12, 10), 10);
        assert_eq!(indent_levels(0, 0), 0);
        assert_eq!(indent_levels(5, 0), 0);
    }

    /// FR-005: the full path on hover, and the old path of a rename.
    #[test]
    fn hover_text_is_the_path_and_a_renames_old_path() {
        assert_eq!(hover_text("src/ui/b.rs", None), "src/ui/b.rs");
        assert_eq!(
            hover_text("src/new.rs", Some("lib/old.rs")),
            "src/new.rs\nRenamed from lib/old.rs"
        );
    }
}
