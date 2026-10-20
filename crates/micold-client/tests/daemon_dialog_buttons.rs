//! What the daemon dialogs' and the Daemons page's buttons send, and what the scrim does (feature
//! 1092, FR-002, FR-005, FR-009, FR-012). Each test presses the real built view, so a button
//! wired to the wrong message fails here and not only in a visual pass.

#[path = "support/mod.rs"]
mod support;

use iced::advanced::layout::Layout;
use iced::advanced::widget::operation::{Focusable, Outcome};
use iced::advanced::widget::{Id, Operation, Tree};
use iced::{Element, Rectangle};
use micold_client::app::{Message, State};
use micold_client::features::connection::ConnectionStatus;
use micold_client::features::daemons::Msg as DaemonsMsg;
use micold_client::features::daemons::{Opener, RowAction};
use micold_client::features::settings::{SettingsDraft, SettingsSection};
use micold_client::links::DaemonLinks;
use micold_core::daemons::{DaemonId, DaemonRegistry, DaemonRuntime};
use micold_core::env_include::EnvIncludeOutcome;
use micold_core::theme::ColorScheme;
use support::layout as lay;

fn state() -> (State, [DaemonId; 2]) {
    let mut state = State::default();
    let mut registry = DaemonRegistry::new(Vec::new(), 1);
    let host = registry.add("Host", DaemonRuntime::Host, false).unwrap();
    let other = registry
        .add(
            "Other",
            DaemonRuntime::Container(micold_core::daemons::ContainerSettings {
                profile: micold_core::sandbox::SandboxProfile::default(),
                container_name: "other".into(),
                port: 7002,
                migration: None,
            }),
            false,
        )
        .unwrap();
    state.settings.links = DaemonLinks::at_launch(&registry);
    state.settings.daemons = registry;
    state.settings.settings_draft = Some(SettingsDraft {
        section: SettingsSection::Daemons,
        ..SettingsDraft::default()
    });
    state.window.window_size = (1280, 800);
    (state, [host, other])
}

fn page(state: &State) -> Element<'_, Message> {
    micold_client::ui::view(
        state,
        None,
        None,
        0,
        None,
        &EnvIncludeOutcome::Disabled,
        &ConnectionStatus::Connected,
        &micold_client::features::sandbox::Sandbox::default(),
    )
}

fn window() -> Rectangle {
    Rectangle::with_size(lay::WINDOW)
}

/// Every distinct message a click anywhere in the window publishes.
fn sweep(element: Element<'_, Message>) -> Vec<Message> {
    let renderer = lay::renderer();
    let mut seen: Vec<Message> = Vec::new();
    for m in lay::clicks_at(element, &renderer, lay::grid(window(), 6.0)) {
        if !seen.contains(&m) {
            seen.push(m);
        }
    }
    seen
}

fn daemons(msg: DaemonsMsg) -> Message {
    Message::Daemons(msg)
}

#[test]
fn the_form_dialog_buttons_send_save_and_discard() {
    let (mut state, _) = state();
    state.update(daemons(DaemonsMsg::AddOpened));
    let outcome = EnvIncludeOutcome::Disabled;
    let dialog = micold_client::ui::daemon_form_dialog::dialog(&state, ColorScheme::Dark, &outcome)
        .expect("the form is open");
    let sent = sweep(dialog);
    assert!(sent.contains(&daemons(DaemonsMsg::Saved)), "Save daemon: {sent:?}");
    assert!(
        sent.contains(&daemons(DaemonsMsg::DraftDiscarded)),
        "Cancel: {sent:?}"
    );
}

#[test]
fn the_details_dialog_close_sends_details_closed() {
    let (mut state, [host, _]) = state();
    state.update(daemons(DaemonsMsg::DetailsOpened(host)));
    let outcome = EnvIncludeOutcome::Disabled;
    let dialog =
        micold_client::ui::daemon_details_dialog::dialog(&state, ColorScheme::Dark, &outcome)
            .expect("details are open");
    let sent = sweep(dialog);
    assert!(
        sent.contains(&daemons(DaemonsMsg::DetailsClosed)),
        "Close: {sent:?}"
    );
}

#[test]
fn the_page_buttons_open_the_dialog_for_their_row() {
    let (state, [host, other]) = state();
    let sent = sweep(page(&state));
    for (what, msg) in [
        ("Add", DaemonsMsg::AddOpened),
        ("Edit of the first row", DaemonsMsg::EditOpened(host)),
        ("Edit of the second row", DaemonsMsg::EditOpened(other)),
        ("Details of the first row", DaemonsMsg::DetailsOpened(host)),
        ("Details of the second row", DaemonsMsg::DetailsOpened(other)),
    ] {
        assert!(sent.contains(&daemons(msg)), "{what}: {sent:?}");
    }
}

/// A click on the scrim, in a corner no dialog reaches, publishes the dialog's cancellation (FR-005).
fn scrim_click(state: &State) -> Vec<Message> {
    let renderer = lay::renderer();
    lay::clicks_at(page(state), &renderer, [iced::Point::new(2.0, 2.0)])
}

#[test]
fn clicking_the_scrim_discards_the_form_draft() {
    let (mut state, _) = state();
    state.update(daemons(DaemonsMsg::AddOpened));
    assert_eq!(
        scrim_click(&state),
        vec![daemons(DaemonsMsg::DraftDiscarded)]
    );
}

#[test]
fn clicking_the_scrim_closes_the_details_dialog() {
    let (mut state, [host, _]) = state();
    state.update(daemons(DaemonsMsg::DetailsOpened(host)));
    assert_eq!(
        scrim_click(&state),
        vec![daemons(DaemonsMsg::DetailsClosed)]
    );
}

/// The widget ids of every focusable in the view, as the focus operation sees them.
struct Ids(Vec<Id>);

impl Operation for Ids {
    fn traverse(&mut self, operate: &mut dyn FnMut(&mut dyn Operation)) {
        operate(self);
    }

    fn focusable(&mut self, id: Option<&Id>, _bounds: Rectangle, _state: &mut dyn Focusable) {
        self.0.extend(id.cloned());
    }

    fn finish(&self) -> Outcome<()> {
        Outcome::None
    }
}

#[test]
fn the_add_and_row_buttons_carry_the_ids_focus_returns_to() {
    use micold_client::ui::daemon_opener_id as id_of;
    let (state, [host, other]) = state();
    let renderer = lay::renderer();
    let mut element = page(&state);
    let mut tree = Tree::new(element.as_widget());
    let node = element.as_widget_mut().layout(
        &mut tree,
        &renderer,
        &iced::advanced::layout::Limits::new(iced::Size::ZERO, lay::WINDOW),
    );
    let mut ids = Ids(Vec::new());
    element
        .as_widget_mut()
        .operate(&mut tree, Layout::new(&node), &renderer, &mut ids);
    let openers = [
        Opener::Add,
        Opener::Row(host, RowAction::Edit),
        Opener::Row(host, RowAction::Details),
        Opener::Row(other, RowAction::Edit),
        Opener::Row(other, RowAction::Details),
    ];
    for (n, opener) in openers.iter().enumerate() {
        assert!(ids.0.contains(&id_of(*opener)), "{opener:?} has no button: {:?}", ids.0);
        for later in &openers[n + 1..] {
            assert_ne!(id_of(*opener), id_of(*later), "two openers share an id");
        }
    }
}
