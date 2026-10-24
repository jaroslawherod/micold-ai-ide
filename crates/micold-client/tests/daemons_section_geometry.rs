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
        migration: None,
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
    state.window.window_size = (1280, 4000);
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
fn the_details_dialog_paints_the_errors_and_the_log_and_the_page_paints_neither() {
    let (mut state, [_, boxed, _]) = with_daemons();
    state.update(Message::Daemons(DaemonsMsg::DetailsOpened(boxed)));
    state.update(Message::Daemons(DaemonsMsg::EventReported(
        boxed,
        DaemonEvent::DialFailed("connection refused".into()),
    )));
    state.update(Message::Daemons(DaemonsMsg::LogLine(
        boxed,
        "first line".into(),
    )));
    let page = painted(&state);
    assert!(!has(&page, "Hide details"), "{page:?}");
    assert!(!has(&page, "Log"), "{page:?}");
    assert!(!has(&page, "first line"), "{page:?}");
    assert_no_text_overflows(&state, "page with details open");
    let outcome = EnvIncludeOutcome::Disabled;
    let dialog_strings = |state: &State| {
        let mut renderer = lay::renderer();
        let body =
            micold_client::ui::daemon_details_dialog::dialog(state, ColorScheme::Dark, &outcome)
                .expect("the dialog is open");
        lay::painted_text(body, &mut renderer)
            .into_iter()
            .map(|drawn| drawn.content)
            .collect::<Vec<String>>()
    };
    let strings = dialog_strings(&state);
    assert!(has(&strings, "connection refused"), "{strings:?}");
    assert!(has(&strings, "first line"), "{strings:?}");
    assert!(!has(&strings, "No connection errors"), "{strings:?}");
    assert!(!has(&strings, "Nothing logged yet"), "{strings:?}");
    assert!(has(&strings, "Close"), "{strings:?}");
    state.update(Message::Daemons(DaemonsMsg::LogLine(
        boxed,
        "second line".into(),
    )));
    let strings = dialog_strings(&state);
    assert!(has(&strings, "first line"), "{strings:?}");
    assert!(has(&strings, "second line"), "{strings:?}");
    let mut renderer = lay::renderer();
    let body =
        micold_client::ui::daemon_details_dialog::dialog(&state, ColorScheme::Dark, &outcome)
            .expect("the dialog is open");
    assert!(
        lay::text_overflows(body, &mut renderer).is_empty(),
        "the dialog text fits"
    );
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
    let painted = painted_tall(&state);
    assert!(
        has(&painted, "Another daemon is already named Idle"),
        "{painted:?}"
    );
    assert_no_text_overflows(&state, "add form with a refusal");
}

/// Feature 890, US1 AS1-AS2 / FR-002: the container form paints its six titles, Connection open
/// and the other five closed (their bodies absent), each header with the chevron for its state;
/// opening one by its header paints its body.
#[test]
fn the_container_form_paints_six_titles_with_only_connection_open() {
    use micold_client::features::daemons::Section;
    use micold_client::icons::Icon;
    let (mut state, _) = with_daemons();
    state.update(Message::Daemons(DaemonsMsg::AddOpened));
    let shown = painted_tall(&state);
    for title in [
        "Connection",
        "Container",
        "Credentials",
        "Network",
        "Resources",
        "Sessions",
    ] {
        assert!(
            shown.iter().any(|c| c == title),
            "title {title:?} missing: {shown:?}"
        );
    }
    assert!(
        has(&shown, "Container name") && has(&shown, "Connect when the application starts"),
        "Connection is open: {shown:?}"
    );
    for closed_body in [
        "Basic image (recommended)",
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
    assert_eq!(count(Icon::MoveDown, &shown), 5, "five closed headers");

    state.update(Message::Daemons(DaemonsMsg::SectionToggled(
        Section::Network,
    )));
    let shown = painted_tall(&state);
    assert!(has(&shown, "Names still resolve either way"), "{shown:?}");
    assert_eq!(
        count(Icon::MoveUp, &shown),
        1,
        "opening one closes the other"
    );
    assert_no_text_overflows(&state, "container form with Network open");
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
    for kind in [AllowedKind::GitConfig, AllowedKind::RegistryPull] {
        state.update(Message::Daemons(DaemonsMsg::AllowedToggled(kind, true)));
    }
    state.update(Message::Daemons(DaemonsMsg::Saved));
    use micold_client::features::daemons::Section;
    // One section is open at a time (feature 1091), so each is laid out in its own measurement.
    let mut form = painted_tall(&state);
    for section in [Section::Image, Section::Resources, Section::Placement] {
        state.update(Message::Daemons(DaemonsMsg::SectionToggled(section)));
        form.extend(painted_tall(&state));
        assert_no_text_overflows(&state, "the Kubernetes form, one section open");
    }
    for label in [
        "Namespace",
        "Image",
        "CPU request",
        "Memory limit",
        "Registry credentials file",
        "Node selector",
        "Tolerations",
        "Volume size",
        "Storage class",
        "Context from your kubeconfig",
    ] {
        assert!(has(&form, label), "{label}: {form:?}");
    }
    assert!(
        form.iter()
            .any(|t| t.contains("Applies only to a new volume")),
        "{form:?}"
    );
    assert_no_text_overflows(&state, "the Kubernetes form with refused files");
    state.update(Message::Daemons(DaemonsMsg::SectionToggled(
        Section::Connection,
    )));

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
    let form = painted_tall(&state);
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
        has(&painted_tall(&state), "Enter a port between 1 and 65535."),
        "the refused port names itself"
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
    state.update(Message::Daemons(DaemonsMsg::DetailsOpened(ssh)));
    assert!(
        has(&painted(&state), "The host key for build has changed"),
        "the reason is on the row"
    );
    assert_no_text_overflows(&state, "a failed SSH row with its recent errors");
}

/// Painted strings of the add form for `runtime`, in a window tall enough for all of it.
fn add_form(runtime: micold_client::features::daemons::DraftRuntime) -> (State, Vec<String>) {
    let (mut state, _) = with_daemons();
    state.update(Message::Daemons(DaemonsMsg::AddOpened));
    state.update(Message::Daemons(DaemonsMsg::RuntimeChosen(runtime)));
    let shown = painted_tall(&state);
    (state, shown)
}

/// Feature 890, FR-001, FR-002, FR-010, SC-001: for every kind the add form paints *Name* and
/// *Runs in*, the kind's section titles in order, only Connection's body, and the actions last;
/// no group label is a heading, and every section title is painted exactly once.
#[test]
fn every_kind_paints_name_runs_in_its_sections_in_order_and_the_actions() {
    use micold_client::features::daemons::{DraftRuntime, Section};
    for runtime in [
        DraftRuntime::Container,
        DraftRuntime::Kubernetes,
        DraftRuntime::Ssh,
        DraftRuntime::Host,
    ] {
        let (state, shown) = add_form(runtime);
        let form_start = shown
            .iter()
            .rposition(|c| c == "Add daemon")
            .expect("the dialog is painted");
        let runs_in = form_start
            + shown[form_start..]
                .iter()
                .position(|c| c == "Runs in")
                .expect("Runs in is painted");
        assert!(shown[form_start..runs_in].iter().any(|c| c == "Name*"));
        // The select's own value ("Container") sits before its label; the sections follow it.
        let position = |text: &str| {
            let hits: Vec<usize> = shown
                .iter()
                .enumerate()
                .skip(runs_in)
                .filter(|(_, c)| c.as_str() == text)
                .map(|(i, _)| i)
                .collect();
            assert!(!hits.is_empty(), "{runtime:?}: {text:?} in {shown:?}");
            assert!(
                hits.len() == 1 || text == "Cancel",
                "{runtime:?}: {text:?} twice"
            );
            hits[0]
        };
        let mut last = runs_in;
        for section in Section::of(runtime) {
            let at = position(section.title());
            assert!(at > last, "{runtime:?}: {} out of order", section.title());
            last = at;
        }
        let save = position("Save daemon");
        assert!(save > last && position("Cancel") > save, "{runtime:?}");
        for heading in ["Kubernetes", "Allowed credentials", "Limits"] {
            assert!(
                !shown[runs_in..].iter().any(|c| c == heading),
                "{runtime:?}: group heading {heading:?} drawn: {shown:?}"
            );
        }
        assert_no_text_overflows(&state, &format!("{runtime:?} add form"));
    }
}

/// SC-005, FR-004: the body of a closed Connection is not built, so a Tab from *Runs in* meets the
/// next section header and none of its fields (the component's own focus stops are pinned in
/// `accordion_section`'s tests).
#[test]
fn a_closed_connection_paints_none_of_its_fields() {
    use micold_client::features::daemons::{DraftRuntime, Section};
    for (runtime, field) in [
        (DraftRuntime::Container, "Container name"),
        (DraftRuntime::Kubernetes, "Namespace"),
        (DraftRuntime::Ssh, "Key file"),
        (DraftRuntime::Host, "Connect when the application starts"),
    ] {
        let (mut state, shown) = add_form(runtime);
        assert!(has(&shown, field), "{runtime:?} open: {shown:?}");
        state.update(Message::Daemons(DaemonsMsg::SectionToggled(
            Section::Connection,
        )));
        let shown = painted_tall(&state);
        assert!(!has(&shown, field), "{runtime:?} closed: {shown:?}");
        assert!(has(&shown, "Connection"), "the header stays: {shown:?}");
    }
}

// ---- Feature 890, M3: one look across the forms (FR-006 to FR-009, FR-014, SC-003). ----

/// The text the add form for `runtime` paints at 1280 x 6000, entrances done, with positions.
fn laid_out(state: &State) -> Vec<(String, f32, f32)> {
    let mut renderer = lay::renderer();
    lay::painted_text_settled_at(view(state), &mut renderer, iced::Size::new(1280.0, 6000.0))
        .0
        .into_iter()
        .map(|d| (d.content, d.origin.x, d.origin.y))
        .collect()
}

/// The add form for `runtime`, fresh: the kind's first section open.
fn fresh_form(runtime: micold_client::features::daemons::DraftRuntime) -> State {
    let (mut state, _) = with_daemons();
    state.update(Message::Daemons(DaemonsMsg::AddOpened));
    state.update(Message::Daemons(DaemonsMsg::RuntimeChosen(runtime)));
    state
}

/// What the add form for `runtime` paints with every section laid out open, with the positions:
/// `(content, x, y)`. Only one section is open at a time (feature 1091, FR-009), so each section is
/// measured open on its own: the form is its all-closed layout plus, for each section, what that
/// section adds when it opens. A position is comparable with another from the same section only.
fn open_form(
    runtime: micold_client::features::daemons::DraftRuntime,
) -> (State, Vec<(String, f32, f32)>) {
    use micold_client::features::daemons::Section;
    use std::collections::HashMap;
    let sections = Section::of(runtime);
    let mut state = fresh_form(runtime);
    state.update(Message::Daemons(DaemonsMsg::SectionToggled(sections[0])));
    let mut stitched = laid_out(&state);
    let mut closed: HashMap<String, usize> = HashMap::new();
    for (content, ..) in &stitched {
        *closed.entry(content.clone()).or_default() += 1;
    }
    for section in sections {
        state.update(Message::Daemons(DaemonsMsg::SectionToggled(*section)));
        let mut left = closed.clone();
        for entry in laid_out(&state) {
            match left.get_mut(&entry.0).filter(|n| **n > 0) {
                Some(n) => *n -= 1,
                None => stitched.push(entry),
            }
        }
        state.update(Message::Daemons(DaemonsMsg::SectionToggled(*section)));
    }
    (fresh_form(runtime), stitched)
}

fn find<'a>(shown: &'a [(String, f32, f32)], text: &str) -> &'a (String, f32, f32) {
    // A closed header with an empty required field paints its title with the marker.
    let marked = format!("{text}*");
    shown
        .iter()
        .find(|(c, ..)| c == text || (text.len() > 3 && *c == marked && !text.ends_with('*')))
        .unwrap_or_else(|| panic!("{text:?} not painted: {shown:?}"))
}

/// FR-008, US2 AS3: each group label is painted above its first checkbox or editor, and its
/// children start at the label's left edge.
#[test]
fn each_group_label_sits_above_its_controls_at_their_left_edge() {
    use micold_client::features::daemons::DraftRuntime::{Container, Kubernetes};
    let (_, container) = open_form(Container);
    let (_, kube) = open_form(Kubernetes);
    for (shown, label, first) in [
        (&container, "AI tools in this daemon", "Claude Code"),
        (&container, "Shared with the container", "Git configuration"),
        (&kube, "AI CLIs installed in the pod", "Claude Code"),
        (&kube, "Allowed credentials", "the git config"),
    ] {
        let (_, lx, ly) = find(shown, label);
        let (_, fx, fy) = find(shown, first);
        assert!(fy > ly, "{label}: its controls follow it");
        assert!(
            (fx - lx).abs() < 30.0,
            "{label}: left edge {lx} vs the checkbox row at {fx}"
        );
    }
    for label in ["Node selector", "Tolerations"] {
        let (_, _, y) = find(&kube, label);
        let (_, _, note) = find(
            &kube,
            if label == "Tolerations" {
                "One per line"
            } else {
                "One key=value per line"
            },
        );
        assert!(note > y, "{label}: its note is under it");
    }
}

/// FR-007: the explanations moved under their field, or to a section's first line.
#[test]
fn explanations_sit_where_the_contract_puts_them() {
    use micold_client::features::daemons::DraftRuntime::Container;
    let (_, shown) = open_form(Container);
    // The last match: a closed header's summary may mention a body's words; the body comes after.
    let y = |t: &str| {
        shown
            .iter()
            .rfind(|(c, ..)| c.contains(t))
            .map(|(_, _, y)| *y)
    };
    // Section-wide notes open their section, before its first control.
    assert!(y("The container starts with none").unwrap() < y("Shared with the container").unwrap());
    assert!(y("What the sandbox may consume").unwrap() < y("Processor limit").unwrap());
    // The network explanation is the select's own supporting text: under "Network" and above the
    // no-outbound caution.
    assert!(y("Names still resolve").unwrap() > y("Network*").or(y("Network")).unwrap());
    assert!(y("Names still resolve").unwrap() < y("The AI agent reaches its provider").unwrap());
    // The AI CLI note is the group's, under its label and above its first checkbox.
    assert!(y("AI tools in this daemon").unwrap() < y("Changes apply at the next start").unwrap());
    assert!(y("Changes apply at the next start").unwrap() < y("Claude Code").unwrap());
}

/// FR-009, US2 AS4: found contexts are chosen from a select above *Context*, not a column of
/// buttons; with none found only the field shows.
#[test]
fn found_contexts_are_a_select_above_the_context_field() {
    use micold_client::features::daemons::DraftRuntime::Kubernetes;
    use micold_client::kube::live::ContextEntry;
    let (mut state, none) = open_form(Kubernetes);
    assert!(!none
        .iter()
        .any(|(c, ..)| c == "Context from your kubeconfig"));
    state.update(Message::Daemons(DaemonsMsg::ContextsListed(Ok(vec![
        ContextEntry {
            name: "staging".into(),
            namespace: None,
        },
        ContextEntry {
            name: "prod".into(),
            namespace: None,
        },
    ]))));
    let mut renderer = lay::renderer();
    let shown: Vec<(String, f32, f32)> =
        lay::painted_text_settled_at(view(&state), &mut renderer, iced::Size::new(1280.0, 6000.0))
            .0
            .into_iter()
            .map(|d| (d.content, d.origin.x, d.origin.y))
            .collect();
    let (_, _, select) = find(&shown, "Context from your kubeconfig");
    let (_, _, field) = find(&shown, "Context*");
    assert!(select < field, "the select is above the field");
    assert!(
        !shown.iter().any(|(c, ..)| c == "staging" || c == "prod"),
        "no per-context button is painted: {shown:?}"
    );
}

/// FR-006, SC-003: the gap between consecutive plain fields is one value in every kind, and the
/// gap between closed section headers is one value in every kind.
#[test]
fn field_and_section_gaps_take_one_value_across_kinds() {
    use micold_client::features::daemons::{DraftRuntime, Section};
    let step = |s: &[(String, f32, f32)], a: &str, b: &str| find(s, b).2 - find(s, a).2;
    let (_, c) = open_form(DraftRuntime::Container);
    let (_, ssh) = open_form(DraftRuntime::Ssh);
    let field = step(&ssh, "User", "Port");
    assert!((step(&ssh, "Port", "Key file") - field).abs() < 0.5);
    assert!(
        (step(&c, "Container name", "Port") - field).abs() < 0.5,
        "container {c:?}"
    );

    let header_gaps = |runtime| {
        let (mut state, _) = with_daemons();
        state.update(Message::Daemons(DaemonsMsg::AddOpened));
        state.update(Message::Daemons(DaemonsMsg::RuntimeChosen(runtime)));
        // Close the open Connection so every header is a closed one.
        state.update(Message::Daemons(DaemonsMsg::SectionToggled(
            Section::Connection,
        )));
        let shown = painted_tall(&state);
        let mut renderer = lay::renderer();
        let at: Vec<(String, f32, f32)> = lay::painted_text_settled_at(
            view(&state),
            &mut renderer,
            iced::Size::new(1280.0, 6000.0),
        )
        .0
        .into_iter()
        .map(|d| (d.content, d.origin.x, d.origin.y))
        .collect();
        let _ = shown;
        // Past *Runs in*, whose select paints its value ("Container") before its label.
        let from = at.iter().position(|(c, ..)| c == "Runs in").unwrap();
        let at = &at[from..];
        Section::of(runtime)
            .windows(2)
            .map(|w| find(at, w[1].title()).2 - find(at, w[0].title()).2)
            .collect::<Vec<_>>()
    };
    let all: Vec<f32> = [
        DraftRuntime::Container,
        DraftRuntime::Kubernetes,
        DraftRuntime::Host,
    ]
    .into_iter()
    .flat_map(header_gaps)
    .collect();
    assert!(
        all.windows(2).all(|w| (w[0] - w[1]).abs() < 0.5),
        "section pitch: {all:?}"
    );
}

/// FR-014, US3 AS1/AS3: closed headers paint their summary, and the title takes `*` for an empty
/// required field.
#[test]
fn closed_headers_paint_their_summary_and_the_required_marker() {
    use micold_client::features::daemons::DraftRuntime;
    let (mut state, _) = with_daemons();
    state.update(Message::Daemons(DaemonsMsg::AddOpened));
    let shown = painted_tall(&state);
    for summary in [
        "Docker · Basic image · ",
        "Nothing shared",
        "No outbound connections",
        "Processes 512",
        "Stops when signed out or idle",
    ] {
        assert!(has(&shown, summary), "{summary:?}: {shown:?}");
    }
    // Connection is open (no summary, nothing to mark on the header); Container is closed and
    // holds nothing required while the image is the basic one.
    assert!(!shown.iter().any(|c| c == "Container*"));
    state.update(Message::Daemons(DaemonsMsg::RuntimeChosen(
        DraftRuntime::Ssh,
    )));
    state.update(Message::Daemons(DaemonsMsg::SectionToggled(
        micold_client::features::daemons::Section::Connection,
    )));
    let shown = painted_tall(&state);
    assert!(shown.iter().any(|c| c == "Connection*"), "{shown:?}");
    assert!(
        has(&shown, "Connects at launch"),
        "the closed SSH summary: {shown:?}"
    );
}

// ---- Feature 1092, T007: the form lives in a dialog, not on the page. ----

/// What is painted with the form dialog open for `id`, at `window`, entrances done.
fn dialog_painted_at(state: &mut State, window: iced::Size) -> Vec<lay::Overflow> {
    state.window.window_size = (window.width as u16, window.height as u16);
    let mut renderer = lay::renderer();
    lay::painted_text_settled_at(view(state), &mut renderer, window).0
}

/// SC-006: with a draft open, the daemon form's fields are painted once, in the dialog, and the
/// dialog's own Save daemon is what saves — the Settings page behind it carries none of them.
#[test]
fn an_open_draft_paints_the_form_in_the_dialog_and_not_on_the_page() {
    let (mut state, [host, ..]) = with_daemons();
    let closed = painted_tall(&state);
    assert!(!has(&closed, "Save daemon"), "{closed:?}");
    assert!(!has(&closed, "Runs in"), "{closed:?}");
    state.update(Message::Daemons(DaemonsMsg::EditOpened(host)));
    let open = painted_tall(&state);
    for once in ["Runs in", "Save daemon"] {
        assert_eq!(
            open.iter().filter(|c| c.as_str() == once).count(),
            1,
            "{once:?} in {open:?}"
        );
    }
}

/// SC-005: at the smallest supported window the dialog's Save daemon and Cancel are still inside
/// it, however long the form is (the body scrolls, the actions stay).
#[test]
fn the_dialog_actions_stay_inside_the_smallest_window() {
    use micold_client::app::MIN_WINDOW_SIZE;
    let (mut state, [_, boxed, _]) = with_daemons();
    state.update(Message::Daemons(DaemonsMsg::EditOpened(boxed)));
    let drawn = dialog_painted_at(&mut state, MIN_WINDOW_SIZE);
    let save = drawn
        .iter()
        .find(|d| d.content == "Save daemon")
        .expect("Save daemon is painted");
    let cancel = drawn
        .iter()
        .find(|d| d.content == "Cancel" && (d.origin.y - save.origin.y).abs() < 1.0)
        .expect("the dialog's Cancel is painted beside Save daemon");
    for action in [save, cancel] {
        assert!(
            action.origin.y + action.natural_height <= MIN_WINDOW_SIZE.height
                && action.origin.x + action.natural_width <= MIN_WINDOW_SIZE.width,
            "{:?} at {:?} is outside {MIN_WINDOW_SIZE:?}",
            action.content,
            action.origin
        );
    }
}

/// FR-013: opening the form leaves the rows' status and actions on the page, each painted as its
/// own label (an exact match, so the dialog's own text cannot stand in for it).
#[test]
fn the_rows_keep_their_status_and_actions_with_the_dialog_open() {
    let (mut state, [host, ..]) = with_daemons();
    let exact = |painted: &[String], label: &str| painted.iter().any(|c| c == label);
    let before = painted_tall(&state);
    let needles = [
        "Connected",
        "Unreachable",
        "Edit",
        "Details",
        "Remove",
        "Start",
        "Stop",
    ];
    for needle in needles {
        assert!(
            exact(&before, needle),
            "{needle:?} not on the page: {before:?}"
        );
    }
    state.update(Message::Daemons(DaemonsMsg::EditOpened(host)));
    let after = painted_tall(&state);
    for needle in needles {
        assert!(
            exact(&after, needle),
            "{needle:?} gone when the dialog opened: {after:?}"
        );
    }
}
