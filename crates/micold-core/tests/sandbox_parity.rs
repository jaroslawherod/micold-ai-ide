//! What the sandbox must not cost (feature 027, US2 — FR-011, FR-014, Principle III).
//!
//! Isolation is only adoptable if it is free of regressions: a sandbox that costs the product's
//! existing guarantees is not a feature anyone keeps switched on. So the guarantees are asserted
//! here as properties of the mount set and the argument vector, where they can be checked without a
//! runtime — the behaviour they enable is checked against a real one in `evidence/`.

use std::path::{Path, PathBuf};

use micold_core::sandbox::argv;
use micold_core::sandbox::dialect::Dialect;
use micold_core::sandbox::image::ImageSource;
use micold_core::sandbox::runtime::{
    IdentityMapping, LimitSupport, RuntimeCapabilities, RuntimeKind,
};
use micold_core::sandbox::{
    CredentialLayout, MountSet, SandboxProfile, SandboxSpec, SecretMount, STATE_CONTAINER_DIR,
};

const PROJECT: &str = "/home/u/projects/micold";
const STATE_DIR: &str = "/home/u/.local/share/micold-ai-ide";

fn mounts(profile: &SandboxProfile) -> MountSet {
    MountSet::build(
        &[PathBuf::from(PROJECT)],
        profile,
        &CredentialLayout::conventional(Path::new("/home/u"), None),
        PathBuf::from(STATE_DIR),
        Path::new("/home/u"),
        SecretMount {
            host: PathBuf::from("/run/user/1000/micold/sandbox.token"),
            container: PathBuf::from("/run/micold/token"),
        },
    )
}

fn spec(profile: &SandboxProfile) -> SandboxSpec {
    SandboxSpec {
        name: "micold-sandbox".into(),
        profile: profile.clone(),
        mounts: mounts(profile),
        uid: 1000,
        gid: 1000,
        control_port: 7727,
        published_ports: Vec::new(),
        network_name: "micold-sandbox-net".into(),
        home: PathBuf::from("/home/u"),
    }
}

fn caps() -> RuntimeCapabilities {
    RuntimeCapabilities {
        kind: RuntimeKind::Docker,
        version: "29.5.1".into(),
        cpus: LimitSupport::Supported,
        memory: LimitSupport::Supported,
        pids: LimitSupport::Supported,
        storage: LimitSupport::Supported,
        identity_mapping: IdentityMapping::ExplicitUidGid,
    }
}

fn argv_strings(spec: &SandboxSpec) -> Vec<String> {
    argv::create(spec, &caps())
        .iter()
        .map(|a| a.to_string_lossy().into_owned())
        .collect()
}

/// FR-011: the daemon's state outlives the container.
///
/// It is a bind mount of the host's own data directory rather than a runtime-managed volume — see
/// `StateMount`'s doc comment for why. What matters for parity is the property, which either shape
/// would satisfy: destroying and recreating the container cannot lose `projects.json`.
#[test]
fn daemon_state_is_mounted_from_a_place_the_container_does_not_own() {
    let profile = SandboxProfile::default();
    let set = mounts(&profile);
    assert_eq!(set.state.host, PathBuf::from(STATE_DIR));
    assert_eq!(set.state.container, PathBuf::from(STATE_CONTAINER_DIR));

    let args = argv_strings(&spec(&profile));
    assert!(
        args.iter()
            .any(|a| a == &format!("{STATE_DIR}:{STATE_CONTAINER_DIR}:rw")),
        "state must be mounted read-write from the host: {args:?}"
    );
}

/// FR-011, the property stated as the cycle it protects: two independently built argument vectors
/// for the same profile mount the same state, so `create → remove → create` cannot land somewhere
/// else and quietly start empty.
#[test]
fn recreating_the_container_mounts_the_same_state() {
    let profile = SandboxProfile::default();
    let first = argv_strings(&spec(&profile));
    let second = argv_strings(&spec(&profile));
    assert_eq!(first, second);
}

/// Principle III: worktrees live under `<project>/.claude/worktrees/`, which is **inside** the
/// mounted project — so a worktree created in the sandbox is on the host, at the same path, with no
/// extra mount and no translation.
///
/// This is the payoff of research R2's identical-path decision, and it is worth asserting rather
/// than assuming: if the project were mounted anywhere else, git's worktree metadata would name a
/// path the host cannot resolve, and every worktree made in the sandbox would be broken outside it.
#[test]
fn worktrees_land_inside_the_mounted_project_at_the_same_path() {
    let profile = SandboxProfile::default();
    let set = mounts(&profile);
    let project = &set.projects[0];

    let worktrees_root = micold_core::worktree::worktrees_root(Path::new(PROJECT));
    assert!(
        worktrees_root.starts_with(&project.host),
        "{worktrees_root:?} must be inside the mounted project"
    );

    if cfg!(not(windows)) {
        // The same absolute path on both sides, which is what makes the metadata agree.
        assert_eq!(project.host, project.container);
        assert!(worktrees_root.starts_with(&project.container));
    }

    // And the project is writable, or a worktree could not be created at all.
    assert!(project.writable);
}

/// FR-014: sessions outlive the client. The container is created **detached** and its lifetime is
/// not tied to the process that created it — asserted as the absence of the flags that would tie
/// them, since a passing test that checked for `--detach` would say nothing about `--rm`.
#[test]
fn the_container_does_not_die_with_the_client() {
    let args = argv_strings(&spec(&SandboxProfile::default()));
    assert!(
        !args.iter().any(|a| a == "--rm"),
        "--rm would destroy the sandbox, and every session in it, when it stops: {args:?}"
    );
    assert!(
        !args.iter().any(|a| a == "-a" || a == "--attach"),
        "the sandbox must not be attached to the client's streams: {args:?}"
    );
}

/// US2 scenario 8: a service a session starts is reachable from the host on the port the user asked
/// to expose — and the control channel is published regardless, whatever the network posture.
#[test]
fn user_exposed_ports_and_the_control_port_are_both_published_to_loopback() {
    let profile = SandboxProfile::default();
    let mut s = spec(&profile);
    s.published_ports = vec![3000, 8080];
    let args = argv_strings(&s);

    for expected in [
        "127.0.0.1:7727:7727",
        "127.0.0.1:3000:3000",
        "127.0.0.1:8080:8080",
    ] {
        assert!(
            args.iter().any(|a| a == expected),
            "missing {expected}: {args:?}"
        );
    }
    // Loopback, never every interface: publishing terminal traffic on the network is not something
    // any part of this feature asks for.
    assert!(!args.iter().any(|a| a.starts_with("0.0.0.0")));
}

/// FR-014a/b: survival is the runtime's restart policy, and it is chosen from the same setting on
/// every platform. No `cfg` here on purpose — that is the point of raising the bar.
#[test]
fn survival_maps_to_the_restart_policy_identically_on_every_platform() {
    for (survive, expected) in [(true, "unless-stopped"), (false, "no")] {
        let profile = SandboxProfile {
            survive_logout: survive,
            ..SandboxProfile::default()
        };
        let args = argv_strings(&spec(&profile));
        let i = args
            .iter()
            .position(|a| a == "--restart")
            .expect("a restart policy");
        assert_eq!(args[i + 1], expected);
    }
}

/// `unless-stopped`, not `always`: a user who explicitly stops the sandbox stays stopped across a
/// reboot. `always` would resurrect it and read as the application ignoring them.
#[test]
fn an_explicit_stop_survives_a_reboot() {
    let profile = SandboxProfile {
        survive_logout: true,
        ..SandboxProfile::default()
    };
    let args = argv_strings(&spec(&profile));
    assert!(!args.iter().any(|a| a == "always"));
}

/// The image reference is the last argument, so everything after it would be the container's own
/// command. Nothing is: the image's entrypoint runs the daemon, and appending a command here would
/// silently replace it.
#[test]
fn nothing_follows_the_image_reference() {
    let profile = SandboxProfile {
        image: ImageSource {
            reference: "micold-daemon:0.27.0".into(),
            ..ImageSource::default()
        },
        ..SandboxProfile::default()
    };
    let args = argv_strings(&spec(&profile));
    assert_eq!(
        args.last().map(String::as_str),
        Some("micold-daemon:0.27.0")
    );
}

/// Both dialects express the same parity guarantees. "The runtime is replaceable" has to mean the
/// promises are too, or the second runtime is a different product.
#[test]
fn both_dialects_preserve_the_same_guarantees() {
    for kind in RuntimeKind::ALL {
        let profile = SandboxProfile {
            survive_logout: true,
            ..SandboxProfile::default()
        };
        let s = spec(&profile);
        let caps = RuntimeCapabilities {
            kind,
            identity_mapping: Dialect::for_kind(kind).identity,
            ..caps()
        };
        let args: Vec<String> = argv::create(&s, &caps)
            .iter()
            .map(|a| a.to_string_lossy().into_owned())
            .collect();

        assert!(args.iter().any(|a| a == "unless-stopped"), "{kind}");
        assert!(args.iter().any(|a| a == "127.0.0.1:7727:7727"), "{kind}");
        assert!(
            args.iter()
                .any(|a| a == &format!("{STATE_DIR}:{STATE_CONTAINER_DIR}:rw")),
            "{kind}"
        );
        assert!(!args.iter().any(|a| a == "--rm"), "{kind}");
    }
}
