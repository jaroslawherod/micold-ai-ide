//! Reading the lines a diff hides (feature 1150, research R1): the runs of unchanged lines between
//! hunks, and a pure shaping step that turns hunks, the new version's lines and an [`Expansion`]
//! into hunks with the revealed lines inserted as ordinary context lines.
//!
//! Because a revealed line is a context [`DiffLine`] with the numbers its neighbours imply, the row
//! builders, the syntax colouring and the pick handling treat it exactly like the diff's own context
//! (FR-002, FR-007). Nothing here reads a file or draws: the caller supplies the new side.

use std::collections::BTreeSet;

use super::diff::{DiffLine, Hunk, LineKind, SideLines};

/// Where a hidden run sits, stable across a refresh while the hunk before it keeps its place.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Position {
    /// Before the first hunk.
    Leading,
    /// Between two hunks; the payload is the new-side number of the previous hunk's last line.
    Between(u32),
    /// After the last hunk.
    Trailing,
}

/// A run of unchanged lines the diff does not show.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Gap {
    /// The hunk this run precedes (`hunks.len()` for the trailing run).
    pub before_hunk: usize,
    /// The new-side number of its first line.
    pub new_start: u32,
    /// How many lines.
    pub len: u32,
    /// The old-side number of its first line.
    pub old_start: u32,
    /// Where it sits.
    pub position: Position,
}

impl Gap {
    /// The key that identifies this run across refreshes.
    pub fn key(&self) -> GapKey {
        GapKey {
            position: self.position,
            new_start: self.new_start,
            len: self.len,
        }
    }
}

/// What identifies a hidden run in the expansion state: its position and new-side span.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct GapKey {
    /// Where it sits.
    pub position: Position,
    /// Its first new-side line.
    pub new_start: u32,
    /// How many lines.
    pub len: u32,
}

/// The new-side line just before a hunk's covered range and the old-side one (0 for none), and the
/// last covered lines. Hunk headers number a pure deletion at the line before it (`new_len == 0`).
struct Span {
    new_before: u32,
    old_before: u32,
    new_end: u32,
    old_end: u32,
}

impl Span {
    fn of(h: &Hunk) -> Self {
        let new_before = if h.new_len == 0 {
            h.new_start
        } else {
            h.new_start.saturating_sub(1)
        };
        let old_before = if h.old_len == 0 {
            h.old_start
        } else {
            h.old_start.saturating_sub(1)
        };
        Self {
            new_before,
            old_before,
            new_end: new_before + h.new_len,
            old_end: old_before + h.old_len,
        }
    }
}

/// The runs of unchanged lines `hunks` leave out of a file of `new_total` lines, in order.
/// Zero-length runs are omitted, so a hunk at line 1, a hunk ending at the last line, an empty file
/// and a file the hunks cover entirely have no marker there. No hunks, no runs.
pub fn gaps(hunks: &[Hunk], new_total: u32) -> Vec<Gap> {
    /// The run between `prev` and `to` (new, old numbers of the lines either side), if any.
    fn run(
        before_hunk: usize,
        position: Position,
        prev: (u32, u32),
        to: (u32, u32),
    ) -> Option<Gap> {
        let len = to.0.saturating_sub(prev.0);
        (len > 0 && to.1.saturating_sub(prev.1) == len).then_some(Gap {
            before_hunk,
            new_start: prev.0 + 1,
            len,
            old_start: prev.1 + 1,
            position,
        })
    }
    let mut out = Vec::new();
    let mut prev = (0u32, 0u32);
    for (i, hunk) in hunks.iter().enumerate() {
        let s = Span::of(hunk);
        let position = if i == 0 {
            Position::Leading
        } else {
            Position::Between(prev.0)
        };
        out.extend(run(i, position, prev, (s.new_before, s.old_before)));
        prev = (s.new_end, s.old_end);
    }
    if !hunks.is_empty() {
        let tail = new_total.saturating_sub(prev.0);
        out.extend(run(
            hunks.len(),
            Position::Trailing,
            prev,
            (prev.0 + tail, prev.1 + tail),
        ));
    }
    out
}

/// Which hidden runs the reader opened in one file of one source.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Expansion {
    /// Every hidden run is shown.
    pub whole: bool,
    /// The runs opened one by one.
    pub open: BTreeSet<GapKey>,
}

impl Expansion {
    /// Open `gap` if closed, close it if open.
    pub fn toggle(&mut self, gap: &Gap) {
        self.toggle_key(gap.key());
    }

    /// Open the run `key` names if closed, close it if open.
    pub fn toggle_key(&mut self, key: GapKey) {
        if !self.open.remove(&key) {
            self.open.insert(key);
        }
    }

    /// Show every run (or go back to the ones opened one by one).
    pub fn set_whole(&mut self, whole: bool) {
        self.whole = whole;
    }

    /// Whether `gap` is shown.
    pub fn is_open(&self, gap: &Gap) -> bool {
        self.whole || self.open.contains(&gap.key())
    }

    /// The state after a refresh produced `gaps`: an open run stays when a current run has its
    /// position and span, any other is dropped; `whole` stays (FR-008).
    pub fn reconcile(&self, gaps: &[Gap]) -> Expansion {
        Expansion {
            whole: self.whole,
            open: gaps
                .iter()
                .map(Gap::key)
                .filter(|k| self.open.contains(k))
                .collect(),
        }
    }
}

/// A revealed run, where it sits in the shaped hunks, so the reader can close it again.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Revealed {
    /// The run.
    pub gap: Gap,
    /// The shaped hunk holding its lines.
    pub hunk: usize,
    /// The index of its first line in that hunk.
    pub line: usize,
}

/// A diff after shaping: the hunks with the open runs inserted, the runs still hidden and the runs
/// shown.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Shaped {
    /// The hunks, touching ones merged.
    pub hunks: Vec<Hunk>,
    /// The hidden runs; `before_hunk` indexes `hunks`.
    pub gaps: Vec<Gap>,
    /// The shown runs.
    pub revealed: Vec<Revealed>,
}

impl Shaped {
    /// `hunks` as they are: nothing revealed, no marker (a diff whose new side is not at hand).
    pub fn unshaped(hunks: &[Hunk]) -> Self {
        Self {
            hunks: hunks.to_vec(),
            ..Self::default()
        }
    }
}

/// A borrowed [`Shaped`], or hunks with nothing hidden or revealed: what the row builders read, so
/// a diff that was never shaped needs no copy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ShapedRef<'a> {
    /// The hunks.
    pub hunks: &'a [Hunk],
    /// The hidden runs.
    pub gaps: &'a [Gap],
    /// The shown runs.
    pub revealed: &'a [Revealed],
}

impl<'a> ShapedRef<'a> {
    /// `hunks` as they are, with no marker.
    pub fn unshaped(hunks: &'a [Hunk]) -> Self {
        Self {
            hunks,
            gaps: &[],
            revealed: &[],
        }
    }
}

impl<'a> From<&'a Shaped> for ShapedRef<'a> {
    fn from(s: &'a Shaped) -> Self {
        Self {
            hunks: &s.hunks,
            gaps: &s.gaps,
            revealed: &s.revealed,
        }
    }
}

/// `hunks` with the runs `e` opens inserted as context lines read from `new_side`.
pub fn shape(hunks: &[Hunk], new_side: &SideLines, e: &Expansion) -> Shaped {
    let total = u32::try_from(new_side.len()).unwrap_or(u32::MAX);
    let all = gaps(hunks, total);
    let mut out = Shaped::default();
    let mut gap_at = all.iter().peekable();
    // The hunk being built; a closed gap or the end finishes it.
    let mut current: Option<Hunk> = None;
    let finish = |current: &mut Option<Hunk>, out: &mut Shaped| {
        if let Some(mut h) = current.take() {
            h.old_len = count(&h.lines, |l| l.old.is_some());
            h.new_len = count(&h.lines, |l| l.new.is_some());
            out.hunks.push(h);
        }
    };
    for i in 0..=hunks.len() {
        if let Some(gap) = gap_at.next_if(|g| g.before_hunk == i).copied() {
            if e.is_open(&gap) {
                let lines = revealed_lines(&gap, new_side);
                let h = current.get_or_insert_with(|| Hunk {
                    old_start: gap.old_start,
                    new_start: gap.new_start,
                    ..empty_hunk(hunks.get(i))
                });
                out.revealed.push(Revealed {
                    gap,
                    hunk: out.hunks.len(),
                    line: h.lines.len(),
                });
                h.lines.extend(lines);
            } else {
                finish(&mut current, &mut out);
                out.gaps.push(Gap {
                    before_hunk: out.hunks.len(),
                    ..gap
                });
            }
        }
        let Some(next) = hunks.get(i) else { break };
        match current.as_mut() {
            Some(h) => {
                h.lines.extend(next.lines.iter().cloned());
                h.old_no_newline = next.old_no_newline;
                h.new_no_newline = next.new_no_newline;
            }
            None => current = Some(next.clone()),
        }
    }
    finish(&mut current, &mut out);
    out
}

fn empty_hunk(template: Option<&Hunk>) -> Hunk {
    Hunk {
        old_start: 0,
        old_len: 0,
        new_start: 0,
        new_len: 0,
        section: template.map(|h| h.section.clone()).unwrap_or_default(),
        lines: Vec::new(),
        old_no_newline: false,
        new_no_newline: false,
    }
}

fn count(lines: &[DiffLine], f: impl Fn(&DiffLine) -> bool) -> u32 {
    u32::try_from(lines.iter().filter(|l| f(l)).count()).unwrap_or(u32::MAX)
}

/// The context lines of `gap`, numbered from its two starts.
fn revealed_lines(gap: &Gap, new_side: &SideLines) -> Vec<DiffLine> {
    (0..gap.len)
        .map(|k| DiffLine {
            kind: LineKind::Context,
            old: Some(gap.old_start + k),
            new: Some(gap.new_start + k),
            text: new_side
                .line(gap.new_start + k)
                .unwrap_or_default()
                .to_owned(),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::review::diff::{parse_unified, FileDiff};

    fn side(n: u32) -> SideLines {
        let text: String = (1..=n).map(|i| format!("line {i}\n")).collect();
        SideLines::from_bytes(text.as_bytes()).unwrap()
    }

    fn hunk(old_start: u32, old_len: u32, new_start: u32, new_len: u32) -> Hunk {
        let mut lines = Vec::new();
        // Context only: old and new lengths must agree for these helper hunks.
        for k in 0..new_len.max(old_len) {
            lines.push(DiffLine {
                kind: LineKind::Context,
                old: (k < old_len).then_some(old_start + k),
                new: (k < new_len).then_some(new_start + k),
                text: format!("line {}", new_start + k),
            });
        }
        Hunk {
            old_start,
            old_len,
            new_start,
            new_len,
            section: String::new(),
            lines,
            old_no_newline: false,
            new_no_newline: false,
        }
    }

    fn spans(gaps: &[Gap]) -> Vec<(u32, u32)> {
        gaps.iter().map(|g| (g.new_start, g.len)).collect()
    }

    #[test]
    fn gaps_lists_leading_between_and_trailing_runs() {
        let h = [hunk(5, 3, 5, 3), hunk(20, 3, 20, 3)];
        let g = gaps(&h, 30);
        assert_eq!(spans(&g), vec![(1, 4), (8, 12), (23, 8)]);
        assert_eq!(g[0].position, Position::Leading);
        assert_eq!(g[1].position, Position::Between(7));
        assert_eq!(g[2].position, Position::Trailing);
        assert_eq!(
            (g[0].before_hunk, g[1].before_hunk, g[2].before_hunk),
            (0, 1, 2)
        );
        assert_eq!(g[1].old_start, 8);
    }

    #[test]
    fn gaps_omits_zero_length_runs() {
        // A hunk at line 1, touching the next one, ending at the last line.
        let h = [hunk(1, 3, 1, 3), hunk(4, 3, 4, 3)];
        assert!(gaps(&h, 6).is_empty());
        assert_eq!(spans(&gaps(&h, 9)), vec![(7, 3)]);
    }

    #[test]
    fn gaps_follow_old_numbers_across_a_size_change() {
        // Two lines added in the first hunk: the new side runs 2 ahead of the old one.
        let h = [hunk(3, 1, 3, 3), hunk(30, 1, 32, 1)];
        let g = gaps(&h, 40);
        assert_eq!(spans(&g), vec![(1, 2), (6, 26), (33, 8)]);
        assert_eq!((g[1].old_start, g[2].old_start), (4, 31));
    }

    #[test]
    fn gaps_handles_a_pure_deletion_hunk() {
        // Old line 10 deleted: header `-10,1 +9,0`.
        let h = [hunk(10, 1, 9, 0)];
        let g = gaps(&h, 20);
        assert_eq!(spans(&g), vec![(1, 9), (10, 11)]);
        assert_eq!((g[0].old_start, g[1].old_start), (1, 11));
    }

    #[test]
    fn gaps_handles_empty_one_line_and_fully_covered_files() {
        assert!(gaps(&[], 10).is_empty());
        // Whole file deleted: new side empty, header `-1,3 +0,0`.
        assert!(gaps(&[hunk(1, 3, 0, 0)], 0).is_empty());
        // One-line file changed.
        assert!(gaps(&[hunk(1, 1, 1, 1)], 1).is_empty());
        // Added file.
        assert!(gaps(&[hunk(0, 0, 1, 5)], 5).is_empty());
    }

    #[test]
    fn shape_closed_returns_the_hunks_and_lists_every_run() {
        let h = [hunk(5, 3, 5, 3), hunk(20, 3, 20, 3)];
        let s = shape(&h, &side(30), &Expansion::default());
        assert_eq!(s.hunks, h.to_vec());
        assert_eq!(s.gaps, gaps(&h, 30));
        assert!(s.revealed.is_empty());
    }

    fn new_numbers(s: &Shaped) -> Vec<u32> {
        s.hunks
            .iter()
            .flat_map(|h| h.lines.iter().filter_map(|l| l.new))
            .collect()
    }

    #[test]
    fn shape_whole_covers_every_new_line_once_in_order() {
        let diff =
            parse_unified(b"@@ -3,3 +3,3 @@\n a\n-old\n+new\n c\n@@ -20,2 +20,3 @@\n x\n+y\n z\n");
        let FileDiff::Text(h) = diff else { panic!() };
        let mut e = Expansion::default();
        e.set_whole(true);
        let s = shape(&h, &side(30), &e);
        assert_eq!(s.hunks.len(), 1);
        assert!(s.gaps.is_empty());
        assert_eq!(new_numbers(&s), (1..=30).collect::<Vec<_>>());
        // Removed line kept in place, with its old number.
        let kinds: Vec<_> = s.hunks[0]
            .lines
            .iter()
            .filter(|l| l.kind != LineKind::Context)
            .map(|l| (l.kind, l.old, l.new))
            .collect();
        assert_eq!(
            kinds,
            vec![
                (LineKind::Removed, Some(4), None),
                (LineKind::Added, None, Some(4)),
                (LineKind::Added, None, Some(21)),
            ]
        );
        // Revealed lines continue the old numbers.
        let line = s.hunks[0].lines.iter().find(|l| l.new == Some(30)).unwrap();
        assert_eq!(line.old, Some(29));
        assert_eq!(line.text, "line 30");
        assert_eq!((s.hunks[0].old_len, s.hunks[0].new_len), (29, 30));
        assert_eq!((s.hunks[0].old_start, s.hunks[0].new_start), (1, 1));
    }

    #[test]
    fn shape_opens_one_run_and_merges_touching_hunks() {
        let h = [hunk(5, 3, 5, 3), hunk(20, 3, 20, 3), hunk(40, 2, 40, 2)];
        let g = gaps(&h, 50);
        let mut e = Expansion::default();
        e.toggle(&g[1]);
        let s = shape(&h, &side(50), &e);
        assert_eq!(s.hunks.len(), 2);
        assert_eq!((s.hunks[0].new_start, s.hunks[0].new_len), (5, 18));
        assert_eq!((s.hunks[0].old_start, s.hunks[0].old_len), (5, 18));
        assert_eq!(s.hunks[1], h[2]);
        // Remaining runs reference shaped hunk indexes.
        assert_eq!(
            s.gaps
                .iter()
                .map(|g| (g.new_start, g.before_hunk))
                .collect::<Vec<_>>(),
            vec![(1, 0), (23, 1), (42, 2)]
        );
        assert_eq!(s.revealed.len(), 1);
        assert_eq!((s.revealed[0].hunk, s.revealed[0].line), (0, 3));
        // Collapsing restores the input.
        e.toggle(&g[1]);
        assert_eq!(shape(&h, &side(50), &e).hunks, h.to_vec());
    }

    #[test]
    fn shape_leading_and_trailing_runs_extend_the_file_ends() {
        let h = [hunk(5, 3, 5, 3)];
        let g = gaps(&h, 12);
        let mut e = Expansion::default();
        e.toggle(&g[0]);
        e.toggle(&g[1]);
        let s = shape(&h, &side(12), &e);
        assert_eq!(s.hunks.len(), 1);
        assert_eq!(new_numbers(&s), (1..=12).collect::<Vec<_>>());
        assert_eq!((s.hunks[0].new_start, s.hunks[0].new_len), (1, 12));
        assert!(s.gaps.is_empty());
    }

    #[test]
    fn shape_keeps_flags_of_the_last_merged_hunk() {
        let mut a = hunk(1, 2, 1, 2);
        a.section = "first".into();
        let mut b = hunk(6, 2, 6, 2);
        b.section = "second".into();
        b.old_no_newline = true;
        b.new_no_newline = true;
        let h = [a, b];
        let mut e = Expansion::default();
        e.toggle(&gaps(&h, 7)[0]);
        let s = shape(&h, &side(7), &e);
        assert_eq!(s.hunks.len(), 1);
        assert_eq!(s.hunks[0].section, "first");
        assert!(s.hunks[0].old_no_newline && s.hunks[0].new_no_newline);
    }

    #[test]
    fn shape_pure_deletion_keeps_numbers_continuous() {
        let diff = parse_unified(b"@@ -9,3 +9,2 @@\n a\n-gone\n b\n");
        let FileDiff::Text(h) = diff else { panic!() };
        let mut e = Expansion::default();
        e.set_whole(true);
        let s = shape(&h, &side(12), &e);
        assert_eq!(new_numbers(&s), (1..=12).collect::<Vec<_>>());
        let removed = s.hunks[0]
            .lines
            .iter()
            .find(|l| l.kind == LineKind::Removed);
        assert_eq!(removed.unwrap().old, Some(10));
    }

    #[test]
    fn shape_five_thousand_lines_is_fast() {
        let h = [hunk(2000, 3, 2000, 3), hunk(4000, 3, 4000, 3)];
        let mut e = Expansion::default();
        e.set_whole(true);
        let side = side(5000);
        let start = std::time::Instant::now();
        let s = shape(&h, &side, &e);
        assert!(start.elapsed() < std::time::Duration::from_millis(100));
        assert_eq!(new_numbers(&s).len(), 5000);
    }

    #[test]
    fn expansion_toggles_sets_whole_and_reports_open() {
        let g = gaps(&[hunk(5, 3, 5, 3)], 20);
        let mut e = Expansion::default();
        assert!(!e.is_open(&g[0]));
        e.toggle(&g[0]);
        assert!(e.is_open(&g[0]) && !e.is_open(&g[1]));
        e.toggle(&g[0]);
        assert!(!e.is_open(&g[0]));
        e.set_whole(true);
        assert!(e.is_open(&g[0]) && e.is_open(&g[1]));
    }

    #[test]
    fn toggle_key_flips_the_run_it_names() {
        let g = gaps(&[hunk(5, 3, 5, 3)], 20);
        let mut e = Expansion::default();
        e.toggle_key(g[1].key());
        assert!(e.is_open(&g[1]));
        e.toggle_key(g[1].key());
        assert!(!e.is_open(&g[1]));
    }

    #[test]
    fn reconcile_keeps_unchanged_runs_and_drops_vanished_ones() {
        let before = gaps(&[hunk(5, 3, 5, 3), hunk(20, 3, 20, 3)], 30);
        let mut e = Expansion::default();
        e.toggle(&before[0]);
        e.toggle(&before[1]);
        e.set_whole(true);
        // The second hunk moved: the middle run changed span, the leading one did not.
        let after = gaps(&[hunk(5, 3, 5, 3), hunk(22, 3, 22, 3)], 30);
        let r = e.reconcile(&after);
        assert!(r.whole);
        assert_eq!(
            r.open.into_iter().collect::<Vec<_>>(),
            vec![before[0].key()]
        );
    }
}
