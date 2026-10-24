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
//! dialogs, no sessions. The vocabulary is `FieldId`, which `window` owns, and — since feature 028
//! — `InstallLocation`, which is a `micold-core` type this feature takes as its input rather than
//! another feature's state.

use micold_client::app::State;
use micold_client::features::window::{self, FieldId};
use micold_client::features::Outcome;
use micold_core::install_location::InstallLocation;

#[test]
fn focus_moves_to_whichever_field_reports_gaining_it() {
    let mut st = State::default();

    window::field_focus_changed(&mut st, FieldId::RenameProjectName, true);

    assert_eq!(st.window.focused_field, Some(FieldId::RenameProjectName));
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
        st.window.focused_field,
        Some(FieldId::AddWorktreeTicket),
        "the field that already lost focus cannot take it away from the one that has it"
    );

    window::field_focus_changed(&mut st, FieldId::AddWorktreeTicket, false);
    assert_eq!(
        st.window.focused_field, None,
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

    assert_eq!(st.window.focused_field, None);

    // And it is safe to call when nothing holds the keyboard at all.
    window::field_focus_cleared(&mut st);
    assert_eq!(st.window.focused_field, None);
}

#[test]
fn at_most_one_field_can_hold_the_keyboard() {
    // `Option<FieldId>` rather than a focus flag on each draft: two fields focused at once is not
    // representable, so nothing has to keep four booleans in step.
    let mut st = State::default();

    window::field_focus_changed(&mut st, FieldId::SettingsEnvIncludePath, true);
    window::field_focus_changed(&mut st, FieldId::SettingsEnvIncludeTimeout, true);

    assert_eq!(
        st.window.focused_field,
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
    assert_eq!(
        st.window.window_size,
        (0, 0),
        "unknown until the window says"
    );

    window::resized(&mut st, 1280, 720);

    assert_eq!(st.window.window_size, (1280, 720));
}

#[test]
fn a_copy_that_is_not_installed_blocks_the_session_ui() {
    // FR-019. Both non-`Installed` answers block, for the same reason and with different
    // explanations: a mounted image is ejected, a translocated copy is discarded on close, and
    // either way everything done in the meantime goes with it.
    for location in [InstallLocation::MountedImage, InstallLocation::Translocated] {
        let mut st = State::default();
        window::install_location_reported(&mut st, location);

        assert!(
            st.install_blocked(),
            "{location:?} must not be allowed to look like an installed application"
        );
    }
}

#[test]
fn an_installed_copy_is_not_blocked_and_neither_is_a_fresh_state() {
    // The default matters as much as the verdict: off macOS `Installed` is the only reachable
    // answer, so a state built before anyone asked must not be a state that refuses to start.
    assert!(!State::default().install_blocked());

    let mut st = State::default();
    window::install_location_reported(&mut st, InstallLocation::Installed);
    assert!(!st.install_blocked());
}

#[test]
fn nothing_this_feature_does_clears_the_block() {
    // There is no "dismiss" reducer, and this is the check that none appears by accident: every
    // other operation the window owns runs against a blocked state, and the block survives all of
    // them. A warning the user can click past is a warning the user learns to click past.
    let mut st = State::default();
    window::install_location_reported(&mut st, InstallLocation::MountedImage);

    window::resized(&mut st, 1280, 720);
    window::field_focus_changed(&mut st, FieldId::SettingsScrollback, true);
    window::field_focus_cleared(&mut st);
    st.clear_for_dialog();

    assert!(st.install_blocked());
}

#[test]
fn exactly_one_place_in_the_crate_writes_the_verdict() {
    // The test above can only speak for the reducers it can name. This one speaks for the crate:
    // the field is written in one function, in this feature, and a second writer anywhere -- a
    // dismissal on a button, a reset in a settings load -- fails here rather than quietly making
    // FR-019's "no dismissal" untrue.
    //
    // Text over types, for the same reason `feature_write_isolation.rs` is: the property is about
    // what a *source file* is allowed to do, and nothing in the type system says it.
    let src = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut writers: Vec<String> = Vec::new();
    let mut stack = vec![src.clone()];
    while let Some(dir) = stack.pop() {
        for entry in std::fs::read_dir(&dir).expect("read src") {
            let path = entry.expect("dir entry").path();
            if path.is_dir() {
                stack.push(path);
            } else if path.extension().is_some_and(|e| e == "rs") {
                let code = std::fs::read_to_string(&path).expect("read source");
                for line in code.lines() {
                    let line = line.trim_start();
                    if line.starts_with("//") {
                        continue;
                    }
                    if line.contains("install_location =") {
                        let rel = path.strip_prefix(&src).unwrap_or(&path);
                        writers.push(format!(
                            "{}: {line}",
                            rel.display().to_string().replace('\\', "/")
                        ));
                    }
                }
            }
        }
    }

    assert_eq!(
        writers.len(),
        1,
        "the install-location verdict must have exactly one writer, and it is \
         `features/window.rs::install_location_reported`. Found: {writers:#?}"
    );
    assert!(
        writers[0].starts_with("features/window.rs:"),
        "found the writer somewhere else: {:?}",
        writers[0]
    );
}

#[test]
fn losing_focus_reports_the_field_left_even_when_the_guard_ignores_it() {
    // Feature 937 (R3): "the user left this field" is true whether or not the guard lets the blur
    // clear `focused_field`, so the outcome is returned in both cases.
    let mut st = State::default();
    window::field_focus_changed(&mut st, FieldId::AddWorktreeName, true);
    window::field_focus_changed(&mut st, FieldId::AddWorktreeTicket, true);

    let ignored = window::field_focus_changed(&mut st, FieldId::AddWorktreeName, false);
    assert_eq!(
        st.window.focused_field,
        Some(FieldId::AddWorktreeTicket),
        "the guard left focus alone"
    );
    assert!(matches!(
        ignored.as_slice(),
        [Outcome::FieldBlurred(FieldId::AddWorktreeName)]
    ));

    let believed = window::field_focus_changed(&mut st, FieldId::AddWorktreeTicket, false);
    assert!(matches!(
        believed.as_slice(),
        [Outcome::FieldBlurred(FieldId::AddWorktreeTicket)]
    ));
}

#[test]
fn gaining_focus_reports_no_blur() {
    let mut st = State::default();
    assert!(window::field_focus_changed(&mut st, FieldId::RunName, true).is_empty());
}

#[path = "support/mod.rs"]
mod support;

/// Feature 1124 (spec edge case, T010): the install-location screen is the one view that draws no
/// status bar — it returns before the window column the bar is mounted in.
#[test]
fn the_install_location_screen_draws_no_status_bar() {
    use support::layout::{painted_text, renderer, view_of, StateUnderTest};

    let mut blocked = State::default();
    window::install_location_reported(&mut blocked, InstallLocation::MountedImage);
    let under = StateUnderTest::new(blocked);
    let drawn: Vec<String> = painted_text(view_of(&under), &mut renderer())
        .into_iter()
        .map(|p| p.content)
        .collect();
    assert!(
        !drawn
            .iter()
            .any(|t| t.starts_with("Daemon ") || t.starts_with("Sandbox ")),
        "the install-location screen draws the status bar: {drawn:?}"
    );

    // The premise: an ordinary window does draw it.
    let under = StateUnderTest::new(State::default());
    let drawn: Vec<String> = painted_text(view_of(&under), &mut renderer())
        .into_iter()
        .map(|p| p.content)
        .collect();
    assert!(
        drawn.iter().any(|t| t == "Daemon connected · local"),
        "an ordinary window draws the status bar: {drawn:?}"
    );
}
