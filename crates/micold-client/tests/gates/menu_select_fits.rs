//! `MenuSelect` holds its parts and answers the right gestures (feature 1272, T016; contract §3,
//! FR-001, FR-011, Edge "Many pull requests", "Long PR title").
//!
//! The showcase's poses of the review's source control, built from `source_menu::rows` and
//! `label`, in both schemes:
//!
//! - the open list floats below the trigger, and the body under the control does not move;
//! - a long pull-request title ends in `…` while the fixed `#n · ` stays whole;
//! - past its height cap the list stops growing and scrolls inside itself;
//! - the Working tree option keeps its two chips and the Default entry's note inside its row;
//! - a heading or a note line takes no press;
//! - a press on the trigger, `Enter` and `Space` emit the toggle; a press outside the open list,
//!   `Esc` and `Tab` emit the dismissal; a press on a chip emits the chip's message and nothing
//!   else.
//!
//! Gestures go through the runtime's own `UserInterface` ([`lay::runtime_messages`]), so the open
//! list sees each event before the trigger under it, as in the application.
//!
//! Compiled into the `layout_snapshot` binary beside `command_palette_fits`.

use crate::support::layout::{self as lay, Gesture, Layer, LayoutRecord, Overflow};
use iced::keyboard::key::Named;
use iced::{Point, Size};
use micold_client::showcase::sections::review::{
    menu_select_sample, MenuEvent, MenuPose, MENU_BODY, MENU_LIST_HEIGHT, MENU_LONG_NUMBER,
    MENU_LONG_TITLE, MENU_MANY, MENU_SAMPLE_WIDTH,
};
use micold_core::review::source_menu::{
    Kind, Stop, DEFAULT_ENTRY_NOTE, FILES, PRS_LOADING, PULL_REQUESTS, WORKING_TREE,
};
use micold_core::theme::ColorScheme;
use micold_core::tokens::{self, spacing, Roles};

/// Half a pixel, matching every geometry gate beside this one.
const TOLERANCE: f32 = 0.5;

const WINDOW: Size = lay::WINDOW;

fn schemes() -> [Roles; 2] {
    [ColorScheme::Light, ColorScheme::Dark].map(tokens::roles)
}

fn sample(pose: MenuPose, roles: Roles) -> iced::Element<'static, MenuEvent> {
    menu_select_sample(pose, roles, |event| event)
}

/// What the pose painted, and both layers' boxes, entrances finished.
fn painted(pose: MenuPose, roles: Roles) -> (Vec<Overflow>, Vec<LayoutRecord>) {
    let mut renderer = lay::renderer();
    lay::painted_text_settled_at(sample(pose, roles), &mut renderer, WINDOW)
}

/// The first text painted in `layer` that is exactly `content`.
fn text<'p>(painted: &'p [Overflow], layer: Layer, content: &str) -> &'p Overflow {
    painted
        .iter()
        .find(|p| p.layer == layer && p.content == content)
        .unwrap_or_else(|| {
            let all: Vec<_> = painted.iter().map(|p| (p.layer, &p.content)).collect();
            panic!("`{content}` is painted in {layer:?}: {all:?}")
        })
}

/// The trigger: the sample's first child, a column's first row.
fn trigger(records: &[LayoutRecord]) -> &LayoutRecord {
    records
        .iter()
        .find(|r| r.layer == Layer::Base && r.path == [0])
        .unwrap_or_else(|| panic!("no trigger at 0 in {records:?}"))
}

/// The floated list's panel: the shallowest overlay node smaller than the window (the floated
/// layers group, and a group is the window's size).
fn panel(records: &[LayoutRecord]) -> &LayoutRecord {
    records
        .iter()
        .filter(|r| r.layer == Layer::Overlay && r.height < WINDOW.height - TOLERANCE)
        .min_by_key(|r| r.path.len())
        .unwrap_or_else(|| panic!("the open pose floats a list: {records:?}"))
}

fn centre(p: &Overflow) -> Point {
    Point::new(
        p.origin.x + p.natural_width.min(p.allowed_width) / 2.0,
        p.origin.y + p.natural_height / 2.0,
    )
}

fn inside_horizontally(p: &Overflow, area: &LayoutRecord) -> bool {
    let right = p.origin.x + p.natural_width.min(p.allowed_width);
    p.origin.x >= area.x - TOLERANCE && right <= area.x + area.width + TOLERANCE
}

#[test]
fn the_open_list_floats_below_the_trigger_and_the_body_keeps_its_place() {
    for roles in schemes() {
        let (closed, _) = painted(MenuPose::Closed, roles);
        let (open, records) = painted(MenuPose::List, roles);
        let body_closed = text(&closed, Layer::Base, MENU_BODY);
        let body_open = text(&open, Layer::Base, MENU_BODY);
        assert!(
            (body_closed.origin.y - body_open.origin.y).abs() <= TOLERANCE
                && (body_closed.origin.x - body_open.origin.x).abs() <= TOLERANCE,
            "the body stays put: {body_closed:?} vs {body_open:?}"
        );
        let trigger = trigger(&records);
        let list = panel(&records);
        assert!(
            list.y >= trigger.y + trigger.height - TOLERANCE,
            "the list starts below the trigger: {list:?} under {trigger:?}"
        );
        assert!(
            (list.x - trigger.x).abs() <= TOLERANCE,
            "the list is aligned with the trigger: {list:?} under {trigger:?}"
        );
        assert!(
            list.y < body_open.origin.y + body_open.natural_height,
            "the list lies over the body rather than pushing it: {list:?}, {body_open:?}"
        );
    }
}

#[test]
fn a_long_title_ends_in_an_ellipsis_and_the_number_stays_whole() {
    let fixed = format!("#{MENU_LONG_NUMBER} · ");
    for roles in schemes() {
        let (painted, records) = painted(MenuPose::ClosedLongTitle, roles);
        let trigger = trigger(&records);
        assert!(
            trigger.width <= MENU_SAMPLE_WIDTH + TOLERANCE,
            "the trigger keeps its width: {trigger:?}"
        );
        let number = text(&painted, Layer::Base, &fixed);
        assert!(
            number.natural_width <= number.allowed_width + TOLERANCE,
            "the fixed part is drawn whole: {number:?}"
        );
        let title = painted
            .iter()
            .find(|p| p.layer == Layer::Base && p.content.ends_with('…'))
            .unwrap_or_else(|| panic!("the title is cut with an ellipsis: {painted:?}"));
        let kept = title.content.trim_end_matches('…');
        assert!(
            !kept.is_empty() && MENU_LONG_TITLE.starts_with(kept) && kept != MENU_LONG_TITLE,
            "the title keeps its start: {:?}",
            title.content
        );
        for p in painted.iter().filter(|p| p.layer == Layer::Base) {
            if p.content == MENU_BODY {
                continue;
            }
            assert!(
                inside_horizontally(p, trigger),
                "`{}` stays inside the trigger {trigger:?}: {p:?}",
                p.content
            );
        }
    }
}

#[test]
fn past_its_cap_the_list_stops_growing_and_scrolls_inside_itself() {
    for roles in schemes() {
        let (painted, records) = painted(MenuPose::Many, roles);
        let list = panel(&records);
        // The panel's padding on either side of the capped viewport, and its outline.
        let chrome = 2.0 * spacing::XS + 2.0;
        assert!(
            list.height <= MENU_LIST_HEIGHT + chrome + TOLERANCE,
            "{MENU_MANY} pull requests are capped at {MENU_LIST_HEIGHT}: {list:?}"
        );
        let first = format!("#{}", 100);
        let last = format!("#{}", 100 - MENU_MANY as u64 + 1);
        let shown = |n: &str| {
            painted
                .iter()
                .any(|p| p.layer == Layer::Overlay && p.content == n)
        };
        assert!(shown(&first), "the first pull request is in view");
        assert!(
            !shown(&last),
            "the last of {MENU_MANY} is past the cap, inside the list's own scroll"
        );
    }
}

#[test]
fn the_working_tree_option_holds_its_chips_and_note_inside_its_row() {
    for roles in schemes() {
        let (painted, records) = painted(MenuPose::DefaultEntry, roles);
        let list = panel(&records);
        let option = text(&painted, Layer::Overlay, WORKING_TREE);
        // The Default entry poses no pull requests, so the next row is Files.
        let next = text(&painted, Layer::Overlay, FILES);
        let note = painted
            .iter()
            .find(|p| p.layer == Layer::Overlay && DEFAULT_ENTRY_NOTE.starts_with(&p.content))
            .unwrap_or_else(|| panic!("the Default entry's note is painted: {painted:?}"));
        for chip in ["Committed", "Uncommitted"] {
            let chip = text(&painted, Layer::Overlay, chip);
            assert!(inside_horizontally(chip, list), "{chip:?} inside {list:?}");
            assert!(
                chip.origin.y + chip.natural_height <= note.origin.y + TOLERANCE,
                "the chip sits above the note: {chip:?}, {note:?}"
            );
            assert!(
                chip.origin.y + TOLERANCE >= option.origin.y - chip.natural_height,
                "the chip is on the option's line: {chip:?}, {option:?}"
            );
        }
        assert!(inside_horizontally(note, list), "{note:?} inside {list:?}");
        assert!(
            note.origin.y > option.origin.y,
            "the note is under the option: {note:?}"
        );
        assert!(
            note.origin.y + note.natural_height <= next.origin.y + TOLERANCE,
            "the note ends before the next row: {note:?}, {next:?}"
        );
    }
}

/// What each gesture published, against a pose.
fn gestures(pose: MenuPose, roles: Roles, gestures: &[Gesture]) -> Vec<Vec<MenuEvent>> {
    lay::runtime_messages(|| sample(pose, roles), WINDOW, gestures)
}

#[test]
fn a_heading_or_a_note_line_takes_no_press() {
    for roles in schemes() {
        let (list, _) = painted(MenuPose::List, roles);
        let heading = centre(text(&list, Layer::Overlay, PULL_REQUESTS));
        let (loading, _) = painted(MenuPose::Loading, roles);
        let note = centre(text(&loading, Layer::Overlay, PRS_LOADING));
        assert_eq!(
            gestures(MenuPose::List, roles, &[Gesture::Click(heading)]),
            vec![Vec::<MenuEvent>::new()],
            "the heading is not pickable"
        );
        assert_eq!(
            gestures(MenuPose::Loading, roles, &[Gesture::Click(note)]),
            vec![Vec::<MenuEvent>::new()],
            "the note is not pickable"
        );
    }
}

#[test]
fn the_trigger_toggles_by_press_enter_and_space() {
    for roles in schemes() {
        let (painted, _) = painted(MenuPose::Closed, roles);
        let at = centre(text(&painted, Layer::Base, WORKING_TREE));
        assert_eq!(
            gestures(
                MenuPose::Closed,
                roles,
                &[
                    Gesture::Click(at),
                    Gesture::Key(Named::Enter),
                    Gesture::Key(Named::Space),
                ],
            ),
            vec![
                vec![MenuEvent::Toggled],
                vec![MenuEvent::Toggled],
                vec![MenuEvent::Toggled],
            ]
        );
    }
}

#[test]
fn outside_esc_and_tab_dismiss_the_open_list() {
    for roles in schemes() {
        let outside = Point::new(WINDOW.width - 8.0, WINDOW.height - 8.0);
        for gesture in [
            Gesture::Click(outside),
            Gesture::Key(Named::Escape),
            Gesture::Key(Named::Tab),
        ] {
            assert_eq!(
                gestures(MenuPose::List, roles, &[gesture]),
                vec![vec![MenuEvent::Dismissed]],
                "{gesture:?} dismisses"
            );
        }
    }
}

#[test]
fn a_chip_press_emits_only_the_chips_message_and_a_row_press_its_pick() {
    for roles in schemes() {
        let (painted, _) = painted(MenuPose::List, roles);
        let chip = centre(text(&painted, Layer::Overlay, "Uncommitted"));
        let option = centre(text(&painted, Layer::Overlay, "Ship the parser"));
        assert_eq!(
            gestures(MenuPose::List, roles, &[Gesture::Click(chip)]),
            vec![vec![MenuEvent::Took(Stop::Kind(Kind::Uncommitted))]],
            "a chip sends its own message and leaves openness to the caller"
        );
        assert_eq!(
            gestures(MenuPose::List, roles, &[Gesture::Click(option)]),
            vec![vec![MenuEvent::Took(Stop::PullRequest(12))]],
            "a row sends its pick"
        );
        assert_eq!(
            gestures(MenuPose::List, roles, &[Gesture::Key(Named::Enter)]),
            vec![vec![MenuEvent::Picked]],
            "Enter takes the highlighted stop"
        );
    }
}
