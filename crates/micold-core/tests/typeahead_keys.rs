//! What each key means (feature 021, contract `match-ranking.md` §4b — FR-017, FR-017a).
//!
//! These are rules, not glue. "Down stops at the last row rather than wrapping" and "Enter on an
//! unavailable row does nothing" are decisions with consequences a user can see, and Principle I's
//! GUI exception covers only code that has no decisions in it. So the rule lives here, where a test
//! can reach it, and the widget is left translating events in and applying answers out — the same
//! line `micold-client`'s `keymap.rs` draws for the terminal.

use micold_core::typeahead::{intent_for, move_highlight, Direction, Intent, Key};

/// The common case: a list of five rows with the third highlighted and available.
fn mid() -> (Option<usize>, usize, bool) {
    (Some(2), 5, true)
}

/// Down and Up move, and say which way.
#[test]
fn the_arrows_move_the_highlight() {
    let (h, n, ok) = mid();
    assert_eq!(intent_for(Key::Down, h, n, ok), Some(Intent::Move(Direction::Next)));
    assert_eq!(intent_for(Key::Up, h, n, ok), Some(Intent::Move(Direction::Prev)));
}

/// Q4b.1 — the end of the list is the end. Wrapping around is disorienting in a list whose length
/// changes on every keystroke: the developer would have no idea where they had arrived.
#[test]
fn moving_stops_at_the_ends_rather_than_wrapping() {
    assert_eq!(intent_for(Key::Down, Some(4), 5, true), None, "already at the last row");
    assert_eq!(intent_for(Key::Up, Some(0), 5, true), None, "already at the first row");
}

/// With nothing highlighted yet, the first press has somewhere to go — that is how the keyboard
/// enters a list it has not touched.
#[test]
fn moving_with_no_highlight_enters_the_list() {
    assert_eq!(intent_for(Key::Down, None, 5, false), Some(Intent::Move(Direction::Next)));
    assert_eq!(intent_for(Key::Up, None, 5, false), Some(Intent::Move(Direction::Prev)));
}

/// Q4b.4 — an empty list has nowhere to move to. Returning a move here would leave the caller
/// computing a highlight for a list with no rows.
#[test]
fn moving_in_an_empty_list_does_nothing() {
    assert_eq!(intent_for(Key::Down, None, 0, false), None);
    assert_eq!(intent_for(Key::Up, None, 0, false), None);
    assert_eq!(intent_for(Key::Down, Some(0), 0, false), None);
}

/// Enter takes the highlighted row.
#[test]
fn enter_picks_the_highlighted_row() {
    let (h, n, ok) = mid();
    assert_eq!(intent_for(Key::Enter, h, n, ok), Some(Intent::Pick));
}

/// Q4b.2 — and refuses an unavailable one. This is FR-012a arriving through the keyboard: the
/// refusal has to be in both routes to a pick, or the mouse enforces a rule the keyboard ignores.
#[test]
fn enter_on_an_unavailable_row_does_nothing() {
    assert_eq!(
        intent_for(Key::Enter, Some(2), 5, false),
        None,
        "an unavailable row can be read but never chosen"
    );
}

/// Q4b.3 — nothing highlighted, nothing to pick. Not a dismissal either: Enter that did nothing
/// visible must not close the list out from under the developer.
#[test]
fn enter_with_no_highlight_does_nothing() {
    assert_eq!(intent_for(Key::Enter, None, 5, true), None);
}

/// Escape closes the list, whatever the highlight is doing.
#[test]
fn escape_dismisses() {
    assert_eq!(intent_for(Key::Escape, Some(2), 5, true), Some(Intent::Dismiss));
    assert_eq!(intent_for(Key::Escape, None, 0, false), Some(Intent::Dismiss));
}

/// Q4b.5 — everything else belongs to the field. This is what "the developer never has to leave
/// the search field" actually means: the list may only consume the keys it has a use for.
#[test]
fn every_other_key_falls_through_to_the_field() {
    for highlight in [None, Some(0), Some(4)] {
        for rows in [0usize, 5] {
            for enabled in [true, false] {
                assert_eq!(
                    intent_for(Key::Other, highlight, rows, enabled),
                    None,
                    "an ordinary key must reach the field (highlight={highlight:?}, rows={rows}, enabled={enabled})"
                );
            }
        }
    }
}

/// Where a move lands, as opposed to whether one happens. Two reducers used to answer this
/// themselves and disagreed about `Up` from nowhere; one function means they cannot.
#[test]
fn a_move_lands_where_the_rule_says() {
    assert_eq!(move_highlight(Some(2), Direction::Next, 5), Some(3));
    assert_eq!(move_highlight(Some(2), Direction::Prev, 5), Some(1));

    // The ends hold, so a move that had nowhere to go leaves the highlight where it was.
    assert_eq!(move_highlight(Some(4), Direction::Next, 5), Some(4));
    assert_eq!(move_highlight(Some(0), Direction::Prev, 5), Some(0));

    // Entering the list from the field arrives at the near end for the direction travelled.
    assert_eq!(move_highlight(None, Direction::Next, 5), Some(0));
    assert_eq!(move_highlight(None, Direction::Prev, 5), Some(4));

    // Nothing to move through, so nothing to move to.
    assert_eq!(move_highlight(None, Direction::Next, 0), None);
    assert_eq!(move_highlight(Some(3), Direction::Prev, 0), None);

    // A highlight stranded past the end of a shrunken list lands inside it in one press, either way.
    assert_eq!(move_highlight(Some(9), Direction::Next, 3), Some(2));
    assert_eq!(move_highlight(Some(9), Direction::Prev, 3), Some(1));
}

/// The same inputs give the same answer — the function holds no state, so a key cannot mean one
/// thing now and another thing later.
#[test]
fn the_rule_is_a_pure_function_of_its_inputs() {
    let once = intent_for(Key::Down, Some(1), 5, true);
    for _ in 0..8 {
        assert_eq!(intent_for(Key::Down, Some(1), 5, true), once);
    }
}

/// Tab moves focus out of the field, so the list it opened must not survive it. An open list goes
/// on claiming Enter and the arrows from wherever focus went — so a developer who tabbed to the
/// Create button and pressed Enter would pick a branch instead of pressing it.
#[test]
fn tab_closes_the_list_because_focus_is_leaving_the_field() {
    assert_eq!(
        intent_for(Key::Tab, Some(1), 5, true),
        Some(Intent::Dismiss),
        "a highlighted, pickable row does not make Tab mean anything else"
    );
    assert_eq!(intent_for(Key::Tab, None, 5, true), Some(Intent::Dismiss));
    assert_eq!(
        intent_for(Key::Tab, None, 0, false),
        Some(Intent::Dismiss),
        "an open list with no rows still has to close — it is showing the no-match message"
    );
}
