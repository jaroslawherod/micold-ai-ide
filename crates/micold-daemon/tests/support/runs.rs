//! Shared support for the run-group tests (feature 483, contracts/run-group-wire.md): a real git
//! repository whose catalog and runs file live in a store directory, a stand-in `claude` that
//! records what is typed into it, and windows connected over in-memory duplexes.
#![allow(dead_code)]

use std::ffi::OsString;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Arc;
use std::time::{Duration, Instant};

use futures_util::{SinkExt, StreamExt};
use micold_core::naming::{ConventionalType, WorktreeNaming};
use micold_core::project::{Availability, Project};
use micold_core::protocol::codec::{ClientCodec, Frame};
use micold_core::protocol::messages::{ClientMsg, DaemonMsg, ErrorKind, OperationResult};
use micold_core::protocol::version::{
    BUILD_FINGERPRINT, PACKAGE_VERSION, PROTOCOL_VERSION, SCHEMA_HASH,
};
use micold_core::runs::store::RunsFile;
use micold_core::runs::{GroupId, RunGroup, RunStatus};
use micold_core::session::{AiCli, SessionId};
use micold_core::settings::JsonFileSettingsStore;
use micold_core::store::{JsonFileStore, ProjectStore};
use micold_core::workspace::Workspace;
use micold_daemon::catalog::Catalog;
use micold_daemon::state::DaemonState;
use tokio_util::codec::Framed;

/// The tests change process-wide variables (`PATH`, `HOME`); one at a time.
pub static ENV: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

/// How long a test waits for something it expects.
pub const BOUND: Duration = Duration::from_secs(30);
/// How long a test waits to be sure something does not come.
pub const QUIET: Duration = Duration::from_millis(500);

/// The prompt every test sends. Distinctive, so a log line holding it is easy to find.
pub const PROMPT: &str = "Add a login page with secret-sauce-483";

pub type Client = Framed<tokio::io::DuplexStream, ClientCodec>;

/// Run `git -C repo args…`, asserting success; its trimmed standard output.
pub fn git(repo: &Path, args: &[&str]) -> String {
    let out = Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(args)
        .output()
        .expect("git runs");
    assert!(
        out.status.success(),
        "git {args:?} failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout).trim().to_owned()
}

/// Whether local branch `branch` exists in `repo`.
pub fn branch_exists(repo: &Path, branch: &str) -> bool {
    Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(["rev-parse", "--verify", "--quiet"])
        .arg(format!("refs/heads/{branch}"))
        .output()
        .expect("git runs")
        .status
        .success()
}

/// Every variable a test changes, restored on drop.
struct Env {
    saved: Vec<(&'static str, Option<OsString>)>,
}

impl Env {
    fn set(vars: &[(&'static str, Option<OsString>)]) -> Self {
        let saved = vars
            .iter()
            .map(|(name, value)| {
                let previous = std::env::var_os(name);
                match value {
                    Some(value) => std::env::set_var(name, value),
                    None => std::env::remove_var(name),
                }
                (*name, previous)
            })
            .collect();
        Self { saved }
    }
}

impl Drop for Env {
    fn drop(&mut self) {
        for (name, previous) in self.saved.drain(..) {
            match previous {
                Some(value) => std::env::set_var(name, value),
                None => std::env::remove_var(name),
            }
        }
    }
}

/// A repository on `main` with a branch `base` one commit ahead, its catalog in a store directory,
/// and a stand-in `claude` on `PATH` (no `copilot`).
pub struct Sandbox {
    _env: Env,
    pub bin: tempfile::TempDir,
    pub home: tempfile::TempDir,
    pub repo: tempfile::TempDir,
    pub store: tempfile::TempDir,
    pub state: Arc<DaemonState>,
}

impl Sandbox {
    pub fn new() -> Self {
        let bin = tempfile::tempdir().unwrap();
        let home = tempfile::tempdir().unwrap();
        let repo = tempfile::tempdir().unwrap();
        let store = tempfile::tempdir().unwrap();
        install_claude(bin.path());
        git(repo.path(), &["init", "-q", "-b", "main"]);
        git(repo.path(), &["config", "user.email", "t@t.test"]);
        git(repo.path(), &["config", "user.name", "T"]);
        git(repo.path(), &["commit", "-q", "--allow-empty", "-m", "root"]);
        git(repo.path(), &["branch", "base"]);
        git(repo.path(), &["checkout", "-q", "base"]);
        git(repo.path(), &["commit", "-q", "--allow-empty", "-m", "ahead"]);
        git(repo.path(), &["checkout", "-q", "main"]);
        let trust = serde_json::json!({"projects": {repo.path().to_str().unwrap(): {"hasTrustDialogAccepted": true}}});
        std::fs::write(home.path().join(".claude.json"), trust.to_string()).unwrap();
        let mut path = vec![bin.path().to_path_buf()];
        path.extend(["/usr/bin", "/bin"].map(PathBuf::from));
        let env = Env::set(&[
            ("PATH", Some(std::env::join_paths(path).unwrap())),
            ("HOME", Some(home.path().into())),
            (
                "XDG_DATA_HOME",
                Some(home.path().join(".local/share").into()),
            ),
            ("CLAUDE_CONFIG_DIR", None),
            ("MICOLD_IMAGE_REFERENCE", None),
        ]);
        let workspace = Workspace {
            projects: vec![Project::new(
                repo.path().to_path_buf(),
                true,
                Availability::Available,
            )],
            active: Some(repo.path().to_path_buf()),
            ..Default::default()
        };
        JsonFileStore::at(store.path().join("projects.json"))
            .save(&workspace)
            .unwrap();
        let state = service(store.path());
        Self {
            _env: env,
            bin,
            home,
            repo,
            store,
            state,
        }
    }

    pub fn project(&self) -> PathBuf {
        self.repo.path().to_path_buf()
    }

    pub fn files(&self) -> JsonFileStore {
        JsonFileStore::at(self.store.path().join("projects.json"))
    }

    /// What the runs file holds now.
    pub fn on_disk(&self) -> RunsFile {
        self.files().load_runs(&self.project())
    }

    /// The runs file's path.
    pub fn runs_path(&self) -> PathBuf {
        self.files().runs_path(&self.project())
    }

    /// The worktree folder a run gets.
    pub fn worktree(&self, dir_name: &str) -> PathBuf {
        self.project().join(".claude/worktrees").join(dir_name)
    }

    /// What was typed into `id` so far.
    pub fn typed(&self, id: SessionId) -> String {
        std::fs::read_to_string(self.bin.path().join(format!("input.{}", id.0))).unwrap_or_default()
    }

    /// What was typed into every session that read input so far.
    pub fn inputs(&self) -> Vec<(SessionId, String)> {
        std::fs::read_dir(self.bin.path())
            .unwrap()
            .filter_map(|entry| {
                let name = entry.ok()?.file_name().into_string().ok()?;
                let id = uuid::Uuid::parse_str(name.strip_prefix("input.")?).ok()?;
                let id = SessionId::from_uuid(id);
                Some((id, self.typed(id)))
            })
            .collect()
    }

    /// The stand-in waits `secs` before it draws anything.
    pub fn slow(&self, secs: f32) {
        std::fs::write(self.bin.path().join("slow"), secs.to_string()).unwrap();
    }

    /// Wait until what was typed into `id` holds `needle`.
    pub async fn typed_holds(&self, id: SessionId, needle: &str) -> String {
        let deadline = Instant::now() + BOUND;
        loop {
            let typed = self.typed(id);
            if typed.contains(needle) {
                return typed;
            }
            assert!(
                Instant::now() < deadline,
                "{} never received {needle:?}; it has {typed:?}",
                id.0
            );
            tokio::time::sleep(Duration::from_millis(25)).await;
        }
    }
}

impl Drop for Sandbox {
    fn drop(&mut self) {
        for (id, _) in self.inputs() {
            if let Some(pty) = self.state.live_session(id) {
                let _ = pty.kill();
            }
        }
    }
}

/// A service over `store`. Called twice, the second is the first restarted.
pub fn service(store: &Path) -> Arc<DaemonState> {
    Arc::new(DaemonState::new(Catalog::load(
        Box::new(JsonFileStore::at(store.join("projects.json"))),
        Box::new(JsonFileSettingsStore::at(store.join("settings.json"))),
    )))
}

/// The stand-in `claude`: waits `<bin>/slow` seconds when that file exists, asks for bracketed
/// paste, draws a prompt, and appends everything typed into it to `<bin>/input.<--session-id>`.
fn install_claude(bin: &Path) {
    let dir = bin.display();
    let path = bin.join("claude");
    std::fs::write(
        &path,
        format!(
            "#!/bin/sh\n\
             sid=''; prev=''\n\
             for a in \"$@\"; do [ \"$prev\" = --session-id ] && sid=\"$a\"; prev=\"$a\"; done\n\
             [ -e '{dir}'/slow ] && sleep \"$(cat '{dir}'/slow)\"\n\
             printf '\\033[?2004h'\n\
             printf 'ready> '\n\
             exec cat >> '{dir}'/input.\"$sid\"\n"
        ),
    )
    .unwrap();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
}

/// The bytes a submission of `prompt` types into a terminal with bracketed paste on.
pub fn submitted(prompt: &str) -> String {
    format!("\u{1b}[200~{prompt}\u{1b}[201~\n")
}

/// Connect and complete the handshake.
pub async fn connect(state: &Arc<DaemonState>) -> Client {
    let (server_io, client_io) = tokio::io::duplex(1024 * 1024);
    tokio::spawn(micold_daemon::server::serve_connection(
        Arc::clone(state),
        server_io,
    ));
    let mut client = Framed::new(client_io, ClientCodec::new());
    client
        .send(Frame::Control(ClientMsg::Hello {
            protocol_version: PROTOCOL_VERSION,
            schema_hash: SCHEMA_HASH,
            client_build: "test".into(),
            client_instance: micold_core::protocol::messages::ClientInstance::current(),
            client_package_version: PACKAGE_VERSION.into(),
            auth_token: None,
            client_fingerprint: BUILD_FINGERPRINT.into(),
            require_fingerprint_match: false,
        }))
        .await
        .unwrap();
    client
}

/// Attach to `project`; the frames up to and including the attach's `CatalogChanged`.
pub async fn attach(client: &mut Client, project: &Path) -> Vec<DaemonMsg> {
    client
        .send(Frame::Control(ClientMsg::Attach {
            project: project.to_path_buf(),
            force: true,
        }))
        .await
        .unwrap();
    let attached = async {
        let mut seen = Vec::new();
        let mut was_attached = false;
        loop {
            if let Frame::Control(msg) = client.next().await.expect("stream open").unwrap() {
                was_attached |= matches!(msg, DaemonMsg::Attached { .. });
                let done = was_attached && matches!(msg, DaemonMsg::CatalogChanged { .. });
                seen.push(msg);
                if done {
                    return seen;
                }
            }
        }
    };
    tokio::time::timeout(BOUND, attached)
        .await
        .expect("attach completes with its catalog")
}

/// Connect and attach to `project`.
pub async fn window(state: &Arc<DaemonState>, project: &Path) -> Client {
    let mut client = connect(state).await;
    attach(&mut client, project).await;
    client
}

/// An answer: the result, or the error's kind and message.
pub type Answer = Result<OperationResult, (ErrorKind, String)>;

/// Send `msg` (carrying `req`) and wait for its answer, collecting the `RunGroupsChanged` pushes
/// seen meanwhile.
pub async fn request(client: &mut Client, req: u64, msg: ClientMsg) -> (Answer, Vec<Vec<RunGroup>>) {
    client.send(Frame::Control(msg)).await.unwrap();
    let mut seen = Vec::new();
    let answered = async {
        loop {
            let Frame::Control(msg) = client.next().await.expect("stream open").unwrap() else {
                continue;
            };
            match msg {
                DaemonMsg::RunGroupsChanged { groups, .. } => seen.push(groups),
                DaemonMsg::OperationOk { req: r, result } if r == req => return Ok(result),
                DaemonMsg::OperationError {
                    req: r,
                    kind,
                    message,
                    ..
                } if r == req => return Err((kind, message)),
                _ => {}
            }
        }
    };
    let result = tokio::time::timeout(BOUND, answered)
        .await
        .expect("the service answers");
    (result, seen)
}

/// The next `RunGroupsChanged` push within `wait`, if one comes.
pub async fn next_runs_push(client: &mut Client, wait: Duration) -> Option<Vec<RunGroup>> {
    let pushed = async {
        loop {
            if let Frame::Control(DaemonMsg::RunGroupsChanged { groups, .. }) =
                client.next().await.expect("stream open").unwrap()
            {
                return groups;
            }
        }
    };
    tokio::time::timeout(wait, pushed).await.ok()
}

/// Read pushes until group `id` has no run still `Creating` or `Starting`; that last push.
pub async fn settled(client: &mut Client, id: GroupId) -> RunGroup {
    let deadline = Instant::now() + BOUND * 2;
    loop {
        let left = deadline.saturating_duration_since(Instant::now());
        let groups = next_runs_push(client, left)
            .await
            .expect("the runs settle with a push");
        if let Some(group) = groups.into_iter().find(|g| g.id == id) {
            if group
                .runs
                .iter()
                .all(|run| !matches!(run.status, RunStatus::Creating | RunStatus::Starting))
            {
                return group;
            }
        }
    }
}

/// `feat` + `login page`.
pub fn login_page() -> WorktreeNaming {
    WorktreeNaming {
        type_: Some(ConventionalType::Feat),
        ticket: None,
        name: "login page".into(),
    }
}

/// A create of `login page` from `base` with `providers`.
pub fn create_msg(req: u64, project: &Path, providers: Vec<AiCli>) -> ClientMsg {
    ClientMsg::RunGroupCreate {
        req,
        project: project.to_path_buf(),
        naming: login_page(),
        prompt: PROMPT.into(),
        base_branch: "base".into(),
        providers,
    }
}

/// The group a create answered with.
pub fn created(answer: &Answer) -> GroupId {
    match answer {
        Ok(OperationResult::RunGroupCreated { group }) => *group,
        other => panic!("the group was created: {other:?}"),
    }
}
