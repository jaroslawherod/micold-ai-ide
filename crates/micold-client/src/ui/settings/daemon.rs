//! The Session service section — everything about the process that runs the agents (feature 027,
//! FR-028).
//!
//! # Why this is one section and not three
//!
//! FR-028 asks that every session-service setting be in one place, and the reason is not tidiness.
//! Where the service runs, what image it runs, and which of the user's credentials it can read are
//! one decision: sharing an SSH agent with a host process means nothing (it already has it) and
//! means a great deal with a container. Splitting them across pages would let a user answer half
//! of a question they never saw whole.
//!
//! # Why the sandbox controls stay visible on the host placement
//!
//! They are disabled-looking-but-present rather than hidden: the configuration is kept whether or
//! not the sandbox is selected (see [`DaemonDraft`]), so hiding it would make a user believe it
//! had been discarded, and moving the keyboard's focus target in and out of existence as a select
//! changes is its own defect.
//!
//! [`DaemonDraft`]: crate::features::settings::DaemonDraft

use crate::app::Message;
use crate::features::sandbox::SandboxLimit;
use crate::features::session::CliAvailability;
use crate::features::settings::Msg as SettingsMsg;
use crate::features::settings::{missing_cli_notice, SettingsDraft, SettingsSection};
use crate::features::window::FieldId;
use crate::ui::focus::TrackFocus;
use crate::ui::material::{Checkbox, Select, TextField};
use crate::ui::settings::{caution, field_note, group, name_of, note, page, Named};
use iced::Element;
use micold_core::sandbox::image::ImageSourceKind;
use micold_core::sandbox::placement::PlacementKind;
use micold_core::sandbox::runtime::{LimitSupport, RuntimeKind};
use micold_core::sandbox::{CredentialShare, NetworkPosture};
use micold_core::tokens::Roles;

/// What this section renders. See [`crate::ui::settings`].
///
/// `daemon.sandbox.image` is one persisted field with three controls — the acquisition path, the
/// reference and the archive — so it is declared once, against the control that chooses between
/// them.
// Read by `tests/settings_sections.rs`, which is a separate crate and cannot be seen from here —
// so to the compiler this is unused. Deleting it would take the gate's evidence with it.
#[allow(dead_code)]
pub const SETTINGS: &[(&str, &str)] = &[
    ("daemon.placement", "PlacementChanged"),
    ("daemon.sandbox.runtime", "RuntimeChanged"),
    ("daemon.sandbox.image", "ImageKindChanged"),
    ("daemon.sandbox.credentials", "CredentialToggled"),
    ("daemon.sandbox.survive_logout", "SurviveLogoutToggled"),
    // One persisted field with four controls, declared once against the first of them — the same
    // shape as `daemon.sandbox.image` above.
    ("daemon.sandbox.budget", "CpuLimitChanged"),
    ("daemon.sandbox.network", "NetworkChanged"),
];

const PLACEMENTS: &[Named<PlacementKind>] = &[
    Named(PlacementKind::HostProcess, "On this computer"),
    Named(PlacementKind::LocalSandbox, "In a container"),
];

const RUNTIMES: &[Named<RuntimeKind>] = &[
    Named(RuntimeKind::Docker, "Docker"),
    Named(RuntimeKind::Podman, "Podman"),
];

const NETWORKS: &[Named<NetworkPosture>] = &[
    Named(NetworkPosture::NoOutbound, "No outbound connections"),
    Named(NetworkPosture::Outbound, "Allow outbound connections"),
];

const IMAGE_SOURCES: &[Named<ImageSourceKind>] = &[
    Named(ImageSourceKind::Registry, "Pull from a registry"),
    Named(ImageSourceKind::ImportedFile, "Load from a file"),
    Named(ImageSourceKind::LocalBuild, "Build from this checkout"),
];

/// What the user has shared, named one by one (FR-004c, rule N-2).
///
/// "3 credentials shared" is the summary this deliberately is not. A count tells a user that
/// something is shared without telling them *what*, which is the one thing they need to decide
/// whether they meant it; and the set is ordered, so the sentence is stable between renders.
fn sharing_summary(draft: &SettingsDraft) -> String {
    let shared: Vec<&str> = draft
        .shared_credentials()
        .iter()
        .map(|c| c.label())
        .collect();
    format!("Shared with the container: {}.", shared.join(", "))
}

/// What the user is told at the moment they choose to cut the sandbox off (FR-018).
///
/// # Why this is a warning and not just a description
///
/// The AI CLI talks to a hosted provider. Blocking outbound connections is the *safe* choice and
/// the default for everything else the sandbox does — and it is also the choice that stops the
/// feature this application exists for from working at all. A user who picks it to be careful and
/// then finds their agent failing has no reason to connect the two: the setting says "network",
/// the failure looks like the AI being broken.
///
/// So it is said here, at the moment of the change, rather than left to a documentation page or to
/// an error later on.
pub const NO_OUTBOUND_WARNING: &str =
    "The AI agent reaches its provider over the network. With outbound connections blocked it \
     cannot sign in or answer, and neither can anything a session tries to fetch. The control \
     channel to this application is unaffected.";

/// The warning to show beside the network picker, if the current choice warrants one.
///
/// A function rather than an `if` inside the view for the same reason [`limits`] is one: a view
/// function is the one thing in this crate a test cannot look inside, and "the warning appears
/// exactly when outbound is blocked" is the whole of FR-018.
pub fn network_warning(draft: &SettingsDraft) -> Option<&'static str> {
    match draft.daemon.profile.network {
        NetworkPosture::NoOutbound => Some(NO_OUTBOUND_WARNING),
        NetworkPosture::Outbound => None,
    }
}

/// One resource limit, as the section is about to draw it.
///
/// Separated from the drawing because this is the whole of what T086 asks: *which* limits appear
/// (all of them, always), which are editable, and what a disabled one says instead. A view
/// function is the one place in this crate a test cannot look inside, so the decision is made here
/// and the rendering below is a transcription of it.
pub struct LimitControl<'a> {
    /// The control the keyboard and any error message address.
    pub field: FieldId,
    /// Its name in the form.
    pub label: &'static str,
    /// What the user has typed, or what was stored. Shown whether or not the field is editable —
    /// a limit the runtime cannot enforce is still a limit the user set, and blanking it would
    /// look like the setting had been discarded.
    pub value: &'a str,
    /// The line under the field: the unit and what empty means, or the runtime's own reason.
    pub supporting: String,
    /// The message typing emits. Present only while the field is editable, which is how
    /// [`TextField`] expresses unavailability — there is nowhere to send the value.
    pub on_input: Option<fn(String) -> Message>,
}

impl LimitControl<'_> {
    /// Whether the user can type in it.
    pub fn editable(&self) -> bool {
        self.on_input.is_some()
    }
}

/// The four limits, in the order the section shows them (FR-012 … FR-015).
///
/// # Why an unsupported limit is disabled rather than hidden
///
/// FR-015 asks for three things at once and they pull against each other: the limit must not be
/// silently accepted (the user would believe a bound exists that does not), must not be hidden
/// (a setting that vanishes on one machine and reappears on another is a setting the user cannot
/// reason about, and the stored value is still there), and must say *why*. A disabled field
/// carrying the runtime's own reason is the only arrangement that does all three.
///
/// Capabilities are `Option` because "not probed yet" is a real state — see
/// [`DaemonDraft::capabilities`]. Unknown means editable: the form must not invent a restriction
/// it has not been told about.
///
/// [`DaemonDraft::capabilities`]: crate::features::settings::DaemonDraft::capabilities
pub fn limits(draft: &SettingsDraft) -> [LimitControl<'_>; 4] {
    let caps = draft.daemon.capabilities.as_ref();
    let d = &draft.daemon;
    [
        // Three of the four labels come from `SandboxLimit`, because a session stopped by one of
        // them is reported by naming its setting (T088) and a report that used a different name
        // than the form would send the user hunting. The processor limit has no `SandboxLimit`
        // variant — a CPU share throttles rather than stopping anything — so it names itself.
        limit(
            FieldId::SettingsCpuLimit,
            "Processor limit",
            &d.cpus,
            "Cores, e.g. 2 or 1.5. Empty leaves the runtime's own default.",
            caps.map(|c| &c.cpus),
            |v| Message::Settings(SettingsMsg::CpuLimitChanged(v)),
        ),
        limit(
            FieldId::SettingsMemoryLimit,
            SandboxLimit::Memory.setting(),
            &d.memory_mib,
            "Mebibytes. Empty leaves the runtime's own default.",
            caps.map(|c| &c.memory),
            |v| Message::Settings(SettingsMsg::MemoryLimitChanged(v)),
        ),
        limit(
            FieldId::SettingsPidLimit,
            SandboxLimit::Processes.setting(),
            &d.pids,
            "Processes the sandbox may run at once. Empty leaves the runtime's own default.",
            caps.map(|c| &c.pids),
            |v| Message::Settings(SettingsMsg::PidLimitChanged(v)),
        ),
        limit(
            FieldId::SettingsStorageLimit,
            SandboxLimit::Storage.setting(),
            &d.storage_mib,
            "Mebibytes the sandbox may write. Not enforceable on every storage driver.",
            caps.map(|c| &c.storage),
            |v| Message::Settings(SettingsMsg::StorageLimitChanged(v)),
        ),
    ]
}

fn limit<'a>(
    field: FieldId,
    label: &'static str,
    value: &'a str,
    hint: &'static str,
    support: Option<&LimitSupport>,
    on_input: fn(String) -> Message,
) -> LimitControl<'a> {
    match support {
        Some(LimitSupport::Unsupported { reason }) => LimitControl {
            field,
            label,
            value,
            supporting: format!("Cannot be enforced here: {reason}"),
            on_input: None,
        },
        _ => LimitControl {
            field,
            label,
            value,
            supporting: hint.to_string(),
            on_input: Some(on_input),
        },
    }
}

/// The Session service page.
pub fn view<'a>(
    draft: &'a SettingsDraft,
    availability: Option<&'a CliAvailability>,
    focused: Option<FieldId>,
    roles: Roles,
) -> Element<'a, Message> {
    let placement = Select::new(
        PLACEMENTS,
        Some(Named(
            draft.daemon.placement,
            name_of(PLACEMENTS, draft.daemon.placement),
        )),
        |chosen: Named<PlacementKind>| Message::Settings(SettingsMsg::PlacementChanged(chosen.0)),
        roles,
    )
    .label("Where sessions run")
    .supporting("Takes effect the next time the application starts");

    let runtime = Select::new(
        RUNTIMES,
        Some(Named(
            draft.daemon.profile.runtime,
            name_of(RUNTIMES, draft.daemon.profile.runtime),
        )),
        |chosen: Named<RuntimeKind>| Message::Settings(SettingsMsg::RuntimeChanged(chosen.0)),
        roles,
    )
    .label("Container runtime");

    let image_kind = Select::new(
        IMAGE_SOURCES,
        Some(Named(
            draft.daemon.profile.image.kind,
            name_of(IMAGE_SOURCES, draft.daemon.profile.image.kind),
        )),
        |chosen: Named<ImageSourceKind>| Message::Settings(SettingsMsg::ImageKindChanged(chosen.0)),
        roles,
    )
    .label("Image source")
    .supporting(match draft.daemon.profile.image.kind {
        ImageSourceKind::Registry => "Needs the network the first time only",
        ImageSourceKind::ImportedFile => "Works with no network at all",
        ImageSourceKind::LocalBuild => "Built by `mise run image` from this working tree",
    });

    let reference = TextField::new("", &draft.daemon.profile.image.reference, roles)
        .label("Image reference")
        .supporting("A digest or an exact tag; a moving tag cannot be reported in a bug")
        .error(super::error_for(
            draft,
            SettingsSection::Daemon,
            FieldId::SettingsImageReference,
        ))
        .track_focus(FieldId::SettingsImageReference, focused)
        .on_input(|v| Message::Settings(SettingsMsg::ImageReferenceChanged(v)))
        .on_submit(Message::Settings(SettingsMsg::Saved));

    let archive = TextField::new("", &draft.daemon.image_path, roles)
        .label("Image file")
        .supporting("The archive to load, when the image comes from a file")
        .error(super::error_for(
            draft,
            SettingsSection::Daemon,
            FieldId::SettingsImagePath,
        ))
        .track_focus(FieldId::SettingsImagePath, focused)
        .on_input(|v| Message::Settings(SettingsMsg::ImagePathChanged(v)))
        .on_submit(Message::Settings(SettingsMsg::Saved));

    let survive = Checkbox::new(
        "Keep sessions running after I sign out",
        draft.daemon.profile.survive_logout,
        roles,
    )
    .track_focus(FieldId::SettingsSurviveLogout, focused)
    .on_toggle(|v| Message::Settings(SettingsMsg::SurviveLogoutToggled(v)));

    let mut controls: Vec<Element<'a, Message>> = vec![
        placement.into(),
        note(
            format!(
                "Currently {}.",
                name_of(PLACEMENTS, draft.daemon.placement).to_lowercase()
            ),
            roles,
        ),
        group("Container", roles),
        runtime.into(),
        image_kind.into(),
    ];

    // FR-023b, at the point the image is chosen. The published image ships every AI CLI (FR-023a);
    // a substituted one inherits that obligation and nothing can make it keep it, so the only
    // honest thing to do is say which CLI the image running *now* does not provide. It sits
    // directly under the reference rather than at the foot of the section, because it is a fact
    // about that field's value.
    //
    // The image it names is the one the service was actually started from, not the one in this
    // field: the field is a draft and may say something the running container has never heard of.
    // Naming the draft's value would describe a machine that does not exist yet.
    //
    // Attached to the reference field rather than stacked after it, so it shares the column that
    // field's own supporting line sits in. See `field_note`.
    controls.push(field_note(
        reference,
        missing_cli_notice(availability),
        roles,
    ));

    controls.extend([
        archive.into(),
        group("Credentials", roles),
        note(
            "The container starts with none of your credentials. Share only what the agent needs.",
            roles,
        ),
    ]);

    for share in CredentialShare::ALL {
        let on = draft.shared_credentials().contains(&share);
        controls.push(
            Checkbox::new(share.label(), on, roles)
                .track_focus(FieldId::SettingsCredential(share), focused)
                .on_toggle(move |checked| {
                    Message::Settings(SettingsMsg::CredentialToggled(share, checked))
                })
                .into(),
        );
    }

    if draft.shares_credentials() {
        controls.push(caution(sharing_summary(draft), roles));
    }

    let network = Select::new(
        NETWORKS,
        Some(Named(
            draft.daemon.profile.network,
            name_of(NETWORKS, draft.daemon.profile.network),
        )),
        |chosen: Named<NetworkPosture>| Message::Settings(SettingsMsg::NetworkChanged(chosen.0)),
        roles,
    )
    .label("Network");

    controls.push(group("Network", roles));
    controls.push(network.into());
    if let Some(warning) = network_warning(draft) {
        controls.push(caution(warning, roles));
    }
    controls.push(note(
        "Names still resolve either way — the runtime's resolver answers from the host side. \
         Blocked means connections fail, not that nothing leaves the container.",
        roles,
    ));

    controls.push(group("Limits", roles));
    controls.push(note(
        "What the sandbox may consume. A limit the runtime cannot enforce is shown, and says so, \
         rather than being quietly dropped.",
        roles,
    ));
    for l in limits(draft) {
        let mut input = TextField::new("", l.value, roles)
            .label(l.label)
            .supporting(l.supporting)
            .error(super::error_for(draft, SettingsSection::Daemon, l.field))
            .track_focus(l.field, focused);
        if let Some(on_input) = l.on_input {
            input = input
                .on_input(on_input)
                .on_submit(Message::Settings(SettingsMsg::Saved));
        }
        controls.push(input.into());
    }

    controls.push(group("Sessions", roles));
    controls.push(survive.into());
    // FR-014d: where the configured placement cannot provide this, the control says so. It is not
    // hidden and it is not left to do nothing — either would leave the user believing they had
    // arranged something.
    let (support, cannot) = survival_support(draft.daemon.placement);
    controls.push(if cannot {
        caution(support, roles)
    } else {
        note(support, roles)
    });

    page(
        "Session service",
        "Where your sessions run, and what that service can reach.",
        controls,
        roles,
    )
}

/// What to say beneath the survival opt-in, and whether it is a caution (feature 027, FR-014d).
///
/// A free function rather than a `match` inside the view, because the claim FR-014d makes is about
/// *every* placement — "where the configured placement cannot provide it, the control must say so"
/// — and a rule stated over a set is only checkable if it can be enumerated. In the view it could
/// only be read.
///
/// No `cfg!` since feature 028. There used to be a Linux branch here promising that the systemd
/// user manager kept the service alive after sign-out; that promise rested on the units and the
/// per-user enablement this feature removes (FR-005, packaging contract §4.11), so it is now the
/// same answer on all three platforms — and one answer is one thing to keep true.
fn survival_support(placement: PlacementKind) -> (&'static str, bool) {
    match placement {
        // The container runtime's restart policy is run by a service the platform keeps alive
        // across logout and reboot, on all three platforms — so this is the one that always works
        // (FR-014b). It is applied at container creation, which is why it names the next start.
        PlacementKind::LocalSandbox => (
            "The container is created with a restart policy, so this takes effect the next time \
             the sandbox starts.",
            false,
        ),
        // Feature 028: the application is the only thing that starts a session service (lifecycle
        // contract §1.1), and nothing it starts outlives the session it was started from. Sessions
        // are still safe — they come back resumable — which is why this says what is lost rather
        // than only that something is.
        PlacementKind::HostProcess => (
            "A service running directly on this computer can't outlive signing out. Your sessions \
             are kept and come back resumable; run the service in a container to keep them \
             running through a sign-out.",
            true,
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// FR-014d, stated over the whole set: no placement leaves this control unexplained, and the
    /// one that cannot honour the opt-in is the one that warns.
    #[test]
    fn every_placement_says_what_it_will_do() {
        for Named(placement, label) in PLACEMENTS {
            let (support, cannot) = survival_support(*placement);
            assert!(!support.is_empty(), "{label} explains nothing");
            match placement {
                // The sandbox can always do it — that is FR-014b's whole point — so a caution here
                // would be telling the user the opposite of the truth.
                PlacementKind::LocalSandbox => assert!(!cannot, "{label} warned when it can do it"),
                // Feature 028, FR-005, deliberately un-`cfg`-ed. This assertion used to read
                // `!cfg!(target_os = "linux")`, and a platform-gated replacement would let the
                // removed promise creep back on the one platform that ever made it.
                PlacementKind::HostProcess => {
                    assert!(
                        cannot,
                        "{label} must warn: nothing it starts outlives sign-out"
                    );
                    assert!(
                        !support.contains("systemd"),
                        "the host mechanism is gone; the control must not name it: {support}"
                    );
                }
            }
        }
    }

    /// The warning has to name the way out, or it is only a refusal. A user told "not here" and
    /// nothing else has no reason to look at the placement select two groups above.
    #[test]
    fn the_warning_names_the_placement_that_does_support_it() {
        let (support, cannot) = survival_support(PlacementKind::HostProcess);
        assert!(cannot);
        assert!(support.contains("container"));
        // And what is *not* lost, or the warning reads as "your work goes away" (FR-005).
        assert!(support.contains("resumable"));
    }
}
