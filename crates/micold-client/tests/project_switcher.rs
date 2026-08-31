//! US2/US3 (feature 008): the top-bar project switcher — toggle + mutual exclusion with the
//! overflow menu, and the pure switcher-row data (active marker, running count, unavailable).
//! Rendering (trigger placement, panel) is build-verified + validated by quickstart.md.

mod support;

use micold_client::app::{Message, State};
use micold_client::features::help::Msg as HelpMsg;
use micold_client::features::project::Msg as ProjectMsg;
use micold_core::project::Availability;
use std::path::{Path, PathBuf};
use support::{idle_session, running_session, workspace_with};

// --- US2: toggle + mutual exclusion (FR-004) ---

#[test]
fn toggling_switcher_opens_and_closes_it() {
    let mut st = State::default();
    assert!(!st.project.switcher_open);
    st.update(Message::Project(ProjectMsg::SwitcherToggled));
    assert!(st.project.switcher_open);
    st.update(Message::Project(ProjectMsg::SwitcherToggled));
    assert!(!st.project.switcher_open);
}

#[test]
fn opening_switcher_closes_the_overflow_menu() {
    let mut st = State::default();
    st.update(Message::Help(HelpMsg::MenuToggled)); // menu open
    assert!(st.help.help_menu_open);

    st.update(Message::Project(ProjectMsg::SwitcherToggled));
    assert!(st.project.switcher_open);
    assert!(
        !st.help.help_menu_open,
        "opening the switcher closes the overflow menu"
    );
}

#[test]
fn opening_the_overflow_menu_closes_the_switcher() {
    let mut st = State::default();
    st.update(Message::Project(ProjectMsg::SwitcherToggled)); // switcher open
    assert!(st.project.switcher_open);

    st.update(Message::Help(HelpMsg::MenuToggled));
    assert!(st.help.help_menu_open);
    assert!(
        !st.project.switcher_open,
        "opening the overflow menu closes the switcher"
    );
}

// --- US2/US3: switcher row data (FR-006, FR-007, FR-008) ---

#[test]
fn switcher_entries_reflect_active_running_and_availability() {
    let mut st = State {
        workspace: workspace_with(vec![
            ("/a", vec![running_session("w1"), running_session("w2")]),
            ("/b", vec![idle_session("w3")]),
            ("/c", vec![]),
        ]),
        ..Default::default()
    };
    st.workspace.active = Some(PathBuf::from("/b"));
    // Mark /c unavailable.
    for p in &mut st.workspace.projects {
        if p.path.as_path() == Path::new("/c") {
            p.availability = Availability::Unavailable;
        }
    }

    let entries = st.switcher_entries();
    assert_eq!(entries.len(), 3, "one row per known project");

    let a = entries.iter().find(|e| e.path == Path::new("/a")).unwrap();
    let b = entries.iter().find(|e| e.path == Path::new("/b")).unwrap();
    let c = entries.iter().find(|e| e.path == Path::new("/c")).unwrap();

    // FR-006 active marker.
    assert!(!a.is_active && b.is_active && !c.is_active);
    // FR-007 running-background count (two on /a, none active on /b, none on /c).
    assert_eq!(a.running_count, 2);
    assert_eq!(b.running_count, 0);
    assert_eq!(c.running_count, 0);
    // FR-008 availability.
    assert!(a.available && b.available && !c.available);
}
