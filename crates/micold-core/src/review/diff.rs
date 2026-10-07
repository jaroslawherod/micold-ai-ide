//! One file's diff, parsed from `git diff -U3` (research R1, R8; data-model § changed files and
//! diffs): hunks of numbered lines with their line endings stripped, or why there are none to show
//! (binary, not UTF-8, only the mode changed, over the size limits).
//!
//! Pure: [`parse_unified`] reads git's bytes, [`added_file`] stands in for git on an untracked file,
//! and [`unified_rows`] lays a diff out as the unified view's rows. The git half is
//! `review::git` (`GitCli::file_diff`).

use super::changes::{content_of, Content};

/// What a diff line is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LineKind {
    /// In both versions.
    Context,
    /// Only in the new version.
    Added,
    /// Only in the old version.
    Removed,
}

/// One line of a hunk. `text` has no line ending: a `\r\n` and a `\n` are both stripped, so a
/// change of line ending alone shows as a `Removed` + `Added` pair with the same text (Edge Cases).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiffLine {
    /// Context, added or removed.
    pub kind: LineKind,
    /// Its number in the old version; `None` for an added line.
    pub old: Option<u32>,
    /// Its number in the new version; `None` for a removed line.
    pub new: Option<u32>,
    /// The line, without its ending.
    pub text: String,
}

/// One `@@ -a,b +c,d @@` hunk.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Hunk {
    /// The first old line the hunk covers (0 when the old version is empty).
    pub old_start: u32,
    /// How many old lines it covers.
    pub old_len: u32,
    /// The first new line it covers (0 when the new version is empty).
    pub new_start: u32,
    /// How many new lines it covers.
    pub new_len: u32,
    /// The text git prints after the second `@@` (usually the enclosing function), trimmed.
    pub section: String,
    /// The lines, in order.
    pub lines: Vec<DiffLine>,
    /// The old version's last line in this hunk has no newline (`\ No newline at end of file`).
    pub old_no_newline: bool,
    /// The new version's last line in this hunk has no newline.
    pub new_no_newline: bool,
}

impl Hunk {
    /// The header as git writes it, without the section: `@@ -a,b +c,d @@`.
    pub fn header(&self) -> String {
        format!(
            "@@ -{},{} +{},{} @@",
            self.old_start, self.old_len, self.new_start, self.new_len
        )
    }
}

/// One file's diff, or why it has no lines to show.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileDiff {
    /// Text: its hunks (none when the content did not change, e.g. a pure rename).
    Text(Vec<Hunk>),
    /// Binary: listed, never rendered (FR-008, SC-005).
    Binary,
    /// A version is not valid UTF-8: listed, not rendered (FR-008).
    NotUtf8,
    /// Only the file mode changed.
    ModeOnly,
    /// Over the limits of R8: not read until the user asks (FR-009, US1 s7).
    TooLarge {
        /// Lines added.
        added: u32,
        /// Lines removed.
        removed: u32,
    },
}

/// One row of the unified layout.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnifiedRow<'a> {
    /// A hunk's header row.
    Header(&'a Hunk),
    /// A line.
    Line(&'a DiffLine),
}

/// The full text of one side's version, as lines without their endings (data-model `SideLines`):
/// what outdated checks and quotes read.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SideLines(Vec<String>);

impl SideLines {
    /// The lines of `bytes`, or `None` when they are not UTF-8 text.
    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        let text = std::str::from_utf8(bytes).ok()?;
        Some(Self(split_lines(text).map(str::to_owned).collect()))
    }

    /// Line `number` (1-based).
    pub fn line(&self, number: u32) -> Option<&str> {
        let index = usize::try_from(number).ok()?.checked_sub(1)?;
        self.0.get(index).map(String::as_str)
    }

    /// How many lines.
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// No lines (an empty file).
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

/// A file's diff with both versions' lines, as `GitCli::file_diff` reads it. A side is `None` when
/// that version does not exist (an added or a deleted file) or was not read (not text).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoadedDiff {
    /// The diff.
    pub diff: FileDiff,
    /// The old version's lines.
    pub old: Option<SideLines>,
    /// The new version's lines.
    pub new: Option<SideLines>,
}

/// The lines of `text` without their endings (`\n` or `\r\n`); a last line without a newline
/// counts, a trailing newline does not start another.
fn split_lines(text: &str) -> impl Iterator<Item = &str> {
    let body = text.strip_suffix('\n').unwrap_or(text);
    let lines = (!text.is_empty()).then(|| body.split('\n'));
    lines
        .into_iter()
        .flatten()
        .map(|line| line.strip_suffix('\r').unwrap_or(line))
}

/// `-a,b` or `+c,d` (the count defaults to 1).
fn range(part: &str) -> Option<(u32, u32)> {
    let (start, len) = match part[1..].split_once(',') {
        Some((start, len)) => (start, len.parse().ok()?),
        None => (&part[1..], 1),
    };
    Some((start.parse().ok()?, len))
}

/// `@@ -a,b +c,d @@ section` → a hunk with no lines yet.
fn hunk_header(line: &str) -> Option<Hunk> {
    let rest = line.strip_prefix("@@ ")?;
    let (ranges, section) = rest.split_once(" @@")?;
    let (old, new) = ranges.split_once(' ')?;
    if !old.starts_with('-') || !new.starts_with('+') {
        return None;
    }
    let (old_start, old_len) = range(old)?;
    let (new_start, new_len) = range(new)?;
    Some(Hunk {
        old_start,
        old_len,
        new_start,
        new_len,
        section: section.trim().to_owned(),
        lines: Vec::new(),
        old_no_newline: false,
        new_no_newline: false,
    })
}

/// Parse the output of `git diff -U3` for one file. Lines inside a hunk are read by its counts, so a
/// removed `-- x` (printed `--- x`) is a line and not a header.
pub fn parse_unified(raw: &[u8]) -> FileDiff {
    let Ok(text) = std::str::from_utf8(raw) else {
        return FileDiff::NotUtf8;
    };
    let mut hunks: Vec<Hunk> = Vec::new();
    let mut mode_changed = false;
    // Old and new lines the current hunk still has to read.
    let (mut old_left, mut new_left) = (0u32, 0u32);
    let (mut old_no, mut new_no) = (0u32, 0u32);
    for line in text.split('\n') {
        let line = line.strip_suffix('\r').unwrap_or(line);
        if let Some(hunk) = hunks.last_mut() {
            if line.starts_with('\\') {
                match hunk.lines.last().map(|l| l.kind) {
                    Some(LineKind::Removed) => hunk.old_no_newline = true,
                    Some(LineKind::Added) => hunk.new_no_newline = true,
                    Some(LineKind::Context) => {
                        hunk.old_no_newline = true;
                        hunk.new_no_newline = true;
                    }
                    None => {}
                }
                continue;
            }
            if old_left > 0 || new_left > 0 {
                let (kind, body) = match line.split_at_checked(1) {
                    Some(("-", body)) if old_left > 0 => (LineKind::Removed, body),
                    Some(("+", body)) if new_left > 0 => (LineKind::Added, body),
                    Some((" ", body)) => (LineKind::Context, body),
                    // Some tools trim the space of an empty context line.
                    _ if line.is_empty() => (LineKind::Context, ""),
                    _ => {
                        old_left = 0;
                        new_left = 0;
                        continue;
                    }
                };
                let (old, new) = match kind {
                    LineKind::Removed => (Some(old_no), None),
                    LineKind::Added => (None, Some(new_no)),
                    LineKind::Context => (Some(old_no), Some(new_no)),
                };
                if old.is_some() {
                    old_no += 1;
                    old_left = old_left.saturating_sub(1);
                }
                if new.is_some() {
                    new_no += 1;
                    new_left = new_left.saturating_sub(1);
                }
                hunk.lines.push(DiffLine {
                    kind,
                    old,
                    new,
                    text: body.to_owned(),
                });
                continue;
            }
        }
        if let Some(hunk) = hunk_header(line) {
            old_left = hunk.old_len;
            new_left = hunk.new_len;
            old_no = hunk.old_start;
            new_no = hunk.new_start;
            hunks.push(hunk);
        } else if hunks.is_empty() {
            if line.starts_with("Binary files ") && line.ends_with(" differ")
                || line == "GIT binary patch"
            {
                return FileDiff::Binary;
            }
            mode_changed |= line.starts_with("old mode ");
        }
    }
    if hunks.is_empty() && mode_changed {
        FileDiff::ModeOnly
    } else {
        FileDiff::Text(hunks)
    }
}

/// The diff of an untracked file with content `bytes`: every line added. A NUL byte makes it
/// binary and invalid UTF-8 not text, as `content_of` decides for the list.
pub fn added_file(bytes: &[u8]) -> FileDiff {
    match content_of(bytes).0 {
        Content::Binary => return FileDiff::Binary,
        Content::NotUtf8 => return FileDiff::NotUtf8,
        Content::Text => {}
    }
    let Ok(text) = std::str::from_utf8(bytes) else {
        return FileDiff::NotUtf8;
    };
    let lines: Vec<DiffLine> = split_lines(text)
        .zip(1u32..)
        .map(|(line, number)| DiffLine {
            kind: LineKind::Added,
            old: None,
            new: Some(number),
            text: line.to_owned(),
        })
        .collect();
    if lines.is_empty() {
        return FileDiff::Text(Vec::new());
    }
    let new_len = u32::try_from(lines.len()).unwrap_or(u32::MAX);
    FileDiff::Text(vec![Hunk {
        old_start: 0,
        old_len: 0,
        new_start: 1,
        new_len,
        section: String::new(),
        lines,
        old_no_newline: false,
        new_no_newline: !text.ends_with('\n'),
    }])
}

/// The unified layout's rows: each hunk's header, then its lines. None for a diff that is not text.
pub fn unified_rows(diff: &FileDiff) -> Vec<UnifiedRow<'_>> {
    let FileDiff::Text(hunks) = diff else {
        return Vec::new();
    };
    let mut rows = Vec::with_capacity(hunks.iter().map(|h| h.lines.len() + 1).sum());
    for hunk in hunks {
        rows.push(UnifiedRow::Header(hunk));
        rows.extend(hunk.lines.iter().map(UnifiedRow::Line));
    }
    rows
}

#[cfg(test)]
mod tests {
    use super::*;

    fn text(diff: FileDiff) -> Vec<Hunk> {
        match diff {
            FileDiff::Text(hunks) => hunks,
            other => panic!("expected a text diff, got {other:?}"),
        }
    }

    fn line(kind: LineKind, old: Option<u32>, new: Option<u32>, text: &str) -> DiffLine {
        DiffLine {
            kind,
            old,
            new,
            text: text.into(),
        }
    }

    /// Captured from git 2.43 (`git diff -U3 -- a.rs`): two hunks, a section after the header.
    const TWO_HUNKS: &str = concat!(
        "diff --git a/a.rs b/a.rs\n",
        "index 1111111..2222222 100644\n",
        "--- a/a.rs\n",
        "+++ b/a.rs\n",
        "@@ -1,3 +1,3 @@ fn main() {\n",
        " one\n",
        "-two\n",
        "+TWO\n",
        " three\n",
        "@@ -10,2 +10,3 @@\n",
        " ten\n",
        "+ten and a half\n",
        " eleven\n",
    );

    #[test]
    fn hunk_headers_and_lines_are_read_with_both_numbers() {
        let hunks = text(parse_unified(TWO_HUNKS.as_bytes()));
        assert_eq!(hunks.len(), 2, "two @@ headers, two hunks");
        let first = &hunks[0];
        assert_eq!(
            (
                first.old_start,
                first.old_len,
                first.new_start,
                first.new_len
            ),
            (1, 3, 1, 3)
        );
        assert_eq!(first.section, "fn main() {");
        assert_eq!(
            first.lines,
            vec![
                line(LineKind::Context, Some(1), Some(1), "one"),
                line(LineKind::Removed, Some(2), None, "two"),
                line(LineKind::Added, None, Some(2), "TWO"),
                line(LineKind::Context, Some(3), Some(3), "three"),
            ],
            "a removed line has only its old number, an added one only its new one"
        );
        let second = &hunks[1];
        assert_eq!(second.header(), "@@ -10,2 +10,3 @@");
        assert_eq!(
            second.lines,
            vec![
                line(LineKind::Context, Some(10), Some(10), "ten"),
                line(LineKind::Added, None, Some(11), "ten and a half"),
                line(LineKind::Context, Some(11), Some(12), "eleven"),
            ]
        );
    }

    #[test]
    fn a_header_without_counts_means_one_line() {
        let raw = "--- a/x\n+++ b/x\n@@ -4 +4 @@\n-a\n+b\n";
        let hunks = text(parse_unified(raw.as_bytes()));
        assert_eq!(
            (
                hunks[0].old_start,
                hunks[0].old_len,
                hunks[0].new_start,
                hunks[0].new_len
            ),
            (4, 1, 4, 1),
            "`-4` is `-4,1`"
        );
    }

    #[test]
    fn lines_that_look_like_headers_inside_a_hunk_are_lines() {
        // A removed line `-- x` is printed `--- x`; the hunk's counts say it is still a line.
        let raw = "--- a/x\n+++ b/x\n@@ -1,2 +1,1 @@\n--- x\n+++ y\n-z\n";
        let hunks = text(parse_unified(raw.as_bytes()));
        assert_eq!(
            hunks[0].lines,
            vec![
                line(LineKind::Removed, Some(1), None, "-- x"),
                line(LineKind::Added, None, Some(1), "++ y"),
                line(LineKind::Removed, Some(2), None, "z"),
            ]
        );
    }

    #[test]
    fn crlf_and_lf_are_stripped_and_an_ending_only_change_is_a_pair() {
        // LF → CRLF on one line, as git prints it: the new line carries the `\r`.
        let raw = "--- a/x\n+++ b/x\n@@ -1,2 +1,2 @@\n keep\r\n-same\n+same\r\n";
        let hunks = text(parse_unified(raw.as_bytes()));
        assert_eq!(
            hunks[0].lines,
            vec![
                line(LineKind::Context, Some(1), Some(1), "keep"),
                line(LineKind::Removed, Some(2), None, "same"),
                line(LineKind::Added, None, Some(2), "same"),
            ],
            "no `\\r` survives, and the line-ending change is still a removed + added pair"
        );
    }

    #[test]
    fn the_no_newline_marker_is_dropped_and_recorded_on_its_side() {
        let raw = "--- a/x\n+++ b/x\n@@ -1 +1 @@\n-old\n\\ No newline at end of file\n+new\n";
        let hunk = &text(parse_unified(raw.as_bytes()))[0];
        assert_eq!(hunk.lines.len(), 2, "the marker is not a line");
        assert!(
            hunk.old_no_newline,
            "it followed a removed line: the old side"
        );
        assert!(!hunk.new_no_newline);

        let raw = "--- a/x\n+++ b/x\n@@ -1 +1 @@\n same\n\\ No newline at end of file\n";
        let hunk = &text(parse_unified(raw.as_bytes()))[0];
        assert!(
            hunk.old_no_newline && hunk.new_no_newline,
            "after a context line it applies to both sides"
        );
    }

    #[test]
    fn a_binary_notice_gives_binary() {
        let raw = "diff --git a/logo.png b/logo.png\nindex 1..2 100644\nBinary files a/logo.png and b/logo.png differ\n";
        assert_eq!(parse_unified(raw.as_bytes()), FileDiff::Binary);
    }

    #[test]
    fn invalid_utf8_gives_not_utf8() {
        let mut raw = b"--- a/x\n+++ b/x\n@@ -1 +1 @@\n-caf".to_vec();
        raw.extend([0xE9, b'\n']);
        raw.extend(b"+cafe\n");
        assert_eq!(parse_unified(&raw), FileDiff::NotUtf8);
    }

    #[test]
    fn a_mode_only_change_gives_mode_only() {
        let raw = "diff --git a/run.sh b/run.sh\nold mode 100644\nnew mode 100755\n";
        assert_eq!(parse_unified(raw.as_bytes()), FileDiff::ModeOnly);
    }

    #[test]
    fn a_mode_change_with_content_is_text() {
        let raw = "diff --git a/run.sh b/run.sh\nold mode 100644\nnew mode 100755\nindex 1..2\n--- a/run.sh\n+++ b/run.sh\n@@ -1 +1 @@\n-a\n+b\n";
        assert_eq!(text(parse_unified(raw.as_bytes())).len(), 1);
    }

    #[test]
    fn an_added_file_is_all_added_lines() {
        let hunks = text(added_file(b"one\r\ntwo\nthree"));
        assert_eq!(hunks.len(), 1);
        let hunk = &hunks[0];
        assert_eq!(
            (hunk.old_start, hunk.old_len, hunk.new_start, hunk.new_len),
            (0, 0, 1, 3)
        );
        assert_eq!(
            hunk.lines,
            vec![
                line(LineKind::Added, None, Some(1), "one"),
                line(LineKind::Added, None, Some(2), "two"),
                line(LineKind::Added, None, Some(3), "three"),
            ]
        );
        assert!(
            hunk.new_no_newline,
            "the last line has no newline, as git would mark it"
        );
        assert_eq!(
            added_file(b""),
            FileDiff::Text(Vec::new()),
            "an empty file has no hunk"
        );
    }

    #[test]
    fn an_added_file_with_a_nul_byte_is_binary_and_invalid_utf8_is_not_text() {
        assert_eq!(added_file(b"PNG\0data"), FileDiff::Binary);
        assert_eq!(added_file(&[b'a', 0xFF, b'\n']), FileDiff::NotUtf8);
    }

    #[test]
    fn unified_rows_are_each_header_then_its_lines() {
        let diff = parse_unified(TWO_HUNKS.as_bytes());
        let rows = unified_rows(&diff);
        let shape: Vec<String> = rows
            .iter()
            .map(|row| match row {
                UnifiedRow::Header(hunk) => hunk.header(),
                UnifiedRow::Line(line) => line.text.clone(),
            })
            .collect();
        assert_eq!(
            shape,
            [
                "@@ -1,3 +1,3 @@",
                "one",
                "two",
                "TWO",
                "three",
                "@@ -10,2 +10,3 @@",
                "ten",
                "ten and a half",
                "eleven"
            ]
        );
        assert!(unified_rows(&FileDiff::Binary).is_empty());
    }

    #[test]
    fn side_lines_strip_endings_and_refuse_non_utf8() {
        let side = SideLines::from_bytes(b"a\r\nb\nc").expect("text");
        assert_eq!(
            (side.len(), side.line(1), side.line(3), side.line(4)),
            (3, Some("a"), Some("c"), None)
        );
        assert_eq!(side.line(0), None, "lines are numbered from 1");
        assert_eq!(SideLines::from_bytes(&[0xFF]), None);
    }
}
