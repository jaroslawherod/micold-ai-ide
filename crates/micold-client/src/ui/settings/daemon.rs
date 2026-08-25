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

use crate::app::{FieldId, Message};
use crate::features::settings::{SettingsDraft, SettingsSection};
use crate::ui::focus::TrackFocus;
use crate::ui::material::{Checkbox, Select, TextField};
use crate::ui::settings::{caution, group, name_of, note, page, Named};
use iced::Element;
use micold_core::sandbox::image::ImageSourceKind;
use micold_core::sandbox::placement::PlacementKind;
use micold_core::sandbox::runtime::RuntimeKind;
use micold_core::sandbox::CredentialShare;
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
    ("daemon.placement", "SettingsPlacementChanged"),
    ("daemon.sandbox.runtime", "SettingsRuntimeChanged"),
    ("daemon.sandbox.image", "SettingsImageKindChanged"),
    ("daemon.sandbox.credentials", "SettingsCredentialToggled"),
    (
        "daemon.sandbox.survive_logout",
        "SettingsSurviveLogoutToggled",
    ),
];

const PLACEMENTS: &[Named<PlacementKind>] = &[
    Named(PlacementKind::HostProcess, "On this computer"),
    Named(PlacementKind::LocalSandbox, "In a container"),
];

const RUNTIMES: &[Named<RuntimeKind>] = &[
    Named(RuntimeKind::Docker, "Docker"),
    Named(RuntimeKind::Podman, "Podman"),
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

/// The Session service page.
pub fn view<'a>(
    draft: &'a SettingsDraft,
    focused: Option<FieldId>,
    roles: Roles,
) -> Element<'a, Message> {
    let placement = Select::new(
        PLACEMENTS,
        Some(Named(draft.daemon.placement, "")),
        |chosen: Named<PlacementKind>| Message::SettingsPlacementChanged(chosen.0),
        roles,
    )
    .label("Where sessions run")
    .supporting("Takes effect the next time the application starts");

    let runtime = Select::new(
        RUNTIMES,
        Some(Named(draft.daemon.profile.runtime, "")),
        |chosen: Named<RuntimeKind>| Message::SettingsRuntimeChanged(chosen.0),
        roles,
    )
    .label("Container runtime");

    let image_kind = Select::new(
        IMAGE_SOURCES,
        Some(Named(draft.daemon.profile.image.kind, "")),
        |chosen: Named<ImageSourceKind>| Message::SettingsImageKindChanged(chosen.0),
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
        .on_input(Message::SettingsImageReferenceChanged)
        .on_submit(Message::SettingsSaved);

    let archive = TextField::new("", &draft.daemon.image_path, roles)
        .label("Image file")
        .supporting("The archive to load, when the image comes from a file")
        .error(super::error_for(
            draft,
            SettingsSection::Daemon,
            FieldId::SettingsImagePath,
        ))
        .track_focus(FieldId::SettingsImagePath, focused)
        .on_input(Message::SettingsImagePathChanged)
        .on_submit(Message::SettingsSaved);

    let survive = Checkbox::new(
        "Keep sessions running after I sign out",
        draft.daemon.profile.survive_logout,
        roles,
    )
    .track_focus(FieldId::SettingsSurviveLogout, focused)
    .on_toggle(Message::SettingsSurviveLogoutToggled);

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
        reference.into(),
        archive.into(),
        group("Credentials", roles),
        note(
            "The container starts with none of your credentials. Share only what the agent needs.",
            roles,
        ),
    ];

    for share in CredentialShare::ALL {
        let on = draft.shared_credentials().contains(&share);
        controls.push(
            Checkbox::new(share.label(), on, roles)
                .track_focus(FieldId::SettingsCredential(share), focused)
                .on_toggle(move |checked| Message::SettingsCredentialToggled(share, checked))
                .into(),
        );
    }

    if draft.shares_credentials() {
        controls.push(caution(sharing_summary(draft), roles));
    }

    controls.push(group("Sessions", roles));
    controls.push(survive.into());

    page(
        "Session service",
        "Where your sessions run, and what that service can reach.",
        controls,
        roles,
    )
}
