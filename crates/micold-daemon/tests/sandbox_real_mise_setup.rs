//! #1439, Story 1 (quickstart A7): on a container daemon on the basic image, a mise project's
//! setup command `mise trust && mise install` installs its toolchain into the sandbox home, and a
//! session builds and tests a crate whose build script compiles C code.
//!
//! One test, in the order a user meets it, on the fixture `tests/fixtures/mise-c-crate` (rust
//! pinned with the minimal profile, a `build.rs` that runs `cc` and `ar`, no crates.io dependency):
//!
//! 1. the first worktree's setup is stopped while it downloads the toolchain, and the next run in
//!    that worktree succeeds (edge case "Network failure during setup": a partial install does
//!    not break the rerun; the failure report itself is #1289's);
//! 2. as the non-root session user, `cargo build` and `cargo test` succeed in a terminal and
//!    `cargo` resolves under the sandbox home (AS1.2, AS1.3, FR-005);
//! 3. a second worktree's setup ends in under a minute and downloads nothing (AS1.5, SC-003);
//! 4. a project with no `mise.toml` sets up with the same command (edge case "Empty project");
//! 5. the setups wrote nothing outside the sandbox home and the projects (FR-010);
//! 6. after a container restart `cargo` is still found and runs (AS1.5).
//!
//! Behind `sandbox-real-runtime` and needs the network (mise and rustup download the toolchain).
//!
//! ```text
//! cargo test --release -p micold-daemon --features sandbox-real-runtime --test sandbox_real_mise_setup -- --nocapture
//! ```

#![cfg(all(feature = "sandbox-real-runtime", unix))]

mod sandbox_real_support;

use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant};

use futures_util::{SinkExt, StreamExt};
use micold_core::protocol::codec::Frame;
use micold_core::protocol::messages::{ClientMsg, DaemonMsg, OperationResult};
use micold_core::session::{SessionLocation, TerminalMode};
use micold_core::setup::{RunId, SetupCommand, SetupState};
use micold_core::worktree::CreateMode;
use sandbox_real_support::{
    cli, input_serial, open_session, seed_projects, start_sandbox, wait_for_accept, SandboxSpec,
    Terminal,
};

const CONTAINER: &str = "micold-mise-setup-probe";
const NETWORK: &str = "micold-mise-setup-probe-net";
const PORT: u16 = 17791;

/// Pinned for the reason the other probes pin it: an inherited `SHELL` is the developer's.
const SESSION_SHELL: &str = "/bin/sh";

/// The setup command the sandbox page and CONTRIBUTING.md give for a mise project.
const SETUP: &str = "mise trust && mise install";

/// A first toolchain download on a CI runner takes well under this; the bound only stops a hang.
const INSTALL_BOUND: Duration = Duration::from_secs(15 * 60);

/// SC-003: a second worktree of the same project sets up in under a minute.
const SECOND_SETUP: Duration = Duration::from_secs(60);

const ANSWER: Duration = Duration::from_secs(30);

/// How long the first setup may take to show its toolchain install under way (mise resolving the
/// version, then rustup starting); generous for a slow runner, it only stops a hang.
const INSTALL_STARTS: Duration = Duration::from_secs(5 * 60);

fn git(dir: &Path, args: &[&str]) {
    let out = Command::new("git")
        .current_dir(dir)
        .args([
            "-c",
            "user.name=probe",
            "-c",
            "user.email=probe@example.invalid",
            "-c",
            "commit.gpgsign=false",
        ])
        .args(args)
        .output()
        .expect("git runs");
    assert!(
        out.status.success(),
        "git {args:?} in {}: {}",
        dir.display(),
        String::from_utf8_lossy(&out.stderr)
    );
}

/// A git repository at `dir` holding the files of `from` (none when `None`) in one commit.
fn repo(dir: &Path, from: Option<&Path>) {
    std::fs::create_dir_all(dir).unwrap();
    if let Some(from) = from {
        copy_tree(from, dir);
    } else {
        std::fs::write(dir.join("README"), "a project with no mise.toml\n").unwrap();
    }
    git(dir, &["init", "-q", "-b", "main"]);
    git(dir, &["add", "-A"]);
    git(dir, &["commit", "-qm", "fixture"]);
}

fn copy_tree(from: &Path, to: &Path) {
    for entry in std::fs::read_dir(from).unwrap() {
        let entry = entry.unwrap();
        let target = to.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            std::fs::create_dir_all(&target).unwrap();
            copy_tree(&entry.path(), &target);
        } else {
            std::fs::copy(entry.path(), &target).unwrap();
        }
    }
}

fn worktree(project: &Path, dir: &str) -> PathBuf {
    project.join(".claude/worktrees").join(dir)
}

fn wt(dir: &str) -> SessionLocation {
    SessionLocation::Worktree(dir.into())
}

/// The daemon's messages that arrived while a test waited for another, so none is lost.
#[derive(Default)]
struct Pending(Vec<DaemonMsg>);

impl Pending {
    /// The first control message, held or arriving within `within`, that `pick` accepts; the
    /// session's grid frames keep the terminal's screen current meanwhile.
    async fn take<T>(
        &mut self,
        term: &mut Terminal<'_>,
        within: Duration,
        what: &str,
        mut pick: impl FnMut(&DaemonMsg) -> Option<T>,
    ) -> T {
        if let Some(i) = self.0.iter().position(|m| pick(m).is_some()) {
            return pick(&self.0.remove(i)).expect("picked");
        }
        let deadline = Instant::now() + within;
        loop {
            let remaining = deadline.saturating_duration_since(Instant::now());
            match tokio::time::timeout(remaining, term.conn.next()).await {
                Ok(Some(Ok(Frame::Grid(frame)))) => {
                    if frame.session == term.session {
                        term.screen.apply(&frame);
                    }
                }
                Ok(Some(Ok(Frame::Control(msg)))) => match pick(&msg) {
                    Some(found) => return found,
                    None => self.0.push(msg),
                },
                Ok(Some(Err(e))) => panic!("stream error while waiting for {what}: {e}"),
                Ok(None) => panic!("the daemon closed the connection while waiting for {what}"),
                Err(_) => {
                    let tail = &self.0[self.0.len().saturating_sub(20)..];
                    panic!("no {what} within {within:?}; the last messages: {tail:#?}")
                }
            }
        }
    }

    async fn request(&mut self, term: &mut Terminal<'_>, msg: ClientMsg, req: u64) -> DaemonMsg {
        term.conn.send(Frame::Control(msg)).await.expect("sent");
        self.take(
            term,
            ANSWER,
            &format!("the answer to request {req}"),
            |m| match m {
                DaemonMsg::OperationOk { req: r, .. }
                | DaemonMsg::OperationError { req: r, .. }
                    if *r == req =>
                {
                    Some(m.clone())
                }
                _ => None,
            },
        )
        .await
    }

    async fn ack(&mut self, term: &mut Terminal<'_>, msg: ClientMsg, req: u64) {
        let answer = self.request(term, msg, req).await;
        assert!(
            matches!(
                answer,
                DaemonMsg::OperationOk {
                    result: OperationResult::Ack,
                    ..
                }
            ),
            "request {req} is acknowledged: {answer:?}"
        );
    }

    async fn set_command(&mut self, term: &mut Terminal<'_>, req: u64, project: &Path) {
        let command = Some(SetupCommand::parse(SETUP).expect("a command"));
        self.ack(
            term,
            ClientMsg::ProjectSetupSet {
                req,
                project: project.to_path_buf(),
                command,
            },
            req,
        )
        .await;
    }

    /// Create worktree `dir` (whose creation starts its setup) and watch its setup output.
    async fn create(&mut self, term: &mut Terminal<'_>, req: u64, project: &Path, dir: &str) {
        let msg = ClientMsg::WorktreeCreate {
            req,
            project: project.to_path_buf(),
            branch: dir.into(),
            dir_name: dir.into(),
            mode: CreateMode::NewBranch,
            session: None,
        };
        match self.request(term, msg, req).await {
            DaemonMsg::OperationOk {
                result: OperationResult::WorktreeCreated { .. },
                ..
            } => {}
            other => panic!("worktree {dir} is created: {other:?}"),
        }
        term.conn
            .send(Frame::Control(ClientMsg::SetupOutputWatch {
                project: project.to_path_buf(),
                location: wt(dir),
                from_line: 0,
            }))
            .await
            .expect("watch");
    }

    /// The id of the run that started at `location`.
    async fn started(&mut self, term: &mut Terminal<'_>, location: &SessionLocation) -> RunId {
        self.take(
            term,
            ANSWER,
            &format!("a running setup in {location:?}"),
            |m| match m {
                DaemonMsg::SetupChanged {
                    location: l,
                    run: Some(s),
                    ..
                } if l == location && *s.run.state() == SetupState::Running => Some(s.run.id()),
                _ => None,
            },
        )
        .await
    }

    /// Run `id`'s watched output at `location` until a line satisfies `pred`, all of it so far.
    /// Fails when the run ends first, so a caller knows the run is still going.
    async fn output_until(
        &mut self,
        term: &mut Terminal<'_>,
        location: &SessionLocation,
        id: RunId,
        within: Duration,
        what: &str,
        pred: impl Fn(&str) -> bool,
    ) -> Vec<String> {
        let deadline = Instant::now() + within;
        let mut lines: Vec<String> = Vec::new();
        while !lines.iter().any(|l| pred(l)) {
            let remaining = deadline.saturating_duration_since(Instant::now());
            let next = self
                .take(term, remaining, what, |m| {
                    match m {
                        DaemonMsg::SetupOutput {
                            location: l,
                            run,
                            first_line,
                            lines,
                            ..
                        } if l == location && *run == id => Ok(Some((*first_line, lines.clone()))),
                        DaemonMsg::SetupChanged {
                            location: l,
                            run: Some(s),
                            ..
                        } if l == location && s.run.id() == id && s.run.state().is_ended() => {
                            Err(s.run.state().clone())
                        }
                        _ => Ok(None),
                    }
                    .transpose()
                })
                .await;
            match next {
                Ok((first, batch)) => append(&mut lines, first, batch),
                Err(state) => panic!("run {id:?} ended as {state:?} before {what}: {lines:#?}"),
            }
        }
        lines
    }

    /// How run `id` at `location` ended, and its whole output (the reason line last), after the
    /// `lines` of it already taken.
    async fn ended(
        &mut self,
        term: &mut Terminal<'_>,
        location: &SessionLocation,
        id: RunId,
        within: Duration,
        mut lines: Vec<String>,
    ) -> (SetupState, Vec<String>) {
        let what = format!("the end of setup run {id:?} in {location:?}");
        let state = self
            .take(term, within, &what, |m| match m {
                DaemonMsg::SetupChanged {
                    location: l,
                    run: Some(s),
                    ..
                } if l == location && s.run.id() == id && s.run.state().is_ended() => {
                    Some(s.run.state().clone())
                }
                _ => None,
            })
            .await;
        let reason = state.reason_line().expect("an ended run has a reason line");
        while lines.last() != Some(&reason) {
            let (first, batch) = self
                .take(
                    term,
                    ANSWER,
                    &format!("the output of {what}"),
                    |m| match m {
                        DaemonMsg::SetupOutput {
                            location: l,
                            run,
                            first_line,
                            lines,
                            ..
                        } if l == location && *run == id => Some((*first_line, lines.clone())),
                        _ => None,
                    },
                )
                .await;
            append(&mut lines, first, batch);
        }
        (state, lines)
    }
}

/// Appends a watched output batch whose first line has index `first`. A batch may repeat lines
/// already held (the backlog of a watch); each index is kept once.
fn append(lines: &mut Vec<String>, first: u64, batch: Vec<String>) {
    let skip = (lines.len() as u64).saturating_sub(first) as usize;
    lines.extend(batch.into_iter().skip(skip));
}

#[tokio::test]
async fn sandbox_real_mise_setup_builds_a_mise_project_and_keeps_its_toolchain() {
    let dir = tempfile::tempdir().unwrap();
    let data = dir.path().join("data");
    // Both projects live under one mounted folder with no mise.toml of its own, so the empty
    // project really has none above it either.
    let projects = dir.path().join("projects");
    let crate_project = projects.join("mise-c-crate");
    let empty_project = projects.join("empty");
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/mise-c-crate");
    repo(&crate_project, Some(&fixture));
    repo(&empty_project, None);
    let session = seed_projects(
        &data,
        &[(&crate_project, true), (&empty_project, true)],
        "mise-setup-probe",
        TerminalMode::Regular,
    );
    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".into());

    // SAFETY: set before any spawn in this single-test binary.
    std::env::set_var("SHELL", SESSION_SHELL);

    let _sandbox = start_sandbox(&SandboxSpec {
        container: CONTAINER,
        network: NETWORK,
        port: PORT,
        data_home: &data,
        project: &projects,
        home: &home,
        survive_logout: false,
        extra: &[],
    });

    let log = data.join("micold-ai-ide").join("micold-daemon.log");
    {
        let (mut conn, catalog) = wait_for_accept(PORT).await;
        let serial = input_serial(&catalog, session);
        let screen = open_session(&mut conn, &crate_project, session, &log).await;
        let mut term = Terminal::new(&mut conn, session, screen, CONTAINER, &log, serial);
        let mut pending = Pending::default();

        let uid = term.run("id -u").await;
        assert_ne!(uid.trim(), "0", "the session user is root");
        let sandbox_home = term
            .run("printf '%s\\n' \"$HOME\"")
            .await
            .trim()
            .to_string();
        // FR-010: everything newer than this marker must be under the home or the projects.
        term.run("touch \"$HOME/.mise-setup-probe-marker\"").await;

        pending.set_command(&mut term, 1, &crate_project).await;
        pending.set_command(&mut term, 2, &empty_project).await;

        // 1. Stopped while the toolchain downloads, then run again.
        let first = wt("first");
        pending.create(&mut term, 3, &crate_project, "first").await;
        let id = pending.started(&mut term, &first).await;
        // Stop once mise or rustup reports the install under way (a `rust@` progress line or a
        // rustup one); `output_until` fails if the run ended before that.
        let begun = pending
            .output_until(
                &mut term,
                &first,
                id,
                INSTALL_STARTS,
                "a line showing the toolchain install under way",
                |l| {
                    let l = l.to_lowercase();
                    l.contains("rust@") || l.contains("rustup") || l.contains("download")
                },
            )
            .await;
        let stop = ClientMsg::SetupStop {
            req: 4,
            project: crate_project.clone(),
            location: first.clone(),
        };
        pending.ack(&mut term, stop, 4).await;
        let (state, lines) = pending.ended(&mut term, &first, id, ANSWER, begun).await;
        assert_eq!(state, SetupState::Stopped, "the first run: {lines:#?}");

        let again = ClientMsg::SetupRun {
            req: 5,
            project: crate_project.clone(),
            location: first.clone(),
        };
        pending.ack(&mut term, again, 5).await;
        let id = pending.started(&mut term, &first).await;
        let (state, lines) = pending
            .ended(&mut term, &first, id, INSTALL_BOUND, Vec::new())
            .await;
        assert_eq!(
            state,
            SetupState::Succeeded,
            "the run after a stopped one: {lines:#?}"
        );

        // 2. Build and test in a terminal, as the session user.
        let first_dir = worktree(&crate_project, "first");
        let built = term
            .run_within(
                &format!(
                    "cd '{}' && cargo build && cargo test && cargo run -q",
                    first_dir.display()
                ),
                Duration::from_secs(10 * 60),
            )
            .await;
        assert!(
            built.contains("test result: ok. 1 passed") && built.contains("answer 42"),
            "cargo build, test and run in the worktree: {built}"
        );
        let cargo = term.run("command -v cargo").await;
        assert!(
            cargo.trim().starts_with(&format!("{sandbox_home}/")),
            "`command -v cargo` is not under the sandbox home {sandbox_home}: {cargo}"
        );

        // 3. A second worktree reuses the toolchain: no download, under a minute. "No download" is
        // read from the home, not the output's wording: no file in a toolchain, install or
        // download folder (rustup's, mise's) is newer than a marker made just before.
        term.run("touch \"$HOME/.mise-setup-second-marker\"").await;
        let second = wt("second");
        let began = Instant::now();
        pending.create(&mut term, 6, &crate_project, "second").await;
        let id = pending.started(&mut term, &second).await;
        let (state, lines) = pending
            .ended(&mut term, &second, id, SECOND_SETUP, Vec::new())
            .await;
        let took = began.elapsed();
        assert_eq!(
            state,
            SetupState::Succeeded,
            "the second worktree: {lines:#?}"
        );
        assert!(
            took < SECOND_SETUP,
            "the second worktree's setup took {took:?}"
        );
        let fetched = term
            .run_within(
                "find \"$HOME\" -type f -newer \"$HOME/.mise-setup-second-marker\" \\( \
                 -path '*/toolchains/*' -o -path '*/installs/*' -o -path '*/downloads/*' \\) \
                 -printf 'NEWER''FILE %p\\n' 2>/dev/null",
                Duration::from_secs(60),
            )
            .await;
        // The command wraps on the terminal, so its echo can leak into the output: count only the
        // lines `-printf` wrote, whose tag is typed split so the echo never carries it whole.
        let fetched: Vec<&str> = fetched
            .lines()
            .filter(|l| l.contains("NEWERFILE "))
            .collect();
        assert!(
            fetched.is_empty(),
            "the second worktree's setup installed or downloaded:\n{fetched:#?}\noutput: {lines:#?}"
        );

        // 4. A project with no mise.toml.
        let bare = wt("bare");
        pending.create(&mut term, 7, &empty_project, "bare").await;
        let id = pending.started(&mut term, &bare).await;
        let (state, lines) = pending
            .ended(&mut term, &bare, id, ANSWER, Vec::new())
            .await;
        assert_eq!(
            state,
            SetupState::Succeeded,
            "the empty project: {lines:#?}"
        );

        // 5. FR-010: no file outside the home, the projects and the daemon's state is newer than
        // the marker. `-xdev` keeps it on the container's own file system; the temp folders are
        // left out (FR-014 excepts temp files, and /tmp need not be a tmpfs).
        let outside = term
            .run_within(
                &format!(
                    "find / -xdev \\( -path /proc -o -path /sys -o -path /dev -o -path /run \
                     -o -path /tmp -o -path /var/tmp \
                     -o -path '{sandbox_home}' -o -path '{}' -o -path '{}' \\) -prune \
                     -o -type f -newer \"$HOME/.mise-setup-probe-marker\" \
                     -printf 'NEWER''FILE %p\\n' 2>/dev/null",
                    projects.display(),
                    sandbox_real_support::CONTAINER_STATE
                ),
                Duration::from_secs(120),
            )
            .await;
        let outside: Vec<&str> = outside
            .lines()
            .filter(|l| l.contains("NEWERFILE "))
            .collect();
        assert!(
            outside.is_empty(),
            "setup wrote outside the sandbox home and the projects:\n{outside:#?}"
        );
    }

    // 6. After a restart the toolchain is still there.
    cli(&["restart", "-t", "1", CONTAINER]);
    let (mut conn, catalog) = wait_for_accept(PORT).await;
    let serial = input_serial(&catalog, session);
    let screen = open_session(&mut conn, &crate_project, session, &log).await;
    let mut term = Terminal::new(&mut conn, session, screen, CONTAINER, &log, serial);
    let after = term
        .run_within(
            &format!(
                "cd '{}' && command -v cargo && cargo --version",
                worktree(&crate_project, "first").display()
            ),
            Duration::from_secs(60),
        )
        .await;
    assert!(
        after.contains("cargo 1.90.0"),
        "cargo is not found after a container restart: {after}"
    );
}
