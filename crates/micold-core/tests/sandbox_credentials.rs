//! What the sandbox can and cannot see (feature 027, FR-004a/b/c, FR-006, FR-007).
//!
//! The load-bearing rule of the whole feature is rule M-1: only what the mount set names is
//! mounted. Everything else in this specification is configuration or presentation on top of it, so
//! this file is where a regression would matter most and where it would be least visible — a
//! sandbox that quietly mounts one extra directory still starts, still runs sessions, and still
//! looks exactly like a working sandbox.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use micold_core::sandbox::{
    CredentialLayout, CredentialShare, MountSet, SandboxProfile, SecretMount,
};

fn layout() -> CredentialLayout {
    CredentialLayout::conventional(Path::new("/home/u"), Some(Path::new("/run/user/1000/ssh")))
}

fn secret() -> SecretMount {
    SecretMount {
        host: PathBuf::from("/run/user/1000/micold/sandbox.token"),
        container: PathBuf::from("/run/micold/token"),
    }
}

fn build(profile: &SandboxProfile) -> MountSet {
    MountSet::build(
        &[PathBuf::from("/home/u/projects/micold")],
        profile,
        &layout(),
        std::path::PathBuf::from("/home/u/.local/share/micold-ai-ide"),
        secret(),
    )
}

/// FR-004a: the default shares nothing. The one default in this feature that is a security property
/// rather than a convenience — an upgrade must never opt anyone in.
#[test]
fn the_default_profile_mounts_no_credentials() {
    let mounts = build(&SandboxProfile::default());
    assert!(mounts.credentials.is_empty());

    // And the host paths it *can* reach are exactly the project, the daemon's own state
    // directory, and the token. Three, and no fourth: the count is the assertion, because a
    // convenience mount added later would pass every other check in this file.
    let paths: Vec<String> = mounts
        .host_paths()
        .iter()
        .map(|p| p.display().to_string())
        .collect();
    assert_eq!(paths.len(), 3, "reachable host paths: {paths:?}");
    assert!(paths.iter().any(|p| p.ends_with("projects/micold")));
    assert!(paths.iter().any(|p| p.ends_with("micold-ai-ide")));
    assert!(paths.iter().any(|p| p.ends_with("sandbox.token")));
}

/// FR-004b: each opt-in adds exactly its own mount and no other. Checked one at a time, because an
/// opt-in that pulled in a second path would be invisible when they are all enabled together.
#[test]
fn each_opt_in_adds_exactly_one_mount() {
    for share in CredentialShare::ALL {
        let profile = SandboxProfile {
            credentials: BTreeSet::from([share]),
            ..SandboxProfile::default()
        };
        let mounts = build(&profile);
        assert_eq!(
            mounts.credentials.len(),
            1,
            "{share:?} added more than itself"
        );
        assert_eq!(mounts.credentials[0].share, share);
    }
}

/// FR-004c: the view renders what is shared *from the set*, so each active share must be
/// individually identifiable rather than collapsed into a count.
#[test]
fn every_active_share_is_individually_identifiable() {
    let profile = SandboxProfile {
        credentials: BTreeSet::from(CredentialShare::ALL),
        ..SandboxProfile::default()
    };
    let mounts = build(&profile);

    let shared: BTreeSet<CredentialShare> = mounts.credentials.iter().map(|c| c.share).collect();
    assert_eq!(shared, BTreeSet::from(CredentialShare::ALL));
    for c in &mounts.credentials {
        assert!(
            !c.share.label().is_empty(),
            "{:?} has no label to show",
            c.share
        );
    }
}

/// An opt-in whose path the host layout does not know is skipped, not substituted.
///
/// The edge case the spec names: a credential opt-in enabled while the item it shares is absent —
/// no authentication agent running, or the socket it named has gone. Mounting *something* nearby
/// would be worse than mounting nothing, because the user would believe the opt-in worked.
#[test]
fn an_opt_in_with_no_known_path_is_skipped_rather_than_substituted() {
    let profile = SandboxProfile {
        credentials: BTreeSet::from([CredentialShare::SshAgent]),
        ..SandboxProfile::default()
    };
    let mounts = MountSet::build(
        &[],
        &profile,
        // No agent socket: the user has one enabled, and there is nothing to enable it with.
        &CredentialLayout::conventional(Path::new("/home/u"), None),
        std::path::PathBuf::from("/home/u/.local/share/micold-ai-ide"),
        secret(),
    );
    assert!(mounts.credentials.is_empty());
}

/// FR-006/FR-007: the mount set holds registered projects and nothing near them.
#[test]
fn only_registered_projects_are_mounted() {
    let profile = SandboxProfile::default();
    let mounts = MountSet::build(
        &[
            PathBuf::from("/home/u/projects/a"),
            PathBuf::from("/home/u/projects/b"),
        ],
        &profile,
        &layout(),
        std::path::PathBuf::from("/home/u/.local/share/micold-ai-ide"),
        secret(),
    );
    assert_eq!(mounts.projects.len(), 2);

    // Not the parent that contains them both, and not the home directory above it. Mounting either
    // would be convenient and would defeat the feature.
    let hosts: Vec<String> = mounts
        .projects
        .iter()
        .map(|p| p.host.display().to_string())
        .collect();
    assert!(!hosts.iter().any(|h| h == "/home/u"));
    assert!(!hosts.iter().any(|h| h == "/home/u/projects"));
}

/// The runtime's own control socket is never in the set. A sandbox that can drive its own runtime
/// can start an unconfined container, which is the whole boundary undone in one command.
#[test]
fn the_runtimes_control_socket_is_never_reachable() {
    let profile = SandboxProfile {
        credentials: BTreeSet::from(CredentialShare::ALL),
        ..SandboxProfile::default()
    };
    let mounts = build(&profile);
    for path in mounts.host_paths() {
        let p = path.display().to_string();
        assert!(!p.contains("docker.sock"), "{p} is reachable");
        assert!(!p.contains("podman.sock"), "{p} is reachable");
    }
}

/// Credentials keep their own absolute paths inside the container, for the same reason projects do:
/// the tools that read them look where they always are.
#[test]
fn credentials_appear_at_the_paths_their_tools_expect() {
    let profile = SandboxProfile {
        credentials: BTreeSet::from([CredentialShare::GitConfig]),
        ..SandboxProfile::default()
    };
    let mounts = build(&profile);
    let c = &mounts.credentials[0];
    if cfg!(not(windows)) {
        assert_eq!(c.host, c.container);
        assert!(c.container.ends_with(".gitconfig"));
    }
}

/// Daemon state is the host's own data directory, bind-mounted.
///
/// It is deliberately *not* a runtime-managed volume: the client has to read `projects.json` to
/// know what to mount before the sandbox exists, and inside a volume that file is unreachable from
/// the host. See `StateMount`'s doc comment for why this satisfies FR-011 anyway.
#[test]
fn daemon_state_is_the_hosts_own_data_directory() {
    let mounts = build(&SandboxProfile::default());
    assert!(mounts.state.host.ends_with("micold-ai-ide"));
    assert_eq!(
        mounts.state.container,
        std::path::PathBuf::from(micold_core::sandbox::STATE_CONTAINER_DIR)
    );
    // It is reachable, and it must be: that is the point.
    assert!(mounts
        .host_paths()
        .iter()
        .any(|p| p.ends_with("micold-ai-ide")));
}
