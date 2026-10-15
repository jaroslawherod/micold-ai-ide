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

use super::container_fields::{self, ContainerView, FieldErrors, Surface};
use crate::app::Message;
use crate::features::session::CliAvailability;
use crate::features::settings::Msg as SettingsMsg;
use crate::features::settings::{
    missing_cli_notice, placement_note, SettingsDraft, SettingsSection,
};
use crate::features::window::FieldId;
use crate::ui::material::Select;
use crate::ui::settings::{name_of, note, page, Named};
use iced::Element;
use micold_core::sandbox::placement::PlacementKind;
use micold_core::tokens::Roles;

pub use super::container_fields::{LimitControl, NO_OUTBOUND_WARNING};

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

/// The fields' view of the page's draft.
fn container_view(draft: &SettingsDraft) -> ContainerView<'_> {
    let d = &draft.daemon;
    ContainerView {
        profile: &d.profile,
        image_path: &d.image_path,
        cpus: &d.cpus,
        memory_mib: &d.memory_mib,
        pids: &d.pids,
        storage_mib: &d.storage_mib,
        capabilities: d.capabilities.as_ref(),
        unshared_sign_in: d.unshared_sign_in.as_deref(),
        placement: d.placement,
    }
}

/// The warning to show beside the network picker, if the current choice warrants one.
pub fn network_warning(draft: &SettingsDraft) -> Option<&'static str> {
    container_fields::network_warning(&container_view(draft))
}

/// The four limits, in the order the page shows them.
pub fn limits(draft: &SettingsDraft) -> [LimitControl<'_>; 4] {
    container_fields::limits(&container_view(draft), Surface::Page)
}

/// The Session service page.
pub fn view<'a>(
    draft: &'a SettingsDraft,
    in_force: PlacementKind,
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
    .label("Where sessions run");

    let error = |field| super::error_for(draft, SettingsSection::Daemon, field);
    let errors = FieldErrors {
        reference: error(FieldId::SettingsImageReference),
        path: error(FieldId::SettingsImagePath),
        limits: [
            error(FieldId::SettingsCpuLimit),
            error(FieldId::SettingsMemoryLimit),
            error(FieldId::SettingsPidLimit),
            error(FieldId::SettingsStorageLimit),
        ],
    };

    let mut controls: Vec<Element<'a, Message>> = vec![
        placement.into(),
        // The placement in force, not the draft's (BUG-003, FR-035b). Built from the draft, this
        // line changed the instant the select did and so said "currently in a container" about a
        // container that did not exist.
        note(placement_note(in_force), roles),
    ];
    // FR-023b names the CLI the image running *now* does not provide, not the draft's value.
    controls.extend(container_fields::groups(
        &container_view(draft),
        Surface::Page,
        errors,
        missing_cli_notice(availability),
        focused,
        roles,
    ));

    page(
        "Session service",
        "Where your sessions run, and what that service can reach.",
        controls,
        roles,
    )
}
