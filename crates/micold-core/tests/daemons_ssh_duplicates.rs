//! Feature 489, T057 (FR-016): a second remote daemon for one machine and account is refused,
//! naming the existing one; blank or invalid fields are refused naming the field.

use micold_core::daemons::{DaemonRegistry, DaemonRuntime, RegistryError};
use micold_core::ssh::config_hosts::parse_g;
use micold_core::ssh::{SshSettings, SshTarget};

fn ssh(host: &str, user: Option<&str>, port: Option<u16>) -> DaemonRuntime {
    DaemonRuntime::Ssh(SshSettings {
        target: SshTarget::new(host, user, port, None).unwrap(),
    })
}

#[test]
fn a_second_daemon_for_the_same_host_user_and_port_is_refused_naming_the_first() {
    let mut r = DaemonRegistry::default();
    let first = r
        .add("One", ssh("box", Some("me"), Some(22)), false)
        .unwrap();
    assert_eq!(
        r.add("Two", ssh("BOX", Some("me"), None), false),
        Err(RegistryError::SshDuplicate { other: first }),
        "host case and the default port do not make a different machine"
    );
}

#[test]
fn a_different_user_or_port_is_a_different_daemon() {
    let mut r = DaemonRegistry::default();
    r.add("One", ssh("box", Some("me"), None), false).unwrap();
    assert!(r.add("Two", ssh("box", Some("you"), None), false).is_ok());
    assert!(r
        .add("Three", ssh("box", Some("me"), Some(2222)), false)
        .is_ok());
}

#[test]
fn two_aliases_of_one_machine_are_refused_when_ssh_resolves_them_alike() {
    let resolve = |t: &SshTarget| {
        let out = if t.host == "a" || t.host == "b" {
            "hostname 10.0.0.1\nuser me\nport 22\n"
        } else {
            "hostname other\n"
        };
        parse_g(out).identity()
    };
    let mut r = DaemonRegistry::default();
    let first = r
        .add_resolved("One", ssh("a", None, None), false, &resolve)
        .unwrap();
    assert_eq!(
        r.add_resolved("Two", ssh("b", None, None), false, &resolve),
        Err(RegistryError::SshDuplicate { other: first })
    );
}

#[test]
fn editing_a_daemon_to_its_own_target_is_not_a_duplicate_but_to_another_is() {
    let mut r = DaemonRegistry::default();
    let one = r.add("One", ssh("a", None, None), false).unwrap();
    let two = r.add("Two", ssh("b", None, None), false).unwrap();
    assert!(r.edit(one, "One", ssh("a", None, None), true).is_ok());
    assert_eq!(
        r.edit(two, "Two", ssh("a", None, None), false),
        Err(RegistryError::SshDuplicate { other: one })
    );
}

#[test]
fn invalid_fields_are_refused_naming_the_field() {
    for (host, user, port, field) in [
        ("", None, None, "host"),
        ("-x", None, None, "host"),
        ("h", Some("a@b"), None, "user"),
        ("h", None, Some(0), "port"),
    ] {
        assert_eq!(
            SshTarget::new(host, user, port, None),
            Err(RegistryError::InvalidField { field })
        );
    }
}
