//! The resource limits, as the Settings form offers them — T086, FR-012 … FR-016, SC-009.
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

use micold_client::app::FieldId;
use micold_client::features::settings::{SettingsDraft, SettingsSection};
use micold_client::ui::sandbox_limits;
use micold_core::sandbox::runtime::{
    IdentityMapping, LimitSupport, RuntimeCapabilities, RuntimeKind,
};
use micold_core::sandbox::{
    Bytes, MilliCpus, MIN_MEMORY, MIN_MILLI_CPUS, MIN_PIDS, MIN_STORAGE,
};

/// A draft whose other sections validate, so a case about the limits fails for the limits.
fn draft() -> SettingsDraft {
    let mut draft = SettingsDraft::default();
    draft.terminal.scrollback_lines = "5000".into();
    draft.environment.timeout_secs = "5".into();
    draft
}

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

// ---------------------------------------------------------------------------------------------
// FR-015: shown, disabled, and saying why
// ---------------------------------------------------------------------------------------------

/// All four are always offered, whatever the runtime turns out to support.
#[test]
fn every_limit_is_offered_whatever_the_runtime_can_do() {
    let mut d = draft();
    d.daemon.capabilities = Some(caps(LimitSupport::unsupported("the overlay2 driver cannot")));

    let fields: Vec<FieldId> = sandbox_limits(&d).iter().map(|l| l.field).collect();
    assert_eq!(
        fields,
        vec![
            FieldId::SettingsCpuLimit,
            FieldId::SettingsMemoryLimit,
            FieldId::SettingsPidLimit,
            FieldId::SettingsStorageLimit,
        ],
        "a limit the runtime cannot enforce was hidden rather than disabled — the stored value is \
         then unreachable and invisible (FR-015)"
    );
}

/// Before anything has been probed, nothing is disabled.
///
/// "Not asked yet" is not "unavailable". An application that has never run the probe and greys the
/// controls anyway has invented a restriction it was never told about, and the user's only way out
/// is to guess that starting the sandbox once will change the form.
#[test]
fn an_unprobed_runtime_leaves_every_limit_editable() {
    let d = draft();
    assert!(d.daemon.capabilities.is_none(), "the fixture must be unprobed");

    assert!(
        sandbox_limits(&d).iter().all(|l| l.editable()),
        "the form disabled a limit on the strength of never having asked"
    );
}

/// The one the probe says no to is the one that stops being editable.
#[test]
fn an_unsupported_limit_is_the_only_one_disabled() {
    let mut d = draft();
    d.daemon.capabilities = Some(caps(LimitSupport::unsupported("the overlay2 driver cannot")));

    let limits = sandbox_limits(&d);
    let disabled: Vec<FieldId> = limits
        .iter()
        .filter(|l| !l.editable())
        .map(|l| l.field)
        .collect();
    assert_eq!(disabled, vec![FieldId::SettingsStorageLimit]);
}

/// And it says *why*, in the runtime's own words.
///
/// "Not available" alone leaves the user with nothing to act on. The probe knows the reason; it is
/// carried through `LimitSupport::Unsupported` precisely so this line can show it.
#[test]
fn a_disabled_limit_carries_the_runtimes_reason() {
    let mut d = draft();
    d.daemon.capabilities = Some(caps(LimitSupport::unsupported("the overlay2 driver cannot")));

    let limits = sandbox_limits(&d);
    let storage = limits
        .iter()
        .find(|l| l.field == FieldId::SettingsStorageLimit)
        .expect("the storage limit is offered");
    assert!(
        storage.supporting.contains("the overlay2 driver cannot"),
        "the disabled field explains nothing: {:?}",
        storage.supporting
    );
}

/// A disabled limit still shows what the user set.
///
/// The value is in the file either way. Blanking the field would read as the setting having been
/// discarded by moving to this machine — and moving back would then look like it had come back
/// from nowhere.
#[test]
fn a_disabled_limit_still_shows_the_stored_value() {
    let mut d = draft();
    d.daemon.storage_mib = "8192".into();
    d.daemon.capabilities = Some(caps(LimitSupport::unsupported("the overlay2 driver cannot")));

    let limits = sandbox_limits(&d);
    let storage = limits
        .iter()
        .find(|l| l.field == FieldId::SettingsStorageLimit)
        .expect("the storage limit is offered");
    assert_eq!(storage.value, "8192");
}

// ---------------------------------------------------------------------------------------------
// Rule RB-2: unset is not zero
// ---------------------------------------------------------------------------------------------

/// An empty field saves as *unset*, which is how the user says "leave the runtime's default".
#[test]
fn an_empty_limit_saves_as_unset_rather_than_as_zero() {
    let mut d = draft();
    d.daemon.cpus = String::new();
    d.daemon.memory_mib = "  ".into();
    d.daemon.pids = String::new();
    d.daemon.storage_mib = String::new();

    let budget = d.validate().expect("an empty limit is not a failure").daemon.sandbox.budget;
    assert_eq!(budget.cpus_milli, None);
    assert_eq!(budget.memory_bytes, None);
    assert_eq!(budget.pids, None);
    assert_eq!(budget.storage_bytes, None);
}

/// And a filled one round-trips through the form in the units the form shows.
#[test]
fn the_limits_round_trip_through_the_form() {
    let mut d = draft();
    d.daemon.cpus = "1.5".into();
    d.daemon.memory_mib = "2048".into();
    d.daemon.pids = "256".into();
    d.daemon.storage_mib = "4096".into();

    let settings = d.validate().expect("in range").into_settings();
    let budget = settings.daemon.sandbox.budget;
    assert_eq!(budget.cpus_milli, Some(MilliCpus(1500)));
    assert_eq!(budget.memory_bytes, Some(Bytes::from_mib(2048)));
    assert_eq!(budget.pids, Some(256));
    assert_eq!(budget.storage_bytes, Some(Bytes::from_mib(4096)));

    let reopened = SettingsDraft::from_settings(&settings);
    assert_eq!(reopened.daemon.cpus, "1.5", "a core count grew decimals it was not typed with");
    assert_eq!(reopened.daemon.memory_mib, "2048");
    assert_eq!(reopened.daemon.pids, "256");
    assert_eq!(reopened.daemon.storage_mib, "4096");
}

/// A whole number of cores comes back whole.
#[test]
fn a_whole_core_count_has_no_decimal_point() {
    let mut d = draft();
    d.daemon.cpus = "2".into();

    let settings = d.validate().expect("in range").into_settings();
    assert_eq!(settings.daemon.sandbox.budget.cpus_milli, Some(MilliCpus(2000)));
    assert_eq!(SettingsDraft::from_settings(&settings).daemon.cpus, "2");
}

// ---------------------------------------------------------------------------------------------
// FR-016: refused, naming the range
// ---------------------------------------------------------------------------------------------

/// Each limit set below what the daemon needs is refused, pointing at its own field and naming the
/// accepted range.
#[test]
fn a_limit_below_the_workable_minimum_is_refused_by_its_own_field() {
    let cases: [(&str, FieldId, String); 4] = [
        ("cpus", FieldId::SettingsCpuLimit, "0.1".into()),
        ("memory_mib", FieldId::SettingsMemoryLimit, (MIN_MEMORY.as_mib() - 1).to_string()),
        ("pids", FieldId::SettingsPidLimit, (MIN_PIDS - 1).to_string()),
        ("storage_mib", FieldId::SettingsStorageLimit, (MIN_STORAGE.as_mib() - 1).to_string()),
    ];

    for (name, field, value) in cases {
        let mut d = draft();
        match name {
            "cpus" => d.daemon.cpus = value,
            "memory_mib" => d.daemon.memory_mib = value,
            "pids" => d.daemon.pids = value,
            _ => d.daemon.storage_mib = value,
        }

        let error = d.validate().expect_err("a limit below the minimum is refused");
        assert_eq!(error.field, field, "{name} was reported against the wrong control");
        assert_eq!(
            error.section,
            SettingsSection::Daemon,
            "the rejection has to name the section holding the field, or the user is shown a \
             message about a control they cannot see (FR-029)"
        );

        let minimum = match field {
            FieldId::SettingsCpuLimit => format!("{}", f64::from(MIN_MILLI_CPUS.0) / 1000.0),
            FieldId::SettingsMemoryLimit => MIN_MEMORY.as_mib().to_string(),
            FieldId::SettingsPidLimit => MIN_PIDS.to_string(),
            _ => MIN_STORAGE.as_mib().to_string(),
        };
        assert!(
            error.message.contains(&minimum),
            "{name}: the refusal must name the accepted range (FR-016), and says {:?}",
            error.message
        );
    }
}

/// Something that is not a number at all gets its own message, not the range one.
///
/// "Enter between 512 and 1048576 MiB" in answer to `abc` reads as though `abc` were a number out
/// of range, and sends the user hunting for a bound rather than a typo.
#[test]
fn text_that_is_not_a_number_says_so() {
    let mut d = draft();
    d.daemon.memory_mib = "lots".into();

    let error = d.validate().expect_err("`lots` is not a memory limit");
    assert_eq!(error.field, FieldId::SettingsMemoryLimit);
    assert!(
        !error.message.contains(&MIN_MEMORY.as_mib().to_string()),
        "a parse failure was reported as a range failure: {:?}",
        error.message
    );
}

/// A rejected limit never reaches the profile.
///
/// `validate` builds the whole `Settings` at once, so this is really a statement that it is
/// all-or-nothing: a form with a good CPU limit and a bad memory limit must save neither.
#[test]
fn a_rejected_form_saves_nothing() {
    let mut d = draft();
    d.daemon.cpus = "4".into();
    d.daemon.memory_mib = "1".into();

    assert!(d.validate().is_err());
    assert_eq!(
        d.daemon.cpus, "4",
        "validation edited the draft; it reports, and the form keeps what was typed"
    );
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
    let mut d = draft();
    d.daemon.profile.network = micold_core::sandbox::NetworkPosture::NoOutbound;

    let warning = micold_client::ui::network_warning(&d).expect("blocking outbound warns");
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
    let mut d = draft();
    d.daemon.profile.network = micold_core::sandbox::NetworkPosture::Outbound;

    assert_eq!(micold_client::ui::network_warning(&d), None);
}

/// The default posture is the blocked one, so the warning is on screen the first time the section
/// is opened — which is the point at which the user can still change it.
#[test]
fn the_default_posture_is_the_one_that_warns() {
    assert_eq!(
        micold_core::sandbox::NetworkPosture::default(),
        micold_core::sandbox::NetworkPosture::NoOutbound
    );
    assert!(micold_client::ui::network_warning(&draft()).is_some());
}
