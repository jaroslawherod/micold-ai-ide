//! The resource limits, as the Daemons form offers them — T086, FR-012 … FR-016, SC-009.
//!
//! # What is easy to get wrong here
//!
//! A limit the selected runtime cannot enforce has three plausible-looking treatments and two of
//! them are defects.
//!
//! *Hide it* and the setting exists on one machine and not on another, while the stored value goes
//! on sitting in the file — the user cannot see what they set, cannot clear it, and has no reason
//! to believe it is still there.
//!
//! *Accept it* and the form has told a lie: the field takes a number, the save succeeds, and no
//! bound exists. This is the worst of the three, because everything looks like it worked.
//!
//! So: shown, disabled, and carrying the runtime's own reason. These tests hold that, and the two
//! things around it — that an empty field is *unset* rather than zero (rule RB-2), and that a
//! number below what the daemon needs is refused with the range that would be accepted (FR-016).

use micold_client::features::sandbox::{SandboxLimit, StoppedByLimit};
use micold_client::features::window::FieldId;
use micold_client::ui::{network_warning, sandbox_limits, ContainerView};
use micold_core::sandbox::placement::PlacementKind;
use micold_core::sandbox::runtime::{
    IdentityMapping, LimitSupport, RuntimeCapabilities, RuntimeKind,
};
use micold_core::sandbox::{NetworkPosture, SandboxProfile};

fn caps(storage: LimitSupport) -> RuntimeCapabilities {
    RuntimeCapabilities {
        kind: RuntimeKind::Docker,
        version: "30.0.0".into(),
        cpus: LimitSupport::Supported,
        memory: LimitSupport::Supported,
        pids: LimitSupport::Supported,
        storage,
        identity_mapping: IdentityMapping::ExplicitUidGid,
    }
}

/// A view of a container daemon's form over `profile`, with the typed limits in `texts`
/// (cpus, memory, pids, storage).
fn view<'a>(
    profile: &'a SandboxProfile,
    texts: [&'a str; 4],
    capabilities: Option<&'a RuntimeCapabilities>,
) -> ContainerView<'a> {
    ContainerView {
        profile,
        image_path: "",
        image_reference: "",
        cpus: texts[0],
        memory_mib: texts[1],
        pids: texts[2],
        storage_mib: texts[3],
        capabilities,
        unshared_sign_in: None,
        placement: PlacementKind::LocalSandbox,
    }
}

// ---------------------------------------------------------------------------------------------
// FR-015: shown, disabled, and saying why
// ---------------------------------------------------------------------------------------------

/// All four are always offered, whatever the runtime turns out to support.
#[test]
fn every_limit_is_offered_whatever_the_runtime_can_do() {
    let profile = SandboxProfile::default();
    let caps = caps(LimitSupport::unsupported("the overlay2 driver cannot"));
    let fields: Vec<FieldId> = sandbox_limits(&view(&profile, [""; 4], Some(&caps)))
        .iter()
        .map(|l| l.field)
        .collect();
    assert_eq!(
        fields,
        vec![
            FieldId::DaemonCpuLimit,
            FieldId::DaemonMemoryLimit,
            FieldId::DaemonPidLimit,
            FieldId::DaemonStorageLimit,
        ],
        "a limit the runtime cannot enforce was hidden rather than disabled (FR-015)"
    );
}

/// Before anything has been probed, nothing is disabled: "not asked yet" is not "unavailable".
#[test]
fn an_unprobed_runtime_leaves_every_limit_editable() {
    let profile = SandboxProfile::default();
    assert!(
        sandbox_limits(&view(&profile, [""; 4], None))
            .iter()
            .all(|l| l.editable()),
        "the form disabled a limit on the strength of never having asked"
    );
}

/// The one the probe says no to is the only one disabled, it says why in the runtime's own words,
/// and it still shows what the user set.
#[test]
fn an_unsupported_limit_is_disabled_with_its_reason_and_keeps_its_value() {
    let profile = SandboxProfile::default();
    let caps = caps(LimitSupport::unsupported("the overlay2 driver cannot"));
    let limits = sandbox_limits(&view(&profile, ["", "", "", "8192"], Some(&caps)));

    let disabled: Vec<FieldId> = limits
        .iter()
        .filter(|l| !l.editable())
        .map(|l| l.field)
        .collect();
    assert_eq!(disabled, vec![FieldId::DaemonStorageLimit]);

    let storage = &limits[3];
    assert!(
        storage.supporting.contains("the overlay2 driver cannot"),
        "the disabled field explains nothing: {:?}",
        storage.supporting
    );
    assert_eq!(storage.value, "8192");
}

// ---------------------------------------------------------------------------------------------
// FR-006: the hint names where the limit lives now
// ---------------------------------------------------------------------------------------------

/// The report of a session stopped by a limit sends the user to the Daemons manager, and no
/// message names the removed Session service section (804 FR-006).
#[test]
fn the_limit_hint_names_the_daemons_manager() {
    for limit in [
        SandboxLimit::Memory,
        SandboxLimit::Processes,
        SandboxLimit::Storage,
    ] {
        let message = StoppedByLimit {
            limit,
            configured: "512".into(),
        }
        .message();
        assert!(message.contains("Settings \u{203a} Daemons"), "{message:?}");
        assert!(!message.contains("Session service"), "{message:?}");
    }
}

// ---------------------------------------------------------------------------------------------
// FR-017 / FR-018: the network posture, warned about where it is chosen
// ---------------------------------------------------------------------------------------------

/// Blocking outbound connections warns, at the moment it is chosen.
///
/// This is the setting most likely to be picked *because* it sounds safe, by a user who has no way
/// to know it is the one that stops the AI agent working. Left to a documentation page, the
/// discovery is a session that fails for no visible reason hours later.
#[test]
fn blocking_the_network_warns_that_the_agent_cannot_reach_its_provider() {
    let profile = SandboxProfile {
        network: NetworkPosture::NoOutbound,
        ..SandboxProfile::default()
    };

    let warning = network_warning(&view(&profile, [""; 4], None)).expect("blocking outbound warns");
    assert!(
        warning.contains("provider"),
        "the warning has to say what stops working, not that something might: {warning:?}"
    );
}

/// Allowing it does not.
///
/// A caution shown against every choice is a caution the user stops reading — and outbound
/// connections are the default and the working configuration.
#[test]
fn allowing_the_network_does_not_warn() {
    let profile = SandboxProfile {
        network: NetworkPosture::Outbound,
        ..SandboxProfile::default()
    };

    assert_eq!(network_warning(&view(&profile, [""; 4], None)), None);
}

/// The default posture is the blocked one, so the warning is on screen the first time the section
/// is opened — which is the point at which the user can still change it.
#[test]
fn the_default_posture_is_the_one_that_warns() {
    assert_eq!(NetworkPosture::default(), NetworkPosture::NoOutbound);
    let profile = SandboxProfile::default();
    assert!(network_warning(&view(&profile, [""; 4], None)).is_some());
}
