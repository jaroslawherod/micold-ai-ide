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
