//! The known-projects list stays usable in a narrow window (feature 003, BUG-002 — FR-016, FR-012,
//! and the spec's small-window edge case).
//!
//! Each row was a plain `row![]` whose only flexible child was the project name, beside three
//! buttons laid out at their intrinsic width. As the row narrowed the name was the only thing that
//! could give, so it wrapped into a clipped second line and then vanished; past that the buttons
//! lost their labels, then their icons, and finally overflowed the card. The layout gates resolve
//! the default 1280×800 window only, so nothing exercised a narrow one.
//!
//! This paints the shell at narrow widths and reads what was drawn, because every failure the
//! walkthrough recorded is a *paint* failure: the name's node is always the width its parent allots,
//! defect or not, and what goes wrong is the paragraph drawn inside it.
//!
//! Held at two widths. The minimum window (640dp) with no project open is where the report's ladder
//! ends; the narrower one stands for the main area beside a sidebar at its minimum width, which is
//! the narrowest the list is ever given while the window is at its own minimum.

mod support;

use iced::widget::container;
use iced::{Element, Length};
use micold_client::app::{Message, State};
use micold_client::features::connection::ConnectionStatus;
use micold_client::features::sandbox::Sandbox;
use micold_core::env_include::EnvIncludeOutcome;
use micold_core::project::Availability;
use support::layout as lay;

/// A name far longer than any row has room for beside its actions.
const LONG: &str = "a-project-whose-folder-name-is-much-longer-than-the-row-can-show";
/// An ordinary name, which must still be shown whole whenever it fits.
const SHORT: &str = "notes";
/// The unavailable entry's name — its reopen button reads "Unavailable", the widest of the three.
const GONE: &str = "gone-away-and-renamed-elsewhere-on-disk";

/// The width each case gives the whole view, and what it stands for.
const WIDTHS: &[(f32, &str)] = &[
    (640.0, "the minimum window, no project open"),
    (440.0, "the main area beside a minimum-width sidebar"),
];

/// A name is readable when at least this much of it is shown. Not a design figure — a floor well
/// under any sensible one, so the test asserts "the name is there" rather than pinning a number.
const READABLE: f32 = 80.0;

fn state() -> State {
    let paths: Vec<String> = [LONG, SHORT, GONE]
        .iter()
        .map(|name| format!("/fixture/{name}"))
        .collect();
    let mut workspace =
        support::workspace_with(paths.iter().map(|p| (p.as_str(), vec![])).collect());
    workspace
        .projects
        .iter_mut()
        .find(|p| p.display_name == GONE)
        .expect("the fixture opened the unavailable project")
        .availability = Availability::Unavailable;
    State {
        workspace,
        ..State::default()
    }
}

fn painted_at(state: &State, width: f32) -> Vec<lay::Overflow> {
    let view: Element<'_, Message> = micold_client::ui::view(
        state,
        None,
        None,
        0,
        None,
        &EnvIncludeOutcome::Disabled,
        &ConnectionStatus::Connected,
        &Sandbox::default(),
    );
    let narrowed: Element<'_, Message> = container(view)
        .width(Length::Fixed(width))
        .height(Length::Fill)
        .into();
    let mut renderer = lay::renderer();
    lay::painted_text_settled(narrowed, &mut renderer)
}

/// Whether `drawn` is `name`, or a prefix of it ending in an ellipsis.
fn shows(drawn: &str, name: &str) -> bool {
    drawn == name
        || drawn
            .strip_suffix('…')
            .is_some_and(|prefix| !prefix.is_empty() && name.starts_with(prefix))
}

fn contents(painted: &[lay::Overflow]) -> Vec<&str> {
    painted.iter().map(|t| t.content.as_str()).collect()
}

#[test]
fn every_project_name_is_shown_whole_or_elided_never_dropped_or_clipped() {
    let state = state();
    for &(width, what) in WIDTHS {
        let painted = painted_at(&state, width);
        for name in [LONG, SHORT, GONE] {
            let drawn: Vec<_> = painted.iter().filter(|t| shows(&t.content, name)).collect();
            assert_eq!(
                drawn.len(),
                1,
                "at {width}dp ({what}) the row for {name:?} must paint its name exactly once, whole \
                 or ending in '…' (FR-012, FR-016); painted: {:?}",
                contents(&painted)
            );
            let t = drawn[0];
            assert!(
                t.natural_width <= t.allowed_width + 0.5,
                "at {width}dp ({what}) {name:?} is drawn as {:?}, {:.1}dp wide in {:.1}dp — a name \
                 cut off by its box instead of elided",
                t.content,
                t.natural_width,
                t.allowed_width
            );
            // Shown whole means shown whole on its line. A paragraph that wrapped reports the
            // width of its widest line, which fits by construction — so the one-line width is
            // measured here rather than read off what was drawn.
            if t.content == name {
                let role = micold_core::tokens::typography::BODY_MEDIUM;
                let one_line = lay::measure(name, lay::reference_font_at(role.weight), role.size);
                assert!(
                    one_line <= t.allowed_width + 1.0,
                    "at {width}dp ({what}) {name:?} needs {one_line:.1}dp on one line and was given \
                     {:.1}dp — it wrapped, and the row cuts the second line off",
                    t.allowed_width
                );
            }
            assert!(
                t.content == name || t.natural_width >= READABLE,
                "at {width}dp ({what}) {name:?} is reduced to {:?}, {:.1}dp — the row has given \
                 the name away to its actions",
                t.content,
                t.natural_width
            );
            assert!(
                t.origin.x + t.natural_width <= width + 0.5,
                "at {width}dp ({what}) {name:?} is drawn past the window's right edge"
            );
        }
        // The one that fits is not shortened merely because a sibling was.
        assert!(
            painted.iter().any(|t| t.content == SHORT),
            "at {width}dp ({what}) the short name {SHORT:?} fits and must be shown whole"
        );
    }
}

#[test]
fn every_action_keeps_its_label_and_stays_on_its_card() {
    let state = state();
    for &(width, what) in WIDTHS {
        let painted = painted_at(&state, width);
        // Two available rows and one unavailable; every row has Rename and Forget.
        for (label, rows) in [
            ("Open", 2),
            ("Unavailable", 1),
            ("Rename", 3),
            ("Forget", 3),
        ] {
            let drawn: Vec<_> = painted.iter().filter(|t| t.content == label).collect();
            assert_eq!(
                drawn.len(),
                rows,
                "at {width}dp ({what}) {label:?} must be painted once per row that has it — an \
                 action that loses its label is a control nobody can identify; painted: {:?}",
                contents(&painted)
            );
            for t in drawn {
                assert!(
                    t.natural_width <= t.allowed_width + 0.5 && t.allowed_width > 0.0,
                    "at {width}dp ({what}) {label:?} is {:.1}dp wide in {:.1}dp — a clipped label",
                    t.natural_width,
                    t.allowed_width
                );
                // The draw origin lies inside the widget however its label is aligned, so an origin
                // past the edge is a control at least half off its card.
                assert!(
                    t.origin.x <= width,
                    "at {width}dp ({what}) {label:?} is drawn past the window's right edge"
                );
            }
        }
    }
}
