//! Reading one file's text from where the reviewer is looking (feature 1150, FR-005, FR-010,
//! FR-014, research R3 and R5): the worktree's own file, or a pull request's head version.
//!
//! Nothing here writes: a read is `git cat-file`, a file read, or one `gh` request through the same
//! [`PullRequestSource`] the pull request diff uses. A path never resolves outside the repository.

use std::path::{Component, Path, PathBuf};

use super::changes::{content_of, Content};
use super::diff::SideLines;
use super::git::{object_sizes, read_bytes, worktree_bytes};
use super::limits;
use crate::github::GithubRepo;
use crate::pull_request::PullRequestSource;

/// Where a file is read from (data-model §5).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileSource {
    /// The entry's own file, as it is on disk now.
    Worktree {
        /// The entry's directory.
        dir: PathBuf,
    },
    /// A pull request's head version of the file.
    PrHead {
        /// The entry's directory: its objects are tried before GitHub.
        dir: PathBuf,
        /// The repository on GitHub, for a head commit that is not local.
        repo: GithubRepo,
        /// The head commit.
        commit: String,
    },
}

/// A relative path already checked to stay inside the repository: no `..`, no root, no drive.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Confined(String);

impl Confined {
    /// `path` as a `/`-separated relative path, or [`FileReadFailure::OutsideRepository`] when it is
    /// empty, absolute, or climbs out with `..` (FR-014). Symlinks are checked when the file is
    /// read, since only the disk knows them.
    pub fn new(path: &str) -> Result<Self, FileReadFailure> {
        let native = path.replace('\\', "/");
        let bytes = native.as_bytes();
        let drive = bytes.len() >= 2 && bytes[0].is_ascii_alphabetic() && bytes[1] == b':';
        let climbs = native.split('/').any(|part| part == "..");
        if native.is_empty() || native.starts_with('/') || drive || climbs {
            return Err(FileReadFailure::OutsideRepository);
        }
        Ok(Self(native))
    }

    /// The path, `/`-separated.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// A file's text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileText {
    /// Its lines.
    pub lines: SideLines,
    /// Its size in bytes.
    pub bytes: u64,
}

/// Why a file's text is not shown (data-model §6, FR-009). Each has one message.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileReadFailure {
    /// The file is binary.
    Binary,
    /// The file is not valid UTF-8.
    NotUtf8,
    /// There is no such file in that version.
    Missing,
    /// The file is over the size limit; reading it anyway is the reviewer's choice.
    TooLarge {
        /// Its size in bytes.
        bytes: u64,
    },
    /// The path leads outside the repository.
    OutsideRepository,
    /// The file could not be read.
    Unreadable,
    /// The pull request's head version could not be obtained: not local, and GitHub did not give it.
    HeadUnavailable,
}

impl FileReadFailure {
    /// The one line that replaces the file's content.
    pub fn message(&self) -> String {
        match self {
            Self::Binary => "This file is binary; there is no text to show".into(),
            Self::NotUtf8 => "This file is not UTF-8 text; there is no text to show".into(),
            Self::Missing => "This file does not exist in this version".into(),
            Self::TooLarge { bytes } => format!(
                "This file is too large to show ({} KB)",
                bytes.div_ceil(1024)
            ),
            Self::OutsideRepository => "This path is outside the repository".into(),
            Self::Unreadable => "This file could not be read".into(),
            Self::HeadUnavailable => {
                "The pull request's version of this file could not be obtained".into()
            }
        }
    }
}

/// Read `path` from `source`. A worktree read touches only the disk; a pull request head read tries
/// the local objects first and asks `gh` (one request, no retry) only when the commit is not local.
/// A file over [`limits::MAX_VERSION_BYTES`] is [`FileReadFailure::TooLarge`] unless `force_large`.
pub fn read_file(
    source: &FileSource,
    path: &Confined,
    force_large: bool,
    gh: &dyn PullRequestSource,
    now: u64,
) -> Result<FileText, FileReadFailure> {
    let bytes = match source {
        FileSource::Worktree { dir } => worktree_file(dir, path, force_large)?,
        FileSource::PrHead { dir, repo, commit } => head_file(dir, repo, commit, path, gh, now)?,
    };
    text_of(bytes, force_large)
}

/// The size gate and the text classification shared by every source.
fn text_of(bytes: Vec<u8>, force_large: bool) -> Result<FileText, FileReadFailure> {
    let size = bytes.len() as u64;
    if !force_large && size > limits::MAX_VERSION_BYTES {
        return Err(FileReadFailure::TooLarge { bytes: size });
    }
    match content_of(&bytes).0 {
        Content::Binary => Err(FileReadFailure::Binary),
        Content::NotUtf8 => Err(FileReadFailure::NotUtf8),
        Content::Text => SideLines::from_bytes(&bytes)
            .map(|lines| FileText { lines, bytes: size })
            .ok_or(FileReadFailure::NotUtf8),
    }
}

fn disk_failure(error: &std::io::Error) -> FileReadFailure {
    if error.kind() == std::io::ErrorKind::NotFound {
        FileReadFailure::Missing
    } else {
        FileReadFailure::Unreadable
    }
}

/// `path` lexically cleaned of `.` and `..`.
fn normalised(path: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for part in path.components() {
        match part {
            Component::CurDir => {}
            Component::ParentDir => {
                out.pop();
            }
            other => out.push(other),
        }
    }
    out
}

/// The worktree file's bytes: the parent directory must resolve inside the root, and a symlink is
/// read as its target path (what git tracks) only when that target stays inside too (R5).
fn worktree_file(dir: &Path, path: &Confined, force_large: bool) -> Result<Vec<u8>, FileReadFailure> {
    let root = dir.canonicalize().map_err(|e| disk_failure(&e))?;
    let full = dir.join(path.as_str());
    let name = full.file_name().ok_or(FileReadFailure::Missing)?.to_owned();
    let parent = full
        .parent()
        .unwrap_or(dir)
        .canonicalize()
        .map_err(|e| disk_failure(&e))?;
    if !parent.starts_with(&root) {
        return Err(FileReadFailure::OutsideRepository);
    }
    let full = parent.join(name);
    let meta = std::fs::symlink_metadata(&full).map_err(|e| disk_failure(&e))?;
    if meta.file_type().is_symlink() {
        let target = std::fs::read_link(&full).map_err(|e| disk_failure(&e))?;
        if !normalised(&parent.join(&target)).starts_with(&root) {
            return Err(FileReadFailure::OutsideRepository);
        }
    } else if !meta.is_file() {
        return Err(FileReadFailure::Unreadable);
    } else if !force_large && meta.len() > limits::MAX_VERSION_BYTES {
        // Refused before reading: a huge file is never pulled into memory unasked.
        return Err(FileReadFailure::TooLarge { bytes: meta.len() });
    }
    worktree_bytes(&full).map_err(|e| disk_failure(&e))
}

/// A head commit is a hex object name; anything else could be read as a git option.
fn is_object_name(commit: &str) -> bool {
    !commit.is_empty() && commit.bytes().all(|b| b.is_ascii_hexdigit())
}

/// The file at the pull request's head: local objects first, `gh` when the commit is not local.
fn head_file(
    dir: &Path,
    repo: &GithubRepo,
    commit: &str,
    path: &Confined,
    gh: &dyn PullRequestSource,
    now: u64,
) -> Result<Vec<u8>, FileReadFailure> {
    if !is_object_name(commit) {
        return Err(FileReadFailure::HeadUnavailable);
    }
    let local = read_bytes(
        dir,
        &["cat-file", "-e", &format!("{commit}^{{commit}}")],
        None,
    )
    .is_ok();
    if !local {
        let rel = super::RelPath::from_native(path.as_str()).ok_or(FileReadFailure::Missing)?;
        return gh.file_at(repo, commit, &rel, now);
    }
    let spec = format!("{commit}:{}", path.as_str());
    if object_sizes(dir, std::slice::from_ref(&spec))[0].is_none() {
        return Err(FileReadFailure::Missing);
    }
    read_bytes(dir, &["cat-file", "blob", &spec], None).map_err(|_| FileReadFailure::Unreadable)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_relative_path_is_confined() {
        assert_eq!(Confined::new("src/a.rs").unwrap().as_str(), "src/a.rs");
        assert_eq!(Confined::new("a\\b.rs").unwrap().as_str(), "a/b.rs");
    }

    #[test]
    fn a_path_that_climbs_or_is_rooted_is_refused() {
        for bad in ["", "../x", "a/../../x", "/etc/passwd", "\\x", "C:/x", "a/.."] {
            assert_eq!(
                Confined::new(bad),
                Err(FileReadFailure::OutsideRepository),
                "{bad:?}"
            );
        }
    }

    #[test]
    fn text_is_classified_and_sized() {
        assert_eq!(text_of(vec![0, 1], false), Err(FileReadFailure::Binary));
        assert_eq!(text_of(vec![0xff, 0xfe], false), Err(FileReadFailure::NotUtf8));
        let big = vec![b'a'; (limits::MAX_VERSION_BYTES + 1) as usize];
        assert!(matches!(
            text_of(big.clone(), false),
            Err(FileReadFailure::TooLarge { .. })
        ));
        assert!(text_of(big, true).is_ok());
        assert_eq!(text_of(b"a\nb\n".to_vec(), false).unwrap().lines.len(), 2);
    }

    #[test]
    fn every_failure_has_a_message() {
        for failure in [
            FileReadFailure::Binary,
            FileReadFailure::NotUtf8,
            FileReadFailure::Missing,
            FileReadFailure::TooLarge { bytes: 3_000_000 },
            FileReadFailure::OutsideRepository,
            FileReadFailure::Unreadable,
            FileReadFailure::HeadUnavailable,
        ] {
            assert!(!failure.message().is_empty());
        }
    }

    #[test]
    fn a_head_commit_must_be_hex() {
        assert!(is_object_name("abc123"));
        assert!(!is_object_name("-x"));
        assert!(!is_object_name(""));
    }
}
