//! Reviewing an entry's changes (feature 482): the render-free half of the Changes view and of
//! review comments. Lists and diffs come from the user's `git` (research R1), parsed here by pure
//! functions; nothing in this module draws or owns a PTY (FR-022).

pub mod base;
pub mod changes;
pub mod comment;
pub mod diff;
pub mod git;
pub mod prompt;
pub mod store;
pub mod target;
pub mod watch;

use std::num::NonZeroU32;

use serde::{Deserialize, Serialize};

/// A path relative to an entry's root, always `/`-separated (Principle VI, FR-015).
///
/// Built from git's `-z` output (git never writes `\` as a separator) or by
/// [`RelPath::from_native`]. Never absolute.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct RelPath(String);

impl RelPath {
    /// A path as the platform writes it: `\` separators become `/`. `None` for an absolute path
    /// (a leading `/` or `\`, or a drive letter) or an empty one.
    pub fn from_native(path: &str) -> Option<Self> {
        let bytes = path.as_bytes();
        let drive = bytes.len() >= 2 && bytes[0].is_ascii_alphabetic() && bytes[1] == b':';
        if path.is_empty() || path.starts_with(['/', '\\']) || drive {
            return None;
        }
        Some(Self(path.replace('\\', "/")))
    }

    /// A path exactly as git printed it (`-z` output, `core.quotepath=false`): already
    /// `/`-separated; a `\` in it is part of a file name, not a separator.
    pub(crate) fn from_git(path: &str) -> Self {
        Self(path.to_owned())
    }

    /// The path as text.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for RelPath {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// Which version of a file a line belongs to: `Old` is a removed line, numbered in the base
/// version; context and added lines are `New` (US2 s4).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Side {
    /// The file as it is now (context and added lines).
    New,
    /// The base version (removed lines).
    Old,
}

/// An inclusive range of 1-based line numbers on one side; one line is `start == end`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "RawRange", into = "RawRange")]
pub struct LineRange {
    start: NonZeroU32,
    end: NonZeroU32,
}

#[derive(Serialize, Deserialize)]
struct RawRange {
    start: u32,
    end: u32,
}

impl TryFrom<RawRange> for LineRange {
    type Error = String;
    fn try_from(raw: RawRange) -> Result<Self, Self::Error> {
        LineRange::new(raw.start, raw.end)
            .ok_or_else(|| format!("invalid line range {}..={}", raw.start, raw.end))
    }
}

impl From<LineRange> for RawRange {
    fn from(range: LineRange) -> Self {
        RawRange {
            start: range.start(),
            end: range.end(),
        }
    }
}

impl LineRange {
    /// `None` when `start` is 0 or after `end`.
    pub fn new(start: u32, end: u32) -> Option<Self> {
        if start > end {
            return None;
        }
        Some(Self {
            start: NonZeroU32::new(start)?,
            end: NonZeroU32::new(end)?,
        })
    }

    /// The first line.
    pub fn start(self) -> u32 {
        self.start.get()
    }

    /// The last line (inclusive).
    pub fn end(self) -> u32 {
        self.end.get()
    }

    /// How many lines the range covers.
    pub fn len(self) -> u32 {
        self.end.get() - self.start.get() + 1
    }

    /// Never true: a range holds at least one line. Present for clippy's `len_without_is_empty`.
    pub fn is_empty(self) -> bool {
        false
    }
}

/// The size limits of research R8.
pub mod limits {
    /// Above this many added + removed lines a file's diff waits for "Show diff".
    pub const MAX_CHANGED_LINES: u32 = 5_000;
    /// Above this many bytes in either version a file's diff waits for "Show diff".
    pub const MAX_VERSION_BYTES: u64 = 2 * 1024 * 1024;
    /// At most this many lines of a range are quoted in the review prompt.
    pub const MAX_QUOTED_LINES: usize = 50;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_native_path_with_backslashes_becomes_slash_separated() {
        let path = RelPath::from_native("src\\ui\\mod.rs").expect("a relative path is accepted");
        assert_eq!(
            path.as_str(),
            "src/ui/mod.rs",
            "a Windows separator is stored as `/`, so the prompt bytes match on every platform"
        );
        assert!(
            !path.as_str().contains('\\'),
            "a RelPath never holds `\\` as a separator"
        );
    }

    #[test]
    fn an_absolute_native_path_is_refused() {
        for absolute in ["/etc/passwd", "\\server\\share", "C:\\Users\\x", "c:/x", ""] {
            assert_eq!(
                RelPath::from_native(absolute),
                None,
                "`{absolute}` is absolute or empty, and a RelPath is relative to the entry root"
            );
        }
    }

    #[test]
    fn a_line_range_refuses_line_zero_and_a_reversed_range() {
        assert_eq!(LineRange::new(0, 3), None, "lines are numbered from 1");
        assert_eq!(LineRange::new(5, 4), None, "start after end is refused");
    }

    #[test]
    fn a_one_line_range_is_accepted_and_lengths_count_both_ends() {
        let one = LineRange::new(7, 7).expect("start == end is one line");
        assert_eq!(one.len(), 1, "a one-line range covers one line");
        let three = LineRange::new(12, 14).expect("12..=14 is a range");
        assert_eq!(three.len(), 3, "the range is inclusive at both ends");
        assert_eq!((three.start(), three.end()), (12, 14));
    }

    #[test]
    fn the_limits_are_those_of_research_r8() {
        assert_eq!(limits::MAX_CHANGED_LINES, 5_000);
        assert_eq!(limits::MAX_VERSION_BYTES, 2 * 1024 * 1024);
        assert_eq!(limits::MAX_QUOTED_LINES, 50);
    }
}
