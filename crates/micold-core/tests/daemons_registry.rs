//! Feature 491 (FR-001, FR-015): the rules a daemon registry enforces on every change.

use micold_core::daemons::{
    ContainerSettings, DaemonId, DaemonRegistry, DaemonRuntime, RegistryError,
};
use micold_core::sandbox::SandboxProfile;

fn container(name: &str, port: u16) -> DaemonRuntime {
    DaemonRuntime::Container(ContainerSettings {
        profile: SandboxProfile::default(),
        container_name: name.to_string(),
        port,
    })
}

fn registry() -> DaemonRegistry {
    DaemonRegistry::new(Vec::new(), 1)
}

#[test]
fn a_name_is_trimmed_before_it_is_stored() {
    let mut r = registry();
    let id = r.add("  Untrusted  ", container("c1", 7728), false).unwrap();
    assert_eq!(
        r.get(id).unwrap().name.as_str(),
        "Untrusted",
        "names are stored trimmed"
    );
}

#[test]
fn a_blank_name_is_refused() {
    let mut r = registry();
    assert_eq!(
        r.add("   ", DaemonRuntime::Host, true),
        Err(RegistryError::BlankName),
        "a name that is empty after trimming is refused"
    );
    assert!(r.entries().is_empty(), "a refused add changes nothing");
}

#[test]
fn a_name_equal_ignoring_case_is_refused() {
    let mut r = registry();
    r.add("Untrusted", container("c1", 7728), false).unwrap();
    assert_eq!(
        r.add("untrusted ", container("c2", 7729), false),
        Err(RegistryError::DuplicateName),
        "names are unique case-insensitively, after trimming"
    );
}

#[test]
fn ids_are_never_reused_after_removal() {
    let mut r = registry();
    let first = r.add("X", container("c1", 7728), false).unwrap();
    r.remove(first).expect("present");
    let second = r.add("X", container("c1", 7728), false).unwrap();
    assert_ne!(first, second, "a re-added name must not inherit the old id");
    assert_eq!(second, DaemonId(first.0 + 1), "ids count up from the counter");
}

#[test]
fn renaming_keeps_the_id() {
    let mut r = registry();
    let id = r.add("Old", container("c1", 7728), false).unwrap();
    r.edit(id, "New", container("c1", 7728), true).unwrap();
    let e = r.get(id).unwrap();
    assert_eq!((e.name.as_str(), e.auto_start), ("New", true), "edit applies");
    assert_eq!(r.entries().len(), 1, "an edit never adds an entry");
}

#[test]
fn a_container_name_or_port_in_use_is_refused_naming_the_holder() {
    let mut r = registry();
    let holder = r.add("A", container("shared", 7728), false).unwrap();
    assert_eq!(
        r.add("B", container("shared", 7729), false),
        Err(RegistryError::ContainerInUse { other: holder }),
        "one container name belongs to one daemon"
    );
    assert_eq!(
        r.add("C", container("other", 7728), false),
        Err(RegistryError::ContainerInUse { other: holder }),
        "one port belongs to one daemon"
    );
}

#[test]
fn an_unusable_container_field_is_refused_naming_the_field() {
    let mut r = registry();
    let mut bad_image = ContainerSettings {
        profile: SandboxProfile::default(),
        container_name: "c1".into(),
        port: 7728,
    };
    bad_image.profile.image.reference = String::new();
    assert_eq!(
        r.add("A", DaemonRuntime::Container(bad_image), false),
        Err(RegistryError::InvalidField { field: "image" }),
        "an empty image reference is refused"
    );
    assert_eq!(
        r.add("A", container("c1", 0), false),
        Err(RegistryError::InvalidField { field: "port" }),
        "port 0 is out of range"
    );
    assert_eq!(
        r.add("A", container("  ", 7728), false),
        Err(RegistryError::InvalidField {
            field: "container_name"
        }),
        "a blank container name is refused"
    );
}

#[test]
fn a_second_host_daemon_is_refused() {
    let mut r = registry();
    r.add("Host", DaemonRuntime::Host, true).unwrap();
    assert_eq!(
        r.add("Host 2", DaemonRuntime::Host, true),
        Err(RegistryError::HostExists),
        "the host daemon is a per-user singleton"
    );
}

#[test]
fn the_runtime_label_is_the_plain_value() {
    assert_eq!(DaemonRuntime::Host.label(), "Host");
    assert_eq!(container("c", 7728).label(), "Container");
}
