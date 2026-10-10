//! Feature 491 (FR-017, R3): two container daemons share nothing on disk or in the runtime.

use std::path::{Path, PathBuf};

use micold_core::daemons::{ContainerSettings, DaemonEntry, DaemonId, DaemonName, DaemonRuntime};
use micold_core::sandbox::{
    CredentialLayout, MountSet, SandboxProfile, SecretMount, CONTAINER_NAME,
};

fn entry(id: u32, name: &str, container: &str, port: u16) -> DaemonEntry {
    DaemonEntry {
        id: DaemonId(id),
        name: DaemonName::new(name).unwrap(),
        runtime: DaemonRuntime::Container(ContainerSettings {
            profile: SandboxProfile::default(),
            container_name: container.into(),
            port,
        }),
        auto_start: false,
    }
}

fn mounts(state_dir: PathBuf, entry: &DaemonEntry) -> MountSet {
    let token = entry.token_path(&state_dir, None).expect("container has a token");
    MountSet::build(
        &[PathBuf::from("/work/p")],
        &SandboxProfile::default(),
        &CredentialLayout::default(),
        state_dir,
        Path::new("/home/u"),
        SecretMount {
            host: token,
            container: PathBuf::from("/run/micold/token"),
        },
    )
}

#[test]
fn two_container_daemons_get_disjoint_state_tokens_and_names() {
    let base = Path::new("/state");
    let a = entry(2, "Alpha", "micold-sandbox-alpha", 7728);
    let b = entry(3, "Beta", "micold-sandbox-beta", 7729);
    let (da, db) = (
        a.state_dir(base, None).unwrap(),
        b.state_dir(base, None).unwrap(),
    );
    assert_ne!(da, db, "each daemon has its own state directory");
    assert!(!da.starts_with(&db) && !db.starts_with(&da), "and they do not nest");
    assert!(da.starts_with(base.join("daemons")), "under the daemons directory");
    let (ma, mb) = (mounts(da.clone(), &a), mounts(db.clone(), &b));
    assert_ne!(ma.state.host, mb.state.host, "disjoint state mounts");
    assert_ne!(ma.home.host, mb.home.host, "disjoint homes");
    assert_ne!(ma.secret.host, mb.secret.host, "distinct tokens");
    assert_ne!(a.container_name(), b.container_name(), "distinct container names");
}

#[test]
fn the_migrated_daemon_keeps_the_legacy_name_port_and_token_path() {
    let base = Path::new("/state");
    let migrated = entry(1, "Container", CONTAINER_NAME, 7727);
    assert_eq!(
        migrated.state_dir(base, Some(DaemonId(1))).unwrap(),
        base,
        "the legacy daemon keeps the legacy state directory so its container is adopted"
    );
    assert_eq!(
        migrated.token_path(base, Some(DaemonId(1))).unwrap(),
        micold_core::protocol::auth::host_token_path(base),
        "and the legacy token location"
    );
    assert_eq!(migrated.container_name(), Some(CONTAINER_NAME));
}

#[test]
fn a_runtime_resolves_to_a_placement_without_the_global_choice() {
    use micold_core::sandbox::placement::Placement;
    let c = entry(2, "A", "c", 7728);
    assert!(matches!(
        Placement::from_runtime(&c.runtime),
        Some(Placement::LocalSandbox(_))
    ));
    assert!(matches!(
        Placement::from_runtime(&DaemonRuntime::Host),
        Some(Placement::HostProcess)
    ));
    let unsupported = DaemonRuntime::Unsupported {
        kind: "ssh".into(),
        raw: serde_json::json!({"kind":"ssh"}),
    };
    assert!(Placement::from_runtime(&unsupported).is_none(), "cannot start");
}
