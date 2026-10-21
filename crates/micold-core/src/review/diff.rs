//! One file's diff, parsed from `git diff -U3` (research R1, R8; data-model § changed files and
//! diffs): hunks of numbered lines with their line endings stripped, or why there are none to show
//! (binary, not UTF-8, only the mode changed, over the size limits).
//!
//! Pure: [`parse_unified`] reads git's bytes, [`added_file`] stands in for git on an untracked file,
//! and [`unified_rows`] lays a diff out as the unified view's rows. The git half is
//! `review::git` (`GitCli::file_diff`).

use super::anchor::Lines;
use super::changes::{content_of, Content};
#[cfg(test)]
use super::expand::Shaped;
use super::expand::{Gap, ShapedRef};
use super::Side;
use crate::tokens::Rgb;

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
    /// A run of unchanged lines: hidden (`open` false) or shown and collapsible.
    Gap {
        /// The run.
        gap: &'a Gap,
        /// Whether its lines are shown.
        open: bool,
    },
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

/// The lines of one side that a text diff's hunks number, and nothing else: what a pull request's
/// diff holds, since it carries no full file (1149 R4). A line outside every hunk reads as absent.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct HunkLines(std::collections::BTreeMap<u32, String>);

impl HunkLines {
    /// The lines `diff`'s hunks number on `side`; none for a diff that is not text.
    pub fn new(diff: &FileDiff, side: Side) -> Self {
        let FileDiff::Text(hunks) = diff else {
            return Self::default();
        };
        let numbered = hunks.iter().flat_map(|h| &h.lines).filter_map(|line| {
            let number = match side {
                Side::Old => line.old,
                Side::New => line.new,
            };
            number.map(|n| (n, line.text.clone()))
        });
        Self(numbered.collect())
    }
}

impl Lines for HunkLines {
    fn line(&self, number: u32) -> Option<&str> {
        self.0.get(&number).map(String::as_str)
    }

    fn len(&self) -> usize {
        self.0
            .last_key_value()
            .map_or(0, |(n, _)| usize::try_from(*n).unwrap_or(usize::MAX))
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
    /// Both versions' syntax colours (R10); empty until the client highlights them.
    pub spans: Spans,
}

/// One line's syntax colours: byte ranges of its text and their colours (R10).
pub type LineSpans = Vec<(std::ops::Range<usize>, Rgb)>;

/// One version's syntax colours in one scheme, by line number; empty for plain text.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SideSpans(pub Vec<LineSpans>);

impl SideSpans {
    /// The spans of line `number` (1-based); none past the end.
    pub fn line(&self, number: u32) -> &[(std::ops::Range<usize>, Rgb)] {
        usize::try_from(number)
            .ok()
            .and_then(|n| n.checked_sub(1))
            .and_then(|i| self.0.get(i))
            .map_or(&[], Vec::as_slice)
    }
}

/// Both versions' syntax colours in one scheme.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SchemeSpans {
    /// The old version's.
    pub old: SideSpans,
    /// The new version's.
    pub new: SideSpans,
}

/// A diff's syntax colours in both schemes, so a theme switch needs no new read (R10, US1 s8).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Spans {
    /// In the light scheme (`InspiredGitHub`).
    pub light: SchemeSpans,
    /// In the dark scheme (`Base16Ocean`).
    pub dark: SchemeSpans,
}

/// The lines of `text` without their endings (`\n` or `\r\n`); a last line without a newline
/// counts, a trailing newline does not start another.
pub(crate) fn split_lines(text: &str) -> impl Iterator<Item = &str> {
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

/// The hunks' lines at which an open run starts, with the run: the places a collapse row goes.
fn marks<'a>(shaped: ShapedRef<'a>, hunk: usize) -> Vec<(usize, &'a Gap)> {
    shaped
        .revealed
        .iter()
        .filter(|r| r.hunk == hunk)
        .map(|r| (r.line, &r.gap))
        .collect()
}

/// The hidden runs shown just before shaped hunk `hunk` (`hunks.len()` for the trailing one).
fn hidden_before<'a>(shaped: ShapedRef<'a>, hunk: usize) -> impl Iterator<Item = &'a Gap> {
    shaped.gaps.iter().filter(move |g| g.before_hunk == hunk)
}

/// The unified layout's rows: for each hunk its hidden run (if any), its header, then its lines,
/// with a row where a revealed run starts; the trailing hidden run last. None for no hunks.
pub fn unified_rows<'a>(shaped: impl Into<ShapedRef<'a>>) -> Vec<UnifiedRow<'a>> {
    let shaped = shaped.into();
    let mut rows = Vec::new();
    for (h, hunk) in shaped.hunks.iter().enumerate() {
        rows.extend(hidden_before(shaped, h).map(|gap| UnifiedRow::Gap { gap, open: false }));
        rows.push(UnifiedRow::Header(hunk));
        let marks = marks(shaped, h);
        for (i, line) in hunk.lines.iter().enumerate() {
            rows.extend(
                marks
                    .iter()
                    .filter(|(at, _)| *at == i)
                    .map(|(_, gap)| UnifiedRow::Gap { gap, open: true }),
            );
            rows.push(UnifiedRow::Line(line));
        }
    }
    rows.extend(
        hidden_before(shaped, shaped.hunks.len()).map(|gap| UnifiedRow::Gap { gap, open: false }),
    );
    rows
}

/// The unified layout's rows, found by index without listing them: one entry per header, hidden
/// run, revealed run start and stretch of lines, so a view that builds only the visible rows costs
/// the hunk count, not the line count (D5).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnifiedIndex<'a> {
    hunks: &'a [Hunk],
    segments: Vec<UnifiedSegment<'a>>,
    len: usize,
}

/// A stretch of unified rows built the same way.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct UnifiedSegment<'a> {
    /// Its first row.
    start: usize,
    kind: UnifiedSegmentKind<'a>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum UnifiedSegmentKind<'a> {
    Gap(&'a Gap, bool),
    Header(usize),
    /// `count` lines of hunk `hunk` from `first`.
    Lines {
        hunk: usize,
        first: usize,
        count: usize,
    },
}

impl<'a> UnifiedIndex<'a> {
    /// The index of `shaped`'s rows.
    pub fn new(shaped: impl Into<ShapedRef<'a>>) -> Self {
        let shaped = shaped.into();
        let mut segments = Vec::new();
        let mut len = 0;
        let mut push = |kind, rows: usize| {
            segments.push(UnifiedSegment { start: len, kind });
            len += rows;
        };
        for (h, hunk) in shaped.hunks.iter().enumerate() {
            for gap in hidden_before(shaped, h) {
                push(UnifiedSegmentKind::Gap(gap, false), 1);
            }
            push(UnifiedSegmentKind::Header(h), 1);
            let mut first = 0;
            for (at, gap) in marks(shaped, h) {
                if at > first {
                    let count = at - first;
                    push(
                        UnifiedSegmentKind::Lines {
                            hunk: h,
                            first,
                            count,
                        },
                        count,
                    );
                    first = at;
                }
                push(UnifiedSegmentKind::Gap(gap, true), 1);
            }
            if first < hunk.lines.len() {
                let count = hunk.lines.len() - first;
                push(
                    UnifiedSegmentKind::Lines {
                        hunk: h,
                        first,
                        count,
                    },
                    count,
                );
            }
        }
        for gap in hidden_before(shaped, shaped.hunks.len()) {
            push(UnifiedSegmentKind::Gap(gap, false), 1);
        }
        Self {
            hunks: shaped.hunks,
            segments,
            len,
        }
    }

    /// How many rows there are.
    pub fn len(&self) -> usize {
        self.len
    }

    /// Whether there are no rows.
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Row `index`, or `None` past the end.
    pub fn row(&self, index: usize) -> Option<UnifiedRow<'a>> {
        if index >= self.len {
            return None;
        }
        let seg = self.segments[self.segments.partition_point(|s| s.start <= index) - 1];
        match seg.kind {
            UnifiedSegmentKind::Gap(gap, open) => Some(UnifiedRow::Gap { gap, open }),
            UnifiedSegmentKind::Header(h) => Some(UnifiedRow::Header(&self.hunks[h])),
            UnifiedSegmentKind::Lines { hunk, first, .. } => self.hunks[hunk]
                .lines
                .get(first + index - seg.start)
                .map(UnifiedRow::Line),
        }
    }
}

/// One cell of the side-by-side layout: a line of one version (data-model `SideRow`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Cell<'a> {
    /// Its number in that version.
    pub number: u32,
    /// The line, without its ending.
    pub text: &'a str,
    /// Context, added or removed.
    pub kind: LineKind,
}

/// One row of the side-by-side layout: a hunk's header, a hidden or revealed run's row, or the old
/// version's line on the left and the new version's on the right, either side `None` where the
/// other has no partner.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SideRow<'a> {
    /// A hunk's header row.
    Header(&'a Hunk),
    /// A run of unchanged lines: hidden (`open` false) or shown and collapsible.
    Gap {
        /// The run.
        gap: &'a Gap,
        /// Whether its lines are shown.
        open: bool,
    },
    /// A pair of cells.
    Pair {
        /// The old version's line.
        left: Option<Cell<'a>>,
        /// The new version's line.
        right: Option<Cell<'a>>,
    },
}

/// The side-by-side layout's rows: each hunk's header, then its context lines on both sides and
/// each run of removed lines paired index by index with the run of added lines that follows it,
/// the shorter run padded with `None` (US1 s4); hidden and revealed runs get a row as in the
/// unified layout.
pub fn side_by_side_rows<'a>(shaped: impl Into<ShapedRef<'a>>) -> Vec<SideRow<'a>> {
    let shaped = shaped.into();
    let mut rows = Vec::new();
    for (h, hunk) in shaped.hunks.iter().enumerate() {
        rows.extend(hidden_before(shaped, h).map(|gap| SideRow::Gap { gap, open: false }));
        rows.push(SideRow::Header(hunk));
        let marks = marks(shaped, h);
        let mut first = 0;
        for end in marks.iter().map(|(at, _)| *at).chain([hunk.lines.len()]) {
            if end > first {
                pair_lines(&hunk.lines[first..end], &mut rows);
                first = end;
            }
            rows.extend(
                marks
                    .iter()
                    .filter(|(at, _)| *at == end && end < hunk.lines.len())
                    .map(|(_, gap)| SideRow::Gap { gap, open: true }),
            );
        }
    }
    rows.extend(
        hidden_before(shaped, shaped.hunks.len()).map(|gap| SideRow::Gap { gap, open: false }),
    );
    rows
}

/// `lines` as pairs: context on both sides, each removed run beside the added run after it.
fn pair_lines<'a>(mut lines: &'a [DiffLine], rows: &mut Vec<SideRow<'a>>) {
    while let Some(first) = lines.first() {
        if first.kind == LineKind::Context {
            rows.push(SideRow::Pair {
                left: side_cell(first, first.old),
                right: side_cell(first, first.new),
            });
            lines = &lines[1..];
            continue;
        }
        let removed = run_of(lines, LineKind::Removed);
        let added = run_of(&lines[removed.len()..], LineKind::Added);
        for i in 0..removed.len().max(added.len()) {
            rows.push(SideRow::Pair {
                left: removed.get(i).and_then(|l| side_cell(l, l.old)),
                right: added.get(i).and_then(|l| side_cell(l, l.new)),
            });
        }
        lines = &lines[removed.len() + added.len()..];
    }
}

/// The leading lines of `lines` that are `kind`.
fn run_of(lines: &[DiffLine], kind: LineKind) -> &[DiffLine] {
    let end = lines
        .iter()
        .position(|l| l.kind != kind)
        .unwrap_or(lines.len());
    &lines[..end]
}

/// `line` as a cell numbered `number` (its old or its new number).
fn side_cell(line: &DiffLine, number: Option<u32>) -> Option<Cell<'_>> {
    Some(Cell {
        number: number?,
        text: &line.text,
        kind: line.kind,
    })
}

/// The side-by-side layout's rows, found by index without listing them: one entry per header,
/// hidden or revealed run, context run and change run, so a view that builds only the visible rows
/// does not lay out the whole file on every redraw (D5).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SideIndex<'a> {
    hunks: &'a [Hunk],
    /// The runs, in row order.
    runs: Vec<Run<'a>>,
    len: usize,
}

/// A stretch of side-by-side rows built the same way.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Run<'a> {
    /// Its first row.
    start: usize,
    /// The hunk it is in.
    hunk: usize,
    /// The index of its first line in the hunk.
    first: usize,
    kind: RunKind<'a>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RunKind<'a> {
    /// The hunk's header: one row.
    Header,
    /// A hidden (`false`) or revealed (`true`) run's row.
    Gap(&'a Gap, bool),
    /// Context lines, one row each.
    Context(usize),
    /// Removed lines then the added lines after them, paired.
    Change { removed: usize, added: usize },
}

impl RunKind<'_> {
    fn rows(self) -> usize {
        match self {
            RunKind::Header | RunKind::Gap(..) => 1,
            RunKind::Context(n) => n,
            RunKind::Change { removed, added } => removed.max(added),
        }
    }
}

impl<'a> SideIndex<'a> {
    /// The index of `shaped`'s rows.
    pub fn new(shaped: impl Into<ShapedRef<'a>>) -> Self {
        let shaped = shaped.into();
        let mut runs = Vec::new();
        let mut len = 0;
        let mut push = |hunk, first, kind: RunKind<'a>| {
            runs.push(Run {
                start: len,
                hunk,
                first,
                kind,
            });
            len += kind.rows();
        };
        for (h, hunk) in shaped.hunks.iter().enumerate() {
            for gap in hidden_before(shaped, h) {
                push(h, 0, RunKind::Gap(gap, false));
            }
            push(h, 0, RunKind::Header);
            let marks = marks(shaped, h);
            let mut first = 0;
            while first < hunk.lines.len() {
                for (_, gap) in marks.iter().filter(|(at, _)| *at == first) {
                    push(h, first, RunKind::Gap(gap, true));
                }
                // A run never crosses the start of a revealed run.
                let limit = marks
                    .iter()
                    .map(|(at, _)| *at)
                    .find(|at| *at > first)
                    .unwrap_or(hunk.lines.len());
                let rest = &hunk.lines[first..limit];
                let context = run_of(rest, LineKind::Context).len();
                let kind = if context > 0 {
                    RunKind::Context(context)
                } else {
                    let removed = run_of(rest, LineKind::Removed).len();
                    let added = run_of(&rest[removed..], LineKind::Added).len();
                    RunKind::Change { removed, added }
                };
                push(h, first, kind);
                first += match kind {
                    RunKind::Change { removed, added } => removed + added,
                    other => other.rows(),
                };
            }
        }
        for gap in hidden_before(shaped, shaped.hunks.len()) {
            push(shaped.hunks.len(), 0, RunKind::Gap(gap, false));
        }
        Self {
            hunks: shaped.hunks,
            runs,
            len,
        }
    }

    /// How many rows there are.
    pub fn len(&self) -> usize {
        self.len
    }

    /// Whether there are no rows.
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Row `index`, or `None` past the end.
    pub fn row(&self, index: usize) -> Option<SideRow<'a>> {
        if index >= self.len {
            return None;
        }
        let run = self.runs[self.runs.partition_point(|r| r.start <= index) - 1];
        let k = index - run.start;
        if let RunKind::Gap(gap, open) = run.kind {
            return Some(SideRow::Gap { gap, open });
        }
        let hunk = &self.hunks[run.hunk];
        let at = |i: usize| &hunk.lines[run.first + i];
        Some(match run.kind {
            RunKind::Header => SideRow::Header(hunk),
            RunKind::Gap(..) => return None,
            RunKind::Context(_) => SideRow::Pair {
                left: side_cell(at(k), at(k).old),
                right: side_cell(at(k), at(k).new),
            },
            RunKind::Change { removed, added } => SideRow::Pair {
                left: (k < removed).then(|| side_cell(at(k), at(k).old)).flatten(),
                right: (k < added)
                    .then(|| side_cell(at(removed + k), at(removed + k).new))
                    .flatten(),
            },
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn shaped(diff: &FileDiff) -> Shaped {
        match diff {
            FileDiff::Text(hunks) => Shaped::unshaped(hunks),
            _ => Shaped::default(),
        }
    }

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
        let sh = shaped(&diff);
        let rows = unified_rows(&sh);
        let shape: Vec<String> = rows
            .iter()
            .map(|row| match row {
                UnifiedRow::Header(hunk) => hunk.header(),
                UnifiedRow::Line(line) => line.text.clone(),
                UnifiedRow::Gap { .. } => unreachable!("nothing is hidden"),
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
        assert!(unified_rows(&shaped(&FileDiff::Binary)).is_empty());
    }

    #[test]
    fn the_unified_index_finds_each_row_without_listing_them() {
        let diff = parse_unified(TWO_HUNKS.as_bytes());
        let sh = shaped(&diff);
        let rows = unified_rows(&sh);
        let index = UnifiedIndex::new(&sh);
        assert_eq!(index.len(), rows.len());
        for (i, row) in rows.iter().enumerate() {
            assert_eq!(index.row(i).as_ref(), Some(row), "row {i}");
        }
        assert_eq!(index.row(rows.len()), None);
        let none = Shaped::default();
        let binary = UnifiedIndex::new(&none);
        assert_eq!((binary.len(), binary.row(0)), (0, None));
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

    /// One hunk: a context line, three removed lines replaced by one added, a context line, one
    /// added line with no removed partner.
    const PAIRING: &str = concat!(
        "@@ -1,5 +1,4 @@\n",
        " keep\n",
        "-r1\n",
        "-r2\n",
        "-r3\n",
        "+a1\n",
        " mid\n",
        "+a2\n",
    );

    fn cell(number: u32, text: &str, kind: LineKind) -> Option<Cell<'_>> {
        Some(Cell { number, text, kind })
    }

    #[test]
    fn side_by_side_pairs_removed_runs_with_the_added_run_after_them() {
        let diff = parse_unified(PAIRING.as_bytes());
        let sh = shaped(&diff);
        let rows = side_by_side_rows(&sh);
        let pairs: Vec<_> = rows
            .iter()
            .filter_map(|row| match row {
                SideRow::Pair { left, right } => Some((*left, *right)),
                SideRow::Header(_) | SideRow::Gap { .. } => None,
            })
            .collect();
        assert!(
            matches!(rows.first(), Some(SideRow::Header(_))),
            "a hunk starts with its header: {rows:?}"
        );
        use LineKind::{Added as A, Context as C, Removed as R};
        assert_eq!(
            pairs,
            vec![
                (cell(1, "keep", C), cell(1, "keep", C)),
                (cell(2, "r1", R), cell(2, "a1", A)),
                (cell(3, "r2", R), None),
                (cell(4, "r3", R), None),
                (cell(5, "mid", C), cell(3, "mid", C)),
                (None, cell(4, "a2", A)),
            ]
        );
        assert!(side_by_side_rows(&shaped(&FileDiff::Binary)).is_empty());
    }

    #[test]
    fn side_by_side_shows_the_same_changed_lines_as_unified() {
        for raw in [TWO_HUNKS, PAIRING] {
            let diff = parse_unified(raw.as_bytes());
            let sh = shaped(&diff);
            let mut unified: Vec<(LineKind, Option<u32>, Option<u32>, &str)> = unified_rows(&sh)
                .into_iter()
                .filter_map(|row| match row {
                    UnifiedRow::Line(l) => Some((l.kind, l.old, l.new, l.text.as_str())),
                    UnifiedRow::Header(_) | UnifiedRow::Gap { .. } => None,
                })
                .collect();
            let mut side = Vec::new();
            for row in side_by_side_rows(&sh) {
                if let SideRow::Pair { left, right } = row {
                    match (left, right) {
                        (Some(l), Some(r)) if l.kind == LineKind::Context => {
                            side.push((l.kind, Some(l.number), Some(r.number), l.text));
                        }
                        (l, r) => {
                            side.extend(l.map(|c| (c.kind, Some(c.number), None, c.text)));
                            side.extend(r.map(|c| (c.kind, None, Some(c.number), c.text)));
                        }
                    }
                }
            }
            unified.sort_by_key(|l| (l.1, l.2, l.3));
            side.sort_by_key(|l| (l.1, l.2, l.3));
            assert_eq!(side, unified, "the same lines and numbers in both layouts");
        }
    }

    #[test]
    fn the_side_index_finds_each_row_without_listing_them() {
        let mut big = String::from("@@ -1,4 +1,5 @@\n");
        big.push_str("-x\n+y\n+z\n c\n-p\n-q\n+r\n c2\n");
        for raw in [TWO_HUNKS, PAIRING, big.as_str()] {
            let diff = parse_unified(raw.as_bytes());
            let sh = shaped(&diff);
            let rows = side_by_side_rows(&sh);
            let index = SideIndex::new(&sh);
            assert_eq!(index.len(), rows.len(), "{raw}");
            for (i, row) in rows.iter().enumerate() {
                assert_eq!(index.row(i).as_ref(), Some(row), "row {i} of {raw}");
            }
            assert_eq!(index.row(rows.len()), None);
        }
        let none = Shaped::default();
        let binary = SideIndex::new(&none);
        assert_eq!((binary.len(), binary.row(0)), (0, None));
    }

    /// A 30-line file with one change at line 10 and one at line 20, shaped with `e`.
    fn two_distant_changes(e: &crate::review::expand::Expansion) -> Shaped {
        let raw = "@@ -9,3 +9,3 @@\n l9\n-old10\n+l10\n l11\n@@ -19,3 +19,3 @@\n l19\n-old20\n+l20\n l21\n";
        let FileDiff::Text(hunks) = parse_unified(raw.as_bytes()) else {
            panic!()
        };
        let text: String = (1..=30).map(|i| format!("l{i}\n")).collect();
        crate::review::expand::shape(&hunks, &SideLines::from_bytes(text.as_bytes()).unwrap(), e)
    }

    fn unified_shape(rows: &[UnifiedRow<'_>]) -> Vec<String> {
        rows.iter()
            .map(|r| match r {
                UnifiedRow::Header(h) => h.header(),
                UnifiedRow::Line(l) => l.text.clone(),
                UnifiedRow::Gap { gap, open } => format!("gap {} {open}", gap.len),
            })
            .collect()
    }

    #[test]
    fn gap_rows_sit_before_hunks_and_after_the_last_in_both_layouts() {
        use crate::review::expand::{gaps, Expansion};
        let closed = two_distant_changes(&Expansion::default());
        let rows = unified_rows(&closed);
        let shape = unified_shape(&rows);
        assert_eq!(shape[0], "gap 8 false");
        assert_eq!(shape[1], "@@ -9,3 +9,3 @@");
        assert_eq!(shape[6], "gap 7 false");
        assert_eq!(shape.last().unwrap(), "gap 9 false");
        let side = side_by_side_rows(&closed);
        assert!(matches!(side[0], SideRow::Gap { open: false, .. }));
        assert!(matches!(
            side.last(),
            Some(SideRow::Gap { open: false, .. })
        ));
        // Opening the middle run: no gap row between, a collapse row where it starts, the lines are
        // context rows numbered continuously.
        let all = gaps(
            &match parse_unified(
                b"@@ -9,3 +9,3 @@\n l9\n-o\n+l10\n l11\n@@ -19,3 +19,3 @@\n l19\n-o\n+l20\n l21\n",
            ) {
                FileDiff::Text(h) => h,
                _ => panic!(),
            },
            30,
        );
        let mut e = Expansion::default();
        e.toggle(&all[1]);
        let open = two_distant_changes(&e);
        let shape = unified_shape(&unified_rows(&open));
        assert_eq!(
            &shape[2..8],
            ["l9", "old10", "l10", "l11", "gap 7 true", "l12"]
        );
        assert_eq!(shape.iter().filter(|s| s.starts_with("@@")).count(), 1);
        let sh = open.clone();
        let revealed: Vec<_> = unified_rows(&sh)
            .into_iter()
            .filter_map(|r| match r {
                UnifiedRow::Line(l) if (12..=18).contains(&l.new.unwrap_or(0)) => {
                    Some((l.kind, l.old, l.new))
                }
                _ => None,
            })
            .collect();
        assert_eq!(revealed.len(), 7);
        assert!(revealed
            .iter()
            .all(|(k, o, n)| *k == LineKind::Context && o.unwrap() == n.unwrap()));
        let side = side_by_side_rows(&open);
        assert_eq!(
            side.iter()
                .filter(|r| matches!(r, SideRow::Gap { open: true, .. }))
                .count(),
            1
        );
    }

    #[test]
    fn indexes_match_the_row_lists_with_hidden_and_revealed_runs() {
        use crate::review::expand::{gaps, Expansion};
        let probe = two_distant_changes(&Expansion::default());
        let all = gaps(&probe.hunks, 30);
        for pick in [vec![], vec![0], vec![1], vec![2], vec![0, 1, 2]] {
            let mut e = Expansion::default();
            for i in pick {
                // Keys of the closed shape's gaps equal the unshaped gaps' keys.
                e.toggle(&all[i]);
            }
            let sh = two_distant_changes(&e);
            let rows = unified_rows(&sh);
            let index = UnifiedIndex::new(&sh);
            assert_eq!(index.len(), rows.len());
            for (i, row) in rows.iter().enumerate() {
                assert_eq!(index.row(i).as_ref(), Some(row), "unified row {i}");
            }
            assert_eq!(index.row(rows.len()), None);
            let rows = side_by_side_rows(&sh);
            let index = SideIndex::new(&sh);
            assert_eq!(index.len(), rows.len());
            for (i, row) in rows.iter().enumerate() {
                assert_eq!(index.row(i).as_ref(), Some(row), "side row {i}");
            }
            assert_eq!(index.row(rows.len()), None);
        }
    }
}
