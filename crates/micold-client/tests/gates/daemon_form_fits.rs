//! The Kubernetes add form fits one window height (feature 890, T009, SC-002).
//!
//! Before the sections the Kubernetes form listed every field at once and needed several window
//! heights. With default section states, no contexts found and no error shown, its section list
//! and actions must end — the bottom of *Cancel* — within 800 logical pixels at the canonical
//! Settings width. Asks the renderer where it drew the text.
//!
//! The form is a dialog now (feature 1092) and scrolls when the window is short, so what is held
//! to a window height is the dialog's own extent: from its title to the bottom of *Cancel*. The
//! window is tall enough here that the dialog is not scrolling, and it is centred in it, so the
//! extent is measured rather than the absolute position (the dialog must only sit inside the window).

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
    state.window.window_size = (TALL.width as u16, TALL.height as u16);
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
    // The page behind carries its own Cancel; the dialog's sits on the row of its *Save daemon*.
    let save = drawn
        .iter()
        .find(|d| d.content == "Save daemon")
        .expect("Save daemon is painted");
    let cancel = drawn
        .iter()
        .find(|d| d.content == "Cancel" && (d.origin.y - save.origin.y).abs() < 1.0)
        .expect("the dialog's Cancel is painted on the Save daemon row");
    // The title is the largest "Add daemon" text; the page's own button carries the same words.
    let title = drawn
        .iter()
        .filter(|d| d.content == "Add daemon")
        .max_by(|a, b| a.natural_height.total_cmp(&b.natural_height))
        .expect("the dialog title is painted");
    let extent = cancel.origin.y + HALF_BUTTON - title.origin.y;
    // The dialog is centred in the forced-tall window, so its absolute bottom is not a height to
    // hold to 800; it must still sit inside that window, not be placed off it.
    assert!(
        title.origin.y >= 0.0 && cancel.origin.y + HALF_BUTTON <= TALL.height,
        "the dialog spans {}..{}, outside the {} tall window",
        title.origin.y,
        cancel.origin.y + HALF_BUTTON,
        TALL.height
    );
    assert!(
        extent <= HEIGHT,
        "the Kubernetes add form is {extent} tall from its title, past {HEIGHT}"
    );
}
