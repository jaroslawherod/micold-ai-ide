//! The window: its size, the pointer in it, and which field holds the keyboard, in isolation
//! (feature 021, SC-004).
//!
//! # Why this file did not exist until T071
//!
//! SC-004 names *eight* feature modules, which was the count when it was written. There are ten:
//! T031 created `features/help.rs` for the homeless overflow menu and T063 created this one for
//! three fields the root reducer was still deciding about. Neither got an isolation test at the
//! time, because neither existed when the list of eight was drawn up — which is exactly how a
//! success criterion phrased as a number goes quietly out of date. T071 found the gap; this file
//! and `features_help.rs` close it.
//!
//! Same caveat as `features_session.rs`: these reducers take `&mut State`, so the file builds one.
//! What it holds to is the other half of SC-004 — it names no other feature's types. No drafts, no
//! dialogs, no sessions; the only vocabulary here is `FieldId`, which `window` owns.

use micold_client::app::State;
use micold_client::features::window::{self, FieldId};

#[test]
fn focus_moves_to_whichever_field_reports_gaining_it() {
    let mut st = State::default();

    window::field_focus_changed(&mut st, FieldId::RenameProjectName, true);

    assert_eq!(st.focused_field, Some(FieldId::RenameProjectName));
}

#[test]
fn a_blur_is_believed_only_from_the_field_that_holds_focus() {
    // Gaining and losing are reported by two different widgets and arrive in whichever order the
    // frame produced them. An unguarded `None` on the way out of the old field would erase the
    // focus the new one had already claimed, and clicking straight from one field to another would
    // leave both drawn at rest — the bug this guard exists for.
    let mut st = State::default();
    window::field_focus_changed(&mut st, FieldId::AddWorktreeName, true);
    window::field_focus_changed(&mut st, FieldId::AddWorktreeTicket, true);

    window::field_focus_changed(&mut st, FieldId::AddWorktreeName, false);

    assert_eq!(
        st.focused_field,
        Some(FieldId::AddWorktreeTicket),
        "the field that already lost focus cannot take it away from the one that has it"
    );

    window::field_focus_changed(&mut st, FieldId::AddWorktreeTicket, false);
    assert_eq!(
        st.focused_field, None,
        "the field that does hold it is believed"
    );
}

#[test]
fn clearing_focus_is_unconditional_where_a_blur_is_not() {
    // The asymmetry is the point, and it is why `field_focus_cleared` is a second function rather
    // than `field_focus_changed(.., false)`. A blur answers "did *this* field lose it?"; a clear
    // answers "nothing holds it now", which is what a terminal taking the keyboard means. Guarding
    // it the same way would let whichever field still believed it had focus defeat the press.
    let mut st = State::default();
    window::field_focus_changed(&mut st, FieldId::SettingsScrollback, true);

    window::field_focus_cleared(&mut st);

    assert_eq!(st.focused_field, None);

    // And it is safe to call when nothing holds the keyboard at all.
    window::field_focus_cleared(&mut st);
    assert_eq!(st.focused_field, None);
}

#[test]
fn at_most_one_field_can_hold_the_keyboard() {
    // `Option<FieldId>` rather than a focus flag on each draft: two fields focused at once is not
    // representable, so nothing has to keep four booleans in step.
    let mut st = State::default();

    window::field_focus_changed(&mut st, FieldId::SettingsEnvIncludePath, true);
    window::field_focus_changed(&mut st, FieldId::SettingsEnvIncludeTimeout, true);

    assert_eq!(
        st.focused_field,
        Some(FieldId::SettingsEnvIncludeTimeout),
        "the later claim replaces the earlier one — there is one slot"
    );
}

#[test]
fn the_window_size_is_recorded_as_reported() {
    // Not chosen by the user and not persisted: reported by the windowing system, and held so a
    // context menu can be clamped inside the window.
    //
    // This covered the tracked pointer too until `main`'s 018 BUG-008 fix deleted it. A menu now
    // anchors at the point its own press landed on, carried by the message, rather than at a
    // position tracked separately and read later — which is a better answer than the one this
    // feature was defending, and leaves `window` owning two fields rather than three.
    let mut st = State::default();
    assert_eq!(st.window_size, (0, 0), "unknown until the window says");

    window::resized(&mut st, 1280, 720);

    assert_eq!(st.window_size, (1280, 720));
}
