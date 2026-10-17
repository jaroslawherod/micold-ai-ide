//! The review batch (feature 946, contracts/batch-format.md; was 482's review prompt): the one
//! text a send types into a session, built from an entry's Draft comments. Pure and deterministic,
//! so the same comments give the same bytes on every platform and build (FR-012). Its layout is
//! documented in the user guide's "The review message" and changes only with it (FR-013).

use super::comment::{CommentId, ReviewComment};
use super::limits::MAX_QUOTED_LINES;
use super::Side;

/// The agent tool a session reports each comment's outcome with; the preamble names it and the
/// tool server's catalog lists it (contracts/batch-format.md, agent-tool.md).
pub const REPORT_TOOL: &str = "report_review_comment";

/// Which kind of entry the comments belong to; it decides the prompt's wording.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntryKind {
    /// A worktree of the project.
    Worktree,
    /// The Default entry: the project root.
    Default,
}

/// The batch holding `comments` (the snapshot of Draft comments): the preamble, then one block per
/// comment in path, line, side, creation and id order; a comment in `outdated` gets the outdated
/// note (contracts/batch-format.md, FR-010).
pub fn build(entry: EntryKind, comments: &[ReviewComment], outdated: &[CommentId]) -> String {
    let (place, root) = match entry {
        EntryKind::Worktree => ("this worktree", "the worktree root"),
        EntryKind::Default => ("the project root", "the project root"),
    };
    let mut blocks = vec![
        format!(
            "Please work through these review comments on the changes in {place}. Each block \
             below is one comment: its reference, the file (relative to {root}), the lines it is \
             about, the code on those lines when the comment was written, and the comment itself."
        ),
        format!(
            "For each comment, either make the change it asks for, or explain why you are not \
             making it. Then report each outcome by the comment's reference with the {REPORT_TOOL} \
             tool of the micold tool server: outcome \"applied\" when you made the change (a reply \
             is optional), or \"answered\" with your explanation as the reply. If you cannot use \
             that tool, answer each comment by its reference in your reply instead."
        ),
    ];

    let mut ordered: Vec<&ReviewComment> = comments.iter().collect();
    ordered.sort_by(|a, b| {
        (&a.path, a.range.start(), a.side, a.created, a.id).cmp(&(
            &b.path,
            b.range.start(),
            b.side,
            b.created,
            b.id,
        ))
    });
    blocks.extend(
        ordered
            .into_iter()
            .map(|comment| comment_block(comment, outdated.contains(&comment.id))),
    );
    blocks.join("\n\n")
}

/// One comment: its heading, the outdated note, the fenced quote and its text.
fn comment_block(comment: &ReviewComment, outdated: bool) -> String {
    let range = comment.range;
    let lines = if range.start() == range.end() {
        format!("line {}", range.start())
    } else {
        format!("lines {}-{}", range.start(), range.end())
    };
    let side = match comment.side {
        Side::New => "(current)",
        Side::Old => "(removed, base version)",
    };

    let quoted: Vec<String> = comment
        .quote
        .iter()
        .map(|line| normalised(line.strip_suffix('\r').unwrap_or(line)))
        .collect();
    let shown: Vec<String> = if quoted.len() > MAX_QUOTED_LINES {
        vec![
            quoted[0].clone(),
            format!("... {} lines not shown ...", quoted.len() - 2),
            quoted[quoted.len() - 1].clone(),
        ]
    } else {
        quoted
    };
    let longest = shown
        .iter()
        .map(|line| longest_backtick_run(line))
        .max()
        .unwrap_or(0);
    let fence = "`".repeat((longest + 1).max(3));

    let mut block = format!(
        "### Comment {}: {}, {lines} {side}\n",
        comment.reference(),
        comment.path
    );
    if outdated {
        block.push_str(
            "Note: the file has changed since this comment was written; the code is quoted as \
             written.\n",
        );
    }
    block.push_str(&fence);
    block.push('\n');
    for line in &shown {
        block.push_str(line);
        block.push('\n');
    }
    block.push_str(&fence);
    block.push('\n');
    block.push_str(&normalised(&comment.text));
    block
}

/// `\r\n` and a lone `\r` become `\n`, so no `\r` reaches the prompt (P5). A quoted line's
/// trailing `\r` (a CRLF file's line end) is dropped before this.
fn normalised(text: &str) -> String {
    text.replace("\r\n", "\n").replace('\r', "\n")
}

fn longest_backtick_run(line: &str) -> usize {
    line.split(|c| c != '`').map(str::len).max().unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::review::comment::CommentState;
    use crate::review::{LineRange, RelPath};
    use std::collections::BTreeSet;
    use uuid::Uuid;

    /// `0000000n-0000-0000-0000-000000000000`, whose reference is `C-0000000n`.
    fn id(n: u128) -> CommentId {
        CommentId(Uuid::from_u128(n << 96))
    }

    #[allow(clippy::too_many_arguments)]
    fn comment(
        n: u128,
        path: &str,
        side: Side,
        start: u32,
        end: u32,
        quote: &[&str],
        text: &str,
        created: u64,
    ) -> ReviewComment {
        ReviewComment {
            id: id(n),
            path: RelPath::from_git(path),
            side,
            range: LineRange::new(start, end).expect("a valid range"),
            quote: quote.iter().map(|line| (*line).to_owned()).collect(),
            text: text.to_owned(),
            state: CommentState::Draft,
            created,
        }
    }

    fn one(n: u128, path: &str, side: Side, start: u32, created: u64) -> ReviewComment {
        comment(
            n,
            path,
            side,
            start,
            start,
            &["x"],
            &format!("text {n}"),
            created,
        )
    }

    /// P1: contracts/batch-format.md's example, byte for byte.
    ///
    /// These bytes are the documented format: they change only together with "The review
    /// message" in `docs/user-guide/reviewing-changes.md` (FR-013), in the same pull request.
    #[test]
    fn p1_the_contract_example_gives_the_exact_bytes() {
        let comments = [
            comment(
                3,
                "src/b.rs",
                Side::New,
                7,
                7,
                &["let x = 1;"],
                "Name this.",
                30,
            ),
            comment(
                1,
                "src/a.rs",
                Side::New,
                12,
                14,
                &["fn a() {", "    b.clone()", "}"],
                "Why clone here?",
                10,
            ),
            comment(
                2,
                "src/a.rs",
                Side::Old,
                3,
                3,
                &["use std::io;"],
                "Keep this import.",
                20,
            ),
        ];
        let expected = "Please work through these review comments on the changes in this worktree. Each block below is one comment: its reference, the file (relative to the worktree root), the lines it is about, the code on those lines when the comment was written, and the comment itself.

For each comment, either make the change it asks for, or explain why you are not making it. Then report each outcome by the comment's reference with the report_review_comment tool of the micold tool server: outcome \"applied\" when you made the change (a reply is optional), or \"answered\" with your explanation as the reply. If you cannot use that tool, answer each comment by its reference in your reply instead.

### Comment C-00000002: src/a.rs, line 3 (removed, base version)
```
use std::io;
```
Keep this import.

### Comment C-00000001: src/a.rs, lines 12-14 (current)
```
fn a() {
    b.clone()
}
```
Why clone here?

### Comment C-00000003: src/b.rs, line 7 (current)
Note: the file has changed since this comment was written; the code is quoted as written.
```
let x = 1;
```
Name this.";
        assert_eq!(build(EntryKind::Worktree, &comments, &[id(3)]), expected);
    }

    #[test]
    fn p2_the_default_entry_names_the_project_root_twice() {
        let batch = build(EntryKind::Default, &[one(1, "a.rs", Side::New, 1, 1)], &[]);
        let first = batch.lines().next().expect("a preamble");
        assert!(
            first.contains("on the changes in the project root.")
                && first.contains("(relative to the project root)"),
            "{first}"
        );
        assert!(!batch.contains("worktree"), "{batch}");
    }

    fn numbered(count: usize) -> Vec<String> {
        (1..=count).map(|n| format!("line {n}")).collect()
    }

    fn quoting(count: usize) -> String {
        let lines = numbered(count);
        let lines: Vec<&str> = lines.iter().map(String::as_str).collect();
        let end = u32::try_from(count).expect("small");
        build(
            EntryKind::Worktree,
            &[comment(
                1,
                "a.rs",
                Side::New,
                1,
                end,
                &lines,
                "Split this.",
                1,
            )],
            &[],
        )
    }

    #[test]
    fn p3_a_quote_over_fifty_lines_shows_first_and_last_and_fifty_are_kept_whole() {
        assert!(
            quoting(51).ends_with(
                "### Comment C-00000001: a.rs, lines 1-51 (current)\n```\nline 1\n\
                 ... 49 lines not shown ...\nline 51\n```\nSplit this."
            ),
            "51 lines: first, elision, last; the heading keeps the full range"
        );
        let whole = quoting(50);
        assert!(
            whole.contains("line 25\nline 26\n"),
            "50 lines are quoted whole"
        );
        assert!(!whole.contains("not shown"), "{whole}");
    }

    #[test]
    fn p4_a_quote_holding_three_backticks_gets_a_four_backtick_fence() {
        let batch = build(
            EntryKind::Worktree,
            &[comment(
                1,
                "a.md",
                Side::New,
                2,
                2,
                &["```rust"],
                "Fence?",
                1,
            )],
            &[],
        );
        assert!(
            batch.ends_with("(current)\n````\n```rust\n````\nFence?"),
            "{batch}"
        );
    }

    #[test]
    fn p5_no_carriage_return_reaches_the_batch() {
        let batch = build(
            EntryKind::Worktree,
            &[comment(
                1,
                "a.rs",
                Side::New,
                1,
                2,
                &["one\r", "t\rwo"],
                "a\r\nb\rc",
                1,
            )],
            &[],
        );
        assert!(!batch.contains('\r'), "{batch:?}");
        assert!(
            batch.ends_with("```\none\nt\nwo\n```\na\nb\nc"),
            "{batch:?}"
        );
    }

    fn references(batch: &str) -> Vec<String> {
        batch
            .lines()
            .filter_map(|line| line.strip_prefix("### Comment "))
            .map(|rest| rest.split(':').next().unwrap_or_default().to_owned())
            .collect()
    }

    #[test]
    fn p6_ties_order_current_before_removed_then_creation_time_then_id() {
        let comments = [
            one(1, "a.rs", Side::Old, 5, 1),
            one(2, "a.rs", Side::New, 5, 9),
            one(3, "a.rs", Side::New, 5, 3),
            one(5, "a.rs", Side::New, 5, 3),
            one(4, "a.rs", Side::New, 5, 3),
        ];
        assert_eq!(
            references(&build(EntryKind::Worktree, &comments, &[])),
            [
                "C-00000003",
                "C-00000004",
                "C-00000005",
                "C-00000002",
                "C-00000001"
            ],
            "same path and start: current first; same side: earlier creation, then id"
        );
    }

    #[test]
    fn p7_twenty_comments_across_five_files_each_appear_exactly_once() {
        let comments: Vec<ReviewComment> = (1..=20u32)
            .map(|n| {
                let path = format!("dir/file{}.rs", n % 5);
                comment(
                    u128::from(n),
                    &path,
                    Side::New,
                    n,
                    n,
                    &["x"],
                    &format!("comment number {n}."),
                    u64::from(n),
                )
            })
            .collect();
        let batch = build(EntryKind::Worktree, &comments, &[]);
        let refs = references(&batch);
        assert_eq!(refs.len(), 20, "one block per comment (SC-002)");
        assert_eq!(
            refs.iter().collect::<BTreeSet<_>>().len(),
            20,
            "distinct references"
        );
        for n in 1..=20 {
            assert_eq!(batch.matches(&format!("comment number {n}.")).count(), 1);
        }
    }

    #[test]
    fn p8_built_twice_the_bytes_are_equal() {
        let comments = [
            one(2, "b.rs", Side::Old, 3, 2),
            one(1, "a.rs", Side::New, 9, 1),
        ];
        assert_eq!(
            build(EntryKind::Worktree, &comments, &[id(1)]),
            build(EntryKind::Worktree, &comments, &[id(1)])
        );
    }

    #[test]
    fn p9_the_preamble_names_the_report_tool() {
        assert_eq!(REPORT_TOOL, "report_review_comment");
        let batch = build(EntryKind::Worktree, &[one(1, "a.rs", Side::New, 1, 1)], &[]);
        let preamble = batch.split("### Comment").next().unwrap_or_default();
        assert_eq!(preamble.matches(REPORT_TOOL).count(), 1, "{preamble}");
    }

    #[test]
    fn paths_with_spaces_and_non_ascii_are_kept_verbatim() {
        let batch = build(
            EntryKind::Worktree,
            &[one(1, "docs/a b é.md", Side::New, 4, 1)],
            &[],
        );
        assert!(batch.contains("### Comment C-00000001: docs/a b é.md, line 4 (current)\n"));
    }

    #[test]
    fn there_is_no_trailing_line_break() {
        let batch = build(EntryKind::Worktree, &[one(1, "a.rs", Side::New, 1, 1)], &[]);
        assert!(batch.ends_with("text 1"), "{batch:?}");
    }
}
