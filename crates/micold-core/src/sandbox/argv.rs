//! PURE: [`SandboxSpec`] + [`RuntimeCapabilities`] → the runtime's argument vector.
//!
//! The heart of this feature's test surface. Given the same spec and capabilities the argv is
//! byte-identical (obligation C-1), so no environment lookup, clock, or randomness may appear
//! here — anything variable (uid, token path, port) arrives *in* the spec.
//!
//! Two obligations are worth stating where the code is, because both fail silently otherwise:
//!
//! - **C-2**: a limit the runtime cannot enforce produces *no flag*. The user's stored value is
//!   preserved, but never passed. Passing it anyway is the silent drift `endpoint.rs` already
//!   refuses to tolerate, and here it would leave the user believing a bound exists.
//! - **C-3**: the mounts are exactly the [`MountSet`] and nothing else. No implicit home mount, no
//!   runtime control socket. A sandbox's guarantee is what it *cannot* reach.

use std::ffi::OsString;

use super::dialect::Dialect;
use super::runtime::RuntimeCapabilities;
use super::{MountSet, NetworkPosture, SandboxSpec};

/// Flags that would hand the sandbox back the host, in whole or in part.
///
/// Asserted against generated argv by [`tests::no_escalation_flag_is_ever_generated`] rather than
/// left to review, so a future dialect cannot quietly add one (obligation C-9).
pub const ESCALATION_DENYLIST: &[&str] = &[
    "--privileged",
    "--cap-add",
    "--pid=host",
    "--pid",
    "--network=host",
    "--ipc=host",
    "--uts=host",
    "--userns=host",
    "--security-opt",
    "-v/var/run/docker.sock",
];

/// The arguments that create the sandbox's network.
///
/// [`NetworkPosture::NoOutbound`] is a user-defined bridge with IP masquerade disabled: outbound
/// connections have no NAT and fail, while inbound port publishing is host-side DNAT and keeps
/// working (research R4). Expressing it as `--internal` instead was measured and rejected — it
/// leaves the container unreachable from the host, severing the control channel this feature needs.
pub fn network_create(spec: &SandboxSpec, dialect: &Dialect) -> Vec<OsString> {
    let mut args: Vec<OsString> = vec![
        "network".into(),
        "create".into(),
        "--driver".into(),
        "bridge".into(),
    ];
    if spec.profile.network == NetworkPosture::NoOutbound {
        args.push("-o".into());
        args.push(dialect.no_masquerade_opt.into());
    }
    args.push((&spec.network_name).into());
    args
}

/// The arguments that create the sandbox container.
///
/// Total: every input has already been validated and reconciled (that is what a [`SandboxSpec`]
/// *is*), so this cannot fail and does not report problems.
pub fn create(spec: &SandboxSpec, caps: &RuntimeCapabilities) -> Vec<OsString> {
    let dialect = Dialect::for_kind(caps.kind);
    let mut args: Vec<OsString> = vec!["create".into(), "--name".into(), (&spec.name).into()];

    // Identity first, so a reader of the argv sees immediately that this is not running as root.
    args.extend(
        dialect
            .identity_args(spec.uid, spec.gid)
            .into_iter()
            .map(Into::into),
    );

    // Restart policy carries the existing session-survival opt-in (research R6). `unless-stopped`
    // rather than `always`, so a user who explicitly stops the sandbox stays stopped over a reboot.
    args.push("--restart".into());
    args.push(if spec.profile.survive_logout {
        "unless-stopped".into()
    } else {
        "no".into()
    });

    args.push("--network".into());
    args.push((&spec.network_name).into());

    // The control channel is always published to loopback, whatever the network posture — that is
    // the whole point of the masquerade-disabled bridge.
    args.push("-p".into());
    args.push(format!("127.0.0.1:{p}:{p}", p = spec.control_port).into());
    for port in &spec.published_ports {
        args.push("-p".into());
        args.push(format!("127.0.0.1:{port}:{port}").into());
    }

    args.extend(budget_args(spec, caps));
    args.extend(mount_args(&spec.mounts));

    args.push((&spec.profile.image.reference).into());
    args
}

/// The limit flags, omitting every limit this runtime cannot enforce (obligation C-2).
fn budget_args(spec: &SandboxSpec, caps: &RuntimeCapabilities) -> Vec<OsString> {
    let b = &spec.profile.budget;
    let mut args: Vec<OsString> = Vec::new();

    if let (Some(c), true) = (b.cpus_milli, caps.cpus.is_supported()) {
        // Thousandths of a core, printed as the decimal both runtimes accept.
        args.push("--cpus".into());
        args.push(format!("{}.{:03}", c.0 / 1000, c.0 % 1000).into());
    }
    if let (Some(m), true) = (b.memory_bytes, caps.memory.is_supported()) {
        args.push("--memory".into());
        args.push(format!("{}m", m.as_mib()).into());
    }
    if let (Some(p), true) = (b.pids, caps.pids.is_supported()) {
        args.push("--pids-limit".into());
        args.push(p.to_string().into());
    }
    if let (Some(s), true) = (b.storage_bytes, caps.storage.is_supported()) {
        args.push("--storage-opt".into());
        args.push(format!("size={}m", s.as_mib()).into());
    }
    args
}

/// The mount flags: exactly the [`MountSet`], and nothing else (obligation C-3).
fn mount_args(mounts: &MountSet) -> Vec<OsString> {
    let mut args: Vec<OsString> = Vec::new();
    for m in &mounts.projects {
        let mode = if m.writable { "rw" } else { "ro" };
        args.push("-v".into());
        args.push(format!("{}:{}:{mode}", m.host.display(), m.container.display()).into());
    }
    args.push("-v".into());
    args.push(
        format!(
            "{}:{}:rw",
            mounts.state.host.display(),
            mounts.state.container.display()
        )
        .into(),
    );
    args.push("-v".into());
    args.push(
        format!(
            "{}:{}:ro",
            mounts.secret.host.display(),
            mounts.secret.container.display()
        )
        .into(),
    );
    // Credentials are read-only without exception: the sandbox is allowed to *use* the user's
    // identity when they opted in, never to rewrite it.
    for c in &mounts.credentials {
        args.push("-v".into());
        args.push(format!("{}:{}:ro", c.host.display(), c.container.display()).into());
    }
    args
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sandbox::image::ImageSource;
    use crate::sandbox::runtime::{IdentityMapping, LimitSupport, RuntimeKind};
    use crate::sandbox::{
        Bytes, CredentialMount, CredentialShare, MilliCpus, NetworkPosture, ProjectMount,
        ResourceBudget, SandboxProfile, SecretMount, StateMount,
    };
    use std::collections::BTreeSet;
    use std::path::PathBuf;

    fn caps(kind: RuntimeKind, storage: LimitSupport) -> RuntimeCapabilities {
        RuntimeCapabilities {
            kind,
            version: "29.5.1".into(),
            cpus: LimitSupport::Supported,
            memory: LimitSupport::Supported,
            pids: LimitSupport::Supported,
            storage,
            identity_mapping: match kind {
                RuntimeKind::Docker => IdentityMapping::ExplicitUidGid,
                RuntimeKind::Podman => IdentityMapping::KeepId,
            },
        }
    }

    fn spec() -> SandboxSpec {
        SandboxSpec {
            name: "micold-sandbox".into(),
            profile: SandboxProfile {
                image: ImageSource {
                    reference: "micold-daemon:0.27.0".into(),
                    ..ImageSource::default()
                },
                ..SandboxProfile::default()
            },
            mounts: MountSet {
                projects: vec![ProjectMount {
                    host: PathBuf::from("/home/u/p"),
                    container: PathBuf::from("/home/u/p"),
                    writable: true,
                }],
                state: StateMount {
                    host: PathBuf::from("/home/u/.local/share/micold-ai-ide"),
                    container: PathBuf::from("/var/lib/micold"),
                },
                secret: SecretMount {
                    host: PathBuf::from("/run/user/1000/micold/token"),
                    container: PathBuf::from("/run/micold/token"),
                },
                credentials: Vec::new(),
            },
            uid: 1000,
            gid: 1000,
            control_port: 7727,
            published_ports: Vec::new(),
            network_name: "micold-net".into(),
        }
    }

    fn strings(args: &[OsString]) -> Vec<String> {
        args.iter()
            .map(|a| a.to_string_lossy().into_owned())
            .collect()
    }

    #[test]
    fn argv_is_a_pure_function_of_the_spec() {
        // Conformance check K-1. If this ever fails, something in here reached for the world, and
        // every other assertion in this file becomes conditional on when it ran.
        let (spec, caps) = (spec(), caps(RuntimeKind::Docker, LimitSupport::Supported));
        assert_eq!(create(&spec, &caps), create(&spec, &caps));
    }

    #[test]
    fn each_supported_limit_produces_exactly_its_flag() {
        // Conformance check K-2, including the unit conversions — a megabyte passed as a byte count
        // is a limit a thousand times looser than the one the user set.
        let mut s = spec();
        s.profile.budget = ResourceBudget {
            cpus_milli: Some(MilliCpus(2500)),
            memory_bytes: Some(Bytes::from_mib(4096)),
            pids: Some(512),
            storage_bytes: Some(Bytes::from_mib(8192)),
        };
        let args = strings(&create(
            &s,
            &caps(RuntimeKind::Docker, LimitSupport::Supported),
        ));

        let value_after = |flag: &str| {
            args.iter()
                .position(|a| a == flag)
                .and_then(|i| args.get(i + 1))
                .cloned()
        };
        assert_eq!(value_after("--cpus").as_deref(), Some("2.500"));
        assert_eq!(value_after("--memory").as_deref(), Some("4096m"));
        assert_eq!(value_after("--pids-limit").as_deref(), Some("512"));
        assert_eq!(value_after("--storage-opt").as_deref(), Some("size=8192m"));
    }

    #[test]
    fn an_unsupported_limit_produces_no_flag_at_all() {
        // Conformance check K-3, and the behavioural half of research R5's answer. The setting is
        // preserved in the profile and simply not passed — the view is what tells the user why.
        let mut s = spec();
        s.profile.budget.storage_bytes = Some(Bytes::from_mib(8192));
        let unsupported = caps(
            RuntimeKind::Docker,
            LimitSupport::unsupported("overlayfs without pquota"),
        );
        let args = strings(&create(&s, &unsupported));
        assert!(
            !args.iter().any(|a| a.starts_with("--storage-opt")),
            "an unenforceable limit must not be passed: {args:?}"
        );
        // The user's intent survives (rule RC-3) — it is the argv that omits it, not the profile.
        assert_eq!(s.profile.budget.storage_bytes, Some(Bytes::from_mib(8192)));
    }

    #[test]
    fn an_unset_limit_produces_no_flag_even_when_supported() {
        let mut s = spec();
        s.profile.budget = ResourceBudget {
            cpus_milli: None,
            memory_bytes: None,
            pids: None,
            storage_bytes: None,
        };
        let args = strings(&create(
            &s,
            &caps(RuntimeKind::Docker, LimitSupport::Supported),
        ));
        for flag in ["--cpus", "--memory", "--pids-limit", "--storage-opt"] {
            assert!(
                !args.iter().any(|a| a == flag),
                "{flag} was passed for an unset limit"
            );
        }
    }

    #[test]
    fn argv_mounts_are_exactly_the_mount_set() {
        // Conformance check K-4, compared as sets. This is the assertion that would fail if anyone
        // added a "convenience" mount — the user's home, a cache directory, the runtime socket.
        let mut s = spec();
        s.mounts.credentials = vec![CredentialMount {
            share: CredentialShare::GitConfig,
            host: PathBuf::from("/home/u/.gitconfig"),
            container: PathBuf::from("/home/u/.gitconfig"),
        }];
        let args = strings(&create(
            &s,
            &caps(RuntimeKind::Docker, LimitSupport::Supported),
        ));

        let mounted: Vec<String> = args
            .iter()
            .enumerate()
            .filter(|(_, a)| a.as_str() == "-v")
            .filter_map(|(i, _)| args.get(i + 1).cloned())
            .collect();
        // projects + state volume + secret + one credential
        assert_eq!(mounted.len(), 4, "mounted: {mounted:?}");
        assert!(mounted
            .iter()
            .any(|m| m.starts_with("/home/u/p:/home/u/p:rw")));
        assert!(mounted
            .iter()
            .any(|m| m.contains("micold-ai-ide:/var/lib/micold")));
        assert!(mounted.iter().any(|m| m.ends_with("/run/micold/token:ro")));
        assert!(mounted
            .iter()
            .any(|m| m.contains(".gitconfig") && m.ends_with(":ro")));
    }

    #[test]
    fn a_default_profile_mounts_no_credentials() {
        // The default posture, checked where it is spent rather than only where it is declared.
        let s = spec();
        assert!(s.profile.credentials.is_empty());
        let args = strings(&create(
            &s,
            &caps(RuntimeKind::Docker, LimitSupport::Supported),
        ));
        assert!(!args.iter().any(|a| a.contains(".gitconfig")));
        assert!(!args.iter().any(|a| a.contains("ssh")));
    }

    #[test]
    fn projects_are_mounted_at_their_own_absolute_paths_on_unix() {
        // Conformance check K-5, the claim git's worktree metadata depends on (research R2).
        let s = spec();
        for m in &s.mounts.projects {
            assert_eq!(m.host, m.container);
        }
        let args = strings(&create(
            &s,
            &caps(RuntimeKind::Docker, LimitSupport::Supported),
        ));
        assert!(args.iter().any(|a| a == "/home/u/p:/home/u/p:rw"));
    }

    #[test]
    fn the_control_port_is_published_to_loopback_not_to_every_interface() {
        // `-p 7727:7727` would publish to 0.0.0.0 and put the daemon on the network. The loopback
        // bind is not cosmetic: it is half of why the shared secret is the *other* half.
        let args = strings(&create(
            &spec(),
            &caps(RuntimeKind::Docker, LimitSupport::Supported),
        ));
        assert!(args.iter().any(|a| a == "127.0.0.1:7727:7727"), "{args:?}");
        assert!(!args.iter().any(|a| a == "7727:7727"));
    }

    #[test]
    fn no_outbound_disables_masquerade_and_keeps_the_published_port() {
        // Conformance check K-7 — the measured answer to research R4, asserted in both directions.
        let mut s = spec();
        s.profile.network = NetworkPosture::NoOutbound;
        let dialect = Dialect::for_kind(RuntimeKind::Docker);

        let net = strings(&network_create(&s, &dialect));
        assert!(
            net.iter().any(|a| a.contains("enable_ip_masquerade=false")),
            "{net:?}"
        );

        // The configuration that was measured to break the control channel must never appear.
        assert!(
            !net.iter().any(|a| a == "--internal"),
            "an --internal network makes the published port inert (research R4): {net:?}"
        );

        let args = strings(&create(
            &s,
            &caps(RuntimeKind::Docker, LimitSupport::Supported),
        ));
        assert!(args.iter().any(|a| a == "127.0.0.1:7727:7727"));
    }

    #[test]
    fn outbound_leaves_masquerade_alone() {
        let mut s = spec();
        s.profile.network = NetworkPosture::Outbound;
        let net = strings(&network_create(&s, &Dialect::for_kind(RuntimeKind::Docker)));
        assert!(!net.iter().any(|a| a.contains("masquerade")), "{net:?}");
    }

    #[test]
    fn survival_selects_the_restart_policy_on_every_platform() {
        // Research R6: the setting keeps one name and one meaning; only the mechanism differs by
        // placement. `unless-stopped`, not `always`, so an explicit stop survives a reboot.
        let mut s = spec();
        s.profile.survive_logout = true;
        let on = strings(&create(
            &s,
            &caps(RuntimeKind::Docker, LimitSupport::Supported),
        ));
        let idx = on.iter().position(|a| a == "--restart").unwrap();
        assert_eq!(on[idx + 1], "unless-stopped");

        s.profile.survive_logout = false;
        let off = strings(&create(
            &s,
            &caps(RuntimeKind::Docker, LimitSupport::Supported),
        ));
        let idx = off.iter().position(|a| a == "--restart").unwrap();
        assert_eq!(off[idx + 1], "no");
    }

    #[test]
    fn a_user_exposed_port_is_published_alongside_the_control_port() {
        let mut s = spec();
        s.published_ports = vec![3000];
        let args = strings(&create(
            &s,
            &caps(RuntimeKind::Docker, LimitSupport::Supported),
        ));
        assert!(args.iter().any(|a| a == "127.0.0.1:3000:3000"));
        assert!(args.iter().any(|a| a == "127.0.0.1:7727:7727"));
    }

    #[test]
    fn no_escalation_flag_is_ever_generated() {
        // Conformance check K-11, obligation C-9. A denylist rather than a review note, so a future
        // dialect cannot quietly hand the host back.
        let mut s = spec();
        s.profile.credentials = BTreeSet::from(CredentialShare::ALL);
        s.published_ports = vec![3000, 8080];

        for kind in RuntimeKind::ALL {
            for storage in [LimitSupport::Supported, LimitSupport::unsupported("no")] {
                let args = strings(&create(&s, &caps(kind, storage)));
                for banned in ESCALATION_DENYLIST {
                    // `--userns=keep-id` is podman's identity mapping and legitimately starts with
                    // `--userns`; the denylist entry is the exact `--userns=host`.
                    assert!(
                        !args.iter().any(|a| a == banned),
                        "{kind} generated the denylisted {banned}: {args:?}"
                    );
                }
                assert!(
                    !args.iter().any(|a| a.contains("docker.sock")),
                    "{kind} mounted the runtime's own control socket"
                );
            }
        }
    }

    #[test]
    fn both_dialects_produce_a_creatable_argv() {
        // Conformance check K-1 across dialects: podman is held to the same suite as Docker, which
        // is the only thing that makes "the runtime is replaceable" a fact rather than a claim.
        for kind in RuntimeKind::ALL {
            let args = strings(&create(&spec(), &caps(kind, LimitSupport::Supported)));
            assert_eq!(args.first().map(String::as_str), Some("create"));
            assert_eq!(
                args.last().map(String::as_str),
                Some("micold-daemon:0.27.0")
            );
            assert!(args.iter().any(|a| a == "--name"));
        }
    }
}
