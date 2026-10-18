//! The Kubernetes add form fits one window height (feature 890, T009, SC-002).
//!
//! Before the sections the Kubernetes form listed every field at once and needed several window
//! heights. With default section states, no contexts found and no error shown, its section list
//! and actions must end — the bottom of *Cancel* — within 800 logical pixels at the canonical
//! Settings width. Asks the renderer where it drew the text.

use crate::support::layout as lay;
use iced::Size;
use micold_client::app::State;
use micold_client::features::connection::ConnectionStatus;
use micold_client::features::daemons::{DraftRuntime, Msg as DaemonsMsg};
use micold_client::features::settings::{SettingsDraft, SettingsSection};
use micold_core::env_include::EnvIncludeOutcome;

/// The window height the form must fit in.
const HEIGHT: f32 = 800.0;
/// The canonical width, tall enough that nothing is clipped from the paint.
const TALL: Size = Size::new(1280.0, 4000.0);
/// Half the height of a button (the 40dp base), to reach its bottom from the text's origin.
const HALF_BUTTON: f32 = 20.0;

#[test]
fn the_kubernetes_add_form_fits_one_window_height() {
    let mut state = State::default();
    state.settings.settings_draft = Some(SettingsDraft {
        section: SettingsSection::Daemons,
        ..SettingsDraft::default()
    });
    let _ = state.update_daemons(DaemonsMsg::AddOpened, Vec::new());
    let _ = state.update_daemons(
        DaemonsMsg::RuntimeChosen(DraftRuntime::Kubernetes),
        Vec::new(),
    );
    let mut renderer = lay::renderer();
    let element = micold_client::ui::view(
        &state,
        None,
        None,
        0,
        None,
        &EnvIncludeOutcome::Disabled,
        &ConnectionStatus::Connected,
        &micold_client::features::sandbox::Sandbox::default(),
    );
    let drawn = lay::painted_text_settled_at(element, &mut renderer, TALL).0;
    let cancel = drawn
        .iter()
        .find(|d| d.content == "Cancel")
        .unwrap_or_else(|| {
            panic!(
                "Cancel is painted — {:?}",
                drawn.iter().map(|d| &d.content).collect::<Vec<_>>()
            )
        });
    let bottom = cancel.origin.y + HALF_BUTTON;
    assert!(
        bottom <= HEIGHT,
        "the Kubernetes add form ends at {bottom}, past {HEIGHT}"
    );
}
