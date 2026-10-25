//! The jump-to field's review entries (#1274, contract J1): the open review's files and comments
//! as quick-switcher candidates, with what each row shows. Ranking stays in
//! [`crate::quick_switch::filter`]. Contract: `specs/1274-review-keyboard-map/contracts/jump-to.md`.

use std::collections::HashMap;

use super::comment::{CommentId, CommentState, ReviewComment};
use super::navigate::{comment_order, CommentFact, NavFile};
use super::RelPath;
use crate::quick_switch::{Candidate, EntryKey};

/// What a review row of the jump-to field shows besides its name (FR-016).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReviewEntry {
    /// A file of the review.
    File {
        /// Its path (the row shows its directory).
        path: RelPath,
        /// Whether it is marked reviewed.
        reviewed: bool,
    },
    /// A comment of the review's shown source.
    Comment {
        /// Its identity.
        id: CommentId,
        /// Draft, sent or reported.
        state: CommentState,
        /// Its file.
        path: RelPath,
        /// Its first line.
        line: u32,
        /// The first line of its text.
        first_line: String,
    },
}

/// The review's entries for the jump-to field (contract J1): `files` in file order, then
/// `comments` in comment order ([`super::navigate::comment_order`]), each with what its row shows.
pub fn review_candidates(
    files: &[RelPath],
    is_reviewed: impl Fn(&RelPath) -> bool,
    comments: &[ReviewComment],
) -> Vec<(Candidate, ReviewEntry)> {
    let file_rows = files.iter().map(|path| {
        let text = path.as_str();
        let candidate = Candidate {
            key: EntryKey::ReviewFile(path.clone()),
            name: text.rsplit('/').next().unwrap_or(text).to_owned(),
            detail: Some(text.to_owned()),
            parent: None,
            selectable: true,
        };
        let entry = ReviewEntry::File {
            path: path.clone(),
            reviewed: is_reviewed(path),
        };
        (candidate, entry)
    });
    let order: Vec<NavFile> = files
        .iter()
        .map(|path| NavFile {
            path: path.clone(),
            has_hunks: false,
        })
        .collect();
    let facts: Vec<CommentFact> = comments
        .iter()
        .map(|c| CommentFact {
            id: c.id,
            path: c.path.clone(),
            line: c.range.start(),
            created: c.created,
            in_diff: true,
        })
        .collect();
    let by_id: HashMap<CommentId, &ReviewComment> = comments.iter().map(|c| (c.id, c)).collect();
    let comment_rows = comment_order(&facts, &order).into_iter().map(|fact| {
        let comment = by_id[&fact.id];
        let candidate = Candidate {
            key: EntryKey::ReviewComment(comment.id),
            name: comment.text.lines().collect::<Vec<_>>().join(" "),
            detail: Some(comment.path.as_str().to_owned()),
            parent: None,
            selectable: true,
        };
        let entry = ReviewEntry::Comment {
            id: comment.id,
            state: comment.state.clone(),
            path: comment.path.clone(),
            line: fact.line,
            first_line: comment.text.lines().next().unwrap_or_default().to_owned(),
        };
        (candidate, entry)
    });
    file_rows.chain(comment_rows).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::quick_switch::filter;
    use crate::review::{LineRange, Side};
    use crate::typeahead::Query;
    use uuid::Uuid;

    fn path(p: &str) -> RelPath {
        RelPath::from_native(p).expect("a relative test path")
    }

    fn comment(n: u128, p: &str, line: u32, text: &str, state: CommentState) -> ReviewComment {
        ReviewComment {
            id: CommentId(Uuid::from_u128(n)),
            path: path(p),
            side: Side::New,
            range: LineRange::new(line, line).expect("a line"),
            quote: vec![String::new()],
            text: text.to_owned(),
            state,
            created: 1,
            pr: None,
        }
    }

    #[test]
    fn a_file_entry_is_named_by_its_file_and_detailed_by_its_path() {
        let files = [path("daemon/tests.rs"), path("README.md")];
        let rows = review_candidates(&files, |p| p.as_str() == "README.md", &[]);
        let shown: Vec<_> = rows
            .iter()
            .map(|(c, _)| {
                (
                    c.key.clone(),
                    c.name.as_str(),
                    c.detail.as_deref(),
                    c.selectable,
                )
            })
            .collect();
        assert_eq!(
            shown,
            vec![
                (
                    EntryKey::ReviewFile(path("daemon/tests.rs")),
                    "tests.rs",
                    Some("daemon/tests.rs"),
                    true
                ),
                (
                    EntryKey::ReviewFile(path("README.md")),
                    "README.md",
                    Some("README.md"),
                    true
                ),
            ],
            "J1: name = file name, detail = full path, in file order"
        );
        assert_eq!(
            rows[0].1,
            ReviewEntry::File {
                path: path("daemon/tests.rs"),
                reviewed: false
            }
        );
        assert_eq!(
            rows[1].1,
            ReviewEntry::File {
                path: path("README.md"),
                reviewed: true
            },
            "FR-016: the row shows the reviewed state"
        );
    }

    #[test]
    fn comment_entries_follow_the_files_in_comment_order() {
        let files = [path("a.rs"), path("c.rs")];
        let comments = [
            comment(1, "c.rs", 7, "rename this", CommentState::Draft),
            comment(
                2,
                "a.rs",
                20,
                "why\nnot a loop?",
                CommentState::Sent { at: 5 },
            ),
            comment(3, "a.rs", 3, "typo", CommentState::Draft),
        ];
        let rows = review_candidates(&files, |_| false, &comments);
        let names: Vec<_> = rows.iter().map(|(c, _)| c.name.as_str()).collect();
        assert_eq!(
            names,
            vec!["a.rs", "c.rs", "typo", "why not a loop?", "rename this"],
            "J1: files in file order, then comments in comment order; lines joined by spaces"
        );
        let (candidate, entry) = &rows[3];
        assert_eq!(
            candidate.key,
            EntryKey::ReviewComment(CommentId(Uuid::from_u128(2)))
        );
        assert_eq!(
            candidate.detail.as_deref(),
            Some("a.rs"),
            "detail = its file path"
        );
        assert_eq!(
            *entry,
            ReviewEntry::Comment {
                id: CommentId(Uuid::from_u128(2)),
                state: CommentState::Sent { at: 5 },
                path: path("a.rs"),
                line: 20,
                first_line: "why".to_owned(),
            },
            "FR-016: status, file and line, and the first line of its text"
        );
    }

    #[test]
    fn four_typed_characters_put_any_of_200_files_in_the_first_three_rows() {
        const WORDS: [&str; 10] = [
            "alpha", "bravo", "delta", "gamma", "kilo", "lima", "omega", "sigma", "tango", "zulu",
        ];
        let files: Vec<RelPath> = (0..200)
            .map(|i| path(&format!("crate{}/src/{}_{i:03}.rs", i % 7, WORDS[i % 10])))
            .collect();
        let rows = review_candidates(&files, |_| false, &[]);
        let candidates: Vec<_> = rows.into_iter().map(|(c, _)| c).collect();
        for (i, file) in files.iter().enumerate() {
            let query = format!("_{i:03}");
            let ranked = filter(&candidates, &Query::new(&query));
            let at = ranked
                .iter()
                .position(|r| candidates[r.index].key == EntryKey::ReviewFile(file.clone()));
            assert!(
                at.is_some_and(|at| at < 3),
                "SC-005: `{query}` puts {file} at row {at:?}, more than 2 Up/Down presses away"
            );
        }
    }
}
