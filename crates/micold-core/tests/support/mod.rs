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

// ---------------------------------------------------------------------------------------
// T002 (feature 029) — the Pi fixture store.
//
// The same bargain `CopilotHome` strikes, for the same two reasons: the corpus in
// `tests/fixtures/pi/` is stored under logical names because Pi's real layout is a function of the
// working directory (`sessions/--<encoded cwd>--/…`), and `PI_CODING_AGENT_DIR` is process-global,
// so a test that set it and forgot to restore it would leak into every other test in the binary —
// and one that never set it would read the developer's real `~/.pi/agent`.
//
// The cwd encoder below is written out here rather than called from `PiProvider`, deliberately: a
// test that derives the path the same way the code under test does can only ever agree with it.
// ---------------------------------------------------------------------------------------

/// The session id every fixture conversation carries in its header line. Substituted for the id
/// the conversation is actually materialised under, so the header agrees with the filename.
pub const PI_FIXTURE_ID: &str = "ffffffff-ffff-4fff-8fff-ffffffffffff";

/// How much filler the padding entry expands to. Comfortably more than any bounded prefix read a
/// label is worth paying for, so "past the bound" stays true without the fixture file carrying a
/// quarter of a megabyte of noise into git.
pub const PI_PADDING_BYTES: usize = 256 * 1024;

/// The timestamp component of a fixture conversation's filename, when the test does not care which
/// one it gets. Pi's own form: the creation time with `:` and `.` replaced by `-`.
pub const PI_FIXTURE_TIMESTAMP: &str = "2026-09-12T08-15-04-123Z";

/// The directory holding the feature-029 corpus, resolved from the calling crate's manifest — this
/// module is `#[path]`-included from other crates' tests too.
pub fn pi_fixture_dir() -> PathBuf {
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
        .join("pi")
}

/// Read one Pi fixture by its logical name, raw — no substitution.
pub fn pi_fixture(name: &str) -> String {
    let path = pi_fixture_dir().join(name);
    std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("read fixture {}: {e}", path.display()))
}

/// Pi's per-working-directory encoding: the absolute path with its leading separator stripped and
/// every `/`, `\` and `:` replaced by `-`, wrapped in `--`…`--`.
pub fn pi_encoded_cwd(cwd: &Path) -> String {
    let raw = cwd.to_string_lossy();
    let trimmed = raw
        .strip_prefix('/')
        .or_else(|| raw.strip_prefix('\\'))
        .unwrap_or(&raw);
    let body: String = trimmed
        .chars()
        .map(|c| {
            if matches!(c, '/' | '\\' | ':') {
                '-'
            } else {
                c
            }
        })
        .collect();
    format!("--{body}--")
}

/// `<base>/sessions/--<encoded cwd>--` — where every conversation for `cwd` lives.
pub fn pi_sessions_dir(base: &Path, cwd: &Path) -> PathBuf {
    base.join("sessions").join(pi_encoded_cwd(cwd))
}

/// One conversation's file: `<timestamp>_<session-id>.jsonl`. The id is the part after the first
/// underscore, which is what makes the timestamp's own `-`-separated form unambiguous.
pub fn pi_conversation_path(base: &Path, cwd: &Path, timestamp: &str, id: Uuid) -> PathBuf {
    pi_sessions_dir(base, cwd).join(format!("{timestamp}_{id}.jsonl"))
}

/// Substitute a fixture's placeholders and expand its padding entry, if it has one.
fn pi_materialise(fixture: &str, cwd: &Path, id: Uuid) -> String {
    let body = pi_fixture(fixture)
        .replace(FIXTURE_CWD, &cwd.to_string_lossy())
        .replace(PI_FIXTURE_ID, &id.to_string());
    if !body.contains("micold_fixture_padding") {
        return body;
    }
    let filler = pi_padding_lines();
    body.lines()
        .map(|line| {
            if line.starts_with("{\"type\":\"micold_fixture_padding\"") {
                filler.as_str()
            } else {
                line
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
        + "\n"
}

/// `PI_PADDING_BYTES` worth of entries of a type nothing reads.
fn pi_padding_lines() -> String {
    let one = format!(
        "{{\"type\":\"micold_fixture_padding\",\"parentId\":null,\"timestamp\":\"2026-09-12T12:01:00.000Z\",\"filler\":\"{}\"}}",
        "x".repeat(160)
    );
    let count = PI_PADDING_BYTES / (one.len() + 1) + 1;
    vec![one; count].join("\n")
}

/// A scratch Pi store with `PI_CODING_AGENT_DIR` pointed at it for as long as the value lives.
///
/// Serialised against every other scratch home in the same test binary — `CopilotHome` included,
/// because they share one lock and one process environment.
pub struct PiHome {
    home: PathBuf,
    previous: Option<String>,
    _guard: MutexGuard<'static, ()>,
    _base: TempDir,
}

/// An empty Pi store, with `PI_CODING_AGENT_DIR` set to it and restored on drop.
pub fn pi_home() -> PiHome {
    let guard = env_lock();
    let base = tempfile::tempdir().expect("scratch PI_CODING_AGENT_DIR");
    let home = base.path().join("pi").join("agent");
    std::fs::create_dir_all(home.join("sessions")).expect("sessions");
    let previous = std::env::var("PI_CODING_AGENT_DIR").ok();
    std::env::set_var("PI_CODING_AGENT_DIR", &home);
    PiHome {
        home,
        previous,
        _guard: guard,
        _base: base,
    }
}

impl PiHome {
    /// The base directory — what `PiProvider::config_dir()` resolves to while this lives.
    pub fn path(&self) -> &Path {
        &self.home
    }

    /// Where `cwd`'s conversations belong.
    pub fn sessions_dir(&self, cwd: &Path) -> PathBuf {
        pi_sessions_dir(&self.home, cwd)
    }

    /// The conversation recorded for `(cwd, id)`, whatever timestamp it was written under — the
    /// id is the part after the first underscore, so this is the lookup a reader performs.
    pub fn conversation_path(&self, cwd: &Path, id: Uuid) -> Option<PathBuf> {
        let wanted = format!("{id}.jsonl");
        std::fs::read_dir(self.sessions_dir(cwd))
            .ok()?
            .filter_map(|entry| entry.ok())
            .map(|entry| entry.path())
            .find(|path| {
                path.file_name()
                    .and_then(|name| name.to_str())
                    .and_then(|name| name.split_once('_'))
                    .is_some_and(|(_, rest)| rest == wanted)
            })
    }

    /// Materialise one fixture conversation for `(cwd, id)` under the default timestamp.
    pub fn with_conversation(self, cwd: &Path, id: Uuid, fixture: &str) -> Self {
        self.with_conversation_at(cwd, PI_FIXTURE_TIMESTAMP, id, fixture)
    }

    /// Materialise one fixture conversation under a chosen timestamp — for the cases where two
    /// conversations must be distinguishable by filename alone.
    pub fn with_conversation_at(
        self,
        cwd: &Path,
        timestamp: &str,
        id: Uuid,
        fixture: &str,
    ) -> Self {
        let path = pi_conversation_path(&self.home, cwd, timestamp, id);
        std::fs::create_dir_all(path.parent().expect("sessions dir")).expect("sessions dir");
        std::fs::write(&path, pi_materialise(fixture, cwd, id)).expect("write conversation");
        self
    }

    /// Materialise the same fixture under many ids — for the volume cases (FR-015, SC-006b) where
    /// naming each conversation would be noise. Each gets its own timestamp.
    pub fn with_conversations(mut self, cwd: &Path, ids: &[Uuid], fixture: &str) -> Self {
        for (nth, id) in ids.iter().enumerate() {
            let timestamp = format!("2026-09-12T08-15-04-{nth:06}Z");
            self = self.with_conversation_at(cwd, &timestamp, *id, fixture);
        }
        self
    }

    /// Append raw JSONL lines to an existing conversation — what a live `pi` does to a file the
    /// application is already reading.
    pub fn append(self, cwd: &Path, id: Uuid, lines: &[&str]) -> Self {
        use std::io::Write;
        let path = self
            .conversation_path(cwd, id)
            .unwrap_or_else(|| panic!("no conversation for {id}"));
        let mut file = std::fs::OpenOptions::new()
            .append(true)
            .open(&path)
            .unwrap_or_else(|e| panic!("append {}: {e}", path.display()));
        for line in lines {
            writeln!(file, "{line}").expect("append line");
        }
        self
    }

    /// Write the app-owned archived sentinel for a session: `<session-id>.archived`, beside the
    /// conversations and filtered out of discovery by the `.jsonl` rule.
    pub fn archived(self, cwd: &Path, id: Uuid) -> Self {
        let dir = self.sessions_dir(cwd);
        std::fs::create_dir_all(&dir).expect("sessions dir");
        std::fs::write(dir.join(format!("{id}.archived")), "").expect("marker");
        self
    }

    /// The whole corpus for one working directory:
    ///
    /// - **A** is named twice, so the later name is the one that must win.
    /// - **B** has no name at all — the label falls back to its first user message.
    /// - **C** is named only past the label read's bound, so it falls back the same way.
    /// - **D** is a header and nothing else — the label stays `Pending`.
    pub fn with_corpus(self, cwd: &Path) -> Self {
        self.with_conversation_at(
            cwd,
            "2026-09-12T08-15-04-123Z",
            fixture_id(FIXTURE_SESSION_A),
            "conversation-named.jsonl",
        )
        .with_conversation_at(
            cwd,
            "2026-09-12T09-02-11-008Z",
            fixture_id(FIXTURE_SESSION_B),
            "conversation-unnamed.jsonl",
        )
        .with_conversation_at(
            cwd,
            "2026-09-12T12-00-00-000Z",
            fixture_id(FIXTURE_SESSION_C),
            "conversation-named-past-bound.jsonl",
        )
        .with_conversation_at(
            cwd,
            "2026-09-12T09-41-00-500Z",
            fixture_id(FIXTURE_SESSION_D),
            "conversation-header-only.jsonl",
        )
    }
}

impl Drop for PiHome {
    fn drop(&mut self) {
        match self.previous.take() {
            Some(value) => std::env::set_var("PI_CODING_AGENT_DIR", value),
            None => std::env::remove_var("PI_CODING_AGENT_DIR"),
        }
    }
}
