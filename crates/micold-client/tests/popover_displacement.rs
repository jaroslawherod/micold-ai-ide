//! Which popover closes which, stated once and in full (feature 021, T067a-2 — FR-020, FR-021).
//!
//! # Why the whole relation and not a rule
//!
//! Until T067a-2 this lived as five toggle reducers each assigning its neighbours' fields: twelve
//! cross-feature writes, the last group of the T067 catalogue, and a rule that could only be read
//! by opening five files and comparing them. It was tempting to replace them with a uniform "at
//! most one popover is open", and that would have been wrong four times over — the project row
//! menu deliberately leaves the switcher it was opened from alone, the worktree menu closes only
//! the project one, and the session and terminal menus close nothing at all.
//!
//! So the mechanism is a declaration — `FloatingSurface::displaces`, applied by
//! `overlay::registry::displace` — and this file is the independent statement of what it should
//! say. [`DISPLACES`] is written from the behaviour, not read out of the registry, and
//! [`the_whole_displacement_relation`] drives all forty-two ordered pairs through `State::update`.
//! A table read back out of the code under test would only ever catch the two halves disagreeing,
//! never both being wrong.
//!
//! # What this adds over the tests that were already here
//!
//! `switcher_forget_menu.rs` pins the switcher exception and the four openers that close the
//! project menu; `keyboard.rs` and `overlay_registry.rs` cover Escape and scroll. None of them
//! says what happens to the *other* thirty pairs, and a displacement quietly widened — every panel
//! popover suddenly closing the session menu, say — would have passed all of them.

use micold_client::app::{Message, State};
use micold_client::ui::terminal::StripTab;
use micold_core::session::{SessionId, ShellInstanceId};
use std::path::PathBuf;

/// Which surfaces each popover closes by opening.
///
/// Read as: opening the surface on the left closes each surface on the right, and leaves every
/// other popover exactly as it was.
const DISPLACES: &[(&str, &[&str])] = &[
    // The three panel popovers are mutually exclusive with each other, and each also closes the
    // project row menu.
    (
        "help_menu",
        &["project_switcher", "sidebar_filter", "project_menu"],
    ),
    (
        "project_switcher",
        &["help_menu", "sidebar_filter", "project_menu"],
    ),
    (
        "sidebar_filter",
        &["help_menu", "project_switcher", "project_menu"],
    ),
    // The project row menu closes two of the three panels and the other context menu. **Not the
    // switcher**: it is opened by right-clicking a row inside the open switcher, and the row list
    // has to stay visible behind it.
    (
        "project_menu",
        &["help_menu", "sidebar_filter", "worktree_menu"],
    ),
    // The two context menus replace each other. A panel popover open elsewhere in the window is
    // unaffected by a right-click in the sidebar.
    ("worktree_menu", &["project_menu"]),
    // None of these has ever closed anything, and this is the first test to say so.
    ("session_menu", &[]),
    ("terminal_context_menu", &[]),
    // Feature 012's terminal-tab menu, which arrived on `main` mid-feature. It replaces itself on a
    // second right-click and touches nothing else — including the pane's own context menu, which is
    // a different surface on the same pane.
    ("shell_instance_menu", &[]),
];

/// The message that opens each popover, by the id it registers under.
fn opener(id: &str) -> Message {
    match id {
        "help_menu" => Message::HelpMenuToggled,
        "project_switcher" => Message::ProjectSwitcherToggled,
        "sidebar_filter" => Message::SidebarFilterMenuToggled,
        "project_menu" => Message::ProjectMenuToggled(PathBuf::from("/a"), (10, 10)),
        "worktree_menu" => Message::WorktreeMenuToggled("w1".into(), (20, 20)),
        "session_menu" => Message::SessionMenuToggled(SessionId::new(), (30, 30)),
        "terminal_context_menu" => Message::TerminalContextMenuOpened { x: 10, y: 20 },
        "shell_instance_menu" => Message::StripTabMenuRequested(StripTab::Instance(ShellInstanceId(1)), 30, 40),
        other => panic!("no opener for `{other}`"),
    }
}

/// The popovers the registry currently reports open, by id.
fn open_popovers(state: &State) -> Vec<&'static str> {
    micold_client::overlay::registry::open_popovers(state)
        .iter()
        .map(|open| open.id().as_str())
        .collect()
}

fn ids() -> Vec<&'static str> {
    DISPLACES.iter().map(|(id, _)| *id).collect()
}

#[test]
fn the_whole_displacement_relation() {
    for (opened, displaced) in DISPLACES {
        for other in ids() {
            if other == *opened {
                continue;
            }
            let mut st = State::default();

            st.update(opener(other));
            assert_eq!(
                open_popovers(&st),
                vec![other],
                "opening `{other}` on its own should leave exactly it open"
            );

            st.update(opener(opened));

            let still_open = open_popovers(&st).contains(&other);
            let expected = !displaced.contains(&other);
            assert_eq!(
                still_open, expected,
                "opening `{opened}` over `{other}`: expected `{other}` to be {} afterwards",
                if expected { "still open" } else { "closed" }
            );
        }
    }
}

#[test]
fn every_popover_is_in_the_table() {
    // The relation is only as complete as the list of surfaces it ranges over: a popover
    // registered later would be silently exempt from all forty-two pairs above, and nothing else
    // in this file could notice. Counted against the registry rather than driven, because the
    // popovers displace each other and no state has them all open at once.
    //
    // Nine of the sixteen registrations are dialogs; `overlay_registry.rs::every_dialog_is_in_the_list`
    // is what holds that half.
    const DIALOGS: usize = 9;
    assert_eq!(
        micold_client::overlay::registry::probes().len(),
        DIALOGS + DISPLACES.len(),
        "a surface was registered that neither DISPLACES nor the dialog list in \
         overlay_registry.rs accounts for — if it is a popover, add its row here and say what it \
         closes, even if the answer is nothing"
    );

    // ...and each opener really does open the one surface it names, so a row that is present but
    // pointed at the wrong message cannot pass the pairs above by accident.
    for id in ids() {
        let mut st = State::default();
        st.update(opener(id));
        assert_eq!(open_popovers(&st), vec![id]);
    }
}

#[test]
fn toggling_a_popover_shut_displaces_nothing() {
    // A behaviour change, and the one this conversion made: the old direct assignments cleared the
    // neighbours on the way *out* as well as on the way in, so shutting the help menu also shut
    // whatever else happened to be open. Nothing tested it either way.
    //
    // Two things hold it now, and only one of them is this test. `surface_opened` reports nothing
    // when the toggle closed its surface, and `registry::displace` would do nothing with the
    // report anyway because it resolves the surface out of the *open* set. So this test survives
    // either mechanism being removed on its own — see
    // `a_toggle_that_shut_its_surface_reports_nothing` for the half no state can observe.
    let mut st = State::default();
    st.update(Message::HelpMenuToggled);
    st.update(Message::HelpMenuToggled);
    assert_eq!(open_popovers(&st), Vec::<&str>::new());

    st.update(Message::SessionMenuToggled(SessionId::new(), (30, 30)));
    st.update(Message::WorktreeMenuToggled("w1".into(), (20, 20)));
    st.update(Message::WorktreeMenuToggled("w1".into(), (20, 20)));

    assert_eq!(
        open_popovers(&st),
        vec!["session_menu"],
        "the worktree menu toggled itself shut and took nothing with it"
    );
}

#[test]
fn a_toggle_that_shut_its_surface_reports_nothing() {
    // The half of the previous test that no state can see. `registry::displace` skips a surface
    // that is not open, so a reducer reporting `SurfaceOpened` for one it just closed changes
    // nothing observable — a probe that made `surface_opened` report unconditionally failed no
    // test in the client suite.
    //
    // That is not a reason to stop caring. An outcome is a feature's statement of what happened,
    // and the whole read/write asymmetry rests on those statements being true; one that is merely
    // harmless today is a trap for the next thing that interprets it. So this reads the return
    // value directly, which is the only place the difference exists.
    let mut st = State::default();

    let opened = micold_client::features::help::menu_toggled(&mut st);
    assert_eq!(opened.len(), 1, "toggling the help menu open reports an opening");

    let closed = micold_client::features::help::menu_toggled(&mut st);
    assert!(
        closed.is_empty(),
        "toggling it shut reports nothing — it did not open, and saying it did would be false \
         whether or not anything acts on it"
    );
}
