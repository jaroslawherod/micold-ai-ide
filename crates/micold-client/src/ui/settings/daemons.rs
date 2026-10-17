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

use crate::app::Message;
use crate::features::daemons::{
    DaemonDetails, Draft, DraftRuntime, FormError, Msg as DaemonsMsg, State, ViewModel,
};
use crate::features::settings::DaemonDraft;
use crate::features::window::FieldId;
use crate::ui::focus::TrackFocus;
use crate::ui::material::{Button, Checkbox, Select, StageProgress, Tag, TextField};
use crate::ui::settings::container_fields::{self, ContainerView, FieldErrors};
use crate::ui::settings::{caution, group, name_of, note, page, Named};
use iced::widget::{column, row};
use iced::{Element, Length};
use micold_core::daemons::{DaemonId, DaemonState, DaemonView, ATTEMPTS_STOPPED};
use micold_core::sandbox::placement::PlacementKind;
use micold_core::tokens::{spacing, Roles};

const RUNTIMES: &[Named<DraftRuntime>] = &[
    Named(DraftRuntime::Container, "Container"),
    Named(DraftRuntime::Host, "Host"),
    Named(DraftRuntime::Ssh, "Remote over SSH"),
];

/// The section's explanation when no daemon is configured.
pub const EMPTY_TITLE: &str = "No daemons are set up";
/// What a daemon is, and what happens until one is added.
pub const EMPTY_BODY: &str = "A daemon runs your sessions and terminals, on this computer or in a \
     container. Until one is added, worktrees cannot start sessions. Add one to get going.";

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
        DaemonState::Unreachable { reason } if reason == ATTEMPTS_STOPPED => Some(format!(
            "not connected to {}: {ATTEMPTS_STOPPED}",
            view.endpoint
        )),
        DaemonState::Unreachable { reason } => {
            Some(format!("Daemon {} unreachable: {reason}", view.name))
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

fn actions<'a>(view: &DaemonView, open: bool, roles: Roles) -> Element<'a, Message> {
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
            buttons = buttons.push(
                Button::text("Stop", roles).on_press(Message::Daemons(DaemonsMsg::Stopped(id))),
            );
        }
        DaemonState::Unreachable { .. } | DaemonState::VersionMismatch { .. } => {
            buttons = buttons.push(
                Button::text("Reconnect", roles)
                    .on_press(Message::Daemons(DaemonsMsg::Reconnected(id))),
            );
            buttons = buttons.push(
                Button::text("Stop", roles).on_press(Message::Daemons(DaemonsMsg::Stopped(id))),
            );
        }
        DaemonState::Starting | DaemonState::Connected => {
            buttons = buttons.push(
                Button::text("Stop", roles).on_press(Message::Daemons(DaemonsMsg::Stopped(id))),
            );
        }
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
    if view.runtime_label == micold_core::daemons::SSH_RUNTIME_LABEL {
        body = body.push(repository_step(&view, state, focused, roles));
    }
    body = body.push(actions(&view, open, roles));
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
    row![
        TextField::new("/path/to/repository on the host", typed, roles)
            .label("Repository on the host")
            .required(repository_state)
            .error(error)
            .track_focus(FieldId::DaemonRepository(id), focused)
            .on_input(move |v| Message::Daemons(DaemonsMsg::RepositoryEdited(id, v))),
        add,
    ]
    .spacing(spacing::SM)
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
    state: &State,
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
    let mut fields = column![
        group(title, roles),
        TextField::new("", &draft.name, roles)
            .label("Name")
            .required(marker(FieldId::DaemonName))
            .track_focus(FieldId::DaemonName, focused)
            .error(error_of(state, "name"))
            .on_input(|v| Message::Daemons(DaemonsMsg::NameEdited(v))),
        Select::new(
            RUNTIMES,
            Some(Named(draft.runtime, name_of(RUNTIMES, draft.runtime))),
            |chosen: Named<DraftRuntime>| Message::Daemons(DaemonsMsg::RuntimeChosen(chosen.0)),
            roles,
        )
        .label("Runs in"),
    ]
    .spacing(spacing::SM)
    .width(Length::Fill);
    if let Some(message) = error_of(state, "runtime") {
        fields = fields.push(caution(message, roles));
    }
    if draft.runtime == DraftRuntime::Container {
        fields = fields
            .push(
                TextField::new("", &draft.container_name, roles)
                    .label("Container name")
                    .required(marker(FieldId::DaemonContainerName))
                    .track_focus(FieldId::DaemonContainerName, focused)
                    .error(error_of(state, "container_name"))
                    .on_input(|v| Message::Daemons(DaemonsMsg::ContainerNameEdited(v))),
            )
            .push(
                TextField::new("A free port is chosen if empty", &draft.port, roles)
                    .label("Port")
                    .track_focus(FieldId::DaemonPort, focused)
                    .error(error_of(state, "port"))
                    .on_input(|v| Message::Daemons(DaemonsMsg::PortEdited(v))),
            );
        let described = draft.editing.is_some() && draft.editing == facts.legacy_default;
        let machine = facts.draft.filter(|_| described);
        let view = ContainerView {
            profile: &draft.profile,
            image_path: &draft.image_path,
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
        for control in container_fields::sections(
            &view,
            errors,
            machine.and(facts.missing_cli.clone()),
            focused,
            |section| state.section_open(section),
            roles,
        ) {
            fields = fields.push(control);
        }
    }
    if draft.runtime == DraftRuntime::Host {
        // A host daemon keeps no container profile: the groups stay out rather than take edits
        // that would be dropped, and the one thing that applies to it is said (FR-014d).
        fields = fields
            .push(group("Sessions", roles))
            .push(container_fields::survival_note(
                PlacementKind::HostProcess,
                roles,
            ));
    }
    if draft.runtime == DraftRuntime::Ssh {
        fields = fields
            .push(
                TextField::new("", &draft.ssh_host, roles)
                    .label("Host")
                    .required(marker(FieldId::DaemonSshHost))
                    .track_focus(FieldId::DaemonSshHost, focused)
                    .error(error_of(state, "host"))
                    .on_input(|v| Message::Daemons(DaemonsMsg::SshHostEdited(v))),
            )
            .push(
                TextField::new("", &draft.ssh_user, roles)
                    .label("User")
                    .track_focus(FieldId::DaemonSshUser, focused)
                    .error(error_of(state, "user"))
                    .on_input(|v| Message::Daemons(DaemonsMsg::SshUserEdited(v))),
            )
            .push(
                TextField::new("", &draft.ssh_port, roles)
                    .label("Port")
                    .track_focus(FieldId::DaemonSshPort, focused)
                    .error(error_of(state, "port"))
                    .on_input(|v| Message::Daemons(DaemonsMsg::SshPortEdited(v))),
            )
            .push(
                TextField::new("", &draft.ssh_key_file, roles)
                    .label("Key file")
                    .track_focus(FieldId::DaemonSshKeyFile, focused)
                    .error(error_of(state, "key_file"))
                    .on_input(|v| Message::Daemons(DaemonsMsg::SshKeyFileEdited(v))),
            );
    }
    fields = fields
        .push(
            Checkbox::new(
                "Connect when the application starts",
                draft.auto_start,
                roles,
            )
            .track_focus(FieldId::DaemonAutoStart, focused)
            .on_toggle(|v| Message::Daemons(DaemonsMsg::AutoStartToggled(v))),
        )
        .push(
            row![
                Button::filled("Save daemon", roles).on_press(Message::Daemons(DaemonsMsg::Saved)),
                Button::outlined("Cancel", roles)
                    .on_press(Message::Daemons(DaemonsMsg::DraftDiscarded)),
            ]
            .spacing(spacing::SM),
        );
    fields.into()
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
