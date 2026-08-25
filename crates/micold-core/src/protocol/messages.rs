//! The full client ↔ daemon message surface (contracts/messages.md).
//!
//! Two categories, distinguished by the envelope `kind` byte and by these two enums:
//!
//! - [`ClientMsg`] — client → daemon. Commands are fire-and-forget; requests are correlated by
//!   `req: u64` and resolve to exactly one outcome (FR-020, FR-031).
//! - [`DaemonMsg`] — daemon → client. State projection is pushed unsolicited; operation results are
//!   correlated back by `req`.
//!
//! Grid frames ([`crate::protocol::grid::GridFrame`]) travel on the same stream under `kind = 1` and
//! are intentionally **not** a `DaemonMsg` variant — they are lossy/convergent where control messages
//! are ordered/lossless (messages.md §Ordering guarantees).
//!
//! Both binaries compile against this one definition; the `SCHEMA_HASH` guard (protocol.md §4) makes
//! any wire-visible edit here refuse a mismatched peer.

use std::fmt;
use std::ops::Range;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::protocol::grid::{LineId, WireLine, WireStyle};
use crate::session::{SessionId, SessionLabel, ShellInstanceId};
use crate::worktree::{BranchCandidate, BranchSituation, CreateMode, CreateStage};

// ---------------------------------------------------------------------------------------------
// Client → Daemon
// ---------------------------------------------------------------------------------------------

/// Which of a session's processes an operation targets (feature 011). A session always has a
/// `Primary` process (its AI CLI or its mode-selected primary shell); `Shell(id)` names one of the
/// additional Regular-terminal instances.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SessionProcess {
    /// The session's primary process.
    Primary,
    /// An additional shell instance, by id.
    Shell(ShellInstanceId),
}

/// The handshake secret as it travels on the wire, in a wrapper that will not print it.
///
/// This field used to be a bare `String`, and [`ClientMsg`] derives `Debug` — so one
/// `tracing::debug!(?msg)` anywhere on the receive path would have written the token into the
/// daemon's log verbatim, and a `{err:?}` on a decode failure could have carried it into a bug
/// report. [`crate::protocol::auth::Token`] has had an opaque `Debug` since it was introduced, for
/// exactly that reason; the protection was being dropped at the moment the value crossed onto the
/// wire, which is the moment it reaches the most code (feature 027, T118 / rule P-3).
///
/// `#[serde(transparent)]`: the encoding is byte-for-byte what a `String` produced, in JSON and in
/// postcard alike. This is a `Debug` fix, not a wire change.
///
/// Declared here rather than beside `Token` deliberately. `SCHEMA_HASH` is computed over the text
/// of this file; a wire-visible type defined elsewhere could change its serde representation
/// without moving the hash, and two builds that disagree would then shake hands.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct PresentedToken(String);

impl PresentedToken {
    /// Wrap a token for presentation.
    pub fn new(token: impl Into<String>) -> Self {
        Self(token.into())
    }

    /// The secret itself. Every caller of this is a place to check when auditing P-3.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for PresentedToken {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Not even a prefix: a partial leak is still a leak when it shrinks the search space.
        f.write_str("PresentedToken(<redacted>)")
    }
}

/// A message from a client to the daemon.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ClientMsg {
    // --- Connection ---
    /// Handshake. Both `protocol_version` **and** `schema_hash` must match (Decision 4).
    Hello {
        /// The client's compiled [`crate::protocol::version::PROTOCOL_VERSION`].
        protocol_version: u32,
        /// The client's compiled [`crate::protocol::version::SCHEMA_HASH`].
        schema_hash: [u8; 32],
        /// Human-facing build string for diagnostics.
        client_build: String,
        /// The client's compiled [`crate::protocol::version::PACKAGE_VERSION`] — changes on every
        /// release, wire-visible or not, so a same-contract `.deb` upgrade over an already-running
        /// daemon is still detected (FR-022a, BUG-002).
        client_package_version: String,
        /// The shared secret, when the daemon is expected to require one (feature 027, R1).
        ///
        /// `None` for the host-process placement, whose `0700`-guarded socket authenticates by
        /// filesystem permission already. `Some` for the sandbox, whose loopback TCP transport
        /// authenticates nobody — which is why this field and the version bump arrive together.
        auth_token: Option<PresentedToken>,
        /// The client's compiled [`crate::protocol::version::BUILD_FINGERPRINT`] (feature 027, R8).
        client_fingerprint: String,
        /// Whether a fingerprint mismatch is a refusal.
        ///
        /// Set by the **client**, because the client is what knows where the daemon's image came
        /// from: a locally built one shares this client's working tree and has no business
        /// disagreeing, while a released one was built separately and legitimately differs.
        require_fingerprint_match: bool,
    },
    /// Attach to a project. `force = true` is a confirmed takeover, only sent after explicit user
    /// confirmation (FR-023).
    Attach {
        /// Project identity path.
        project: PathBuf,
        /// Whether to displace the current holder.
        force: bool,
    },
    /// Release a project attachment.
    Detach {
        /// Project identity path.
        project: PathBuf,
    },
    /// Clean disconnect. Does **not** stop sessions.
    Goodbye,

    // --- Session commands (fire-and-forget) ---
    /// Append input bytes to a session's PTY. `serial` is monotonic per session and exists to
    /// detect loss, never to enable coalescing — input is an append-only log (G2).
    SessionInput {
        /// Target session.
        session: SessionId,
        /// Monotonic per-session serial.
        serial: u64,
        /// The raw VT bytes (the client already translated keys, FR-019).
        bytes: Vec<u8>,
    },
    /// Resize a session's PTY / grid.
    SessionResize {
        /// Target session.
        session: SessionId,
        /// New column count.
        cols: u16,
        /// New row count.
        rows: u16,
    },
    /// Start (or resume) a session: `Idle | Failed | InterruptedResumable` → `Starting`.
    SessionStart {
        /// Target session.
        session: SessionId,
    },
    /// Graceful stop → `Idle`, no restart.
    SessionStop {
        /// Target session.
        session: SessionId,
    },
    /// Force-kill via the escalation ladder.
    SessionKill {
        /// Target session.
        session: SessionId,
    },
    /// Write `0x03` to the PTY master (never a real signal — protocol.md §7).
    SessionInterrupt {
        /// Target session.
        session: SessionId,
    },

    // --- Shell instances (feature 011): a session hosts one primary process (AI CLI or shell) plus
    // any number of additional Regular-terminal shell instances; exactly one is *attached* (viewed +
    // driven) at a time. Input/grid frames stay `SessionId`-addressed and route to the attached one.
    /// Choose which of a session's processes is attached (streamed + receives input).
    SessionAttachProcess {
        /// Target session.
        session: SessionId,
        /// Which process to attach.
        process: SessionProcess,
    },
    /// Open (spawn) an additional shell instance for a session (the client owns the id).
    SessionOpenShell {
        /// Target session.
        session: SessionId,
        /// The new instance's id (client-allocated; unique within the session).
        instance: ShellInstanceId,
    },
    /// Close (kill) a session's shell instance.
    SessionCloseShell {
        /// Target session.
        session: SessionId,
        /// The instance to close.
        instance: ShellInstanceId,
    },
    /// Restart (kill + respawn) a session's shell instance.
    SessionRestartShell {
        /// Target session.
        session: SessionId,
        /// The instance to restart.
        instance: ShellInstanceId,
    },

    // --- View commands ---
    /// Choose which session receives full grid streaming; `None` means none is viewed (FR-016).
    SetViewedSession {
        /// Project identity path.
        project: PathBuf,
        /// The viewed session, or `None`.
        session: Option<SessionId>,
    },
    /// Request scrollback by `LineId` range. Advisory, never an error (protocol.md §6).
    ScrollbackRequest {
        /// Target session.
        session: SessionId,
        /// Correlation id.
        req: u64,
        /// The requested line-id ranges.
        ranges: Vec<Range<LineId>>,
    },

    // --- Mutating requests (correlated) ---
    /// Add a project to the catalog.
    ProjectAdd {
        /// Correlation id.
        req: u64,
        /// Project path.
        path: PathBuf,
    },
    /// Remove a project from the catalog.
    ProjectRemove {
        /// Correlation id.
        req: u64,
        /// Project path.
        path: PathBuf,
    },
    /// Rename a project's display name.
    ProjectRename {
        /// Correlation id.
        req: u64,
        /// Project path.
        path: PathBuf,
        /// New display name.
        display_name: String,
    },
    /// Create a worktree under a project.
    WorktreeCreate {
        /// Correlation id.
        req: u64,
        /// Project path.
        project: PathBuf,
        /// Branch to create/check out.
        branch: String,
        /// Worktree directory name.
        dir_name: String,
        /// How to obtain the branch: a fresh one, an existing local one, a forced replacement, or
        /// one tracking a remote (feature 016). The daemon re-verifies this against a fresh
        /// pre-flight before acting (FR-009).
        mode: CreateMode,
    },
    /// Classify what stands between the user and a new worktree on `branch`, so the client can
    /// offer reuse/overwrite instead of failing (feature 016, FR-001). Read-only — the daemon
    /// mutates nothing while answering this.
    BranchPreflight {
        /// Correlation id.
        req: u64,
        /// Project path.
        project: PathBuf,
        /// Branch name the client derived or the user selected.
        branch: String,
        /// Worktree directory name that would be created, for the directory-clash check.
        dir_name: String,
    },
    /// Ask whether `path` is the ROOT of a git repository — the open-project gate (FR-001a) —
    /// answered by the daemon's git rather than the client's (feature 027, research R2 part 2).
    ///
    /// Read-only: the daemon runs `git rev-parse --show-toplevel` and mutates nothing.
    ///
    /// The client asks this only when it has no git view of the daemon's filesystem: a Windows
    /// host cannot mount `C:\Users\u\p` at that path inside a Linux container, so the two sides
    /// see different absolute paths and git's worktree metadata — which stores absolute paths —
    /// would disagree. A remote daemon has no shared filesystem at all. On Linux and macOS the
    /// mount is the identity and the client answers this itself, without a round trip.
    RepoRootQuery {
        /// Correlation id.
        req: u64,
        /// The directory the user chose, **as the daemon will see it**.
        path: PathBuf,
    },
    /// List every local and remote-tracking branch, annotated with why each is unavailable, for
    /// the existing-branch picker (feature 016, FR-011). Reads local ref storage only — the daemon
    /// never contacts a remote for this (FR-020).
    BranchList {
        /// Correlation id.
        req: u64,
        /// Project path.
        project: PathBuf,
    },
    /// Show a worktree the repository already knows about that lives outside the directory this
    /// app creates its own in (016 BUG-002, FR-027). **Mutates nothing but the app's own settings**
    /// — no git command runs, because the worktree is already registered, which is precisely why it
    /// could block a branch. Idempotent.
    WorktreeInclude {
        /// Correlation id.
        req: u64,
        /// Project path.
        project: PathBuf,
        /// Absolute path of the worktree to show.
        path: PathBuf,
    },
    /// Stop showing an included worktree (016 BUG-002, FR-030). Removes it from the list and leaves
    /// it exactly as it is on disk. Idempotent.
    WorktreeExclude {
        /// Correlation id.
        req: u64,
        /// Project path.
        project: PathBuf,
        /// Absolute path of the worktree to stop showing.
        path: PathBuf,
    },
    /// Delete a worktree. `stop_sessions` MUST be true if sessions are live, else it fails (W2).
    WorktreeDelete {
        /// Correlation id.
        req: u64,
        /// Project path.
        project: PathBuf,
        /// Worktree directory name.
        dir_name: String,
        /// Whether to stop live sessions first.
        stop_sessions: bool,
        /// Whether to also delete the worktree's git branch (feature 013, FR-011/FR-012).
        /// `true` is today's (and the spec's) default; `false` keeps the branch.
        delete_branch: bool,
    },
    /// Rename a worktree's display name.
    WorktreeRename {
        /// Correlation id.
        req: u64,
        /// Project path.
        project: PathBuf,
        /// Worktree directory name.
        dir_name: String,
        /// New display name.
        display_name: String,
    },
    /// Create a session bound to a worktree.
    SessionCreate {
        /// Correlation id.
        req: u64,
        /// Project path.
        project: PathBuf,
        /// Worktree directory name.
        worktree_dir: String,
    },
    /// Delete a session record.
    SessionDelete {
        /// Correlation id.
        req: u64,
        /// Target session.
        session: SessionId,
    },
    /// Set service-owned settings: the scrollback limit (FR-012a) and/or the environment-include
    /// setting (FR-012b). Each field is independently optional — `None` leaves that setting
    /// unchanged.
    SettingsSet {
        /// Correlation id.
        req: u64,
        /// New scrollback line cap, or `None` to leave unchanged.
        scrollback_lines: Option<usize>,
        /// New environment-include enabled flag, or `None` to leave unchanged.
        env_include_enabled: Option<bool>,
        /// New environment-include script path, or `None` to leave unchanged.
        env_include_script_path: Option<String>,
        /// New environment-include timeout in seconds, or `None` to leave unchanged.
        env_include_timeout_secs: Option<u64>,
    },

    // --- Diagnostics ---
    /// Ask where the daemon writes its log.
    LogLocationRequest {
        /// Correlation id.
        req: u64,
    },
    /// Ask for the most recent daemon error entries.
    RecentErrorsRequest {
        /// Correlation id.
        req: u64,
        /// Maximum number of entries.
        limit: u32,
    },
    /// Reload the runtime `EnvFilter` directives (FR-043).
    SetLogLevel {
        /// Correlation id.
        req: u64,
        /// The new `tracing` directives.
        directives: String,
    },

    // --- Keepalive ---
    /// Liveness probe (protocol.md §5 Liveness). Answered with [`DaemonMsg::Pong`].
    Ping {
        /// Opaque echo value.
        nonce: u64,
    },
}

// ---------------------------------------------------------------------------------------------
// Daemon → Client
// ---------------------------------------------------------------------------------------------

/// A message from the daemon to a client.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DaemonMsg {
    // --- Connection ---
    /// Handshake accepted.
    Welcome {
        /// Human-facing daemon build string.
        daemon_build: String,
        /// Full catalog snapshot.
        catalog: CatalogSnapshot,
        /// Current service settings.
        settings: DaemonSettings,
    },
    /// Handshake or attach refused.
    Refused {
        /// Why.
        reason: RefusalReason,
    },
    /// Attach accepted; the current session set for the project.
    Attached {
        /// Project identity path.
        project: PathBuf,
        /// Its sessions.
        sessions: Vec<SessionSummary>,
    },
    /// This client lost a project to a takeover. MUST NOT terminate the client (FR-024).
    Displaced {
        /// Project identity path.
        project: PathBuf,
        /// Who took over (build/identity string).
        by: String,
    },
    /// Keepalive reply.
    Pong {
        /// Echo of the [`ClientMsg::Ping`] nonce.
        nonce: u64,
    },

    // --- State projection (pushed, unsolicited) ---
    /// Full catalog snapshot — idempotent, self-healing (messages.md §Ordering 4).
    CatalogChanged {
        /// The new snapshot.
        catalog: CatalogSnapshot,
    },
    /// A long-running operation reached a new stage (feature 016, FR-024).
    ///
    /// Pushed between the request and its `OperationOk`/`OperationError`, so the client can name
    /// the step actually being performed instead of a generic "working…". Lossy by nature: a
    /// client that misses one simply shows the next stage, and the terminal reply is what closes
    /// the operation. Only the *stage* travels — the wording is the client's, derived from the
    /// stage and the mode it asked for.
    OperationProgress {
        /// Correlation id of the in-flight request.
        req: u64,
        /// The stage that operation just entered.
        stage: CreateStage,
        /// The operation's most recent live output line, when the stage has been running long
        /// enough to have some (BUG-009, T123). `None` on the frame that announces a stage.
        ///
        /// Rate-limited by the sender, not per line: a submodule fetch emits thousands, and this
        /// is a "still working, here's where" signal rather than a log to be reassembled. Lossy
        /// like the stage itself — a missed line is simply superseded by the next.
        detail: Option<String>,
    },
    /// A single session's summary changed.
    SessionChanged {
        /// Which session.
        session: SessionId,
        /// Its new summary.
        summary: SessionSummary,
    },
    /// Service settings changed.
    SettingsChanged {
        /// The new settings.
        settings: DaemonSettings,
    },

    // --- Terminal-originated notifications ---
    /// OSC title change.
    SessionTitleChanged {
        /// Which session.
        session: SessionId,
        /// New title, or `None` to clear.
        title: Option<String>,
    },
    /// Terminal bell.
    SessionBell {
        /// Which session.
        session: SessionId,
    },
    /// The session's process exited.
    SessionExited {
        /// Which session.
        session: SessionId,
        /// How it exited.
        status: ExitStatus,
        /// Whether the daemon is auto-restarting it.
        restarting: bool,
    },
    /// The terminal requested a clipboard write.
    ClipboardStore {
        /// Which session.
        session: SessionId,
        /// The content to store.
        content: String,
    },

    // --- Scrollback ---
    /// A chunked scrollback response (protocol.md §6).
    ScrollbackResponse {
        /// Which session.
        session: SessionId,
        /// Correlation id echoed from the request.
        req: u64,
        /// Oldest retained line.
        oldest_available: LineId,
        /// Newest retained line.
        newest: LineId,
        /// The returned lines — may be fewer than requested (advisory, not an error).
        lines: Vec<WireLine>,
        /// The interned style palette the `lines`' `StyleRun`s index into (per-response, like a
        /// `GridFrame`'s own palette — the client resolves against it and never across responses).
        styles: Vec<WireStyle>,
        /// The interned hyperlink URIs the `lines`' `CellExtras` index into (per-response).
        hyperlinks: Vec<String>,
        /// Whether more chunks follow.
        more: bool,
    },

    // --- Operation results ---
    /// A mutating request succeeded.
    OperationOk {
        /// Correlation id.
        req: u64,
        /// The result payload.
        result: OperationResult,
    },
    /// A mutating request failed specifically and actionably (FR-031/034).
    OperationError {
        /// Correlation id.
        req: u64,
        /// Category.
        kind: ErrorKind,
        /// Human-facing summary.
        message: String,
        /// The underlying diagnostic, preserved verbatim (e.g. git's stderr).
        detail: Option<String>,
    },

    // --- Diagnostics ---
    /// Where the daemon logs.
    LogLocation {
        /// Correlation id.
        req: u64,
        /// The log file path, if it logs to a file.
        path: Option<PathBuf>,
        /// The active sink.
        sink: LogSink,
    },
    /// Recent daemon error entries.
    RecentErrors {
        /// Correlation id.
        req: u64,
        /// The entries.
        entries: Vec<LogEntry>,
    },
}

// ---------------------------------------------------------------------------------------------
// Supporting types
// ---------------------------------------------------------------------------------------------

/// Why a handshake or attach was refused (contracts/messages.md).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RefusalReason {
    /// Version or schema-hash mismatch. Names **both** sides so the client can render an actionable
    /// diagnostic and offer a restart (FR-021/022).
    VersionMismatch {
        /// Client protocol version.
        client: u32,
        /// Daemon protocol version.
        daemon: u32,
        /// Client schema hash.
        client_hash: [u8; 32],
        /// Daemon schema hash.
        daemon_hash: [u8; 32],
        /// Daemon build string.
        daemon_build: String,
    },
    /// Same wire contract, different package version — a `.deb` upgrade over an already-running
    /// daemon that a `VersionMismatch` would not catch (FR-022a, BUG-002). Names both builds so the
    /// client can render a distinct, lower-severity diagnostic and offer the same restart action,
    /// without implying that sessions are put at risk (the contract still matches).
    BuildMismatch {
        /// Client build string.
        client_build: String,
        /// Daemon build string.
        daemon_build: String,
    },
    /// The project already has an attached client.
    ProjectBusy {
        /// Project identity path.
        project: PathBuf,
        /// The current holder's identity.
        holder: String,
        /// How long it has been held.
        since_secs: u64,
    },
    /// A generic refusal with a specific reason.
    NotPermitted {
        /// The detail.
        detail: String,
    },
    /// The handshake presented no token, or the wrong one (feature 027, R1).
    ///
    /// Carries nothing about *how* wrong the token was — no length, no prefix, no distinction
    /// between absent and incorrect. A refusal that described the difference would be an oracle for
    /// recovering the token one guess at a time.
    AuthRejected,
    /// The daemon was built from a different working tree than the client, and the client said that
    /// was a refusal because the image is a local build (feature 027, FR-024d, research R8).
    ///
    /// Names the image so the remedy can point at something the user can act on: the three
    /// constants the handshake already compares all match here, which is exactly why this exists.
    StaleDevImage {
        /// The client's fingerprint.
        client_fingerprint: String,
        /// The daemon's fingerprint.
        daemon_fingerprint: String,
        /// The image reference the daemon is running from, when the client knows it.
        image: String,
    },
}

/// The projected summary of a single session, sent for **every** session so the client can render
/// the activity indicator in the session list (FR-016d).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionSummary {
    /// Stable identity.
    pub id: SessionId,
    /// Hosting worktree directory name, or `None` for a session hosted directly by the project
    /// root (the "Default" location, feature 010-root-dir-session — mirrors the durable
    /// `StoredSession.worktree_dir: Option<String>`).
    pub worktree_dir: Option<String>,
    /// Sidebar label.
    pub title: SessionLabel,
    /// Lifecycle state (the wire form — includes `InterruptedResumable` and `Failed{..}`).
    pub lifecycle: WireLifecycle,
    /// Derived activity signal.
    pub activity: ActivitySignal,
    /// The input serial the service expects next for this session — its
    /// [`InputReceiver`](crate::input::InputReceiver) high-water mark (FR-028a, BUG-006).
    ///
    /// Authoritative, and runtime-only: the receiver lives with the live session, so a client that
    /// did not start this session cannot infer it. A client MUST adopt this value when it has no
    /// counter of its own for the session, or its first keystroke is stamped `0`, classified
    /// [`Stale`](crate::input::InputOutcome::Stale), and silently dropped — along with every one
    /// after it. Sessions the service is not hosting have no receiver and report `0`.
    pub input_serial: u64,
}

/// The wire form of a session's lifecycle (data-model §SessionLifecycle state machine).
///
/// Distinct from [`crate::session::SessionLifecycle`] because the wire must express two states the
/// in-process domain enum does not yet carry — `InterruptedResumable` (FR-006a) and a `Failed`
/// variant with a persisted reason + attempt count (S3). T073 wires the domain↔wire mapping.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum WireLifecycle {
    /// Identity exists, no process.
    Idle,
    /// Spawn requested, process not yet confirmed up.
    Starting,
    /// Process alive.
    Running,
    /// Unexpected exit; a retry is in progress.
    Restarting {
        /// Consecutive failed (re)starts so far.
        attempts: u8,
    },
    /// Retries exhausted or spawn failed — persisted, manually restartable (S3).
    Failed {
        /// Why it gave up.
        reason: String,
        /// How many attempts were made.
        attempts: u8,
    },
    /// The daemon restarted and found a durable record of a running session. Never auto-relaunched
    /// (FR-006a/b).
    InterruptedResumable,
}

/// Derived activity signal (data-model §ActivitySignal). `Unknown` MUST NEVER be rendered as
/// `AwaitingInput` (A1).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ActivitySignal {
    /// No signal yet, or hooks unconfigured. Ambient, never a notification.
    Unknown,
    /// Actively working. Ambient.
    Working,
    /// Blocked awaiting the user. Notification-grade.
    AwaitingInput,
    /// The session ended.
    Ended {
        /// Why it ended.
        reason: String,
    },
}

/// A full catalog snapshot (data-model §Catalog). Idempotent by construction.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct CatalogSnapshot {
    /// Persistence schema version.
    pub schema_version: u32,
    /// The last-active project, if any.
    pub last_active: Option<PathBuf>,
    /// The projects.
    pub projects: Vec<ProjectSnapshot>,
}

/// A project within a [`CatalogSnapshot`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProjectSnapshot {
    /// Identity path (lexically canonicalised).
    pub path: PathBuf,
    /// Display name.
    pub display_name: String,
    /// Whether the path is a git repository.
    pub is_git_repo: bool,
    /// Whether the project is currently reachable on disk (derived, never persisted).
    pub available: bool,
    /// Its worktrees.
    pub worktrees: Vec<WorktreeSnapshot>,
    /// Its sessions.
    pub sessions: Vec<SessionSummary>,
}

/// A worktree within a [`ProjectSnapshot`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorktreeSnapshot {
    /// The identity a session binds to.
    pub dir_name: String,
    /// Its branch, if known.
    pub branch: Option<String>,
    /// Its display name.
    pub display_name: String,
    /// Its status.
    pub status: WorktreeStatus,
    /// Where it is on disk (016 BUG-002, FR-029). Carried for every worktree so one type answers
    /// for all of them, and load-bearing for the [included](Self::included) ones: a folder name
    /// says nothing about where a worktree the app did not create actually lives.
    pub path: PathBuf,
    /// Shown because the user asked for it, not because of where it lives (016 BUG-002, FR-027).
    pub included: bool,
}

/// Worktree status (data-model §Worktree).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum WorktreeStatus {
    /// Present and healthy.
    Clean,
    /// The directory is gone.
    Missing,
    /// Git-locked.
    Locked,
    /// Prunable per git.
    Prunable,
}

/// The service-owned settings mirrored to clients (FR-012a, FR-012b).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DaemonSettings {
    /// The scrollback retention limit, in lines.
    pub scrollback_lines: usize,
    /// Whether the environment-include script is sourced for spawned sessions (FR-012b).
    pub env_include_enabled: bool,
    /// The configured environment-include script path.
    pub env_include_script_path: String,
    /// The environment-include sourcing timeout, in seconds.
    pub env_include_timeout_secs: u64,
}

/// The result payload of a successful mutating request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum OperationResult {
    /// No payload — the operation simply succeeded.
    Ack,
    /// A session was created.
    SessionCreated {
        /// The new session's identity.
        session: SessionId,
    },
    /// A worktree was created.
    WorktreeCreated {
        /// The new worktree's directory name.
        dir_name: String,
    },
    /// A worktree was deleted (feature 013, FR-011/FR-015). git has released the worktree and its
    /// sessions are archived by this point; `branch_delete_failed` and `leftovers` report the two
    /// separate, non-fatal parts that can still come up short.
    WorktreeDeleted {
        /// `true` when branch deletion was requested but git could not delete it (e.g. it holds
        /// commits unreachable from elsewhere). Always `false` when the branch was kept.
        branch_delete_failed: bool,
        /// Paths of the worktree directory that could not be removed — empty on the ordinary
        /// path. Non-empty means the directory survives and will reappear as an unregistered
        /// orphan, so the client must say which paths blocked it and why (usually another uid).
        leftovers: Vec<crate::worktree::Leftover>,
    },
    /// The classification of a branch name (feature 016, FR-001).
    BranchPreflight {
        /// What stands between the user and a new worktree on that name.
        situation: BranchSituation,
    },
    /// The repository's branches for the picker (feature 016, FR-011).
    BranchList {
        /// Every branch, ordered and annotated with any block reason.
        candidates: Vec<BranchCandidate>,
    },
    /// A worktree is now shown (016 BUG-002, FR-027). Carries it as discovery sees it, so the
    /// client renders the daemon's answer rather than deriving a second one.
    WorktreeIncluded {
        /// The worktree, exactly as the catalog now lists it.
        worktree: WorktreeSnapshot,
    },
    /// A worktree is no longer shown (016 BUG-002, FR-030). Nothing on disk changed.
    WorktreeExcluded {
        /// The path that is no longer shown.
        path: PathBuf,
    },
    /// The answer to [`ClientMsg::RepoRootQuery`] (feature 027, research R2 part 2).
    ///
    /// Carries the path back so a client that has moved on since asking — the user cancelled, or
    /// chose a different folder — can tell that this answer is about something else.
    RepoRoot {
        /// The directory that was asked about.
        path: PathBuf,
        /// Whether the daemon's git calls it a repository root.
        is_repo_root: bool,
    },
}

/// The category of a failed mutating request (contracts/messages.md).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ErrorKind {
    /// The target does not exist.
    NotFound,
    /// The target already exists.
    AlreadyExists,
    /// The request was malformed.
    InvalidInput,
    /// The target is busy (e.g. a live session blocks a delete).
    Busy,
    /// A git invocation failed; `detail` carries git's stderr **verbatim** (FR-034).
    GitFailed,
    /// A filesystem operation failed.
    IoFailed,
    /// The operation was refused by policy.
    Refused,
    /// An unexpected internal error.
    Internal,
}

/// Where the daemon writes its log.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LogSink {
    /// Standard error.
    Stderr,
    /// systemd journal.
    Journald,
    /// A rotating file.
    File,
}

/// A single log entry surfaced to the diagnostics UI. MUST NOT contain terminal content or user
/// input (FR-047) — it references sessions by identity and state only.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LogEntry {
    /// Wall-clock seconds since the Unix epoch.
    pub timestamp_secs: u64,
    /// Level (`ERROR`, `WARN`, …).
    pub level: String,
    /// The `tracing` target.
    pub target: String,
    /// The (redacted) message.
    pub message: String,
}

/// How a process exited (a serializable stand-in for `std::process::ExitStatus`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExitStatus {
    /// The exit code, if it exited normally.
    pub code: Option<i32>,
    /// The terminating signal, if it was killed by one (Unix).
    pub signal: Option<i32>,
}
