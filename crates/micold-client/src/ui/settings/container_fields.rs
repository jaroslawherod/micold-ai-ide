//! The container settings of a daemon: runtime, image, credential shares, network, limits and
//! survive-logout (feature 027, FR-028; moved into the Daemons form by feature 804).
//!
//! One copy of the fields and of the decisions about them (which limit is editable, what is said
//! about an unshared sign-in), drawn by the Daemons form. The form hands the fields a
//! [`ContainerView`] instead of a settings draft, so the fields own no state.
//!
//! # Why an unsupported limit is disabled rather than hidden
//!
//! FR-015 asks for three things at once and they pull against each other: the limit must not be
//! silently accepted (the user would believe a bound exists that does not), must not be hidden
//! (a setting that vanishes on one machine and reappears on another is a setting the user cannot
//! reason about, and the stored value is still there), and must say *why*. A disabled field
//! carrying the runtime's own reason is the only arrangement that does all three.

use crate::app::Message;
use crate::features::daemon_sections::{marked, summary};
use crate::features::daemons::ImageKind;
use crate::features::daemons::{Draft, State};
use crate::features::daemons::{Msg as DaemonsMsg, Section};
use crate::features::sandbox::SandboxLimit;
use crate::features::window::FieldId;
use crate::ui::focus::TrackFocus;
use crate::ui::material::{AccordionSection, Checkbox, FieldGroup, Select, TextField};
use crate::ui::settings::{caution, field_note, name_of, note, section_body, Named};
use iced::Element;
use micold_core::required::FieldRequirement;
use micold_core::sandbox::placement::PlacementKind;
use micold_core::sandbox::runtime::{LimitSupport, RuntimeCapabilities, RuntimeKind};
use micold_core::sandbox::{CredentialShare, NetworkPosture, SandboxProfile};
use micold_core::session::AiCli;
use micold_core::tokens::Roles;

const RUNTIMES: &[Named<RuntimeKind>] = &[
    Named(RuntimeKind::Docker, "Docker"),
    Named(RuntimeKind::Podman, "Podman"),
];

const NETWORKS: &[Named<NetworkPosture>] = &[
    Named(NetworkPosture::NoOutbound, "No outbound connections"),
    Named(NetworkPosture::Outbound, "Allow outbound connections"),
];

/// The two entries of the image form (feature 806, FR-001).
const IMAGE_SOURCES: &[Named<ImageKind>] = &[
    Named(ImageKind::Basic, "Basic image (recommended)"),
    Named(ImageKind::Own, "My own image"),
];

/// The note under "My own image" (FR-002).
const OWN_IMAGE_NOTE: &str = "Your image must run an SSH server; see the user guide.";

/// The note under the AI CLI checkboxes (FR-010): a change reaches the container at its next start,
/// where the selection is installed and offered.
pub const AI_CLI_NOTE: &str = "Changes apply at the next start. Only the AI tools ticked here are \
     installed in the container and offered for sessions; with none ticked, none is offered.";

/// One checkbox per AI CLI the app supports, in the order the menus offer them, with the name a
/// menu uses and whether this daemon's profile has it selected.
pub fn ai_cli_rows(profile: &SandboxProfile) -> Vec<(AiCli, &'static str, bool)> {
    AiCli::ALL
        .into_iter()
        .map(|cli| {
            (
                cli,
                cli.provider().display_name(),
                profile.ai_clis.0.contains(&cli),
            )
        })
        .collect()
}

/// What the fields read: one daemon's profile, the typed texts beside it and the machine facts.
#[derive(Debug, Clone, Copy)]
pub struct ContainerView<'a> {
    /// The profile being edited.
    pub profile: &'a SandboxProfile,
    /// The archive path as typed.
    pub image_path: &'a str,
    /// The image reference as typed.
    pub image_reference: &'a str,
    /// The processor limit as typed.
    pub cpus: &'a str,
    /// The memory limit as typed.
    pub memory_mib: &'a str,
    /// The process limit as typed.
    pub pids: &'a str,
    /// The storage limit as typed.
    pub storage_mib: &'a str,
    /// What the runtime was probed to enforce; `None` is *not yet known*, which renders every
    /// limit editable (the form must not invent a restriction it has not been told about).
    pub capabilities: Option<&'a RuntimeCapabilities>,
    /// The shared sign-in the running container does not mount (FR-004g, BUG-008).
    pub unshared_sign_in: Option<&'a str>,
    /// Where the service runs, which decides what survive-logout can promise (FR-014d).
    pub placement: PlacementKind,
}

/// Errors to show beside the fields, looked up by the caller in its own error store.
#[derive(Debug, Clone, Default)]
pub struct FieldErrors {
    /// The image reference.
    pub reference: Option<String>,
    /// The image archive.
    pub path: Option<String>,
    /// Processor, memory, process and storage limits, in that order.
    pub limits: [Option<String>; 4],
    /// How the image reference is marked (feature 937).
    pub reference_required: FieldRequirement,
    /// How the image archive is marked (feature 937).
    pub path_required: FieldRequirement,
}

/// The share whose sign-in is reported unmounted, while the profile still shares it. A report about
/// a share the user has just unticked is not shown.
fn unshared_sign_in<'a>(view: &ContainerView<'a>) -> Option<&'a str> {
    view.profile
        .credentials
        .contains(&CredentialShare::AiCliAuth)
        .then_some(view.unshared_sign_in)
        .flatten()
}

/// What to say under the *AI CLI sign-in* share when the running sandbox has no token from it
/// (FR-004g, BUG-008), or `None`.
///
/// It names the path, because "not shared" alone sends the user looking for why. It offers the two
/// ways that work and not a restart: a restart adopts the same container, which keeps the mounts
/// it was created with. Moving the service to this computer removes the container, so moving it
/// back creates one with the token (`shell::sandbox::stop`).
///
/// The Keychain is named on macOS only, where it is the usual reason (FR-004g).
pub fn unshared_sign_in_notice(view: &ContainerView<'_>) -> Option<String> {
    let keychain = if cfg!(target_os = "macos") {
        " On macOS, Claude Code keeps the token in the Keychain rather than in that file."
    } else {
        ""
    };
    unshared_sign_in(view).map(|path| {
        format!(
            "The sign-in is not shared: the running sandbox's container has no token from {path}.\
             {keychain} Sign in inside a session with /login. Or put the token there, then set \
             \u{2018}Where sessions run\u{2019} to On this computer and back, which creates the \
             container again with it."
        )
    })
}

/// What the user has shared, named one by one (FR-004c, rule N-2).
///
/// "3 credentials shared" is the summary this deliberately is not: a count says that something is
/// shared without saying *what*, which is the one thing a user needs to decide whether they meant
/// it. A sign-in the running sandbox has no token from is left out (FR-004g); with nothing else
/// shared there is no summary at all rather than an empty list.
pub fn sharing_summary(view: &ContainerView<'_>) -> Option<String> {
    let sign_in_unshared = unshared_sign_in(view).is_some();
    let shared: Vec<CredentialShare> = view
        .profile
        .credentials
        .iter()
        .copied()
        .filter(|c| !(sign_in_unshared && *c == CredentialShare::AiCliAuth))
        .collect();
    if shared.is_empty() {
        return None;
    }
    let labels: Vec<&str> = shared.iter().map(|c| c.label()).collect();
    let summary = format!("Shared with the container: {}.", labels.join(", "));
    // FR-004e: every share is read-only but the sign-in, which the CLI rewrites on each refresh.
    Some(if shared.contains(&CredentialShare::AiCliAuth) {
        format!(
            "{summary} A session can use the AI CLI sign-in and replace its token, which the \
                 CLI does each time it refreshes it."
        )
    } else {
        summary
    })
}

/// What the user is told at the moment they choose to cut the sandbox off (FR-018).
///
/// The AI CLI talks to a hosted provider. Blocking outbound connections is the *safe* choice and
/// also the choice that stops the feature this application exists for from working at all, so it
/// is said at the moment of the change rather than left to an error later on.
pub const NO_OUTBOUND_WARNING: &str =
    "The AI agent reaches its provider over the network. With outbound connections blocked it \
     cannot sign in or answer, and neither can anything a session tries to fetch. The control \
     channel to this application is unaffected.";

/// The warning to show beside the network picker, if the current choice warrants one.
pub fn network_warning(view: &ContainerView<'_>) -> Option<&'static str> {
    match view.profile.network {
        NetworkPosture::NoOutbound => Some(NO_OUTBOUND_WARNING),
        NetworkPosture::Outbound => None,
    }
}

/// One resource limit, as it is about to be drawn.
///
/// The decision (which limits appear, which are editable, what a disabled one says instead) is made
/// here and the drawing is a transcription of it, because a view function is the one thing in this
/// crate a test cannot look inside.
pub struct LimitControl<'a> {
    /// The control the keyboard and any error message address.
    pub field: FieldId,
    /// Its name in the form.
    pub label: &'static str,
    /// What the user has typed, or what was stored. Shown whether or not the field is editable.
    pub value: &'a str,
    /// The line under the field: the unit and what empty means, or the runtime's own reason.
    pub supporting: String,
    /// The message typing emits. Present only while the field is editable.
    pub on_input: Option<fn(String) -> Message>,
}

impl LimitControl<'_> {
    /// Whether the user can type in it.
    pub fn editable(&self) -> bool {
        self.on_input.is_some()
    }
}

/// The four limits' fields, in the order they are shown.
const LIMIT_FIELDS: [FieldId; 4] = [
    FieldId::DaemonCpuLimit,
    FieldId::DaemonMemoryLimit,
    FieldId::DaemonPidLimit,
    FieldId::DaemonStorageLimit,
];

/// The message typing in the `index`th limit emits.
fn limit_input(index: usize) -> fn(String) -> Message {
    match index {
        0 => |v| Message::Daemons(DaemonsMsg::CpuLimitEdited(v)),
        1 => |v| Message::Daemons(DaemonsMsg::MemoryLimitEdited(v)),
        2 => |v| Message::Daemons(DaemonsMsg::PidLimitEdited(v)),
        _ => |v| Message::Daemons(DaemonsMsg::StorageLimitEdited(v)),
    }
}

/// The four limits, in the order they are shown (FR-012 … FR-015).
///
/// Unsupported is disabled rather than hidden (see the module docs). Capabilities are `Option`
/// because "not probed yet" is a real state: unknown means editable.
pub fn limits<'a>(view: &ContainerView<'a>) -> [LimitControl<'a>; 4] {
    let caps = view.capabilities;
    // Three of the four labels come from `SandboxLimit`, because a session stopped by one of them
    // is reported by naming its setting (T088) and a report that used a different name than the
    // form would send the user hunting. A CPU share throttles rather than stopping anything, so
    // the processor limit names itself.
    let specs: [(&'static str, &'a str, &'static str, Option<&LimitSupport>); 4] = [
        (
            "Processor limit",
            view.cpus,
            "Cores, e.g. 2 or 1.5. Empty leaves the runtime's own default.",
            caps.map(|c| &c.cpus),
        ),
        (
            SandboxLimit::Memory.setting(),
            view.memory_mib,
            "Mebibytes. Empty leaves the runtime's own default.",
            caps.map(|c| &c.memory),
        ),
        (
            SandboxLimit::Processes.setting(),
            view.pids,
            "Processes the sandbox may run at once. Empty leaves the runtime's own default.",
            caps.map(|c| &c.pids),
        ),
        (
            SandboxLimit::Storage.setting(),
            view.storage_mib,
            "Mebibytes the sandbox may write. Not enforceable on every storage driver.",
            caps.map(|c| &c.storage),
        ),
    ];
    let mut index = 0;
    specs.map(|(label, value, hint, support)| {
        let i = index;
        index += 1;
        let (supporting, on_input) = match support {
            Some(LimitSupport::Unsupported { reason }) => {
                (format!("Cannot be enforced here: {reason}"), None)
            }
            _ => (hint.to_string(), Some(limit_input(i))),
        };
        LimitControl {
            field: LIMIT_FIELDS[i],
            label,
            value,
            supporting,
            on_input,
        }
    })
}

/// One limit's text field: the one copy of how a limit is drawn, used by the container form and the
/// Kubernetes form (feature 686, T131; #806 inherits it, contract shared-ssh-image §5).
pub fn limit_field<'a>(
    l: LimitControl<'a>,
    error: Option<String>,
    focused: Option<FieldId>,
    roles: Roles,
) -> Element<'a, Message> {
    let mut input = TextField::new("", l.value, roles)
        .label(l.label)
        .supporting(l.supporting)
        .error(error)
        .track_focus(l.field, focused);
    if let Some(on_input) = l.on_input {
        input = input
            .on_input(on_input)
            .on_submit(Message::Daemons(DaemonsMsg::Saved));
    }
    input.into()
}

/// The Kubernetes form's four resource fields: CPU request, CPU limit, memory request, memory limit
/// (FR-005). Each is edited on its own; an empty one is unset.
pub fn kube_resources<'a>(
    draft: &'a crate::features::daemons_kube::KubeDraft,
) -> [LimitControl<'a>; 4] {
    let control = |field, label, value: &'a str, supporting: &str, on_input| LimitControl {
        field,
        label,
        value,
        supporting: supporting.to_string(),
        on_input: Some(on_input),
    };
    [
        control(
            FieldId::DaemonCpuRequest,
            "CPU request",
            &draft.cpu_request,
            "Cores, e.g. 0.5. Empty reserves none.",
            (|v| Message::Daemons(DaemonsMsg::CpuRequestEdited(v))) as fn(String) -> Message,
        ),
        control(
            FieldId::DaemonCpuLimit,
            "CPU limit",
            &draft.cpu_limit,
            "Cores, e.g. 2 or 1.5. Empty sets no limit.",
            |v| Message::Daemons(DaemonsMsg::KubeCpuLimitEdited(v)),
        ),
        control(
            FieldId::DaemonMemoryRequest,
            "Memory request",
            &draft.memory_request,
            "Mebibytes. Empty reserves none.",
            |v| Message::Daemons(DaemonsMsg::MemoryRequestEdited(v)),
        ),
        control(
            FieldId::DaemonMemoryLimit,
            "Memory limit",
            &draft.memory_limit,
            "Mebibytes. Empty sets no limit.",
            |v| Message::Daemons(DaemonsMsg::KubeMemoryLimitEdited(v)),
        ),
    ]
}

/// What to say beneath the survival opt-in, and whether it is a caution (feature 027, FR-014d).
///
/// A free function rather than a `match` inside the view, because the claim FR-014d makes is about
/// *every* placement and a rule stated over a set is only checkable if it can be enumerated.
///
/// No `cfg!` since feature 028: the same answer on all three platforms is one thing to keep true.
pub fn survival_support(placement: PlacementKind) -> (&'static str, bool) {
    match placement {
        // The container runtime's restart policy is run by a service the platform keeps alive
        // across logout and reboot (FR-014b). It is applied at container creation, which is why
        // it names the next start.
        PlacementKind::LocalSandbox => (
            "The container is created with a restart policy, and is not stopped for being unused, \
             so this takes effect the next time the sandbox starts.",
            false,
        ),
        // Feature 028: nothing the application starts outlives the session it was started from.
        // Sessions are still safe — they come back resumable — which is why this says what is lost
        // rather than only that something is.
        PlacementKind::HostProcess => (
            "A service running directly on this computer can't outlive signing out, and stops \
             itself after 30 minutes with no window connected. Your sessions are kept and come \
             back resumable; run the service in a container to keep them running.",
            true,
        ),
    }
}

/// The note under the survive-logout control.
pub fn survival_note<'a>(placement: PlacementKind, roles: Roles) -> Element<'a, Message> {
    // FR-014d: where the placement cannot provide this, the control says so. It is not hidden and
    // it is not left to do nothing.
    let (support, cannot) = survival_support(placement);
    if cannot {
        caution(support, roles)
    } else {
        note(support, roles)
    }
}

/// One titled [`AccordionSection`] around `body`, open as `is_open` says and toggled by its header,
/// showing the section's summary and required marker while closed (feature 890, FR-001, FR-004,
/// FR-014).
pub fn accordion<'a>(
    section: Section,
    body: Element<'a, Message>,
    is_open: bool,
    draft: &Draft,
    roles: Roles,
) -> Element<'a, Message> {
    AccordionSection::new(section.title(), body, roles)
        .open(is_open)
        .on_toggle(Message::Daemons(DaemonsMsg::SectionToggled(section)))
        .summary_maybe(summary(draft, section))
        .required(marked(draft, section))
        .into()
}

/// The six container sections, from the connection to the survive-logout opt-in, each a titled
/// [`AccordionSection`] open as `is_open` says (FR-012). `connection` is the controls of the first
/// one, which the form owns. `missing_cli` is the notice that the image
/// running now lacks an AI CLI (FR-023b), attached to the reference field.
pub fn sections<'a>(
    view: &ContainerView<'a>,
    connection: Vec<Element<'a, Message>>,
    errors: FieldErrors,
    missing_cli: Option<String>,
    focused: Option<FieldId>,
    state: &State,
    roles: Roles,
) -> Vec<Element<'a, Message>> {
    let Some(draft) = state.draft.as_ref() else {
        return Vec::new();
    };
    let runtime = Select::new(
        RUNTIMES,
        Some(Named(
            view.profile.runtime,
            name_of(RUNTIMES, view.profile.runtime),
        )),
        |chosen: Named<RuntimeKind>| Message::Daemons(DaemonsMsg::ContainerRuntimeChosen(chosen.0)),
        roles,
    )
    .label("Container runtime");

    let kind = ImageKind::of(&view.profile.image);
    let image_kind = Select::new(
        IMAGE_SOURCES,
        Some(Named(kind, name_of(IMAGE_SOURCES, kind))),
        |chosen: Named<ImageKind>| Message::Daemons(DaemonsMsg::ImageKindChosen(chosen.0)),
        roles,
    )
    .label("Image")
    .supporting(match kind {
        ImageKind::Basic => "The app copies its own daemon into it and installs the AI tools",
        ImageKind::Own => OWN_IMAGE_NOTE,
    });

    let reference = TextField::new("", view.image_reference, roles)
        .label("Image reference")
        .supporting("A digest or an exact tag; used when no image file is given")
        .required(errors.reference_required)
        .error(errors.reference)
        .track_focus(FieldId::DaemonImage, focused)
        .on_input(|v| Message::Daemons(DaemonsMsg::ImageEdited(v)))
        .on_submit(Message::Daemons(DaemonsMsg::Saved));

    let archive = TextField::new("", view.image_path, roles)
        .label("Image file")
        .supporting("An archive made by `docker save`, loaded instead of the reference")
        .required(errors.path_required)
        .error(errors.path)
        .track_focus(FieldId::DaemonImagePath, focused)
        .on_input(|v| Message::Daemons(DaemonsMsg::ImagePathEdited(v)))
        .on_submit(Message::Daemons(DaemonsMsg::Saved));

    // Feature 028: the service also stops itself after 30 continuous minutes with nothing
    // connected, and this opt-in is the one thing that suppresses that too — so the label names
    // both, before the choice is made rather than after it (FR-022).
    let survive = Checkbox::new(
        crate::features::sandbox::KEEP_RUNNING_SETTING,
        view.profile.survive_logout,
        roles,
    )
    .track_focus(FieldId::DaemonSurviveLogout, focused)
    .on_toggle(|v| Message::Daemons(DaemonsMsg::SurviveLogoutToggled(v)));

    // FR-023b, at the point the image is chosen: the note is about that field's value, so it shares
    // the column the field's own supporting line sits in (see `field_note`).
    let mut container: Vec<Element<'a, Message>> = vec![runtime.into(), image_kind.into()];
    if kind == ImageKind::Own {
        container.push(field_note(reference, missing_cli, roles));
        container.push(archive.into());
    }

    let mut tools = FieldGroup::new("AI tools in this daemon", roles).note(AI_CLI_NOTE);
    for (cli, name, on) in ai_cli_rows(view.profile) {
        tools = tools.push(
            Checkbox::new(name, on, roles)
                .track_focus(FieldId::DaemonAiCli(cli), focused)
                .on_toggle(move |checked| Message::Daemons(DaemonsMsg::AiCliToggled(cli, checked))),
        );
    }
    container.push(tools.into());

    let mut shares = FieldGroup::new("Shared with the container", roles);
    for share in CredentialShare::ALL {
        let on = view.profile.credentials.contains(&share);
        shares = shares.push(
            Checkbox::new(share.label(), on, roles)
                .track_focus(FieldId::DaemonCredential(share), focused)
                .on_toggle(move |checked| {
                    Message::Daemons(DaemonsMsg::CredentialToggled(share, checked))
                }),
        );
        // FR-004g: directly under the share it is about, so it cannot be read as the next one's.
        if share == CredentialShare::AiCliAuth {
            if let Some(notice) = unshared_sign_in_notice(view) {
                shares = shares.push(caution(notice, roles));
            }
        }
    }
    if let Some(summary) = sharing_summary(view) {
        shares = shares.push(caution(summary, roles));
    }
    let credentials: Vec<Element<'a, Message>> = vec![shares.into()];

    let network = Select::new(
        NETWORKS,
        Some(Named(
            view.profile.network,
            name_of(NETWORKS, view.profile.network),
        )),
        |chosen: Named<NetworkPosture>| Message::Daemons(DaemonsMsg::NetworkChosen(chosen.0)),
        roles,
    )
    .label("Network")
    .supporting(
        "Names still resolve either way — the runtime's resolver answers from the host side. \
         Blocked means connections fail, not that nothing leaves the container.",
    );

    let mut net: Vec<Element<'a, Message>> = vec![network.into()];
    if let Some(warning) = network_warning(view) {
        net.push(caution(warning, roles));
    }

    let mut limit_fields: Vec<Element<'a, Message>> = Vec::new();
    let [e0, e1, e2, e3] = errors.limits;
    for (l, error) in limits(view).into_iter().zip([e0, e1, e2, e3]) {
        limit_fields.push(limit_field(l, error, focused, roles));
    }

    let sessions: Vec<Element<'a, Message>> =
        vec![survive.into(), survival_note(view.placement, roles)];

    [
        (Section::Connection, None, connection),
        (Section::Container, None, container),
        (
            Section::Credentials,
            Some(
                "The container starts with none of your credentials. Share only what the agent \
                 needs.",
            ),
            credentials,
        ),
        (Section::Network, None, net),
        (
            Section::Resources,
            Some(
                "What the sandbox may consume. A limit the runtime cannot enforce is shown, and \
                 says so, rather than being quietly dropped.",
            ),
            limit_fields,
        ),
        (Section::Sessions, None, sessions),
    ]
    .into_iter()
    .map(|(section, lead, controls)| {
        let body = section_body(lead, controls, roles);
        accordion(section, body, state.section_open(section), draft, roles)
    })
    .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sharing(shares: &[CredentialShare]) -> SandboxProfile {
        SandboxProfile {
            credentials: shares.iter().copied().collect(),
            ..SandboxProfile::default()
        }
    }

    fn view_of<'a>(profile: &'a SandboxProfile, unshared: Option<&'a str>) -> ContainerView<'a> {
        ContainerView {
            profile,
            image_path: "",
            image_reference: "",
            cpus: "",
            memory_mib: "",
            pids: "",
            storage_mib: "",
            capabilities: None,
            unshared_sign_in: unshared,
            placement: PlacementKind::LocalSandbox,
        }
    }

    /// FR-014d, stated over the whole set: no placement leaves this control unexplained, and the
    /// one that cannot honour the opt-in is the one that warns.
    #[test]
    fn every_placement_says_what_it_will_do() {
        for placement in [PlacementKind::LocalSandbox, PlacementKind::HostProcess] {
            let (support, cannot) = survival_support(placement);
            assert!(!support.is_empty(), "{placement:?} explains nothing");
            match placement {
                // The sandbox can always do it — that is FR-014b's whole point.
                PlacementKind::LocalSandbox => assert!(!cannot, "warned when it can do it"),
                // Feature 028, FR-005, deliberately un-`cfg`-ed.
                PlacementKind::HostProcess => {
                    assert!(cannot, "must warn: nothing it starts outlives sign-out");
                    assert!(
                        !support.contains("systemd"),
                        "the host mechanism is gone; the control must not name it: {support}"
                    );
                }
            }
        }
    }

    /// The warning has to name the way out, or it is only a refusal.
    #[test]
    fn the_warning_names_the_placement_that_does_support_it() {
        let (support, cannot) = survival_support(PlacementKind::HostProcess);
        assert!(cannot);
        assert!(support.contains("container"));
        // And what is *not* lost, or the warning reads as "your work goes away" (FR-005).
        assert!(support.contains("resumable"));
    }

    /// FR-004e/FR-004b (BUG-006): the one writable share says so where it is chosen.
    #[test]
    fn a_shared_sign_in_says_a_session_can_replace_the_token() {
        let profile = sharing(&[CredentialShare::AiCliAuth]);
        let summary = sharing_summary(&view_of(&profile, None)).expect("the sign-in is shared");
        assert!(
            summary.contains("replace"),
            "the sign-in share is writable and the caution does not say so: {summary}"
        );
    }

    const LOOKED_FOR: &str = "/Users/u/.claude/.credentials.json";

    /// FR-004g (BUG-008): the share says, where it is ticked, that the running sandbox has no token
    /// from it, and names the path. It does not offer a restart, which would adopt the same
    /// container.
    #[test]
    fn an_unshared_sign_in_is_named_under_its_share() {
        let profile = sharing(&[CredentialShare::AiCliAuth]);
        let notice = unshared_sign_in_notice(&view_of(&profile, Some(LOOKED_FOR)))
            .expect("no notice under the share");
        assert!(
            notice.contains(LOOKED_FOR),
            "the path is not named: {notice}"
        );
        assert!(
            !notice.to_lowercase().contains("restart"),
            "a restart adopts the same container and shares nothing new: {notice}"
        );
        assert!(
            notice.contains("On this computer"),
            "the one in-app way to a new container is not named: {notice}"
        );
        assert_eq!(
            notice.contains("Keychain"),
            cfg!(target_os = "macos"),
            "the Keychain is named where it is not the reason: {notice}"
        );
    }

    /// …and the summary stops calling it shared. Alone, there is nothing shared to state.
    #[test]
    fn an_unshared_sign_in_is_not_called_shared() {
        let both = sharing(&[CredentialShare::GitConfig, CredentialShare::AiCliAuth]);
        let summary = sharing_summary(&view_of(&both, Some(LOOKED_FOR)))
            .expect("git configuration is still shared");
        assert!(
            !summary.contains(CredentialShare::AiCliAuth.label()),
            "the sign-in is listed as shared: {summary}"
        );
        assert!(!summary.contains("replace"), "{summary}");
        assert!(
            summary.contains(CredentialShare::GitConfig.label()),
            "{summary}"
        );

        let alone = sharing(&[CredentialShare::AiCliAuth]);
        assert_eq!(
            sharing_summary(&view_of(&alone, Some(LOOKED_FOR))),
            None,
            "an empty list of what is shared"
        );
    }

    /// Without a report, nothing changes; with the share off, a report is not shown.
    #[test]
    fn a_sign_in_with_no_report_or_no_share_gets_no_notice() {
        let signed_in = sharing(&[CredentialShare::AiCliAuth]);
        let reported_nothing = view_of(&signed_in, None);
        assert_eq!(unshared_sign_in_notice(&reported_nothing), None);
        let summary = sharing_summary(&reported_nothing).expect("the sign-in is shared");
        assert!(
            summary.contains(CredentialShare::AiCliAuth.label()),
            "{summary}"
        );

        let git_only = sharing(&[CredentialShare::GitConfig]);
        assert_eq!(
            unshared_sign_in_notice(&view_of(&git_only, Some(LOOKED_FOR))),
            None
        );
    }

    /// The form edits all four limits, each through its own field.
    #[test]
    fn the_form_offers_four_editable_limits() {
        let profile = SandboxProfile::default();
        let controls = limits(&view_of(&profile, None));
        assert!(controls.iter().all(LimitControl::editable));
        let mut ids: Vec<_> = controls.iter().map(|c| format!("{:?}", c.field)).collect();
        ids.dedup();
        assert_eq!(ids.len(), 4);
    }

    /// FR-010: one checkbox per AI CLI the app supports, ticked as the profile has them, and the
    /// note that a change applies at the next start.
    #[test]
    fn the_form_has_one_checkbox_per_ai_cli_ticked_as_selected() {
        let profile = SandboxProfile {
            ai_clis: micold_core::sandbox::AiCliSelection::new_daemon(),
            ..SandboxProfile::default()
        };
        let rows = ai_cli_rows(&profile);
        let clis: Vec<AiCli> = rows.iter().map(|r| r.0).collect();
        assert_eq!(clis, AiCli::ALL);
        let ticked: Vec<AiCli> = rows.iter().filter(|r| r.2).map(|r| r.0).collect();
        assert_eq!(ticked, [AiCli::ClaudeCode]);
        assert!(rows.iter().all(|r| !r.1.is_empty()));
        assert!(AI_CLI_NOTE.starts_with("Changes apply at the next start"));
    }

    /// Both choices, the way the tests below range over them.
    fn both_choices() -> [SandboxProfile; 2] {
        [
            SandboxProfile::default(),
            SandboxProfile {
                image: micold_core::sandbox::image::ImageChoice::Own(
                    micold_core::sandbox::image::OwnImage::Reference(
                        "registry.example/own:1".into(),
                    ),
                ),
                ..SandboxProfile::default()
            },
        ]
    }

    /// US4 #5, FR-001: the select has the two entries and none of them names the released image.
    #[test]
    fn the_image_select_has_exactly_the_basic_and_the_own_entry() {
        let names: Vec<&str> = IMAGE_SOURCES.iter().map(|n| n.1).collect();
        assert_eq!(names, ["Basic image (recommended)", "My own image"]);
        assert!(names.iter().all(|n| !n.to_lowercase().contains("released")));
        assert_eq!(
            OWN_IMAGE_NOTE,
            "Your image must run an SSH server; see the user guide."
        );
        let kinds: Vec<_> = IMAGE_SOURCES.iter().map(|n| n.0).collect();
        assert_eq!(kinds, [ImageKind::Basic, ImageKind::Own]);
    }

    /// The section builds for both entries (the fields it holds are checked by the geometry gate).
    #[test]
    fn the_container_section_builds_for_both_image_choices() {
        let [basic, own] = both_choices();
        let mut app = crate::app::State::default();
        app.update(Message::Daemons(DaemonsMsg::AddOpened));
        let form = &app.daemon_list;
        let count = |profile: &SandboxProfile| {
            let view = view_of(profile, None);
            // One accordion per section; the container section is open.
            sections(
                &view,
                Vec::new(),
                FieldErrors::default(),
                None,
                None,
                form,
                micold_core::tokens::roles(micold_core::theme::ColorScheme::Dark),
            )
            .len()
        };
        assert_eq!(count(&basic), 6);
        assert_eq!(count(&own), 6);
        assert_eq!(ImageKind::of(&basic.image), ImageKind::Basic);
        assert_eq!(ImageKind::of(&own.image), ImageKind::Own);
    }

    /// US3 #4, FR-003: "My own image" saved empty is refused, and a released-image reference is
    /// refused with the FR-003 message.
    #[test]
    fn an_empty_own_image_and_a_released_reference_are_refused() {
        use crate::features::settings::image_choice;
        use micold_core::sandbox::image::{ImageChoice, ImageSourceProblem, OwnImage};
        let own = ImageChoice::Own(OwnImage::Reference(String::new()));
        assert_eq!(
            image_choice(&own, "  ", ""),
            Err(ImageSourceProblem::MissingImage)
        );
        let released =
            image_choice(&own, "ghcr.io/jaroslawherod/micold-daemon:1.0.0", "").unwrap_err();
        assert_eq!(released, ImageSourceProblem::ReleasedImage);
        assert_eq!(
            released.message(),
            micold_core::sandbox::image::RELEASED_IMAGE_REFUSED
        );
        assert!(image_choice(&own, "registry.example/own:1", "").is_ok());
        assert_eq!(
            image_choice(&ImageChoice::Basic, "", ""),
            Ok(ImageChoice::Basic)
        );
    }

    /// US2 #2–#3, FR-014: for both choices the four limits keep their minimums, and a limit the
    /// runtime cannot enforce is disabled with the runtime's reason while the rest stay editable.
    #[test]
    fn the_limits_behave_the_same_for_both_image_choices() {
        use crate::features::settings::parse_budget;
        use micold_core::sandbox::runtime::IdentityMapping;
        let caps = RuntimeCapabilities {
            kind: RuntimeKind::Podman,
            version: "5".into(),
            cpus: LimitSupport::Supported,
            memory: LimitSupport::Supported,
            pids: LimitSupport::Supported,
            storage: LimitSupport::unsupported("overlay without quota"),
            identity_mapping: IdentityMapping::ExplicitUidGid,
        };
        for profile in both_choices() {
            let mut view = view_of(&profile, None);
            view.capabilities = Some(&caps);
            let controls = limits(&view);
            assert_eq!(
                controls
                    .iter()
                    .map(LimitControl::editable)
                    .collect::<Vec<_>>(),
                [true, true, true, false]
            );
            assert!(controls[3]
                .supporting
                .contains("Cannot be enforced here: overlay without quota"));
            for args in [
                ("0.001", "", "", ""),
                ("", "1", "", ""),
                ("", "", "1", ""),
                ("", "", "", "1"),
            ] {
                assert!(parse_budget(args.0, args.1, args.2, args.3).is_err());
            }
        }
    }
}
