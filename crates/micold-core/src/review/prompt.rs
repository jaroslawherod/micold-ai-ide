//! The review batch (feature 946, contracts/batch-format.md; was 482's review prompt): the one
//! text a send types into a session, built from an entry's Draft comments. Pure and deterministic,
//! so the same comments give the same bytes on every platform and build (FR-012). Its layout is
//! documented in the user guide's "The review message" and changes only with it (FR-013).

use super::comment::{CommentId, ReviewComment};
use super::limits::MAX_QUOTED_LINES;
use super::Side;
use crate::pull_request::BranchPrState;
use std::collections::BTreeSet;

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

/// What a pull request's batch says about the pull request (feature 1149, data-model § `PrContext`;
/// the batch itself is `build_pr`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrContext {
    /// The pull request's number.
    pub number: u64,
    /// Its title, any text.
    pub title: String,
    /// Its head branch.
    pub head_branch: String,
    /// Open, draft, merged or closed.
    pub state: BranchPrState,
    /// The head commit the list shows now.
    pub current_head: String,
    /// The entry's checked-out branch; `None` when detached or unknown.
    pub checked_out: Option<String>,
    /// Recorded commits that are not in the worktree.
    pub unreadable: Vec<String>,
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

    blocks.extend(comment_blocks(comments, outdated, false));
    blocks.join("\n\n")
}

/// The pull request's batch (feature 1149, contracts/batch-format.md P-1 to P-4): a preamble
/// naming the pull request and head branch, the apply, commit and push request, the notes that
/// apply, then one block per comment as [`build`] makes them. It never asks for anything to be
/// posted. The title is flattened here, so any text is safe.
pub fn build_pr(pr: &PrContext, comments: &[ReviewComment], outdated: &[CommentId]) -> String {
    let state = match pr.state {
        BranchPrState::Open | BranchPrState::Draft => "",
        BranchPrState::Merged => ", which is merged",
        BranchPrState::Closed => ", which is closed",
    };
    let head = &pr.head_branch;
    let mut blocks = vec![
        format!(
            "Please work through these review comments on pull request #{} \"{}\" (head branch \
             {head}){state}. Each block below is one comment: its reference, the file (relative to \
             the worktree root), the lines it is about, the code on those lines at the pull \
             request's head commit when the comment was written, and the comment itself.",
            pr.number,
            flatten_title(&pr.title),
        ),
        format!(
            "For each comment, either make the change it asks for, or explain why you are not \
             making it. Commit the changes you make and push them to the branch {head}. Then \
             report each outcome by the comment's reference with the {REPORT_TOOL} tool of the \
             micold tool server: outcome \"applied\" when you made the change (a reply is \
             optional), or \"answered\" with your explanation as the reply. If you cannot use that \
             tool, answer each comment by its reference in your reply instead."
        ),
    ];
    if pr.checked_out.as_deref() != Some(head.as_str()) {
        blocks.push(format!(
            "This worktree does not have {head} checked out. Switch to {head} before changing \
             anything."
        ));
    }
    let recorded: BTreeSet<&str> = comments
        .iter()
        .filter_map(|comment| comment.pr.as_ref())
        .map(|anchor| anchor.head_commit.as_str())
        .collect();
    if recorded
        .iter()
        .any(|commit| !commit.eq_ignore_ascii_case(&pr.current_head))
    {
        blocks.push(format!(
            "The comments were written against commit {}; the pull request's head is now {}, so \
             the code may have moved. Find it by the quoted code.",
            recorded.iter().copied().collect::<Vec<_>>().join(", "),
            pr.current_head
        ));
    }
    if !pr.unreadable.is_empty() {
        blocks.push(format!(
            "Commit {} is not available in this worktree; the code is quoted as recorded.",
            pr.unreadable.join(", ")
        ));
    }
    blocks.extend(comment_blocks(comments, outdated, recorded.len() > 1));
    blocks.join("\n\n")
}

/// `title` on one line: control characters become spaces, at most 200 characters, cut on a
/// character boundary (P-1).
pub fn flatten_title(title: &str) -> String {
    title
        .chars()
        .map(|c| {
            if c.is_control() || matches!(c, '\u{2028}' | '\u{2029}') {
                ' '
            } else {
                c
            }
        })
        .take(200)
        .collect()
}

/// One block per comment in path, line, side, creation and id order. `with_commit` adds each
/// block's `Written against commit` line (a batch holding more than one recorded commit).
fn comment_blocks(
    comments: &[ReviewComment],
    outdated: &[CommentId],
    with_commit: bool,
) -> Vec<String> {
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
    ordered
        .into_iter()
        .map(|comment| {
            let commit = with_commit
                .then(|| {
                    comment
                        .pr
                        .as_ref()
                        .map(|anchor| anchor.head_commit.as_str())
                })
                .flatten();
            comment_block(comment, outdated.contains(&comment.id), commit)
        })
        .collect()
}

/// One comment: its heading, the outdated note, the fenced quote and its text.
fn comment_block(comment: &ReviewComment, outdated: bool, commit: Option<&str>) -> String {
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
    if let Some(commit) = commit {
        block.push_str(&format!("Written against commit {commit}.\n"));
    }
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
            pr: None,
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

    #[test]
    fn files_are_in_bytewise_path_order() {
        let comments = [
            comment(1, "b", Side::New, 1, 1, &["x"], "in b", 1),
            comment(2, "B", Side::New, 1, 1, &["x"], "in B", 1),
            comment(3, "a/z", Side::New, 1, 1, &["x"], "in a/z", 1),
        ];
        let prompt = build(EntryKind::Worktree, &comments, &[]);
        let at = |needle: &str| prompt.find(needle).expect(needle);
        assert!(
            at(": B, line") < at(": a/z, line") && at(": a/z, line") < at(": b, line"),
            "{prompt}"
        );
    }

    // --- The pull request's batch (feature 1149, contracts/batch-format.md P-1 to P-5) ---

    fn sha(digit: char) -> String {
        digit.to_string().repeat(40)
    }

    fn on_pr(mut comment: ReviewComment, commit: &str) -> ReviewComment {
        comment.pr = Some(crate::review::comment::PrAnchor {
            number: 12,
            head_branch: "feat/retry".to_owned(),
            head_commit: commit.to_owned(),
        });
        comment
    }

    fn pr_context(state: BranchPrState) -> PrContext {
        PrContext {
            number: 12,
            title: "Add retry".to_owned(),
            head_branch: "feat/retry".to_owned(),
            state,
            current_head: sha('2'),
            checked_out: Some("feat/retry".to_owned()),
            unreadable: Vec::new(),
        }
    }

    fn pr_batch(pr: &PrContext, comments: &[ReviewComment]) -> String {
        build_pr(pr, comments, &[])
    }

    fn pr_comment(n: u128, commit: &str) -> ReviewComment {
        on_pr(one(n, "src/a.rs", Side::New, n as u32, n as u64), commit)
    }

    const PR_PREAMBLE_1: &str = "Please work through these review comments on pull request #12 \"Add retry\" (head branch feat/retry). Each block below is one comment: its reference, the file (relative to the worktree root), the lines it is about, the code on those lines at the pull request's head commit when the comment was written, and the comment itself.";
    const PR_PREAMBLE_2: &str = "For each comment, either make the change it asks for, or explain why you are not making it. Commit the changes you make and push them to the branch feat/retry. Then report each outcome by the comment's reference with the report_review_comment tool of the micold tool server: outcome \"applied\" when you made the change (a reply is optional), or \"answered\" with your explanation as the reply. If you cannot use that tool, answer each comment by its reference in your reply instead.";

    /// P-5: the documented bytes; they change only together with "The review message" in
    /// `docs/user-guide/reviewing-changes.md` (FR-017).
    #[test]
    fn p1_pr_example_gives_the_exact_bytes() {
        let mut pr = pr_context(BranchPrState::Open);
        pr.checked_out = Some("main".to_owned());
        let comment = on_pr(
            comment(
                10,
                "src/a.rs",
                Side::New,
                7,
                7,
                &["let x = 1;"],
                "Name this.",
                1,
            ),
            &sha('1'),
        );
        let expected = format!(
            "{PR_PREAMBLE_1}\n\n{PR_PREAMBLE_2}\n\n\
             This worktree does not have feat/retry checked out. Switch to feat/retry before changing anything.\n\n\
             The comments were written against commit {}; the pull request's head is now {}, so the code may have moved. Find it by the quoted code.\n\n\
             ### Comment C-0000000a: src/a.rs, line 7 (current)\n```\nlet x = 1;\n```\nName this.",
            sha('1'),
            sha('2')
        );
        assert_eq!(build_pr(&pr, &[comment], &[]), expected);
    }

    #[test]
    fn the_state_is_named_only_for_merged_and_closed() {
        let state_of = |state| {
            let batch = pr_batch(&pr_context(state), &[pr_comment(1, &sha('2'))]);
            batch.lines().next().expect("a preamble").to_owned()
        };
        for open in [BranchPrState::Open, BranchPrState::Draft] {
            assert!(state_of(open).contains("(head branch feat/retry). Each"));
        }
        assert!(state_of(BranchPrState::Merged)
            .contains("(head branch feat/retry), which is merged. Each"));
        assert!(state_of(BranchPrState::Closed)
            .contains("(head branch feat/retry), which is closed. Each"));
    }

    #[test]
    fn a_head_that_has_not_moved_and_the_branch_checked_out_add_no_note() {
        let batch = pr_batch(
            &pr_context(BranchPrState::Open),
            &[pr_comment(1, &sha('2'))],
        );
        assert_eq!(
            batch.split("\n\n").count(),
            3,
            "two preambles and one block: {batch}"
        );
        assert!(!batch.contains("checked out") && !batch.contains("written against"));
    }

    #[test]
    fn p3_note_1_another_branch_or_a_detached_head_says_to_switch() {
        for checked_out in [Some("main".to_owned()), None] {
            let mut pr = pr_context(BranchPrState::Open);
            pr.checked_out = checked_out;
            let batch = pr_batch(&pr, &[pr_comment(1, &sha('2'))]);
            assert!(
                batch.contains(
                    "\n\nThis worktree does not have feat/retry checked out. Switch to feat/retry before changing anything.\n\n### Comment"
                ),
                "{batch}"
            );
        }
    }

    #[test]
    fn p3_note_3_names_the_unreadable_commits_after_the_head_note() {
        let mut pr = pr_context(BranchPrState::Open);
        pr.unreadable = vec![sha('1'), sha('3')];
        let batch = pr_batch(&pr, &[pr_comment(1, &sha('1'))]);
        assert!(
            batch.contains(&format!(
                "Find it by the quoted code.\n\nCommit {}, {} is not available in this worktree; the code is quoted as recorded.\n\n### Comment",
                sha('1'),
                sha('3')
            )),
            "{batch}"
        );
    }

    #[test]
    fn two_recorded_commits_give_every_block_its_own_line_and_one_sorted_note() {
        let comments = [
            pr_comment(1, &sha('3')),
            pr_comment(2, &sha('1')),
            pr_comment(3, &sha('3')),
        ];
        let batch = pr_batch(&pr_context(BranchPrState::Open), &comments);
        assert!(
            batch.contains(&format!(
                "written against commit {}, {};",
                sha('1'),
                sha('3')
            )),
            "{batch}"
        );
        assert!(batch.contains(&format!(
            "### Comment C-00000002: src/a.rs, line 2 (current)\nWritten against commit {}.\n```",
            sha('1')
        )));
        assert_eq!(batch.matches("Written against commit").count(), 3);
    }

    #[test]
    fn one_recorded_commit_adds_no_per_block_line() {
        let batch = pr_batch(
            &pr_context(BranchPrState::Open),
            &[pr_comment(1, &sha('1')), pr_comment(2, &sha('1'))],
        );
        assert!(!batch.contains("Written against"), "{batch}");
    }

    #[test]
    fn the_title_is_flattened_to_one_line_of_at_most_200_characters() {
        let mut pr = pr_context(BranchPrState::Open);
        pr.title = "a\nb\x1bc".to_owned();
        let first = pr_batch(&pr, &[pr_comment(1, &sha('2'))]);
        assert!(first.starts_with(
            "Please work through these review comments on pull request #12 \"a b c\" (head"
        ));
        pr.title = "é".repeat(300);
        let long = pr_batch(&pr, &[pr_comment(1, &sha('2'))]);
        let title = long.split('"').nth(1).expect("quoted title");
        assert_eq!(title.chars().count(), 200);
        assert_eq!(flatten_title("tab\there"), "tab here");
        assert_eq!(
            flatten_title("a\u{2028}b\u{2029}c"),
            "a b c",
            "line separators too"
        );
    }

    #[test]
    fn no_variant_asks_for_a_post_or_names_github() {
        for state in [
            BranchPrState::Open,
            BranchPrState::Draft,
            BranchPrState::Merged,
            BranchPrState::Closed,
        ] {
            for checked_out in [Some("feat/retry".to_owned()), None] {
                let mut pr = pr_context(state);
                pr.checked_out = checked_out;
                pr.unreadable = vec![sha('1')];
                let batch = pr_batch(&pr, &[pr_comment(1, &sha('1')), pr_comment(2, &sha('3'))]);
                let preamble = batch
                    .split("### Comment")
                    .next()
                    .unwrap_or_default()
                    .to_lowercase();
                assert!(
                    !preamble.contains("post") && !preamble.contains("github"),
                    "{preamble}"
                );
            }
        }
    }

    #[test]
    fn a_pr_batch_built_twice_has_equal_bytes() {
        let comments = [pr_comment(2, &sha('1')), pr_comment(1, &sha('3'))];
        let pr = pr_context(BranchPrState::Merged);
        assert_eq!(pr_batch(&pr, &comments), pr_batch(&pr, &comments));
    }
}
