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

/// What the page paints in a window tall enough to hold the whole container form, entrances done,
/// so nothing is clipped from the paint.
fn painted_tall(state: &State) -> Vec<String> {
    let mut renderer = lay::renderer();
    lay::painted_text_settled_at(view(state), &mut renderer, iced::Size::new(1280.0, 4000.0))
        .0
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

/// Feature 804, US4 AS1 / SC-005: the container form paints its five titles, the Container
/// section open and the other four closed (their bodies absent), each header with the chevron for
/// its state; opening one by its header paints its body.
#[test]
fn the_container_form_paints_five_titles_with_only_container_open() {
    use micold_client::features::daemons::Section;
    use micold_client::icons::Icon;
    let (mut state, _) = with_daemons();
    state.update(Message::Daemons(DaemonsMsg::AddOpened));
    let shown = painted_tall(&state);
    for title in ["Container", "Credentials", "Network", "Limits", "Sessions"] {
        assert!(
            shown.iter().any(|c| c == title),
            "title {title:?} missing: {shown:?}"
        );
    }
    assert!(has(&shown, "Image source"), "Container is open: {shown:?}");
    for closed_body in [
        "The container starts with none of your credentials",
        "Names still resolve either way",
        "What the sandbox may consume",
    ] {
        assert!(
            !has(&shown, closed_body),
            "{closed_body:?} shown in a closed section: {shown:?}"
        );
    }
    let count = |icon: Icon, painted: &[String]| {
        painted
            .iter()
            .filter(|c| **c == icon.glyph().to_string())
            .count()
    };
    assert_eq!(count(Icon::MoveUp, &shown), 1, "one open header");
    assert_eq!(count(Icon::MoveDown, &shown), 4, "four closed headers");

    state.update(Message::Daemons(DaemonsMsg::SectionToggled(
        Section::Network,
    )));
    let shown = painted_tall(&state);
    assert!(has(&shown, "Names still resolve either way"), "{shown:?}");
    assert_eq!(count(Icon::MoveUp, &shown), 2);
    assert_no_text_overflows(&state, "container form with two sections open");
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

/// Feature 489, T021: the SSH fields of the add form and an SSH row in each state badge hold.
#[test]
fn the_ssh_form_and_ssh_rows_paint_their_fields_and_badges_without_overflow() {
    let (mut state, _) = with_daemons();
    let target = micold_core::ssh::SshTarget::new(
        "a-rather-long-host-name.internal.example.com",
        Some("deploy"),
        Some(2222),
        None,
    )
    .unwrap();
    let ssh = state
        .settings
        .daemons
        .add(
            "Remote over SSH",
            DaemonRuntime::Ssh(micold_core::ssh::SshSettings { target }),
            true,
        )
        .unwrap();
    state.settings.links = DaemonLinks::at_launch(&state.settings.daemons);
    use micold_core::provision::{ProvisionEvent, Step};
    state.settings.links.apply(ssh, DaemonEvent::Start);
    state.update(Message::Daemons(DaemonsMsg::Provisioning(
        ssh,
        ProvisionEvent::Entered(Step::Copy),
    )));
    state.update(Message::Daemons(DaemonsMsg::Provisioning(
        ssh,
        ProvisionEvent::CopyPercent(40),
    )));
    let painted_rows = painted(&state);
    assert!(has(&painted_rows, "Remote over SSH"), "{painted_rows:?}");
    assert!(has(&painted_rows, "copying the daemon"), "{painted_rows:?}");
    assert_no_text_overflows(&state, "an SSH row while starting");

    state.settings.links.apply(ssh, DaemonEvent::Connected);
    assert_no_text_overflows(&state, "an SSH row connected");

    state.update(Message::Daemons(DaemonsMsg::AddOpened));
    state.update(Message::Daemons(DaemonsMsg::RuntimeChosen(
        micold_client::features::daemons::DraftRuntime::Ssh,
    )));
    state.update(Message::Daemons(DaemonsMsg::NameEdited("Far".into())));
    state.update(Message::Daemons(DaemonsMsg::SshHostEdited("-bad".into())));
    state.update(Message::Daemons(DaemonsMsg::Saved));
    let form = painted(&state);
    assert!(has(&form, "Host"), "{form:?}");
    assert_no_text_overflows(&state, "the SSH add form with a refused host");
}

/// Feature 489, T031: the repository field of a connected SSH row paints its label and a refusal
/// without overflow.
#[test]
fn the_repository_field_paints_its_label_and_a_refusal_without_overflow() {
    let (mut state, _) = with_daemons();
    let target =
        micold_core::ssh::SshTarget::new("devbox.internal.example.com", None, None, None).unwrap();
    let ssh = state
        .settings
        .daemons
        .add(
            "Remote over SSH",
            DaemonRuntime::Ssh(micold_core::ssh::SshSettings { target }),
            true,
        )
        .unwrap();
    state.settings.links = DaemonLinks::at_launch(&state.settings.daemons);
    state.settings.links.apply(ssh, DaemonEvent::Connected);
    let painted_rows = painted(&state);
    assert!(
        has(&painted_rows, "Repository on the host"),
        "{painted_rows:?}"
    );
    assert_no_text_overflows(&state, "the repository field");

    state.update(Message::Daemons(DaemonsMsg::RepositoryEdited(
        ssh,
        "relative/path/that/is/rather/long/for/a/repository/on/a/remote/host".into(),
    )));
    state.update(Message::Daemons(DaemonsMsg::RepositoryAdded(ssh)));
    assert_no_text_overflows(&state, "the repository field with a refusal");
}

/// Feature 489, T042: a reconnecting SSH row paints its badge, its reason and its actions without
/// overflow, and the row after the user stopped trying paints the contract's sentence.
#[test]
fn a_reconnecting_ssh_row_paints_its_badge_reason_and_actions_without_overflow() {
    let (mut state, _) = with_daemons();
    let target = micold_core::ssh::SshTarget::new(
        "a-rather-long-host-name.internal.example.com",
        Some("deploy"),
        Some(2222),
        None,
    )
    .unwrap();
    let ssh = state
        .settings
        .daemons
        .add(
            "Remote over SSH",
            DaemonRuntime::Ssh(micold_core::ssh::SshSettings { target }),
            true,
        )
        .unwrap();
    state.settings.links = DaemonLinks::at_launch(&state.settings.daemons);
    state.settings.links.apply(ssh, DaemonEvent::Connected);
    state
        .settings
        .links
        .apply(ssh, DaemonEvent::Lost("Connection timed out".into()));
    let rows = painted(&state);
    for needle in [
        "Reconnecting",
        "reconnecting to deploy@a-rather-long-host-name.internal.example.com:2222: Connection timed out",
        "Stop trying",
        "Reconnect",
    ] {
        assert!(has(&rows, needle), "{needle:?} missing: {rows:?}");
    }
    assert_no_text_overflows(&state, "a reconnecting SSH row");

    state.update(Message::Daemons(DaemonsMsg::GaveUp(ssh)));
    let rows = painted(&state);
    assert!(
        has(
            &rows,
            "not connected to deploy@a-rather-long-host-name.internal.example.com:2222: attempts stopped"
        ),
        "{rows:?}"
    );
    assert!(!has(&rows, "Stop trying"), "{rows:?}");
    assert_no_text_overflows(&state, "an SSH row after stopping the attempts");
}

/// Feature 686, T032: the Kubernetes form (context list, resources, allowed credentials with their
/// file fields, the volume note) and a Kubernetes row paint their labels without overflow.
#[test]
fn the_kubernetes_form_and_row_paint_their_fields_without_overflow() {
    use micold_client::features::daemons::AllowedKind;
    use micold_client::kube::live::ContextEntry;
    let (mut state, _) = with_daemons();
    let kube = state
        .settings
        .daemons
        .add(
            "Cluster",
            DaemonRuntime::Kubernetes(micold_core::kube::settings::KubeSettings::new(
                "a-rather-long-context-name.eu-west-1.eks.example.com",
                "team-namespace",
                &[micold_core::session::AiCli::ClaudeCode],
            )),
            false,
        )
        .unwrap();
    state.settings.links = DaemonLinks::at_launch(&state.settings.daemons);
    let rows = painted(&state);
    assert!(has(&rows, "Kubernetes"), "{rows:?}");
    assert_no_text_overflows(&state, "a Kubernetes row");

    state.update(Message::Daemons(DaemonsMsg::EditOpened(kube)));
    state.update(Message::Daemons(DaemonsMsg::ContextsListed(Ok(vec![
        ContextEntry {
            name: "staging".into(),
            namespace: Some("apps".into()),
        },
    ]))));
    for kind in [AllowedKind::GitSshKey, AllowedKind::RegistryPull] {
        state.update(Message::Daemons(DaemonsMsg::AllowedToggled(kind, true)));
    }
    state.update(Message::Daemons(DaemonsMsg::Saved));
    let form = painted_tall(&state);
    for label in [
        "Kubernetes",
        "Namespace",
        "Image",
        "CPU request",
        "Memory limit",
        "Allowed credentials",
        "SSH key file",
        "Registry credentials file",
        "Node selector",
        "Tolerations",
        "Volume size",
        "Storage class",
        "staging",
    ] {
        assert!(has(&form, label), "{label}: {form:?}");
    }
    assert!(
        form.iter()
            .any(|t| t.contains("Applies only to a new volume")),
        "{form:?}"
    );
    assert_no_text_overflows(&state, "the Kubernetes form with refused files");

    state.update(Message::Daemons(DaemonsMsg::ContextsListed(Err(
        "no kubeconfig found; looked in /home/someone/.kube/config".into(),
    ))));
    let failed = painted_tall(&state);
    assert!(
        failed
            .iter()
            .any(|t| t.contains("no kubeconfig found; looked in")),
        "{failed:?}"
    );
    assert_no_text_overflows(&state, "the Kubernetes form without a kubeconfig");
}

/// Feature 686, T082: the placement fields refuse beside themselves, and a Kubernetes row paints
/// its Logs and Forget locally actions, the pod log and the next-start note without overflow.
#[test]
fn the_kubernetes_placement_errors_and_row_actions_paint_without_overflow() {
    let (mut state, _) = with_daemons();
    let kube = state
        .settings
        .daemons
        .add(
            "Cluster",
            DaemonRuntime::Kubernetes(micold_core::kube::settings::KubeSettings::new(
                "ctx",
                "ns",
                &[],
            )),
            false,
        )
        .unwrap();
    state.settings.links = DaemonLinks::at_launch(&state.settings.daemons);
    let row = painted_tall(&state);
    for label in ["Logs", "Forget locally", "Remove"] {
        assert!(has(&row, label), "{label}: {row:?}");
    }
    state.update(Message::Daemons(DaemonsMsg::LogsFetched(
        kube,
        Ok("pod started\nsshd listening".into()),
    )));
    assert!(has(&painted_tall(&state), "sshd listening"));
    assert_no_text_overflows(&state, "a Kubernetes row with its pod log");

    state.update(Message::Daemons(DaemonsMsg::EditOpened(kube)));
    state.update(Message::Daemons(DaemonsMsg::VolumeSizeEdited(
        "lots".into(),
    )));
    state.update(Message::Daemons(DaemonsMsg::Saved));
    let form = painted_tall(&state);
    assert!(has(&form, "Volume size"), "{form:?}");
    assert!(has(&form, "The volume must be at least 1 GiB."), "{form:?}");
    assert_no_text_overflows(&state, "the Kubernetes form with a refused volume size");
}

/// Feature 489, T059: the host picker and the refusal rows of the SSH form paint without overflow,
/// and a failed SSH daemon's recent errors paint their named reasons.
#[test]
fn the_host_picker_and_the_error_rows_paint_without_overflow() {
    let (mut state, _) = with_daemons();
    state.update(Message::Daemons(DaemonsMsg::AddOpened));
    state.update(Message::Daemons(DaemonsMsg::RuntimeChosen(
        micold_client::features::daemons::DraftRuntime::Ssh,
    )));
    state.update(Message::Daemons(DaemonsMsg::SshHostsLoaded(vec![
        "build".into(),
        "a-rather-long-host-name.internal.example.com".into(),
    ])));
    let form = painted(&state);
    assert!(
        has(&form, "Host from your SSH configuration"),
        "the picker is drawn: {form:?}"
    );
    assert_no_text_overflows(&state, "the SSH form with the host picker");

    state.update(Message::Daemons(DaemonsMsg::SshHostChosen("build".into())));
    state.update(Message::Daemons(DaemonsMsg::NameEdited(
        "A name that is long enough to need the whole width of the form field".into(),
    )));
    state.update(Message::Daemons(DaemonsMsg::SshPortEdited("99999".into())));
    state.update(Message::Daemons(DaemonsMsg::Saved));
    assert!(
        has(&painted(&state), "Enter a port between 1 and 65535."),
        "the refused port names itself: {:?}",
        painted(&state)
    );
    assert_no_text_overflows(&state, "the SSH form with a refused port");

    // A failed daemon lists its recent errors with the named reason.
    state.update(Message::Daemons(DaemonsMsg::DraftDiscarded));
    let target = micold_core::ssh::SshTarget::new("build", None, None, None).unwrap();
    let ssh = state
        .settings
        .daemons
        .add(
            "Build",
            DaemonRuntime::Ssh(micold_core::ssh::SshSettings { target }),
            true,
        )
        .unwrap();
    state.settings.links = DaemonLinks::at_launch(&state.settings.daemons);
    let reason = "The host key for build has changed. Check with the host's owner, then fix known_hosts yourself.";
    state.update(Message::Daemons(DaemonsMsg::EventReported(
        ssh,
        DaemonEvent::StartFailed(reason.into()),
    )));
    state.update(Message::Daemons(DaemonsMsg::DetailsToggled(ssh)));
    assert!(
        has(&painted(&state), "The host key for build has changed"),
        "the reason is on the row"
    );
    assert_no_text_overflows(&state, "a failed SSH row with its recent errors");
}
