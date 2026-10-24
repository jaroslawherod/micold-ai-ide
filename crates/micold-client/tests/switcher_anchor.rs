//! The project switcher's panel hangs from the trigger that opened it (BUG-1023): it used to sit
//! at the window's trailing edge, under the overflow button, whichever control was pressed.

mod support;

use iced::Element;
use micold_client::app::{Message, State};
use micold_client::features::project::{self, Msg};
use micold_client::features::sandbox::Sandbox;
use micold_core::env_include::EnvIncludeOutcome;
use support::layout as lay;

const PANEL_WIDTH: f32 = 240.0;

fn view(state: &State) -> Element<'_, Message> {
    micold_client::ui::view(
        state,
        None,
        None,
        0,
        None,
        &EnvIncludeOutcome::Disabled,
        &micold_client::features::connection::ConnectionStatus::Connected,
        &Sandbox::default(),
    )
}

fn open_state(anchor: Option<(u16, u16)>) -> State {
    State {
        workspace: support::workspace_with(vec![("/fixture/a", vec![]), ("/fixture/b", vec![])]),
        project: project::State {
            switcher_open: true,
            switcher_anchor: anchor,
            ..Default::default()
        },
        ..State::default()
    }
}

fn panel_at(state: &State, x: f32, y: f32) -> bool {
    lay::resolve(view(state), &lay::renderer()).iter().any(|r| {
        (r.width - PANEL_WIDTH).abs() < 0.5 && (r.x - x).abs() < 0.5 && (r.y - y).abs() < 0.5
    })
}

/// Feature 1127 (R8, FR-005): the panel's leading edge is the trigger's reported `x`, and its top
/// edge is the bar's bottom edge whatever `y` the trigger reported — the trigger is a 40dp button
/// inside the 43dp bar, so its own bottom is not the bar's.
#[test]
fn the_panel_opens_where_the_trigger_reported() {
    let edge = micold_core::tokens::anatomy::app_bar::BOTTOM_EDGE;
    for reported_y in [42, 64] {
        let state = open_state(Some((300, reported_y)));
        assert!(
            panel_at(&state, 300.0, edge),
            "panel not at x 300, y {edge} for a reported y of {reported_y}"
        );
    }
}

#[test]
fn a_reported_corner_is_kept_by_the_reducer() {
    let mut state = open_state(None);
    let _ = project::update(&mut state, Msg::SwitcherAnchored((300, 64)));
    assert_eq!(state.project.switcher_anchor, Some((300, 64)));
}
