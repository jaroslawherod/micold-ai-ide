//! The review prompt (feature 482, contracts/review-prompt.md): the one text a send types into a
//! session, built from an entry's pending comments. Pure and deterministic, so the same comments
//! give the same bytes on every platform (FR-015, SC-006).

use super::comment::{CommentId, ReviewComment};
use super::limits::MAX_QUOTED_LINES;
use super::Side;

/// Which kind of entry the comments belong to; it decides the prompt's wording.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntryKind {
    /// A worktree of the project.
    Worktree,
    /// The Default entry: the project root.
    Default,
}

/// The prompt holding `comments` (the snapshot of pending comments), grouped by file and in line
/// order; a comment in `outdated` gets the outdated suffix (FR-013).
pub fn build(entry: EntryKind, comments: &[ReviewComment], outdated: &[CommentId]) -> String {
    let (place, root) = match entry {
        EntryKind::Worktree => ("this worktree", "the worktree root"),
        EntryKind::Default => ("the project root", "the project root"),
    };
    let mut blocks = vec![format!(
        "Please address these review comments on the changes in {place}. Each one names a file \
         (relative to {root}), the lines it is about, and the code on those lines when the comment \
         was written."
    )];

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
    let mut file = None;
    for comment in ordered {
        if file != Some(&comment.path) {
            file = Some(&comment.path);
            blocks.push(format!("## {}", comment.path));
        }
        blocks.push(comment_block(comment, outdated.contains(&comment.id)));
    }
    blocks.join("\n\n")
}

/// One comment: its heading, the fenced quote and its text.
fn comment_block(comment: &ReviewComment, outdated: bool) -> String {
    let range = comment.range;
    let lines = if range.start() == range.end() {
        format!("Line {}", range.start())
    } else {
        format!("Lines {}-{}", range.start(), range.end())
    };
    let side = match comment.side {
        Side::New => "(current)".to_owned(),
        Side::Old => format!("(removed; {} of the base version)", lines.to_lowercase()),
    };
    let suffix = if outdated {
        " - the file has changed since; quoted as written"
    } else {
        ""
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

    let mut block = format!("### {lines} {side}{suffix}\n{fence}\n");
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
    use uuid::Uuid;

    fn id(n: u128) -> CommentId {
        CommentId(Uuid::from_u128(n))
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
            state: CommentState::Pending,
            created,
        }
    }

    const INTRO_WORKTREE: &str = "Please address these review comments on the changes in this \
        worktree. Each one names a file (relative to the worktree root), the lines it is about, \
        and the code on those lines when the comment was written.";

    #[test]
    fn p1_two_files_and_three_comments_give_the_exact_prompt() {
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
        let expected = format!(
            "{INTRO_WORKTREE}\n\
             \n\
             ## src/a.rs\n\
             \n\
             ### Line 3 (removed; line 3 of the base version)\n\
             ```\n\
             use std::io;\n\
             ```\n\
             Keep this import.\n\
             \n\
             ### Lines 12-14 (current)\n\
             ```\n\
             fn a() {{\n    b.clone()\n}}\n\
             ```\n\
             Why clone here?\n\
             \n\
             ## src/b.rs\n\
             \n\
             ### Line 7 (current)\n\
             ```\n\
             let x = 1;\n\
             ```\n\
             Name this."
        );
        assert_eq!(build(EntryKind::Worktree, &comments, &[]), expected);
    }

    #[test]
    fn p2_a_removed_range_says_removed_and_numbers_it_in_the_base_version() {
        let comments = [comment(
            1,
            "lib.rs",
            Side::Old,
            40,
            42,
            &["a", "b", "c"],
            "Why drop these?",
            1,
        )];
        let prompt = build(EntryKind::Worktree, &comments, &[]);
        assert!(
            prompt.contains("### Lines 40-42 (removed; lines 40-42 of the base version)\n```\na\nb\nc\n```\nWhy drop these?"),
            "a removed range is named as removed, with its base line numbers (US2 s4): {prompt}"
        );
    }

    fn numbered(count: usize) -> Vec<String> {
        (1..=count).map(|n| format!("line {n}")).collect()
    }

    #[test]
    fn p3_a_quote_over_fifty_lines_shows_first_and_last_and_fifty_are_kept_whole() {
        let long = numbered(51);
        let long: Vec<&str> = long.iter().map(String::as_str).collect();
        let prompt = build(
            EntryKind::Worktree,
            &[comment(1, "a", Side::New, 1, 51, &long, "Too long.", 1)],
            &[],
        );
        assert!(
            prompt.contains(
                "### Lines 1-51 (current)\n```\nline 1\n... 49 lines not shown ...\nline 51\n```\nToo long."
            ),
            "51 lines elide to first, marker, last; the heading keeps the full range: {prompt}"
        );

        let fifty = numbered(50);
        let fifty: Vec<&str> = fifty.iter().map(String::as_str).collect();
        let prompt = build(
            EntryKind::Worktree,
            &[comment(1, "a", Side::New, 1, 50, &fifty, "Fine.", 1)],
            &[],
        );
        assert!(
            prompt.contains(&format!("```\n{}\n```\nFine.", fifty.join("\n"))),
            "50 lines are quoted whole: {prompt}"
        );
        assert!(!prompt.contains("not shown"), "nothing elided at 50 lines");
    }

    #[test]
    fn p4_a_quote_holding_three_backticks_gets_a_four_backtick_fence() {
        let prompt = build(
            EntryKind::Worktree,
            &[comment(
                1,
                "notes.md",
                Side::New,
                2,
                2,
                &["```rust"],
                "Tag.",
                1,
            )],
            &[],
        );
        assert!(
            prompt.contains("### Line 2 (current)\n````\n```rust\n````\nTag."),
            "the fence is one backtick longer than any run inside: {prompt}"
        );
    }

    #[test]
    fn p5_carriage_returns_become_line_feeds_and_none_is_left() {
        let prompt = build(
            EntryKind::Worktree,
            &[comment(
                1,
                "a.txt",
                Side::New,
                1,
                1,
                &["x\r"],
                "one\r\ntwo\rthree",
                1,
            )],
            &[],
        );
        assert!(
            prompt.ends_with("```\nx\n```\none\ntwo\nthree"),
            "CRLF and CR in the text become LF: {prompt:?}"
        );
        assert!(!prompt.contains('\r'), "no `\\r` anywhere in the prompt");
    }

    #[test]
    fn p6_ties_order_current_before_removed_then_creation_time_then_id() {
        let comments = [
            comment(4, "f", Side::New, 5, 5, &["n"], "new later", 20),
            comment(3, "f", Side::Old, 5, 5, &["o"], "old", 1),
            comment(
                2,
                "f",
                Side::New,
                5,
                5,
                &["n"],
                "new same time, larger id",
                10,
            ),
            comment(
                1,
                "f",
                Side::New,
                5,
                5,
                &["n"],
                "new same time, smaller id",
                10,
            ),
        ];
        let prompt = build(EntryKind::Worktree, &comments, &[]);
        let at = |needle: &str| prompt.find(needle).expect(needle);
        assert!(at("smaller id") < at("larger id"), "same time: id order");
        assert!(
            at("larger id") < at("new later"),
            "same side: creation time"
        );
        assert!(
            at("new later") < at("\nold"),
            "same start line: current before removed"
        );
    }

    #[test]
    fn p7_the_default_entry_names_the_project_root() {
        let prompt = build(
            EntryKind::Default,
            &[comment(1, "a", Side::New, 1, 1, &["x"], "t", 1)],
            &[],
        );
        assert!(
            prompt.starts_with(
                "Please address these review comments on the changes in the project root. Each \
                 one names a file (relative to the project root), the lines it is about, and the \
                 code on those lines when the comment was written.\n\n## a\n"
            ),
            "{prompt}"
        );
    }

    #[test]
    fn p8_an_outdated_comment_heading_says_the_file_has_changed() {
        let comments = [
            comment(1, "a", Side::New, 1, 1, &["x"], "stale", 1),
            comment(2, "a", Side::New, 2, 2, &["y"], "fresh", 2),
        ];
        let prompt = build(EntryKind::Worktree, &comments, &[id(1)]);
        assert!(
            prompt
                .contains("### Line 1 (current) - the file has changed since; quoted as written\n"),
            "{prompt}"
        );
        assert!(
            prompt.contains("### Line 2 (current)\n"),
            "a comment not in `outdated` has no suffix: {prompt}"
        );
    }

    #[test]
    fn p9_twenty_comments_across_five_files_each_appear_exactly_once() {
        let mut comments = Vec::new();
        for n in 0..20u32 {
            let path = format!("dir/file{}.rs", n % 5);
            let text = format!("comment number {n:02} end");
            let quote = format!("code {n:02}");
            comments.push(comment(
                u128::from(n) + 1,
                &path,
                Side::New,
                n + 1,
                n + 1,
                &[quote.as_str()],
                &text,
                u64::from(n),
            ));
        }
        let prompt = build(EntryKind::Worktree, &comments, &[]);
        for n in 0..20u32 {
            let text = format!("comment number {n:02} end");
            assert_eq!(
                prompt.matches(&text).count(),
                1,
                "{text} appears once (SC-002)"
            );
            let quote = format!("code {n:02}\n");
            assert_eq!(prompt.matches(&quote).count(), 1, "its quote appears once");
        }
        for file in 0..5 {
            assert_eq!(
                prompt.matches(&format!("## dir/file{file}.rs\n")).count(),
                1,
                "one section per file"
            );
        }
    }

    #[test]
    fn p10_paths_with_spaces_and_non_ascii_are_kept_verbatim() {
        let prompt = build(
            EntryKind::Worktree,
            &[comment(
                1,
                "docs/my notes/żółw ü.md",
                Side::New,
                1,
                1,
                &["x"],
                "t",
                1,
            )],
            &[],
        );
        assert!(
            prompt.contains("\n## docs/my notes/żółw ü.md\n"),
            "{prompt}"
        );
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
            at("## B\n") < at("## a/z\n") && at("## a/z\n") < at("## b\n"),
            "{prompt}"
        );
    }
}
