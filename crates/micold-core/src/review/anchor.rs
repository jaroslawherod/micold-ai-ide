//! Re-anchoring comments (feature 946, research R2, data-model § Core: `review::anchor`): where a
//! comment's quoted lines are now, so a comment follows its code as the file changes.

use super::comment::Status;
use super::diff::SideLines;
use super::LineRange;

/// Where a comment's quoted code is in the current version of its file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Located {
    /// Still on the lines the comment holds.
    Same(LineRange),
    /// The same code, on other lines: the occurrence nearest the old place.
    Moved(LineRange),
    /// The quoted code is not in the file.
    Gone,
    /// Two occurrences equally near the old place: not guessed.
    Ambiguous,
}

impl Located {
    /// The place to draw the comment: the range for `Same` and `Moved`; `last` clamped into the
    /// file (`len` lines, at least one) for `Gone` and `Ambiguous` (FR-021).
    pub fn shown(&self, last: LineRange, len: usize) -> LineRange {
        match self {
            Self::Same(range) | Self::Moved(range) => *range,
            Self::Gone | Self::Ambiguous => clamp(last, len),
        }
    }
}

/// `last` moved to lie inside `1..=max(len, 1)`, keeping as many of its lines as fit.
fn clamp(last: LineRange, len: usize) -> LineRange {
    let max = u32::try_from(len.max(1)).unwrap_or(u32::MAX);
    let end = last.end().min(max);
    let start = last.start().min(end);
    LineRange::new(start, end).unwrap_or(last)
}

/// Whether a comment with `status` and this `located` shows the outdated mark: only a Draft or
/// Sent comment whose code is gone or ambiguous (FR-021, FR-024).
pub fn is_outdated(located: &Located, status: Status) -> bool {
    matches!(located, Located::Gone | Located::Ambiguous)
        && matches!(status, Status::Draft | Status::Sent)
}

/// A line without a trailing `\r` (FR-025).
fn bare(line: &str) -> &str {
    line.strip_suffix('\r').unwrap_or(line)
}

/// Whether the lines of `lines` from 1-based `start` equal `quote`.
fn holds(quote: &[String], start: u32, lines: &SideLines) -> bool {
    quote.iter().zip(start..).all(|(quoted, number)| {
        lines
            .line(number)
            .is_some_and(|line| bare(line) == bare(quoted))
    })
}

/// Where `quote`, last seen at `last`, is in `lines` (research R2): `Same` while `last` still holds
/// it, else the one occurrence nearest `last.start()`, `Ambiguous` for two equally near, `Gone`
/// for none. Lines differing only by a trailing `\r` are equal.
pub fn locate(quote: &[String], last: LineRange, lines: &SideLines) -> Located {
    let wanted = quote.len();
    if wanted == 0 {
        return Located::Gone;
    }
    if usize::try_from(last.len()).is_ok_and(|n| n == wanted) && holds(quote, last.start(), lines) {
        return Located::Same(last);
    }
    let Some(first) = quote.first().map(|line| bare(line)) else {
        return Located::Gone;
    };
    let Ok(wanted_u32) = u32::try_from(wanted) else {
        return Located::Gone;
    };
    let mut best: Option<(u32, u32)> = None;
    let mut tied = false;
    for start in 1..=lines.len().saturating_sub(wanted - 1) {
        let Ok(start) = u32::try_from(start) else {
            break;
        };
        if lines.line(start).map(bare) != Some(first) || !holds(quote, start, lines) {
            continue;
        }
        let distance = start.abs_diff(last.start());
        match best {
            Some((nearest, _)) if distance > nearest => {}
            Some((nearest, _)) if distance == nearest => tied = true,
            _ => {
                best = Some((distance, start));
                tied = false;
            }
        }
    }
    match best {
        None => Located::Gone,
        Some(_) if tied => Located::Ambiguous,
        Some((_, start)) => LineRange::new(start, start + wanted_u32 - 1)
            .map_or(Located::Gone, Located::Moved),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Instant;

    fn range(start: u32, end: u32) -> LineRange {
        LineRange::new(start, end).unwrap()
    }

    fn lines(texts: &[String]) -> SideLines {
        SideLines::from_bytes(texts.join("\n").as_bytes()).unwrap()
    }

    /// `count` distinct filler lines named `tag`.
    fn filler(tag: &str, count: usize) -> Vec<String> {
        (0..count).map(|n| format!("{tag} {n}")).collect()
    }

    /// A 30-line file whose lines 10–12 are the quote.
    fn file() -> (Vec<String>, Vec<String>) {
        let mut all = filler("line", 30);
        let quote: Vec<String> = ["fn a() {", "    body();", "}"]
            .map(str::to_owned)
            .to_vec();
        all.splice(9..12, quote.clone());
        (all, quote)
    }

    #[test]
    fn unchanged_code_stays_where_it_is() {
        let (all, quote) = file();
        assert_eq!(
            locate(&quote, range(10, 12), &lines(&all)),
            Located::Same(range(10, 12))
        );
    }

    #[test]
    fn lines_added_above_move_the_comment_down() {
        let (mut all, quote) = file();
        all.splice(0..0, filler("new", 5));
        let found = locate(&quote, range(10, 12), &lines(&all));
        assert_eq!(found, Located::Moved(range(15, 17)));
        assert_eq!(found.shown(range(10, 12), all.len()), range(15, 17));
    }

    #[test]
    fn lines_removed_above_move_the_comment_up() {
        let (mut all, quote) = file();
        all.drain(0..3);
        assert_eq!(
            locate(&quote, range(10, 12), &lines(&all)),
            Located::Moved(range(7, 9))
        );
    }

    #[test]
    fn a_changed_line_makes_the_code_gone_and_shown_at_its_last_place() {
        let (mut all, quote) = file();
        all[10] = "    other();".to_owned();
        let found = locate(&quote, range(10, 12), &lines(&all));
        assert_eq!(found, Located::Gone);
        assert_eq!(found.shown(range(10, 12), all.len()), range(10, 12));
    }

    #[test]
    fn gone_code_is_shown_inside_a_shorter_file() {
        let found = Located::Gone;
        assert_eq!(found.shown(range(10, 12), 4), range(4, 4));
        assert_eq!(found.shown(range(10, 12), 11), range(10, 11));
        assert_eq!(found.shown(range(10, 12), 0), range(1, 1));
        assert_eq!(Located::Ambiguous.shown(range(3, 5), 100), range(3, 5));
    }

    #[test]
    fn the_nearest_of_two_occurrences_wins() {
        let (mut all, quote) = file();
        all.splice(24..24, quote.clone());
        // Occurrences at 10 and 25; the comment was last at 12.
        assert_eq!(
            locate(&quote, range(13, 15), &lines(&all)),
            Located::Moved(range(10, 12))
        );
        assert_eq!(
            locate(&quote, range(20, 22), &lines(&all)),
            Located::Moved(range(25, 27))
        );
    }

    #[test]
    fn two_equally_near_occurrences_are_ambiguous() {
        let (_, quote) = file();
        let mut twice = filler("x", 20);
        twice.splice(4..4, quote.clone());
        twice.splice(14..14, quote.clone());
        // Occurrences at 5 and 15; last starts at 10 → both 5 away.
        assert_eq!(
            locate(&quote, range(10, 12), &lines(&twice)),
            Located::Ambiguous
        );
    }

    #[test]
    fn restored_code_is_found_again() {
        let (all, quote) = file();
        let mut gone = all.clone();
        gone[10] = "changed".to_owned();
        assert_eq!(locate(&quote, range(10, 12), &lines(&gone)), Located::Gone);
        let mut back = all;
        back.splice(0..0, filler("new", 2));
        assert_eq!(
            locate(&quote, range(10, 12), &lines(&back)),
            Located::Moved(range(12, 14))
        );
    }

    #[test]
    fn a_trailing_carriage_return_is_not_a_difference() {
        let (all, quote) = file();
        let crlf = SideLines::from_bytes(all.join("\r\n").as_bytes()).unwrap();
        assert_eq!(
            locate(&quote, range(10, 12), &crlf),
            Located::Same(range(10, 12))
        );
        let quoted_cr: Vec<String> = quote.iter().map(|line| format!("{line}\r")).collect();
        assert_eq!(
            locate(&quoted_cr, range(10, 12), &lines(&all)),
            Located::Same(range(10, 12))
        );
        let mut shifted = all;
        shifted.insert(0, "top".to_owned());
        assert_eq!(
            locate(&quoted_cr, range(10, 12), &lines(&shifted)),
            Located::Moved(range(11, 13))
        );
    }

    #[test]
    fn only_a_draft_or_sent_comment_is_outdated() {
        for located in [Located::Gone, Located::Ambiguous] {
            assert!(is_outdated(&located, Status::Draft));
            assert!(is_outdated(&located, Status::Sent));
            assert!(!is_outdated(&located, Status::Applied));
            assert!(!is_outdated(&located, Status::Answered));
        }
        for located in [Located::Same(range(1, 1)), Located::Moved(range(2, 2))] {
            assert!(!is_outdated(&located, Status::Draft));
            assert!(!is_outdated(&located, Status::Sent));
        }
    }

    /// SC-004: over many edits around a quote, `Same` and `Moved` always land on lines that equal
    /// the quote.
    #[test]
    fn a_placed_comment_always_sits_on_its_quoted_lines() {
        let (base, quote) = file();
        for offset in 0..=base.len() {
            for size in 0..4 {
                for insert in [true, false] {
                    let mut edited = base.clone();
                    if insert {
                        edited.splice(offset..offset, filler("ins", size));
                    } else {
                        let end = (offset + size).min(edited.len());
                        edited.drain(offset..end);
                    }
                    let side = lines(&edited);
                    match locate(&quote, range(10, 12), &side) {
                        Located::Same(found) | Located::Moved(found) => {
                            let on: Vec<&str> = (found.start()..=found.end())
                                .map(|n| side.line(n).unwrap())
                                .collect();
                            assert_eq!(on, quote, "offset {offset} size {size} insert {insert}");
                        }
                        Located::Gone | Located::Ambiguous => {}
                    }
                }
            }
        }
    }

    /// SC-006: 200 comments against a 5,000-line file stay inside the send path's budget.
    #[test]
    fn two_hundred_comments_locate_inside_the_budget() {
        let mut all: Vec<String> = (0..5000).map(|n| format!("    let v{n} = {n};")).collect();
        all.splice(0..0, filler("new", 7));
        let side = lines(&all);
        let started = Instant::now();
        for n in 0..200_usize {
            let first = 20 + n * 20;
            let quote: Vec<String> = (first..first + 3)
                .map(|k| format!("    let v{k} = {k};"))
                .collect();
            let last = range(u32::try_from(first + 1).unwrap(), u32::try_from(first + 3).unwrap());
            assert!(matches!(locate(&quote, last, &side), Located::Moved(_)));
        }
        assert!(
            started.elapsed().as_millis() < 100,
            "took {:?}",
            started.elapsed()
        );
    }
}
