//! #1002 (T012) — a sandbox holds none of the user's git identity, whatever the saved profile says.
//!
//! The host is a hostile one: a home with a private key, a git credential store and a `gh` token
//! file, an SSH agent socket path and `GH_TOKEN` / `GITHUB_TOKEN` in the environment, each carrying
//! a marker. The saved profile names the two removed shares (`ssh_agent`, `git_credentials`), which
//! still load. The container is created from the mount set that profile yields, and neither its
//! `inspect` output, its environment, nor its filesystem may contain a marker.
//!
//! Behind `sandbox-real-runtime` (Principle VI: the default suite needs nothing installed).
//!
//! ```text
//! cargo test -p micold-daemon --features sandbox-real-runtime sandbox_real_no_user_credentials -- --nocapture
//! ```

#![cfg(all(feature = "sandbox-real-runtime", unix))]

mod sandbox_real_support;

use micold_core::sandbox::{CredentialLayout, MountSet, SandboxProfile, SecretMount};
use sandbox_real_support::{cli_out, seed, start_sandbox, SandboxSpec};
use std::path::PathBuf;

const CONTAINER: &str = "micold-no-credentials-probe";
const NETWORK: &str = "micold-no-credentials-probe-net";
/// Not 7727, and not a neighbour's: a developer's own sandbox must not be disturbed.
const PORT: u16 = 17741;

use micold_core::sandbox::hostile_host::{
    HostileHost, CREDENTIAL_STORE_MARKER, GH_HOSTS_MARKER, GH_TOKEN_MARKER, GITHUB_TOKEN_MARKER,
    SSH_KEY_MARKER,
};

const MARKERS: [&str; 5] = [
    SSH_KEY_MARKER,
    CREDENTIAL_STORE_MARKER,
    GH_HOSTS_MARKER,
    GH_TOKEN_MARKER,
    GITHUB_TOKEN_MARKER,
];

/// Run `script` in the container, returning stdout whatever its exit status.
fn in_container(script: &str) -> String {
    cli_out(&["exec", CONTAINER, "sh", "-c", &format!("{script}; true")])
}

#[test]
fn sandbox_real_no_user_credentials_reach_the_container() {
    let dir = tempfile::tempdir().unwrap();
    let data = dir.path().join("data");
    let project = dir.path().join("project");
    std::fs::create_dir_all(&project).unwrap();
    seed(&data, &project, "no-credentials");
    let host = HostileHost::create(dir.path()).unwrap();
    let (home, agent) = (host.home.clone(), host.agent_socket.clone());

    // The host's environment as the client would see it. The harness creates the container itself
    // and passes no `-e` of these, which is the claim: the daemon never forwards them (FR-001).
    let _host_env = host.env();

    // A profile saved before #1002, naming the removed shares.
    let profile: SandboxProfile = serde_json::from_str(
        r#"{"credentials": ["ssh_agent", "git_config", "git_credentials", "ai_cli_auth"]}"#,
    )
    .unwrap();
    assert!(profile.removed_git_credentials);

    let mounts = MountSet::build(
        std::slice::from_ref(&project),
        &profile,
        &CredentialLayout::conventional(&home, Some(&agent)),
        data.join("micold-ai-ide"),
        &home,
        SecretMount {
            host: data.join("micold-ai-ide").join("ssh"),
            container: PathBuf::from("/run/micold/token"),
        },
    );
    // Only what the profile still shares is mounted, read-only.
    let extra: Vec<String> = mounts
        .credentials
        .iter()
        .filter(|c| c.host.exists())
        .flat_map(|c| {
            let mode = if c.share.writable() { "rw" } else { "ro" };
            [
                "-v".to_string(),
                format!("{}:{}:{mode}", c.host.display(), c.container.display()),
            ]
        })
        .collect();

    let home_str = home.display().to_string();
    let _sandbox = start_sandbox(&SandboxSpec {
        container: CONTAINER,
        network: NETWORK,
        port: PORT,
        data_home: &data,
        project: &project,
        home: &home_str,
        survive_logout: false,
        extra: &extra,
    });

    // What the runtime was told: mounts and environment -----------------------------------------
    let inspected = cli_out(&["inspect", CONTAINER]);
    for marker in MARKERS {
        assert!(!inspected.contains(marker), "inspect carries {marker}");
    }
    for name in ["SSH_AUTH_SOCK", "GH_TOKEN", "GITHUB_TOKEN"] {
        assert!(!inspected.contains(name), "inspect names {name}");
    }
    for path in [
        "/u/.ssh",
        "/u/.git-credentials",
        "/u/.config/gh",
        "ssh-agent.sock",
    ] {
        assert!(!inspected.contains(path), "inspect mounts {path}");
    }

    // What a process in it sees -----------------------------------------------------------------
    let env = in_container("env");
    for name in ["SSH_AUTH_SOCK", "GH_TOKEN", "GITHUB_TOKEN"] {
        assert!(!env.contains(name), "env names {name}:\n{env}");
    }
    for marker in MARKERS {
        assert!(!env.contains(marker), "env carries {marker}");
    }

    // FR-006: the scoped credential's directory is memory, a tmpfs that is no host mount.
    let mounts = in_container("grep ' /run/micold/git ' /proc/self/mounts");
    assert!(
        mounts.starts_with("tmpfs /run/micold/git tmpfs "),
        "/run/micold/git is not a tmpfs: {mounts:?}"
    );

    let pattern = MARKERS.join("\\|");
    let skip = "--exclude-dir=proc --exclude-dir=sys --exclude-dir=dev";
    for root in ["/", "\"$HOME\""] {
        let found = in_container(&format!("grep -rIl {skip} '{pattern}' {root} 2>/dev/null"));
        assert!(
            found.trim().is_empty(),
            "a marker is readable under {root}:\n{found}"
        );
    }
}
