//! The showcase's reducer (feature 020, T005 — Principle I).
//!
//! Every state transition the showcase has lives here and is driven directly, so the `view` that
//! consumes it can be the thin glue Principle I's exception covers. There is no lifecycle to model:
//! each message is a toggle or a counter bump, nothing is asynchronous, and nothing can fail. That is
//! not a gap — it is what makes "the same content on every launch" (FR-022, SC-010) hold by
//! construction rather than by arrangement.
//!
//! The one property worth stating as a property: a message naming an entry that does not exist is
//! ignored rather than panicking. The gallery indexes these vectors by catalogue position, and a
//! stale index is the shape a reordered catalogue would take.

use micold_client::showcase::state::{Floating, Message, Showcase};
use micold_core::theme::ColorScheme;
use micold_core::typeahead::Direction;

/// A gallery with a handful of entries — enough that "only entry `i` moved" means something.
const ENTRIES: usize = 4;

fn showcase() -> Showcase {
    Showcase::new(ENTRIES)
}

// ---------------------------------------------------------------------------------------------
// At rest
// ---------------------------------------------------------------------------------------------

/// The starting state, in full. A gallery that opened mid-transition, or with something already
/// running, would burn frames before the developer had asked for anything (FR-023, SC-009).
#[test]
fn a_fresh_showcase_is_at_rest() {
    let s = showcase();
    assert_eq!(s.scheme, ColorScheme::Light);
    assert!(s.open.is_none(), "nothing is open on launch");
    assert!(
        !s.typeahead_open(),
        "the type-ahead's list starts closed — the state the branch picker rests in. An entry that \
         could not be closed would teach the opposite of FR-001b (BUG-001, FR-020a)"
    );
    for i in 0..ENTRIES {
        assert_eq!(s.replays(i), 0, "entry {i} starts with no replay behind it");
        assert!(!s.running(i), "entry {i} starts stopped (FR-023a)");
        assert!(
            s.shown(i),
            "entry {i} starts settled and visible — an element built already-open must not animate \
             into existence"
        );
    }
}

/// The vectors are sized from the entry count, so an index derived from a catalogue position is
/// always in range for the catalogue it came from.
#[test]
fn the_per_entry_state_is_sized_from_the_entry_count() {
    assert_eq!(Showcase::new(0).len(), 0);
    assert_eq!(Showcase::new(38).len(), 38);
}

/// The scheme control says what pressing it will do, not what is currently in force. A control
/// labelled with the state it is already in is the classic toggle bug, and the label is a decision
/// about state — which is why it lives in the reducer rather than in the view (`showcase_glue.rs`).
#[test]
fn the_scheme_control_names_the_scheme_it_would_switch_to() {
    let mut s = showcase();
    assert_eq!(s.scheme_control_label(), "Switch to dark");
    s.update(Message::SchemeToggled);
    assert_eq!(s.scheme_control_label(), "Switch to light");
}

// ---------------------------------------------------------------------------------------------
// Replay and reverse (FR-007b)
// ---------------------------------------------------------------------------------------------

/// "Play it again" is a changed identity, nothing more: the wrapper sees a different
/// `restart_on(key)` and replays from zero. So the only thing the reducer has to do is move.
#[test]
fn replaying_bumps_that_entrys_counter() {
    let mut s = showcase();
    s.update(Message::Replayed(1));
    assert_eq!(s.replays(1), 1);
    s.update(Message::Replayed(1));
    assert_eq!(
        s.replays(1),
        2,
        "replay is repeatable, as many times as asked"
    );
}

#[test]
fn replaying_leaves_every_other_entry_alone() {
    let mut s = showcase();
    s.update(Message::Replayed(1));
    for i in [0, 2, 3] {
        assert_eq!(
            s.replays(i),
            0,
            "entry {i} moved when entry 1 was replayed — the two share state"
        );
    }
}

/// Replay plays the *entrance*. Pressing it on an entry someone had reversed must not replay the
/// exit, or the control would do different things depending on invisible history.
#[test]
fn replaying_a_reversed_entry_plays_the_entrance() {
    let mut s = showcase();
    s.update(Message::Reversed(2));
    assert!(!s.shown(2), "precondition: it is on its way out");
    s.update(Message::Replayed(2));
    assert!(
        s.shown(2),
        "replay must restore the destination it plays toward"
    );
}

#[test]
fn reversing_flips_only_that_entry() {
    let mut s = showcase();
    s.update(Message::Reversed(0));
    assert!(!s.shown(0));
    for i in 1..ENTRIES {
        assert!(s.shown(i), "entry {i} flipped when entry 0 was reversed");
    }
    s.update(Message::Reversed(0));
    assert!(s.shown(0), "reversing again brings it back");
}

// ---------------------------------------------------------------------------------------------
// The run control (FR-023a)
// ---------------------------------------------------------------------------------------------

/// No component in the library runs continuously yet, so nothing uses this at delivery. The
/// mechanism is here because 018's indeterminate indicator is the first, and because a gallery that
/// displayed one running with nothing running would be showing a defect as if it were a feature.
#[test]
fn the_run_control_starts_and_stops_one_entry() {
    let mut s = showcase();
    s.update(Message::RunToggled(3));
    assert!(s.running(3));
    for i in 0..3 {
        assert!(!s.running(i), "entry {i} started when entry 3 was asked to");
    }
    s.update(Message::RunToggled(3));
    assert!(
        !s.running(3),
        "and it stops again — at rest it asks for no frames"
    );
}

// ---------------------------------------------------------------------------------------------
// Floating surfaces (FR-007, and the deadlock Edge Case)
// ---------------------------------------------------------------------------------------------

#[test]
fn opening_a_surface_records_which_one() {
    let mut s = showcase();
    s.update(Message::Opened(Floating::Modal));
    assert_eq!(s.open, Some(Floating::Modal));
}

/// The spec's Edge Case — "two floating components could be opened at once … must not deadlock
/// itself into a state where a surface cannot be dismissed" — is answered by the type: `open` holds
/// one. Opening a second replaces the first rather than stacking behind it.
#[test]
fn opening_a_second_surface_replaces_the_first() {
    let mut s = showcase();
    s.update(Message::Opened(Floating::Menu));
    s.update(Message::Opened(Floating::ProjectSwitcher));
    assert_eq!(s.open, Some(Floating::ProjectSwitcher));
}

#[test]
fn dismissing_closes_whatever_was_open() {
    let mut s = showcase();
    s.update(Message::Opened(Floating::ContextMenu));
    s.update(Message::Dismissed);
    assert!(s.open.is_none());
}

#[test]
fn dismissing_nothing_is_harmless() {
    let mut s = showcase();
    s.update(Message::Dismissed);
    assert!(s.open.is_none());
}

// ---------------------------------------------------------------------------------------------
// Robustness
// ---------------------------------------------------------------------------------------------

/// A message naming an entry the catalogue no longer has must be ignored, not fatal. The gallery
/// indexes by catalogue position; a stale index is the shape a reordered catalogue takes, and a
/// showcase that panicked on one would be less useful than the page it replaced.
#[test]
fn a_message_for_an_entry_that_does_not_exist_is_ignored() {
    let mut s = showcase();
    s.update(Message::Replayed(ENTRIES));
    s.update(Message::Reversed(9_999));
    s.update(Message::RunToggled(usize::MAX));
    assert_eq!(s.len(), ENTRIES, "the state did not grow to accommodate it");
}

/// Components in the gallery are genuinely interactive (FR-002), so their messages have to go
/// somewhere. They go nowhere, and that is the point: a catalogue has no behaviour to invoke.
///
/// Compared field by field rather than with `assert_eq!` on the whole value: `Showcase` holds the
/// sample `GridCache`, which is `Clone` but not `PartialEq`, and deriving equality for a render cache
/// to satisfy one test would be the tail wagging the dog.
#[test]
fn no_op_changes_nothing() {
    let mut s = showcase();
    let before = snapshot(&s);
    s.update(Message::NoOp);
    assert_eq!(snapshot(&s), before);
}

// --- the type-ahead example (feature 021, US3) -------------------------------------------------

/// The gallery's type-ahead is a *live* example, not a picture of one (FR-020), so it owns a query
/// of its own — and that makes typing into it a state transition like any other, which is why it is
/// tested here rather than left to whoever clicks on it.
#[test]
fn typing_into_the_typeahead_narrows_the_sample_rows() {
    let mut s = showcase();
    assert_eq!(
        s.typeahead_rows().len(),
        micold_client::showcase::samples::SEARCH_RESULTS.len(),
        "an empty query offers everything"
    );

    s.update(Message::TypeaheadQueryChanged("log".into()));

    assert_eq!(s.typeahead_query(), "log");
    let rows = s.typeahead_rows();
    assert!(
        rows.len() < micold_client::showcase::samples::SEARCH_RESULTS.len(),
        "the query narrows the list"
    );
    assert!(
        rows.iter().all(|r| !r.spans.is_empty()),
        "every surviving row says which characters put it there"
    );
}

/// The highlight moves the same way the picker's does, because it is the same rule — the gallery
/// example would be worth little if its keyboard behaved differently from the real one.
#[test]
fn the_typeahead_highlight_moves_and_stops_at_the_ends() {
    let mut s = showcase();
    assert_eq!(
        s.typeahead_highlight(),
        None,
        "nothing is highlighted at rest"
    );

    s.update(Message::TypeaheadHighlightMoved(Direction::Next));
    assert_eq!(
        s.typeahead_highlight(),
        Some(0),
        "the first move enters the list"
    );

    let last = s.typeahead_rows().len() - 1;
    for _ in 0..s.typeahead_rows().len() + 3 {
        s.update(Message::TypeaheadHighlightMoved(Direction::Next));
    }
    assert_eq!(
        s.typeahead_highlight(),
        Some(last),
        "it stops at the end rather than wrapping"
    );

    for _ in 0..last + 3 {
        s.update(Message::TypeaheadHighlightMoved(Direction::Prev));
    }
    assert_eq!(s.typeahead_highlight(), Some(0), "and at the start");
}

/// A highlight left pointing past the end of a shrinking list is the bug the picker's reducer also
/// guards against; the gallery re-seats it the same way.
#[test]
fn narrowing_the_typeahead_reseats_a_dangling_highlight() {
    let mut s = showcase();
    for _ in 0..s.typeahead_rows().len() {
        s.update(Message::TypeaheadHighlightMoved(Direction::Next));
    }
    let was = s.typeahead_highlight().expect("a highlight to strand");

    s.update(Message::TypeaheadQueryChanged("log".into()));

    let highlight = s.typeahead_highlight();
    if let Some(i) = highlight {
        assert!(
            i < s.typeahead_rows().len(),
            "the highlight was left at {was} and now points past the {} remaining rows",
            s.typeahead_rows().len()
        );
    }
}

/// A pick registers, so the example can show a selection marker — the third thing a row says at
/// once, and the one that needs somewhere to be remembered.
#[test]
fn picking_a_typeahead_row_registers_the_selection() {
    let mut s = showcase();
    let chosen = s.typeahead_rows()[1].label.clone();

    s.update(Message::TypeaheadPicked(1));
    assert_eq!(s.typeahead_selected(), Some(1));

    // And a pick is the only thing that writes it: typing does not clear it.
    s.update(Message::TypeaheadQueryChanged("log".into()));
    assert!(
        s.typeahead_selected().is_some(),
        "the choice survives the search"
    );
    assert_eq!(
        s.typeahead_rows()[s.typeahead_selected().unwrap()].label,
        chosen,
        "and the marker is still on the row that was chosen, not on whatever is third now"
    );
}

/// The marker follows the row it was put on, rather than the position that row happened to occupy.
/// Stored as an index it would slide onto an unrelated branch the moment a search reordered the
/// list — the failure a developer reading this page would take to be how the real picker behaves.
#[test]
fn the_typeahead_marker_stays_on_the_row_that_was_chosen() {
    let mut s = showcase();
    let last = s.typeahead_rows().len() - 1;
    let chosen = s.typeahead_rows()[last].label.clone();
    s.update(Message::TypeaheadPicked(last));

    // Narrow to a list the chosen row is not in: the marker has nowhere to sit and says so, rather
    // than marking whatever now occupies that position.
    s.update(Message::TypeaheadQueryChanged("logout".into()));
    let rows = s.typeahead_rows();
    if !rows.iter().any(|r| r.label == chosen) {
        assert_eq!(s.typeahead_selected(), None, "no row is the chosen one");
    }

    // Widen again and it comes back on the same row.
    s.update(Message::TypeaheadQueryChanged(String::new()));
    assert_eq!(
        s.typeahead_rows()[s.typeahead_selected().unwrap()].label,
        chosen
    );
}

/// The entry opens the way the picker does: reaching the field is enough, before anything is typed.
///
/// The gallery example used to be handed `open(true)` and nothing else, so this transition existed in
/// the application and nowhere on the page that documents the component (BUG-001).
#[test]
fn reaching_the_typeahead_field_opens_its_list() {
    let mut s = showcase();
    s.update(Message::TypeaheadFocused);
    assert!(s.typeahead_open(), "reaching the field opens the list");
}

/// Typing opens it too, so a developer who starts typing into a dismissed field sees results again
/// rather than typing into a box that answers nothing. `app.rs` opens on a query change for exactly
/// this reason, and the rule here is that rule rather than a second one.
#[test]
fn typing_into_the_typeahead_opens_its_list() {
    let mut s = showcase();
    s.update(Message::TypeaheadDismissed);
    s.update(Message::TypeaheadQueryChanged("log".into()));
    assert!(s.typeahead_open(), "typing reopens a dismissed list");
}

/// Picking is terminal: the choice is registered *and* the list closes, in one step.
#[test]
fn picking_a_typeahead_row_closes_the_list() {
    let mut s = showcase();
    s.update(Message::TypeaheadFocused);
    s.update(Message::TypeaheadPicked(1));
    assert!(!s.typeahead_open(), "a pick closes the list");
    assert_eq!(
        s.typeahead_selected(),
        Some(1),
        "and still registers the choice — closing must not cost the selection"
    );
}

/// Dismissing closes it and changes nothing else — the query and the choice both survive, so a
/// developer who dismisses by accident has lost no work.
#[test]
fn dismissing_the_typeahead_closes_its_list() {
    let mut s = showcase();
    s.update(Message::TypeaheadFocused);
    s.update(Message::TypeaheadQueryChanged("log".into()));
    s.update(Message::TypeaheadPicked(0));
    let chosen = s.typeahead_selected();

    s.update(Message::TypeaheadFocused);
    s.update(Message::TypeaheadDismissed);
    assert!(!s.typeahead_open(), "dismissal closes the list");
    assert_eq!(s.typeahead_query(), "log", "the search text survives");
    assert_eq!(s.typeahead_selected(), chosen, "so does the choice");
}

/// Everything about the showcase a message could change.
fn snapshot(
    s: &Showcase,
) -> (
    ColorScheme,
    Option<Floating>,
    Vec<u64>,
    Vec<bool>,
    Vec<bool>,
) {
    (
        s.scheme,
        s.open,
        (0..s.len()).map(|i| s.replays(i)).collect(),
        (0..s.len()).map(|i| s.running(i)).collect(),
        (0..s.len()).map(|i| s.shown(i)).collect(),
    )
}
