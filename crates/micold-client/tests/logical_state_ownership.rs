//! No logical state moved into a component (feature 017, T044 — FR-012, FR-016, SC-006).
//!
//! Feature 017 moved *presentation* state out of the application and into the components that
//! render it: how far a dialog has faded, how lit a resize handle is, how far a drawer has slid.
//! The risk in a migration shaped like that is over-reach — a component that starts owning a
//! decision rather than an appearance, at which point the application can no longer reason about
//! its own behaviour and the state stops being persistable.
//!
//! The line is whether a value would still mean something with the screen switched off. A drawer's
//! slide progress would not; whether the sidebar is *hidden* would, and does — it is written to
//! disk and restored on the next run. So the second belongs to the application and the first does
//! not, and this file pins that split for every piece of state the task enumerates.
//!
//! The deviation recorded against T040 is the same judgement: the hovered-row *field* stayed in the
//! core, because it is what arms a row's delete button. A widget owning it privately would be a
//! widget deciding whether a destructive action is available.

use micold_client::app::{Message, State, SIDEBAR_MIN_WIDTH};
use micold_client::features::help::Msg as HelpMsg;
use micold_client::features::project::Msg as ProjectMsg;
use micold_client::features::settings::Msg as SettingsMsg;
use micold_client::features::sidebar::Msg as SidebarMsg;
use micold_client::features::sidebar::TagFilter;
use micold_client::features::worktree::Msg as WorktreeMsg;
use micold_core::naming::ConventionalType;
use micold_core::project::{Availability, Project};
use micold_core::theme::ThemePreference;
use std::path::PathBuf;

fn with_project() -> State {
    let mut state = State::default();
    let path = PathBuf::from("/repo");
    state.workspace.projects.push(Project {
        path: path.clone(),
        display_name: "repo".to_string(),
        is_git_repo: true,
        availability: Availability::Available,
    });
    state.workspace.active = Some(path);
    state
}

/// Sidebar visibility survives in application state, and is reachable without rendering anything.
///
/// This is the one most easily mistaken for presentation — it *is* what the drawer animates
/// toward. But the drawer owns only how far along the slide is; whether the sidebar should be open
/// is a preference that outlives the window.
#[test]
fn sidebar_visibility_is_application_owned() {
    let mut state = State::default();
    assert!(!state.sidebar.hidden);
    state.update(Message::Sidebar(SidebarMsg::Toggled));
    assert!(state.sidebar.hidden, "the flag must live on State");
}

/// Likewise the width. The handle reports where the pointer is; the application decides what width
/// that means, including the clamp.
#[test]
fn sidebar_width_is_application_owned_and_clamped_here() {
    let mut state = State::default();
    state.update(Message::Sidebar(SidebarMsg::DragMoved(10)));
    assert_eq!(
        state.sidebar_width_px(),
        SIDEBAR_MIN_WIDTH,
        "clamping is the application's decision, not the edge's"
    );
}

/// Which overlay is open decides what the Escape key does and what the scrim dismisses. A
/// component owning it would be a component deciding the application's modality.
#[test]
fn open_overlay_identity_is_application_owned() {
    // Since T037 the identity is not a slot but the state each dialog draws from, read back
    // through the registry — the same answer to the same question, asked of what now holds it.
    let mut state = State::default();
    assert!(micold_client::overlay::registry::open_dialog(&state).is_none());
    state.update(Message::Help(HelpMsg::AboutOpened));
    assert_eq!(
        micold_client::overlay::registry::open_dialog(&state).map(|open| open.id()),
        Some(micold_client::overlay::SurfaceId::new("about"))
    );
}

/// Menu identity is a *worktree and a point* — whose menu is open and where it was opened from —
/// not a boolean about a panel. The panel owns its fade; the application owns whose menu it is and
/// where the user asked for it (018 FR-029d).
#[test]
fn open_menu_identity_is_application_owned() {
    let mut state = State::default();
    state.update(Message::Worktree(WorktreeMsg::MenuToggled(
        "feat-a".to_string(),
        (120, 300),
    )));
    let open = state.worktree.menu_open.as_ref().expect("the menu is open");
    assert_eq!(open.dir_name, "feat-a");
    assert_eq!(open.anchor, (120, 300));
    state.update(Message::Worktree(WorktreeMsg::MenuToggled(
        "feat-a".to_string(),
        (120, 300),
    )));
    assert_eq!(state.worktree.menu_open, None);
}

/// Expanded tree nodes decide what the sidebar contains, not how it looks getting there.
#[test]
fn expanded_nodes_are_application_owned() {
    let mut state = with_project();
    state.update(Message::Sidebar(SidebarMsg::WorktreeExpansionToggled(
        "feat-a".to_string(),
    )));
    assert!(state.sidebar.expanded.contains("feat-a"));
    state.update(Message::Sidebar(SidebarMsg::WorktreeExpansionToggled(
        "feat-a".to_string(),
    )));
    assert!(!state.sidebar.expanded.contains("feat-a"));
}

/// A filter changes which rows exist. A component that owned it would be a component deciding what
/// the user is allowed to see.
#[test]
fn tag_filters_are_application_owned() {
    let mut state = State::default();
    let feature = TagFilter::Type(ConventionalType::Feat);
    state.update(Message::Sidebar(SidebarMsg::FilterToggled(feature)));
    assert!(state.sidebar.filters.contains(&feature));
    state.update(Message::Sidebar(SidebarMsg::FilterToggled(feature)));
    assert!(!state.sidebar.filters.contains(&feature));
}

/// The theme preference is written to disk and restored, so it could not live in a widget tree
/// even in principle — the tree is rebuilt from scratch every frame.
#[test]
fn theme_preference_is_application_owned() {
    let mut state = State::default();
    let before = state.settings.theme_pref;
    state.update(Message::Settings(SettingsMsg::ThemePreferenceChanged(
        before.next(),
    )));
    assert_ne!(state.settings.theme_pref, before);
    assert!(matches!(
        state.settings.theme_pref,
        ThemePreference::FollowSystem | ThemePreference::Light | ThemePreference::Dark
    ));
}

/// Drafts are user input. Losing one to a re-render would be data loss, which is the sharpest form
/// of the distinction this feature draws.
#[test]
fn drafts_are_application_owned() {
    let mut state = with_project();
    assert!(state.project.rename_draft.is_none());

    state.update(Message::Project(ProjectMsg::RenameStarted(PathBuf::from(
        "/repo",
    ))));
    state.update(Message::Project(ProjectMsg::RenameTextChanged(
        "renamed".to_string(),
    )));

    let draft = state
        .project
        .rename_draft
        .as_ref()
        .expect("an in-progress rename must survive on State, not in a rebuilt widget");
    assert_eq!(
        draft.text, "renamed",
        "the typed text is the part that would be lost to a re-render"
    );
}

/// The active session decides what the terminal is attached to.
#[test]
fn active_session_is_application_owned() {
    let state = with_project();
    assert!(state.session.active.is_none());
    // The field exists and is readable without a renderer, which is the property under test:
    // nothing about it requires a widget tree to interpret.
}

/// Worktrees are domain data, not presentation, and belong to the workspace either way.
#[test]
fn worktrees_are_application_owned() {
    let state = with_project();
    assert!(state.workspace.active_project().is_some());
}

/// The negative case, and the reason the others are worth asserting: the application holds *no*
/// animation state at all any more. If a progress value, a motion key or an animator reappeared on
/// `State`, presentation would have leaked back the other way.
///
/// Checked against the source rather than the type, because the failure is a field being *added* —
/// something no assertion about existing fields could notice.
#[test]
fn no_animation_state_remains_on_the_application() {
    let source = include_str!("../src/app.rs");
    for forbidden in [
        "MotionKey",
        "Animator",
        "sidebar_dragging",
        "AnimationTick",
        "progress: f32",
    ] {
        assert!(
            !source.contains(forbidden),
            "`{forbidden}` is back in app.rs — presentation state has leaked into the application"
        );
    }
}
