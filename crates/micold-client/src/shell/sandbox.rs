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

use iced::Task;
use micold_client::app::Message;
use micold_client::features::sandbox::Msg as SandboxMsg;
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

/// Everything a bring-up needs that the application has to remember in order to run it *again*.
///
/// Boot has all of this to hand; a restart, minutes later, has none of it — the profile came from
/// settings the shell read once, and the project list from a store the daemon now owns. Held as one
/// value rather than as three fields so that "can this be restarted?" is a single `Option` check
/// (R9).
#[derive(Debug, Clone)]
pub struct BootPlan {
    pub profile: SandboxProfile,
    pub state_dir: PathBuf,
    /// The projects to share. Replaced when the daemon's catalog changes, which is what makes the
    /// *next* restart pick up a project registered since boot (M-4).
    pub projects: Vec<PathBuf>,
}

/// Run a bring-up off the render thread and report how it ended.
///
/// Blocking, because every step shells out to a container runtime and image acquisition can take
/// minutes — on the render thread that would freeze the window for the whole of it, the opposite of
/// SC-004's "continuous progress".
pub fn boot(plan: BootPlan) -> iced::Task<micold_client::app::Message> {
    iced::Task::future(async move {
        let outcome = tokio::task::spawn_blocking(move || {
            let facts = HostFacts::gather(plan.state_dir);
            // Progress is dropped here rather than streamed: a `Task::future` yields one message,
            // and threading a channel through boot for the sake of the first release's progress bar
            // would buy less than the settings view (US3) will when it renders this properly.
            start(
                &plan.profile,
                &plan.projects,
                &facts,
                control_port(),
                &mut |_| {},
            )
            .map(|ready| ready.started)
        })
        .await;

        match outcome {
            Ok(Ok(started)) => {
                micold_client::app::Message::Sandbox(SandboxMsg::Started(Box::new(started)))
            }
            Ok(Err(failure)) => {
                micold_client::app::Message::Sandbox(SandboxMsg::Failed(Box::new(failure)))
            }
            // A panicked or cancelled blocking task is still a sandbox that did not come up, and
            // the user needs the same standing banner for it as for a runtime that refused.
            Err(join) => {
                micold_client::app::Message::Sandbox(SandboxMsg::Failed(Box::new(Failure {
                    stage: micold_core::sandbox::lifecycle::Stage::Starting,
                    error: micold_core::sandbox::runtime::RuntimeError::Unknown {
                        stderr: join.to_string(),
                    },
                })))
            }
        }
    })
}

/// Ask the runtime, once, whether our container is still running (FR-036, US6 scenario 3).
///
/// Yields [`SandboxMsg::Lost`] only when the answer is a definite no —
/// the container is absent, or present and stopped. A runtime that cannot be reached to ask is
/// *not* an answer: reporting the sandbox lost because `docker` was briefly busy would replace a
/// transient reconnect with a banner and a restart the user did not need.
pub fn check_alive(plan: &BootPlan) -> iced::Task<micold_client::app::Message> {
    let runtime = plan.profile.runtime;
    iced::Task::future(async move {
        let gone = tokio::task::spawn_blocking(move || {
            use micold_core::sandbox::runtime::ContainerRuntime;
            CliRuntime::new(runtime, SystemRunner)
                .find(CONTAINER_NAME)
                .map(|found| found.is_none_or(|facts| !facts.running))
                .unwrap_or(false)
        })
        .await
        .unwrap_or(false);

        if gone {
            micold_client::app::Message::Sandbox(SandboxMsg::Lost)
        } else {
            micold_client::app::Message::NoOp
        }
    })
}

/// Read the service's own log out of the container (FR-038, US6 scenario 6).
///
/// This is the path that works when the *other* one cannot: asking the service for its recent
/// errors needs a connection to it, and the question is most often asked precisely because there
/// isn't one. The runtime kept the process's output whether or not anything ever connected.
///
/// An empty answer is returned as an empty list rather than as a failure — "the container is there
/// and it has said nothing" is itself the diagnosis, and reporting it as an error would send the
/// user looking for a problem with the log rather than with the service.
pub fn diagnostics(plan: &BootPlan) -> iced::Task<micold_client::app::Message> {
    let runtime = plan.profile.runtime;
    iced::Task::future(async move {
        let lines = tokio::task::spawn_blocking(move || {
            use micold_core::sandbox::runtime::ContainerRuntime;
            let rt = CliRuntime::new(runtime, SystemRunner);
            let facts = rt.find(CONTAINER_NAME).ok().flatten()?;
            rt.logs(&micold_core::sandbox::runtime::ContainerId(facts.id), 50)
                .ok()
        })
        .await
        .ok()
        .flatten()
        .unwrap_or_default();
        micold_client::app::Message::Sandbox(SandboxMsg::Diagnostics(lines))
    })
}

/// Stop the sandbox and remove its container (US6 scenario 4, FR-036).
///
/// Both steps, and in that order: stopping alone leaves a container that the next start finds and
/// has to reason about, which is the orphan the scenario is about. Both are idempotent by contract
/// obligation C-7, so a user who already ran `docker stop` themselves gets the same outcome rather
/// than an error.
///
/// Note what has *no* equivalent here: closing the application. The sandbox is left running by
/// design, because the sessions inside it are meant to outlive the window that opened them — which
/// is the whole reason the service is a separate process in the first place.
pub fn stop(plan: &BootPlan) -> iced::Task<micold_client::app::Message> {
    let runtime = plan.profile.runtime;
    iced::Task::future(async move {
        let _ = tokio::task::spawn_blocking(move || {
            use micold_core::sandbox::runtime::ContainerRuntime;
            let rt = CliRuntime::new(runtime, SystemRunner);
            if let Ok(Some(facts)) = rt.find(CONTAINER_NAME) {
                let id = micold_core::sandbox::runtime::ContainerId(facts.id);
                let _ = rt.stop(&id);
                let _ = rt.remove(&id);
            }
        })
        .await;
        micold_client::app::Message::Sandbox(SandboxMsg::Lost)
    })
}

/// The port the sandbox publishes its control channel on.
pub fn control_port() -> u16 {
    DEFAULT_SANDBOX_PORT
}

/// This feature's entry point: one arm in `main.rs` routes here (feature 028, contract M2).
///
/// Shape **B** with no pure half, like `shell/connection.rs` beside it: every arm returns a
/// `Task` or writes the binary-owned `app.sandbox`, so by M2's rule none of the six belongs in
/// `features/sandbox.rs`. The bodies did not move — they are `main.rs`'s six arms, verbatim, with
/// the routing decision stated once next to them instead of interleaved with the overlay and
/// session arms that surrounded them.
pub fn update(app: &mut crate::App, msg: SandboxMsg) -> Task<Message> {
    use micold_client::features::sandbox::Msg;

    match msg {
        // Feature 027. The sandbox's outcome is recorded, never acted on: a failure must not start
        // a session somewhere else, and a success needs no prompting because the connection
        // subscription is already retrying against the loopback port.
        Msg::Started(started) => {
            app.sandbox.started(*started);
            iced::Task::none()
        }
        Msg::Failed(failure) => {
            // Recorded and left standing. The banner draws it for as long as it lasts — the queue
            // would show it once, for four seconds, while the sandbox stayed broken (FR-035b, S-3),
            // which is exactly the edge case the spec calls out.
            app.sandbox.failed(*failure);
            iced::Task::none()
        }
        Msg::Diagnostics(lines) => {
            match lines.last() {
                // Phrased like the connected path's `RecentErrors` answer, because from the user's
                // side it is the same question — only the route it took to be answered differs.
                Some(latest) => app.core.notify_error(format!(
                    "The session service logged {} line(s) inside the sandbox; most recent: {}",
                    lines.len(),
                    latest.trim()
                )),
                None => app.core.notify_info(
                    "The sandbox is there, but the session service inside it has logged nothing.",
                ),
            }
            iced::Task::none()
        }
        Msg::Lost => {
            app.sandbox.container_lost(CONTAINER_NAME);
            iced::Task::none()
        }
        // The one edge back into bring-up, and it is here because a person pressed something
        // (R9, FR-035a). Both of these are user actions; neither is ever sent by the application
        // to itself.
        Msg::RestartRequested => {
            match app.sandbox_boot.clone() {
                Some(plan)
                    if app
                        .sandbox
                        .restart(micold_core::sandbox::lifecycle::RestartRequested) =>
                {
                    // Going back to the sandbox has to take the connection with it. If a fallback
                    // was in force, `placement.kind` is the host process; left there, the sandbox
                    // would come up and every session would still be running outside it — the
                    // banner saying "sandboxed" over an unconfined shell, which is the one thing
                    // FR-035b exists to prevent.
                    app.placement.kind =
                        micold_core::sandbox::placement::PlacementKind::LocalSandbox;
                    boot(plan)
                }
                _ => iced::Task::none(),
            }
        }
        Msg::FallbackAccepted => {
            if let Some(offer) = app.sandbox.fallback_offer() {
                // Consent is not the whole of it. `daemon::connection` dials from `app.placement`,
                // and for `LocalSandbox` it deliberately never falls back to a host process
                // (FR-035, and the comment in `daemon.rs` says so). So the *only* thing that can
                // turn accepted consent into a working service is moving the placement here —
                // without this the user presses "Run without it for now", the banner changes, and
                // the client goes on dialling a port nothing is listening on, forever.
                //
                // In memory only: nothing writes it back to the settings store, which is what
                // makes the choice last for this occurrence alone (FR-035a).
                if app.sandbox.accept_fallback(offer) {
                    app.placement.kind =
                        micold_core::sandbox::placement::PlacementKind::HostProcess;
                }
            }
            iced::Task::none()
        }
    }
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
