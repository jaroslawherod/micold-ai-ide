//! Reviewed marks (feature 1271, research R1–R7): a file's change id, the whole working-tree change
//! it is read from whatever the Committed / Uncommitted toggles show, the marks one entry keeps
//! per file and per source, and the review state and *n of m* progress derived from them.
//!
//! Everything here is pure: [`crate::git::GitCli::whole_change`] runs git and hands its output to
//! [`WholeChange::assemble`]; deciding a file's state and counting progress never touch git or the
//! view (FR-014).

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use serde::{Deserialize, Serialize};
use sha1::{Digest, Sha1};

use super::changes::{ChangeKind, ChangedFile, NameStatus, Origin};
use super::comment::Scope;
use super::RelPath;
use crate::task_progress::Ratio;

/// A file's change's identity (research R1): the SHA-1 of a canonical record of the change,
/// written as 40 lowercase hex characters in JSON and on the wire.
///
/// Two working-tree changes have the same id exactly when kind, path, both modes and both sides'
/// object ids are the same, so it holds for binary and large files whose lines are never read.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct ChangeId([u8; 20]);

impl ChangeId {
    /// The id of a working-tree file's change from its `git diff --raw --no-abbrev` record, the
    /// new id already filled in where git printed zeros (research R2 step 3). Digests `w`, the
    /// kind tag, the path, the old mode and id and the new mode and id, separated by `\0`.
    pub fn of_worktree(record: &NameStatus) -> Self {
        let mode_only = record.mode_changed && record.old_id == record.new_id;
        let tag = match &record.kind {
            ChangeKind::Modified if mode_only => kind_tag(&ChangeKind::ModeOnly),
            kind => kind_tag(kind),
        };
        Self::digest(&[
            b"w",
            tag.as_bytes(),
            record.path.as_str().as_bytes(),
            record.old_mode.as_bytes(),
            record.old_id.as_bytes(),
            record.new_mode.as_bytes(),
            record.new_id.as_bytes(),
        ])
    }

    /// The id of a pull request file's change: `p`, the kind tag, the path and the file's own
    /// section of the pull request's diff.
    pub fn of_patch(file: &ChangedFile, patch: &[u8]) -> Self {
        Self::digest(&[
            b"p",
            kind_tag(&file.kind).as_bytes(),
            file.path.as_str().as_bytes(),
            patch,
        ])
    }

    /// Exactly 40 hex characters, either case (stored lower-cased); anything else is `None`.
    pub fn parse(text: &str) -> Option<Self> {
        let bytes = text.as_bytes();
        if bytes.len() != 40 {
            return None;
        }
        let mut id = [0u8; 20];
        for (byte, pair) in id.iter_mut().zip(bytes.as_chunks::<2>().0) {
            let pair = std::str::from_utf8(pair.as_slice()).ok()?;
            if !pair.bytes().all(|b| b.is_ascii_hexdigit()) {
                return None;
            }
            *byte = u8::from_str_radix(pair, 16).ok()?;
        }
        Some(Self(id))
    }

    fn digest(fields: &[&[u8]]) -> Self {
        let mut hasher = Sha1::new();
        for (i, field) in fields.iter().enumerate() {
            if i > 0 {
                hasher.update([0u8]);
            }
            hasher.update(field);
        }
        Self(hasher.finalize().into())
    }
}

/// The record's kind tag: `A` added (an untracked file too), `M` modified, `D` deleted, `R` and
/// the old path for a rename, `T` for a mode-only change.
fn kind_tag(kind: &ChangeKind) -> String {
    match kind {
        ChangeKind::Added | ChangeKind::Untracked => "A".to_owned(),
        ChangeKind::Modified => "M".to_owned(),
        ChangeKind::Deleted => "D".to_owned(),
        ChangeKind::Renamed { from } => format!("R{from}"),
        ChangeKind::ModeOnly => "T".to_owned(),
    }
}

impl fmt::Display for ChangeId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for byte in self.0 {
            write!(f, "{byte:02x}")?;
        }
        Ok(())
    }
}

impl fmt::Debug for ChangeId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ChangeId({self})")
    }
}

impl TryFrom<String> for ChangeId {
    type Error = String;
    fn try_from(text: String) -> Result<Self, Self::Error> {
        Self::parse(&text).ok_or_else(|| format!("not a change id (40 hex characters): {text:?}"))
    }
}

impl From<ChangeId> for String {
    fn from(id: ChangeId) -> Self {
        id.to_string()
    }
}

/// One file of an entry's whole working-tree change (research R2).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WholeFile {
    /// The path now (the new path of a rename).
    pub path: RelPath,
    /// How it changed, as [`ChangedFile::kind`]; an untracked file is `Added` (research R1).
    pub kind: ChangeKind,
    /// Committed, uncommitted or both; uncommitted for an entry without a base.
    pub origin: Origin,
    /// `None` when a side's object id could not be read (R2 step 3): the file cannot be marked.
    pub id: Option<ChangeId>,
}

/// An entry's whole working-tree change, whatever the toggles show (FR-005): one file per path.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct WholeChange {
    /// The files, by path.
    pub files: BTreeMap<RelPath, WholeFile>,
}

impl WholeChange {
    /// The whole change from git's records (contracts/marks-wire.md § G2).
    ///
    /// - `names`: `git diff --raw --no-abbrev` over the base (or `HEAD`) to the working tree;
    /// - `committed`: the paths the commits since the base change, `None` for an entry without a
    ///   base (every file is then Uncommitted); `uncommitted`: those `HEAD` → working tree changes;
    /// - `hashed`: object ids from `hash-object`, for the new sides git printed as zeros and the
    ///   untracked files; a file whose id is missing gets `id: None`;
    /// - `untracked`: untracked, not-ignored files with their mode on disk. Each is recorded as
    ///   git's added file (old mode and id all zeros), so staging or committing it unaltered keeps
    ///   its id (research R1).
    pub fn assemble(
        names: Vec<NameStatus>,
        committed: Option<&BTreeSet<RelPath>>,
        uncommitted: &BTreeSet<RelPath>,
        hashed: &BTreeMap<RelPath, String>,
        untracked: Vec<(RelPath, String)>,
    ) -> Self {
        let mut files = BTreeMap::new();
        for mut name in names {
            let origin = match committed {
                None => Origin::Uncommitted,
                Some(committed) => match (
                    committed.contains(&name.path),
                    uncommitted.contains(&name.path),
                ) {
                    (true, false) => Origin::Committed,
                    (false, true) => Origin::Uncommitted,
                    _ => Origin::Both,
                },
            };
            let unhashed = is_zeros(&name.new_id) && !is_zeros(&name.new_mode);
            let known = if unhashed {
                match hashed.get(&name.path) {
                    Some(id) => {
                        name.new_id = id.clone();
                        true
                    }
                    None => false,
                }
            } else {
                true
            };
            files.insert(name.path.clone(), whole_file(name, origin, known));
        }
        for (path, mode) in untracked {
            if files.contains_key(&path) {
                continue;
            }
            let hash = hashed.get(&path);
            let zeros = "0".repeat(hash.map_or(40, String::len));
            let name = NameStatus {
                path: path.clone(),
                kind: ChangeKind::Added,
                mode_changed: false,
                old_mode: "000000".to_owned(),
                new_mode: mode,
                old_id: zeros,
                new_id: hash.cloned().unwrap_or_default(),
            };
            files.insert(path, whole_file(name, Origin::Uncommitted, hash.is_some()));
        }
        Self { files }
    }
}

/// `name` as a [`WholeFile`] of `origin`, with its id when both sides' ids are `known`.
fn whole_file(name: NameStatus, origin: Origin, known: bool) -> WholeFile {
    let id = known.then(|| ChangeId::of_worktree(&name));
    let kind = if name.kind == ChangeKind::Modified
        && name.mode_changed
        && known
        && name.old_id == name.new_id
    {
        ChangeKind::ModeOnly
    } else {
        name.kind
    };
    WholeFile {
        path: name.path,
        kind,
        origin,
        id,
    }
}

/// A mode or object id git spells as all zeros: the side does not exist, or was not hashed.
fn is_zeros(text: &str) -> bool {
    !text.is_empty() && text.bytes().all(|b| b == b'0')
}

/// One file marked reviewed, in one source, on one change (data-model § `ReviewMark`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "RawMark", into = "RawMark")]
pub struct ReviewMark {
    /// The source: the working tree or pull request `n > 0`.
    pub scope: Scope,
    /// The file, worktree-relative and `/`-separated.
    pub path: RelPath,
    /// The file's change the mark was made on.
    pub change: ChangeId,
}

#[derive(Serialize, Deserialize)]
struct RawMark {
    scope: Scope,
    path: RelPath,
    change: ChangeId,
}

impl TryFrom<RawMark> for ReviewMark {
    type Error = String;
    fn try_from(raw: RawMark) -> Result<Self, Self::Error> {
        if raw.scope == Scope::PullRequest(0) {
            return Err("a mark of pull request 0".to_owned());
        }
        Ok(Self {
            scope: raw.scope,
            path: raw.path,
            change: raw.change,
        })
    }
}

impl From<ReviewMark> for RawMark {
    fn from(mark: ReviewMark) -> Self {
        Self {
            scope: mark.scope,
            path: mark.path,
            change: mark.change,
        }
    }
}

/// Every mark of one entry: at most one per file per source (FR-006). Nothing but [`Self::set`]
/// removes one (FR-016): a file leaving the list, a differing change or a closed pull request
/// leave it in place.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct EntryMarks(BTreeMap<(Scope, RelPath), ChangeId>);

impl EntryMarks {
    /// Mark `path` in `scope` reviewed on `change` (replacing an earlier mark), or unmark it with
    /// `None`. `false` when nothing changed: the same mark again, or an unmark of no mark.
    pub fn set(&mut self, scope: Scope, path: RelPath, change: Option<ChangeId>) -> bool {
        let key = (scope, path);
        match change {
            Some(change) => self.0.insert(key, change) != Some(change),
            None => self.0.remove(&key).is_some(),
        }
    }

    /// The mark of `path` in `scope`.
    pub fn get(&self, scope: Scope, path: &RelPath) -> Option<&ChangeId> {
        self.0.get(&(scope, path.clone()))
    }

    /// The marks of `scope`, by path: the shown source's.
    pub fn of_scope(&self, scope: Scope) -> impl Iterator<Item = (&RelPath, &ChangeId)> {
        self.0
            .iter()
            .filter(move |((s, _), _)| *s == scope)
            .map(|((_, path), change)| (path, change))
    }

    /// Every mark, by scope then path: the wire and store order.
    pub fn to_vec(&self) -> Vec<ReviewMark> {
        self.0
            .iter()
            .map(|((scope, path), change)| ReviewMark {
                scope: *scope,
                path: path.clone(),
                change: *change,
            })
            .collect()
    }

    /// The marks of `marks`; of two for the same file and source, the last.
    pub fn from_vec(marks: Vec<ReviewMark>) -> Self {
        Self(
            marks
                .into_iter()
                .map(|mark| ((mark.scope, mark.path), mark.change))
                .collect(),
        )
    }

    /// No marks.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

/// A listed file's review state (FR-001), derived and never stored.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReviewState {
    /// Not marked, or marked on another change, and no drafts.
    Unseen,
    /// Not reviewed, with draft comments.
    HasDrafts,
    /// Marked on the change it has now; wins over drafts (D2).
    Reviewed,
}

/// A file's state, and whether it changed since it was marked (FR-003).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FileReview {
    /// Its state.
    pub state: ReviewState,
    /// Marked, but on a change other than the one it has now.
    pub changed_since: bool,
}

/// The state of a file with `mark`, the `current` id of its change (`None`: not read) and
/// `drafts` draft comments (data-model § `file_review`).
pub fn file_review(
    mark: Option<&ChangeId>,
    current: Option<&ChangeId>,
    drafts: usize,
) -> FileReview {
    let unreviewed = if drafts > 0 {
        ReviewState::HasDrafts
    } else {
        ReviewState::Unseen
    };
    match (mark, current) {
        (Some(mark), Some(current)) if mark == current => FileReview {
            state: ReviewState::Reviewed,
            changed_since: false,
        },
        (Some(_), Some(_)) => FileReview {
            state: unreviewed,
            changed_since: true,
        },
        _ => FileReview {
            state: unreviewed,
            changed_since: false,
        },
    }
}

/// *n of m* (FR-008, FR-009): `done` the Reviewed files, `total` all of `files`; `None` for none.
pub fn progress(files: impl IntoIterator<Item = FileReview>) -> Option<Ratio> {
    let (done, total) = files.into_iter().fold((0u32, 0u32), |(done, total), file| {
        (
            done + u32::from(file.state == ReviewState::Reviewed),
            total + 1,
        )
    });
    (total > 0).then_some(Ratio { done, total })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::review::changes::Content;

    fn p(path: &str) -> RelPath {
        RelPath::from_git(path)
    }

    const OLD: &str = "422c2b7ab3b3c668038da977e4e93a5fc623169c";
    const NEW: &str = "8ba3a16384aacc37d01564b28401755ce8053f51";
    const ZEROS: &str = "0000000000000000000000000000000000000000";

    fn record(path: &str, kind: ChangeKind, old_id: &str, new_id: &str) -> NameStatus {
        NameStatus {
            path: p(path),
            kind,
            mode_changed: false,
            old_mode: "100644".into(),
            new_mode: "100644".into(),
            old_id: old_id.into(),
            new_id: new_id.into(),
        }
    }

    fn modified() -> NameStatus {
        record("src/a.rs", ChangeKind::Modified, OLD, NEW)
    }

    fn id(n: u8) -> ChangeId {
        ChangeId([n; 20])
    }

    // ---- ChangeId (T002) ------------------------------------------------------------------------

    #[test]
    fn equal_records_give_equal_ids() {
        assert_eq!(
            ChangeId::of_worktree(&modified()),
            ChangeId::of_worktree(&modified())
        );
    }

    #[test]
    fn changing_any_one_field_of_the_record_changes_the_id() {
        let base = ChangeId::of_worktree(&modified());
        let renamed = |from: &str| ChangeKind::Renamed { from: p(from) };
        let variants: Vec<(&str, NameStatus)> = vec![
            (
                "kind",
                NameStatus {
                    kind: ChangeKind::Added,
                    ..modified()
                },
            ),
            (
                "path",
                NameStatus {
                    path: p("src/b.rs"),
                    ..modified()
                },
            ),
            (
                "old mode",
                NameStatus {
                    old_mode: "100755".into(),
                    ..modified()
                },
            ),
            (
                "new mode",
                NameStatus {
                    new_mode: "100755".into(),
                    ..modified()
                },
            ),
            (
                "old id",
                NameStatus {
                    old_id: NEW.into(),
                    ..modified()
                },
            ),
            (
                "new id",
                NameStatus {
                    new_id: OLD.into(),
                    ..modified()
                },
            ),
        ];
        for (field, variant) in variants {
            assert_ne!(
                ChangeId::of_worktree(&variant),
                base,
                "a different {field} is a different change"
            );
        }
        let from_x = NameStatus {
            kind: renamed("x.rs"),
            ..modified()
        };
        let from_y = NameStatus {
            kind: renamed("y.rs"),
            ..modified()
        };
        assert_ne!(
            ChangeId::of_worktree(&from_x),
            ChangeId::of_worktree(&from_y),
            "a rename from another path is another change"
        );
    }

    #[test]
    fn a_mode_only_change_has_its_own_tag() {
        let mode = NameStatus {
            mode_changed: true,
            new_mode: "100755".into(),
            new_id: OLD.into(),
            ..modified()
        };
        let as_mode_only = NameStatus {
            kind: ChangeKind::ModeOnly,
            ..mode.clone()
        };
        assert_eq!(
            ChangeId::of_worktree(&mode),
            ChangeId::of_worktree(&as_mode_only),
            "a modified record with equal ids and different modes is digested as mode only"
        );
    }

    fn pr_file() -> ChangedFile {
        ChangedFile {
            path: p("src/a.rs"),
            kind: ChangeKind::Modified,
            content: Content::Text,
            added: 1,
            removed: 1,
            large: false,
            origin: Origin::Committed,
        }
    }

    #[test]
    fn a_patch_id_follows_the_patch_bytes() {
        let patch = b"@@ -1 +1 @@\n-a\n+b\n";
        assert_eq!(
            ChangeId::of_patch(&pr_file(), patch),
            ChangeId::of_patch(&pr_file(), patch)
        );
        assert_ne!(
            ChangeId::of_patch(&pr_file(), patch),
            ChangeId::of_patch(&pr_file(), b"@@ -1 +1 @@\n-a\n+c\n"),
            "any byte of the patch changes the id"
        );
    }

    #[test]
    fn a_worktree_id_and_a_patch_id_never_meet() {
        let record = modified();
        let fields = ["M", "src/a.rs", "100644", OLD, "100644", NEW].join("\0");
        assert_ne!(
            ChangeId::of_worktree(&record),
            ChangeId::of_patch(&pr_file(), fields.as_bytes()),
            "the source tags `w` and `p` keep the two derivations apart"
        );
    }

    #[test]
    fn parse_takes_exactly_forty_hex_characters_and_lower_cases_them() {
        let upper = "ABCDEF0123456789ABCDEF0123456789ABCDEF01";
        let parsed = ChangeId::parse(upper).expect("40 hex characters");
        assert_eq!(parsed.to_string(), upper.to_lowercase());
        assert_eq!(ChangeId::parse(&upper[..39]), None, "39 characters");
        assert_eq!(ChangeId::parse(&format!("{upper}0")), None, "41 characters");
        assert_eq!(ChangeId::parse("g".repeat(40).as_str()), None, "not hex");
        assert_eq!(
            ChangeId::parse(&format!("+{}", &upper[..39])),
            None,
            "a sign is not a digit"
        );
        assert_eq!(ChangeId::parse(&"é".repeat(20)), None, "not ASCII");
    }

    #[test]
    fn a_change_id_is_a_forty_hex_string_in_json_and_postcard() {
        let change = ChangeId::of_worktree(&modified());
        let json = serde_json::to_string(&change).expect("serialises");
        assert_eq!(json, format!("\"{change}\""));
        assert_eq!(change.to_string().len(), 40);
        let back: ChangeId = serde_json::from_str(&json).expect("reads back");
        assert_eq!(back, change);
        let wire = postcard::to_stdvec(&change).expect("serialises");
        let back: ChangeId = postcard::from_bytes(&wire).expect("reads back");
        assert_eq!(back, change);
        let text: String = postcard::from_bytes(&wire).expect("a string on the wire");
        assert_eq!(text, change.to_string(), "one shape: the 40-hex string");
        assert!(serde_json::from_str::<ChangeId>("\"not hex\"").is_err());
        let bad = postcard::to_stdvec(&"z".repeat(40)).expect("serialises");
        assert!(postcard::from_bytes::<ChangeId>(&bad).is_err());
    }

    // ---- EntryMarks (T004) ----------------------------------------------------------------------

    #[test]
    fn set_inserts_replaces_and_says_whether_anything_changed() {
        let mut marks = EntryMarks::default();
        assert!(marks.is_empty());
        assert!(marks.set(Scope::WorkingTree, p("a.rs"), Some(id(1))));
        assert!(!marks.is_empty());
        assert!(
            !marks.set(Scope::WorkingTree, p("a.rs"), Some(id(1))),
            "the same mark again changes nothing"
        );
        assert!(
            marks.set(Scope::WorkingTree, p("a.rs"), Some(id(2))),
            "a mark on another change replaces it"
        );
        assert_eq!(marks.get(Scope::WorkingTree, &p("a.rs")), Some(&id(2)));
        assert!(marks.set(Scope::WorkingTree, p("a.rs"), None), "unmark");
        assert_eq!(marks.get(Scope::WorkingTree, &p("a.rs")), None);
        assert!(
            !marks.set(Scope::WorkingTree, p("a.rs"), None),
            "unmarking an unmarked file changes nothing"
        );
        assert!(marks.is_empty());
    }

    #[test]
    fn sources_are_kept_apart() {
        let mut marks = EntryMarks::default();
        marks.set(Scope::WorkingTree, p("a.rs"), Some(id(1)));
        marks.set(Scope::PullRequest(12), p("a.rs"), Some(id(2)));
        marks.set(Scope::PullRequest(12), p("b.rs"), Some(id(3)));
        assert_eq!(marks.get(Scope::WorkingTree, &p("a.rs")), Some(&id(1)));
        assert_eq!(marks.get(Scope::PullRequest(12), &p("a.rs")), Some(&id(2)));
        assert_eq!(marks.get(Scope::PullRequest(13), &p("a.rs")), None);
        let pr: Vec<(&str, ChangeId)> = marks
            .of_scope(Scope::PullRequest(12))
            .map(|(path, change)| (path.as_str(), *change))
            .collect();
        assert_eq!(pr, vec![("a.rs", id(2)), ("b.rs", id(3))]);
        let wt: Vec<&str> = marks
            .of_scope(Scope::WorkingTree)
            .map(|(path, _)| path.as_str())
            .collect();
        assert_eq!(wt, vec!["a.rs"]);
    }

    fn mark(scope: Scope, path: &str, n: u8) -> ReviewMark {
        ReviewMark {
            scope,
            path: p(path),
            change: id(n),
        }
    }

    #[test]
    fn to_vec_orders_by_scope_then_path_and_from_vec_keeps_the_last_duplicate() {
        let marks = EntryMarks::from_vec(vec![
            mark(Scope::PullRequest(3), "a.rs", 1),
            mark(Scope::WorkingTree, "b.rs", 2),
            mark(Scope::WorkingTree, "a.rs", 3),
            mark(Scope::WorkingTree, "b.rs", 4),
        ]);
        assert_eq!(
            marks.to_vec(),
            vec![
                mark(Scope::WorkingTree, "a.rs", 3),
                mark(Scope::WorkingTree, "b.rs", 4),
                mark(Scope::PullRequest(3), "a.rs", 1),
            ]
        );
    }

    #[test]
    fn a_mark_of_pull_request_zero_does_not_decode() {
        let good = serde_json::json!({ "scope": { "PullRequest": 12 }, "path": "a.rs", "change": "ab".repeat(20) });
        let back: ReviewMark = serde_json::from_value(good).expect("a mark of pull request 12");
        assert_eq!(back.scope, Scope::PullRequest(12));
        let zero = serde_json::json!({ "scope": { "PullRequest": 0 }, "path": "a.rs", "change": "ab".repeat(20) });
        assert!(serde_json::from_value::<ReviewMark>(zero).is_err());
    }

    // ---- WholeChange::assemble (T008) -----------------------------------------------------------

    fn set(paths: &[&str]) -> BTreeSet<RelPath> {
        paths.iter().map(|path| p(path)).collect()
    }

    fn file<'a>(change: &'a WholeChange, path: &str) -> &'a WholeFile {
        change
            .files
            .get(&p(path))
            .unwrap_or_else(|| panic!("{path} is in the whole change"))
    }

    #[test]
    fn origins_come_from_the_two_path_sets() {
        let names = vec![
            record("both.rs", ChangeKind::Modified, OLD, NEW),
            record("c.rs", ChangeKind::Modified, OLD, NEW),
            record("u.rs", ChangeKind::Modified, OLD, NEW),
        ];
        let change = WholeChange::assemble(
            names,
            Some(&set(&["both.rs", "c.rs"])),
            &set(&["both.rs", "u.rs"]),
            &BTreeMap::new(),
            Vec::new(),
        );
        assert_eq!(file(&change, "both.rs").origin, Origin::Both);
        assert_eq!(file(&change, "c.rs").origin, Origin::Committed);
        assert_eq!(file(&change, "u.rs").origin, Origin::Uncommitted);
    }

    #[test]
    fn without_a_committed_set_every_file_is_uncommitted() {
        let change = WholeChange::assemble(
            vec![record("a.rs", ChangeKind::Modified, OLD, NEW)],
            None,
            &BTreeSet::new(),
            &BTreeMap::new(),
            vec![(p("new.rs"), "100644".into())],
        );
        assert!(change
            .files
            .values()
            .all(|file| file.origin == Origin::Uncommitted));
    }

    #[test]
    fn a_zero_new_id_is_filled_from_hashed_or_leaves_no_id() {
        let names = vec![
            record("hashed.rs", ChangeKind::Modified, OLD, ZEROS),
            record("missing.rs", ChangeKind::Modified, OLD, ZEROS),
            NameStatus {
                new_mode: "000000".into(),
                ..record("gone.rs", ChangeKind::Deleted, OLD, ZEROS)
            },
        ];
        let hashed = BTreeMap::from([(p("hashed.rs"), NEW.to_owned())]);
        let change = WholeChange::assemble(names, None, &BTreeSet::new(), &hashed, Vec::new());
        assert_eq!(
            file(&change, "hashed.rs").id,
            Some(ChangeId::of_worktree(&record(
                "hashed.rs",
                ChangeKind::Modified,
                OLD,
                NEW
            ))),
            "the hashed id stands in for git's zeros"
        );
        assert_eq!(file(&change, "missing.rs").id, None, "no id could be read");
        assert!(
            file(&change, "gone.rs").id.is_some(),
            "a deleted file's zero new side is real, not unhashed"
        );
    }

    #[test]
    fn an_untracked_file_is_recorded_as_gits_added_file() {
        let hashed = BTreeMap::from([(p("new.rs"), NEW.to_owned())]);
        let untracked = WholeChange::assemble(
            Vec::new(),
            None,
            &BTreeSet::new(),
            &hashed,
            vec![(p("new.rs"), "100755".into())],
        );
        let new = file(&untracked, "new.rs");
        assert_eq!(new.kind, ChangeKind::Added);
        assert_eq!(new.origin, Origin::Uncommitted);
        let staged = NameStatus {
            old_mode: "000000".into(),
            new_mode: "100755".into(),
            ..record("new.rs", ChangeKind::Added, ZEROS, NEW)
        };
        assert_eq!(
            new.id,
            Some(ChangeId::of_worktree(&staged)),
            "the same id as git's added record of the same bytes (research R1)"
        );
        let unhashed = WholeChange::assemble(
            Vec::new(),
            None,
            &BTreeSet::new(),
            &BTreeMap::new(),
            vec![(p("new.rs"), "100644".into())],
        );
        assert_eq!(file(&unhashed, "new.rs").id, None);
    }

    #[test]
    fn one_file_per_path_keyed_by_the_path_now() {
        let names = vec![
            record(
                "new name.rs",
                ChangeKind::Renamed { from: p("old.rs") },
                OLD,
                OLD,
            ),
            record("a.rs", ChangeKind::Modified, OLD, NEW),
        ];
        let change = WholeChange::assemble(
            names,
            None,
            &BTreeSet::new(),
            &BTreeMap::new(),
            vec![(p("a.rs"), "100644".into())],
        );
        let paths: Vec<&str> = change.files.keys().map(RelPath::as_str).collect();
        assert_eq!(paths, ["a.rs", "new name.rs"]);
        assert_eq!(
            file(&change, "a.rs").kind,
            ChangeKind::Modified,
            "a tracked record wins over an untracked one of the same path"
        );
        assert_eq!(
            file(&change, "new name.rs").kind,
            ChangeKind::Renamed { from: p("old.rs") }
        );
    }

    #[test]
    fn a_mode_change_with_equal_ids_is_mode_only() {
        let names = vec![NameStatus {
            mode_changed: true,
            new_mode: "100755".into(),
            ..record("m.sh", ChangeKind::Modified, OLD, OLD)
        }];
        let change =
            WholeChange::assemble(names, None, &BTreeSet::new(), &BTreeMap::new(), Vec::new());
        assert_eq!(file(&change, "m.sh").kind, ChangeKind::ModeOnly);
    }

    // ---- file_review and progress (T012) --------------------------------------------------------

    fn review(state: ReviewState, changed_since: bool) -> FileReview {
        FileReview {
            state,
            changed_since,
        }
    }

    #[test]
    fn file_review_follows_the_data_model_table() {
        use ReviewState::{HasDrafts, Reviewed, Unseen};
        let (m, c) = (id(1), id(2));
        let rows = [
            (Some(&m), Some(&m), 0, review(Reviewed, false)),
            (Some(&m), Some(&m), 3, review(Reviewed, false)),
            (Some(&m), Some(&c), 0, review(Unseen, true)),
            (Some(&m), Some(&c), 2, review(HasDrafts, true)),
            (Some(&m), None, 0, review(Unseen, false)),
            (Some(&m), None, 1, review(HasDrafts, false)),
            (None, Some(&c), 0, review(Unseen, false)),
            (None, Some(&c), 1, review(HasDrafts, false)),
            (None, None, 0, review(Unseen, false)),
            (None, None, 4, review(HasDrafts, false)),
        ];
        for (mark, current, drafts, expected) in rows {
            assert_eq!(
                file_review(mark, current, drafts),
                expected,
                "mark {mark:?}, current {current:?}, {drafts} drafts"
            );
        }
    }

    #[test]
    fn progress_counts_the_reviewed_files_of_those_passed() {
        let reviewed = review(ReviewState::Reviewed, false);
        let unseen = review(ReviewState::Unseen, true);
        let drafts = review(ReviewState::HasDrafts, false);
        assert_eq!(progress(Vec::new()), None, "no files, no progress");
        assert_eq!(
            progress([unseen, drafts]),
            Some(Ratio { done: 0, total: 2 })
        );
        assert_eq!(
            progress([reviewed, unseen, drafts]),
            Some(Ratio { done: 1, total: 3 })
        );
        assert_eq!(
            progress([reviewed, reviewed]),
            Some(Ratio { done: 2, total: 2 })
        );
    }
}
