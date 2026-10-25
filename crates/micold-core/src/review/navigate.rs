//! The Review tab's keyboard navigation rules (#1274): which file `j`/`k` select, which hunk
//! `n`/`p` make current (crossing files with a pending edge), which comment `]`/`[` show, which
//! file `g` jumps to, and what `c` opens. Render-free and over plain inputs, so the client's view
//! state only feeds them (research R1, R5). Contract: `specs/1274-review-keyboard-map/contracts/`.
//!
//! Every rule takes the file order as the tree lists it now (a filtered tree passes only the files
//! it shows), so a press re-reads the order and never keeps a stale position.

use std::collections::HashMap;

use super::changes::{ChangedFile, Content};
use super::comment::CommentId;
use super::diff::{DiffLine, Hunk, LineKind};
use super::{LineRange, RelPath, Side};
use crate::navigation::{self, Dir};

/// The note `g` shows when every file is marked reviewed (FR-008).
pub const ALL_FILES_REVIEWED: &str = "All files reviewed";

/// The note `c` shows with no picked lines and no current hunk (FR-009).
pub const PICK_OR_HUNK: &str = "Pick lines or move to a hunk (n) to comment";

/// One file of the review, in file order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NavFile {
    /// Its path.
    pub path: RelPath,
    /// Whether its diff has hunks `n`/`p` can stop on ([`has_hunks`]).
    pub has_hunks: bool,
}

/// Whether a listed working-tree file has hunks: a text diff with lines changed, and not a large
/// file whose diff waits for "Show diff" (`large_shown` false). Binary, not-UTF-8, mode-only and
/// pure renames have none.
pub fn has_hunks(file: &ChangedFile, large_shown: bool) -> bool {
    file.content == Content::Text && file.added + file.removed > 0 && (!file.large || large_shown)
}

/// The file `j` (`Next`) or `k` (`Previous`) selects (FR-003): the neighbour of `selected` in
/// `order`, wrapping; with no selection, or one no longer listed, the first or the last file.
pub fn step_file(order: &[NavFile], selected: Option<&RelPath>, dir: Dir) -> Option<RelPath> {
    let paths: Vec<&RelPath> = order.iter().map(|f| &f.path).collect();
    navigation::step(&paths, selected.as_ref(), dir).cloned()
}

/// The selected file's diff, as the view holds it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SelectedDiff {
    /// Still being read.
    Loading,
    /// Reading it failed.
    Failed,
    /// Shown, with this many hunks.
    Hunks(usize),
}

/// Which hunk to make current once a file's diff arrives.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Edge {
    /// Its first hunk (`n` crossed into it).
    First,
    /// Its last hunk (`p` crossed into it).
    Last,
}

/// What `n` or `p` does (FR-004, research R5).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HunkMove {
    /// Make this hunk of the selected file current.
    Current(usize),
    /// Select this file and, once its diff loads, make the hunk at `edge` current.
    File {
        /// The file to select.
        path: RelPath,
        /// The hunk to make current when its diff arrives.
        edge: Edge,
    },
    /// Nothing to move to.
    Nothing,
}

/// What `n` (`Next`) or `p` (`Previous`) does from hunk `current` of `selected`, whose diff is
/// `diff` (FR-004). Inside the file it moves one hunk; past an end it crosses to the next or
/// previous file in `order` that has hunks, wrapping, with the edge to make current once that file
/// loads; wrapping back to `selected` itself stays in it. A diff still loading or failed gives
/// `Nothing`; with no selection (or one no longer listed) it goes to the first or last file with
/// hunks.
pub fn step_hunk(
    order: &[NavFile],
    selected: Option<&RelPath>,
    diff: SelectedDiff,
    current: Option<usize>,
    dir: Dir,
) -> HunkMove {
    let Some(at) = selected.and_then(|s| order.iter().position(|f| &f.path == s)) else {
        return cross(order, None, dir);
    };
    let count = match diff {
        SelectedDiff::Hunks(count) => count,
        SelectedDiff::Loading | SelectedDiff::Failed => return HunkMove::Nothing,
    };
    let inside = match (current, dir) {
        (None, Dir::Next) => (count > 0).then_some(0),
        (None, Dir::Previous) => count.checked_sub(1),
        (Some(i), Dir::Next) => (i + 1 < count).then_some(i + 1),
        // `min` clamps a current hunk past a re-read, shorter diff to just after its last.
        (Some(i), Dir::Previous) => i.min(count).checked_sub(1),
    };
    if let Some(i) = inside {
        return HunkMove::Current(i);
    }
    match cross(order, Some(at), dir) {
        // Wrapped back to the selected file itself: stay in it, at the far end.
        HunkMove::File { path, edge } if path == order[at].path => {
            resolve_edge(edge, count).map_or(HunkMove::Nothing, HunkMove::Current)
        }
        other => other,
    }
}

/// The next (`Next`) or previous file with hunks after index `from` of `order`, wrapping and
/// ending on `from` itself; from the start or end of the order when `from` is `None`.
fn cross(order: &[NavFile], from: Option<usize>, dir: Dir) -> HunkMove {
    let len = order.len();
    let edge = match dir {
        Dir::Next => Edge::First,
        Dir::Previous => Edge::Last,
    };
    let found = (1..=len)
        .map(|step| match (from, dir) {
            (Some(at), Dir::Next) => (at + step) % len,
            (Some(at), Dir::Previous) => (at + len - step % len) % len,
            (None, Dir::Next) => step - 1,
            (None, Dir::Previous) => len - step,
        })
        .find(|&i| order[i].has_hunks);
    found.map_or(HunkMove::Nothing, |i| HunkMove::File {
        path: order[i].path.clone(),
        edge,
    })
}

/// The hunk index `edge` names in a diff of `count` hunks; `None` for a diff with none.
pub fn resolve_edge(edge: Edge, count: usize) -> Option<usize> {
    match edge {
        Edge::First => (count > 0).then_some(0),
        Edge::Last => count.checked_sub(1),
    }
}

/// What `g` does (FR-008).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Unreviewed {
    /// Select this file.
    Select(RelPath),
    /// Every file is marked reviewed: keep the selection, show [`ALL_FILES_REVIEWED`].
    AllReviewed,
    /// The review lists no file.
    Nothing,
}

/// The first file after `selected` in `order`, wrapping, that `is_reviewed` says is not marked
/// reviewed (FR-008). `is_reviewed` is read at the call, so a mark made elsewhere counts (US2 s5).
/// With no selection the scan starts at the first file; with the selected file the only one left
/// it is returned (the selection stays).
pub fn next_unreviewed(
    order: &[NavFile],
    selected: Option<&RelPath>,
    is_reviewed: impl Fn(&RelPath) -> bool,
) -> Unreviewed {
    let len = order.len();
    if len == 0 {
        return Unreviewed::Nothing;
    }
    let start = selected
        .and_then(|s| order.iter().position(|f| &f.path == s))
        .map_or(0, |at| at + 1);
    (0..len)
        .map(|step| &order[(start + step) % len].path)
        .find(|path| !is_reviewed(path))
        .map_or(Unreviewed::AllReviewed, |path| {
            Unreviewed::Select(path.clone())
        })
}

/// What `]` and `[` know of one comment.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommentFact {
    /// Its identity.
    pub id: CommentId,
    /// Its file.
    pub path: RelPath,
    /// Its located first line.
    pub line: u32,
    /// When it was made (Unix seconds).
    pub created: u64,
    /// Whether its card is in the selected diff when its file is shown (not outdated).
    pub in_diff: bool,
}

/// `facts` in comment order (FR-006): by their file's index in `order`, then line, then
/// `created`, then id; comments of files `order` does not list go last, by path.
pub fn comment_order(facts: &[CommentFact], order: &[NavFile]) -> Vec<CommentFact> {
    let index: HashMap<&RelPath, usize> = order
        .iter()
        .enumerate()
        .map(|(i, f)| (&f.path, i))
        .collect();
    let mut ordered = facts.to_vec();
    // Listed files first by their index (`Some` sorts after `None`, hence the flag), then
    // unlisted ones by path.
    ordered.sort_by(|a, b| {
        let file = |f: &CommentFact| match index.get(&f.path) {
            Some(&i) => (false, i, None),
            None => (true, 0, Some(f.path.clone())),
        };
        (file(a), a.line, a.created, a.id).cmp(&(file(b), b.line, b.created, b.id))
    });
    ordered
}

/// How a comment reached by `]`/`[` is shown.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Show {
    /// Scroll its card in the diff into view.
    Card,
    /// Open the comments drawer on it (it is not in the diff).
    Drawer,
}

/// What `]` or `[` does.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommentMove {
    /// The comment reached.
    pub id: CommentId,
    /// The file to select first: only a listed file other than the selected one.
    pub select: Option<RelPath>,
    /// How it is shown.
    pub show: Show,
}

/// The comment `]` (`Next`) or `[` (`Previous`) reaches from `last`, the one the previous press
/// reached, in `ordered` ([`comment_order`]), wrapping (FR-006). With no `last`, or one no longer
/// present, the first or last comment. `None` when there are no comments.
pub fn step_comment(
    ordered: &[CommentFact],
    order: &[NavFile],
    selected: Option<&RelPath>,
    last: Option<CommentId>,
    dir: Dir,
) -> Option<CommentMove> {
    let ids: Vec<CommentId> = ordered.iter().map(|f| f.id).collect();
    let id = navigation::step(&ids, last.as_ref(), dir)?;
    let fact = ordered.iter().find(|f| f.id == id)?;
    let listed = order.iter().any(|f| f.path == fact.path);
    let select = (listed && selected != Some(&fact.path)).then(|| fact.path.clone());
    let show = if listed && fact.in_diff {
        Show::Card
    } else {
        Show::Drawer
    };
    Some(CommentMove { id, select, show })
}

/// The lines `c` comments on in a hunk (FR-009): its first to last added line on the new side,
/// or, for a hunk that only removes, its first to last removed line on the old side. `None` for a
/// hunk with neither.
pub fn hunk_change_range(hunk: &Hunk) -> Option<(Side, LineRange)> {
    let span = |kind: LineKind, number: fn(&DiffLine) -> Option<u32>| {
        let mut numbers = hunk
            .lines
            .iter()
            .filter(|l| l.kind == kind)
            .filter_map(number);
        let first = numbers.next()?;
        LineRange::new(first, numbers.next_back().unwrap_or(first))
    };
    span(LineKind::Added, |l| l.new)
        .map(|range| (Side::New, range))
        .or_else(|| span(LineKind::Removed, |l| l.old).map(|range| (Side::Old, range)))
}

/// What `c` does (FR-009).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommentOpen {
    /// A composer is already open: focus it, keeping its text.
    FocusComposer,
    /// Open the composer on the picked lines, as **Add comment** does.
    Picked,
    /// Pick these lines of the current hunk, then open the composer on them.
    Hunk {
        /// The side the lines are numbered on.
        side: Side,
        /// The lines.
        range: LineRange,
    },
    /// Open nothing; show [`PICK_OR_HUNK`].
    Refused,
}

/// What `c` opens, in this order of precedence: an open composer, picked lines, the current
/// hunk's changed lines, else nothing (FR-009).
pub fn comment_target(composer_open: bool, picked: bool, current: Option<&Hunk>) -> CommentOpen {
    if composer_open {
        return CommentOpen::FocusComposer;
    }
    if picked {
        return CommentOpen::Picked;
    }
    current
        .and_then(hunk_change_range)
        .map_or(CommentOpen::Refused, |(side, range)| CommentOpen::Hunk {
            side,
            range,
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::review::changes::{ChangeKind, Origin};
    use std::time::{Duration, Instant};
    use uuid::Uuid;

    fn path(p: &str) -> RelPath {
        RelPath::from_native(p).expect("a relative test path")
    }

    fn files(spec: &[(&str, bool)]) -> Vec<NavFile> {
        spec.iter()
            .map(|(p, has_hunks)| NavFile {
                path: path(p),
                has_hunks: *has_hunks,
            })
            .collect()
    }

    fn abc() -> Vec<NavFile> {
        files(&[("a.rs", true), ("b.rs", true), ("c.rs", true)])
    }

    fn changed(content: Content, added: u32, removed: u32, large: bool) -> ChangedFile {
        ChangedFile {
            path: path("a.rs"),
            kind: ChangeKind::Modified,
            content,
            added,
            removed,
            large,
            origin: Origin::Uncommitted,
        }
    }

    // --- has_hunks, step_file (T006) ---

    #[test]
    fn a_text_file_with_changed_lines_has_hunks_and_the_others_do_not() {
        assert!(
            has_hunks(&changed(Content::Text, 2, 1, false), false),
            "text with changes"
        );
        assert!(
            !has_hunks(&changed(Content::Binary, 0, 0, false), false),
            "binary has none"
        );
        assert!(
            !has_hunks(&changed(Content::NotUtf8, 0, 0, false), false),
            "not UTF-8 has none"
        );
        let mut renamed = changed(Content::Text, 0, 0, false);
        renamed.kind = ChangeKind::Renamed {
            from: path("old.rs"),
        };
        assert!(!has_hunks(&renamed, false), "a pure rename changes no line");
        let large = changed(Content::Text, 9_000, 0, true);
        assert!(
            !has_hunks(&large, false),
            "a large file not shown has no hunk to stop on"
        );
        assert!(
            has_hunks(&large, true),
            "a large file once shown has its hunks"
        );
    }

    #[test]
    fn j_and_k_step_through_the_file_order_and_wrap() {
        let order = abc();
        assert_eq!(
            step_file(&order, Some(&path("a.rs")), Dir::Next),
            Some(path("b.rs")),
            "US1 s1: j selects the next file"
        );
        assert_eq!(
            step_file(&order, Some(&path("c.rs")), Dir::Next),
            Some(path("a.rs")),
            "US1 s2: j on the last file wraps to the first"
        );
        assert_eq!(
            step_file(&order, Some(&path("a.rs")), Dir::Previous),
            Some(path("c.rs")),
            "US1 s2: k on the first file wraps to the last"
        );
    }

    #[test]
    fn j_and_k_visit_only_the_order_passed() {
        let filtered = files(&[("b.rs", true), ("c.rs", true)]);
        assert_eq!(
            step_file(&filtered, Some(&path("b.rs")), Dir::Next),
            Some(path("c.rs")),
            "US1 s5: a filtered tree passes only its files"
        );
        assert_eq!(
            step_file(&filtered, Some(&path("c.rs")), Dir::Next),
            Some(path("b.rs")),
            "US1 s5: a.rs, filtered out, is never visited"
        );
    }

    #[test]
    fn with_no_selection_or_an_unlisted_one_j_takes_the_first_and_k_the_last() {
        let order = abc();
        for selected in [None, Some(path("gone.rs"))] {
            assert_eq!(
                step_file(&order, selected.as_ref(), Dir::Next),
                Some(path("a.rs")),
                "FR-003: j with no listed selection selects the first file"
            );
            assert_eq!(
                step_file(&order, selected.as_ref(), Dir::Previous),
                Some(path("c.rs")),
                "FR-003: k with no listed selection selects the last file"
            );
        }
        assert_eq!(
            step_file(&[], None, Dir::Next),
            None,
            "an empty review has nothing"
        );
    }

    // --- step_hunk, resolve_edge (T008) ---

    fn hunk_step(
        order: &[NavFile],
        selected: Option<&str>,
        diff: SelectedDiff,
        current: Option<usize>,
        dir: Dir,
    ) -> HunkMove {
        step_hunk(order, selected.map(path).as_ref(), diff, current, dir)
    }

    fn to(p: &str, edge: Edge) -> HunkMove {
        HunkMove::File {
            path: path(p),
            edge,
        }
    }

    #[test]
    fn with_no_current_hunk_n_takes_the_first_and_p_the_last() {
        let order = abc();
        let two = SelectedDiff::Hunks(2);
        assert_eq!(
            hunk_step(&order, Some("b.rs"), two, None, Dir::Next),
            HunkMove::Current(0),
            "US1 s3: n with no current hunk makes the first current"
        );
        assert_eq!(
            hunk_step(&order, Some("b.rs"), two, None, Dir::Previous),
            HunkMove::Current(1),
            "FR-004: p with no current hunk makes the last current"
        );
    }

    #[test]
    fn inside_a_file_n_and_p_move_one_hunk() {
        let order = abc();
        let three = SelectedDiff::Hunks(3);
        assert_eq!(
            hunk_step(&order, Some("b.rs"), three, Some(0), Dir::Next),
            HunkMove::Current(1)
        );
        assert_eq!(
            hunk_step(&order, Some("b.rs"), three, Some(2), Dir::Previous),
            HunkMove::Current(1)
        );
    }

    #[test]
    fn p_from_a_hunk_past_a_shrunken_diff_takes_its_last_hunk() {
        let order = abc();
        let two = SelectedDiff::Hunks(2);
        assert_eq!(
            hunk_step(&order, Some("b.rs"), two, Some(4), Dir::Previous),
            HunkMove::Current(1),
            "a stale current hunk past the re-read diff's end: p stays in the file"
        );
    }

    #[test]
    fn past_an_end_n_and_p_cross_to_the_next_file_with_hunks() {
        let order = files(&[
            ("a.rs", true),
            ("b.rs", true),
            ("d.bin", false),
            ("c.rs", true),
        ]);
        let two = SelectedDiff::Hunks(2);
        assert_eq!(
            hunk_step(&order, Some("b.rs"), two, Some(1), Dir::Next),
            to("c.rs", Edge::First),
            "US1 s4: past the last hunk, the next file with hunks (d.bin passed over), first hunk"
        );
        assert_eq!(
            hunk_step(&order, Some("c.rs"), two, Some(0), Dir::Previous),
            to("b.rs", Edge::Last),
            "US1 s4: before the first hunk, the previous file with hunks, last hunk"
        );
        assert_eq!(
            hunk_step(&order, Some("c.rs"), two, Some(1), Dir::Next),
            to("a.rs", Edge::First),
            "FR-004: crossing wraps at the end of the file order"
        );
        assert_eq!(
            hunk_step(&order, Some("a.rs"), two, Some(0), Dir::Previous),
            to("c.rs", Edge::Last),
            "FR-004: crossing wraps at the start of the file order"
        );
    }

    #[test]
    fn a_selected_file_without_hunks_crosses_at_once() {
        let order = files(&[("a.rs", true), ("d.bin", false), ("c.rs", true)]);
        assert_eq!(
            hunk_step(
                &order,
                Some("d.bin"),
                SelectedDiff::Hunks(0),
                None,
                Dir::Next
            ),
            to("c.rs", Edge::First)
        );
        assert_eq!(
            hunk_step(
                &order,
                Some("d.bin"),
                SelectedDiff::Hunks(0),
                None,
                Dir::Previous
            ),
            to("a.rs", Edge::Last)
        );
    }

    #[test]
    fn wrapping_back_to_the_selected_file_stays_in_it() {
        let order = files(&[("a.rs", true), ("d.bin", false)]);
        let two = SelectedDiff::Hunks(2);
        assert_eq!(
            hunk_step(&order, Some("a.rs"), two, Some(1), Dir::Next),
            HunkMove::Current(0),
            "the only file with hunks: n past its last wraps to its first"
        );
        assert_eq!(
            hunk_step(&order, Some("a.rs"), two, Some(0), Dir::Previous),
            HunkMove::Current(1),
            "and p before its first wraps to its last"
        );
    }

    #[test]
    fn a_loading_or_failed_diff_moves_nothing() {
        let order = abc();
        for diff in [SelectedDiff::Loading, SelectedDiff::Failed] {
            for dir in [Dir::Next, Dir::Previous] {
                assert_eq!(
                    hunk_step(&order, Some("b.rs"), diff, None, dir),
                    HunkMove::Nothing,
                    "{diff:?}: no hunk is known yet"
                );
            }
        }
    }

    #[test]
    fn with_no_selection_n_and_p_go_to_the_first_and_last_file_with_hunks() {
        let order = files(&[
            ("d.bin", false),
            ("a.rs", true),
            ("c.rs", true),
            ("e.bin", false),
        ]);
        for selected in [None, Some("gone.rs")] {
            assert_eq!(
                hunk_step(&order, selected, SelectedDiff::Loading, None, Dir::Next),
                to("a.rs", Edge::First)
            );
            assert_eq!(
                hunk_step(&order, selected, SelectedDiff::Loading, None, Dir::Previous),
                to("c.rs", Edge::Last)
            );
        }
        let none = files(&[("d.bin", false), ("e.bin", false)]);
        assert_eq!(
            hunk_step(&none, None, SelectedDiff::Loading, None, Dir::Next),
            HunkMove::Nothing,
            "no file has hunks"
        );
        assert_eq!(
            hunk_step(
                &none,
                Some("d.bin"),
                SelectedDiff::Hunks(0),
                None,
                Dir::Next
            ),
            HunkMove::Nothing,
            "no file has hunks, the selected one included"
        );
    }

    #[test]
    fn a_pending_edge_resolves_once_the_diff_arrives() {
        assert_eq!(resolve_edge(Edge::First, 3), Some(0));
        assert_eq!(resolve_edge(Edge::Last, 3), Some(2));
        assert_eq!(resolve_edge(Edge::First, 0), None, "a diff with no hunk");
        assert_eq!(resolve_edge(Edge::Last, 0), None, "a diff with no hunk");
    }

    // --- next_unreviewed (T022) ---

    fn five() -> Vec<NavFile> {
        files(&[
            ("1.rs", true),
            ("2.rs", true),
            ("3.rs", true),
            ("4.rs", true),
            ("5.rs", true),
        ])
    }

    #[test]
    fn g_visits_the_unreviewed_files_in_order_and_wraps() {
        let order = five();
        let reviewed = |p: &RelPath| !matches!(p.as_str(), "2.rs" | "4.rs");
        let mut at = path("1.rs");
        for expected in ["2.rs", "4.rs", "2.rs"] {
            assert_eq!(
                next_unreviewed(&order, Some(&at), reviewed),
                Unreviewed::Select(path(expected)),
                "US2 s3: from {at} g selects {expected}"
            );
            at = path(expected);
        }
    }

    #[test]
    fn with_every_file_reviewed_g_reports_it() {
        assert_eq!(
            next_unreviewed(&five(), Some(&path("3.rs")), |_| true),
            Unreviewed::AllReviewed,
            "US2 s4"
        );
        assert_eq!(ALL_FILES_REVIEWED, "All files reviewed");
    }

    #[test]
    fn g_reads_the_marks_at_the_press() {
        let marked_elsewhere = std::cell::Cell::new(false);
        let reviewed = |p: &RelPath| match p.as_str() {
            "2.rs" => marked_elsewhere.get(),
            "4.rs" => false,
            _ => true,
        };
        assert_eq!(
            next_unreviewed(&five(), Some(&path("1.rs")), reviewed),
            Unreviewed::Select(path("2.rs"))
        );
        marked_elsewhere.set(true);
        assert_eq!(
            next_unreviewed(&five(), Some(&path("1.rs")), reviewed),
            Unreviewed::Select(path("4.rs")),
            "US2 s5: a file marked in another window is skipped"
        );
    }

    #[test]
    fn g_edge_cases_no_selection_only_the_selected_left_and_empty() {
        let reviewed = |p: &RelPath| p.as_str() != "3.rs";
        assert_eq!(
            next_unreviewed(&five(), None, reviewed),
            Unreviewed::Select(path("3.rs")),
            "no selection: the first unreviewed file"
        );
        assert_eq!(
            next_unreviewed(&five(), Some(&path("3.rs")), reviewed),
            Unreviewed::Select(path("3.rs")),
            "the selected file is the only one left: the selection stays"
        );
        assert_eq!(next_unreviewed(&[], None, |_| false), Unreviewed::Nothing);
    }

    // --- comment_order, step_comment (T028) ---

    fn id(n: u128) -> CommentId {
        CommentId(Uuid::from_u128(n))
    }

    fn fact(n: u128, p: &str, line: u32, created: u64, in_diff: bool) -> CommentFact {
        CommentFact {
            id: id(n),
            path: path(p),
            line,
            created,
            in_diff,
        }
    }

    fn ids(facts: &[CommentFact]) -> Vec<CommentId> {
        facts.iter().map(|f| f.id).collect()
    }

    #[test]
    fn comments_order_by_file_then_line_then_time_then_id_unlisted_last() {
        let order = abc();
        let facts = [
            fact(1, "c.rs", 7, 10, true),
            fact(2, "zz.rs", 1, 10, true),
            fact(3, "a.rs", 20, 10, true),
            fact(4, "a.rs", 3, 30, true),
            fact(5, "a.rs", 3, 20, true),
            fact(7, "a.rs", 3, 20, true),
            fact(6, "a.rs", 3, 20, true),
            fact(8, "yy.rs", 9, 10, true),
        ];
        assert_eq!(
            ids(&comment_order(&facts, &order)),
            [5, 6, 7, 4, 3, 1, 8, 2].map(id),
            "file index, line, created, id; unlisted files last by path"
        );
    }

    fn three_comments() -> Vec<CommentFact> {
        comment_order(
            &[
                fact(1, "a.rs", 3, 1, true),
                fact(2, "a.rs", 20, 1, true),
                fact(3, "c.rs", 7, 1, true),
            ],
            &abc(),
        )
    }

    #[test]
    fn bracket_steps_through_comments_selecting_their_file_and_wraps() {
        let (order, comments) = (abc(), three_comments());
        let a = path("a.rs");
        let first = step_comment(&comments, &order, Some(&a), None, Dir::Next);
        assert_eq!(
            first,
            Some(CommentMove {
                id: id(1),
                select: None,
                show: Show::Card
            }),
            "US3 s1: the first comment, in the selected file: nothing to select"
        );
        let second = step_comment(&comments, &order, Some(&a), Some(id(1)), Dir::Next);
        assert_eq!(second.map(|m| (m.id, m.select)), Some((id(2), None)));
        let third = step_comment(&comments, &order, Some(&a), Some(id(2)), Dir::Next);
        assert_eq!(
            third.map(|m| (m.id, m.select)),
            Some((id(3), Some(path("c.rs")))),
            "US3 s1: the third is in c.rs, which is selected"
        );
        let c = path("c.rs");
        let fourth = step_comment(&comments, &order, Some(&c), Some(id(3)), Dir::Next);
        assert_eq!(
            fourth.map(|m| (m.id, m.select)),
            Some((id(1), Some(a.clone()))),
            "US3 s1: a fourth ] wraps to a.rs line 3"
        );
        let back = step_comment(&comments, &order, Some(&c), Some(id(3)), Dir::Previous);
        assert_eq!(
            back.map(|m| m.id),
            Some(id(2)),
            "US3 s2: [ goes to the previous comment"
        );
    }

    #[test]
    fn a_comment_not_in_the_diff_opens_the_drawer() {
        let order = abc();
        let outdated = comment_order(&[fact(1, "b.rs", 4, 1, false)], &order);
        assert_eq!(
            step_comment(&outdated, &order, Some(&path("a.rs")), None, Dir::Next),
            Some(CommentMove {
                id: id(1),
                select: Some(path("b.rs")),
                show: Show::Drawer
            }),
            "US3 s3: its file is selected and the drawer opens on it"
        );
        let unlisted = comment_order(&[fact(2, "gone.rs", 4, 1, true)], &order);
        assert_eq!(
            step_comment(&unlisted, &order, Some(&path("a.rs")), None, Dir::Next),
            Some(CommentMove {
                id: id(2),
                select: None,
                show: Show::Drawer
            }),
            "a comment of an unlisted file has no file to select: the drawer"
        );
    }

    #[test]
    fn no_comments_or_a_gone_last_comment() {
        let order = abc();
        assert_eq!(step_comment(&[], &order, None, None, Dir::Next), None);
        let comments = three_comments();
        assert_eq!(
            step_comment(&comments, &order, None, Some(id(99)), Dir::Next).map(|m| m.id),
            Some(id(1)),
            "a last comment no longer present counts as none: ] takes the first"
        );
        assert_eq!(
            step_comment(&comments, &order, None, Some(id(99)), Dir::Previous).map(|m| m.id),
            Some(id(3)),
            "and [ the last"
        );
    }

    // --- hunk_change_range, comment_target (T030) ---

    fn hunk(lines: &[(LineKind, Option<u32>, Option<u32>)]) -> Hunk {
        Hunk {
            old_start: 1,
            old_len: 0,
            new_start: 1,
            new_len: 0,
            section: String::new(),
            lines: lines
                .iter()
                .map(|&(kind, old, new)| DiffLine {
                    kind,
                    old,
                    new,
                    text: String::new(),
                })
                .collect(),
            old_no_newline: false,
            new_no_newline: false,
        }
    }

    fn range(start: u32, end: u32) -> LineRange {
        LineRange::new(start, end).expect("a valid range")
    }

    fn mixed() -> Hunk {
        hunk(&[
            (LineKind::Context, Some(9), Some(9)),
            (LineKind::Removed, Some(10), None),
            (LineKind::Added, None, Some(10)),
            (LineKind::Context, Some(11), Some(11)),
            (LineKind::Added, None, Some(12)),
            (LineKind::Context, Some(12), Some(13)),
        ])
    }

    #[test]
    fn a_hunks_change_range_is_its_added_lines_else_its_removed_lines() {
        assert_eq!(
            hunk_change_range(&mixed()),
            Some((Side::New, range(10, 12))),
            "first to last added line, new side"
        );
        let removes = hunk(&[
            (LineKind::Context, Some(4), Some(4)),
            (LineKind::Removed, Some(5), None),
            (LineKind::Removed, Some(6), None),
        ]);
        assert_eq!(
            hunk_change_range(&removes),
            Some((Side::Old, range(5, 6))),
            "a remove-only hunk: first to last removed line, base side"
        );
        let context = hunk(&[(LineKind::Context, Some(1), Some(1))]);
        assert_eq!(hunk_change_range(&context), None, "nothing changed");
    }

    #[test]
    fn c_prefers_an_open_composer_then_picked_lines_then_the_current_hunk() {
        let h = mixed();
        assert_eq!(
            comment_target(true, true, Some(&h)),
            CommentOpen::FocusComposer,
            "FR-009: an open composer keeps its text and is focused"
        );
        assert_eq!(
            comment_target(false, true, Some(&h)),
            CommentOpen::Picked,
            "US3 s4: on the picked lines"
        );
        assert_eq!(
            comment_target(false, false, Some(&h)),
            CommentOpen::Hunk {
                side: Side::New,
                range: range(10, 12)
            },
            "US3 s5: on the current hunk's changed lines"
        );
        assert_eq!(
            comment_target(false, false, None),
            CommentOpen::Refused,
            "US3 s6: nothing to comment on"
        );
        let context = hunk(&[(LineKind::Context, Some(1), Some(1))]);
        assert_eq!(
            comment_target(false, false, Some(&context)),
            CommentOpen::Refused,
            "a current hunk with no changed line gives nothing to pick"
        );
        assert_eq!(PICK_OR_HUNK, "Pick lines or move to a hunk (n) to comment");
    }

    // --- timing (T032) ---

    #[test]
    fn every_rule_together_is_fast_on_a_large_review() {
        let order: Vec<NavFile> = (0..200)
            .map(|i| NavFile {
                path: path(&format!("src/m{i:03}.rs")),
                has_hunks: i % 3 != 0,
            })
            .collect();
        let facts: Vec<CommentFact> = (0..500u32)
            .map(|i| {
                let file = i * 7 % 200;
                fact(
                    u128::from(i),
                    &format!("src/m{file:03}.rs"),
                    i % 90 + 1,
                    1,
                    true,
                )
            })
            .collect();
        let selected = path("src/m199.rs");
        let started = Instant::now();
        let file = step_file(&order, Some(&selected), Dir::Next);
        let hunk = step_hunk(
            &order,
            Some(&selected),
            SelectedDiff::Hunks(2),
            Some(1),
            Dir::Next,
        );
        let ordered = comment_order(&facts, &order);
        let comment = step_comment(&ordered, &order, Some(&selected), Some(id(250)), Dir::Next);
        let next = next_unreviewed(&order, Some(&selected), |p| p.as_str() != "src/m100.rs");
        let elapsed = started.elapsed();
        assert!(file.is_some() && hunk != HunkMove::Nothing && comment.is_some());
        assert_eq!(next, Unreviewed::Select(path("src/m100.rs")));
        assert!(
            elapsed < Duration::from_millis(50),
            "SC-003: one press of each rule on 200 files and 500 comments took {elapsed:?}"
        );
    }
}
