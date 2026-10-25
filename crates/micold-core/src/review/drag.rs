//! Picking lines by dragging over the diff's line numbers (#1275, US2; FR-015–FR-018).
//!
//! A press on a number starts a [`Drag`] on that number's side; each row the pointer passes is fed
//! to [`Drag::over`], and [`Drag::release`] says whether the gesture was a click (today's pick, no
//! composer) or a range (the composer opens under the line released on). Render-free: the client's
//! view only maps pointer events to row numbers.

use super::{LineRange, Side};

/// The side and line a press on a row's numbers picks (C1): a row with a new-side number (an
/// added or context line) counts as the new side, a row with only an old-side number as the old
/// side, and a row with neither (a hunk header, a gap) picks nothing.
pub fn gutter_line(old: Option<u32>, new: Option<u32>) -> Option<(Side, u32)> {
    match (old, new) {
        (_, Some(n)) => Some((Side::New, n)),
        (Some(o), None) => Some((Side::Old, o)),
        (None, None) => None,
    }
}

/// A drag in progress: the side and line pressed and the last line of that side passed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Drag {
    side: Side,
    pressed: u32,
    last: u32,
    moved: bool,
}

/// What a drag amounts to when the pointer is released.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Release {
    /// Pressed and released without reaching another line: today's click (FR-018).
    Click { side: Side, line: u32 },
    /// Reached another line at least once: pick `anchor..=head` and open the composer under
    /// `head`, the line released on (FR-015).
    Range { side: Side, anchor: u32, head: u32 },
}

impl Drag {
    /// Starts a drag on `line` of `side`.
    pub fn press(side: Side, line: u32) -> Self {
        Self {
            side,
            pressed: line,
            last: line,
            moved: false,
        }
    }

    /// The pointer passed a row with these numbers. A row without a number on the drag's side
    /// (a hunk header, a gap, a comment card, a line of the other side) changes nothing (FR-017).
    pub fn over(&mut self, old: Option<u32>, new: Option<u32>) {
        let line = match self.side {
            Side::New => new,
            Side::Old => old,
        };
        if let Some(line) = line {
            self.last = line;
            self.moved |= line != self.pressed;
        }
    }

    /// The lines to show as picked while dragging (FR-016).
    pub fn span(&self) -> (Side, LineRange) {
        let (start, end) = (self.pressed.min(self.last), self.pressed.max(self.last));
        let range = LineRange::new(start, end).expect("gutter line numbers are 1-based");
        (self.side, range)
    }

    /// Ends the drag.
    pub fn release(self) -> Release {
        if self.moved {
            Release::Range {
                side: self.side,
                anchor: self.pressed,
                head: self.last,
            }
        } else {
            Release::Click {
                side: self.side,
                line: self.pressed,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lines(start: u32, end: u32) -> LineRange {
        LineRange::new(start, end).unwrap()
    }

    /// Press on `(side, pressed)`, then pass each `(old, new)` row in turn.
    fn drag(side: Side, pressed: u32, rows: &[(Option<u32>, Option<u32>)]) -> Drag {
        let mut drag = Drag::press(side, pressed);
        for &(old, new) in rows {
            drag.over(old, new);
        }
        drag
    }

    /// Rows with only a new-side number, one per number in `numbers`.
    fn new_rows(numbers: impl Iterator<Item = u32>) -> Vec<(Option<u32>, Option<u32>)> {
        numbers.map(|n| (None, Some(n))).collect()
    }

    /// D1 (C1): the new number wins, else the old one, else nothing.
    #[test]
    fn d1_a_rows_numbers_pick_the_new_side_first() {
        assert_eq!(gutter_line(Some(4), Some(7)), Some((Side::New, 7)));
        assert_eq!(gutter_line(None, Some(7)), Some((Side::New, 7)));
        assert_eq!(gutter_line(Some(4), None), Some((Side::Old, 4)));
        assert_eq!(gutter_line(None, None), None);
    }

    /// D2 (FR-018): a press and release with no row passed is a click.
    #[test]
    fn d2_press_and_release_in_place_is_a_click() {
        let drag = Drag::press(Side::New, 12);
        assert_eq!(drag.span(), (Side::New, lines(12, 12)));
        assert_eq!(
            drag.release(),
            Release::Click {
                side: Side::New,
                line: 12
            }
        );
    }

    /// D3 (FR-015, FR-016): dragging down picks every line passed.
    #[test]
    fn d3_dragging_down_picks_the_range() {
        let drag = drag(Side::New, 12, &new_rows(13..=18));
        assert_eq!(drag.span(), (Side::New, lines(12, 18)));
        assert_eq!(
            drag.release(),
            Release::Range {
                side: Side::New,
                anchor: 12,
                head: 18
            }
        );
    }

    /// D4: dragging up picks the same lines, with the head where it was released.
    #[test]
    fn d4_dragging_up_picks_the_range_in_either_direction() {
        let drag = drag(Side::New, 18, &new_rows((12..=17).rev()));
        assert_eq!(drag.span(), (Side::New, lines(12, 18)));
        assert_eq!(
            drag.release(),
            Release::Range {
                side: Side::New,
                anchor: 18,
                head: 12
            }
        );
    }

    /// D5 (FR-017): rows without a number on the pressed side neither move the end nor change
    /// the side.
    #[test]
    fn d5_rows_without_the_pressed_sides_number_change_nothing() {
        let drag = drag(
            Side::New,
            12,
            &[(None, Some(13)), (Some(40), None), (None, None)],
        );
        assert_eq!(drag.span(), (Side::New, lines(12, 13)));
        assert_eq!(
            drag.release(),
            Release::Range {
                side: Side::New,
                anchor: 12,
                head: 13
            }
        );
    }

    /// D6: an old-side drag follows the old numbers of context rows, which carry both.
    #[test]
    fn d6_an_old_side_drag_follows_context_rows_old_numbers() {
        let drag = drag(Side::Old, 5, &[(Some(6), Some(9)), (Some(7), Some(10))]);
        assert_eq!(drag.span(), (Side::Old, lines(5, 7)));
        assert_eq!(
            drag.release(),
            Release::Range {
                side: Side::Old,
                anchor: 5,
                head: 7
            }
        );
    }

    /// D7 (research R7): leaving the pressed line and coming back is still a range, so the
    /// composer opens.
    #[test]
    fn d7_coming_back_to_the_pressed_line_is_still_a_range() {
        let drag = drag(Side::New, 12, &new_rows([13, 12].into_iter()));
        assert_eq!(drag.span(), (Side::New, lines(12, 12)));
        assert_eq!(
            drag.release(),
            Release::Range {
                side: Side::New,
                anchor: 12,
                head: 12
            }
        );
    }

    /// D8 (FR-016): the span shown always runs from the pressed line to the last line passed.
    #[test]
    fn d8_the_span_follows_the_pointer_line_by_line() {
        let mut drag = Drag::press(Side::New, 12);
        for (n, expected) in [
            (13, (12, 13)),
            (15, (12, 15)),
            (10, (10, 12)),
            (12, (12, 12)),
        ] {
            drag.over(None, Some(n));
            assert_eq!(
                drag.span(),
                (Side::New, lines(expected.0, expected.1)),
                "after passing {n}"
            );
        }
    }
}
