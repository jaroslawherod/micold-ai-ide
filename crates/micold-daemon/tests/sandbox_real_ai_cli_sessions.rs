//! BUG-006, SC-012a, US2 scenario 10: a conversation recorded in the sandbox survives it.
//!
//! # Why this test exists
//!
//! The "AI CLI sign-in" share used to mount the host's whole `~/.claude` read-only, on top of the
//! sandbox's own writable home. `claude` keeps its session store in that same directory, so a
//! session started in the sandbox warned that its files were read-only and that its state would
//! be lost when `claude` restarted — and it was. The share is now the token file alone, writable
//! (FR-004e, rule N-4), and every conversation is recorded in `<state>/sandbox-home`.
//!
//! The fast suites check the mount set and the argv. Neither can check what the real runtime does
//! with a single-file mount inside another mount, whether `claude` can then write its store, or
//! whether the daemon's prune — which runs in the container and reads the container's home —
//! keeps what was written there once the sandbox is recreated. This probe checks all three.
//!
//! # What it does
//!
//! Once without the share and once with it, for every [`AiCli`]:
//!
//! 1. seed one never-labelled session per CLI, plus a control session that records nothing;
//! 2. record a conversation for each CLI's session from a shell in the sandbox;
//! 3. recreate the sandbox and attach the project, which is what makes the daemon prune;
//! 4. expect every recorded session to be kept, each transcript to be under
//!    `<state>/sandbox-home`, and the control to be pruned — the control is what shows the prune
//!    ran at all, so a kept session means something.
//!
//! With the share on, `claude`'s conversation is a real one, through the host's own sign-in: the
//! point is that the real CLI can write its store beside a token mounted into it. Every other
//! conversation is written into the CLI's layout from inside the sandbox, because Copilot and Pi
//! cannot sign in here and `claude` cannot without the share. Those still exercise the part this
//! bug broke on the other side: where the record lands, and who judges it.
//!
//! With the share on, the probe also checks that the host's `~/.claude` gained no record of these
//! sessions. Only `.credentials.json` may change there: `claude` replaces it when it refreshes.
//!
//! Behind `sandbox-real-runtime` (Principle VI: the default suite needs nothing installed). The
//! shared run needs a signed-in host `claude`, and it can rotate that sign-in's refresh token.
//!
//! ```text
//! cargo test -p micold-daemon --features sandbox-real-runtime sandbox_real_ai_cli_sessions -- --nocapture --test-threads=1
//! ```

#![cfg(all(feature = "sandbox-real-runtime", unix))]

mod sandbox_real_support;

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use futures_util::{SinkExt, StreamExt};
use micold_core::connect::DaemonConnection;
use micold_core::project::{Availability, Project};
use micold_core::protocol::auth::Token;
use micold_core::protocol::codec::Frame;
use micold_core::protocol::messages::{ClientMsg, DaemonMsg};
use micold_core::sandbox::{
    CredentialLayout, CredentialShare, MountSet, SandboxProfile, SecretMount, SANDBOX_HOME_DIR,
};
use micold_core::session::{
    AiCli, Session, SessionId, SessionLabel, SessionLocation, TerminalMode,
};
use micold_core::store::ProjectStore;
use micold_core::workspace::Workspace;

use sandbox_real_support::{
    credentials, input_serial, open_session, start_sandbox, wait_for_accept, SandboxSpec, Terminal,
};

/// Pinned for the reason the other probes pin it.
const SESSION_SHELL: &str = "/bin/sh";

#[tokio::test]
async fn sandbox_real_ai_cli_sessions_survive_the_sandbox_without_the_sign_in() {
    probe(false, "micold-sessions-probe", 17750).await;
}

#[tokio::test]
async fn sandbox_real_ai_cli_sessions_survive_the_sandbox_with_the_sign_in_shared() {
    probe(true, "micold-sessions-shared-probe", 17751).await;
}

/// The host's own sign-in token, as the application's layout names it.
///
/// Read through `CredentialLayout` rather than spelled here, so that a layout which moves the
/// token moves this probe's precondition with it.
fn host_sign_in() -> Option<PathBuf> {
    let home = std::env::var("HOME").ok()?;
    CredentialLayout::conventional(Path::new(&home), None).ai_cli_auth
}

/// The sessions one run seeds: a shell to type into, one pending session per CLI, and a control.
struct Seeded {
    shell: SessionId,
    per_cli: Vec<(AiCli, SessionId)>,
    control: SessionId,
}

async fn probe(shared: bool, container: &str, port: u16) {
    // The shared run needs a signed-in host `claude`, which a CI runner does not have and cannot
    // be given: the token is the user's. Skipped rather than failed, because the question this
    // probe asks — does a conversation recorded in the sandbox survive it — is answered for the
    // unshared run on every machine, and for the shared run on a developer's own.
    if shared && !host_sign_in().is_some_and(|token| token.exists()) {
        eprintln!("skipped: no signed-in host `claude` to share. Sign in on this host to run it.");
        return;
    }
    let network = format!("{container}-net");
    let dir = tempfile::tempdir().unwrap();
    let data = dir.path().join("data");
    let project = dir.path().join("project");
    std::fs::create_dir_all(&project).unwrap();
    let state = data.join("micold-ai-ide");
    let seeded = seed(&state, &project);

    let token = Token::generate();
    let token_path = state.join("sandbox.token");
    token.write_to(&token_path).unwrap();
    let home = std::env::var("HOME").expect("HOME");
    let sandbox_home = state.join(SANDBOX_HOME_DIR);

    // The share as the application renders it: from the mount set, not spelled here, so this probe
    // cannot go on passing for a share that the application has since widened again.
    let extra = if shared {
        let mounts = MountSet::build(
            &[project.clone()],
            &SandboxProfile {
                credentials: BTreeSet::from([CredentialShare::AiCliAuth]),
                ..SandboxProfile::default()
            },
            &CredentialLayout::conventional(Path::new(&home), None),
            state.clone(),
            Path::new(&home),
            SecretMount {
                host: token_path.clone(),
                container: PathBuf::from("/run/micold/token"),
            },
        );
        assert_eq!(
            mounts.home.host, sandbox_home,
            "the harness and the app disagree on the home"
        );
        for dir in mounts.home_dirs_to_create() {
            std::fs::create_dir_all(dir).unwrap();
        }
        mounts
            .credentials
            .iter()
            .flat_map(|c| {
                assert!(
                    c.host.exists(),
                    "{} is missing: the shared run needs a signed-in host `claude`",
                    c.host.display()
                );
                let mode = if c.share.writable() { "rw" } else { "ro" };
                [
                    "-v".to_string(),
                    format!("{}:{}:{mode}", c.host.display(), c.container.display()),
                ]
            })
            .collect()
    } else {
        Vec::new()
    };

    // SAFETY: set before any spawn; the tests in this binary run one at a time.
    std::env::set_var("SHELL", SESSION_SHELL);
    let spec = SandboxSpec {
        container,
        network: &network,
        port,
        data_home: &data,
        project: &project,
        token_path: &token_path,
        home: &home,
        survive_logout: false,
        extra: &extra,
    };
    let log = state.join("micold-daemon.log");

    // 1–2. Record a conversation for every CLI's session, from a shell in the sandbox.
    {
        let _sandbox = start_sandbox(&spec);
        let (mut conn, catalog) = wait_for_accept(port, &credentials(&token)).await;
        let serial = input_serial(&catalog, seeded.shell);
        let screen = open_session(&mut conn, &project, seeded.shell, &log).await;
        let mut term = Terminal::new(&mut conn, seeded.shell, screen, container, &log, serial);

        for &(which, id) in &seeded.per_cli {
            if shared && which == AiCli::ClaudeCode {
                let printed = term
                    .run_within(
                        &format!(
                            "cd '{}' && claude -p 'Reply with the single word hi.' --session-id {}; echo rc=$?",
                            project.display(),
                            id.0
                        ),
                        Duration::from_secs(180),
                    )
                    .await;
                // Printed for the record: `--nocapture` shows what the real CLI answered.
                eprintln!("claude in the sandbox printed:\n{printed}");
                let lower = printed.to_lowercase();
                assert!(
                    !lower.contains("read-only") && !lower.contains("readonly"),
                    "`claude` still warns that its store is read-only in the sandbox:\n{printed}"
                );
                assert!(
                    printed.contains("rc=0"),
                    "`claude` could not hold a conversation with the shared sign-in:\n{printed}"
                );
            } else {
                let (dir, file) = layout_in_sandbox(which, &home, &project, id);
                let printed = term
                    .run(&format!(
                        "mkdir -p '{dir}' && printf '{{}}\\n' > '{dir}/{file}'; echo rc=$?"
                    ))
                    .await;
                assert!(
                    printed.contains("rc=0"),
                    "{which:?}: could not record:\n{printed}"
                );
            }
        }
    }

    // Every record is in the sandbox's own home, where the CLI's own layout says it belongs.
    for &(which, id) in &seeded.per_cli {
        let config = sandbox_home.join(config_under_home(which));
        assert!(
            which
                .provider()
                .has_recorded_conversation(&config, &project, id.0),
            "{which:?}: no conversation under {} (shared: {shared})",
            config.display()
        );
    }

    // 3. Recreate the sandbox and attach, which is when the daemon prunes.
    let kept = {
        let _sandbox = start_sandbox(&spec);
        let (mut conn, _) = wait_for_accept(port, &credentials(&token)).await;
        attach(&mut conn, &project, &log).await
    };

    // 4. The recorded sessions are kept, and the control shows the prune did run.
    for &(which, id) in &seeded.per_cli {
        assert!(
            kept.contains(&id),
            "{which:?}: the recreated sandbox pruned a session with a recorded conversation \
             (shared: {shared}). SC-012a: it must be kept.\n--- daemon log ---\n{}",
            std::fs::read_to_string(&log).unwrap_or_default()
        );
    }
    assert!(
        !kept.contains(&seeded.control),
        "the control session, which recorded nothing, was kept: the prune did not run, so the \
         sessions it kept prove nothing"
    );

    // With the share on, the host's `~/.claude` must hold no record of these sessions.
    if shared {
        let host_claude = Path::new(&home).join(".claude");
        let encoded = encode_for_claude(&project);
        assert!(
            !host_claude.join("projects").join(&encoded).exists(),
            "the sandbox wrote a conversation into the host's ~/.claude/projects/{encoded}"
        );
        let ids: Vec<String> = seeded
            .per_cli
            .iter()
            .map(|(_, id)| id.0.to_string())
            .collect();
        let leaked = names_under(&host_claude.join("projects"))
            .into_iter()
            .filter(|name| ids.iter().any(|id| name.contains(id)))
            .collect::<Vec<_>>();
        assert!(leaked.is_empty(), "the host's ~/.claude holds {leaked:?}");
    }
}

fn seed(state: &Path, project: &Path) -> Seeded {
    std::fs::create_dir_all(state).unwrap();
    let session = |id, label, mode, which| {
        Session::restored(id, SessionLocation::Default, label, mode, which)
    };
    let shell = SessionId::new();
    let control = SessionId::new();
    let per_cli: Vec<(AiCli, SessionId)> = AiCli::ALL
        .iter()
        .map(|&which| (which, SessionId::new()))
        .collect();

    // `Pending` is what makes a session a prune candidate: one the AI CLI has not named yet.
    let mut sessions = vec![
        session(
            shell,
            SessionLabel::Named("shell".into()),
            TerminalMode::Regular,
            AiCli::default(),
        ),
        session(
            control,
            SessionLabel::Pending,
            TerminalMode::AiCli,
            AiCli::default(),
        ),
    ];
    sessions.extend(
        per_cli
            .iter()
            .map(|&(which, id)| session(id, SessionLabel::Pending, TerminalMode::AiCli, which)),
    );
    let workspace = Workspace {
        projects: vec![Project::new(
            project.to_path_buf(),
            false,
            Availability::Available,
        )],
        active: Some(project.to_path_buf()),
        sessions: BTreeMap::from([(project.to_path_buf(), sessions)]),
        worktree_names: BTreeMap::new(),
        ..Default::default()
    };
    micold_core::store::JsonFileStore::at(state.join("projects.json"))
        .save(&workspace)
        .unwrap();
    Seeded {
        shell,
        per_cli,
        control,
    }
}

/// Each CLI's configuration directory, relative to the home it runs under.
fn config_under_home(which: AiCli) -> PathBuf {
    match which {
        AiCli::ClaudeCode => PathBuf::from(".claude"),
        AiCli::Copilot => PathBuf::from(".copilot"),
        AiCli::Pi => Path::new(".pi").join("agent"),
    }
}

/// Where each CLI records a conversation, as the directory and file name inside the sandbox.
///
/// A second copy of the layouts in `provider.rs`, which is the price of writing a record the CLI
/// would have written. The host-side check above reads it back through the provider's own
/// `has_recorded_conversation`, so a copy that drifted fails there rather than passing quietly.
fn layout_in_sandbox(which: AiCli, home: &str, cwd: &Path, id: SessionId) -> (String, String) {
    let config = Path::new(home).join(config_under_home(which));
    let (dir, file) = match which {
        AiCli::ClaudeCode => (
            config.join("projects").join(encode_for_claude(cwd)),
            format!("{}.jsonl", id.0),
        ),
        AiCli::Copilot => (
            config.join("session-state").join(id.0.to_string()),
            "events.jsonl".to_string(),
        ),
        AiCli::Pi => {
            let body = cwd
                .to_string_lossy()
                .trim_start_matches('/')
                .replace('/', "-");
            (
                config.join("sessions").join(format!("--{body}--")),
                format!("2026-09-19T00-00-00-000Z_{}.jsonl", id.0),
            )
        }
    };
    (dir.display().to_string(), file)
}

fn encode_for_claude(cwd: &Path) -> String {
    cwd.to_string_lossy()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect()
}

fn names_under(dir: &Path) -> Vec<String> {
    let mut names = Vec::new();
    let Ok(entries) = std::fs::read_dir(dir) else {
        return names;
    };
    for entry in entries.flatten() {
        names.push(entry.file_name().to_string_lossy().into_owned());
        if entry.file_type().is_ok_and(|kind| kind.is_dir()) {
            names.extend(names_under(&entry.path()));
        }
    }
    names
}

/// Attach `project` and return the ids of its sessions that are still listed afterwards.
async fn attach(conn: &mut DaemonConnection, project: &Path, log: &Path) -> BTreeSet<SessionId> {
    conn.send(Frame::Control(ClientMsg::Attach {
        project: project.to_path_buf(),
        force: true,
    }))
    .await
    .expect("attach");
    let deadline = Instant::now() + Duration::from_secs(30);
    loop {
        let remaining = deadline.saturating_duration_since(Instant::now());
        match tokio::time::timeout(remaining, conn.next()).await {
            Ok(Some(Ok(Frame::Control(DaemonMsg::Attached { sessions, .. })))) => {
                return sessions.into_iter().map(|s| s.id).collect()
            }
            Ok(Some(Ok(_))) => continue,
            Ok(Some(Err(e))) => panic!("stream error while attaching: {e}"),
            Ok(None) => panic!("the daemon closed the connection while attaching"),
            Err(_) => panic!(
                "no `Attached` within 30s.\n--- daemon log ---\n{}",
                std::fs::read_to_string(log).unwrap_or_default()
            ),
        }
    }
}
