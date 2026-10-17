//! Keyboard navigation's step rule (feature 803, FR-005): which entry of an ordered list a
//! next/previous press lands on. Render-free, so every level (sessions, locations, projects)
//! shares one wrap rule that is tested here once.

/// The direction of a navigation press.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Dir {
    /// The entry after the current one (wraps from the last to the first).
    Next,
    /// The entry before the current one (wraps from the first to the last).
    Previous,
}

/// The entry of `order` a press in `dir` lands on from `current`.
///
/// An empty list gives `None`. With `current` in `order` the neighbour in `dir` is returned,
/// wrapping at both ends, so a one-entry list returns that entry. With `current` `None` or not in
/// `order`, `Next` gives the first entry and `Previous` the last.
pub fn step<T: PartialEq + Clone>(order: &[T], current: Option<&T>, dir: Dir) -> Option<T> {
    let len = order.len();
    if len == 0 {
        return None;
    }
    let index = current.and_then(|c| order.iter().position(|entry| entry == c));
    let target = match (index, dir) {
        (Some(i), Dir::Next) => (i + 1) % len,
        (Some(i), Dir::Previous) => (i + len - 1) % len,
        (None, Dir::Next) => 0,
        (None, Dir::Previous) => len - 1,
    };
    order.get(target).cloned()
}

#[cfg(test)]
mod tests {
    use super::*;

    const ORDER: [char; 4] = ['a', 'b', 'c', 'd'];

    #[test]
    fn next_and_previous_from_the_middle_land_on_the_neighbours() {
        assert_eq!(
            step(&ORDER, Some(&'b'), Dir::Next),
            Some('c'),
            "Next steps one forward"
        );
        assert_eq!(
            step(&ORDER, Some(&'b'), Dir::Previous),
            Some('a'),
            "Previous steps one back"
        );
    }

    #[test]
    fn stepping_past_either_end_wraps() {
        assert_eq!(
            step(&ORDER, Some(&'d'), Dir::Next),
            Some('a'),
            "Next on the last wraps to the first"
        );
        assert_eq!(
            step(&ORDER, Some(&'a'), Dir::Previous),
            Some('d'),
            "Previous on the first wraps to the last"
        );
    }

    #[test]
    fn a_one_entry_list_stays_on_its_entry() {
        for dir in [Dir::Next, Dir::Previous] {
            assert_eq!(
                step(&['x'], Some(&'x'), dir),
                Some('x'),
                "{dir:?} keeps the only entry"
            );
        }
    }

    #[test]
    fn an_empty_list_gives_nothing() {
        for dir in [Dir::Next, Dir::Previous] {
            assert_eq!(
                step::<char>(&[], None, dir),
                None,
                "{dir:?} on an empty list does nothing"
            );
            assert_eq!(
                step(&[], Some(&'a'), dir),
                None,
                "{dir:?} on an empty list does nothing"
            );
        }
    }

    #[test]
    fn no_current_entry_starts_at_the_end_the_press_points_to() {
        assert_eq!(
            step(&ORDER, None, Dir::Next),
            Some('a'),
            "Next with no current → first"
        );
        assert_eq!(
            step(&ORDER, None, Dir::Previous),
            Some('d'),
            "Previous with no current → last"
        );
    }

    #[test]
    fn a_current_entry_missing_from_the_order_counts_as_none() {
        assert_eq!(
            step(&ORDER, Some(&'z'), Dir::Next),
            Some('a'),
            "absent current, Next → first"
        );
        assert_eq!(
            step(&ORDER, Some(&'z'), Dir::Previous),
            Some('d'),
            "absent current, Previous → last"
        );
    }

    /// Presses from `from` to `to` in `dir`, or `None` when never reached within `n` presses.
    fn presses(order: &[usize], from: usize, to: usize, dir: Dir) -> Option<usize> {
        let mut at = from;
        for count in 0..=order.len() {
            if at == to {
                return Some(count);
            }
            at = step(order, Some(&at), dir)?;
        }
        None
    }

    #[test]
    fn every_entry_is_reached_within_half_the_list_in_the_shorter_direction() {
        // SC-001: wrap in both directions puts any target at most ⌊N/2⌋ presses away.
        for n in 1..=6 {
            let order: Vec<usize> = (0..n).collect();
            for from in 0..n {
                for to in 0..n {
                    let shortest = [Dir::Next, Dir::Previous]
                        .into_iter()
                        .filter_map(|dir| presses(&order, from, to, dir))
                        .min();
                    assert!(
                        shortest.is_some_and(|p| p <= n / 2),
                        "N={n}: {from}→{to} took {shortest:?} presses, more than ⌊N/2⌋"
                    );
                }
            }
        }
    }
}
