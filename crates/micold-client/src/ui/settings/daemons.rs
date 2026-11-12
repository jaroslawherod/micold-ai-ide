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
    container_name_hint, runtime_choices, DaemonDetails, Draft, DraftRuntime, ExtraPath, FormError,
    Msg as DaemonsMsg, Opener, RowAction, Section, State, ViewModel,
};
use crate::features::daemons_kube::{AllowedKind, ContextList};
use crate::features::daemons_start::StartSource;
use crate::features::settings::DaemonDraft;
use crate::features::window::FieldId;
use crate::kube::live::ContextEntry;
use crate::ui::focus::TrackFocus;
use crate::ui::material::{
    Button, Checkbox, FieldGroup, HostPicker, KeyValueList, RequiredLegend, Select, StageProgress,
    Tag, TextArea, TextField, TolerationList,
};
use crate::ui::settings::container_fields::{self, ContainerView, FieldErrors};
use crate::ui::settings::{caution, group, name_of, note, page, section_body, Named, SECTION_GAP};
use iced::widget::{column, row};
use iced::{Element, Length};
use micold_core::daemon_config::{OnFailure, RunPolicy};
use micold_core::daemons::{DaemonId, DaemonState, DaemonView, ATTEMPTS_STOPPED};
use micold_core::sandbox::placement::PlacementKind;
use micold_core::tokens::{spacing, Roles};

const START_SOURCES: &[Named<StartSource>] = &[
    Named(StartSource::None, "No start script"),
    Named(StartSource::File, "A script file"),
    Named(StartSource::Inline, "Written here"),
];
const ON_FAILURES: &[Named<OnFailure>] = &[
    Named(OnFailure::Refuse, "Refuse to serve"),
    Named(OnFailure::Warn, "Warn and serve"),
];
const RUN_POLICIES: &[Named<RunPolicy>] = &[
    Named(RunPolicy::EveryStart, "At every start"),
    Named(RunPolicy::OncePerContainer, "Once per container"),
];

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

/// The widget id of the button that opened the daemon dialog, so focus can return to it (FR-009).
pub fn opener_id(opener: Opener) -> iced::advanced::widget::Id {
    iced::advanced::widget::Id::from(match opener {
        Opener::Add => "daemons-add".to_string(),
        Opener::Row(id, RowAction::Edit) => format!("daemons-edit-{id:?}"),
        Opener::Row(id, RowAction::Details) => format!("daemons-details-{id:?}"),
    })
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

fn actions<'a>(view: &DaemonView, kube: bool, roles: Roles) -> Element<'a, Message> {
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
        .push(
            Button::text("Edit", roles)
                .id(opener_id(Opener::Row(id, RowAction::Edit)))
                .on_press(Message::Daemons(DaemonsMsg::EditOpened(id))),
        )
        .push(
            Button::text("Details", roles)
                .id(opener_id(Opener::Row(id, RowAction::Details)))
                .on_press(Message::Daemons(DaemonsMsg::DetailsOpened(id))),
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

pub(crate) fn details_block<'a>(details: DaemonDetails, roles: Roles) -> Element<'a, Message> {
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
    // A changed start script runs only at the next start (#1533, US5.2).
    if view.startup_restart {
        body = body.push(note(
            crate::features::daemons_start::RESTART_TO_APPLY,
            roles,
        ));
    }
    // Errors found in the daemon's rendered configuration file (FR-013f).
    for error in &view.config_errors {
        body = body.push(caution(error.clone(), roles));
    }
    // The git credential of each project of a sandboxed daemon (#1002, FR-013).
    for line in &view.credential_lines {
        body = body.push(if line.warning {
            caution(line.text.clone(), roles)
        } else {
            note(line.text.clone(), roles)
        });
    }
    if let Some(sentence) = &view.notice {
        body = body.push(note(sentence.clone(), roles));
        if view.notice_dismissable {
            body = body.push(
                Button::text("Dismiss", roles)
                    .on_press(Message::Daemons(DaemonsMsg::DismissNotice(view.id))),
            );
        }
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
    body = body.push(actions(&view, kube, roles));
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
    /// Whether the Start script group is shown, in place of what the kind delivers
    /// ([`micold_core::daemons::start_script_delivered`]): the showcase and the geometry gates
    /// force it on while no kind delivers (#1533).
    pub start_script: Option<bool>,
}

/// The daemon form: the dialog's body (feature 1092). Its Save daemon / Cancel live in the dialog.
pub(crate) fn form<'a>(
    draft: &'a Draft,
    state: &'a State,
    facts: MachineFacts<'a>,
    focused: Option<FieldId>,
    roles: Roles,
) -> Element<'a, Message> {
    let required = draft.required_fields();
    let marker = |id| draft.required.state_in(&required, id);
    // The marker's explanation, once for the whole form, above its first field (feature 937,
    // FR-004); the container and SSH fields below add none. The dialog's own title names the form.
    let legend = RequiredLegend::for_fields(&required, roles);
    let mut fields = column![]
        .spacing(SECTION_GAP)
        .width(Length::Fill)
        .extend(legend)
        .push(
            TextField::new("", &draft.name, roles)
                .label("Name")
                .required(marker(FieldId::DaemonName))
                .track_focus(FieldId::DaemonName, focused)
                .error(error_of(state, "name"))
                .on_input(|v| Message::Daemons(DaemonsMsg::NameEdited(v))),
        )
        .push(
            Select::new(
                offered(draft),
                Some(Named(draft.runtime, name_of(RUNTIMES, draft.runtime))),
                |chosen: Named<DraftRuntime>| Message::Daemons(DaemonsMsg::RuntimeChosen(chosen.0)),
                roles,
            )
            .label("Runs in"),
        );
    if let Some(message) = error_of(state, "runtime") {
        fields = fields.push(caution(message, roles));
    }
    // Only on an existing daemon: three more fields push the Kubernetes add form past one window
    // height (daemon_form_fits), and a new daemon starts from the defaults.
    if draft.editing.is_some() {
        fields = fields
            .push(
                TextField::new("Default", &draft.data_dir, roles)
                    .label("Data directory")
                    .track_focus(FieldId::DaemonDataDir, focused)
                    .supporting(
                        "Absolute path on the daemon's machine; written to its configuration file",
                    )
                    .on_input(|v| Message::Daemons(DaemonsMsg::DataDirEdited(v))),
            )
            .push(
                TextField::new("Not set", &draft.clone_root, roles)
                    .label("Clone root")
                    .track_focus(FieldId::DaemonCloneRoot, focused)
                    .supporting("Absolute path projects with a remote are cloned under")
                    .on_input(|v| Message::Daemons(DaemonsMsg::CloneRootEdited(v))),
            )
            .push(
                TextField::new("Default", &draft.client_token_file, roles)
                    .label("Client token file")
                    .track_focus(FieldId::DaemonClientTokenFile, focused)
                    .supporting("Absolute path of the file holding the client token")
                    .on_input(|v| Message::Daemons(DaemonsMsg::ClientTokenFileEdited(v))),
            );
        for (which, label, hint, id) in [
            (
                ExtraPath::SshKey,
                "SSH key file",
                "Private key sessions use for git over SSH",
                FieldId::DaemonSessionSshKey,
            ),
            (
                ExtraPath::KnownHosts,
                "Known hosts file",
                "known_hosts for the SSH key",
                FieldId::DaemonSessionKnownHosts,
            ),
            (
                ExtraPath::SigningKey,
                "Signing key file",
                "Key that signs commits in sessions",
                FieldId::DaemonSessionSigningKey,
            ),
            (
                ExtraPath::CaFile,
                "CA file",
                "CA bundle sessions and the daemon trust",
                FieldId::DaemonSessionCaFile,
            ),
        ] {
            fields = fields.push(
                TextField::new("Not set", &draft.extra_paths[which as usize], roles)
                    .label(label)
                    .track_focus(id, focused)
                    .supporting(hint)
                    .on_input(move |v| Message::Daemons(DaemonsMsg::ExtraPathEdited(which, v))),
            );
        }
    }
    let open = |section| state.section_open(section);
    let sections: Vec<Element<'a, Message>> = match draft.runtime {
        DraftRuntime::Container => {
            let connection = vec![
                TextField::new(container_name_hint(draft), &draft.container_name, roles)
                    .label("Container name")
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
    // Its own accordion, closed by default, so this paints only its header on an add form or an
    // existing daemon alike (#1532, FR-013d; `Section::of` lists it last, before Start script,
    // for every kind).
    fields = fields.push(container_fields::accordion(
        Section::AiCli,
        section_body(None, ai_cli_controls(draft, focused, roles), roles),
        open(Section::AiCli),
        draft,
        roles,
    ));
    let kind = crate::features::daemons_start::kind_of(draft.runtime);
    if facts
        .start_script
        .unwrap_or_else(|| micold_core::daemons::start_script_delivered(kind))
    {
        fields = fields.push(container_fields::accordion(
            Section::StartScript,
            section_body(
                None,
                start_script_controls(draft, state, focused, roles),
                roles,
            ),
            open(Section::StartScript),
            draft,
            roles,
        ));
    }
    fields.into()
}

/// The `[[ai.cli]]` section's controls (#1532, FR-013d): one group per supported CLI, in
/// [`micold_core::daemon_config::CliId::ALL`] order — enabled, version, args, model, base URL,
/// sign-in file, API key file and settings file. `model` and `api_base_url` are Claude Code's and
/// Codex's own fields (FR-013d); left visible but unused for the others costs nothing a mutant
/// would catch, so they stay off rather than branch per CLI here.
fn ai_cli_controls<'a>(
    draft: &'a Draft,
    focused: Option<FieldId>,
    roles: Roles,
) -> Vec<Element<'a, Message>> {
    let mut controls = Vec::with_capacity(micold_core::daemon_config::CliId::ALL.len() * 2);
    for id in micold_core::daemon_config::CliId::ALL {
        let index = id as usize;
        let entry = &draft.ai_cli[index];
        controls.push(group(id.ai_cli().provider().display_name(), roles));
        controls.push(
            Checkbox::new("Enabled", entry.enabled, roles)
                .track_focus(FieldId::DaemonAiCliEnabled(id), focused)
                .on_toggle(move |v| Message::Daemons(DaemonsMsg::AiCliEnabledToggled(id, v)))
                .into(),
        );
        for (label, value, field) in [
            (
                "Version",
                &entry.version,
                crate::features::daemons::AiCliField::Version,
            ),
            (
                "Launch arguments",
                &entry.args,
                crate::features::daemons::AiCliField::Args,
            ),
            (
                "Model",
                &entry.model,
                crate::features::daemons::AiCliField::Model,
            ),
            (
                "Base URL",
                &entry.api_base_url,
                crate::features::daemons::AiCliField::ApiBaseUrl,
            ),
            (
                "Sign-in file",
                &entry.sign_in_file,
                crate::features::daemons::AiCliField::SignInFile,
            ),
            (
                "API key file",
                &entry.api_key_file,
                crate::features::daemons::AiCliField::ApiKeyFile,
            ),
            (
                "Settings file",
                &entry.settings_file,
                crate::features::daemons::AiCliField::SettingsFile,
            ),
        ] {
            controls.push(
                TextField::new("Not set", value, roles)
                    .label(label)
                    .track_focus(FieldId::DaemonAiCliField(id, field), focused)
                    .on_input(move |v| Message::Daemons(DaemonsMsg::AiCliFieldEdited(id, field, v)))
                    .into(),
            );
        }
    }
    controls
}

/// The Start script group's controls (#1533, FR-010): the script as a path or inline text, then
/// shell, timeout, failure and run policies, the secret file and the export setting.
fn start_script_controls<'a>(
    draft: &'a Draft,
    state: &'a State,
    focused: Option<FieldId>,
    roles: Roles,
) -> Vec<Element<'a, Message>> {
    let start = &draft.start;
    let mut controls: Vec<Element<'a, Message>> = vec![Select::new(
        START_SOURCES,
        Some(Named(start.source, name_of(START_SOURCES, start.source))),
        |chosen: Named<StartSource>| Message::Daemons(DaemonsMsg::StartSourceChosen(chosen.0)),
        roles,
    )
    .label("Script")
    .into()];
    match start.source {
        StartSource::None => return controls,
        StartSource::File => controls.push(
            TextField::new("Path on this computer", &start.file, roles)
                .label("Script file")
                .track_focus(FieldId::DaemonStartFile, focused)
                .on_input(|v| Message::Daemons(DaemonsMsg::StartFileEdited(v)))
                .into(),
        ),
        StartSource::Inline => controls.push(
            TextArea::new(start.editor.content(), roles)
                .placeholder("#!/bin/sh")
                .on_action(|a| Message::Daemons(DaemonsMsg::StartTextAction(EditorAction(a))))
                .into(),
        ),
    }
    if let Some(message) = error_of(state, "startup_script") {
        controls.push(caution(message, roles));
    }
    controls.push(
        TextField::new("Default shell", &start.shell, roles)
            .label("Shell")
            .track_focus(FieldId::DaemonStartShell, focused)
            .on_input(|v| Message::Daemons(DaemonsMsg::StartShellEdited(v)))
            .into(),
    );
    controls.push(
        TextField::new("No timeout", &start.timeout, roles)
            .label("Timeout (seconds)")
            .track_focus(FieldId::DaemonStartTimeout, focused)
            .supporting("From 1 to 3600")
            .error(error_of(state, "startup_timeout"))
            .on_input(|v| Message::Daemons(DaemonsMsg::StartTimeoutEdited(v)))
            .into(),
    );
    controls.push(
        Select::new(
            ON_FAILURES,
            Some(Named(
                start.on_failure,
                name_of(ON_FAILURES, start.on_failure),
            )),
            |chosen: Named<OnFailure>| Message::Daemons(DaemonsMsg::StartOnFailureChosen(chosen.0)),
            roles,
        )
        .label("When the script fails")
        .into(),
    );
    controls.push(
        Select::new(
            RUN_POLICIES,
            Some(Named(start.run, name_of(RUN_POLICIES, start.run))),
            |chosen: Named<RunPolicy>| Message::Daemons(DaemonsMsg::StartRunChosen(chosen.0)),
            roles,
        )
        .label("Run")
        .into(),
    );
    controls.push(
        TextField::new("Not set", &start.env_file, roles)
            .label("Secrets file")
            .track_focus(FieldId::DaemonStartEnvFile, focused)
            .supporting("Path of a file of KEY=value lines; its values never enter the config file")
            .on_input(|v| Message::Daemons(DaemonsMsg::StartEnvFileEdited(v)))
            .into(),
    );
    controls.push(
        Checkbox::new(
            "Export the script's environment to sessions",
            start.export_env,
            roles,
        )
        .track_focus(FieldId::DaemonStartExport, focused)
        .on_toggle(|v| Message::Daemons(DaemonsMsg::StartExportToggled(v)))
        .into(),
    );
    controls
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
            AllowedKind::RegistryPull => Some((
                "Registry credentials file",
                &kube.pull_file,
                FieldId::DaemonKubePullFile,
                "pull_file",
            )),
            _ => None,
        };
        if let Some((label, value, id, name)) = file.filter(|_| kube.allowed.contains(&kind)) {
            let on_input: fn(String) -> Message =
                |v| Message::Daemons(DaemonsMsg::PullFileEdited(v));
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
        Button::filled("Add daemon", roles)
            .id(opener_id(Opener::Add))
            .on_press(Message::Daemons(DaemonsMsg::AddOpened)),
    ]
    .spacing(spacing::SM)
    .width(Length::Fill)
    .into()
}

/// The Daemons page.
pub fn view<'a>(
    state: &'a State,
    list: ViewModel,
    focused: Option<FieldId>,
    roles: Roles,
) -> Element<'a, Message> {
    let mut controls: Vec<Element<'a, Message>> = Vec::new();
    if list.empty {
        controls.push(empty_state(roles));
    }
    for view in list.rows {
        controls.push(daemon_row(view, state, focused, roles));
    }
    if !list.empty {
        controls.push(
            Button::outlined("Add daemon", roles)
                .id(opener_id(Opener::Add))
                .on_press(Message::Daemons(DaemonsMsg::AddOpened))
                .into(),
        );
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

#[cfg(test)]
mod tests {
    use super::*;
    use iced::advanced::widget::Tree;

    fn view_of(config_errors: &[&str]) -> DaemonView {
        DaemonView {
            id: DaemonId(1),
            name: "Box".into(),
            runtime_label: "Container".into(),
            endpoint: "127.0.0.1:7000".into(),
            version: None,
            state: DaemonState::Starting,
            bound: Vec::new(),
            activity: None,
            install_warnings: Vec::new(),
            notice: None,
            notice_dismissable: false,
            stop_blocked: false,
            explanation: None,
            replace_offered: false,
            replacing: false,
            stopped_reason: None,
            credential_lines: Vec::new(),
            config_errors: config_errors.iter().map(ToString::to_string).collect(),
            startup_restart: false,
        }
    }

    fn widgets(tree: &Tree) -> usize {
        1 + tree.children.iter().map(widgets).sum::<usize>()
    }

    /// Collects every fragment an `iced::widget::Text` reports through [`Operation::text`], in
    /// traversal order — enough to search the rendered row for a string, not to reconstruct its
    /// layout.
    struct CollectedText(Vec<String>);

    impl<T> iced::advanced::widget::Operation<T> for CollectedText {
        fn traverse(
            &mut self,
            operate: &mut dyn FnMut(&mut dyn iced::advanced::widget::Operation<T>),
        ) {
            operate(self);
        }

        fn text(
            &mut self,
            _id: Option<&iced::advanced::widget::Id>,
            _bounds: iced::Rectangle,
            text: &str,
        ) {
            self.0.push(text.to_string());
        }
    }

    /// Every piece of text `element` renders, laid out the way the real window would (the generic
    /// `Text` widget only reports its content from `operate`, which needs a layout to walk).
    fn rendered_text(element: &mut Element<'_, Message>) -> String {
        let renderer = crate::ui::material::test_support::renderer();
        let mut tree = Tree::new(element.as_widget());
        let node = element.as_widget_mut().layout(
            &mut tree,
            &renderer,
            &iced::advanced::layout::Limits::new(iced::Size::ZERO, iced::Size::new(2000.0, 4000.0)),
        );
        let mut collected = CollectedText(Vec::new());
        element.as_widget_mut().operate(
            &mut tree,
            iced::advanced::Layout::new(&node),
            &renderer,
            &mut collected,
        );
        collected.0.join("\n")
    }

    /// Each line of `config_errors` is a row of its own under the daemon (FR-013f): the row grows
    /// by the same amount for every line, and the error's own text is findable in it.
    #[test]
    fn each_config_error_adds_a_line_to_the_row() {
        let roles = micold_core::tokens::roles(micold_core::theme::ColorScheme::Light);
        let state = State::default();
        let size = |errors: &[&str]| {
            let row = daemon_row(view_of(errors), &state, None, roles);
            widgets(&Tree::new(&row))
        };
        let (none, one, two) = (size(&[]), size(&["a"]), size(&["a", "b"]));
        assert!(one > none, "one error adds nothing: {none} vs {one}");
        assert_eq!(
            two - one,
            one - none,
            "each error adds the same: {none} {one} {two}"
        );

        let mut with_error = daemon_row(view_of(&["disk quota exceeded"]), &state, None, roles);
        let text = rendered_text(&mut with_error);
        assert!(
            text.contains("disk quota exceeded"),
            "the config error's own text is missing from the rendered row: {text}"
        );
    }

    /// The `[[ai.cli]]` accordion (`ai_cli_controls`) names no runtime (SC-006): it reads only
    /// `draft.ai_cli`, so the same config renders the same controls whichever of the four kinds
    /// `draft.runtime` is set to. The doc comment at its definition explains why — branching it per
    /// kind would cost nothing a mutant could catch — and this guards that choice: if a future edit
    /// made it branch on `draft.runtime` after all, the kinds below would stop agreeing.
    #[test]
    fn ai_cli_controls_are_the_same_for_every_daemon_kind() {
        let roles = micold_core::tokens::roles(micold_core::theme::ColorScheme::Light);
        let mut state = State::default();
        crate::features::daemons::begin_add(&mut state);
        let mut draft = state.draft.clone().expect("begin_add sets a draft");
        draft.ai_cli[0].version = "1.2.3".into();
        draft.ai_cli[0].args = "--flag value".into();
        draft.ai_cli[1].model = "gpt-5".into();

        let kinds = [
            DraftRuntime::Host,
            DraftRuntime::Ssh,
            DraftRuntime::Container,
            DraftRuntime::Kubernetes,
        ];
        let rendered: Vec<(usize, String)> = kinds
            .iter()
            .map(|&kind| {
                draft.runtime = kind;
                let mut section: Element<'_, Message> =
                    column(ai_cli_controls(&draft, None, roles)).into();
                (widgets(&Tree::new(&section)), rendered_text(&mut section))
            })
            .collect();
        for (kind, (count, text)) in kinds.iter().zip(&rendered) {
            assert_eq!(
                *count, rendered[0].0,
                "{kind:?}'s AI CLI section has a different number of controls: {rendered:?}"
            );
            assert_eq!(
                text, &rendered[0].1,
                "{kind:?}'s AI CLI section renders different text: {rendered:?}"
            );
        }
    }
}
