//! T114 — the Windows path mapping, exercised on whatever platform CI happens to run.
//!
//! The mapping itself has unit tests in `sandbox::pathmap`. What was untested is everything
//! *downstream* of it: the mount set that assembles the mapped paths, and the `-v` flags `argv`
//! renders from that mount set. Those two steps ran only under `cfg!(windows)`, which no runner in
//! this project's CI provides — so the code path that makes Windows different from Linux was
//! compiled by nobody and asserted by nothing.
//!
//! `ProjectMount::project_for` and `MountSet::build_for` take the platform as a value for exactly
//! this reason, and this file drives them both ways. Where a claim holds on both platforms it is
//! asserted on both, so a change that quietly makes one of them special fails here.

use micold_core::sandbox::argv;
use micold_core::sandbox::image::ImageSource;
use micold_core::sandbox::pathmap::{self, WINDOWS_MOUNT_ROOT};
use micold_core::sandbox::placement::{GitRouting, Placement};
use micold_core::sandbox::runtime::{
    IdentityMapping, LimitSupport, RuntimeCapabilities, RuntimeKind,
};
use micold_core::sandbox::{
    CredentialLayout, CredentialShare, MountSet, SandboxProfile, SandboxSpec, SecretMount,
};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

/// A Windows host, spelled the way Windows spells it: drive letter, backslashes.
const WIN_PROJECT: &str = r"C:\Users\u\code\thing";
const WIN_HOME: &str = r"C:\Users\u";
const WIN_STATE: &str = r"C:\Users\u\AppData\Roaming\micold-ai-ide";
const WIN_TOKEN: &str = r"C:\Users\u\AppData\Roaming\micold-ai-ide\token";

const NIX_PROJECT: &str = "/home/u/code/thing";
const NIX_HOME: &str = "/home/u";
const NIX_STATE: &str = "/home/u/.local/share/micold-ai-ide";
const NIX_TOKEN: &str = "/run/user/1000/micold/token";

fn caps() -> RuntimeCapabilities {
    RuntimeCapabilities {
        kind: RuntimeKind::Docker,
        version: "29.5.1".into(),
        cpus: LimitSupport::Supported,
        memory: LimitSupport::Supported,
        pids: LimitSupport::Supported,
        storage: LimitSupport::Unsupported {
            reason: "rootful overlay2 does not enforce a per-container size".into(),
        },
        identity_mapping: IdentityMapping::ExplicitUidGid,
    }
}

/// A spec for one project, with every credential opted in, under a named platform's mapping.
///
/// All four credentials are shared deliberately: they are the mounts whose *host* paths come from
/// the user's home directory, so they are where a Windows path would leak into a container path if
/// the mapping were skipped for anything but projects.
fn spec_for(windows_host: bool) -> SandboxSpec {
    let (project, home, state, token) = if windows_host {
        (WIN_PROJECT, WIN_HOME, WIN_STATE, WIN_TOKEN)
    } else {
        (NIX_PROJECT, NIX_HOME, NIX_STATE, NIX_TOKEN)
    };
    let profile = SandboxProfile {
        image: ImageSource {
            reference: "micold-daemon:0.27.0".into(),
            ..ImageSource::default()
        },
        credentials: BTreeSet::from([
            CredentialShare::GitConfig,
            CredentialShare::GitCredentials,
            CredentialShare::AiCliAuth,
        ]),
        ..SandboxProfile::default()
    };
    let layout = CredentialLayout::conventional(Path::new(home), None);
    let mounts = MountSet::build_for(
        &[PathBuf::from(project)],
        &profile,
        &layout,
        state,
        SecretMount {
            host: PathBuf::from(token),
            container: PathBuf::from("/run/micold/token"),
        },
        windows_host,
    );
    SandboxSpec {
        name: "micold-sandbox".into(),
        profile,
        mounts,
        uid: 1000,
        gid: 1000,
        control_port: 7727,
        published_ports: Vec::new(),
        network_name: "micold-net".into(),
        home: PathBuf::from(home),
    }
}

/// Every `host:container:mode` triple in the rendered argv, split back apart.
///
/// Split from the *right*, because a Windows host path contains a colon — `C:\Users\...` — and
/// splitting from the left would hand back `C` as the host and the rest as the container. That is
/// not a hypothetical: it is the shape of bug this whole file exists to catch, and a parser here
/// that got it wrong would hide it.
fn volumes(args: &[std::ffi::OsString]) -> Vec<(String, String, String)> {
    let rendered: Vec<String> = args
        .iter()
        .map(|a| a.to_string_lossy().into_owned())
        .collect();
    rendered
        .windows(2)
        .filter(|w| w[0] == "-v")
        .map(|w| {
            let (rest, mode) = w[1].rsplit_once(':').expect("a mount carries a mode");
            let (host, container) = rest.rsplit_once(':').expect("a mount names both sides");
            (host.to_string(), container.to_string(), mode.to_string())
        })
        .collect()
}

/// The mounts whose container path comes from [`pathmap`], which is not all of them.
///
/// The state directory and the token are mounted at *fixed* container paths — `/var/lib/...` and
/// `/run/micold/token` — because they are where the daemon inside the image looks, on every host.
/// So they are never an identity mount, not even on Linux, and a test that demanded they were
/// would be asserting the wrong invariant about the right code.
fn mapped_volumes(spec: &SandboxSpec, args: &[std::ffi::OsString]) -> Vec<(String, String, String)> {
    let fixed = [
        spec.mounts.state.host.to_string_lossy().into_owned(),
        spec.mounts.secret.host.to_string_lossy().into_owned(),
    ];
    volumes(args)
        .into_iter()
        .filter(|(host, _, _)| !fixed.contains(host))
        .collect()
}

/// The claim: on Windows the two halves of every mount differ, and the container half is Linux.
///
/// A `-v` whose container half still said `C:\Users\u\code\thing` would be rejected by the runtime
/// outright — but one whose container half was silently the *host* half, because the mapping was
/// applied to projects and forgotten for credentials, starts a container the daemon then cannot
/// find `~/.gitconfig` in. This asserts the whole set, not the project.
#[test]
fn a_windows_host_mounts_every_path_under_the_container_root() {
    let spec = spec_for(true);
    let mounts = volumes(&argv::create(&spec, &caps()));
    assert!(!mounts.is_empty(), "the spec has mounts");

    for (host, container, _mode) in &mounts {
        assert!(
            !container.contains('\\') && !container.contains(':'),
            "a container path must be a Linux path, got {container:?} (host {host:?})"
        );
        assert!(
            container.starts_with('/'),
            "a container path must be absolute, got {container:?}"
        );
    }

    // The project and the three credentials all come from the host's filesystem, so all four
    // land under the container root. The state and token mounts do not: they have fixed container
    // paths the image dictates.
    let mapped = mapped_volumes(&spec, &argv::create(&spec, &caps()));
    assert_eq!(mapped.len(), 4, "one project and three credentials: {mapped:?}");
    assert!(
        mapped.iter().all(|(_, c, _)| c.starts_with(WINDOWS_MOUNT_ROOT)),
        "every mapped path belongs under {WINDOWS_MOUNT_ROOT}: {mapped:?}"
    );

    let project = mounts
        .iter()
        .find(|(h, _, _)| h == WIN_PROJECT)
        .expect("the project is mounted under its own host path");
    assert_eq!(project.1, "/mnt/host/c/Users/u/code/thing");
    assert_eq!(project.2, "rw", "a project is writable; that is the point");
}

/// The host half stays the host's own spelling. It is the runtime on Windows that reads it.
#[test]
fn a_windows_host_path_reaches_the_argv_unrewritten() {
    let mounts = volumes(&argv::create(&spec_for(true), &caps()));
    assert!(
        mounts.iter().any(|(h, _, _)| h == WIN_PROJECT),
        "the host half must be the path Windows gave us, backslashes and all: {mounts:?}"
    );
}

/// Rule M-1 is not relaxed by the mapping: the argv still mounts nothing outside the mount set.
///
/// The existing conformance check runs this on the platform it happens to be built for. Under the
/// Windows mapping the two halves of a mount differ, which is precisely the condition under which a
/// containment check written against the wrong half would pass for the wrong reason.
#[test]
fn rule_m1_holds_under_both_mappings() {
    for windows_host in [false, true] {
        let spec = spec_for(windows_host);
        let allowed: Vec<&Path> = spec.mounts.host_paths();
        for (host, _, _) in volumes(&argv::create(&spec, &caps())) {
            assert!(
                allowed.iter().any(|p| p.to_string_lossy() == host),
                "argv mounted {host:?}, which is not in the mount set (windows_host={windows_host})"
            );
        }
        assert_eq!(
            volumes(&argv::create(&spec, &caps())).len(),
            allowed.len(),
            "argv must mount the whole set and nothing else (windows_host={windows_host})"
        );
    }
}

/// On Linux and macOS the mapping is the identity, and the argv shows it.
#[test]
fn a_unix_host_mounts_every_path_at_itself() {
    let spec = spec_for(false);
    let mapped = mapped_volumes(&spec, &argv::create(&spec, &caps()));
    assert_eq!(mapped.len(), 4, "one project and three credentials: {mapped:?}");
    for (host, container, _) in mapped {
        assert_eq!(
            host, container,
            "the identity mapping must reach the argv as an identity"
        );
    }
}

/// The seam T113 turns on: git routes to the daemon exactly when the two halves differ.
///
/// These are two independent pieces of code — `pathmap::map_for` builds the mounts,
/// `Placement::git_routing_for` decides who answers the open-project gate — and the feature is
/// correct only while they agree. Asserted against the *rendered argv* rather than against
/// `is_identity_for`, so this fails if the mount set ever stops honouring the mapping the routing
/// decision is made from.
#[test]
fn git_routes_to_the_daemon_exactly_when_the_mounted_paths_differ() {
    for windows_host in [false, true] {
        let spec = spec_for(windows_host);
        let sandbox = Placement::LocalSandbox(Box::new(spec.profile.clone()));
        let paths_differ = mapped_volumes(&spec, &argv::create(&spec, &caps()))
            .iter()
            .any(|(h, c, _)| h != c);

        let routing = sandbox.git_routing_for(pathmap::is_identity_for(windows_host));
        assert_eq!(
            routing == GitRouting::ViaDaemon,
            paths_differ,
            "routing and the mounts disagree about whether git sees one set of paths \
             (windows_host={windows_host})"
        );
    }
}
