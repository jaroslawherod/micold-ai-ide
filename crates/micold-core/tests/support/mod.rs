//! T001 — shared test scaffolding for multi-project state (feature 008).
//!
//! Builds a `Workspace` with two-or-more projects, each holding sessions in caller-chosen
//! lifecycles, with no filesystem access. Included via `mod support;` from the feature-008
//! integration tests. Compiles under `cargo test --no-default-features` (pure core only).

#![allow(dead_code)]

use micold_core::fs_scan::FakeFolderScanner;
use micold_core::project::canonicalize_best_effort;
use micold_core::session::{
    AiCli, RestartDecision, Session, SessionId, SessionLabel, SessionLocation, TerminalMode,
};
use micold_core::workspace::Workspace;
use std::path::{Path, PathBuf};

/// The shared in-memory scanner, primed the way this scaffolding's callers expect: every folder
/// is a git repository and every folder is present. Replaces a hand-written `FakeScanner`
/// (feature 021 T048).
pub fn fake_scanner() -> FakeFolderScanner {
    FakeFolderScanner::new().git_repos(true)
}

/// A session that is currently `Running`.
pub fn running_session(worktree_dir: &str) -> Session {
    let mut s = Session::start_new(
        SessionLocation::Worktree(worktree_dir.to_string()),
        AiCli::ClaudeCode,
    );
    s.mark_running();
    s
}

/// A `SessionLocation::Default` session that is currently `Running` (feature 010).
pub fn running_default_session() -> Session {
    let mut s = Session::start_new(SessionLocation::Default, AiCli::ClaudeCode);
    s.mark_running();
    s
}

/// A persisted session restored as `Idle`.
pub fn idle_session(worktree_dir: &str) -> Session {
    Session::restored(
        SessionId::new(),
        SessionLocation::Worktree(worktree_dir.to_string()),
        SessionLabel::Pending,
        TerminalMode::AiCli,
        AiCli::ClaudeCode,
    )
}

/// A session driven to `Failed` via repeated unexpected exits (crash-loop guard).
pub fn failed_session(worktree_dir: &str) -> Session {
    let mut s = Session::start_new(
        SessionLocation::Worktree(worktree_dir.to_string()),
        AiCli::ClaudeCode,
    );
    s.mark_running();
    loop {
        if s.on_unexpected_exit("exit status 1") == RestartDecision::GiveUp {
            break;
        }
    }
    s
}

/// Build a `Workspace` with the given `(path, sessions)` projects — all `Available`, **no**
/// active project (caller sets `active` + `active_session` explicitly). Sessions are keyed by
/// the same canonicalized path the app uses.
pub fn workspace_with(projects: Vec<(&str, Vec<Session>)>) -> Workspace {
    let mut ws = Workspace::empty();
    let scanner = fake_scanner();
    for (path, sessions) in projects {
        ws.open_or_activate(PathBuf::from(path), &scanner);
        let key = canonicalize_best_effort(Path::new(path));
        ws.sessions.insert(key, sessions);
    }
    ws.active = None;
    ws
}

// ---------------------------------------------------------------------------------------
// T002 (feature 026) — the Copilot fixture store.
//
// Every test that reads Copilot's storage reads it through here. Two reasons, and the second is
// the one that makes it mandatory rather than convenient:
//
// 1. The corpus in `tests/fixtures/copilot/` is stored under *logical* names, because Copilot's
//    real layout is a function of the working directory (`sidebar-sessions-state/
//    <sha256_hex(cwd)>.json`) and a test's cwd is a fresh temporary directory. Something has to
//    assemble one from the other.
// 2. `COPILOT_HOME` is process-global. A test that set it and forgot to restore it would leak
//    into every other test in the same binary — and a test that never set it would read the
//    developer's real `~/.copilot`, which is a defect even on the runs where it passes.
// ---------------------------------------------------------------------------------------

use micold_core::protocol::hashing::sha256_hex;
use std::sync::{Mutex, MutexGuard, OnceLock};
use tempfile::TempDir;
use uuid::Uuid;

/// The three fixture session ids, in the order `index-well-formed.json` lists them.
pub const FIXTURE_SESSION_A: &str = "aaaaaaaa-1111-4111-8111-111111111111";
pub const FIXTURE_SESSION_B: &str = "bbbbbbbb-2222-4222-8222-222222222222";
pub const FIXTURE_SESSION_C: &str = "cccccccc-3333-4333-8333-333333333333";
/// A fourth session that exists on disk but is listed in **no** index — the index is the source
/// of truth for a working directory, not the session-state directory listing.
pub const FIXTURE_SESSION_D: &str = "dddddddd-4444-4444-8444-444444444444";

/// The working directory the fixture files were written for. `CopilotHome::with_corpus` rewrites
/// it to whatever directory the test actually uses.
pub const FIXTURE_CWD: &str = "/fixture/worktree";

/// The directory holding the T001 corpus, resolved from the calling crate's manifest rather than
/// assumed relative to it — this module is `#[path]`-included from other crates' tests too.
pub fn copilot_fixture_dir() -> PathBuf {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let workspace = manifest
        .ancestors()
        .find(|dir| dir.join("crates").join("micold-core").is_dir())
        .unwrap_or(&manifest)
        .to_path_buf();
    workspace
        .join("crates")
        .join("micold-core")
        .join("tests")
        .join("fixtures")
        .join("copilot")
}

/// Read one fixture file by its logical name.
pub fn copilot_fixture(name: &str) -> String {
    let path = copilot_fixture_dir().join(name);
    std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("read fixture {}: {e}", path.display()))
}

fn env_lock() -> MutexGuard<'static, ()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    // A poisoned lock only means some other test panicked while holding it; the environment is
    // restored by the guard's `Drop` either way, so there is nothing to recover.
    LOCK.get_or_init(|| Mutex::new(()))
        .lock()
        .unwrap_or_else(|e| e.into_inner())
}

/// A scratch Copilot store with `COPILOT_HOME` pointed at it for as long as the value lives.
///
/// Serialised against every other `CopilotHome` in the same test binary, because the environment
/// is process-wide and Rust runs tests on threads.
pub struct CopilotHome {
    home: PathBuf,
    previous: Option<String>,
    _guard: MutexGuard<'static, ()>,
    _base: TempDir,
}

/// An empty Copilot store, with `COPILOT_HOME` set to it and restored on drop.
pub fn copilot_home() -> CopilotHome {
    let guard = env_lock();
    let base = tempfile::tempdir().expect("scratch COPILOT_HOME");
    let home = base.path().join("copilot");
    std::fs::create_dir_all(home.join("session-state")).expect("session-state");
    std::fs::create_dir_all(home.join("sidebar-sessions-state")).expect("sidebar-sessions-state");
    let previous = std::env::var("COPILOT_HOME").ok();
    std::env::set_var("COPILOT_HOME", &home);
    CopilotHome {
        home,
        previous,
        _guard: guard,
        _base: base,
    }
}

impl CopilotHome {
    /// The base directory — what `CopilotProvider::config_dir()` resolves to while this lives.
    pub fn path(&self) -> &Path {
        &self.home
    }

    /// Where the per-working-directory index for `cwd` belongs.
    pub fn index_path(&self, cwd: &Path) -> PathBuf {
        self.home.join("sidebar-sessions-state").join(format!(
            "{}.json",
            sha256_hex(cwd.to_string_lossy().as_bytes())
        ))
    }

    /// Where session `id`'s own directory belongs.
    pub fn session_dir(&self, id: Uuid) -> PathBuf {
        self.home.join("session-state").join(id.to_string())
    }

    /// Write one index fixture as `cwd`'s index, with the fixture's own `cwd` string rewritten to
    /// the real one — so the file on disk says what Copilot's would.
    pub fn with_index(self, cwd: &Path, fixture: &str) -> Self {
        let body = copilot_fixture(fixture).replace(FIXTURE_CWD, &cwd.to_string_lossy());
        std::fs::write(self.index_path(cwd), body).expect("write index");
        self
    }

    /// Write an index listing exactly these ids — for the volume cases (~250 recorded
    /// conversations) where naming each one in a fixture file would be noise.
    pub fn with_index_of(self, cwd: &Path, ids: &[Uuid]) -> Self {
        let listed = ids
            .iter()
            .map(|id| format!("    \"{id}\""))
            .collect::<Vec<_>>()
            .join(",\n");
        let body = format!(
            "{{\n  \"schemaVersion\": 1,\n  \"cwd\": {:?},\n  \"sessionIds\": [\n{listed}\n  ]\n}}\n",
            cwd.to_string_lossy()
        );
        std::fs::write(self.index_path(cwd), body).expect("write index");
        self
    }

    /// Materialise one session directory: its `workspace.yaml` and, when the session recorded a
    /// conversation, its `events.jsonl`. `None` for either leaves that file absent, which is the
    /// state the contract gives meaning to (no title yet / opened and never used).
    pub fn with_session(
        self,
        id: Uuid,
        workspace_fixture: Option<&str>,
        events_fixture: Option<&str>,
    ) -> Self {
        let dir = self.session_dir(id);
        std::fs::create_dir_all(&dir).expect("session dir");
        if let Some(name) = workspace_fixture {
            std::fs::write(dir.join("workspace.yaml"), copilot_fixture(name)).expect("workspace");
        }
        if let Some(name) = events_fixture {
            std::fs::write(dir.join("events.jsonl"), copilot_fixture(name)).expect("events");
        }
        self
    }

    /// Write the app-owned archived sentinel for a session (`micold.archived`).
    pub fn archived(self, id: Uuid) -> Self {
        let dir = self.session_dir(id);
        std::fs::create_dir_all(&dir).expect("session dir");
        std::fs::write(dir.join("micold.archived"), "").expect("marker");
        self
    }

    /// The whole T001 corpus for one working directory: a well-formed index naming A, C and B, and
    /// all four session directories.
    ///
    /// - **A** has a plain title and a full turn recorded.
    /// - **B** has a single-quoted title containing a colon, and a turn that never ended.
    /// - **C** has no title and **no** `events.jsonl` — opened and never used.
    /// - **D** is on disk with a double-quoted title and an awkward log, but is in **no** index.
    pub fn with_corpus(self, cwd: &Path) -> Self {
        self.with_index(cwd, "index-well-formed.json")
            .with_session(
                fixture_id(FIXTURE_SESSION_A),
                Some("workspace-named-plain.yaml"),
                Some("events-full-turn.jsonl"),
            )
            .with_session(
                fixture_id(FIXTURE_SESSION_B),
                Some("workspace-named-quoted-colon.yaml"),
                Some("events-dangling-turn.jsonl"),
            )
            .with_session(
                fixture_id(FIXTURE_SESSION_C),
                Some("workspace-unnamed.yaml"),
                None,
            )
            .with_session(
                fixture_id(FIXTURE_SESSION_D),
                Some("workspace-named-double-quoted.yaml"),
                Some("events-unknown-and-malformed.jsonl"),
            )
    }
}

/// Parse one of the `FIXTURE_SESSION_*` constants.
pub fn fixture_id(literal: &str) -> Uuid {
    literal.parse().expect("fixture uuid")
}

impl Drop for CopilotHome {
    fn drop(&mut self) {
        match self.previous.take() {
            Some(value) => std::env::set_var("COPILOT_HOME", value),
            None => std::env::remove_var("COPILOT_HOME"),
        }
    }
}
