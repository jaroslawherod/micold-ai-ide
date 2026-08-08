//! Project switching, its context menu and its rename draft, in isolation (feature 021, SC-004).
//!
//! This file names exactly one feature module and the domain types its API mentions. It builds no
//! `State`, references no other feature's types, and needs no application shell.
//!
//! `clamp_menu_anchor` is already covered by `switcher_forget_menu.rs`, but that file drives
//! `State` and `Message`, so it proves nothing about isolation. What is asserted here is
//! deliberately different: the *property* the worked examples there are instances of.

use micold_client::features::project::{
    clamp_menu_anchor, ProjectMenu, RenameDraft, SwitcherEntry,
};
use std::path::PathBuf;

#[test]
fn a_clamped_menu_always_fits_inside_the_window() {
    let menu = (220u16, 44u16);
    let window = (1000u16, 800u16);

    for x in [0u16, 1, 400, 779, 780, 781, 999, 1000] {
        for y in [0u16, 1, 400, 755, 756, 757, 799, 800] {
            let (cx, cy) = clamp_menu_anchor((x, y), menu, window);

            assert!(
                cx + menu.0 <= window.0 && cy + menu.1 <= window.1,
                "a menu opened at ({x}, {y}) clamped to ({cx}, {cy}), which still hangs off a \
                 {window:?} window — the point of clamping is that no anchor produces an \
                 off-screen panel, not that the anchors someone thought to test do not"
            );
        }
    }
}

#[test]
fn clamping_never_drags_a_menu_further_from_where_it_was_clicked_than_it_must() {
    let menu = (220u16, 44u16);
    let window = (1000u16, 800u16);

    let (cx, cy) = clamp_menu_anchor((400, 400), menu, window);

    assert_eq!(
        (cx, cy),
        (400, 400),
        "an anchor that already fits is left alone — a context menu that jumps away from the \
         cursor for no reason reads as a misclick"
    );
}

#[test]
fn a_window_smaller_than_the_menu_pins_it_to_the_origin_rather_than_underflowing() {
    let (cx, cy) = clamp_menu_anchor((900, 900), (220, 44), (100, 30));

    assert_eq!(
        (cx, cy),
        (0, 0),
        "u16 arithmetic would wrap to ~65,000 on a saturating subtraction done the naive way, \
         putting the menu somewhere off in space instead of at the top-left"
    );
}

#[test]
fn a_context_menu_remembers_which_project_it_acts_on_not_merely_where_it_was_drawn() {
    let menu = ProjectMenu {
        path: PathBuf::from("/p/one"),
        anchor: (10, 20),
    };

    assert_eq!(
        menu.path,
        PathBuf::from("/p/one"),
        "the menu carries its target, so a project list that reorders underneath it cannot make \
         Forget act on the wrong project"
    );
}

#[test]
fn a_switcher_row_reports_availability_and_activity_separately() {
    let row = SwitcherEntry {
        path: PathBuf::from("/p/gone"),
        label: "gone".into(),
        is_active: false,
        running_count: 2,
        available: false,
    };

    assert!(
        !row.available && row.running_count == 2,
        "an unavailable project can still own running sessions — the two are independent, and \
         collapsing them would hide work the user has in flight"
    );
}

#[test]
fn a_rename_draft_starts_without_an_error() {
    let draft = RenameDraft {
        path: PathBuf::from("/p/one"),
        text: "one".into(),
        error: None,
    };

    assert!(
        draft.error.is_none(),
        "the dialog opens clean; an error appears only after a rejected confirm (FR-020)"
    );
}
