//! Bringing the sandbox up, off the render thread (feature 027).
//!
//! The shell is where the impure work happens: reading the host's uid, finding the credential
//! paths, issuing and writing the token, and driving the container runtime. Every *decision* the
//! bring-up makes is in `micold_core::sandbox`, so what is left here is gathering the facts the
//! core cannot know and handing them over.
//!
//! That split is not bookkeeping. It is what lets the entire sandbox sequence be tested without a
//! container runtime — see `micold-core/tests/sandbox_state.rs` — while this file stays small
//! enough to read.

use std::path::PathBuf;

use micold_core::endpoint::DEFAULT_SANDBOX_PORT;
use micold_core::protocol::auth::{host_token_path, Token, CONTAINER_TOKEN_PATH};
use micold_core::sandbox::cli::CliRuntime;
use micold_core::sandbox::exec::SystemRunner;
use micold_core::sandbox::lifecycle::{bring_up, Failure, SandboxState, Started};
use micold_core::sandbox::runtime::RuntimeCapabilities;
use micold_core::sandbox::{CredentialLayout, MountSet, SandboxProfile, SandboxSpec, SecretMount};

/// The container's name. Fixed, so a sandbox left over from a previous run of the app is
/// recognisable as ours rather than accumulating beside itself (US6 scenario 5).
pub const CONTAINER_NAME: &str = "micold-sandbox";

/// The user-defined network the sandbox joins. Created with IP masquerade disabled when the posture
/// is `NoOutbound` (research R4).
pub const NETWORK_NAME: &str = "micold-sandbox-net";

/// Everything the host has to tell the core before a sandbox can be built.
///
/// Gathered in one place so the impure lookups happen once, at a call site that is allowed to fail,
/// rather than being scattered through the sequence.
pub struct HostFacts {
    pub uid: u32,
    pub gid: u32,
    pub state_dir: PathBuf,
    /// The host user's home, passed into the container as `HOME` — see `SandboxSpec::home`.
    pub home: PathBuf,
    pub layout: CredentialLayout,
}

impl HostFacts {
    /// Read the host's identity and credential layout.
    ///
    /// The uid and gid are read **now** rather than baked into the image, so one published image
    /// serves every user and files written into a project come back owned by whoever ran the app
    /// (research R3).
    pub fn gather(state_dir: PathBuf) -> Self {
        let home = directories::UserDirs::new()
            .map(|d| d.home_dir().to_path_buf())
            .unwrap_or_default();
        let ssh_auth_sock = std::env::var_os("SSH_AUTH_SOCK").map(PathBuf::from);
        let (uid, gid) = micold_core::sandbox::host_identity();
        Self {
            uid,
            gid,
            state_dir,
            layout: CredentialLayout::conventional(&home, ssh_auth_sock.as_deref()),
            home,
        }
    }
}

/// A sandbox that came up.
///
/// Deliberately does **not** carry the dial address or the token. Both are re-derived at dial time
/// by the connection subscription — the address from the configured port, the token by reading the
/// file this function wrote — so that a sandbox started by a *previous* run of the client is still
/// reachable by this one. Handing them out here would make the connection depend on having been
/// the process that started the sandbox, which is exactly the property the daemon exists to avoid.
pub struct Ready {
    pub started: Started,
}

/// Bring the sandbox up for `profile`, sharing `projects`.
///
/// `observe` is called as each stage is entered, so the view can show progress while the image is
/// being acquired — the one stage that may take minutes (SC-004).
pub fn start(
    profile: &SandboxProfile,
    projects: &[PathBuf],
    facts: &HostFacts,
    port: u16,
    observe: &mut dyn FnMut(SandboxState),
) -> Result<Ready, Failure> {
    // A fresh token per sandbox lifetime. Written 0600 and mounted read-only, so it reaches the
    // container through the filesystem rather than through argv — where `inspect` would show it.
    let token = Token::generate();
    let token_path = host_token_path(&facts.state_dir);
    if let Err(e) = token.write_to(&token_path) {
        return Err(Failure {
            stage: micold_core::sandbox::lifecycle::Stage::Creating,
            error: micold_core::sandbox::runtime::RuntimeError::Unknown {
                stderr: format!(
                    "could not write the sandbox token to {}: {e}",
                    token_path.display()
                ),
            },
        });
    }

    let mounts = MountSet::build(
        projects,
        profile,
        &facts.layout,
        facts.state_dir.clone(),
        SecretMount {
            host: token_path,
            container: PathBuf::from(CONTAINER_TOKEN_PATH),
        },
    );

    let runtime = CliRuntime::new(profile.runtime, SystemRunner);
    let build_spec = |_caps: &RuntimeCapabilities| SandboxSpec {
        name: CONTAINER_NAME.to_string(),
        profile: profile.clone(),
        mounts: mounts.clone(),
        uid: facts.uid,
        gid: facts.gid,
        control_port: port,
        published_ports: Vec::new(),
        network_name: NETWORK_NAME.to_string(),
        home: facts.home.clone(),
    };

    // The fingerprint this client was built with. A sandbox whose container carries a different
    // one came from another source tree, which only matters — and only refuses — for a locally
    // built image (research R8).
    let started = bring_up(
        &runtime,
        profile,
        &mounts,
        micold_core::protocol::version::BUILD_FINGERPRINT,
        build_spec,
        observe,
    )?;
    Ok(Ready { started })
}

/// The port the sandbox publishes its control channel on.
pub fn control_port() -> u16 {
    DEFAULT_SANDBOX_PORT
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_container_name_is_fixed() {
        // A name derived from the working directory or a timestamp would accumulate a container per
        // run, which is the leftover-sandbox failure the spec names (US6 scenario 5).
        assert_eq!(CONTAINER_NAME, "micold-sandbox");
    }

    #[test]
    fn host_facts_are_gathered_without_reading_a_runtime() {
        // Cheap and side-effect-free: gathering must not depend on Docker being installed, or the
        // "runtime is not installed" failure could never be reported properly.
        let facts = HostFacts::gather(std::env::temp_dir());
        assert!(facts.layout.git_config.is_some());
    }

    #[test]
    fn the_control_port_is_the_documented_default() {
        assert_eq!(control_port(), DEFAULT_SANDBOX_PORT);
    }
}
