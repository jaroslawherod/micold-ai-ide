//! The Daemons section and the removal dialog hold their geometry (feature 491, T032).
//!
//! Modelled on `a_field_note_shares_its_fields_column.rs` and `layout_text_overflow.rs`: the view
//! is asked what the renderer actually painted, because a row built from a name, a runtime tag and
//! a state badge can overrun without any layout node being wrong. Covered: several rows, a row
//! with its details open, the empty state, the add form with a refused field, and the removal
//! confirmation.

#[path = "support/mod.rs"]
mod support;

use micold_client::app::{Message, State};
use micold_client::features::connection::ConnectionStatus;
use micold_client::features::daemons::Msg as DaemonsMsg;
use micold_client::features::settings::{SettingsDraft, SettingsSection};
use micold_client::links::DaemonLinks;
use micold_core::daemons::{
    ContainerSettings, DaemonEvent, DaemonId, DaemonRegistry, DaemonRuntime,
};
use micold_core::env_include::EnvIncludeOutcome;
use micold_core::sandbox::SandboxProfile;
use micold_core::theme::ColorScheme;
use support::layout as lay;

const TOLERANCE: f32 = 0.5;

fn container(name: &str, port: u16) -> DaemonRuntime {
    DaemonRuntime::Container(ContainerSettings {
        profile: SandboxProfile::default(),
        container_name: name.into(),
        port,
    })
}

/// Host (connected), a long-named container (unreachable) and a stopped one.
fn with_daemons() -> (State, [DaemonId; 3]) {
    let mut state = State::default();
    let mut registry = DaemonRegistry::new(Vec::new(), 1);
    let host = registry.add("Host", DaemonRuntime::Host, true).unwrap();
    let boxed = registry
        .add(
            "A container daemon with a rather long display name",
            container("box", 7001),
            true,
        )
        .unwrap();
    let idle = registry
        .add("Idle", container("idle", 7002), false)
        .unwrap();
    let mut links = DaemonLinks::at_launch(&registry);
    links.apply(host, DaemonEvent::Connected);
    for _ in 0..4 {
        links.apply(boxed, DaemonEvent::DialFailed("connection refused".into()));
    }
    state.settings.daemons = registry;
    state.settings.links = links;
    state.settings.settings_draft = Some(SettingsDraft {
        section: SettingsSection::Daemons,
        ..SettingsDraft::default()
    });
    state.window.window_size = (1280, 900);
    (state, [host, boxed, idle])
}

fn view(state: &State) -> iced::Element<'_, Message> {
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

fn painted(state: &State) -> Vec<String> {
    let mut renderer = lay::renderer();
    lay::painted_text(view(state), &mut renderer)
        .into_iter()
        .map(|drawn| drawn.content)
        .collect()
}

fn assert_no_text_overflows(state: &State, what: &str) {
    let mut renderer = lay::renderer();
    let overflows: Vec<_> = lay::text_overflows(view(state), &mut renderer)
        .into_iter()
        .filter(|o| o.excess() > TOLERANCE)
        .map(|o| {
            format!(
                "{:?} wants {} in {}",
                o.content, o.natural_width, o.allowed_width
            )
        })
        .collect();
    assert!(overflows.is_empty(), "{what}: {overflows:?}");
}

fn has(painted: &[String], needle: &str) -> bool {
    painted.iter().any(|c| c.contains(needle))
}

#[test]
fn several_rows_paint_name_runtime_and_state_without_overflow() {
    let (state, _) = with_daemons();
    let painted = painted(&state);
    for needle in [
        "Host",
        "Container",
        "Connected",
        "Unreachable",
        "Stopped",
        "Idle",
    ] {
        assert!(has(&painted, needle), "{needle:?} missing: {painted:?}");
    }
    assert_no_text_overflows(&state, "several rows");
}

#[test]
fn a_row_with_its_details_open_paints_its_errors_and_log() {
    let (mut state, [_, boxed, _]) = with_daemons();
    state.update(Message::Daemons(DaemonsMsg::DetailsToggled(boxed)));
    let painted = painted(&state);
    assert!(has(&painted, "Hide details"), "{painted:?}");
    assert!(has(&painted, "connection refused"), "{painted:?}");
    assert_no_text_overflows(&state, "details open");
}

#[test]
fn the_empty_state_explains_and_offers_the_add_action() {
    let (mut state, _) = with_daemons();
    state.settings.daemons = DaemonRegistry::new(Vec::new(), 1);
    state.settings.links = DaemonLinks::default();
    let painted = painted(&state);
    assert!(has(&painted, "No daemons are set up"), "{painted:?}");
    assert!(has(&painted, "Add daemon"), "{painted:?}");
    assert_no_text_overflows(&state, "empty state");
}

#[test]
fn the_add_form_shows_a_refusal_beside_its_field() {
    let (mut state, _) = with_daemons();
    state.update(Message::Daemons(DaemonsMsg::AddOpened));
    state.update(Message::Daemons(DaemonsMsg::NameEdited("Idle".into())));
    state.update(Message::Daemons(DaemonsMsg::ContainerNameEdited(
        "fresh".into(),
    )));
    state.update(Message::Daemons(DaemonsMsg::PortEdited("7003".into())));
    state.update(Message::Daemons(DaemonsMsg::Saved));
    let painted = painted(&state);
    assert!(
        has(&painted, "Another daemon is already named Idle"),
        "{painted:?}"
    );
    assert_no_text_overflows(&state, "add form with a refusal");
}

#[test]
fn the_removal_dialog_states_both_counts_and_fits() {
    let (mut state, [host, ..]) = with_daemons();
    state.update(Message::Daemons(DaemonsMsg::RemovalRequested(host)));
    // The dialog is a floating surface, not part of the page: it is laid out from its own body.
    let outcome = EnvIncludeOutcome::Disabled;
    let body = || {
        micold_client::ui::confirm_daemon_removal::dialog(&state, ColorScheme::Dark, &outcome)
            .expect("a removal is pending")
    };
    let mut renderer = lay::renderer();
    let painted: Vec<String> = lay::painted_text(body(), &mut renderer)
        .into_iter()
        .map(|drawn| drawn.content)
        .collect();
    assert!(has(&painted, "0 worktrees"), "{painted:?}");
    assert!(has(&painted, "0 running sessions"), "{painted:?}");
    assert!(has(&painted, "nothing on disk is deleted"), "{painted:?}");
    assert!(
        has(&painted, "Continue") && has(&painted, "Cancel"),
        "{painted:?}"
    );
    let overflows: Vec<_> = lay::text_overflows(body(), &mut renderer)
        .into_iter()
        .filter(|o| o.excess() > TOLERANCE)
        .collect();
    assert!(overflows.is_empty(), "removal dialog: {overflows:?}");
}
