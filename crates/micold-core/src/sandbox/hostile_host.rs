//! A host holding every git credential a sandbox must never see (#1002, FR-014).
//!
//! The FR-014 tests start, or build the whole start-up of, a daemon of each runtime on this host and
//! search what the container would get — its arguments, environment, mounts, secrets and files —
//! for any of it. Each item carries its own marker string, so a leak names what leaked.
//!
//! Compiled for tests only, as `kube::fake` is: the lib's unit tests reach it under `cfg(test)`,
//! the integration tests through the `kube-fake` feature.

use std::path::{Path, PathBuf};

use super::runtime::{IdentityMapping, LimitSupport, RuntimeCapabilities, RuntimeKind};
use super::{CredentialLayout, MountSet, SandboxProfile, SandboxSpec, SecretMount};

/// The content of the user's SSH private key.
pub const SSH_KEY_MARKER: &str = "HOSTILE-SSH-PRIVATE-KEY-7c1e";
/// The content of the user's git credential store.
pub const CREDENTIAL_STORE_MARKER: &str = "HOSTILE-GIT-CREDENTIAL-STORE-3b9d";
/// The token in the GitHub CLI's `hosts.yml`.
pub const GH_HOSTS_MARKER: &str = "HOSTILE-GH-HOSTS-OAUTH-TOKEN-51fa";
/// The value of `GH_TOKEN` in the host's environment.
pub const GH_TOKEN_MARKER: &str = "HOSTILE-GH-TOKEN-ENV-e04c";
/// The value of `GITHUB_TOKEN` in the host's environment.
pub const GITHUB_TOKEN_MARKER: &str = "HOSTILE-GITHUB-TOKEN-ENV-92d6";

/// The host: a home under some root, holding the user's key, credential store and CLI token, an SSH
/// agent socket path, and personal tokens in the environment.
#[derive(Debug, Clone)]
pub struct HostileHost {
    /// The user's home directory.
    pub home: PathBuf,
    /// Where `SSH_AUTH_SOCK` points. A path only: nothing the tests build may connect to it.
    pub agent_socket: PathBuf,
}

impl HostileHost {
    /// Write the host's files under `root` (a temporary directory the caller owns).
    pub fn create(root: &Path) -> std::io::Result<Self> {
        let home = root.join("home").join("u");
        let ssh = home.join(".ssh");
        std::fs::create_dir_all(&ssh)?;
        std::fs::write(
            ssh.join("id_ed25519"),
            format!(
                "-----BEGIN OPENSSH PRIVATE KEY-----\n{SSH_KEY_MARKER}\n-----END OPENSSH PRIVATE KEY-----\n"
            ),
        )?;
        std::fs::write(
            ssh.join("id_ed25519.pub"),
            format!("ssh-ed25519 AAAA{SSH_KEY_MARKER} u@host\n"),
        )?;
        std::fs::write(
            home.join(".git-credentials"),
            format!("https://u:{CREDENTIAL_STORE_MARKER}@github.com\n"),
        )?;
        let gh = home.join(".config").join("gh");
        std::fs::create_dir_all(&gh)?;
        std::fs::write(
            gh.join("hosts.yml"),
            format!("github.com:\n    oauth_token: {GH_HOSTS_MARKER}\n    user: u\n"),
        )?;
        // The commit identity a kept item shares: nothing in it is a credential (M2 sanitizes the
        // rest).
        std::fs::write(
            home.join(".gitconfig"),
            "[user]\n\tname = U\n\temail = u@example.com\n",
        )?;
        // The AI CLI sign-in a kept item shares.
        let claude = home.join(".claude");
        std::fs::create_dir_all(&claude)?;
        std::fs::write(claude.join(".credentials.json"), "{}")?;
        let agent_socket = root.join("agent").join("ssh-agent.sock");
        std::fs::create_dir_all(root.join("agent"))?;
        Ok(Self { home, agent_socket })
    }

    /// The host's environment as a builder may see it: the personal tokens and the agent socket.
    pub fn env(&self) -> Vec<(String, String)> {
        vec![
            ("GH_TOKEN".into(), GH_TOKEN_MARKER.into()),
            ("GITHUB_TOKEN".into(), GITHUB_TOKEN_MARKER.into()),
            (
                "SSH_AUTH_SOCK".into(),
                self.agent_socket.display().to_string(),
            ),
        ]
    }

    /// The conventional credential layout of this host, as the client builds it.
    pub fn layout(&self) -> CredentialLayout {
        CredentialLayout::conventional(&self.home, Some(&self.agent_socket))
    }

    /// The whole container start-up of a daemon with `profile` on this host, one project shared:
    /// the spec `argv::create` renders, with the mount set the client builds from this host's
    /// layout.
    pub fn container_spec(&self, profile: SandboxProfile) -> SandboxSpec {
        let state = self.home.join(".local").join("share").join("micold-ai-ide");
        let mounts = MountSet::build(
            &[self.home.join("code").join("thing")],
            &profile,
            &self.layout(),
            state.clone(),
            &self.home,
            SecretMount {
                host: state.join("token"),
                container: PathBuf::from("/run/micold/token"),
            },
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
            time_zone: Some("Europe/Berlin".into()),
            home: self.home.clone(),
        }
    }

    /// Every string whose presence in a container's start-up is a leak: each marker, and the path of
    /// each item, in both the host's spelling and the container's.
    pub fn forbidden(&self) -> Vec<String> {
        let paths = [
            self.home.join(".ssh"),
            self.home.join(".git-credentials"),
            self.home.join(".config").join("gh"),
            self.agent_socket.clone(),
        ];
        let mut out: Vec<String> = [
            SSH_KEY_MARKER,
            CREDENTIAL_STORE_MARKER,
            GH_HOSTS_MARKER,
            GH_TOKEN_MARKER,
            GITHUB_TOKEN_MARKER,
        ]
        .iter()
        .map(|m| (*m).to_string())
        .collect();
        for path in paths {
            out.push(path.display().to_string());
            out.push(
                super::pathmap::map_for(&path, cfg!(windows))
                    .display()
                    .to_string(),
            );
        }
        out.push("SSH_AUTH_SOCK".into());
        out.push("GH_TOKEN".into());
        out.push("GITHUB_TOKEN".into());
        out.sort();
        out.dedup();
        out
    }

    /// The forbidden strings `text` contains; empty when nothing leaked.
    pub fn leaks(&self, text: &str) -> Vec<String> {
        self.forbidden()
            .into_iter()
            .filter(|f| text.contains(f.as_str()))
            .collect()
    }
}

/// A Docker runtime that enforces every limit, for rendering a start-up's argv.
pub fn docker_caps() -> RuntimeCapabilities {
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
