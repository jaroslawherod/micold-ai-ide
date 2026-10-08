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

use crate::attach::{AttachItem, AttachResult, DiscoveryReport};
use crate::attention::{NotificationKind, NotificationKinds};
use crate::cli_reason::SpawnEnv;
use crate::mcp::policy::CrossSessionAccess;
use crate::protocol::grid::{LineId, WireLine, WireStyle};
use crate::review::comment::{CommentId, ReviewComment};
use crate::review::Side;
use crate::naming::WorktreeNaming;
use crate::runs::{GroupId, Integration, RunGroup};
use crate::session::{AiCli, SessionId, SessionLabel, ShellInstanceId};
use crate::settings::DiffLayout;
use crate::theme::ColorScheme;
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

/// Identifies one client *process* — one window — across every connection it makes.
///
/// The protocol had no such thing until `010` BUG-022: a connection was identified by its
/// `ClientId`, which dies with it, and a client was described by its build string, which is
/// identical for every window of one binary. Between those two there was no way to say "the
/// connection that just attached and the connection that just dropped are the same window", and a
/// window whose keepalive expired therefore displaced *itself* on reconnect and locked itself out
/// of its own project.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ClientInstance {
    /// The client process's OS pid. Human-facing: it is what a banner can name so that two windows
    /// of one build are told apart by something the user can also see in a process list. Not
    /// unique on its own — pids are reused, and two clients dialling one daemon need not share a
    /// host — which is what `nonce` is for.
    pub pid: u32,
    /// A per-process random token. This is what makes the identity *unique*; comparisons are on
    /// the whole struct, never on `pid` alone.
    pub nonce: String,
}

impl ClientInstance {
    /// This process's instance, generated once and stable for the life of the process.
    ///
    /// Stability across reconnects is the entire point: a window that reconnects must present the
    /// same instance it presented before, or the daemon's refusal and displacement frames cannot
    /// be recognised as being about itself.
    pub fn current() -> Self {
        static CURRENT: std::sync::OnceLock<ClientInstance> = std::sync::OnceLock::new();
        CURRENT
            .get_or_init(|| ClientInstance {
                pid: std::process::id(),
                nonce: uuid::Uuid::new_v4().simple().to_string(),
            })
            .clone()
    }
}

/// Who a client is, as the daemon reports it to *other* clients: a build string to show and an
/// instance to compare.
///
/// Both halves are load-bearing and neither substitutes for the other. `build` is what a person
/// reads; `instance` is what a window compares against its own to know whether a displacement or a
/// refusal is about itself (`010` BUG-022) — a comparison `build` cannot make, because two windows
/// of one binary carry the same one.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClientIdentity {
    /// The client's human-facing build string.
    pub build: String,
    /// The client's process instance.
    pub instance: ClientInstance,
}

impl ClientIdentity {
    /// Build an identity from its two halves.
    pub fn new(build: impl Into<String>, instance: ClientInstance) -> Self {
        Self {
            build: build.into(),
            instance,
        }
    }

    /// Whether this identity names `instance` — i.e. the same window.
    pub fn is(&self, instance: &ClientInstance) -> bool {
        self.instance == *instance
    }
}

impl fmt::Display for ClientIdentity {
    /// The banner form. Names the window, not only the build: `010` BUG-023 filed the build-only
    /// string as naming nothing, because it is the same for the window reading it.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} (window {})", self.build, self.instance.pid)
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
        /// Which *window* this is (`010` BUG-022). Unlike `client_build` it is unique per client
        /// process and stable across the connections that process makes, so the daemon can tell
        /// another window's attach from this one's reconnect — and so a client can recognise a
        /// displacement or a refusal that names itself.
        client_instance: ClientInstance,
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
    /// The client's resolved light/dark scheme (`006` FR-003a, BUG-007). Sent on every connection
    /// before `Attach` and again whenever it changes; the daemon answers the dynamic-colour queries
    /// (`OSC 10/11/12`) from the last one any client sent, and never acknowledges it
    /// (`contracts/messages.md`, `contracts/protocol.md` §8 of `010`).
    TerminalColorScheme {
        /// The scheme the client's window is drawn in.
        scheme: ColorScheme,
    },
    /// What this window has in view (feature 039, W1.1). Sent once after every `Welcome` and
    /// again whenever the value changes. Not an operation: there is no `req` and no reply, and it
    /// needs no project attachment (W1.5, W1.6).
    WindowView {
        /// The window has keyboard focus.
        focused: bool,
        /// The session the window has in view. `Some` only with `focused: true`.
        in_view: Option<SessionId>,
    },
    /// This window asks to be the one that raises the notification for a session's attention event
    /// (feature 039, W1.3). Answered with [`DaemonMsg::AttentionGranted`] to this connection only,
    /// or not at all when the event was granted already.
    AttentionClaim {
        /// The session that changed.
        session: SessionId,
        /// The `attention_seq` being claimed.
        seq: u64,
    },
    /// A notification for `session` was clicked in this window: ask for the session to be shown
    /// (feature 039, W3). The service forwards it as [`DaemonMsg::RevealSession`] to exactly one
    /// window (W3.1) without checking that the project or session exists (W3.2), and changes
    /// nothing (W3.3). Not an operation: no `req`, no reply, no attachment needed.
    SessionReveal {
        /// The project the session belongs to.
        project: PathBuf,
        /// The session to show.
        session: SessionId,
        /// The Wayland activation token of the click, when the notification service sent one
        /// (W3.4). The service forwards it unread.
        activation: Option<String>,
    },

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
    /// A user's manual restart of the session's AI CLI: as `SessionStart`, after re-sourcing the
    /// environment-include script for the session's directory (011 FR-007(b), BUG-442). A plain
    /// `SessionStart` (selecting a session, reconnecting) is served the cached environment.
    SessionRestart {
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
    /// Record that a project already in the catalog is now the active one, so it is the project
    /// the next launch restores (002 FR-010/FR-011, BUG-003). Refused as `NotFound` when the path
    /// is unknown or its folder is unavailable — the catalog never names such a project as last
    /// active (FR-023).
    ProjectActivate {
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
    /// List the project repository's own remotes and their URLs, in config order (feature 034,
    /// FR-002), so the client can find the GitHub repository behind them. Reads local config
    /// only; the daemon never contacts a remote for this. A non-repository is refused exactly as
    /// [`ClientMsg::BranchList`] refuses it.
    RemoteList {
        /// Correlation id.
        req: u64,
        /// Project path.
        project: PathBuf,
    },
    /// Ask what `project` offers to attach: provider worktrees without a provenance record and
    /// stored sessions the catalog does not hold (feature 582, FR-001). Read-only; answered with
    /// [`DaemonMsg::AttachReport`].
    AttachDiscover {
        /// Correlation id.
        req: u64,
        /// Project path.
        project: PathBuf,
    },
    /// Attach `targets` in one action (feature 582): one catalog write and one broadcast for the
    /// whole batch. Answered with [`OperationResult::AttachApplied`].
    AttachApply {
        /// Correlation id.
        req: u64,
        /// Project path.
        project: PathBuf,
        /// What to attach.
        targets: Vec<AttachItem>,
    },
    /// Ask, for each merged pull request's branch, whether the local branch holds commits beyond
    /// the pull request's last commit (feature 040, FR-015, FR-017). Read-only and local: the
    /// daemon reads refs and the object store, fetches nothing and writes nothing. At most 50
    /// queries; a longer list is refused. A non-repository is refused exactly as
    /// [`ClientMsg::RemoteList`] refuses it.
    MergedBranchCheck {
        /// Correlation id.
        req: u64,
        /// Project path.
        project: PathBuf,
        /// The branches to check, each with its pull request's last commit.
        checks: Vec<MergedBranchQuery>,
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
    /// Re-read a project's worktrees from git + the filesystem, on the user's explicit request
    /// (feature 029, FR-003). **Mutates nothing** — neither the repository nor the app's own
    /// settings. Idempotent, and safe to send at any time a project is open.
    ///
    /// Correlated despite mutating nothing, for the reason [`ClientMsg::BranchList`] is and one
    /// more that is load-bearing: the reply is the only thing that ends the refresh control's
    /// busy state (029 FR-007), and an uncorrelated request could not.
    ///
    /// The refreshed listing is **not** in the reply — it arrives as the `CatalogChanged` the
    /// daemon already broadcasts, so a refreshed listing travels the one path every other listing
    /// travels. That is 029 FR-004's implementation, not merely its consequence: a user must not
    /// be able to tell which trigger produced the list, and one path guarantees that where two
    /// kept in agreement would only promise it.
    WorktreeRefresh {
        /// Correlation id.
        req: u64,
        /// Project path.
        project: PathBuf,
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
    /// Record a worktree the app did not create as the user's own (feature 029, FR-020).
    ///
    /// Durable catalog state with no git involved, exactly like [`Self::WorktreeRename`] — the
    /// worktree directory and its branch are untouched (FR-003). It is answered with
    /// [`OperationResult::Ack`] and a catalog broadcast, or with
    /// [`ErrorKind::IoFailed`] if the record cannot be persisted; there is no `InvalidInput`
    /// case, because unlike a rename there is no user-supplied string to validate.
    ///
    /// There is deliberately no inverse (FR-024): a worktree leaves the user-owned set only by
    /// being deleted, or by its project being forgotten.
    WorktreeClaim {
        /// Correlation id.
        req: u64,
        /// Project path.
        project: PathBuf,
        /// Worktree directory name.
        dir_name: String,
    },
    /// Create a session bound to a worktree.
    SessionCreate {
        /// Correlation id.
        req: u64,
        /// Project path.
        project: PathBuf,
        /// Worktree directory name.
        worktree_dir: String,
        /// Which AI CLI to run (feature 026, FR-004). The **client** resolves default-or-override
        /// and sends the answer, so no launch depends on the two processes agreeing about a file.
        /// The daemon does read `settings.json` — `Catalog::new` loads it at boot — it is simply
        /// not asked to make this choice.
        provider: AiCli,
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
        /// New default AI CLI, or `None` to leave unchanged (feature 026, FR-003).
        default_ai_cli: Option<AiCli>,
        /// Load Pi's activity component into new Pi sessions, or `None` to leave unchanged
        /// (feature 029, FR-012e).
        pi_activity_component: Option<bool>,
        /// Bind new sessions to the service's tool server, or `None` to leave unchanged
        /// (feature 034, FR-004).
        tool_server_enabled: Option<bool>,
        /// Whether agents may read and type into other sessions, or `None` to leave unchanged
        /// (feature 034, FR-016).
        cross_session_access: Option<CrossSessionAccess>,
        /// Read and show pull request status, or `None` to leave unchanged (feature 040,
        /// FR-029).
        pr_status_enabled: Option<bool>,
        /// Raise desktop notifications, or `None` to leave unchanged (feature 039, FR-026).
        desktop_notifications: Option<bool>,
        /// The per-kind switches, all four at once, or `None` to leave them unchanged
        /// (feature 613, W5.4).
        notification_kinds: Option<NotificationKinds>,
        /// The long-task threshold in seconds, or `None` to leave it unchanged (feature 613,
        /// W5.6). The service clamps it into 10–3600 (FR-026).
        long_task_threshold_secs: Option<u64>,
        /// The Changes view's diff layout, or `None` to leave unchanged (feature 482, R12).
        diff_layout: Option<DiffLayout>,
    },

    // --- Review comments (feature 482, contracts/review-wire.md) ---
    /// Change an entry's review comments (W1–W5). Answered `OperationOk(Ack)` once the change is
    /// stored, and pushed to every attached client as [`DaemonMsg::ReviewChanged`].
    ReviewEdit {
        /// Correlation id.
        req: u64,
        /// The project.
        project: PathBuf,
        /// The worktree directory name; `""` is the Default entry (the project root).
        worktree_dir: String,
        /// What to change.
        edit: ReviewEditOp,
    },
    /// Send the entry's pending comments to its session as one prompt (W6–W10). Answered
    /// `OperationOk(ReviewSent)` once delivered.
    ReviewSend {
        /// Correlation id.
        req: u64,
        /// The project.
        project: PathBuf,
        /// The worktree directory name; `""` is the Default entry.
        worktree_dir: String,
        /// The comments the client judged outdated (R14), used only for the prompt's wording.
        outdated: Vec<CommentId>,
    },

    // --- Parallel runs (feature 483, contracts/run-group-wire.md) ---
    /// Run one prompt across several agents: one worktree and one session per provider, kept as
    /// one group (W1). Answered `OperationOk(RunGroupCreated)` once the group is recorded; each
    /// run then progresses on its own and is reported by [`DaemonMsg::RunGroupsChanged`] (W2).
    RunGroupCreate {
        /// Correlation id.
        req: u64,
        /// The project.
        project: PathBuf,
        /// Type, optional ticket and name the runs' names are derived from.
        naming: WorktreeNaming,
        /// The prompt every run's session receives once, as its first input.
        prompt: String,
        /// The local branch every run starts from.
        base_branch: String,
        /// One provider per run, in run order; `MIN_RUNS..=MAX_RUNS` of them.
        providers: Vec<AiCli>,
    },
    /// Pick a group's run: integrate its branch into the group's base branch (W3).
    RunGroupPick {
        /// Correlation id.
        req: u64,
        /// The project.
        project: PathBuf,
        /// The group.
        group: GroupId,
        /// The run's number.
        run: u8,
    },
    /// Forget a group, leaving its worktrees, branches and sessions as they are (W4).
    RunGroupDismiss {
        /// Correlation id.
        req: u64,
        /// The project.
        project: PathBuf,
        /// The group.
        group: GroupId,
    },

    // --- AI CLIs ---
    /// Ask which AI CLIs exist **where sessions run** (feature 027, FR-023c).
    ///
    /// The client cannot answer this for itself. Under sandboxed placement it is on the host and
    /// the sessions are in the container, so its own `PATH` describes the wrong machine — and
    /// describes it plausibly, which is worse than describing nothing. The service is the process
    /// that spawns the CLI, so the service is the one that gets to say whether it is there.
    ///
    /// Answered with [`DaemonMsg::AiCliAvailability`]. Asked when the choice is *offered* — the
    /// Settings view opening, the per-session override menu opening — and never per frame: the
    /// answer changes when an image changes, not between renders.
    AiCliAvailabilityRequest {
        /// Correlation id.
        req: u64,
        /// The directory the choice is being made for — a project root or a worktree — or `None`
        /// where no directory is in play (Settings), which the service answers for the user's home
        /// directory (feature 029, BUG-001, FR-003b). The answer follows the environment a session
        /// started there would be spawned with, so a CLI that only the environment-include script
        /// puts on `PATH` is offered exactly where a session would find it.
        cwd: Option<PathBuf>,
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

    // --- Agent confirmations (feature 034, FR-014) ---
    /// The user's answer to a [`DaemonMsg::ConfirmationRequested`]. The first answer for a live
    /// `id` decides; a later one, or one for an unknown `id`, is ignored.
    ConfirmationAnswer {
        /// The prompt's id.
        id: u64,
        /// `true` performs the operation; `false` refuses it ("declined by the user").
        allow: bool,
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

/// Why the daemon refused to open or restart a shell instance (feature 010 FR-006c, BUG-592).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ShellOpenFailure {
    /// The session's working directory (its worktree) no longer exists on disk.
    WorkingDirMissing,
    /// Any other refusal, carrying the daemon's own description of it.
    Other(String),
}

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
    /// The claim for this session's event `seq` is granted to this window: it raises the
    /// notification (feature 039, W1.3). Sent to the claimer only.
    AttentionGranted {
        /// The session that changed.
        session: SessionId,
        /// The granted `attention_seq`.
        seq: u64,
        /// The kind the service decided when it noted event `seq` (feature 613, W5.1). Never
        /// `SessionError`.
        kind: NotificationKind,
    },
    /// Show this session: a notification for it was clicked, in this window or another
    /// (feature 039, W3). Sent to one window only; that window decides whether the session can
    /// still be shown (W3.2).
    RevealSession {
        /// The project the session belongs to.
        project: PathBuf,
        /// The session to show.
        session: SessionId,
        /// The activation token of the click, as the sender wrote it (W3.4).
        activation: Option<String>,
    },
    /// A session not in view ended because of an error: this window raises its **Session error**
    /// notification (feature 613, W5.2). Sent to one window only, once per error ending, and
    /// never answered.
    SessionErrorNotice {
        /// The project the session belongs to.
        project: PathBuf,
        /// The session that ended.
        session: SessionId,
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
        /// Who took over. Carries the instance as well as the build so the receiver can tell a
        /// real takeover from its own reconnect displacing its own dead connection (BUG-022).
        by: ClientIdentity,
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
    /// An entry's review comments changed (feature 482): pushed to every client attached to
    /// `project` after `Attached` and after every change, an empty list once they are gone.
    ReviewChanged {
        /// The project.
        project: PathBuf,
        /// The worktree directory name; `""` is the Default entry.
        worktree_dir: String,
        /// Every comment of the entry, pending and sent.
        comments: Vec<ReviewComment>,
        /// A send is in progress for this entry: sending, editing and deleting are unavailable.
        sending: bool,
    },
    /// A project's run groups changed (feature 483): the whole list, pushed to every client
    /// attached to `project` after `Attached` and after every change. Idempotent.
    RunGroupsChanged {
        /// The project.
        project: PathBuf,
        /// Every group of the project, oldest first.
        groups: Vec<RunGroup>,
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
    /// A `SessionOpenShell` or `SessionRestartShell` was refused, so the instance does not exist
    /// (FR-006c, BUG-592). Sent to the requesting client only: those requests have no reply
    /// otherwise, and a client that opened the instance optimistically must take it back.
    ShellOpenFailed {
        /// Which session.
        session: SessionId,
        /// The instance that was not opened.
        instance: ShellInstanceId,
        /// Why.
        reason: ShellOpenFailure,
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
    /// The answer to [`ClientMsg::AttachDiscover`] (feature 582).
    AttachReport {
        /// Correlation id.
        req: u64,
        /// What the project offers to attach.
        report: DiscoveryReport,
    },
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

    // --- AI CLIs ---
    /// Which AI CLIs the service found in its own environment (feature 027, FR-023c).
    ///
    /// Only the fact the service alone has. It does **not** say which image it is running or
    /// whether it is containerised at all: the client started it and holds both, and a second
    /// copy of a fact the client already owns is a second thing that can disagree. What the client
    /// composes from the two is FR-023b's sentence — naming the CLI, the image, and that the image
    /// must provide it.
    AiCliAvailability {
        /// Correlation id.
        req: u64,
        /// Present, in `AiCli::ALL`'s order. Empty is a real answer, not an error.
        available: Vec<AiCli>,
        /// The state of the environment `available` was walked in (feature 037, FR-012): why a
        /// CLI that is not in the set would not be found. `None` when the service could not
        /// resolve a directory at all, so it has no attempt to report.
        env: Option<SpawnEnv>,
    },

    // --- Agent confirmations (feature 034, FR-014) ---
    /// An agent asked for a destructive operation; every window shows this prompt until one
    /// answers, it expires, or it is withdrawn. Also sent, once per pending prompt, to a window
    /// that completes its handshake while prompts are pending.
    ConfirmationRequested {
        /// The prompt's id, echoed by [`ClientMsg::ConfirmationAnswer`].
        id: u64,
        /// The project both the caller and the target belong to.
        project: PathBuf,
        /// The calling session.
        caller: SessionId,
        /// The calling session's display label.
        caller_label: String,
        /// What the agent asks to do.
        operation: ConfirmOperation,
        /// The target's display name: a worktree's display name or a session's label.
        target_label: String,
        /// Milliseconds left of the 60 s the prompt waits for an answer.
        expires_in_ms: u32,
    },
    /// The prompt is resolved (answered, expired, abandoned, or its target is gone); every window
    /// closes it.
    ConfirmationWithdrawn {
        /// The prompt's id.
        id: u64,
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

/// The destructive operation a [`DaemonMsg::ConfirmationRequested`] asks the user to allow
/// (feature 034, FR-014). Names the operation only: `SendInput` deliberately carries no text.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConfirmOperation {
    /// Delete a worktree.
    DeleteWorktree {
        /// Its live sessions are stopped first.
        stop_sessions: bool,
        /// Its branch is deleted too.
        delete_branch: bool,
    },
    /// Delete (archive) a session.
    DeleteSession,
    /// Stop another session.
    StopSession,
    /// Interrupt another session.
    InterruptSession,
    /// Type into another session (FR-016 "Confirm each send").
    SendInput,
}

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
        /// The current holder's identity — instance included, so a client refused by a
        /// connection of its *own* that the daemon has not yet reaped can say so (BUG-022).
        holder: ClientIdentity,
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
    /// Which AI CLI this session runs (feature 026, FR-016), carried outward the way
    /// `worktree_dir` and `title` already are so the client can label rows without a second
    /// request.
    ///
    /// **Not** the way `mode` does — `mode` does not travel at all, and citing it as the precedent
    /// sends a reader looking for a field that is not there.
    pub provider: AiCli,
    /// The input serial the service expects next for this session — its
    /// [`InputReceiver`](crate::input::InputReceiver) high-water mark (FR-028a, BUG-006).
    ///
    /// Authoritative, and runtime-only: the receiver lives with the live session, so a client that
    /// did not start this session cannot infer it. A client MUST adopt this value when it has no
    /// counter of its own for the session, or its first keystroke is stamped `0`, classified
    /// [`Stale`](crate::input::InputOutcome::Stale), and silently dropped — along with every one
    /// after it. Sessions the service is not hosting have no receiver and report `0`.
    pub input_serial: u64,
    /// Which of this session's Regular-terminal shell instances the service currently hosts
    /// (`012` FR-008, BUG-003).
    ///
    /// Runtime-only and overlaid from the live registry, exactly like `activity` and
    /// `input_serial`: `LiveSession.procs` is keyed by [`SessionProcess::Shell`], so the service
    /// already knows, and the durable catalog cannot. The **client** allocates instance ids and
    /// owns the set of instances; this reports which of them are alive, and never introduces one.
    ///
    /// Absence is meaningful in one direction only. An id here means that instance's process
    /// exists; an id missing means the service is not hosting it, which is a spawn still in flight
    /// as well as a shell that exited — so a client MUST NOT read a first absence as death. It is
    /// still the only signal that can distinguish an exited shell from a quiet one, which no
    /// amount of watching for frames can do.
    pub live_shells: Vec<ShellInstanceId>,
    /// How many attention events the session has had (feature 039, W1.3): changes into
    /// [`ActivitySignal::AwaitingInput`] while no window had the session in view.
    ///
    /// Durable, unlike the three fields above: the service stores it with the session, so it
    /// never decreases while the catalog is intact.
    pub attention_seq: u64,
    /// Whether the session has had an attention event since a window last reported it in view
    /// (feature 039, W2). Durable, as `attention_seq` is, and the same for every window (FR-024).
    pub unread: bool,
}

/// What a window reports having in view: the fields of [`ClientMsg::WindowView`], as the client
/// remembers its last report and the service keeps one per connection (feature 039, W1.1, W1.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WindowView {
    /// The window has keyboard focus.
    pub focused: bool,
    /// The session the window has in view. `Some` only with `focused: true`.
    pub in_view: Option<SessionId>,
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
    /// Every session directory whose environment-include resolution by the service failed, sorted
    /// by directory (011 FR-022, BUG-454). Runtime-only: the service projects it from its
    /// per-directory cache at send time and never writes it to the catalog file, so it holds the
    /// script's captured output only in memory (FR-013).
    ///
    /// Left out of the encoding when empty, and read back empty when absent. That keeps a frame
    /// with nothing to report as it was; it does not make an older peer compatible, which the
    /// handshake refuses on `PROTOCOL_VERSION` and `SCHEMA_HASH` anyway.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub env_include_failures: Vec<EnvIncludeFailure>,
}

/// One session directory whose environment-include resolution failed (011 FR-022, BUG-454): the
/// directory the script was sourced in, and the outcome of that attempt — never `Success` or
/// `Disabled`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EnvIncludeFailure {
    /// The directory the script was sourced in.
    pub dir: PathBuf,
    /// The failed attempt's outcome: its category and captured output.
    pub outcome: crate::env_include::EnvIncludeOutcome,
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
    /// Whether this application created this worktree (feature 029, FR-001).
    ///
    /// The provenance record, projected. Sent per worktree rather than as a per-project set for
    /// the reason `display_name` is: the client renders rows, and a row that has to reach back
    /// into a second collection to learn what it is can disagree with the one it came from.
    ///
    /// **Not the classification.** Whether a worktree is *hidden* additionally depends on where it
    /// lives and on whether the project's records could be read at all — questions the client
    /// answers with [`micold_core::worktree::classify_owner`] using this as one input.
    ///
    /// `#[serde(default)]` so a snapshot written before the field existed decodes as `false`
    /// rather than failing. Note the postcard encoding cannot honour a default for a missing
    /// field, which costs nothing here: the handshake refuses a mismatched client/daemon pair
    /// before any snapshot is sent.
    #[serde(default)]
    pub user_created: bool,
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
    /// Which AI CLI a new session runs when nothing is chosen for it (feature 026, FR-003).
    ///
    /// Service-owned, alongside the scrollback limit rather than beside `theme`. `settings.json`
    /// has two writers and the split is by field: the daemon's `set_scrollback` and
    /// `set_env_include` each persist their whole `Settings` struct from the copy loaded at boot,
    /// so a client-written field is reverted by the next unrelated settings change. That is
    /// already true of `theme`; this preference must not inherit it.
    pub default_ai_cli: AiCli,
    /// Whether a Pi session is started with this application's activity component loaded
    /// (feature 029, FR-012e). Service-owned like the default CLI, because the spawn reads it.
    pub pi_activity_component: bool,
    /// Whether new sessions are bound to the service's tool server (feature 034, FR-004).
    /// Service-owned for the same reason: the spawn reads it, and running sessions keep theirs.
    pub tool_server_enabled: bool,
    /// Whether agents may read and type into other sessions of their project (feature 034,
    /// FR-016). Service-owned because the tool server reads it on every request.
    pub cross_session_access: CrossSessionAccess,
    /// Whether pull request status is read from GitHub and shown on worktree rows (feature 040,
    /// FR-029, FR-030). Service-owned so that every window follows one switch.
    pub pr_status_enabled: bool,
    /// Whether a session that comes to await input while not in view raises a desktop
    /// notification (feature 039, FR-026, FR-027). Service-owned so that every window follows one
    /// switch, and because the service is what grants a claim.
    pub desktop_notifications: bool,
    /// Which kinds of event notify while `desktop_notifications` is on (feature 613, W5.3).
    pub notification_kinds: NotificationKinds,
    /// How long, in seconds, a turn must last to be **Long task finished** (feature 613, W5.6).
    pub long_task_threshold_secs: u64,
    /// The Changes view's diff layout (feature 482, R12). Service-owned so every window and the
    /// next start keep the last choice.
    pub diff_layout: DiffLayout,
}

/// One change to an entry's review comments (feature 482, contracts/review-wire.md W1–W4).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReviewEditOp {
    /// Add a pending comment on lines `start..=end` of `side` of `path` (W1).
    Add {
        /// The file, relative to the entry root, `/`-separated.
        path: String,
        /// Which version the lines are numbered in.
        side: Side,
        /// The first line (1-based).
        start: u32,
        /// The last line (inclusive).
        end: u32,
        /// Those lines' text, one entry per line.
        quote: Vec<String>,
        /// What the user wrote.
        text: String,
    },
    /// Replace a pending comment's text.
    SetText {
        /// The comment.
        id: CommentId,
        /// Its new text.
        text: String,
    },
    /// Delete a pending comment.
    Delete {
        /// The comment.
        id: CommentId,
    },
    /// Remove every sent comment of the entry (W4).
    ClearSent,
    /// Remove every pending comment not inside an open send (W4).
    DiscardPending,
}

/// One question of [`ClientMsg::MergedBranchCheck`]: does local `branch` hold anything beyond
/// `head`, the last commit of its merged pull request (feature 040, FR-015)? A branch name and a
/// commit id, never a title or an address.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MergedBranchQuery {
    /// The local branch, as the worktree listing shows it.
    pub branch: String,
    /// The pull request's last commit: 40 or 64 hexadecimal characters. Anything else is
    /// answered [`BranchContainment::Unknown`] without git being run.
    pub head: String,
}

/// Whether a local branch holds commits beyond its merged pull request (feature 040, FR-015,
/// FR-017). A removal is suggested for `Contained` only.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BranchContainment {
    /// The branch's tip is the pull request's last commit or an ancestor of it.
    Contained,
    /// The branch has commits the pull request did not merge.
    Beyond,
    /// The repository cannot tell: the branch does not exist locally, the pull request's last
    /// commit was never fetched, or git failed.
    Unknown,
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
    /// What each target of a [`ClientMsg::AttachApply`] came to (feature 582).
    AttachApplied {
        /// One result per requested target, in request order.
        results: Vec<AttachResult>,
    },
    /// The repository's own remotes (feature 034, FR-002).
    RemoteList {
        /// Every remote with a URL, in config order, URLs as written.
        remotes: Vec<crate::git::GitRemote>,
    },
    /// The answers to [`ClientMsg::MergedBranchCheck`] (feature 040, FR-015).
    MergedBranchCheck {
        /// One answer per query, in the order of the queries.
        answers: Vec<BranchContainment>,
    },
    /// An entry's review comments were delivered to `session` (feature 482, W8).
    ReviewSent {
        /// The session that received the prompt.
        session: SessionId,
        /// Whether the send started that session (no session was running).
        started: bool,
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
    /// A run group was recorded (feature 483, W1); its runs progress on their own.
    RunGroupCreated {
        /// The new group.
        group: GroupId,
    },
    /// A group's run was picked and integrated into the base branch (feature 483, W3).
    RunPicked {
        /// The group.
        group: GroupId,
        /// The picked run's number.
        run: u8,
        /// What the pick did to the base branch.
        integration: Integration,
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

#[cfg(test)]
mod attention_wire_tests {
    //! Feature 039, contract W1 (versions 21 and 23): the view report, the attention sequence, the
    //! claim and the grant travel.

    use super::*;

    fn session() -> SessionId {
        SessionId::from_uuid(uuid::Uuid::from_u128(0x039))
    }

    fn through_json<T: Serialize + serde::de::DeserializeOwned>(value: &T) -> T {
        let json = serde_json::to_string(value).expect("the message encodes");
        serde_json::from_str(&json).expect("the message decodes")
    }

    #[test]
    fn a_view_report_encodes_and_decodes() {
        for report in [
            ClientMsg::WindowView {
                focused: true,
                in_view: Some(session()),
            },
            ClientMsg::WindowView {
                focused: true,
                in_view: None,
            },
            ClientMsg::WindowView {
                focused: false,
                in_view: None,
            },
        ] {
            assert_eq!(
                through_json(&report),
                report,
                "a view report is read as it was written"
            );
        }
    }

    #[test]
    fn a_claim_and_its_grant_encode_and_decode() {
        // U13. A sequence no default produces, so a field that never encoded cannot read back as it.
        const SEQ: u64 = 5;
        let claim = ClientMsg::AttentionClaim {
            session: session(),
            seq: SEQ,
        };
        let grant = DaemonMsg::AttentionGranted {
            session: session(),
            seq: SEQ,
            kind: NotificationKind::LongTaskFinished,
        };
        assert_eq!(
            through_json(&claim),
            claim,
            "a claim is read as it was written"
        );
        assert_eq!(
            through_json(&grant),
            grant,
            "a grant is read as it was written"
        );
    }

    #[test]
    fn a_reveal_and_its_forward_encode_and_decode() {
        // U15 (W3). A path and a session no default produces.
        let asked = ClientMsg::SessionReveal {
            project: PathBuf::from("/repo"),
            session: session(),
            activation: None,
        };
        let forwarded = DaemonMsg::RevealSession {
            project: PathBuf::from("/repo"),
            session: session(),
            activation: None,
        };
        assert_eq!(
            through_json(&asked),
            asked,
            "a reveal is read as it was written"
        );
        assert_eq!(
            through_json(&forwarded),
            forwarded,
            "a forwarded reveal is read as it was written"
        );
    }

    #[test]
    fn the_activation_token_of_a_reveal_encodes_and_decodes_on_both_messages() {
        // U16 (W3.4, version 26). A token no default produces, with and without one.
        const TOKEN: &str = "wayland-token-7f3a";
        for activation in [Some(TOKEN.to_string()), None] {
            let asked = ClientMsg::SessionReveal {
                project: PathBuf::from("/repo"),
                session: session(),
                activation: activation.clone(),
            };
            let forwarded = DaemonMsg::RevealSession {
                project: PathBuf::from("/repo"),
                session: session(),
                activation,
            };
            assert_eq!(through_json(&asked), asked, "a reveal keeps its token");
            assert_eq!(
                through_json(&forwarded),
                forwarded,
                "a forwarded reveal keeps its token"
            );
        }
    }

    #[test]
    fn a_session_summary_carries_its_attention_sequence() {
        // A count no default produces, so a field that never encoded cannot read back as it.
        const COUNTED: u64 = 7;
        let summary = SessionSummary {
            id: session(),
            worktree_dir: None,
            title: SessionLabel::Pending,
            lifecycle: WireLifecycle::Running,
            activity: ActivitySignal::AwaitingInput,
            provider: AiCli::ClaudeCode,
            input_serial: 0,
            live_shells: Vec::new(),
            attention_seq: COUNTED,
            unread: false,
        };

        assert_eq!(
            through_json(&summary).attention_seq,
            COUNTED,
            "the count of attention events reaches the window"
        );
    }

    #[test]
    fn a_session_summary_carries_unread() {
        let summary = SessionSummary {
            id: session(),
            worktree_dir: None,
            title: SessionLabel::Pending,
            lifecycle: WireLifecycle::Running,
            activity: ActivitySignal::AwaitingInput,
            provider: AiCli::ClaudeCode,
            input_serial: 0,
            live_shells: Vec::new(),
            attention_seq: 1,
            // `true` is the value no default produces, so a field that never encoded cannot read
            // back as it.
            unread: true,
        };

        assert!(
            through_json(&summary).unread,
            "unread state reaches the window"
        );
    }
}

#[cfg(test)]
mod desktop_notifications_wire_tests {
    //! Feature 039, contract W4: the **Desktop notifications** setting travels both ways.

    use super::*;

    fn through_json<T: Serialize + serde::de::DeserializeOwned>(value: &T) -> T {
        let json = serde_json::to_string(value).expect("the message encodes");
        serde_json::from_str(&json).expect("the message decodes")
    }

    fn settings(desktop_notifications: bool) -> DaemonSettings {
        DaemonSettings {
            scrollback_lines: 10_000,
            env_include_enabled: true,
            env_include_script_path: String::new(),
            env_include_timeout_secs: 10,
            default_ai_cli: AiCli::ClaudeCode,
            pi_activity_component: true,
            tool_server_enabled: true,
            cross_session_access: CrossSessionAccess::Auto,
            pr_status_enabled: false,
            desktop_notifications,
            notification_kinds: NotificationKinds::default(),
            long_task_threshold_secs: 60,
            diff_layout: Default::default(),
        }
    }

    /// U17: the service's settings carry the switch, on and off.
    #[test]
    fn the_service_settings_carry_desktop_notifications() {
        for on in [true, false] {
            let pushed = DaemonMsg::SettingsChanged {
                settings: settings(on),
            };
            match through_json(&pushed) {
                DaemonMsg::SettingsChanged { settings } => assert_eq!(
                    settings.desktop_notifications, on,
                    "the switch reaches the window as the service holds it"
                ),
                other => panic!("expected SettingsChanged, got {other:?}"),
            }
        }
    }

    /// U17: `SettingsSet` carries the choice, and `None` for "leave it".
    #[test]
    fn a_settings_change_carries_desktop_notifications_or_leaves_it() {
        for chosen in [Some(false), Some(true), None] {
            let asked = ClientMsg::SettingsSet {
                req: 1,
                scrollback_lines: None,
                env_include_enabled: None,
                env_include_script_path: None,
                env_include_timeout_secs: None,
                default_ai_cli: None,
                pi_activity_component: None,
                tool_server_enabled: None,
                cross_session_access: None,
                pr_status_enabled: None,
                desktop_notifications: chosen,
                notification_kinds: None,
                long_task_threshold_secs: None,
                diff_layout: None,
            };
            match through_json(&asked) {
                ClientMsg::SettingsSet {
                    desktop_notifications,
                    ..
                } => assert_eq!(
                    desktop_notifications, chosen,
                    "the service reads the choice as the window sent it"
                ),
                other => panic!("expected SettingsSet, got {other:?}"),
            }
        }
    }
}
