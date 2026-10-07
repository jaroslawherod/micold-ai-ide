//! The Desktop notifications kind rows and the threshold field, by geometry (feature 613, T075).
//!
//! `settings_sections` reads the view's source for their order, and `layout_snapshot` records
//! where they are; the record was regenerated with the change that added them, so it compares
//! against what it was shown. This states the expected arrangement instead (S1, S5, FR-009):
//!
//! - one run of five rows in the Environment page is indented, each by one spacing step
//!   (`spacing::MD`): the four kind rows and the threshold field;
//! - the run follows the master switch's row directly, and its rows stack top to bottom without
//!   overlapping, inside the page's width;
//! - the four kind rows share one height, and the fourth row of the run, the one directly after
//!   **Long task finished** (third in `NotificationKind::ALL`), is the threshold field: the only row
//!   of another height (a labelled field with supporting text).
//!
//! **Compiled into the `layout_snapshot` binary** for the reason `containment` gives.

use crate::support::covered_states::covered_states;
use crate::support::layout::{self as lay, LayoutRecord};
use iced::advanced::widget::Tree;
use iced::advanced::{clipboard, layout, mouse, Layout, Shell};
use iced::{keyboard, window, Event, Point, Rectangle, Size};
use micold_client::app::Message;
use micold_client::features::settings::Msg as SettingsMsg;
use micold_core::attention::NotificationKind;
use micold_core::theme::ColorScheme;
use micold_core::tokens::spacing;

const RECORDED_SCHEME: ColorScheme = ColorScheme::Light;

/// Half a pixel, matching the other gates.
const TOLERANCE: f32 = 0.5;

/// The covered state that shows the Environment page with the master switch on.
const ENVIRONMENT: &str = "settings-view-environment";

/// The direct children of the record at `parent`, in order.
fn children<'a>(records: &'a [LayoutRecord], parent: &[usize]) -> Vec<&'a LayoutRecord> {
    let mut found: Vec<&LayoutRecord> = records
        .iter()
        .filter(|r| {
            r.layer == lay::Layer::Base
                && r.path.len() == parent.len() + 1
                && r.path.starts_with(parent)
        })
        .collect();
    found.sort_by_key(|r| r.path[parent.len()]);
    found
}

/// A row is indented when its first child starts one spacing step inside it and spans the rest.
fn indented(records: &[LayoutRecord], row: &LayoutRecord) -> bool {
    children(records, &row.path).first().is_some_and(|inner| {
        (inner.x - (row.x + spacing::MD)).abs() <= TOLERANCE
            && (inner.width - (row.width - spacing::MD)).abs() <= TOLERANCE
    })
}

/// The one run of indented rows in the Environment page: its column, the index of its first row
/// among the column's children, and its rows.
fn indented_run(records: &[LayoutRecord]) -> (Vec<usize>, usize, Vec<&LayoutRecord>) {
    // Every run of consecutive indented siblings, under any parent.
    let mut runs: Vec<(Vec<usize>, usize, Vec<&LayoutRecord>)> = Vec::new();
    for parent in records.iter().filter(|r| r.layer == lay::Layer::Base) {
        let rows = children(records, &parent.path);
        let mut i = 0;
        while i < rows.len() {
            if indented(records, rows[i]) {
                let start = i;
                while i < rows.len() && indented(records, rows[i]) {
                    i += 1;
                }
                runs.push((parent.path.clone(), start, rows[start..i].to_vec()));
            } else {
                i += 1;
            }
        }
    }
    assert_eq!(
        runs.len(),
        1,
        "one run of indented rows in the Environment page, found {}: {:?}",
        runs.len(),
        runs.iter()
            .map(|(p, s, r)| (p, s, r.len()))
            .collect::<Vec<_>>()
    );
    runs.remove(0)
}

#[test]
fn the_kind_rows_and_the_threshold_sit_indented_under_the_master_switch() {
    let renderer = lay::renderer();
    let all = lay::cached_records(covered_states(), &renderer, RECORDED_SCHEME);
    let (_, records) = covered_states()
        .iter()
        .zip(all.iter())
        .find(|(c, _)| c.name == ENVIRONMENT)
        .unwrap_or_else(|| panic!("no covered state named {ENVIRONMENT}"));

    let (column, start, run) = &indented_run(records);
    assert_eq!(run.len(), 5, "four kind rows and the threshold field");
    assert!(*start > 0, "the run follows a row");

    let rows = children(records, column);
    let master = rows[start - 1];
    let parent = records
        .iter()
        .find(|r| r.layer == lay::Layer::Base && &r.path == column)
        .expect("the column's own record");

    // Directly under the master switch, stacked in order, none overlapping, inside the page.
    let mut above = master;
    for row in run {
        assert!(
            row.y >= above.y + above.height - TOLERANCE,
            "row {:?} at y {} overlaps or precedes the row above, which ends at {}",
            row.path,
            row.y,
            above.y + above.height
        );
        assert!(
            (row.x - master.x).abs() <= TOLERANCE,
            "row {:?} starts where the master switch's row does; its content carries the indent",
            row.path
        );
        assert!(
            row.x + row.width <= parent.x + parent.width + TOLERANCE,
            "row {:?} stays inside the page's width",
            row.path
        );
        above = row;
    }

    // The fourth row is the threshold field: the only row whose height differs from the kinds'.
    let kind_height = run[0].height;
    for (index, row) in run.iter().enumerate() {
        if index == 3 {
            assert!(
                (row.height - kind_height).abs() > TOLERANCE,
                "the row after Long task finished is the threshold field, taller than a kind row \
                 ({} against {kind_height})",
                row.height
            );
        } else {
            assert!(
                (row.height - kind_height).abs() <= TOLERANCE,
                "kind row {index} is {} tall, the others {kind_height}",
                row.height
            );
        }
    }
}

/// The Environment page's state with the master switch set to `on`.
fn environment(on: bool) -> lay::StateUnderTest {
    let covered = covered_states()
        .iter()
        .find(|c| c.name == ENVIRONMENT)
        .unwrap_or_else(|| panic!("no covered state named {ENVIRONMENT}"));
    let mut under = (covered.build)();
    under
        .state
        .settings
        .settings_draft
        .as_mut()
        .expect("the covered state opens Settings")
        .environment
        .desktop_notifications = on;
    under
}

/// Hand `events` to the page, one after another with the cursor at `at`, and return what it
/// published.
fn publish(under: &lay::StateUnderTest, at: Point, events: &[Event]) -> Vec<Message> {
    let renderer = lay::renderer();
    let mut element = lay::view_of(under);
    let mut tree = Tree::new(element.as_widget());
    let node = element.as_widget_mut().layout(
        &mut tree,
        &renderer,
        &layout::Limits::new(Size::ZERO, lay::WINDOW),
    );
    let viewport = Rectangle::with_size(lay::WINDOW);
    let mut messages = Vec::new();
    // The page mounts with an entrance and takes no input until it has run.
    let origin = std::time::Instant::now();
    let frames: Vec<Event> = (0..lay::SETTLE_FRAMES * 2)
        .map(|f| Event::Window(window::Event::RedrawRequested(origin + lay::FRAME * f)))
        .collect();
    for event in frames.iter().chain(events) {
        let mut shell = Shell::new(&mut messages);
        element.as_widget_mut().update(
            &mut tree,
            event,
            Layout::new(&node),
            mouse::Cursor::Available(at),
            &renderer,
            &mut clipboard::Null,
            &mut shell,
            &viewport,
        );
    }
    messages
}

/// The first leaf under `row`, following first children: a kind row's box.
fn first_leaf<'a>(records: &'a [LayoutRecord], row: &'a LayoutRecord) -> &'a LayoutRecord {
    let mut at = row;
    while let Some(child) = children(records, &at.path).first() {
        at = child;
    }
    at
}

fn centre(row: &LayoutRecord) -> Point {
    Point::new(row.x + row.width / 2.0, row.y + row.height / 2.0)
}

const CLICK: [Event; 2] = [
    Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
    Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)),
];

/// S2, US2.5, FR-012: a click on a kind row toggles that kind, and a click in the threshold field
/// followed by a digit edits it, only while Desktop notifications is on. Off, the rows still show
/// their stored values (they are laid out as with it on) but take no input.
#[test]
fn the_kind_rows_and_the_threshold_take_input_only_while_the_master_switch_is_on() {
    let renderer = lay::renderer();
    for on in [true, false] {
        let under = environment(on);
        let records = lay::resolve(lay::view_of(&under), &renderer);
        let (_, _, run) = indented_run(&records);
        assert_eq!(
            run.len(),
            5,
            "four kind rows and the threshold field (on: {on})"
        );

        let kind_rows = run.iter().enumerate().filter(|(i, _)| *i != 3);
        for (row, kind) in kind_rows.zip(NotificationKind::ALL) {
            let published = publish(&under, centre(first_leaf(&records, row.1)), &CLICK);
            let toggled: Vec<_> = published
                .iter()
                .filter_map(|m| match m {
                    Message::Settings(SettingsMsg::NotificationKindToggled(k, v)) => Some((*k, *v)),
                    _ => None,
                })
                .collect();
            let stored = under
                .state
                .settings
                .settings_draft
                .as_ref()
                .expect("Settings is open")
                .environment
                .notification_kinds
                .is_on(kind);
            if on {
                assert_eq!(toggled, vec![(kind, !stored)], "a click toggles {kind:?}");
            } else {
                assert!(
                    published.is_empty(),
                    "with the master switch off a click on {kind:?} publishes nothing, got {published:?}"
                );
            }
        }

        let mut typed = CLICK.to_vec();
        typed.push(Event::Keyboard(keyboard::Event::KeyPressed {
            key: keyboard::Key::Character("5".into()),
            modified_key: keyboard::Key::Character("5".into()),
            physical_key: keyboard::key::Physical::Code(keyboard::key::Code::Digit5),
            location: keyboard::Location::Standard,
            modifiers: keyboard::Modifiers::default(),
            text: Some("5".into()),
            repeat: false,
        }));
        let edits: Vec<_> = publish(&under, centre(run[3]), &typed)
            .into_iter()
            .filter(|m| {
                matches!(
                    m,
                    Message::Settings(SettingsMsg::LongTaskThresholdChanged(_))
                )
            })
            .collect();
        assert_eq!(
            edits.len(),
            usize::from(on),
            "typing into the threshold field edits it only with the master switch on (on: {on})"
        );
    }
}
