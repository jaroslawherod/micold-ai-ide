//! What changed since the list on screen (feature 1275, FR-001–FR-004, data-model §6): the set of
//! files whose change differs between the shown list and the latest read, and the hold that keeps
//! the shown list still while the latest read waits to be applied. Pure: nothing here reads git.

use std::collections::{BTreeMap, BTreeSet};

use super::changes::{ChangeKind, ChangeList, ChangedFile};
use super::pr_diff::PrDiff;
use super::RelPath;

/// The files whose change differs between `shown` and `latest` (FR-001), matched by path: a path
/// in only one of them, or in both with a different kind, added or removed count, or change
/// identity. A latest rename counts once, under its new path: its old path is not counted again.
pub fn changed_since(shown: &[ChangedFile], latest: &[ChangedFile]) -> BTreeSet<RelPath> {
    changed_between(shown.iter(), latest.iter())
}

fn changed_between<'a>(
    shown: impl Iterator<Item = &'a ChangedFile>,
    latest: impl Iterator<Item = &'a ChangedFile>,
) -> BTreeSet<RelPath> {
    let mut shown: BTreeMap<&RelPath, &ChangedFile> = shown.map(|row| (&row.path, row)).collect();
    let mut changed = BTreeSet::new();
    let mut renamed_from = BTreeSet::new();
    for row in latest {
        if let ChangeKind::Renamed { from } = &row.kind {
            renamed_from.insert(from.clone());
        }
        match shown.remove(&row.path) {
            Some(before) if same_change(before, row) => {}
            _ => {
                changed.insert(row.path.clone());
            }
        }
    }
    changed.extend(
        shown
            .into_keys()
            .filter(|path| !renamed_from.contains(*path))
            .cloned(),
    );
    changed
}

/// Two rows of one path show the same change: `content`, `large` and `origin` do not count.
fn same_change(a: &ChangedFile, b: &ChangedFile) -> bool {
    a.kind == b.kind && a.added == b.added && a.removed == b.removed && a.identity == b.identity
}

/// A list the hold works on: the working tree's [`ChangeList`] or a pull request's [`PrDiff`].
pub trait Listing: Clone {
    /// The rows, in the listing's order.
    fn rows(&self) -> Vec<&ChangedFile>;

    /// Make `path`'s entry equal `from`'s (with whatever goes with the row), or remove it when
    /// `from` lacks it, keeping this listing's order.
    fn put(&mut self, path: &RelPath, from: &Self);
}

impl Listing for ChangeList {
    fn rows(&self) -> Vec<&ChangedFile> {
        self.files.iter().collect()
    }

    /// Path order, as the list read sorts it.
    fn put(&mut self, path: &RelPath, from: &Self) {
        self.files.retain(|row| &row.path != path);
        if let Some(row) = from.files.iter().find(|row| &row.path == path) {
            let at = self.files.partition_point(|other| other.path < row.path);
            self.files.insert(at, row.clone());
        }
    }
}

impl Listing for PrDiff {
    fn rows(&self) -> Vec<&ChangedFile> {
        self.files.iter().map(|file| &file.file).collect()
    }

    /// The patch comes with the row; GitHub's order is kept: a path new here goes after the last
    /// file that precedes it in `from` and is listed here.
    fn put(&mut self, path: &RelPath, from: &Self) {
        let theirs = from.files.iter().position(|file| &file.file.path == path);
        let ours = self.files.iter().position(|file| &file.file.path == path);
        match (ours, theirs) {
            (Some(at), Some(theirs)) => self.files[at] = from.files[theirs].clone(),
            (Some(at), None) => {
                self.files.remove(at);
            }
            (None, Some(theirs)) => {
                let at = from.files[..theirs]
                    .iter()
                    .rev()
                    .find_map(|before| {
                        self.files
                            .iter()
                            .position(|file| file.file.path == before.file.path)
                    })
                    .map_or(0, |at| at + 1);
                self.files.insert(at, from.files[theirs].clone());
            }
            (None, None) => {}
        }
    }
}

/// A latest read held back from the screen, and the files it changes (FR-001, FR-002). Exists
/// only while that set is non-empty.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Held<L: Listing> {
    latest: L,
    changed: BTreeSet<RelPath>,
}

/// What a read means for the list on screen.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Observed<L: Listing> {
    /// Nothing differs: the caller may show the read.
    Same(L),
    /// Something differs: the read is held.
    Held(Held<L>),
}

/// Compare a read with the list on screen (FR-001).
pub fn observe<L: Listing>(shown: &L, latest: L) -> Observed<L> {
    let changed = changed_between(shown.rows().into_iter(), latest.rows().into_iter());
    if changed.is_empty() {
        Observed::Same(latest)
    } else {
        Observed::Held(Held { latest, changed })
    }
}

impl<L: Listing> Held<L> {
    /// How many files changed since the shown list (FR-002): each counts once.
    pub fn count(&self) -> usize {
        self.changed.len()
    }

    /// Whether `path` is in the changed set (its row carries the dot).
    pub fn contains(&self, path: &RelPath) -> bool {
        self.changed.contains(path)
    }

    /// Whether the latest read lists `path`.
    pub fn latest_lists(&self, path: &RelPath) -> bool {
        self.latest.rows().iter().any(|row| &row.path == path)
    }

    /// The latest read.
    pub fn latest(&self) -> &L {
        &self.latest
    }

    /// A newer read replaces the held one and the set is computed again (FR-003): a file whose
    /// row equals the shown one again leaves it.
    pub fn observe(self, shown: &L, latest: L) -> Observed<L> {
        observe(shown, latest)
    }

    /// The user opened `path` and `fresh` is a read of it (FR-009). When `fresh` lists it, the
    /// shown and the latest list both take that row and `path` leaves the set (US1 AS10); when
    /// not, only the latest list drops it and it stays held (AS11). A fresh rename also takes its
    /// old path from both lists. The set is then computed again, so it stays the difference of
    /// the two lists. `None` once the set is empty.
    pub fn adopt(mut self, shown: &mut L, path: &RelPath, fresh: &L) -> Option<Self> {
        let kind = fresh
            .rows()
            .into_iter()
            .find(|row| &row.path == path)
            .map(|row| row.kind.clone());
        self.latest.put(path, fresh);
        if let Some(kind) = kind {
            shown.put(path, fresh);
            if let ChangeKind::Renamed { from } = kind {
                shown.put(&from, fresh);
                self.latest.put(&from, fresh);
            }
        }
        self.changed = changed_between(shown.rows().into_iter(), self.latest.rows().into_iter());
        (!self.changed.is_empty()).then_some(self)
    }

    /// Show the latest read (FR-004): the set is gone with the hold.
    pub fn apply(self, shown: &mut L) {
        *shown = self.latest;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::review::base::{Base, BaseUnavailable, ReviewScope};
    use crate::review::changes::{ChangeIdentity, Content, Origin, VersionId};
    use crate::review::pr_diff::PrFile;

    fn p(path: &str) -> RelPath {
        RelPath::from_git(path)
    }

    fn row(path: &str, added: u32, id: &str) -> ChangedFile {
        ChangedFile {
            path: p(path),
            kind: ChangeKind::Modified,
            content: Content::Text,
            added,
            removed: 0,
            large: false,
            origin: Origin::Uncommitted,
            identity: ChangeIdentity::Sides {
                old: VersionId::Object("old".into()),
                new: VersionId::Object(id.into()),
                modes: [0o100644, 0o100644],
            },
        }
    }

    fn list(rows: Vec<ChangedFile>) -> ChangeList {
        ChangeList {
            files: rows,
            scope: ReviewScope::Worktree {
                base: Base::Unavailable(BaseUnavailable::NoDefaultBranch),
            },
        }
    }

    fn set(paths: &[&str]) -> BTreeSet<RelPath> {
        paths.iter().map(|path| p(path)).collect()
    }

    fn held(shown: &ChangeList, latest: ChangeList) -> Held<ChangeList> {
        match observe(shown, latest) {
            Observed::Held(held) => held,
            Observed::Same(_) => panic!("a difference was expected"),
        }
    }

    #[test]
    fn s1_equal_lists_change_nothing() {
        let shown = list(vec![row("a", 1, "x"), row("b", 2, "y")]);
        assert!(changed_since(&shown.files, &shown.files).is_empty());
        assert_eq!(
            observe(&shown, shown.clone()),
            Observed::Same(shown.clone())
        );
    }

    #[test]
    fn s2_kind_counts_and_identity_count_but_content_large_and_origin_do_not() {
        let base = row("a", 1, "x");
        let differs = |change: &dyn Fn(&mut ChangedFile)| {
            let mut latest = base.clone();
            change(&mut latest);
            changed_since(std::slice::from_ref(&base), &[latest]) == set(&["a"])
        };
        assert!(differs(&|r| r.kind = ChangeKind::Added));
        assert!(differs(&|r| r.added = 2));
        assert!(differs(&|r| r.removed = 1));
        assert!(differs(&|r| r.identity = ChangeIdentity::Unknown));
        assert!(!differs(&|r| r.content = Content::Binary));
        assert!(!differs(&|r| r.large = true));
        assert!(!differs(&|r| r.origin = Origin::Both));
    }

    #[test]
    fn s3_a_new_path_and_a_gone_path_count() {
        let shown = [row("a", 1, "x"), row("gone", 1, "g")];
        let latest = [row("a", 1, "x"), row("new", 1, "n")];
        assert_eq!(changed_since(&shown, &latest), set(&["gone", "new"]));
    }

    #[test]
    fn s4_a_rename_counts_once_under_its_new_path() {
        let mut renamed = row("y", 1, "x");
        renamed.kind = ChangeKind::Renamed { from: p("x") };
        let shown = [row("x", 1, "x")];
        assert_eq!(changed_since(&shown, &[renamed.clone()]), set(&["y"]));

        // Y listed before too: it counts by its row; X is the rename's `from`, so not again.
        let shown = [row("x", 1, "x"), row("y", 1, "old")];
        assert_eq!(changed_since(&shown, &[renamed.clone()]), set(&["y"]));
        // A shown path that is no rename's `from` still counts when it goes.
        let shown = [row("x", 1, "x"), row("z", 1, "z")];
        assert_eq!(changed_since(&shown, &[renamed]), set(&["y", "z"]));
    }

    #[test]
    fn s5_the_count_is_the_set_size_and_a_file_counts_once() {
        let shown = list(vec![row("a", 1, "x"), row("b", 1, "y")]);
        let first = held(&shown, list(vec![row("a", 2, "x2"), row("b", 3, "y2")]));
        assert_eq!(first.count(), 2);
        let again = first.observe(&shown, list(vec![row("a", 4, "x3"), row("b", 3, "y2")]));
        let Observed::Held(again) = again else {
            panic!("still held")
        };
        assert_eq!(again.count(), 2, "a changed again, still one file (AS3)");
        assert!(again.contains(&p("a")) && again.contains(&p("b")));
    }

    #[test]
    fn s6_a_row_equal_again_leaves_the_set_and_an_empty_set_is_same() {
        let shown = list(vec![row("a", 1, "x"), row("b", 1, "y")]);
        let first = held(&shown, list(vec![row("a", 2, "x2"), row("b", 2, "y2")]));
        let Observed::Held(back) =
            first.observe(&shown, list(vec![row("a", 1, "x"), row("b", 2, "y2")]))
        else {
            panic!("b still differs")
        };
        assert_eq!(back.count(), 1);
        assert!(!back.contains(&p("a")), "a is back to what is shown (AS4)");
        assert_eq!(
            back.observe(&shown, shown.clone()),
            Observed::Same(shown.clone())
        );
    }

    #[test]
    fn s7_apply_shows_the_latest_read() {
        let mut shown = list(vec![row("a", 1, "x")]);
        let latest = list(vec![row("a", 2, "x2"), row("b", 1, "y")]);
        held(&shown, latest.clone()).apply(&mut shown);
        assert_eq!(shown, latest);
    }

    #[test]
    fn s8_adopting_a_listed_file_takes_its_fresh_row_and_leaves_the_rest_held() {
        let mut shown = list(vec![row("a", 1, "x"), row("b", 1, "y")]);
        let hold = held(&shown, list(vec![row("a", 2, "x2"), row("b", 2, "y2")]));
        let fresh = list(vec![row("a", 3, "x3")]);
        let hold = hold
            .adopt(&mut shown, &p("a"), &fresh)
            .expect("b still held");
        assert_eq!(
            shown.files[0], fresh.files[0],
            "shown takes the fresh row (AS10)"
        );
        assert_eq!(hold.latest().files[0], fresh.files[0], "so does the latest");
        assert_eq!(shown.files[1], row("b", 1, "y"), "b stays as shown");
        assert_eq!(hold.latest().files[1], row("b", 2, "y2"));
        assert!(!hold.contains(&p("a")));
        assert!(hold.contains(&p("b")));
        assert_eq!(hold.count(), 1);
        assert!(
            hold.adopt(&mut shown, &p("b"), &list(vec![row("b", 2, "y2")]))
                .is_none(),
            "the last file adopted ends the hold"
        );
    }

    #[test]
    fn s9_adopting_a_file_no_longer_listed_keeps_it_shown_and_held() {
        let mut shown = list(vec![row("a", 1, "x")]);
        let before = shown.clone();
        let hold = held(&shown, list(vec![row("a", 2, "x2")]));
        let hold = hold
            .adopt(&mut shown, &p("a"), &list(Vec::new()))
            .expect("a stays held (AS11)");
        assert_eq!(shown, before, "its row stays on screen");
        assert!(!hold.latest_lists(&p("a")), "the latest read drops it");
        assert!(hold.contains(&p("a")));
        assert_eq!(hold.count(), 1);
    }

    /// The set is always what differs between the shown list and the latest read.
    fn assert_consistent(hold: &Held<ChangeList>, shown: &ChangeList) {
        assert_eq!(
            hold.changed,
            changed_since(&shown.files, &hold.latest().files),
            "the set is the difference of shown and latest"
        );
    }

    #[test]
    fn s11_adopting_a_rename_drops_its_old_path_from_the_shown_list() {
        let mut shown = list(vec![row("x", 1, "x"), row("z", 1, "z")]);
        let mut renamed = row("y", 1, "x");
        renamed.kind = ChangeKind::Renamed { from: p("x") };
        let hold = held(&shown, list(vec![renamed.clone(), row("z", 2, "z2")]));
        let hold = hold
            .adopt(&mut shown, &p("y"), &list(vec![renamed.clone()]))
            .expect("z still held");
        assert_eq!(order(&shown), ["y", "z"], "x is not listed twice");
        assert_eq!(order(hold.latest()), ["y", "z"]);
        assert_eq!(hold.count(), 1);
        assert_consistent(&hold, &shown);

        let mut shown = list(vec![row("x", 1, "x")]);
        let hold = held(&shown, list(vec![renamed.clone()]));
        assert!(hold
            .adopt(&mut shown, &p("y"), &list(vec![renamed.clone()]))
            .is_none());
        assert_eq!(shown, list(vec![renamed]), "shown == {{y}}");
    }

    #[test]
    fn s12_adopting_an_undotted_file_no_longer_listed_dots_it() {
        let mut shown = list(vec![row("a", 1, "x"), row("b", 1, "y")]);
        let hold = held(&shown, list(vec![row("a", 2, "x2"), row("b", 1, "y")]));
        assert!(!hold.contains(&p("b")));
        let hold = hold
            .adopt(&mut shown, &p("b"), &list(Vec::new()))
            .expect("still held");
        assert!(hold.contains(&p("b")), "shown lists b, the latest does not");
        assert_eq!(hold.count(), 2);
        assert_consistent(&hold, &shown);
    }

    fn pr(files: &[(&str, &str)]) -> PrDiff {
        PrDiff {
            files: files
                .iter()
                .map(|(path, patch)| PrFile {
                    file: row(path, 1, patch),
                    patch: patch.as_bytes().to_vec(),
                })
                .collect(),
        }
    }

    fn order<L: Listing>(listing: &L) -> Vec<String> {
        listing
            .rows()
            .iter()
            .map(|row| row.path.as_str().to_owned())
            .collect()
    }

    #[test]
    fn s10_a_pull_requests_put_copies_the_patch_in_githubs_order_and_a_list_keeps_path_order() {
        let mut shown = pr(&[("z", "z1"), ("a", "a1")]);
        let fresh = pr(&[("z", "z2"), ("m", "m2"), ("a", "a2")]);
        shown.put(&p("a"), &fresh);
        assert_eq!(shown.files[1].patch, b"a2", "the patch comes with the row");
        shown.put(&p("m"), &fresh);
        assert_eq!(
            order(&shown),
            ["z", "m", "a"],
            "GitHub's order, not path order"
        );
        assert_eq!(shown.files[1].patch, b"m2");
        shown.put(&p("z"), &pr(&[]));
        assert_eq!(order(&shown), ["m", "a"], "absent from `from`: removed");

        let mut shown = list(vec![row("a", 1, "x"), row("z", 1, "z")]);
        shown.put(&p("m"), &list(vec![row("m", 1, "m")]));
        assert_eq!(order(&shown), ["a", "m", "z"], "path order");
    }
}
