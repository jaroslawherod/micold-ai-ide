//! The changed-file list (FR-002, FR-004, FR-005, FR-008, research R1, R8): pure parsers over
//! `git diff -z` output and the rules that turn them into one row per path.
//!
//! The kinds come from `git diff -z -M --raw` rather than `--name-status --summary`: git prints
//! no summary beside `--name-status` (checked on git 2.43), while `--raw` carries both modes, so a
//! mode-only change is told from the same output.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use super::base::ReviewScope;
use super::{limits, RelPath};

/// How a file changed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ChangeKind {
    /// New in the range.
    Added,
    /// Content changed (possibly the mode too).
    Modified,
    /// Gone in the range.
    Deleted,
    /// Moved from `from` (git's rename detection, `-M`).
    Renamed {
        /// The path before.
        from: RelPath,
    },
    /// Only the file mode changed.
    ModeOnly,
    /// Not tracked and not ignored; shown as added.
    Untracked,
}

/// Whether a file's diff can be shown as text (FR-008).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Content {
    /// Text: its diff can be shown.
    Text,
    /// Binary (git says so, or the bytes hold a NUL).
    Binary,
    /// Not valid UTF-8.
    NotUtf8,
}

/// Where a listed file's change comes from (Key Entities).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Origin {
    /// Only commits since the base.
    Committed,
    /// Only the working tree and index.
    Uncommitted,
    /// Both.
    Both,
}

/// One row of the list.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChangedFile {
    /// The path now (the new path of a rename).
    pub path: RelPath,
    /// How it changed.
    pub kind: ChangeKind,
    /// Whether its diff is text.
    pub content: Content,
    /// Lines added (0 for binary).
    pub added: u32,
    /// Lines removed (0 for binary).
    pub removed: u32,
    /// Above the R8 limits: the diff waits for "Show diff".
    pub large: bool,
    /// Committed, uncommitted or both.
    pub origin: Origin,
}

/// An entry's changed files under the current toggles, sorted by path, one per path.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChangeList {
    /// The rows.
    pub files: Vec<ChangedFile>,
    /// What they were compared with.
    pub scope: ReviewScope,
}

/// One record of `git diff -z -M --numstat`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NumStat {
    /// The path (the new path of a rename).
    pub path: RelPath,
    /// `(added, removed)`, or `None` for a binary file (`-\t-`).
    pub lines: Option<(u32, u32)>,
}

/// One record of `git diff -z -M --raw`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NameStatus {
    /// The path (the new path of a rename).
    pub path: RelPath,
    /// `Added`, `Modified`, `Deleted` or `Renamed`.
    pub kind: ChangeKind,
    /// The two modes differ.
    pub mode_changed: bool,
}

/// An untracked, not-ignored file as read from disk.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Untracked {
    /// Its path.
    pub path: RelPath,
    /// Its lines (0 unless text).
    pub lines: u32,
    /// Text, binary or not UTF-8, from its bytes.
    pub content: Content,
    /// Its size in bytes.
    pub bytes: u64,
}

/// The sizes of a file's two versions, where known (R8).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct VersionSizes {
    /// The base (or `HEAD`) version.
    pub old: Option<u64>,
    /// The new version.
    pub new: Option<u64>,
}

/// Parse `git diff -z -M --numstat`: `added\tremoved\tpath\0`, or for a rename
/// `added\tremoved\t\0old\0new\0`; a binary file has `-` for both counts.
pub fn parse_numstat(raw: &str) -> Vec<NumStat> {
    let mut tokens = raw.split('\0');
    let mut stats = Vec::new();
    while let Some(record) = tokens.next() {
        let mut fields = record.splitn(3, '\t');
        let (Some(added), Some(removed), Some(path)) =
            (fields.next(), fields.next(), fields.next())
        else {
            continue;
        };
        let path = if path.is_empty() {
            // A rename: the old and the new path follow as their own records.
            let _old = tokens.next();
            match tokens.next() {
                Some(new) => new,
                None => break,
            }
        } else {
            path
        };
        let lines = match (added.parse(), removed.parse()) {
            (Ok(added), Ok(removed)) => Some((added, removed)),
            _ => None,
        };
        stats.push(NumStat {
            path: RelPath::from_git(path),
            lines,
        });
    }
    stats
}

/// Parse `git diff -z -M --raw`: `:<old mode> <new mode> <old id> <new id> <status>\0<path>\0`,
/// with `\0<old>\0<new>\0` after a rename or copy status. A type change counts as modified.
pub fn parse_name_status(raw: &str) -> Vec<NameStatus> {
    let mut tokens = raw.split('\0');
    let mut names = Vec::new();
    while let Some(header) = tokens.next() {
        let Some(header) = header.strip_prefix(':') else {
            continue;
        };
        let fields: Vec<&str> = header.split(' ').collect();
        let [old_mode, new_mode, _, _, status] = fields[..] else {
            continue;
        };
        let Some(first) = tokens.next() else {
            break;
        };
        let letter = status.chars().next().unwrap_or('M');
        let (path, kind) = match letter {
            'R' | 'C' => {
                let Some(new) = tokens.next() else {
                    break;
                };
                let kind = if letter == 'R' {
                    ChangeKind::Renamed {
                        from: RelPath::from_git(first),
                    }
                } else {
                    ChangeKind::Added
                };
                (new, kind)
            }
            'A' => (first, ChangeKind::Added),
            'D' => (first, ChangeKind::Deleted),
            _ => (first, ChangeKind::Modified),
        };
        let absent = |mode: &str| mode.bytes().all(|b| b == b'0');
        let mode_changed = !absent(old_mode) && !absent(new_mode) && old_mode != new_mode;
        names.push(NameStatus {
            path: RelPath::from_git(path),
            kind,
            mode_changed,
        });
    }
    names
}

/// Whether bytes read from disk are text, binary (a NUL byte, as git's own heuristic) or not
/// UTF-8, and how many lines they hold (a last line without a newline counts).
pub fn content_of(bytes: &[u8]) -> (Content, u32) {
    if bytes.contains(&0) {
        return (Content::Binary, 0);
    }
    if std::str::from_utf8(bytes).is_err() {
        return (Content::NotUtf8, 0);
    }
    let newlines = bytes.iter().filter(|&&b| b == b'\n').count();
    let unterminated = usize::from(bytes.last().is_some_and(|&b| b != b'\n'));
    let lines = u32::try_from(newlines + unterminated).unwrap_or(u32::MAX);
    (Content::Text, lines)
}

/// The rows of one diff range: kinds from `names`, counts from `stats`, origin from the two path
/// sets (a path in both is `Both`; in neither, `fallback`). Sorted by path, one row per path.
pub fn assemble(
    names: Vec<NameStatus>,
    stats: Vec<NumStat>,
    committed: &BTreeSet<RelPath>,
    uncommitted: &BTreeSet<RelPath>,
    fallback: Origin,
) -> Vec<ChangedFile> {
    let mut stats: BTreeMap<RelPath, Option<(u32, u32)>> = stats
        .into_iter()
        .map(|stat| (stat.path, stat.lines))
        .collect();
    let mut rows: BTreeMap<RelPath, ChangedFile> = BTreeMap::new();
    for name in names {
        let lines = stats.remove(&name.path).unwrap_or(Some((0, 0)));
        let (content, (added, removed)) = match lines {
            Some(counts) => (Content::Text, counts),
            None => (Content::Binary, (0, 0)),
        };
        let kind =
            if name.mode_changed && name.kind == ChangeKind::Modified && lines == Some((0, 0)) {
                ChangeKind::ModeOnly
            } else {
                name.kind
            };
        let origin = match (
            committed.contains(&name.path),
            uncommitted.contains(&name.path),
        ) {
            (true, true) => Origin::Both,
            (true, false) => Origin::Committed,
            (false, true) => Origin::Uncommitted,
            (false, false) => fallback,
        };
        rows.insert(
            name.path.clone(),
            ChangedFile {
                path: name.path,
                kind,
                content,
                added,
                removed,
                large: false,
                origin,
            },
        );
    }
    rows.into_values().collect()
}

/// Add the untracked files as `Untracked`, uncommitted rows; a path already listed keeps its row.
/// The result stays sorted by path.
pub fn merge_untracked(files: Vec<ChangedFile>, untracked: Vec<Untracked>) -> Vec<ChangedFile> {
    let mut rows: BTreeMap<RelPath, ChangedFile> = files
        .into_iter()
        .map(|file| (file.path.clone(), file))
        .collect();
    for file in untracked {
        rows.entry(file.path.clone()).or_insert_with(|| {
            let mut row = ChangedFile {
                path: file.path,
                kind: ChangeKind::Untracked,
                content: file.content,
                added: file.lines,
                removed: 0,
                large: false,
                origin: Origin::Uncommitted,
            };
            classify(
                &mut row,
                VersionSizes {
                    old: None,
                    new: Some(file.bytes),
                },
            );
            row
        });
    }
    rows.into_values().collect()
}

/// Mark `file` large when its added + removed lines exceed [`limits::MAX_CHANGED_LINES`] or either
/// version exceeds [`limits::MAX_VERSION_BYTES`] (R8).
pub fn classify(file: &mut ChangedFile, sizes: VersionSizes) {
    let lines = u64::from(file.added) + u64::from(file.removed);
    let too_big = |size: Option<u64>| size.is_some_and(|size| size > limits::MAX_VERSION_BYTES);
    file.large =
        lines > u64::from(limits::MAX_CHANGED_LINES) || too_big(sizes.old) || too_big(sizes.new);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(path: &str) -> RelPath {
        RelPath::from_git(path)
    }

    fn set(paths: &[&str]) -> BTreeSet<RelPath> {
        paths.iter().map(|path| p(path)).collect()
    }

    /// Captured from git 2.43 (`git -c core.quotepath=false diff -z -M --numstat`): a modified
    /// file, an added one, a binary one, a mode-only one and a rename to a path with a space and
    /// a non-ASCII letter.
    const NUMSTAT: &str = "1\t1\ta.txt\x001\t0\tadded.txt\x00-\t-\tbin.dat\x000\t0\tm.sh\x000\t0\t\x00old.txt\x00new name é.txt\x00";

    /// The same range through `--raw -z`.
    const RAW: &str = ":100644 100644 422c2b7 0f7bc76 M\x00a.txt\x00:000000 100644 0000000 8ba3a16 A\x00added.txt\x00:100644 100644 bdc955b 350ed01 M\x00bin.dat\x00:100644 100755 587be6b 587be6b M\x00m.sh\x00:100644 100644 74c047b 74c047b R100\x00old.txt\x00new name é.txt\x00:100644 000000 1111111 0000000 D\x00gone.rs\x00";

    #[test]
    fn numstat_reads_counts_binary_and_renames_with_paths_verbatim() {
        let stats = parse_numstat(NUMSTAT);
        assert_eq!(
            stats,
            vec![
                NumStat {
                    path: p("a.txt"),
                    lines: Some((1, 1))
                },
                NumStat {
                    path: p("added.txt"),
                    lines: Some((1, 0))
                },
                NumStat {
                    path: p("bin.dat"),
                    lines: None
                },
                NumStat {
                    path: p("m.sh"),
                    lines: Some((0, 0))
                },
                NumStat {
                    path: p("new name é.txt"),
                    lines: Some((0, 0))
                },
            ],
            "a rename is keyed by its new path and `-\\t-` is binary"
        );
    }

    #[test]
    fn raw_status_reads_kinds_renames_and_mode_changes() {
        let names = parse_name_status(RAW);
        assert_eq!(
            names,
            vec![
                NameStatus {
                    path: p("a.txt"),
                    kind: ChangeKind::Modified,
                    mode_changed: false
                },
                NameStatus {
                    path: p("added.txt"),
                    kind: ChangeKind::Added,
                    mode_changed: false
                },
                NameStatus {
                    path: p("bin.dat"),
                    kind: ChangeKind::Modified,
                    mode_changed: false
                },
                NameStatus {
                    path: p("m.sh"),
                    kind: ChangeKind::Modified,
                    mode_changed: true
                },
                NameStatus {
                    path: p("new name é.txt"),
                    kind: ChangeKind::Renamed { from: p("old.txt") },
                    mode_changed: false,
                },
                NameStatus {
                    path: p("gone.rs"),
                    kind: ChangeKind::Deleted,
                    mode_changed: false
                },
            ]
        );
    }

    #[test]
    fn assembling_gives_one_sorted_row_per_path_with_kind_counts_and_content() {
        let rows = assemble(
            parse_name_status(RAW),
            parse_numstat(NUMSTAT),
            &set(&["a.txt", "m.sh"]),
            &set(&["a.txt", "bin.dat"]),
            Origin::Committed,
        );
        let paths: Vec<&str> = rows.iter().map(|row| row.path.as_str()).collect();
        assert_eq!(
            paths,
            [
                "a.txt",
                "added.txt",
                "bin.dat",
                "gone.rs",
                "m.sh",
                "new name é.txt"
            ],
            "sorted by path, one row each"
        );
        let row = |path: &str| rows.iter().find(|row| row.path.as_str() == path).unwrap();
        assert_eq!((row("a.txt").added, row("a.txt").removed), (1, 1));
        assert_eq!(row("a.txt").origin, Origin::Both, "in both sets");
        assert_eq!(row("bin.dat").origin, Origin::Uncommitted);
        assert_eq!(row("m.sh").origin, Origin::Committed);
        assert_eq!(
            row("added.txt").origin,
            Origin::Committed,
            "in neither: the fallback"
        );
        assert_eq!(
            row("bin.dat").content,
            Content::Binary,
            "numstat `-` is binary"
        );
        assert_eq!((row("bin.dat").added, row("bin.dat").removed), (0, 0));
        assert_eq!(row("a.txt").content, Content::Text);
        assert_eq!(
            row("m.sh").kind,
            ChangeKind::ModeOnly,
            "modes differ and no line changed: mode only"
        );
        assert_eq!(
            row("new name é.txt").kind,
            ChangeKind::Renamed { from: p("old.txt") }
        );
        assert_eq!(row("gone.rs").kind, ChangeKind::Deleted);
    }

    #[test]
    fn a_mode_change_with_content_changes_is_modified() {
        let rows = assemble(
            vec![NameStatus {
                path: p("m.sh"),
                kind: ChangeKind::Modified,
                mode_changed: true,
            }],
            vec![NumStat {
                path: p("m.sh"),
                lines: Some((2, 0)),
            }],
            &BTreeSet::new(),
            &BTreeSet::new(),
            Origin::Uncommitted,
        );
        assert_eq!(rows[0].kind, ChangeKind::Modified);
    }

    #[test]
    fn untracked_files_are_merged_as_uncommitted_rows_in_path_order() {
        let tracked = assemble(
            parse_name_status(RAW),
            parse_numstat(NUMSTAT),
            &BTreeSet::new(),
            &BTreeSet::new(),
            Origin::Uncommitted,
        );
        let rows = merge_untracked(
            tracked,
            vec![
                Untracked {
                    path: p("b new.md"),
                    lines: 3,
                    content: Content::Text,
                    bytes: 9,
                },
                Untracked {
                    path: p("zz.png"),
                    lines: 0,
                    content: Content::Binary,
                    bytes: 40,
                },
            ],
        );
        let paths: Vec<&str> = rows.iter().map(|row| row.path.as_str()).collect();
        assert_eq!(
            paths,
            [
                "a.txt",
                "added.txt",
                "b new.md",
                "bin.dat",
                "gone.rs",
                "m.sh",
                "new name é.txt",
                "zz.png"
            ]
        );
        let new = rows
            .iter()
            .find(|row| row.path.as_str() == "b new.md")
            .unwrap();
        assert_eq!(new.kind, ChangeKind::Untracked);
        assert_eq!(
            (new.added, new.removed),
            (3, 0),
            "every line of an untracked file is added"
        );
        assert_eq!(new.origin, Origin::Uncommitted);
        let png = rows
            .iter()
            .find(|row| row.path.as_str() == "zz.png")
            .unwrap();
        assert_eq!(png.content, Content::Binary);
    }

    #[test]
    fn bytes_on_disk_are_text_binary_or_not_utf8_with_their_line_count() {
        assert_eq!(content_of(b"a\nb\n"), (Content::Text, 2));
        assert_eq!(
            content_of(b"a\nb"),
            (Content::Text, 2),
            "a last line without newline counts"
        );
        assert_eq!(content_of(b""), (Content::Text, 0));
        assert_eq!(
            content_of(b"a\x00b"),
            (Content::Binary, 0),
            "a NUL byte is binary"
        );
        assert_eq!(
            content_of(b"caf\xe9\n"),
            (Content::NotUtf8, 0),
            "Latin-1 is not UTF-8"
        );
    }

    fn text_row(added: u32, removed: u32) -> ChangedFile {
        ChangedFile {
            path: p("f.rs"),
            kind: ChangeKind::Modified,
            content: Content::Text,
            added,
            removed,
            large: false,
            origin: Origin::Uncommitted,
        }
    }

    #[test]
    fn a_file_is_large_above_5000_changed_lines_but_not_at_it() {
        let mut at = text_row(2_500, 2_500);
        classify(&mut at, VersionSizes::default());
        assert!(!at.large, "5,000 changed lines is within the limit");
        let mut over = text_row(2_501, 2_500);
        classify(&mut over, VersionSizes::default());
        assert!(over.large, "5,001 changed lines is over it");
    }

    #[test]
    fn a_file_is_large_when_either_version_is_above_2_mb_but_not_at_it() {
        let limit = 2 * 1024 * 1024;
        let mut at = text_row(1, 1);
        classify(
            &mut at,
            VersionSizes {
                old: Some(limit),
                new: Some(limit),
            },
        );
        assert!(!at.large, "exactly 2 MB is within the limit");
        let mut old = text_row(1, 1);
        classify(
            &mut old,
            VersionSizes {
                old: Some(limit + 1),
                new: Some(10),
            },
        );
        assert!(old.large, "a base version above 2 MB is large");
        let mut new = text_row(1, 1);
        classify(
            &mut new,
            VersionSizes {
                old: None,
                new: Some(limit + 1),
            },
        );
        assert!(new.large, "a new version above 2 MB is large");
    }
}
