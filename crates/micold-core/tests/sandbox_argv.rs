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
use micold_core::sandbox::image::{ImageChoice, OwnImage};
use micold_core::sandbox::pathmap::{self, WINDOWS_MOUNT_ROOT};
use micold_core::sandbox::placement::{GitRouting, Placement};
use micold_core::sandbox::runtime::{
    IdentityMapping, LimitSupport, NoOutboundSupport, RuntimeCapabilities, RuntimeKind,
};
use micold_core::sandbox::{
    CredentialLayout, CredentialShare, MountSet, SandboxProfile, SandboxSpec, SecretMount,
    CONTAINER_SECRETS_DIR,
};
use micold_core::session::AiCli;
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
        no_outbound: NoOutboundSupport::docker(),
        selinux_enforcing: false,
    }
}

/// A spec for one project, with every credential opted in, under a named platform's mapping.
///
/// Every credential is shared deliberately: they are the mounts whose *host* paths come from
/// the user's home directory, so they are where a Windows path would leak into a container path if
/// the mapping were skipped for anything but projects.
fn spec_for(windows_host: bool) -> SandboxSpec {
    let (project, home, state, token) = if windows_host {
        (WIN_PROJECT, WIN_HOME, WIN_STATE, WIN_TOKEN)
    } else {
        (NIX_PROJECT, NIX_HOME, NIX_STATE, NIX_TOKEN)
    };
    let profile = SandboxProfile {
        image: ImageChoice::Own(OwnImage::Reference("micold-daemon:0.27.0".into())),
        credentials: BTreeSet::from([CredentialShare::AiCliAuth]),
        ..SandboxProfile::default()
    };
    let layout = CredentialLayout::conventional(Path::new(home), None);
    let mounts = MountSet::build_for(
        &[PathBuf::from(project)],
        &profile,
        &layout,
        state,
        Path::new(home),
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
        time_zone: None,
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

/// The mounts that show one host location under two spellings, which is not all of them.
///
/// The state directory and the token are mounted at *fixed* container paths — `/var/lib/...` and
/// `/run/micold/token` — because they are where the daemon inside the image looks, on every host.
/// So they are never an identity mount, not even on Linux, and a test that demanded they were
/// would be asserting the wrong invariant about the right code.
///
/// The sandbox's home is excluded for a different reason (FR-004d): it is a **substitution**, not a
/// mapping. Its host half is an application-owned directory and its container half is the user's
/// home path, and the two are supposed to differ on every platform — that is what shadowing the
/// host home means. Only its container half comes from [`pathmap`], which the Windows test below
/// still covers by asserting every container half is a Linux path.
///
/// Every AI CLI's sign-in is excluded for the state directory's reason (T065): it is no longer
/// mounted at its own live path but under [`CONTAINER_SECRETS_DIR`]`/ai-cli-auth/<suffix>`, a fixed
/// container path the sandbox's own relink step resolves — so its two halves are supposed to
/// differ on every platform, Linux included.
fn mapped_volumes(
    spec: &SandboxSpec,
    args: &[std::ffi::OsString],
) -> Vec<(String, String, String)> {
    let fixed = [
        spec.mounts.state.host.to_string_lossy().into_owned(),
        spec.mounts.home.host.to_string_lossy().into_owned(),
        // The sanitized git configuration: built in the state directory, mounted at `~/.gitconfig`.
        spec.mounts
            .state
            .host
            .join("gitconfig")
            .to_string_lossy()
            .into_owned(),
    ]
    .into_iter()
    // The configuration and secrets folders (#1527): fixed container paths, like the state's.
    .chain(
        spec.mounts
            .delivery
            .iter()
            .flat_map(|d| [&d.config, &d.secrets])
            .map(|p| p.to_string_lossy().into_owned()),
    )
    .chain(
        ["host_ed25519", "host_ed25519.pub", "authorized_keys"].map(|f| {
            spec.mounts
                .secret
                .host
                .join(f)
                .to_string_lossy()
                .into_owned()
        }),
    )
    .collect::<Vec<_>>();
    volumes(args)
        .into_iter()
        .filter(|(host, _, _)| !fixed.contains(host))
        .filter(|(_, container, _)| !container.starts_with(CONTAINER_SECRETS_DIR))
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

    // The project comes from the host's filesystem, so it lands under the container root. The
    // state, token and sign-in mounts do not: they have fixed container paths the image dictates
    // (the sign-ins since T065 — read-only under the secrets directory, relinked inside).
    let mapped = mapped_volumes(&spec, &argv::create(&spec, &caps()));
    assert_eq!(mapped.len(), 1, "the one project: {mapped:?}");
    assert!(
        mapped
            .iter()
            .all(|(_, c, _)| c.starts_with(WINDOWS_MOUNT_ROOT)),
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
                allowed.iter().any(|p| Path::new(&host).starts_with(p)),
                "argv mounted {host:?}, which is not in the mount set (windows_host={windows_host})"
            );
        }
        assert_eq!(
            volumes(&argv::create(&spec, &caps())).len(),
            allowed.len() + 2, // the key directory is mounted as three single files
            "argv must mount the whole set and nothing else (windows_host={windows_host})"
        );
    }
}

/// On Linux and macOS the mapping is the identity, and the argv shows it.
#[test]
fn a_unix_host_mounts_every_path_at_itself() {
    let spec = spec_for(false);
    let mapped = mapped_volumes(&spec, &argv::create(&spec, &caps()));
    assert_eq!(
        mapped.len(),
        1,
        "the one project; the git config is the state-directory copy and every sign-in is \
         delivered under the secrets directory (T065): {mapped:?}"
    );
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

/// SC-008 (T065): no credential mount is writable any more, the sign-in included.
///
/// It used to be the one exception (rule N-4, BUG-006, FR-004e) because `claude` replaces its own
/// token when it refreshes it, and a read-only token stops working the first time it expires.
/// T065 removes the exception instead of the capability: the sign-in is mounted read-only under
/// the secrets directory and the sandbox symlinks the CLI's live path to it at session start, so
/// the CLI's refresh writes a fresh file in its own home and never writes through a host mount.
#[test]
fn no_credential_is_mounted_writable_not_even_the_ai_cli_sign_in() {
    let spec = spec_for(false);
    let credential_hosts: Vec<String> = spec
        .mounts
        .credentials
        .iter()
        .map(|c| c.host.to_string_lossy().into_owned())
        .collect();
    // Joined a segment at a time: `Path::join` keeps a `/` inside a segment verbatim, so the one
    // string spelling of this path does not match the layout's on Windows.
    let sign_in = Path::new(NIX_HOME)
        .join(".claude")
        .join(".credentials.json")
        .to_string_lossy()
        .into_owned();
    assert!(
        credential_hosts.contains(&sign_in),
        "the spec shares the sign-in, or this test checks nothing: {credential_hosts:?}"
    );

    // Every CLI's sign-in file, as that provider names it (feature 488, T036): named explicitly so
    // this still asserts something about the sign-ins in particular, not just "all mounts are ro".
    let sign_ins: Vec<String> = AiCli::ALL
        .iter()
        .filter_map(|cli| cli.provider().sandbox_auth_file(Path::new(NIX_HOME)))
        .map(|f| f.to_string_lossy().into_owned())
        .collect();
    let mut seen_sign_in = 0;
    for (host, container, mode) in volumes(&argv::create(&spec, &caps())) {
        if !credential_hosts.contains(&host) {
            continue;
        }
        assert_eq!(mode, "ro", "credential mount {host} must be read-only");
        if sign_ins.contains(&host) {
            seen_sign_in += 1;
            assert!(
                container.starts_with(CONTAINER_SECRETS_DIR),
                "a sign-in is delivered under {CONTAINER_SECRETS_DIR}, not at its live path: \
                 {host} -> {container}"
            );
        }
    }
    assert!(
        seen_sign_in > 0,
        "no sign-in was mounted, so this test checked nothing"
    );
}

/// U52, FR-036c (BUG-574): which projects a container shares. One this bring-up created shares its
/// mount set's projects; an adopted one shares its destinations less this set's own non-project
/// mounts, including a project this client never registered.
#[test]
fn a_containers_projects_are_its_project_mounts() {
    let mounts = MountSet::build_for(
        &[PathBuf::from("/proj/P")],
        &SandboxProfile {
            credentials: BTreeSet::from([CredentialShare::AiCliAuth]),
            ..SandboxProfile::default()
        },
        &CredentialLayout::conventional(Path::new("/home/u"), None),
        PathBuf::from("/home/u/.local/share/micold-ai-ide"),
        Path::new("/home/u"),
        SecretMount {
            host: PathBuf::from("/home/u/.local/share/micold-ai-ide/sandbox.token"),
            container: PathBuf::from("/run/micold/token"),
        },
        false,
    );
    assert_eq!(
        mounts.container_projects(
            None,
            &CredentialLayout::conventional(Path::new("/home/u"), None)
        ),
        vec!["/proj/P".to_string()]
    );

    let mut destinations: Vec<String> = [
        &mounts.state.container,
        &mounts.home.container,
        &mounts.secret.container,
    ]
    .iter()
    .map(|p| p.to_string_lossy().into_owned())
    .collect();
    destinations.extend(
        mounts
            .credentials
            .iter()
            .map(|c| c.container.to_string_lossy().into_owned()),
    );
    destinations.push("/proj/Q".into());
    destinations.push("/proj/P".into());
    assert_eq!(
        mounts.container_projects(
            Some(&destinations),
            &CredentialLayout::conventional(Path::new("/home/u"), None)
        ),
        vec!["/proj/Q".to_string(), "/proj/P".to_string()],
        "an adopted container's projects are what it mounts besides state, home, token and \
         credentials, whoever registered them"
    );
}

/// U52, FR-036c (BUG-574, review A): a credential an adopted container mounts is not a project,
/// even when this client's profile does not share it. Another window may have created the
/// container with the AI CLI sign-in shared, or the share was turned off after bring-up.
#[test]
fn an_adopted_containers_credential_mounts_are_not_projects() {
    let home = Path::new("/home/u");
    let layout = CredentialLayout::conventional(home, None);
    let credential = |host: &Path| -> String {
        pathmap::map_for(host, cfg!(windows))
            .to_string_lossy()
            .into_owned()
    };
    let mounts = MountSet::build_for(
        &[PathBuf::from("/proj/P")],
        &SandboxProfile::default(),
        &layout,
        PathBuf::from("/home/u/.local/share/micold-ai-ide"),
        Path::new("/home/u"),
        SecretMount {
            host: PathBuf::from("/home/u/.local/share/micold-ai-ide/sandbox.token"),
            container: PathBuf::from("/run/micold/token"),
        },
        false,
    );
    let destinations: Vec<String> = [
        mounts.state.container.to_string_lossy().into_owned(),
        mounts.home.container.to_string_lossy().into_owned(),
        mounts.secret.container.to_string_lossy().into_owned(),
        // Mapped as the bring-up maps a credential on this platform: under the Windows mount root
        // on a Windows host, so the test holds on every CI runner.
        credential(&home.join(".claude").join(".credentials.json")),
        credential(&home.join(".gitconfig")),
        "/proj/P".into(),
    ]
    .into();
    assert_eq!(
        mounts.container_projects(Some(&destinations), &layout),
        vec!["/proj/P".to_string()],
        "a credential this profile does not share is still a credential, not a project"
    );
}

/// #1002 (FR-001, spec edge case "personal token in the host environment", T003): `argv::create`
/// is a function of the spec alone; it reads no environment. The one host variable the client reads, `SSH_AUTH_SOCK`, reaches it
/// only through `CredentialLayout::conventional` (`HostFacts::gather`), so the layout here names a
/// socket as a real host would. The argv names each variable it sets, so every one is checked
/// against the short list a container may get: a pass-through of `GH_TOKEN` or any other host
/// variable fails that list.
#[test]
fn no_personal_token_or_agent_socket_reaches_the_container_environment() {
    let mut spec = spec_for(false);
    let layout = CredentialLayout::conventional(
        Path::new(NIX_HOME),
        Some(Path::new("/tmp/ssh-host/agent.sock")),
    );
    assert!(
        layout
            .retired
            .contains(&PathBuf::from("/tmp/ssh-host/agent.sock")),
        "the layout carries the host's socket, so the search below is not vacuous"
    );
    spec.mounts = MountSet::build_for(
        &[PathBuf::from(NIX_PROJECT)],
        &spec.profile,
        &layout,
        NIX_STATE,
        Path::new(NIX_HOME),
        SecretMount {
            host: PathBuf::from(NIX_TOKEN),
            container: PathBuf::from("/run/micold/token"),
        },
        false,
    );
    // Other values are passed: the zone. The idle rule is a key of the delivered `daemon.toml`.
    spec.time_zone = Some("Europe/Berlin".into());
    spec.profile.survive_logout = true;
    let args: Vec<String> = argv::create(&spec, &caps())
        .iter()
        .map(|a| a.to_string_lossy().into_owned())
        .collect();
    let env: Vec<&String> = args
        .windows(2)
        .filter(|w| w[0] == "-e" || w[0] == "--env")
        .map(|w| &w[1])
        .collect();
    assert!(env.iter().any(|e| e.starts_with("TZ=")), "{env:?}");
    for e in &env {
        let name = e.split('=').next().unwrap();
        assert!(
            name == "HOME"
                || name == "TZ"
                || name.starts_with("GIT_CONFIG_")
                || name == "GIT_TERMINAL_PROMPT",
            "{e} is passed into the container"
        );
    }
    let all = args.join(" ");
    for leaked in ["GH_TOKEN", "GITHUB_TOKEN", "SSH_AUTH_SOCK", "/tmp/ssh-host"] {
        assert!(
            !all.contains(leaked),
            "{leaked} reaches the container: {all}"
        );
    }
    assert!(!args
        .iter()
        .any(|a| a == "--env-host" || a.starts_with("--env-file")));
}

/// FR-004: git in the container is told, in its environment, to reset every credential helper,
/// and each pair of that block is one `-e` argument.
#[test]
fn git_in_the_container_resets_every_credential_helper() {
    let spec = spec_for(false);
    let args: Vec<String> = argv::create(&spec, &caps())
        .iter()
        .map(|a| a.to_string_lossy().into_owned())
        .collect();
    let env: Vec<&String> = args
        .windows(2)
        .filter(|w| w[0] == "-e")
        .map(|w| &w[1])
        .collect();
    let block =
        micold_core::scoped_credential::git_env(micold_core::scoped_credential::GITHUB_BASE);
    assert!(block.contains_key("GIT_CONFIG_COUNT"));
    for (name, value) in block {
        assert!(
            env.iter().any(|e| **e == format!("{name}={value}")),
            "{name}={value} is not passed: {env:?}"
        );
    }
}

/// #1530: the git configuration is no mount any more: the app hands the daemon a sanitized copy as
/// a secret and `[git] config_file`, whichever platform the host is.
#[test]
fn no_git_config_is_mounted_into_the_container() {
    for windows_host in [false, true] {
        let spec = spec_for(windows_host);
        assert!(spec
            .mounts
            .credentials
            .iter()
            .all(|c| !c.container.to_string_lossy().ends_with(".gitconfig")));
        let args = argv::create(&spec, &caps());
        assert!(
            volumes(&args)
                .into_iter()
                .all(|(_, container, _)| !container.ends_with(".gitconfig")),
            "a .gitconfig is mounted: {args:?}"
        );
    }
}

/// FR-006: the scoped git credential's directory is a small tmpfs owned by the container's uid,
/// and no host path is mounted there.
#[test]
fn the_git_credential_directory_is_a_memory_tmpfs_and_no_host_mount() {
    let spec = spec_for(false);
    let args: Vec<String> = argv::create(&spec, &caps())
        .iter()
        .map(|a| a.to_string_lossy().into_owned())
        .collect();
    let at = args.iter().position(|a| a == "--tmpfs").expect("--tmpfs");
    assert_eq!(
        args[at + 1],
        "/run/micold/git:rw,noexec,nosuid,size=64k,mode=0700,uid=1000,gid=1000"
    );
    assert!(
        !args
            .iter()
            .any(|a| a.contains(":/run/micold/git:") || a.ends_with(":/run/micold/git")),
        "a host path is mounted at /run/micold/git: {args:?}"
    );
}
