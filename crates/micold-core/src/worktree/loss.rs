//! What deleting a worktree would lose (#1602, FR-006): the parsers for the two local git calls
//! behind [`crate::git::Git::worktree_loss`], and the 50-entry bound of the wire summary.

use crate::protocol::messages::{LossCommit, LossSummary};

/// Entries of each list the prompt carries; the rest is counted.
pub const LOSS_LIMIT: usize = 50;

/// Everything a worktree would lose, unbounded.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct WorktreeLoss {
    /// Changed and untracked files, repository-relative, in git's order.
    pub files: Vec<String>,
    /// Commits on no remote-tracking branch, newest first.
    pub commits: Vec<LossCommit>,
}

/// Longest a path or subject is shown in the prompt, in characters.
pub const LOSS_ENTRY_CHARS: usize = 200;

/// `text` made safe to show in the confirmation: each control, bidi/format or line/paragraph
/// separator character becomes `?` (the asking session controls file names and subjects, and could otherwise fake or hide
/// rows), and an entry over [`LOSS_ENTRY_CHARS`] is cut with `…`.
pub fn sanitize_entry(text: &str) -> String {
    let mut out: String = text
        .chars()
        .take(LOSS_ENTRY_CHARS)
        .map(|c| {
            // U+2028 and U+2029 (Zl, Zp) break a line in a text shaper but are not control chars.
            if crate::github::is_invisible(c) || matches!(c, '\u{2028}' | '\u{2029}') {
                '?'
            } else {
                c
            }
        })
        .collect();
    if text.chars().nth(LOSS_ENTRY_CHARS).is_some() {
        out.push('…');
    }
    out
}

/// The paths of `git status --porcelain=v1 -z` output. Each entry is `XY path`; a rename or copy
/// is followed by a second entry holding the original path, which is skipped.
pub fn parse_status_z(raw: &str) -> Vec<String> {
    let mut entries = raw.split('\0').filter(|e| !e.is_empty());
    let mut files = Vec::new();
    while let Some(entry) = entries.next() {
        let Some((status, path)) = entry.split_at_checked(3) else {
            continue;
        };
        if status.contains(['R', 'C']) {
            entries.next();
        }
        files.push(path.to_string());
    }
    files
}

/// The commits of `git log --format=%h%x00%s`: one `hash NUL subject` per line.
pub fn parse_log(raw: &str) -> Vec<LossCommit> {
    raw.lines()
        .filter_map(|line| line.split_once('\0'))
        .map(|(hash, subject)| LossCommit {
            short_hash: hash.to_string(),
            subject: subject.to_string(),
        })
        .collect()
}

impl LossSummary {
    /// `loss` cut to [`LOSS_LIMIT`] entries a list, with the counts left out.
    pub fn bounded(mut loss: WorktreeLoss) -> Self {
        let files_more = loss.files.len().saturating_sub(LOSS_LIMIT);
        let commits_more = loss.commits.len().saturating_sub(LOSS_LIMIT);
        loss.files.truncate(LOSS_LIMIT);
        loss.commits.truncate(LOSS_LIMIT);
        Self {
            files: loss.files,
            files_more: u32::try_from(files_more).unwrap_or(u32::MAX),
            commits: loss.commits,
            commits_more: u32::try_from(commits_more).unwrap_or(u32::MAX),
            unreadable: false,
        }
    }

    /// Git could not be read: empty lists, and the dialog says so.
    pub fn unreadable() -> Self {
        Self {
            files: Vec::new(),
            files_more: 0,
            commits: Vec::new(),
            commits_more: 0,
            unreadable: true,
        }
    }

    /// Nothing would be lost, as far as git could say: the dialog shows no list (FR-006).
    pub fn is_empty(&self) -> bool {
        self.files.is_empty() && self.commits.is_empty() && !self.unreadable
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_lists_changed_and_untracked_files_and_skips_a_rename_origin() {
        let raw = " M src/a.rs\0?? new file.txt\0R  to.rs\0from.rs\0A  b.rs\0";
        assert_eq!(
            parse_status_z(raw),
            ["src/a.rs", "new file.txt", "to.rs", "b.rs"]
        );
    }

    #[test]
    fn an_empty_status_lists_nothing() {
        assert!(parse_status_z("").is_empty());
    }

    #[test]
    fn log_lines_split_into_hash_and_subject() {
        let raw = "abc1234\0fix: a thing\ndef5678\0subject with \0 no\n";
        let commits = parse_log(raw);
        assert_eq!(commits[0].short_hash, "abc1234");
        assert_eq!(commits[0].subject, "fix: a thing");
        assert_eq!(commits.len(), 2);
        assert!(parse_log("").is_empty());
    }

    #[test]
    fn each_list_is_cut_at_fifty_and_the_rest_counted() {
        let loss = WorktreeLoss {
            files: (0..53).map(|i| format!("f{i}")).collect(),
            commits: (0..50)
                .map(|i| LossCommit {
                    short_hash: format!("h{i}"),
                    subject: String::new(),
                })
                .collect(),
        };
        let s = LossSummary::bounded(loss);
        assert_eq!((s.files.len(), s.files_more), (50, 3));
        assert_eq!((s.commits.len(), s.commits_more), (50, 0));
        assert!(!s.unreadable);
    }

    #[test]
    fn unreadable_is_not_empty_and_a_clean_summary_is() {
        assert!(!LossSummary::unreadable().is_empty());
        assert!(LossSummary::bounded(WorktreeLoss::default()).is_empty());
    }
}
