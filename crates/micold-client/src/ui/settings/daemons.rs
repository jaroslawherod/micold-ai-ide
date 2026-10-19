//! The Daemons section: every configured daemon, its state and what runs on it, with the actions
//! to start, stop, reconnect, edit and remove it (feature 491, M3: US3, FR-010 to FR-015).
//!
//! A container daemon's form carries its runtime, image, credential shares, network, limits and
//! survive-logout (feature 804), saved by the form's own save with its restart confirmation.
//!
//! A row says what a daemon runs in as a plain label (FR-015): nothing here branches on the kind
//! of runtime except the form's container fields.
//!
//! This section renders nothing from the settings draft: the registry is saved by its own form
//! (`DaemonsMsg::Saved`), so it declares no `SETTINGS` and `tests/settings_sections.rs` records
//! `daemons`, `next_daemon_id` and `legacy_default_daemon` as chosen here.

use crate::app::{EditorAction, Message};
use crate::features::daemons::{
    runtime_choices, DaemonDetails, Draft, DraftRuntime, FormError, Msg as DaemonsMsg, Section,
    State, ViewModel,
};
use crate::features::daemons_kube::{AllowedKind, ContextList};
use crate::features::settings::DaemonDraft;
use crate::features::window::FieldId;
use crate::kube::live::ContextEntry;
use crate::ui::focus::TrackFocus;
use crate::ui::material::{
    Button, Checkbox, FieldGroup, HostPicker, KeyValueList, RequiredLegend, Select, StageProgress,
    Tag, TextField, TolerationList,
};
use crate::ui::settings::container_fields::{self, ContainerView, FieldErrors};
use crate::ui::settings::{caution, group, name_of, note, page, section_body, Named, SECTION_GAP};
use iced::widget::{column, row};
use iced::{Element, Length};
use micold_core::daemons::{DaemonId, DaemonState, DaemonView, ATTEMPTS_STOPPED};
use micold_core::sandbox::placement::PlacementKind;
use micold_core::tokens::{spacing, Roles};

const RUNTIMES: &[Named<DraftRuntime>] = &[
    Named(DraftRuntime::Container, "Container"),
    Named(DraftRuntime::Host, "Host"),
    Named(DraftRuntime::Ssh, "Remote over SSH"),
    Named(DraftRuntime::Kubernetes, "Kubernetes"),
];

/// The runtimes offered for `draft`: [`RUNTIMES`] without what this build cannot run (FR-024).
fn offered(draft: &Draft) -> &'static [Named<DraftRuntime>] {
    let n = runtime_choices(draft).len();
    &RUNTIMES[..n]
}

/// The section's explanation when no daemon is configured.
pub const EMPTY_TITLE: &str = "No daemons are set up";
/// What a daemon is, and what happens until one is added.
pub const EMPTY_BODY: &str = "A daemon runs your sessions and terminals, on this computer or in a \
     container. Until one is added, worktrees cannot start sessions. Add one to get going.";

/// The runtime label of a Kubernetes daemon's row.
const KUBERNETES_LABEL: &str = "Kubernetes";

fn state_tag<'a>(state: &DaemonState, roles: Roles) -> Element<'a, Message> {
    let (text, accent) = match state {
        DaemonState::Connected => ("Connected", roles.primary),
        DaemonState::Starting => ("Starting", roles.secondary),
        DaemonState::Stopped => ("Stopped", roles.secondary),
        DaemonState::Reconnecting { .. } => ("Reconnecting", roles.secondary),
        DaemonState::Unreachable { .. } => ("Unreachable", roles.error),
        DaemonState::VersionMismatch { .. } => ("Version mismatch", roles.error),
    };
    Tag::new(text, accent).into()
}

/// The sentence under a row that is not plainly fine.
fn state_note(view: &DaemonView) -> Option<String> {
    match &view.state {
        DaemonState::Reconnecting { reason } => {
            Some(format!("reconnecting to {}: {reason}", view.endpoint))
        }
        DaemonState::Stopped => view.stopped_reason.clone(),
        DaemonState::Unreachable { reason } if reason == ATTEMPTS_STOPPED => Some(format!(
            "not connected to {}: {ATTEMPTS_STOPPED}",
            view.endpoint
        )),
        DaemonState::Unreachable { reason } => {
            Some(format!("Daemon {} unreachable: {reason}", view.name))
        }
        DaemonState::VersionMismatch { .. } if view.explanation.is_some() => {
            view.explanation.clone()
        }
        DaemonState::VersionMismatch { client, daemon } => Some(format!(
            "Daemon {} version mismatch: client {client}, daemon {daemon}. Update the daemon's \
             image or this application, then reconnect.",
            view.name
        )),
        _ => None,
    }
}

fn bound_line(view: &DaemonView) -> String {
    if view.bound.is_empty() {
        return "No worktrees are bound to it".to_string();
    }
    let names: Vec<String> = view
        .bound
        .iter()
        .map(|loc| {
            let project = loc
                .project
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_else(|| loc.project.display().to_string());
            if loc.worktree.is_empty() {
                format!("{project} (Default)")
            } else {
                format!("{project}/{}", loc.worktree)
            }
        })
        .collect();
    format!("Worktrees: {}", names.join(", "))
}

/// Stop, which a container still on the released image offers only with no session running (FR-017).
fn stop_button<'a>(view: &DaemonView, roles: Roles) -> Button<'a, Message> {
    let stop = Button::text("Stop", roles);
    if view.stop_blocked {
        stop
    } else {
        stop.on_press(Message::Daemons(DaemonsMsg::Stopped(view.id)))
    }
}

fn actions<'a>(view: &DaemonView, open: bool, kube: bool, roles: Roles) -> Element<'a, Message> {
    let id = view.id;
    let mut buttons = row![].spacing(spacing::XS);
    match view.state {
        DaemonState::Stopped => {
            buttons = buttons.push(
                Button::text("Start", roles).on_press(Message::Daemons(DaemonsMsg::Started(id))),
            );
        }
        // Stop the attempts, or skip the wait and try now.
        DaemonState::Reconnecting { .. } => {
            buttons = buttons.push(
                Button::text("Stop trying", roles)
                    .on_press(Message::Daemons(DaemonsMsg::GaveUp(id))),
            );
            buttons = buttons.push(
                Button::text("Reconnect", roles)
                    .on_press(Message::Daemons(DaemonsMsg::Reconnected(id))),
            );
            buttons = buttons.push(stop_button(view, roles));
        }
        DaemonState::Unreachable { .. } | DaemonState::VersionMismatch { .. } => {
            if view.replace_offered {
                buttons = buttons.push(
                    Button::text("Replace daemon", roles)
                        .on_press(Message::Daemons(DaemonsMsg::ReplaceRequested(id))),
                );
            }
            buttons = buttons.push(
                Button::text("Reconnect", roles)
                    .on_press(Message::Daemons(DaemonsMsg::Reconnected(id))),
            );
            buttons = buttons.push(stop_button(view, roles));
        }
        DaemonState::Starting | DaemonState::Connected => {
            if kube && view.state == DaemonState::Starting {
                buttons = buttons.push(
                    Button::text("Cancel", roles)
                        .on_press(Message::Daemons(DaemonsMsg::StartCancelled(id))),
                );
            }
            buttons = buttons.push(stop_button(view, roles));
        }
    }
    if kube {
        buttons = buttons.push(
            Button::text("Logs", roles).on_press(Message::Daemons(DaemonsMsg::LogsRequested(id))),
        );
    }
    buttons = buttons
        .push(Button::text("Edit", roles).on_press(Message::Daemons(DaemonsMsg::EditOpened(id))))
        .push(
            Button::text(if open { "Hide details" } else { "Details" }, roles)
                .on_press(Message::Daemons(DaemonsMsg::DetailsToggled(id))),
        )
        .push(
            Button::text("Remove", roles)
                .on_press(Message::Daemons(DaemonsMsg::RemovalRequested(id))),
        );
    if kube {
        buttons = buttons.push(
            Button::text("Forget locally", roles)
                .on_press(Message::Daemons(DaemonsMsg::ForgetLocallyRequested(id))),
        );
    }
    buttons.into()
}

fn details_block<'a>(details: DaemonDetails, roles: Roles) -> Element<'a, Message> {
    let mut block = column![group("Details", roles)].spacing(spacing::XS);
    if let Some(reason) = details.runtime_missing {
        block = block.push(caution(reason, roles));
    }
    if details.errors.is_empty() {
        block = block.push(note("No connection errors", roles));
    }
    for error in details.errors {
        block = block.push(caution(error.reason, roles));
    }
    block = block.push(group("Log", roles));
    if details.log.is_empty() {
        block = block.push(note("Nothing logged yet", roles));
    }
    for line in details.log {
        block = block.push(note(line, roles));
    }
    block.into()
}

fn daemon_row<'a>(
    view: DaemonView,
    state: &'a State,
    focused: Option<FieldId>,
    roles: Roles,
) -> Element<'a, Message> {
    let open = state.details_shown.contains(&view.id);
    let header = row![
        crate::ui::material::Text::new(
            view.name.clone(),
            crate::ui::material::TypeRole::Label,
            roles
        ),
        Tag::new(view.runtime_label.clone(), roles.tertiary),
        state_tag(&view.state, roles),
    ]
    .spacing(spacing::SM);
    let version = view
        .version
        .clone()
        .map(|v| format!(" - {v}"))
        .unwrap_or_default();
    let mut body = column![
        header,
        note(format!("{}{version}", view.endpoint), roles),
        note(bound_line(&view), roles),
    ]
    .spacing(spacing::XS)
    .width(Length::Fill);
    if let Some(activity) = &view.activity {
        body = body.push(StageProgress::new(activity.clone(), roles));
    }
    if let Some(sentence) = state_note(&view) {
        body = body.push(caution(sentence, roles));
    }
    for warning in &view.install_warnings {
        body = body.push(caution(warning.clone(), roles));
    }
    if let Some(sentence) = &view.notice {
        body = body.push(note(sentence.clone(), roles));
    }
    if view.runtime_label == micold_core::daemons::SSH_RUNTIME_LABEL {
        body = body.push(repository_step(&view, state, focused, roles));
    }
    let kube = view.runtime_label == KUBERNETES_LABEL;
    if kube {
        let details = crate::features::daemons::details(state, view.id);
        if details.applies_on_next_start {
            body = body.push(note(crate::features::daemons::APPLIES_ON_NEXT_START, roles));
        }
        match details.pod_log {
            Some(Ok(log)) => {
                body = body.push(group("Pod log", roles)).push(note(log, roles));
            }
            Some(Err(why)) => body = body.push(caution(why, roles)),
            None => {}
        }
    }
    body = body.push(actions(&view, open, kube, roles));
    if open {
        body = body.push(details_block(
            crate::features::daemons::details(state, view.id),
            roles,
        ));
    }
    body.into()
}

/// The repository step under a remote daemon (feature 489, FR-012): the path on the host and the
/// action that adds it, which is available once the daemon is connected.
fn repository_step<'a>(
    view: &DaemonView,
    state: &'a State,
    focused: Option<FieldId>,
    roles: Roles,
) -> Element<'a, Message> {
    let id = view.id;
    let typed = state.repositories.get(&id).map_or("", String::as_str);
    let error = state
        .repository_error
        .as_ref()
        .filter(|(for_id, _)| *for_id == id)
        .map(|(_, message)| message.clone());
    let repository = state.repository_field(id);
    let repository_state = state
        .repository_marks
        .state_in(std::slice::from_ref(&repository), repository.0);
    let mut add = Button::text("Add repository", roles);
    if view.state == DaemonState::Connected {
        add = add.on_press(Message::Daemons(DaemonsMsg::RepositoryAdded(id)));
    }
    let step = row![
        TextField::new("/path/to/repository on the host", typed, roles)
            .label("Repository on the host")
            .required(repository_state)
            .error(error)
            .track_focus(FieldId::DaemonRepository(id), focused)
            .on_input(move |v| Message::Daemons(DaemonsMsg::RepositoryEdited(id, v))),
        add,
    ]
    .spacing(spacing::SM);
    // The marker's explanation, once, above the step's field (feature 937, FR-004).
    column![]
        .spacing(spacing::SM)
        .extend(RequiredLegend::for_fields(
            std::slice::from_ref(&repository),
            roles,
        ))
        .push(step)
        .into()
}

fn error_of(state: &State, field: &str) -> Option<String> {
    state
        .form_error
        .as_ref()
        .filter(|e: &&FormError| e.field == field)
        .map(|e| e.message.clone())
}

/// What the machine says about the running container, which only the legacy default daemon is.
#[derive(Debug, Clone, Default)]
pub struct MachineFacts<'a> {
    /// The settings draft's seed of the sandbox's probed capabilities and unshared sign-in.
    pub draft: Option<&'a DaemonDraft>,
    /// The daemon those facts describe.
    pub legacy_default: Option<DaemonId>,
    /// What a session would not find in the running image (FR-023b), shown under *Image reference*.
    pub missing_cli: Option<String>,
}

fn form<'a>(
    draft: &'a Draft,
    state: &'a State,
    facts: MachineFacts<'a>,
    focused: Option<FieldId>,
    roles: Roles,
) -> Element<'a, Message> {
    let title = match draft.editing {
        Some(_) => "Edit daemon",
        None => "Add a daemon",
    };
    let required = draft.required_fields();
    let marker = |id| draft.required.state_in(&required, id);
    // The marker's explanation, once for the whole form, between its title and its first field
    // (feature 937, FR-004); the container and SSH fields below add none.
    let heading = column![group(title, roles)]
        .spacing(spacing::SM)
        .extend(RequiredLegend::for_fields(&required, roles));
    let mut fields = column![
        heading,
        TextField::new("", &draft.name, roles)
            .label("Name")
            .required(marker(FieldId::DaemonName))
            .track_focus(FieldId::DaemonName, focused)
            .error(error_of(state, "name"))
            .on_input(|v| Message::Daemons(DaemonsMsg::NameEdited(v))),
        Select::new(
            offered(draft),
            Some(Named(draft.runtime, name_of(RUNTIMES, draft.runtime))),
            |chosen: Named<DraftRuntime>| Message::Daemons(DaemonsMsg::RuntimeChosen(chosen.0)),
            roles,
        )
        .label("Runs in"),
    ]
    .spacing(SECTION_GAP)
    .width(Length::Fill);
    if let Some(message) = error_of(state, "runtime") {
        fields = fields.push(caution(message, roles));
    }
    let open = |section| state.section_open(section);
    let sections: Vec<Element<'a, Message>> = match draft.runtime {
        DraftRuntime::Container => {
            let connection = vec![
                TextField::new("", &draft.container_name, roles)
                    .label("Container name")
                    .required(marker(FieldId::DaemonContainerName))
                    .track_focus(FieldId::DaemonContainerName, focused)
                    .error(error_of(state, "container_name"))
                    .on_input(|v| Message::Daemons(DaemonsMsg::ContainerNameEdited(v)))
                    .into(),
                TextField::new("A free port is chosen if empty", &draft.port, roles)
                    .label("Port")
                    .track_focus(FieldId::DaemonPort, focused)
                    .error(error_of(state, "port"))
                    .on_input(|v| Message::Daemons(DaemonsMsg::PortEdited(v)))
                    .into(),
                auto_start(draft, focused, roles),
            ];
            let described = draft.editing.is_some() && draft.editing == facts.legacy_default;
            let machine = facts.draft.filter(|_| described);
            let view = ContainerView {
                profile: &draft.profile,
                image_path: &draft.image_path,
                image_reference: &draft.image_reference,
                cpus: &draft.cpus,
                memory_mib: &draft.memory_mib,
                pids: &draft.pids,
                storage_mib: &draft.storage_mib,
                capabilities: machine.and_then(|d| d.capabilities.as_ref()),
                unshared_sign_in: machine.and_then(|d| d.unshared_sign_in.as_deref()),
                placement: PlacementKind::LocalSandbox,
            };
            let errors = FieldErrors {
                reference_required: marker(FieldId::DaemonImage),
                path_required: marker(FieldId::DaemonImagePath),
                reference: error_of(state, "image"),
                path: error_of(state, "image_path"),
                limits: [
                    error_of(state, "cpus"),
                    error_of(state, "memory"),
                    error_of(state, "pids"),
                    error_of(state, "storage"),
                ],
            };
            container_fields::sections(
                &view,
                connection,
                errors,
                machine.and(facts.missing_cli.clone()),
                focused,
                state,
                roles,
            )
        }
        DraftRuntime::Kubernetes => kube_sections(draft, state, &marker, focused, roles),
        DraftRuntime::Host => vec![
            container_fields::accordion(
                Section::Connection,
                section_body(None, vec![auto_start(draft, focused, roles)], roles),
                open(Section::Connection),
                draft,
                roles,
            ),
            // A host daemon keeps no container profile: the groups stay out rather than take
            // edits that would be dropped, and the one thing that applies to it is said
            // (FR-014d).
            container_fields::accordion(
                Section::Sessions,
                section_body(
                    None,
                    vec![container_fields::survival_note(
                        PlacementKind::HostProcess,
                        roles,
                    )],
                    roles,
                ),
                open(Section::Sessions),
                draft,
                roles,
            ),
        ],
        DraftRuntime::Ssh => {
            let mut connection: Vec<Element<'a, Message>> = Vec::new();
            if !state.ssh_hosts.is_empty() {
                connection.push(
                    HostPicker::new(
                        &state.ssh_hosts,
                        &draft.ssh_host,
                        |host| Message::Daemons(DaemonsMsg::SshHostChosen(host)),
                        roles,
                    )
                    .into(),
                );
            }
            connection.push(
                TextField::new("", &draft.ssh_host, roles)
                    .label("Host")
                    .required(marker(FieldId::DaemonSshHost))
                    .track_focus(FieldId::DaemonSshHost, focused)
                    .error(error_of(state, "host"))
                    .on_input(|v| Message::Daemons(DaemonsMsg::SshHostEdited(v)))
                    .into(),
            );
            connection.push(
                TextField::new("", &draft.ssh_user, roles)
                    .label("User")
                    .track_focus(FieldId::DaemonSshUser, focused)
                    .error(error_of(state, "user"))
                    .on_input(|v| Message::Daemons(DaemonsMsg::SshUserEdited(v)))
                    .into(),
            );
            connection.push(
                TextField::new("", &draft.ssh_port, roles)
                    .label("Port")
                    .track_focus(FieldId::DaemonSshPort, focused)
                    .error(error_of(state, "port"))
                    .on_input(|v| Message::Daemons(DaemonsMsg::SshPortEdited(v)))
                    .into(),
            );
            connection.push(
                TextField::new("", &draft.ssh_key_file, roles)
                    .label("Key file")
                    .track_focus(FieldId::DaemonSshKeyFile, focused)
                    .error(error_of(state, "key_file"))
                    .on_input(|v| Message::Daemons(DaemonsMsg::SshKeyFileEdited(v)))
                    .into(),
            );
            connection.push(auto_start(draft, focused, roles));
            vec![container_fields::accordion(
                Section::Connection,
                section_body(None, connection, roles),
                open(Section::Connection),
                draft,
                roles,
            )]
        }
    };
    for control in sections {
        fields = fields.push(control);
    }
    fields
        .push(
            row![
                Button::filled("Save daemon", roles).on_press(Message::Daemons(DaemonsMsg::Saved)),
                Button::outlined("Cancel", roles)
                    .on_press(Message::Daemons(DaemonsMsg::DraftDiscarded)),
            ]
            .spacing(spacing::SM),
        )
        .into()
}

/// The connect-at-start checkbox, which closes the `Connection` section of every kind.
fn auto_start<'a>(draft: &Draft, focused: Option<FieldId>, roles: Roles) -> Element<'a, Message> {
    Checkbox::new(
        "Connect when the application starts",
        draft.auto_start,
        roles,
    )
    .track_focus(FieldId::DaemonAutoStart, focused)
    .on_toggle(|v| Message::Daemons(DaemonsMsg::AutoStartToggled(v)))
    .into()
}

/// The Kubernetes sections (feature 686, M4a and M4b; grouped by feature 890, FR-002): connection,
/// image, credentials, resources, then placement and storage.
fn kube_sections<'a>(
    draft: &'a Draft,
    state: &'a State,
    marker: &dyn Fn(FieldId) -> micold_core::required::FieldRequirement,
    focused: Option<FieldId>,
    roles: Roles,
) -> Vec<Element<'a, Message>> {
    let kube = &draft.kube;
    let open = |section| state.section_open(section);
    let mut connection: Vec<Element<'a, Message>> = Vec::new();
    if let ContextList::Found(list) = &state.contexts {
        if !list.is_empty() {
            let chosen = list.iter().find(|c| c.name == kube.context).cloned();
            connection.push(
                Select::new(
                    list.as_slice(),
                    chosen,
                    |c: ContextEntry| Message::Daemons(DaemonsMsg::ContextChosen(c.name)),
                    roles,
                )
                .label("Context from your kubeconfig")
                .placeholder("Choose a context")
                .into(),
            );
        }
    }
    connection.push(
        TextField::new("", &kube.context, roles)
            .label("Context")
            .supporting("A context from your kubeconfig; cluster credentials stay there")
            .required(marker(FieldId::DaemonKubeContext))
            .track_focus(FieldId::DaemonKubeContext, focused)
            .error(error_of(state, "context"))
            .on_input(|v| Message::Daemons(DaemonsMsg::ContextChosen(v)))
            .into(),
    );
    if let ContextList::Failed(why) = &state.contexts {
        connection.push(caution(why.clone(), roles));
    }
    connection.push(
        TextField::new("", &kube.namespace, roles)
            .label("Namespace")
            .required(marker(FieldId::DaemonKubeNamespace))
            .track_focus(FieldId::DaemonKubeNamespace, focused)
            .error(error_of(state, "namespace"))
            .on_input(|v| Message::Daemons(DaemonsMsg::NamespaceEdited(v)))
            .into(),
    );
    connection.push(auto_start(draft, focused, roles));

    let mut image: Vec<Element<'a, Message>> = vec![TextField::new("", &kube.image, roles)
        .label("Image")
        .supporting("Empty uses the default SSH image")
        .track_focus(FieldId::DaemonKubeImage, focused)
        .error(error_of(state, "image"))
        .on_input(|v| Message::Daemons(DaemonsMsg::KubeImageEdited(v)))
        .into()];
    let mut clis = FieldGroup::new("AI CLIs installed in the pod", roles)
        .note("Installed when the pod starts.");
    for cli in micold_core::session::AiCli::ALL {
        clis = clis.push(
            Checkbox::new(
                cli.provider().display_name(),
                kube.providers.contains(&cli),
                roles,
            )
            .track_focus(FieldId::DaemonKubeProvider(cli), focused)
            .on_toggle(move |on| Message::Daemons(DaemonsMsg::ProviderToggled(cli, on))),
        );
    }
    image.push(clis.into());

    let mut allowed = FieldGroup::new("Allowed credentials", roles)
        .note("Nothing reaches the pod unless it is ticked here.");
    for (place, kind) in AllowedKind::all().into_iter().enumerate() {
        allowed = allowed.push(
            Checkbox::new(kind.label(), kube.allowed.contains(&kind), roles)
                .track_focus(FieldId::DaemonKubeAllowed(place as u8), focused)
                .on_toggle(move |on| Message::Daemons(DaemonsMsg::AllowedToggled(kind, on))),
        );
        let file = match kind {
            AllowedKind::GitSshKey => Some((
                "SSH key file",
                &kube.ssh_key_file,
                FieldId::DaemonKubeSshKeyFile,
                "ssh_key_file",
            )),
            AllowedKind::RegistryPull => Some((
                "Registry credentials file",
                &kube.pull_file,
                FieldId::DaemonKubePullFile,
                "pull_file",
            )),
            _ => None,
        };
        if let Some((label, value, id, name)) = file.filter(|_| kube.allowed.contains(&kind)) {
            let on_input: fn(String) -> Message = if kind == AllowedKind::GitSshKey {
                |v| Message::Daemons(DaemonsMsg::KubeSshKeyEdited(v))
            } else {
                |v| Message::Daemons(DaemonsMsg::PullFileEdited(v))
            };
            allowed = allowed.push(
                TextField::new("", value, roles)
                    .label(label)
                    .track_focus(id, focused)
                    .error(error_of(state, name))
                    .on_input(on_input),
            );
        }
    }
    if let Some(message) = error_of(state, "allowed") {
        allowed = allowed.push(caution(message, roles));
    }
    let credentials: Vec<Element<'a, Message>> = vec![allowed.into()];

    let errors = [
        error_of(state, "cpu_request"),
        error_of(state, "cpu_limit"),
        error_of(state, "memory_request"),
        error_of(state, "memory_limit"),
    ];
    let resources: Vec<Element<'a, Message>> = container_fields::kube_resources(kube)
        .into_iter()
        .zip(errors)
        .map(|(control, error)| container_fields::limit_field(control, error, focused, roles))
        .collect();

    let mut selector = FieldGroup::new("Node selector", roles)
        .note("One key=value per line")
        .push(
            KeyValueList::new(kube.node_selector_editor.content(), roles)
                .on_action(|a| Message::Daemons(DaemonsMsg::NodeSelectorAction(EditorAction(a)))),
        );
    if let Some(message) = error_of(state, "node_selector") {
        selector = selector.push(caution(message, roles));
    }
    let mut tolerations = FieldGroup::new("Tolerations", roles)
        .note("One per line")
        .push(
            TolerationList::new(kube.tolerations_editor.content(), roles)
                .on_action(|a| Message::Daemons(DaemonsMsg::TolerationsAction(EditorAction(a)))),
        );
    if let Some(message) = error_of(state, "tolerations") {
        tolerations = tolerations.push(caution(message, roles));
    }
    let mut placement: Vec<Element<'a, Message>> = vec![selector.into(), tolerations.into()];
    placement.push(
        TextField::new("20Gi", &kube.volume_size, roles)
            .label("Volume size")
            .supporting(
                draft
                    .volume_notice()
                    .unwrap_or("In Kubernetes form, e.g. 20Gi"),
            )
            .track_focus(FieldId::DaemonKubeVolumeSize, focused)
            .error(error_of(state, "volume_size"))
            .on_input(|v| Message::Daemons(DaemonsMsg::VolumeSizeEdited(v)))
            .into(),
    );
    placement.push(
        TextField::new("", &kube.storage_class, roles)
            .label("Storage class")
            .supporting("Empty uses the cluster default")
            .track_focus(FieldId::DaemonKubeStorageClass, focused)
            .error(error_of(state, "storage_class"))
            .on_input(|v| Message::Daemons(DaemonsMsg::StorageClassEdited(v)))
            .into(),
    );

    [
        (Section::Connection, connection),
        (Section::Image, image),
        (Section::Credentials, credentials),
        (Section::Resources, resources),
        (Section::Placement, placement),
    ]
    .into_iter()
    .map(|(section, controls)| {
        let body = section_body(None, controls, roles);
        container_fields::accordion(section, body, open(section), draft, roles)
    })
    .collect()
}

/// The empty state: what a daemon is, and the way to add one.
pub fn empty_state<'a>(roles: Roles) -> Element<'a, Message> {
    column![
        crate::ui::material::Text::new(EMPTY_TITLE, crate::ui::material::TypeRole::Label, roles),
        note(EMPTY_BODY, roles),
        Button::filled("Add daemon", roles).on_press(Message::Daemons(DaemonsMsg::AddOpened)),
    ]
    .spacing(spacing::SM)
    .width(Length::Fill)
    .into()
}

/// The Daemons page.
pub fn view<'a>(
    state: &'a State,
    list: ViewModel,
    facts: MachineFacts<'a>,
    focused: Option<FieldId>,
    roles: Roles,
) -> Element<'a, Message> {
    let mut controls: Vec<Element<'a, Message>> = Vec::new();
    if list.empty && state.draft.is_none() {
        controls.push(empty_state(roles));
    }
    for view in list.rows {
        controls.push(daemon_row(view, state, focused, roles));
    }
    match &state.draft {
        Some(draft) => controls.push(form(draft, state, facts, focused, roles)),
        None if !list.empty => controls.push(
            Button::outlined("Add daemon", roles)
                .on_press(Message::Daemons(DaemonsMsg::AddOpened))
                .into(),
        ),
        None => {}
    }
    page(
        "Daemons",
        "The daemons your worktrees run on. Removing one keeps its worktrees, which then show \
         \"no daemon\" until you bind them again; nothing on disk and no container is touched.",
        controls,
        roles,
    )
}

/// Which daemon a row's actions address, for tests that press them.
#[allow(dead_code)]
pub fn row_id(view: &DaemonView) -> DaemonId {
    view.id
}
