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
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use iced::Task;
use micold_client::app::Message;
use micold_client::features::sandbox::Msg as SandboxMsg;
use micold_core::endpoint::DEFAULT_SANDBOX_PORT;
use micold_core::protocol::auth::{host_token_path, Token, CONTAINER_TOKEN_PATH};
use micold_core::sandbox::cli::CliRuntime;
use micold_core::sandbox::exec::{CommandRunner, SystemRunner};
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
pub fn start<R: CommandRunner>(
    profile: &SandboxProfile,
    projects: &[PathBuf],
    facts: &HostFacts,
    port: u16,
    runner: R,
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

    // The sandbox's own home, created here rather than left to the runtime (FR-004d). A bind
    // source the runtime has to create it creates as **root**, which leaves the uid the container
    // runs as unable to write to its own `HOME` — the failure this mount exists to fix, arriving by
    // a different route. Created before `create` for the same reason the token is.
    let sandbox_home = facts.state_dir.join(micold_core::sandbox::SANDBOX_HOME_DIR);
    if let Err(e) = std::fs::create_dir_all(&sandbox_home) {
        return Err(Failure {
            stage: micold_core::sandbox::lifecycle::Stage::Creating,
            error: micold_core::sandbox::runtime::RuntimeError::Unknown {
                stderr: format!(
                    "could not create the sandbox home directory {}: {e}",
                    sandbox_home.display()
                ),
            },
        });
    }

    let mounts = MountSet::build(
        projects,
        profile,
        &facts.layout,
        facts.state_dir.clone(),
        &facts.home,
        SecretMount {
            host: token_path,
            container: PathBuf::from(CONTAINER_TOKEN_PATH),
        },
    );

    let runtime = CliRuntime::new(profile.runtime, runner);
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
pub fn boot(
    plan: BootPlan,
    running: &mut Option<InFlight>,
) -> iced::Task<micold_client::app::Message> {
    BringUp::now(plan).run(running)
}

/// Cancel the bring-up in `running`, if there is one: one still waiting its delay never starts, and
/// one under way reports nothing more (FR-036a).
///
/// A runtime call already in progress is not interrupted — `start` runs on a blocking thread — but
/// the bring-up issues no further command, and removes the container if it had already begun
/// creating or starting it. Without that, one cancelled mid-pull went on to create the container
/// after the move's own stop had found none, and left it holding the control port (#369).
pub fn cancel(running: &mut Option<InFlight>) {
    if let Some(in_flight) = running.take() {
        in_flight.cancel();
    }
}

/// A bring-up that was run, and what cancels it (see [`cancel`]).
#[derive(Debug, Clone)]
pub struct InFlight {
    task: iced::task::Handle,
    cancelled: Arc<AtomicBool>,
}

impl InFlight {
    fn cancel(self) {
        self.cancelled.store(true, Ordering::SeqCst);
        self.task.abort();
    }
}

/// The runner a bring-up drives the runtime through, which refuses every command once the bring-up
/// is cancelled and remembers whether the container was already being made (#369).
struct Cancellable<R> {
    runner: R,
    cancelled: Arc<AtomicBool>,
    launched: AtomicBool,
}

impl<R: CommandRunner> Cancellable<R> {
    fn admit(&self, args: &[std::ffi::OsString]) -> std::io::Result<()> {
        if self.cancelled.load(Ordering::SeqCst) {
            return Err(std::io::Error::new(
                std::io::ErrorKind::Interrupted,
                "the sandbox bring-up was cancelled",
            ));
        }
        // Marked before the command runs: a cancel that lands while it runs still cleans up.
        if args.first().is_some_and(|a| a == "create" || a == "start") {
            self.launched.store(true, Ordering::SeqCst);
        }
        Ok(())
    }

    /// Remove the container, if this bring-up was cancelled after it began making one. By name,
    /// because a `create` interrupted mid-way never reported an id.
    fn clean_up(&self, runtime: micold_core::sandbox::runtime::RuntimeKind) {
        use micold_core::sandbox::runtime::{ContainerId, ContainerRuntime};
        if self.cancelled.load(Ordering::SeqCst) && self.launched.load(Ordering::SeqCst) {
            let _ = CliRuntime::new(runtime, &self.runner)
                .remove(&ContainerId(CONTAINER_NAME.to_string()));
        }
    }
}

impl<R: CommandRunner> CommandRunner for Cancellable<R> {
    fn run(
        &self,
        program: &std::ffi::OsStr,
        args: &[std::ffi::OsString],
    ) -> std::io::Result<micold_core::sandbox::exec::CommandOutput> {
        self.admit(args)?;
        self.runner.run(program, args)
    }

    fn run_streaming(
        &self,
        program: &std::ffi::OsStr,
        args: &[std::ffi::OsString],
        on_line: &mut dyn FnMut(&str),
    ) -> std::io::Result<micold_core::sandbox::exec::CommandOutput> {
        self.admit(args)?;
        self.runner.run_streaming(program, args, on_line)
    }
}

/// A bring-up the application has decided to run, and when.
///
/// A value rather than a task, so that the decision — which plan, after how long — can be read back
/// by a test, where a `Task` is opaque.
#[derive(Debug, Clone)]
pub struct BringUp {
    pub plan: BootPlan,
    /// The spacing S-6 puts between unattended bring-ups; a person's restart does not wait.
    pub after: std::time::Duration,
}

#[cfg(test)]
thread_local! {
    static SCHEDULED: std::cell::RefCell<Vec<BringUp>> = const { std::cell::RefCell::new(Vec::new()) };
}

impl BringUp {
    /// A bring-up that starts at once.
    pub fn now(plan: BootPlan) -> Self {
        Self {
            plan,
            after: std::time::Duration::ZERO,
        }
    }

    /// Run it, keeping what cancels it in `running` (see [`cancel`]). A bring-up only starts when
    /// none is in flight, so the one it replaces has finished; it is cancelled regardless, so two
    /// can never run against the one container.
    pub fn run(self, running: &mut Option<InFlight>) -> iced::Task<micold_client::app::Message> {
        let cancelled = Arc::new(AtomicBool::new(false));
        let (task, handle) = self.task(cancelled.clone()).abortable();
        let in_flight = InFlight {
            task: handle,
            cancelled,
        };
        if let Some(previous) = running.replace(in_flight) {
            previous.cancel();
        }
        task
    }

    /// Run it against the real container runtime. Private: every caller goes through [`Self::run`],
    /// or the bring-up it starts cannot be cancelled.
    fn task(self, cancelled: Arc<AtomicBool>) -> iced::Task<micold_client::app::Message> {
        #[cfg(test)]
        SCHEDULED.with(|scheduled| scheduled.borrow_mut().push(self.clone()));
        #[cfg(not(test))]
        let stream = self.stream(SystemRunner, cancelled);
        // A test runs the work an update returns to see what it reports, and must never reach the
        // host's container runtime by doing so.
        #[cfg(test)]
        let stream = self.stream(
            micold_core::sandbox::exec::RecordingRunner::new(),
            cancelled,
        );
        iced::Task::stream(stream)
    }

    /// The bring-ups this test's thread turned into tasks, oldest first, and forgets them. A `Task`
    /// cannot be read back, so this is how a test tells the bring-up apart from any other work.
    #[cfg(test)]
    pub fn scheduled() -> Vec<BringUp> {
        SCHEDULED.with(|scheduled| scheduled.take())
    }

    /// The messages the bring-up produces, driving the runtime through `runner`.
    fn stream<R: CommandRunner + 'static>(
        self,
        runner: R,
        cancelled: Arc<AtomicBool>,
    ) -> impl iced::futures::Stream<Item = Message> + Send + 'static {
        let Self { plan, after } = self;
        reported(after, move |observe| {
            let facts = HostFacts::gather(plan.state_dir);
            let runner = Cancellable {
                runner,
                cancelled,
                launched: AtomicBool::new(false),
            };
            let outcome = start(
                &plan.profile,
                &plan.projects,
                &facts,
                control_port(),
                &runner,
                observe,
            )
            .map(|ready| ready.started);
            runner.clean_up(plan.profile.runtime);
            outcome
        })
    }
}

/// Run `work` off the render thread and turn what it reports into messages.
///
/// Every stage `work` enters becomes a [`SandboxMsg::Progress`], in order, and how it ended comes
/// last. Split from [`BringUp::stream`] so that ordering is testable without a container runtime.
fn reported<W>(
    after: std::time::Duration,
    work: W,
) -> impl iced::futures::Stream<Item = Message> + Send + 'static
where
    W: FnOnce(&mut dyn FnMut(SandboxState)) -> Result<Started, Failure> + Send + 'static,
{
    use iced::futures::StreamExt;

    let (tx, rx) = iced::futures::channel::mpsc::unbounded();
    let run = async move {
        if !after.is_zero() {
            tokio::time::sleep(after).await;
        }
        // Each stage goes out as it is entered. `observe` is called on the blocking thread, and an
        // unbounded send never blocks it, so a slow render cannot slow the pull (SC-004c).
        let progress = tx.clone();
        let outcome = tokio::task::spawn_blocking(move || {
            work(&mut |state| {
                let _ = progress
                    .unbounded_send(Message::Sandbox(SandboxMsg::Progress(Box::new(state))));
            })
        })
        .await;
        let _ = tx.unbounded_send(finished(outcome));
    };
    iced::futures::stream::select(
        iced::futures::stream::once(run).filter_map(|()| std::future::ready(None)),
        rx,
    )
}

/// The message a finished bring-up leaves behind.
fn finished(outcome: Result<Result<Started, Failure>, tokio::task::JoinError>) -> Message {
    match outcome {
        Ok(Ok(started)) => Message::Sandbox(SandboxMsg::Started(Box::new(started))),
        Ok(Err(failure)) => Message::Sandbox(SandboxMsg::Failed(Box::new(failure))),
        // A panicked or cancelled blocking task is still a sandbox that did not come up, and the
        // user needs the same standing banner for it as for a runtime that refused.
        Err(join) => Message::Sandbox(SandboxMsg::Failed(Box::new(Failure {
            stage: micold_core::sandbox::lifecycle::Stage::Starting,
            error: micold_core::sandbox::runtime::RuntimeError::Unknown {
                stderr: join.to_string(),
            },
        }))),
    }
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
        // A bring-up reports into the sandbox only while the sandbox is the placement: one that
        // outlived a move to the host describes a container nobody is running (FR-036b).
        Msg::Progress(_) | Msg::Started(_) | Msg::Failed(_)
            if app.placement.kind
                != micold_core::sandbox::placement::PlacementKind::LocalSandbox =>
        {
            iced::Task::none()
        }
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
            // Brought back in the same update, so `Failed` is never drawn: the card and its
            // fallback would otherwise stand for as long as the next refused dial took (FR-036b).
            if app.sandbox.container_lost(CONTAINER_NAME) {
                if let Some(bring_up) = crate::shell::daemon_sync::bring_up_again(app) {
                    return bring_up.run(&mut app.sandbox_bring_up);
                }
            }
            iced::Task::none()
        }
        Msg::Progress(state) => {
            app.sandbox.observe(*state);
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
                    app.core.update(Message::Settings(
                        micold_client::features::settings::Msg::PlacementMoved(
                            micold_core::sandbox::placement::PlacementKind::LocalSandbox,
                        ),
                    ));
                    boot(plan, &mut app.sandbox_bring_up)
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
                    // Belt and braces: a fallback is offered only from `Failed`, and a waiting
                    // bring-up has already moved the state to `Probing`, so there is normally
                    // nothing to cancel. Kept so no bring-up can outlive the move to the host.
                    cancel(&mut app.sandbox_bring_up);
                    app.placement.kind =
                        micold_core::sandbox::placement::PlacementKind::HostProcess;
                    // And tell the form, which reports where sessions run *now* rather than what
                    // the file says (FR-035b, BUG-003). This is the case that makes the two
                    // differ: the settings still say container, and the host is what is running.
                    app.core.update(Message::Settings(
                        micold_client::features::settings::Msg::PlacementMoved(
                            micold_core::sandbox::placement::PlacementKind::HostProcess,
                        ),
                    ));
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

    /// SC-004c at its source: every stage a bring-up enters has to leave the worker, in order, and
    /// before the outcome — not be handed to a callback that drops it.
    #[tokio::test]
    async fn every_stage_a_bring_up_enters_is_reported_before_how_it_ended() {
        use iced::futures::StreamExt;
        use micold_core::sandbox::lifecycle::Stage;
        use micold_core::sandbox::runtime::{Progress, RuntimeError};

        let acquiring = SandboxState::Acquiring(Progress {
            stage: "Downloading".into(),
            detail: None,
            percent: Some(10),
        });
        let failure = Failure {
            stage: Stage::Starting,
            error: RuntimeError::Timeout {
                operation: "starting the sandbox".into(),
            },
        };
        let (stages, outcome) = (
            [SandboxState::Probing, acquiring, SandboxState::Starting],
            failure.clone(),
        );
        let reported_stages = stages.clone();

        let messages: Vec<Message> = reported(std::time::Duration::ZERO, move |observe| {
            for stage in reported_stages {
                observe(stage);
            }
            Err(outcome)
        })
        .collect()
        .await;

        let mut expected: Vec<Message> = stages
            .into_iter()
            .map(|s| Message::Sandbox(SandboxMsg::Progress(Box::new(s))))
            .collect();
        expected.push(Message::Sandbox(SandboxMsg::Failed(Box::new(failure))));
        assert_eq!(messages, expected);
    }

    /// SC-004c through the production bring-up rather than a closure the test writes: the work
    /// [`BringUp`] runs has to hand its stages on, or the view is left on whatever it showed before.
    #[tokio::test]
    async fn the_production_bring_up_reports_probing_first_and_how_it_ended_last() {
        use iced::futures::StreamExt;
        use micold_core::sandbox::exec::RecordingRunner;

        let state_dir = tempfile::tempdir().expect("tempdir");
        let plan = BootPlan {
            profile: SandboxProfile::default(),
            state_dir: state_dir.path().to_path_buf(),
            projects: Vec::new(),
        };

        let messages: Vec<Message> = BringUp::now(plan)
            .stream(RecordingRunner::new(), Default::default())
            .collect()
            .await;

        assert!(
            matches!(
                messages.first(),
                Some(Message::Sandbox(SandboxMsg::Progress(stage))) if **stage == SandboxState::Probing
            ),
            "the first stage a bring-up enters has to reach the view (SC-004c): {messages:?}"
        );
        assert!(
            matches!(
                messages.last(),
                Some(Message::Sandbox(
                    SandboxMsg::Started(_) | SandboxMsg::Failed(_)
                ))
            ),
            "how the bring-up ended comes last, after every stage: {messages:?}"
        );
    }

    /// S-6's spacing at the place it is applied: an unattended bring-up is handed its delay, and the
    /// runtime must not be touched before the delay is over — or the budget's spacing is a number
    /// nobody waits for, and three attempts arrive on three consecutive connection retries.
    #[tokio::test]
    async fn a_delayed_bring_up_does_not_start_before_its_delay() {
        use iced::futures::StreamExt;
        use micold_core::sandbox::lifecycle::Stage;
        use micold_core::sandbox::runtime::RuntimeError;

        const DELAY: std::time::Duration = std::time::Duration::from_millis(200);
        let (began_after, waited) = std::sync::mpsc::channel();
        let asked = std::time::Instant::now();

        let _: Vec<Message> = reported(DELAY, move |_observe| {
            let _ = began_after.send(asked.elapsed());
            Err(Failure {
                stage: Stage::Probing,
                error: RuntimeError::Timeout {
                    operation: "probing".into(),
                },
            })
        })
        .collect()
        .await;

        let waited = waited.recv().expect("the bring-up never ran");
        assert!(
            waited >= DELAY,
            "the bring-up started {waited:?} after it was asked for, before its {DELAY:?} delay (S-6)"
        );
    }

    /// The runtime a bring-up drives, scripted as far as a created and started sandbox, which
    /// cancels the bring-up as the first command whose argv begins with `cancel_on` is issued — the
    /// moment a person moves the placement to the host while that command runs (#369).
    struct CancelledDuring {
        runtime: std::sync::Arc<micold_core::sandbox::exec::RecordingRunner>,
        cancel_on: &'static str,
        cancelled: std::sync::Arc<std::sync::atomic::AtomicBool>,
    }

    impl CancelledDuring {
        fn new(cancel_on: &'static str) -> Self {
            use micold_core::sandbox::exec::{CommandOutput, RecordingRunner};
            let fixture = |name: &str| {
                std::fs::read_to_string(
                    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                        .join("../micold-core/tests/fixtures/runtime")
                        .join(name),
                )
                .expect("runtime fixture")
            };
            // Probe (version, info), the image already present, no sandbox yet, the network,
            // create's own probe, create, start.
            let runtime = RecordingRunner::new();
            runtime.push_ok(fixture("docker_version.json"));
            runtime.push_ok(fixture("docker_info.json"));
            runtime.push_ok(fixture("docker_inspect_image.json"));
            runtime.push(Ok(CommandOutput::err(
                1,
                "Error: No such container: micold-sandbox",
            )));
            runtime.push_ok("micold-sandbox-net");
            runtime.push_ok(fixture("docker_version.json"));
            runtime.push_ok(fixture("docker_info.json"));
            runtime.push_ok("9f2b1c4d7e8a");
            runtime.push_ok("");
            Self {
                runtime: std::sync::Arc::new(runtime),
                cancel_on,
                cancelled: std::sync::Arc::default(),
            }
        }
    }

    impl CommandRunner for CancelledDuring {
        fn run(
            &self,
            program: &std::ffi::OsStr,
            args: &[std::ffi::OsString],
        ) -> std::io::Result<micold_core::sandbox::exec::CommandOutput> {
            let out = self.runtime.run(program, args);
            if args.first().is_some_and(|first| first == self.cancel_on) {
                self.cancelled
                    .store(true, std::sync::atomic::Ordering::SeqCst);
            }
            out
        }
    }

    async fn bring_up_cancelled_during(cancel_on: &'static str) -> Vec<String> {
        use iced::futures::StreamExt;
        let state_dir = tempfile::tempdir().expect("tempdir");
        let plan = BootPlan {
            profile: SandboxProfile::default(),
            state_dir: state_dir.path().to_path_buf(),
            projects: Vec::new(),
        };
        let runtime = CancelledDuring::new(cancel_on);
        let (runtime_log, cancelled) = (runtime.runtime.clone(), runtime.cancelled.clone());
        let _: Vec<Message> = BringUp::now(plan)
            .stream(runtime, cancelled)
            .collect()
            .await;
        runtime_log
            .calls()
            .iter()
            .map(|call| call.args_lossy().join(" "))
            .collect()
    }

    /// Cancelled before the container exists, a bring-up creates nothing: the move's own stop found
    /// no container, and one created afterwards would be nobody's (#369).
    #[tokio::test]
    async fn a_bring_up_cancelled_before_create_creates_no_container() {
        let commands = bring_up_cancelled_during("image").await;

        assert!(
            !commands
                .iter()
                .any(|c| c.starts_with("create") || c.starts_with("start")),
            "a cancelled bring-up went on to create the sandbox: {commands:?}"
        );
    }

    /// Cancelled once the container is being created, the bring-up removes what it made rather than
    /// leave it holding the control port (#369).
    #[tokio::test]
    async fn a_bring_up_cancelled_during_create_removes_the_container_it_made() {
        let commands = bring_up_cancelled_during("create").await;

        assert!(
            !commands.iter().any(|c| c.starts_with("start")),
            "a cancelled bring-up started the sandbox: {commands:?}"
        );
        assert_eq!(
            commands.last().map(String::as_str),
            Some("rm -f micold-sandbox"),
            "a cancelled bring-up left its container behind: {commands:?}"
        );
    }

    #[test]
    fn the_control_port_is_the_documented_default() {
        assert_eq!(control_port(), DEFAULT_SANDBOX_PORT);
    }
}
