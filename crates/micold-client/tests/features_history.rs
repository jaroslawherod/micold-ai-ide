//! Back/Forward in isolation (feature 1465, SC-004 of feature 021): its own state and messages
//! only. What a press does in a populated project is `tests/history.rs`.

use micold_client::app::State;
use micold_client::features::history::{self, Msg, Source, Step};
use micold_core::history::{Dir, Place, View};

fn overview(project: &str) -> Place {
    Place {
        project: project.into(),
        view: View::Overview,
    }
}

#[test]
fn observing_records_a_shown_place_once_and_nothing_while_moving() {
    let mut state = history::State::default();
    history::observe(&mut state, None);
    assert!(state.list.entries().is_empty(), "no place shown");
    history::observe(&mut state, Some(overview("/p")));
    history::observe(&mut state, Some(overview("/p")));
    assert_eq!(state.list.entries(), &[overview("/p")]);
    state.moving = true;
    history::observe(&mut state, Some(overview("/q")));
    assert_eq!(state.list.entries().len(), 1, "off while a move runs");
    history::finish(&mut state, Some(overview("/q")));
    assert!(!state.moving);
    assert_eq!(state.list.entries().len(), 2, "finishing observes again");
}

#[test]
fn with_no_project_open_a_press_changes_nothing() {
    let mut state = State::default();
    for dir in [Dir::Back, Dir::Forward] {
        for source in [Source::Button, Source::Key] {
            assert_eq!(
                history::resolve(&mut state, Msg::Pressed { dir, source }),
                Step::None
            );
        }
    }
    assert!(!state.history.moving);
}
