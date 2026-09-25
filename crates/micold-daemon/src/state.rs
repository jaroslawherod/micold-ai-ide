//! Shared daemon state: the client registry, per-project attachments, and the push projection that
//! keeps every connected client current (data-model §Attachment, FR-011, FR-023, task T022).
//!
//! One `std::sync::Mutex` guards the mutable state; it is **never** held across an `.await` — every
//! method locks, mutates, and (for pushes) hands `DaemonMsg`s to per-client unbounded channels whose
//! writer tasks own the socket sink. That keeps a slow or stuck client from blocking the state lock.

use std::cell::RefCell;
use std::collections::{BTreeSet, HashMap};
use std::ffi::OsString;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Instant;

use micold_core::git::GitCli;
use micold_core::input::{InputOutcome, InputReceiver};
use micold_core::protocol::codec::Frame;
use micold_core::protocol::messages::{
    ActivitySignal, CatalogSnapshot, ClientIdentity, ClientInstance, DaemonMsg, DaemonSettings,
    RefusalReason, SessionProcess, SessionSummary, WireLifecycle, WorktreeSnapshot, WorktreeStatus,
};
use micold_core::provider::ActivitySource;
use micold_core::session::{
    AiCli, Session, SessionId, SessionLabel, SessionLifecycle, SessionLocation, ShellInstanceId,
    TerminalMode, RESTART_STABLE_AFTER,
};
use micold_core::terminal::{LaunchMode, LaunchSpec};
use micold_core::worktree::{self, Worktree};
use tokio::sync::mpsc;

use crate::activity::{Activity, ActivityEvent};
use crate::catalog::Catalog;
use crate::framer::Framer;
use crate::idle::Presence;
use crate::supervision::{ExitOutcome, SupervisionAction};
use crate::supervisor::PtySession;
use crate::terminal::TerminalColors;

/// A per-connection client identity (ephemeral; never persisted).
pub type ClientId = u64;

/// What one [`DaemonState::drain_signals`] pass observed on the live sessions.
///
/// Two things, because the supervisor does two different things with them: `changed` decides
/// whether to push a `CatalogChanged`, and `names` is durable state that has to be written — on a
/// blocking thread, which is why it comes back here rather than being written where it was seen
/// (feature 029, research R2).
#[derive(Debug, Default)]
pub struct DrainedSignals {
    /// A session's projected summary changed, so the supervisor tick pushes one `CatalogChanged`.
    pub changed: bool,
    /// The sessions whose stripped OSC-0 title differed from the previous drain, with the new
    /// title. Debounced against `LiveSession::last_title`, so each change appears exactly once
    /// however many ticks pass with the title unchanged.
    ///
    /// The title is the **stripped** one — the leading status glyph the agent puts in front of it
    /// is already gone, and the braille-spinner edge was taken separately as activity evidence.
    /// So this is precisely the text the row displays, which is what keeps the record and the
    /// screen from being able to diverge.
    pub names: Vec<(SessionId, String)>,
}

/// The daemon's shared, mutable runtime state.
pub struct DaemonState {
    inner: Mutex<Inner>,
    next_id: AtomicU64,
    /// The connection count the idle rule reads (feature 028, data-model G1).
    ///
    /// Its own mutex rather than a field on `Inner`, because it is read on a timer by the idle
    /// tick and taken on every connect and disconnect, while `Inner` is the lock a multi-minute
    /// git operation can be holding. Sharing one would make "is anyone connected?" wait on
    /// whatever the session catalogue is doing.
    ///
    /// A mutex rather than the two atomics the retired `Lifecycle` used, because the count and its
    /// armed deadline have to move together — see [`Presence`].
    presence: Mutex<Presence>,
    /// The diagnostics handle (log location, runtime level reload, recent-errors ring), set once at
    /// startup by `server::run`. Absent for tests and the ephemeral catalog, which don't init logging.
    diagnostics: std::sync::OnceLock<crate::logging::Logging>,
    /// The loopback hook receiver (US2, T045/T046), set once at startup by `server::run`. Absent for
    /// tests and when binding fails — activity then degrades to `Unknown` (H1), never to a wrong
    /// signal. When present, `start_session` writes each AI-CLI session a `--settings` file pointing
    /// `claude`'s lifecycle hooks at it.
    hooks: std::sync::OnceLock<crate::hooks::HookReceiver>,
    /// The shared secret this daemon requires of a client, if it was started with one (feature
    /// 027, research R1).
    ///
    /// Set once at startup from the file named by `MICOLD_TOKEN_PATH`, which the container runtime
    /// bind-mounts read-only. **Absent for the host-process placement**, whose `0700`-guarded
    /// socket already proves the caller is the user — requiring a token there would break every
    /// existing installation and protect nothing that is not already protected.
    auth_token: std::sync::OnceLock<micold_core::protocol::auth::Token>,
    /// The light/dark scheme a client last reported (`ClientMsg::TerminalColorScheme`), which every
    /// session's terminal answers `OSC 10/11/12` from (`006` FR-003a, BUG-007).
    terminal_colors: TerminalColors,
}

struct Inner {
    catalog: Catalog,
    clients: HashMap<ClientId, ClientHandle>,
    /// At most one attachment per project (data-model P2/T1).
    attachments: HashMap<PathBuf, Attachment>,
    /// The live PTY-backed sessions the daemon is hosting, keyed by id (data-model §Session). Each
    /// carries its own [`InputReceiver`] so the append-only input contract (G2) is enforced per
    /// session, independent of which connection — or how many reconnects — drove it.
    sessions: HashMap<SessionId, LiveSession>,
    /// Shell instances already announced as dead (`012` FR-008, BUG-003). A shell that exits is not
    /// removed from `procs` — its PTY holds the final screen — so nothing else marks the moment it
    /// stopped being live. The supervision tick broadcasts once per instance and records it here;
    /// without the marker a dead shell would re-broadcast on every tick for as long as it is open.
    /// Cleared for an instance when it is closed or restarted, which is what makes a restart
    /// announceable again.
    announced_dead_shells: std::collections::HashSet<(SessionId, ShellInstanceId)>,
    /// The worktrees discovered live from git + the filesystem, per project (FR-018, T053). Git is
    /// the single source of truth for worktrees; this is a cache refreshed at well-defined points
    /// (client attach, and after each worktree mutation) so the catalog snapshot can surface them
    /// **without** running a git subprocess under the state lock. Absent for a project not yet
    /// attached/refreshed — the snapshot then shows no worktrees for it until the first refresh.
    worktrees: HashMap<PathBuf, Vec<Worktree>>,
    /// Why a session's last start attempt failed, when the reason is worth telling the user
    /// (feature 026, FR-010 — today, a CLI that is not installed).
    ///
    /// Runtime-only and per session, like `sessions` itself: it is a fact about this machine right
    /// now, not about the session, and a `PATH` fixed between runs must not leave a stale
    /// complaint behind. Cleared the moment the session starts.
    ///
    /// It lives here rather than on the domain `SessionLifecycle` because that enum's `Failed`
    /// means one specific thing — "auto-restart gave up after repeated quick failures" — and a
    /// start that never got as far as a first process is not that. `010` BUG-017 gave that variant
    /// a `reason` of its own, so the reason this is separate is no longer "there is nowhere to put
    /// the text"; it is that the two failures are different failures. This one spends no restart
    /// budget (`attempts: 0`) and is cleared by a start rather than by a survival.
    start_failures: HashMap<SessionId, String>,
    /// The final activity signal of a session whose process has exited, kept until it runs again
    /// (`010` BUG-018).
    ///
    /// `Ended` is the FSM's absorbing state, but the FSM lives on the [`LiveSession`], and
    /// supervision *drops* that entry the moment a session stops or gives up — so the one signal
    /// that describes the session's whole remaining life was the one guaranteed not to survive to
    /// be shown. A stopped row fell back to `Unknown` and drew nothing, exactly like a live session
    /// whose hooks never fired.
    ///
    /// Runtime-only and per session, like `start_failures` beside it: activity resets to `Unknown`
    /// on a daemon restart (H3/A4), and this holds no more than the FSM already produced.
    ///
    /// It is not derived from the lifecycle instead, which would need no field at all, because a
    /// clean exit leaves the session `Idle` — and so does a session that was created and never
    /// started. Deriving `Ended` from `Idle` would put a "finished" ring on a session that has
    /// never run, which is precisely the invented state H1 forbids. `Failed` is unambiguous, but
    /// covering only the crash-loop half would leave S3's actual case (a session that exited
    /// cleanly) still reading nothing.
    ///
    /// Cleared when the session runs again ([`Self::register_session`]) and when it leaves the
    /// catalog for good (`remove_live_by_ids`).
    ended: HashMap<SessionId, ActivitySignal>,
    /// Per-directory cache of the environment-include-resolved variables (feature 011), already
    /// merged with the hardcoded `TERM` pair — ready to hand straight to a spawn site's `env`
    /// (FR-012b, BUG-003). Keyed by the directory the sourcing subprocess ran in, mirroring
    /// `micold-client`'s pre-010 `env_include_cache`: a version-manager hook (mise, asdf, nvm,
    /// pyenv, rbenv, …) computes its `PATH` contribution from the sourcing shell's own cwd, so one
    /// directory-agnostic snapshot can never be correct for more than one project. Never persisted.
    /// Cleared entirely on a `SettingsSet` that changes any env-include field; a single entry is
    /// removed on that directory's `WorktreeDelete`.
    env_include_cache: HashMap<PathBuf, Vec<(String, String)>>,
    /// One mutual-exclusion gate per project for mutating worktree work (BUG-009, T120). Worktree
    /// creates run as spawned tasks now — they must not park the connection loop that dispatched
    /// them (FR-026a) — so the serialization the old inline `.await` provided as a side effect is
    /// stated explicitly here instead of being lost. Keyed by project because two projects share no
    /// git state to race over. Entries are cheap (`Arc<Mutex<()>>`) and bounded by project count.
    worktree_gates: HashMap<PathBuf, Arc<tokio::sync::Mutex<()>>>,
    /// Sessions whose start is in flight, each holding the input typed while it runs, in arrival
    /// order (BUG-009, T125). Present only for the duration of a start — see
    /// [`DaemonState::session_input`] for why the input is held rather than dropped, and
    /// [`DaemonState::finish_start`] for how the buffer is closed without a gap.
    starting: HashMap<SessionId, Vec<(u64, Vec<u8>)>>,
    /// One mutual-exclusion gate per session for starts (T125), for the same reason
    /// [`Self::worktree_gates`] exists: spawning the work removed the serialization the connection
    /// loop provided incidentally, and two concurrent starts would spawn two processes.
    session_gates: HashMap<SessionId, Arc<tokio::sync::Mutex<()>>>,
    /// The `(cols, rows)` a client last asked for per session — the size of its terminal pane —
    /// kept **whether or not the session is live** (BUG-003, `006-real-terminal-emulator`
    /// FR-014a). A `SessionResize` used to reach live PTYs only, so a size reported for a session
    /// whose process did not exist yet was lost and the spawn came up at the 100×30 seed; every
    /// spawn site now seeds from here instead. Survives stop/start and crash-respawn (the pane is
    /// still that size); dropped when the session is archived. Never persisted — it describes a
    /// client's window, not the session.
    sizes: HashMap<SessionId, (u16, u16)>,
}

/// One live process of a session: its PTY and its framer. The [`PtySession`] is behind an `Arc` so
/// a caller can clone it and write to the PTY *after* dropping the state lock — PTY writes must never
/// block the shared lock (module invariant). The framer is per-process and session-lived so its
/// `scrolled_off` watermark (and every line's absolute `LineId`) is stable across reattach, and
/// scrollback-by-range resolves against the same eviction state the stream produced.
struct Proc {
    pty: Arc<PtySession>,
    framer: Arc<Mutex<Framer>>,
}

/// A running session's processes (feature 011): a `Primary` (its AI CLI or mode-selected primary
/// shell) plus any additional Regular-terminal shell instances, of which exactly one is *attached*
/// (streamed + driven) at a time. Input and grid frames stay `SessionId`-addressed and route to the
/// attached process.
struct LiveSession {
    procs: HashMap<SessionProcess, Proc>,
    attached: SessionProcess,
    /// The input log is **per session, not per process**: the client's `SessionInputStamper` mints
    /// one monotonic serial stream per `SessionId`, so the receiver must live at this level too —
    /// otherwise switching the attached process (a fresh per-process receiver) would classify the
    /// next legitimate serial as a false `Lost` (G2, protocol.md §7).
    input: InputReceiver,
    /// The derived activity FSM (US2, T046). Per session (not per process) and **not persisted** —
    /// it resets to `Unknown` on daemon restart (H3/A4). Fed by claude-CLI lifecycle hooks (the
    /// loopback receiver) and by braille-spinner title evidence (`SpinnerObserved`, Working-only).
    activity: Activity,
    /// The most recent OSC-0 title observed on the AI CLI's `Primary` process (glyph-stripped,
    /// the CLI's own startup title excluded), used to project a live session title and to
    /// debounce title-change pushes (T047). Persisted separately, as the catalog label (feature
    /// 029); re-emitted by `claude` on resume.
    last_title: Option<String>,
    /// Whether the conversation behind this session may have gained a name since it was last
    /// looked for. Set when the session starts and whenever its activity moves; cleared by
    /// [`DaemonState::recover_live_session_names`], which does the looking. A CLI whose terminal
    /// title names nothing until the user names the conversation — `pi` — would otherwise leave a
    /// running row on the placeholder while its store already holds the first message (feature
    /// 029, FR-011). A flag rather than a timer: the read happens because something happened.
    name_stale: bool,
    /// The tail of this session's own event log, for a provider whose activity source is
    /// `EventLog` (feature 026, T064). `None` for a `Hooks` provider — and `None` for every
    /// session this application merely *discovered* rather than started, since a tail is only ever
    /// opened here, for a session in the live registry (FR-018, SC-006).
    ///
    /// Dropping the `LiveSession` drops this, which unregisters the watch. That is the whole
    /// teardown: there is no timer to cancel, because there is no timer.
    event_log: Option<crate::event_log::EventLogTail>,
    /// When supervision last respawned this session's primary after a crash, or `None` if it has
    /// not. A `Restarting` session counts as recovered only once its respawn has stayed up for
    /// [`RESTART_STABLE_AFTER`] from this reading (`005` BUG-004, FR-022a).
    respawned_at: Option<micold_core::clock::Uptime>,
}

/// Build a fresh [`Proc`] around a spawned PTY, with a session-lived framer.
/// The variable Pi's activity component reads to find its log (feature 029, contracts/pi-cli.md).
const PI_ACTIVITY_LOG_VAR: &str = "MICOLD_PI_ACTIVITY_LOG";

/// The component itself, compiled into this binary so it travels with the session service
/// wherever that runs — host or sandbox — and adds nothing an image has to carry (FR-012a,
/// FR-017a). Its full extent is that one file (FR-012b).
const PI_ACTIVITY_COMPONENT: &str = include_str!("../assets/pi-activity.ts");

/// Write the component into the service's own data directory and return its path, for `pi -e`.
///
/// Rewritten on every Pi launch, and only when the bytes differ, so a daemon upgrade replaces what
/// an older one left behind and an unchanged file is not touched. It never goes near Pi's own
/// extension directories, so a `pi` run outside this application does not load it (FR-012a).
fn materialise_pi_activity_component() -> io::Result<PathBuf> {
    let dir = directories::ProjectDirs::from("", "", "micold-ai-ide")
        .map(|dirs| dirs.data_dir().join("pi"))
        .unwrap_or_else(|| std::env::temp_dir().join("micold-daemon-pi"));
    std::fs::create_dir_all(&dir)?;
    let path = dir.join("pi-activity.ts");
    if std::fs::read(&path).ok().as_deref() != Some(PI_ACTIVITY_COMPONENT.as_bytes()) {
        std::fs::write(&path, PI_ACTIVITY_COMPONENT)?;
    }
    Ok(path)
}

/// What a start refused for a missing AI CLI tells the user (FR-010), as advice that works where
/// sessions run and for what is being started (`029` quickstart §D, finding 1).
///
/// `cli` is the provider's `display_name()`, not its `command()`: this is a sentence, and "copilot
/// isn't installed" reads as a shell error rather than as something to go and fix. `image` is the
/// service's `MICOLD_IMAGE_REFERENCE`, empty on the host. In a container, installing on this
/// computer changes nothing, so the image is named instead, as the settings notices name it. A
/// resume continues a conversation only the same CLI holds, so it is not offered another one.
fn missing_cli_reason(cli: &str, image: &str, launch: LaunchMode) -> String {
    match (image.is_empty(), launch) {
        (true, LaunchMode::Fresh) => {
            format!("{cli} isn't installed. Install it, or start this session on another AI CLI.")
        }
        (true, LaunchMode::Resume) => format!(
            "{cli} isn't installed, and this conversation can only continue in it. Install it, \
             then restart this session."
        ),
        (false, LaunchMode::Fresh) => format!(
            "{cli} isn't in {image}, where sessions run. Choose an image that provides it, or \
             start this session on another AI CLI."
        ),
        (false, LaunchMode::Resume) => format!(
            "{cli} isn't in {image}, where sessions run, and this conversation can only continue \
             in it. Choose an image that provides it, then restart this session."
        ),
    }
}

fn new_proc(pty: Arc<PtySession>, id: SessionId) -> Proc {
    Proc {
        pty,
        framer: Arc::new(Mutex::new(Framer::new(id))),
    }
}

/// Project the core [`worktree::WorktreeStatus`] (git-discovery facts) onto the wire enum the client
/// renders. `Invalid` (an on-disk dir git does not know) maps to `Prunable` — the actionable state
/// telling the user git would drop it.
fn wire_worktree_status(status: worktree::WorktreeStatus) -> WorktreeStatus {
    match status {
        worktree::WorktreeStatus::Valid => WorktreeStatus::Clean,
        worktree::WorktreeStatus::Missing => WorktreeStatus::Missing,
        worktree::WorktreeStatus::Invalid => WorktreeStatus::Prunable,
    }
}

/// What `start_session` needs to spawn a session, resolved from the catalog under the lock and then
/// used to spawn *outside* it.
struct SpawnPlan {
    cwd: std::path::PathBuf,
    mode: TerminalMode,
    /// Which AI CLI this session runs (feature 026, FR-007) — read from the record, never chosen
    /// here.
    provider: AiCli,
    /// Whether the record is one the daemon has already judged resumable — a conversation was
    /// found for it at startup (FR-008). Gates the "the CLI no longer has this conversation" check
    /// in [`DaemonState::start_session`], which must not fire for a session that never had one.
    resumable: bool,
    scrollback: usize,
}

struct ClientHandle {
    /// The client's outgoing frame channel. Carries **both** control messages (wrapped
    /// `Frame::Control`) and pushed grid frames (`Frame::Grid`) so the writer task delivers them in
    /// one ordered stream — a grid delta never overtakes the control message that announced it.
    tx: mpsc::UnboundedSender<Frame<DaemonMsg>>,
    /// Who this connection's client is: its build string *and* its window instance. The instance
    /// is what the daemon reports back to other clients so a window can recognise itself
    /// (`010` BUG-022); the daemon itself never compares instances — exclusivity is per
    /// connection, and always was.
    identity: ClientIdentity,
    /// Which session (if any) this client is viewing per project (FR-016).
    viewed: HashMap<PathBuf, Option<SessionId>>,
}

struct Attachment {
    client: ClientId,
    since: Instant,
}

/// A session [`DaemonState::record_recovered_names`] reads a name for: read under the lock, used off
/// it.
struct RecoveryCandidate {
    id: SessionId,
    /// Where the session runs, which is where its CLI keeps its records.
    cwd: PathBuf,
    /// The CLI that owns it — the only store its name may come from (contract C16).
    provider: AiCli,
    /// It has no label of any kind yet, so a first-turn label may be derived for it (feature 032).
    unlabelled: bool,
}

impl RecoveryCandidate {
    fn of(session: &Session, project: &Path) -> Self {
        Self {
            id: session.id,
            cwd: session.location.cwd(project),
            provider: session.provider,
            unlabelled: matches!(session.label, SessionLabel::Pending),
        }
    }
}

/// What recovery found for a candidate: its CLI's title, or with none, its first-turn label.
enum Recovered {
    Title(String),
    Label(String),
}

impl DaemonState {
    /// Build the shared state around an adopted [`Catalog`].
    pub fn new(catalog: Catalog) -> Self {
        Self {
            inner: Mutex::new(Inner {
                catalog,
                clients: HashMap::new(),
                attachments: HashMap::new(),
                sessions: HashMap::new(),
                announced_dead_shells: std::collections::HashSet::new(),
                worktrees: HashMap::new(),
                start_failures: HashMap::new(),
                ended: HashMap::new(),
                env_include_cache: HashMap::new(),
                worktree_gates: HashMap::new(),
                starting: HashMap::new(),
                session_gates: HashMap::new(),
                sizes: HashMap::new(),
            }),
            next_id: AtomicU64::new(1),
            // Armed from construction: a daemon spawned by a client that dies before handshaking
            // must not outlive the machine (G1).
            presence: Mutex::new(Presence::new(micold_core::clock::now())),
            diagnostics: std::sync::OnceLock::new(),
            hooks: std::sync::OnceLock::new(),
            auth_token: std::sync::OnceLock::new(),
            terminal_colors: TerminalColors::default(),
        }
    }

    /// The scheme sessions answer colour queries for (`006` FR-003a, BUG-007). Clones share it.
    pub fn terminal_colors(&self) -> &TerminalColors {
        &self.terminal_colors
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, Inner> {
        self.inner.lock().expect("daemon state mutex poisoned")
    }

    /// A snapshot of who is connected, for the idle rule and for tests (data-model G1).
    ///
    /// By value, and `Presence` is `Copy`: the rule is a pure function of a snapshot, so handing
    /// out a reference into the lock would let a caller hold it across an evaluation for no gain.
    pub fn presence(&self) -> Presence {
        *self.presence.lock().expect("presence mutex poisoned")
    }

    /// Record the diagnostics handle at startup so the `LogLocation`/`RecentErrors`/`SetLogLevel`
    /// RPCs can serve it (FR-043–046). A no-op if already set.
    pub fn set_diagnostics(&self, logging: crate::logging::Logging) {
        let _ = self.diagnostics.set(logging);
    }

    /// The diagnostics handle, if logging was initialised (absent in tests / ephemeral catalog).
    pub fn diagnostics(&self) -> Option<&crate::logging::Logging> {
        self.diagnostics.get()
    }

    /// Record the loopback hook receiver at startup so AI-CLI spawns can be pointed at it (US2,
    /// T045/T046). A no-op if already set.
    pub fn set_hooks(&self, receiver: crate::hooks::HookReceiver) {
        let _ = self.hooks.set(receiver);
    }

    /// The launch arguments — and, for Pi, the environment — that wire a session's activity
    /// reporting, keyed on the provider's [`ActivitySource`]. Returns no arguments when there is
    /// nothing to wire or preparing it failed; the caller then spawns without it and activity stays
    /// `Unknown` (H1), never wrong. Blocking (small file writes); the AI-CLI spawn path is already
    /// off the async runtime.
    ///
    /// - `Hooks` → `--settings <file>`, if the hook receiver is running (tests or a bind failure
    ///   leave it absent). The file is `claude`'s mechanism — a port and a per-session bearer token
    ///   in a `--settings` JSON — and `copilot` has no flag to hand it to (feature 026, T016a).
    /// - `Extension { log }` → `-e <component>` and `MICOLD_PI_ACTIVITY_LOG=<log>` (feature 029,
    ///   FR-012a), unless the user declined the component (FR-012e).
    /// - `EventLog` → nothing: Copilot writes its own log, and the tail reads it.
    fn activity_launch_for(&self, id: SessionId, spec: &mut LaunchSpec) -> Vec<OsString> {
        let provider = spec.provider.provider();
        // The source is derived from a config dir the provider may not be able to resolve. For
        // `Hooks` the question is only *which mechanism* — it carries no payload — so an
        // unresolvable directory is not a reason to skip the file; ask with what is available.
        let config_dir = provider.config_dir();
        let source = provider.activity_source(
            config_dir.as_deref().unwrap_or(Path::new("")),
            &spec.cwd,
            spec.session_id,
        );
        match source {
            ActivitySource::Hooks => self
                .hook_settings_file(id)
                .map(|path| vec!["--settings".into(), path.into_os_string()])
                .unwrap_or_default(),
            // Pi (feature 029). Everything here is best-effort in the same way the hook file is: a
            // failure costs the badge, which reads `Unknown`, and never the start (FR-012d).
            ActivitySource::Extension { log } => {
                // Declined (FR-012e): no `-e`, no variable, and `open_event_log_tail` opens no
                // watch. The session is otherwise identical, and nothing reports it as a fault.
                if !self.lock().catalog.pi_activity_component() {
                    return Vec::new();
                }
                // A log under an empty base would land relative to the session's cwd.
                if config_dir.is_none() {
                    return Vec::new();
                }
                let Some(log_var) = log.to_str() else {
                    tracing::warn!(session = %id.0, "Pi activity log path is not UTF-8; activity will be Unknown");
                    return Vec::new();
                };
                if let Some(parent) = log.parent() {
                    if let Err(err) = std::fs::create_dir_all(parent) {
                        tracing::warn!(session = %id.0, %err, "could not create the Pi activity log directory; activity will be Unknown");
                        return Vec::new();
                    }
                }
                match materialise_pi_activity_component() {
                    Ok(component) => {
                        spec.env
                            .push((PI_ACTIVITY_LOG_VAR.to_string(), log_var.to_string()));
                        vec!["-e".into(), component.into_os_string()]
                    }
                    Err(err) => {
                        tracing::warn!(session = %id.0, %err, "could not write the Pi activity component; activity will be Unknown");
                        Vec::new()
                    }
                }
            }
            ActivitySource::EventLog { .. } | ActivitySource::None => Vec::new(),
        }
    }

    fn hook_settings_file(&self, id: SessionId) -> Option<PathBuf> {
        let receiver = self.hooks.get()?;
        match receiver.prepare_settings(id) {
            Ok(path) => Some(path),
            Err(e) => {
                tracing::warn!(session = %id.0, error = %e, "could not write hook settings; activity will be Unknown");
                None
            }
        }
    }

    /// The environment-include-resolved variables for `cwd`, merged with the hardcoded `TERM` pair
    /// (FR-012b, BUG-003) — ready to hand straight to a spawn site's `env`. Disabled/blank-path
    /// short-circuits to `merge_with_term(&[])` without spawning a subprocess or caching (contracts/
    /// env-include-resolution.md's Non-goals). Otherwise served from `env_include_cache`, or
    /// resolved and cached on a miss.
    ///
    /// Sourcing the script (`env_include::resolve`) spawns a real, disposable subprocess and may
    /// block for up to the configured timeout (module invariant: never done under the state lock,
    /// same reason PTY spawning itself happens off-lock) — so the settings/cache are read under a
    /// short lock, the lock is dropped before resolving, and the result is written back under a
    /// second short lock.
    fn env_include_vars_for(&self, cwd: &Path) -> Vec<(String, String)> {
        let (enabled, script_path, timeout_secs) = {
            let settings = self.lock().catalog.settings_wire();
            (
                settings.env_include_enabled,
                settings.env_include_script_path,
                settings.env_include_timeout_secs,
            )
        };
        if !enabled || script_path.trim().is_empty() {
            return micold_core::env_include::merge_with_term(&[]);
        }
        if let Some(cached) = self.lock().env_include_cache.get(cwd) {
            return cached.clone();
        }
        let (vars, outcome) = micold_core::env_include::resolve(
            Path::new(&script_path),
            cwd,
            std::time::Duration::from_secs(timeout_secs),
        );
        if outcome != micold_core::env_include::EnvIncludeOutcome::Success {
            tracing::warn!(?outcome, cwd = %cwd.display(), "env-include resolution did not succeed");
        }
        let merged = micold_core::env_include::merge_with_term(&vars);
        self.lock()
            .env_include_cache
            .insert(cwd.to_path_buf(), merged.clone());
        merged
    }

    /// The environment an **AI-CLI** launch runs in: the session's resolved environment
    /// ([`Self::env_include_vars_for`]) with the provider's own [`AiCliProvider::launch_env`]
    /// merged in (feature 029, FR-020).
    ///
    /// Asked of every provider, never of one by name. Two of the three answer with nothing, and
    /// that is the point: the alternative was a `match` on which CLI a session runs at the spawn
    /// site, which is precisely the conditional the provider seam exists to remove (FR-019).
    ///
    /// **The session's own environment wins.** A provider's pair is added only where the resolved
    /// environment does not already carry that name, so a user who sets one of these in their
    /// environment-include script keeps it. That is the trait's own contract — a provider may not
    /// use `launch_env` to reach past settings that belong to the user (FR-007) — and it costs
    /// nothing in practice: nobody sets `PI_OFFLINE` by accident, and someone who sets it
    /// deliberately means it.
    fn ai_cli_env_for(&self, cwd: &Path, provider: AiCli) -> Vec<(String, String)> {
        let mut env = self.env_include_vars_for(cwd);
        for (name, value) in provider.provider().launch_env() {
            if !env.iter().any(|(existing, _)| *existing == name) {
                env.push((name, value));
            }
        }
        env
    }

    /// Invalidate the cached environment-include resolution for one directory (BUG-003) — called
    /// when the worktree at that path is deleted, mirroring the equivalent fix recorded in
    /// `specs/011-env-include-script/bugs/BUG-002.md`'s Resolution: a worktree recreated for the
    /// same branch reuses the exact same path (dir names are derived from the branch name), so a
    /// stale pre-deletion snapshot would otherwise be served forever for that path.
    pub fn invalidate_env_include(&self, cwd: &Path) {
        self.lock().env_include_cache.remove(cwd);
    }

    /// Invalidate every cached environment-include resolution (BUG-003) — called when the
    /// enabled/path/timeout settings themselves change (`SettingsSet`), since every cached
    /// directory's snapshot was resolved under the now-stale configuration.
    pub fn invalidate_env_include_all(&self) {
        self.lock().env_include_cache.clear();
    }

    /// Adopt the token this daemon will require, read from `path`.
    ///
    /// Called once at startup. A daemon that was given a token path and cannot read it must fail
    /// loudly rather than fall back to accepting everyone: a sandbox whose authentication silently
    /// turned itself off is worse than one that refuses to start.
    pub fn set_auth_token(&self, path: &Path) -> std::io::Result<()> {
        let token = micold_core::protocol::auth::Token::read_from(path)?;
        let _ = self.auth_token.set(token);
        Ok(())
    }

    /// What this daemon compares a client's introduction against (feature 027).
    pub fn expectation(&self) -> micold_core::protocol::handshake::Expectation {
        micold_core::protocol::handshake::Expectation {
            token: self.auth_token.get().cloned(),
            build: format!("micold-daemon {}", env!("CARGO_PKG_VERSION")),
            image: std::env::var("MICOLD_IMAGE_REFERENCE").unwrap_or_default(),
        }
    }

    /// The `Welcome` payload for a freshly-handshaked client.
    pub fn welcome_payload(&self) -> (CatalogSnapshot, DaemonSettings) {
        let inner = self.lock();
        (Self::snapshot_locked(&inner), inner.catalog.settings_wire())
    }

    /// A full catalog snapshot with each project's worktrees overlaid from the live git+fs discovery
    /// cache (T053). The catalog is the single writer of *durable* state (projects, sessions, display
    /// names); worktree existence/branch/status is git truth, cached in [`Inner::worktrees`] and
    /// merged here so a snapshot never runs a git subprocess under the state lock. Display names still
    /// come from the durable `worktree_names` overrides.
    fn snapshot_locked(inner: &Inner) -> CatalogSnapshot {
        let mut snapshot = inner.catalog.snapshot();
        let overrides = &inner.catalog.workspace().worktree_names;
        for project in &mut snapshot.projects {
            Self::overlay_live_summaries(inner, &mut project.sessions);
            if let Some(discovered) = inner.worktrees.get(&project.path) {
                let names = overrides.get(&project.path);
                // Feature 029: the overlay replaces the durable list wholesale, so the provenance
                // flag has to be re-derived here too — a worktree the catalog never heard of (no
                // session, no rename) still appears in discovery, and its flag must come from the
                // record rather than defaulting to `false` because this list was built elsewhere.
                let created = inner
                    .catalog
                    .workspace()
                    .user_created_worktrees(&project.path);
                project.worktrees = discovered
                    .iter()
                    .map(|wt| WorktreeSnapshot {
                        dir_name: wt.dir_name.clone(),
                        branch: wt.branch.clone(),
                        display_name: names
                            .and_then(|m| m.get(&wt.dir_name))
                            .cloned()
                            .unwrap_or_else(|| wt.dir_name.clone()),
                        status: wire_worktree_status(wt.status),
                        path: wt.path.clone(),
                        included: wt.included,
                        user_created: created.contains(&wt.dir_name),
                    })
                    .collect();
            }
        }
        snapshot
    }

    /// Overlay each summary's runtime-only fields — `activity`, the live OSC-0 title, and the input
    /// high-water mark — from the live registry (US2, T046/T047; T110/FR-028a). Durable state (the
    /// catalog) never carries activity (H3/A4), the persisted `label` lags the terminal title, and
    /// the `InputReceiver` lives with the session rather than the catalog, so all three are projected
    /// here at snapshot time. A session with no live entry keeps the catalog's values (activity
    /// `Unknown`, the persisted label, input serial `0`).
    ///
    /// `input_serial` is what lets a **new client process** drive a session it did not start: its
    /// stamper is empty, so without this it would stamp `0` into a receiver already at `N` and have
    /// every keystroke discarded as `Stale` (BUG-006).
    /// How one client is named to another: build **and** window instance (`010` BUG-022).
    ///
    /// A client that has already gone is named by a placeholder whose nonce is empty, so it can
    /// never compare equal to a live window's instance — a "who holds it" answer about a
    /// connection that no longer exists must not read as "you do".
    fn identify(inner: &Inner, id: ClientId) -> ClientIdentity {
        inner
            .clients
            .get(&id)
            .map(|c| c.identity.clone())
            .unwrap_or_else(|| {
                ClientIdentity::new(
                    "(a client that has since disconnected)",
                    ClientInstance {
                        pid: 0,
                        nonce: String::new(),
                    },
                )
            })
    }

    fn overlay_live_summaries(inner: &Inner, summaries: &mut [SessionSummary]) {
        for summary in summaries {
            if let Some(live) = inner.sessions.get(&summary.id) {
                summary.activity = live.activity.signal().clone();
                summary.input_serial = live.input.expected();
                // `012` FR-008/BUG-003: which shell instances exist is durable-ish client state,
                // but which are *alive* is only knowable here. The client cannot infer it — no
                // frames is a quiet shell as much as a dead one.
                // Filtered by actual liveness, not merely by presence in the map: a shell that
                // exits stays registered so its final screen survives, and reporting it live would
                // make `exited` unreachable — the state `012` FR-008 most needs (BUG-003).
                summary.live_shells = live
                    .procs
                    .iter()
                    .filter_map(|(p, proc)| match p {
                        SessionProcess::Shell(id) if proc.pty.is_alive() => Some(*id),
                        _ => None,
                    })
                    .collect();
                if let Some(title) = &live.last_title {
                    summary.title = SessionLabel::Named(title.clone());
                }
            } else if let Some(signal) = inner.ended.get(&summary.id) {
                // No live entry, but this daemon watched it end (`010` BUG-018). The catalog cannot
                // carry activity (H3/A4) and the FSM went with the live entry, so this is the only
                // place the signal can come from — and it is only ever consulted here, where there
                // is no live FSM to disagree with.
                summary.activity = signal.clone();
            }
            // A start that failed for a reason worth saying (feature 026, FR-010). `attempts: 0`
            // deliberately: a missing binary is not a crash loop, and letting it climb toward
            // `MAX_RESTART_ATTEMPTS` would be three retries of a `PATH` problem — noise, and it
            // makes the reason arrive late.
            if let Some(reason) = inner.start_failures.get(&summary.id) {
                summary.lifecycle = WireLifecycle::Failed {
                    reason: reason.clone(),
                    attempts: 0,
                };
            }
        }
    }

    /// Register a client, returning its id and the receiver its writer task drains. Increments the
    /// connected-client count (FR-002).
    pub fn register(
        &self,
        identity: ClientIdentity,
    ) -> (ClientId, mpsc::UnboundedReceiver<Frame<DaemonMsg>>) {
        let id = self.next_id.fetch_add(1, Ordering::SeqCst);
        let (tx, rx) = mpsc::unbounded_channel();
        self.lock().clients.insert(
            id,
            ClientHandle {
                tx,
                identity,
                viewed: HashMap::new(),
            },
        );
        self.presence
            .lock()
            .expect("presence mutex poisoned")
            .client_connected();
        (id, rx)
    }

    /// Deregister a client on disconnect (for any reason). Releases every attachment it held —
    /// since the connection owns the attachment, EOF is the release signal (data-model T2).
    pub fn deregister(&self, id: ClientId) {
        {
            let mut inner = self.lock();
            inner.clients.remove(&id);
            inner.attachments.retain(|_, att| att.client != id);
        }
        self.presence
            .lock()
            .expect("presence mutex poisoned")
            .client_disconnected(micold_core::clock::now());
    }

    /// Release every attachment `id` holds, without deregistering it (FR-025a, BUG-009, T121).
    ///
    /// `deregister` above is the ordinary release, and it runs when the connection's message loop
    /// exits. That is the right *place* only while nothing can park that loop: a handler blocked on
    /// a multi-minute git operation kept a departed client's project held for the rest of it, so the
    /// client's own reconnect was refused as busy by a connection that no longer existed — the
    /// takeover banner naming the reconnecting window's own build. T120 removes the parking; this
    /// makes the release independent of it, driven by the transport itself: the writer task calls it
    /// the moment a push to this client fails, which is the earliest observable proof the peer is
    /// gone. Idempotent, and safe to interleave with `deregister`.
    pub fn release_attachments(&self, id: ClientId) {
        self.lock().attachments.retain(|_, att| att.client != id);
    }

    /// The per-project gate serializing mutating worktree work (BUG-009, T120). Created on first
    /// use. Callers `lock()` it inside the spawned operation — never on the connection loop, which
    /// must stay free to answer this client's other frames (FR-026a).
    pub fn worktree_gate(&self, project: &Path) -> Arc<tokio::sync::Mutex<()>> {
        Arc::clone(
            self.lock()
                .worktree_gates
                .entry(project.to_path_buf())
                .or_default(),
        )
    }

    /// Send one message to a specific client (best-effort; a dead channel is ignored).
    pub fn send(&self, id: ClientId, msg: DaemonMsg) {
        // A refused mutation is a failure of this service, so it belongs in this service's log
        // (S13: "for each failure in S7, S8 and S11, confirm the cause is determinable from
        // logs"). It was determinable only from the UI: every `OperationError` was sent and none
        // was logged, so a user who had closed the dialog had nothing left to read (`010`
        // BUG-020).
        //
        // Here rather than at the call sites: there are twenty-five of them across `server.rs` and
        // this is the one door they all leave by, so a new refusal is logged without anyone having
        // to remember to. `detail` is included because it is usually the only part that names the
        // cause — git's own stderr for a worktree create — and it is never terminal content or
        // session input, which is the only thing FR-047 forbids.
        if let DaemonMsg::OperationError {
            req,
            kind,
            message,
            detail,
        } = &msg
        {
            tracing::warn!(
                req = %req,
                kind = ?kind,
                detail = detail.as_deref().unwrap_or("-"),
                "operation refused: {message}"
            );
        }
        if let Some(client) = self.lock().clients.get(&id) {
            let _ = client.tx.send(Frame::Control(msg));
        }
    }

    /// A clone of a client's outgoing frame sender, for a streaming task to push `Frame::Grid` into
    /// the same ordered channel the writer drains. `None` if the client has gone.
    pub fn frame_sender(&self, id: ClientId) -> Option<mpsc::UnboundedSender<Frame<DaemonMsg>>> {
        self.lock().clients.get(&id).map(|c| c.tx.clone())
    }

    /// Attach `id` to `project`. A second attach on a held project is refused with an actionable
    /// takeover offer unless `force`, in which case the prior holder is `Displaced` (FR-023/024).
    /// Returns the project's session summaries on success.
    pub fn attach(
        &self,
        id: ClientId,
        project: PathBuf,
        force: bool,
    ) -> Result<Vec<SessionSummary>, RefusalReason> {
        let mut inner = self.lock();

        if let Some(att) = inner.attachments.get(&project) {
            if att.client != id {
                if !force {
                    let holder = Self::identify(&inner, att.client);
                    return Err(RefusalReason::ProjectBusy {
                        project,
                        holder,
                        since_secs: att.since.elapsed().as_secs(),
                    });
                }
                // Forced takeover: notify the displaced holder, but do NOT terminate it (T4).
                let displaced = att.client;
                let by = Self::identify(&inner, id);
                if let Some(prev) = inner.clients.get(&displaced) {
                    let _ = prev.tx.send(Frame::Control(DaemonMsg::Displaced {
                        project: project.clone(),
                        by,
                    }));
                }
            }
        }

        inner.attachments.insert(
            project.clone(),
            Attachment {
                client: id,
                since: Instant::now(),
            },
        );
        let mut sessions = inner.catalog.sessions_for(&project);
        Self::overlay_live_summaries(&inner, &mut sessions);
        Ok(sessions)
    }

    /// Release `id`'s attachment on `project` (if it holds it).
    pub fn detach(&self, id: ClientId, project: &Path) {
        let mut inner = self.lock();
        if inner.attachments.get(project).map(|a| a.client) == Some(id) {
            inner.attachments.remove(project);
        }
        if let Some(client) = inner.clients.get_mut(&id) {
            client.viewed.remove(project);
        }
    }

    /// Record which session a client is viewing for a project (FR-016), and remember it durably
    /// so reopening the application lands there (feature 025, FR-001).
    ///
    /// Two records, deliberately different in lifetime. `client.viewed` is per connection and dies
    /// with it — it is what the daemon streams output to. The catalog's memory outlives every
    /// process, and is what a launch reads.
    ///
    /// **Only a `Some` is remembered.** A report of no session does not clear the memory: the
    /// pointer goes to nothing for reasons the user never took — closing a session, an internal
    /// cleanup — and erasing the memory on those would silently cost them the place they would
    /// have returned to. A memory naming a session that can no longer be shown is harmless,
    /// because restoring declines it (feature 025, FR-005a).
    ///
    /// A write failure is logged rather than propagated: losing the memory for one project is not
    /// a reason to fail the message that reports it, and the session the user is viewing is
    /// unaffected either way.
    pub fn set_viewed(&self, id: ClientId, project: PathBuf, session: Option<SessionId>) {
        let mut inner = self.lock();
        if let Some(client) = inner.clients.get_mut(&id) {
            client.viewed.insert(project.clone(), session);
        }
        if let Some(session) = session {
            if let Err(err) = inner.catalog.remember_foreground(&project, session) {
                tracing::warn!(
                    project = %project.display(),
                    %err,
                    "could not persist the last-used session for this project"
                );
            }
        }
    }

    /// Set the scrollback limit and push `SettingsChanged` to every client (FR-012a, FR-011).
    pub fn set_scrollback(&self, lines: usize) -> std::io::Result<()> {
        let settings = {
            let mut inner = self.lock();
            inner.catalog.set_scrollback(lines)?;
            inner.catalog.settings_wire()
        };
        self.broadcast(DaemonMsg::SettingsChanged { settings });
        Ok(())
    }

    /// Set the default AI CLI and push `SettingsChanged` to every client (feature 026, FR-003).
    pub fn set_default_ai_cli(&self, which: AiCli) -> std::io::Result<()> {
        let settings = {
            let mut inner = self.lock();
            inner.catalog.set_default_ai_cli(which)?;
            inner.catalog.settings_wire()
        };
        self.broadcast(DaemonMsg::SettingsChanged { settings });
        Ok(())
    }

    /// Turn the Pi activity component on or off and push `SettingsChanged` to every client
    /// (feature 029, FR-012e).
    pub fn set_pi_activity_component(&self, on: bool) -> std::io::Result<()> {
        let settings = {
            let mut inner = self.lock();
            inner.catalog.set_pi_activity_component(on)?;
            inner.catalog.settings_wire()
        };
        self.broadcast(DaemonMsg::SettingsChanged { settings });
        Ok(())
    }

    /// Set any of the three environment-include settings and push `SettingsChanged` to every
    /// client (FR-012b, FR-011). Invalidates every cached per-directory resolution (T098/BUG-003):
    /// each cached directory's snapshot was resolved under the now-stale configuration.
    pub fn set_env_include(
        &self,
        enabled: Option<bool>,
        script_path: Option<String>,
        timeout_secs: Option<u64>,
    ) -> std::io::Result<()> {
        let settings = {
            let mut inner = self.lock();
            inner
                .catalog
                .set_env_include(enabled, script_path, timeout_secs)?;
            inner.catalog.settings_wire()
        };
        self.invalidate_env_include_all();
        self.broadcast(DaemonMsg::SettingsChanged { settings });
        Ok(())
    }

    /// A full catalog snapshot (worktrees overlaid from the discovery cache). The client-facing
    /// projection of durable state + git-discovered worktrees.
    pub fn catalog_snapshot(&self) -> CatalogSnapshot {
        Self::snapshot_locked(&self.lock())
    }

    /// Push a full `CatalogChanged` snapshot to every connected client (FR-011; idempotent).
    pub fn broadcast_catalog(&self) {
        let catalog = Self::snapshot_locked(&self.lock());
        self.broadcast(DaemonMsg::CatalogChanged { catalog });
    }

    /// Send `msg` to every connected client (a mutation reaches all affected clients without further
    /// user action — FR-011).
    pub fn broadcast(&self, msg: DaemonMsg) {
        let inner = self.lock();
        for client in inner.clients.values() {
            let _ = client.tx.send(Frame::Control(msg.clone()));
        }
    }

    /// Create a new durable session in `project` at `worktree_dir` (empty = project root), returning
    /// the daemon-assigned id. Persists the catalog under the lock; the caller then `start_session`s
    /// it and broadcasts the updated catalog.
    pub fn create_session(
        &self,
        project: &Path,
        worktree_dir: &str,
        provider: AiCli,
    ) -> io::Result<SessionId> {
        self.lock()
            .catalog
            .create_session(project, worktree_dir, provider)
    }

    // --- US3: worktree management through the daemon (T053) ---

    /// A known project's repo path + whether it is a git repository. `None` if unknown.
    pub fn project_repo(&self, project: &Path) -> Option<(PathBuf, bool)> {
        self.lock().catalog.project_repo(project)
    }

    /// Re-discover `project`'s worktrees from git + the filesystem and refresh the cache the catalog
    /// snapshot reads (T053). **Blocking** — it runs a `git` subprocess — so the caller must invoke it
    /// off the async runtime (`spawn_blocking`) and, critically, this never holds the state lock across
    /// the subprocess: it locks once to read the repo path, runs git *unlocked*, then locks again to
    /// store the result. A non-git or unknown project clears any stale cache entry.
    pub fn refresh_worktrees(&self, project: &Path) {
        let repo = self.lock().catalog.project_repo(project);
        match repo {
            Some((repo, true)) => {
                let included = self.included_worktrees(project);
                let discovered = worktree::discover(&GitCli::new(), &repo, &included);
                let mut inner = self.lock();
                // Feature 029 FR-006: the one-time evidence backfill, here because this is the
                // single point where the discovered list, the durable records and the marker are
                // all in hand — already off the async runtime, and already on every path that can
                // reveal a worktree to a user. It is a no-op after the first run per project
                // (FR-006c) and refuses to run on an unreadable one (FR-011), so paying for the
                // check on every refresh costs a set lookup.
                if let Err(e) = inner
                    .catalog
                    .migrate_worktree_provenance(project, &discovered)
                {
                    tracing::warn!(
                        project = %project.display(),
                        error = %e,
                        "could not persist the worktree provenance backfill"
                    );
                }
                inner.worktrees.insert(project.to_path_buf(), discovered);
            }
            _ => {
                self.lock().worktrees.remove(project);
            }
        }
    }

    /// The worktrees `project` shows from outside the app's own directory (016 BUG-002, FR-030).
    ///
    /// Read under the lock and returned by value, never held across the git subprocess that uses it
    /// — the same discipline [`Self::refresh_worktrees`] states for the repo path.
    pub fn included_worktrees(&self, project: &Path) -> Vec<PathBuf> {
        self.lock().catalog.included_worktrees(project)
    }

    /// What this app knows about who created `project`'s worktrees (029 FR-004), owned rather than
    /// borrowed.
    ///
    /// Returned by value for the same reason as [`Self::included_worktrees`]: every caller hands it
    /// to a `spawn_blocking` closure that shells out to git, and the lock must not be held across
    /// that. The pair is reassembled into a [`ProvenanceView`] inside the closure, which is why
    /// this returns the two facts rather than the view — the view borrows, and a borrow cannot
    /// cross into a `'static` task.
    pub fn provenance(&self, project: &Path) -> (BTreeSet<String>, bool) {
        let inner = self.lock();
        let workspace = inner.catalog.workspace();
        (
            workspace.user_created_worktrees(project).clone(),
            workspace.unreadable_projects.contains(project),
        )
    }

    /// Start showing `path` among `project`'s worktrees, persisting it (016 BUG-002, FR-027).
    pub fn include_worktree(&self, project: &Path, path: &Path) -> io::Result<()> {
        self.lock().catalog.include_worktree(project, path)
    }

    /// Record that this app created `dir_name` under `project` (feature 029, FR-002/FR-003).
    pub fn record_worktree_provenance(&self, project: &Path, dir_name: &str) -> io::Result<()> {
        self.lock()
            .catalog
            .record_worktree_provenance(project, dir_name)
    }

    /// Claim `dir_name` under `project` as the user's own (029 FR-020) — the same record a create
    /// writes, requested by the user instead of by the app.
    pub fn claim_worktree(&self, project: &Path, dir_name: &str) -> io::Result<()> {
        self.lock().catalog.claim_worktree(project, dir_name)
    }

    /// Drop the provenance record for a worktree that has just been removed (029 FR-018/FR-019).
    pub fn forget_worktree_provenance(&self, project: &Path, dir_name: &str) -> io::Result<()> {
        self.lock()
            .catalog
            .forget_worktree_provenance(project, dir_name)
    }

    /// Stop showing `path` (016 BUG-002, FR-030).
    pub fn exclude_worktree(&self, project: &Path, path: &Path) -> io::Result<()> {
        self.lock().catalog.exclude_worktree(project, path)
    }

    /// `project`'s worktree at `path`, as the catalog snapshot now presents it (016 BUG-002).
    ///
    /// Read after a refresh, so it answers from live discovery rather than from what the caller
    /// hoped: a path that was included but is no longer there simply is not here.
    pub fn worktree_snapshot_at(
        &self,
        project: &Path,
        path: &Path,
    ) -> Option<micold_core::protocol::messages::WorktreeSnapshot> {
        self.catalog_snapshot()
            .projects
            .into_iter()
            .find(|p| p.path == project)?
            .worktrees
            .into_iter()
            .find(|w| w.path == path)
    }

    /// Discover sessions started outside this application, for every location of `project` and
    /// every registered AI CLI (feature 026, FR-014/FR-015 — research R15).
    ///
    /// **Blocking**: it reads each provider's own conversation store, so the caller runs it off the
    /// async runtime — in the same `spawn_blocking` hop that already refreshed the worktrees, whose
    /// cache this reads for the location list.
    ///
    /// # Why it lives here
    ///
    /// The daemon is `projects.json`'s single writer, it already enumerates the project's
    /// worktrees, and a catalog snapshot is about to be sent. So the pass is a fourth step in the
    /// attach sequence rather than a new RPC, a protocol change or a client round trip (R15).
    ///
    /// # The cost rule, and how it is held
    ///
    /// FR-014's work is **per location** — one index read or one directory listing each — never per
    /// conversation, so a worktree holding hundreds of recorded conversations costs what one
    /// holding three costs. That is held by **ordering**: the catalog's own ids are subtracted
    /// *before* any `is_archived` check, so the per-id filesystem probe only ever runs over ids the
    /// application has genuinely never seen. Reverse those two steps and a project with a long
    /// history stats every conversation on every open.
    ///
    /// Each provider's `config_dir()` is resolved independently: one returning `None` must not
    /// suppress the other's contribution.
    ///
    /// Returns how many sessions were adopted (`0` ⇒ no write happened).
    pub fn discover_external_sessions(&self, project: &Path) -> usize {
        // Locations, and the ids we already know — both read under the lock, once.
        let (locations, known) = {
            let inner = self.lock();
            let mut locations = vec![SessionLocation::Default];
            if let Some(worktrees) = inner.worktrees.get(project) {
                locations.extend(
                    worktrees
                        .iter()
                        .filter(|wt| wt.can_start_session())
                        .map(|wt| SessionLocation::Worktree(wt.dir_name.clone())),
                );
            }
            (locations, inner.catalog.known_session_ids(project))
        };

        // Off the lock: everything below touches the providers' stores.
        let mut found = Vec::new();
        let mut seen = known;
        for which in AiCli::ALL {
            let provider = which.provider();
            let Some(config_dir) = provider.config_dir() else {
                continue;
            };
            for location in &locations {
                let cwd = location.cwd(project);
                for id in provider.recorded_session_ids(&config_dir, &cwd) {
                    // Subtract first — see the cost rule above.
                    if !seen.insert(id) {
                        continue;
                    }
                    if provider.is_archived(&config_dir, &cwd, id) {
                        continue;
                    }
                    // A title wins; with none, the first typed turn (feature 032, C6.1). Both are
                    // read once, here, and persisted by the adoption below.
                    let label = match provider.read_title(&config_dir, &cwd, id) {
                        Some(title) => SessionLabel::Named(title),
                        None => provider
                            .read_label(&config_dir, &cwd, id)
                            .map_or(SessionLabel::Pending, SessionLabel::Derived),
                    };
                    found.push(Session::restored(
                        SessionId::from_uuid(id),
                        location.clone(),
                        label,
                        TerminalMode::AiCli,
                        which,
                    ));
                }
            }
        }

        if found.is_empty() {
            return 0;
        }
        self.lock()
            .catalog
            .adopt_discovered_sessions(project, found)
    }

    /// Give every **known but unnamed** session of `project` the name its own AI CLI recorded for
    /// it, persisting each one (feature 029, US2 — FR-006, FR-007, FR-010). Returns how many names
    /// **and labels** were recorded (`0` ⇒ no write happened, so the caller need not broadcast —
    /// feature 032, C6.3a).
    ///
    /// "Unnamed" is **not `Named`**, which since feature 032 is two states, not one: `Pending` (the
    /// row reads "New session") and `Derived` (the row reads the first thing the user typed). A
    /// `Derived` session is still a candidate here, because the title the CLI writes later has to
    /// replace the label (FR-006, C6.2); it is asked for a title only, never for a second label.
    ///
    /// This is the one-time repair for every session that predates the live write path: the name
    /// was observed and displayed for as long as the session ran, and never written, so the record
    /// says `Pending` and the row reads "New session" until something runs it again. The
    /// conversation's own store still has the name, and this pass fetches it without starting
    /// anything.
    ///
    /// # Why it is not folded into [`Self::discover_external_sessions`]
    ///
    /// That pass deliberately subtracts the ids the catalog already knows *before* touching the
    /// filesystem, which is what holds its per-*location* cost rule. These are exactly the ids it
    /// subtracts. Recovery cannot be per location — a name is per conversation — so it is a
    /// separate pass with a different bound: a recovered name is **persisted**, so a session costs
    /// one read once and is `Named` for good (research R4).
    ///
    /// # Blocking
    ///
    /// Reads the providers' stores, so it runs in the same `spawn_blocking` hop as the worktree
    /// refresh and the discovery pass, never on the async runtime.
    ///
    /// A `Named` session is filtered out **under the lock, before any filesystem access** — that is
    /// what bounds the pass, and it is also what protects FR-008: a name already recorded is never
    /// re-read, so a deleted transcript cannot take it away. A `None` read is a no-op for the same
    /// reason: never an error, never a wrong name, and never a way back to `Pending`.
    ///
    /// A session showing a derived label (feature 032) **is** a candidate: it has no name yet, and
    /// the title the CLI writes later must replace the label (FR-006, C6.2). It is asked for a
    /// title only — `unlabelled` is false, so its records are never read for a second label
    /// (FR-007).
    pub fn recover_session_names(&self, project: &Path) -> usize {
        // Candidates, read under the lock, once: the session, where it runs, which CLI owns it, and
        // whether it has no label at all yet.
        let candidates: Vec<RecoveryCandidate> = {
            let inner = self.lock();
            inner
                .catalog
                .workspace()
                .sessions
                .get(project)
                .map(|sessions| {
                    sessions
                        .iter()
                        .filter(|s| !s.archived && !matches!(s.label, SessionLabel::Named(_)))
                        .map(|s| RecoveryCandidate::of(s, project))
                        .collect()
                })
                .unwrap_or_default()
        };
        self.record_recovered_names(candidates)
    }

    /// [`Self::recover_session_names`] for the sessions running here whose conversation may have
    /// gained a name since it was last looked for — they started, or their activity moved — and
    /// that still have none (feature 029, FR-011). Returns how many were named, so the caller knows
    /// whether to broadcast.
    ///
    /// This is what shows a running `pi` row its first message: `pi` titles its terminal with a
    /// name only after `/name`, so the terminal alone would hold the row on the placeholder until
    /// the next refresh. Bounded the same way — a `Named` session is never a candidate — and by the
    /// flag, which only an event sets.
    ///
    /// # Blocking
    ///
    /// Reads the providers' stores, so it runs in the supervisor's `spawn_blocking` hop.
    pub fn recover_live_session_names(&self) -> usize {
        let candidates: Vec<RecoveryCandidate> = {
            let mut guard = self.lock();
            let inner = &mut *guard;
            let workspace = inner.catalog.workspace();
            inner
                .sessions
                .iter_mut()
                .filter(|(_, live)| live.name_stale)
                .filter_map(|(id, live)| {
                    live.name_stale = false;
                    let (project, session) = workspace.find_session(*id)?;
                    (session.mode == TerminalMode::AiCli
                        && !session.archived
                        && !matches!(session.label, SessionLabel::Named(_)))
                    .then(|| RecoveryCandidate::of(session, project))
                })
                .collect()
        };
        self.record_recovered_names(candidates)
    }

    /// Read each candidate's name from its own provider's store, off the lock, and record the ones
    /// found against sessions that are still unnamed. Returns how many were recorded — titles and
    /// labels alike, so a label alone makes the supervisor broadcast (feature 032, C6.3a).
    ///
    /// "Still unnamed" is re-checked **under the lock**, and the two kinds are checked differently:
    /// a title is recorded unless the session became `Named` while this was off the lock, and a
    /// label only while the session is still `Pending` (C6.3). So a title and a label racing for
    /// one session always end on the title, and nothing ever walks a session back toward `Pending`
    /// (C6.6).
    ///
    /// A candidate with no title and no label yet is also asked for its first-turn label (feature
    /// 032, C6.3): the title first, because a title outranks a label (FR-005), and the label only
    /// when there is no title — one read per session, and never a second label (FR-007). Both kinds
    /// count, so a label alone is enough for the caller to broadcast (C6.3a).
    fn record_recovered_names(&self, candidates: Vec<RecoveryCandidate>) -> usize {
        if candidates.is_empty() {
            return 0;
        }

        // Off the lock: everything below touches the providers' stores. Each session asks its
        // **own** provider — one hoisted provider would read a Copilot session's name out of
        // `claude`'s store, find nothing, or worse find another CLI's conversation under the same
        // id and name the row wrongly (contract C16). A provider with no resolvable config dir
        // contributes nothing and stops nothing (C20).
        let found: Vec<(SessionId, Recovered)> = candidates
            .into_iter()
            .filter_map(|candidate| {
                let RecoveryCandidate {
                    id,
                    cwd,
                    provider: which,
                    unlabelled,
                } = candidate;
                let provider = which.provider();
                let config_dir = provider.config_dir()?;
                if let Some(title) = provider.read_title(&config_dir, &cwd, id.0) {
                    return Some((id, Recovered::Title(title)));
                }
                if !unlabelled {
                    return None;
                }
                let label = provider.read_label(&config_dir, &cwd, id.0)?;
                Some((id, Recovered::Label(label)))
            })
            .collect();
        if found.is_empty() {
            return 0;
        }

        let mut inner = self.lock();
        let mut recovered = 0;
        for (id, found) in found {
            let result = match found {
                Recovered::Title(name) => {
                    // The live path may have named it in the time this spent off the lock, and the
                    // terminal's name is the newer one. A label is replaced (FR-006).
                    let named = inner
                        .catalog
                        .workspace()
                        .find_session(id)
                        .is_none_or(|(_, s)| matches!(s.label, SessionLabel::Named(_)));
                    if named {
                        continue;
                    }
                    inner.catalog.record_session_name(id, &name)
                }
                // Only onto a session still without any label: the catalog refuses a `Named` or
                // `Derived` one, so a title that arrived meanwhile keeps its place (C6.6).
                Recovered::Label(label) => inner.catalog.record_session_label(id, &label),
            };
            match result {
                Ok(true) => recovered += 1,
                Ok(false) => {}
                Err(err) => tracing::warn!(
                    session = %id.0,
                    %err,
                    "could not persist the recovered session name or label; it is still shown"
                ),
            }
        }
        recovered
    }

    /// Set a worktree's display-name override for `project` (validated by the caller), persisting.
    pub fn set_worktree_display_name(
        &self,
        project: &Path,
        dir_name: &str,
        name: &str,
    ) -> io::Result<()> {
        self.lock()
            .catalog
            .set_worktree_display_name(project, dir_name, name)
    }

    /// The ids of `project`'s `dir_name` worktree sessions that are **currently live** (hosted in the
    /// registry) — the ones a delete would orphan (W2). A durable-but-not-live session is not counted:
    /// there is no process to strand, so it never blocks a `stop_sessions:false` delete.
    pub fn worktree_live_sessions(&self, project: &Path, dir_name: &str) -> Vec<SessionId> {
        let inner = self.lock();
        inner
            .catalog
            .worktree_session_ids(project, dir_name)
            .into_iter()
            .filter(|id| inner.sessions.contains_key(id))
            .collect()
    }

    /// The git branch bound to `project`'s `dir_name` worktree, from the live git-discovery cache
    /// (T053; feature 013, FR-011). `None` if the worktree is unknown or has no bound branch
    /// (an orphan/invalid directory) — the caller treats that as "nothing to delete", never as
    /// an error.
    pub fn worktree_branch(&self, project: &Path, dir_name: &str) -> Option<String> {
        let inner = self.lock();
        inner
            .worktrees
            .get(project)?
            .iter()
            .find(|w| w.dir_name == dir_name)
            .and_then(|w| w.branch.clone())
    }

    /// Archive (durably) every session of `project`'s `dir_name` worktree and drop each from the live
    /// registry, returning the removed primaries so the caller can `kill()` them **outside** the lock
    /// (module invariant; mirrors `remove_session`). Gated: the caller invokes this only after the git
    /// worktree removal succeeded (main `d88c7a1`) — never before — so a failed delete leaves the
    /// sessions untouched, not permanently archived.
    pub fn archive_and_remove_worktree_sessions(
        &self,
        project: &Path,
        dir_name: &str,
    ) -> io::Result<Vec<Arc<PtySession>>> {
        let mut inner = self.lock();
        let ids = inner.catalog.archive_worktree_sessions(project, dir_name)?;
        Ok(Self::remove_live_by_ids(&mut inner, ids))
    }

    /// Remove the given session ids from the live registry, returning **every** removed process
    /// handle so the caller can `kill()` them **outside** the lock (module invariant). Shared by the
    /// worktree-, project-, and session-delete paths.
    ///
    /// All of them, not just each primary (BUG-001): dropping the `LiveSession` only reclaims a
    /// process when its `Arc` refcount reaches zero, and a view-stream task holds a clone of
    /// whichever process it is streaming. The explicit `kill()` is what makes teardown prompt rather
    /// than dependent on when some other task happens to let go — the primary always got one, and
    /// shell instances silently did not.
    fn remove_live_by_ids(inner: &mut Inner, ids: Vec<SessionId>) -> Vec<Arc<PtySession>> {
        let mut removed = Vec::new();
        for id in ids {
            if let Some(live) = inner.sessions.remove(&id) {
                removed.extend(live.procs.values().map(|p| Arc::clone(&p.pty)));
            }
            // Every caller here is an archive/forget path — the session is not coming back, so its
            // recorded size goes with it (FR-020a). A mere stop/kill does *not* come through here,
            // which is what keeps the size across a restart of the same session.
            inner.sizes.remove(&id);
            // Same reasoning for how it ended (BUG-018): an archived session has no row to badge.
            inner.ended.remove(&id);
        }
        removed
    }

    /// Add (open) a project by path, discovering its git/availability facts, persisting (T053). The
    /// caller broadcasts the refreshed catalog.
    pub fn add_project(&self, path: &Path) -> io::Result<()> {
        self.lock()
            .catalog
            .add_project(path, &micold_core::fs_scan::StdFolderScanner::new())
    }

    /// Record a known project as the active one (002 BUG-003). `Ok(false)` when the path is not a
    /// known project or its folder is unavailable; nothing is persisted then.
    pub fn activate_project(&self, path: &Path) -> io::Result<bool> {
        self.lock()
            .catalog
            .activate_project(path, &micold_core::fs_scan::StdFolderScanner::new())
    }

    /// Forget a known project, dropping its discovery cache and returning its live primaries so the
    /// caller can `kill()` them outside the lock (T053, feature 014). A no-op for an unknown path.
    pub fn forget_project(&self, path: &Path) -> io::Result<Vec<Arc<PtySession>>> {
        let mut inner = self.lock();
        let ids = inner.catalog.forget_project(path)?;
        inner.worktrees.remove(path);
        Ok(Self::remove_live_by_ids(&mut inner, ids))
    }

    /// Rename a project's display name (validated by the caller), persisting (T053).
    pub fn rename_project(&self, path: &Path, name: &str) -> io::Result<()> {
        self.lock().catalog.rename_project(path, name)
    }

    /// Delete (archive) a session and stop its live processes (T053). Returns the owning project
    /// path (if the session was known) and every removed process handle for the caller to `kill()`
    /// outside the lock. A `None` project means the id was unknown — the handler replies `NotFound`.
    pub fn delete_session(
        &self,
        session: SessionId,
    ) -> io::Result<(Option<PathBuf>, Vec<Arc<PtySession>>)> {
        let mut inner = self.lock();
        let owner = inner.catalog.archive_session(session)?;
        let ptys = Self::remove_live_by_ids(&mut inner, vec![session]);
        Ok((owner, ptys))
    }

    /// Prune (archive) empty sessions of `project` — those the AI CLI never recorded a conversation
    /// for (FR-007a, T056). **Blocking**: it stats the provider's conversation store, so the caller
    /// runs it off the async runtime. The filesystem checks happen **outside** the state lock (the
    /// lock is taken only to gather candidates and, at the end, to archive). Live sessions are
    /// excluded — a just-created session has no conversation yet but must not be pruned. Returns the
    /// ids archived (empty ⇒ no write happened). The handler calls this only for an attached project,
    /// so pruning always has an observer.
    pub fn prune_empty_sessions(&self, project: &Path) -> io::Result<Vec<SessionId>> {
        // 1. Candidates under the lock: non-archived and not currently live. Each carries its own
        //    provider (feature 026) — this loop *archives* what it judges empty, so judging a mixed
        //    set with one hoisted provider would silently archive every session of the other CLI.
        let candidates: Vec<(SessionId, PathBuf, AiCli)> = {
            let inner = self.lock();
            inner
                .catalog
                .prunable_session_cwds(project)
                .into_iter()
                .filter(|(id, _, _)| !inner.sessions.contains_key(id))
                .collect()
        };
        if candidates.is_empty() {
            return Ok(Vec::new());
        }
        // 2. Off-lock: keep only those with NO recorded conversation, each asked of **its own**
        //    provider. Config dirs are resolved once per provider and independently: one provider
        //    being undeterminable must not condemn or spare the other's sessions, and an
        //    undeterminable one still prunes nothing of its own (never drop a session on
        //    uncertainty).
        let mut config_dirs: HashMap<AiCli, Option<PathBuf>> = HashMap::new();
        let empty: Vec<SessionId> = candidates
            .into_iter()
            .filter(|(id, cwd, which)| {
                let provider = which.provider();
                let config = config_dirs
                    .entry(*which)
                    .or_insert_with(|| provider.config_dir());
                match config {
                    Some(config) => !provider.has_recorded_conversation(config, cwd, id.0),
                    None => false,
                }
            })
            .map(|(id, _, _)| id)
            .collect();
        if empty.is_empty() {
            return Ok(Vec::new());
        }
        // 3. Archive under the lock, persisting once.
        self.lock().catalog.archive_session_ids(&empty)
    }

    /// FR-006a/b: at service startup, present every session with a recorded AI-CLI conversation as
    /// `InterruptedResumable` (never auto-relaunched — only an explicit `SessionStart` resumes it).
    /// **Blocking** (it stats the provider's conversation store), so the caller runs it off the async
    /// runtime. Runs exactly once, before the accept loop starts, so there is no other client to
    /// contend for the lock — the provider stat happening under the lock is safe here for that reason
    /// (unlike the steady-state paths, which must never block under the lock). Returns the count.
    pub fn present_interrupted_resumable_at_startup(&self) -> usize {
        // Per session, not per workspace (feature 026). Asking one provider about every session
        // means none of the other CLI's is ever presented as resumable — it reports no recorded
        // conversation for ids it has never seen, which reads as "created but never started".
        // Config dirs are resolved lazily, once each, and independently: one provider's
        // unresolvable directory suppresses only its own sessions.
        let config_dirs: RefCell<HashMap<AiCli, Option<PathBuf>>> = RefCell::new(HashMap::new());
        self.lock()
            .catalog
            .present_interrupted_resumable(|id, cwd, _mode, which| {
                let provider = which.provider();
                let mut dirs = config_dirs.borrow_mut();
                let config = dirs.entry(which).or_insert_with(|| provider.config_dir());
                match config {
                    Some(config) => provider.has_recorded_conversation(config, cwd, id.0),
                    None => false,
                }
            })
    }

    /// Phase one of the idle unwind: make every live session durable as `InterruptedResumable`
    /// (feature 028, data-model G5, lifecycle contract §3.11). Returns how many records changed.
    ///
    /// **Runs before [`Self::take_live_sessions`], and that order is the whole point.** The record
    /// has to be true of a daemon that is already gone, so it reaches disk while the processes it
    /// describes are still running — a stop interrupted between the two phases then leaves a
    /// resumable session and an orphaned process tree, which is recoverable, rather than a record
    /// claiming `Running` with nothing behind it, which is not.
    ///
    /// Sessions are *marked*, never resumed (FR-006b/c): the next start presents them as resumable
    /// and waits for the user to ask.
    pub fn mark_live_sessions_interrupted(&self) -> usize {
        let mut inner = self.lock();
        let ids: Vec<SessionId> = inner.sessions.keys().copied().collect();
        match inner.catalog.mark_sessions_interrupted(&ids) {
            Ok(marked) => marked,
            Err(err) => {
                // Logged, not propagated. The daemon is stopping either way; failing the unwind
                // here would skip the process teardown below it and leave the tree orphaned, which
                // is strictly worse than a stale record the next start will reconcile.
                tracing::error!(%err, "could not persist sessions as interrupted-resumable");
                0
            }
        }
    }

    /// Phase two of the idle unwind: drop the live session table, returning how many were dropped.
    ///
    /// Dropping is the teardown. Each [`LiveSession`] owns its `PtySession`, whose `Drop`
    /// terminates the process tree (`supervisor.rs`), so there is no kill loop here and no way for
    /// one to drift out of step with the normal close path.
    ///
    /// Taken out of the lock before being dropped, so the (possibly slow) process teardown does not
    /// happen with the state lock held.
    pub fn take_live_sessions(&self) -> usize {
        let sessions = std::mem::take(&mut self.lock().sessions);
        let count = sessions.len();
        drop(sessions);
        count
    }

    /// The (non-archived) session summaries for a project, from durable state. Used to build the
    /// `Attached` reply after any attach-time pruning so it reflects the pruned result.
    ///
    /// **Not overlaid** — unlike every snapshot path, this does not run
    /// [`Self::overlay_live_summaries`], so each summary carries the catalog's defaults for the
    /// runtime-only fields: `activity` is always `Unknown`, `title` is the persisted label rather
    /// than the live OSC-0 title, and `input_serial` is `0` even for a session the daemon has been
    /// driving for hours. Correct for what `Attached` is (an acknowledgement naming the project's
    /// sessions), and harmless because a `CatalogChanged` built from the real snapshot follows every
    /// `Attached` — but it means a client MUST NOT adopt these values as state. Seeding an input
    /// counter from one would re-create BUG-006 exactly.
    pub fn sessions_for(&self, project: &Path) -> Vec<SessionSummary> {
        self.lock().catalog.sessions_for(project)
    }

    /// Bring a durable session to life: spawn its PTY-backed process and adopt it into the live
    /// registry (`ClientMsg::SessionStart`). Idempotent — a session that is already live is a no-op,
    /// so a redundant Start never double-spawns.
    ///
    /// The launch plan is read from the catalog (cwd from the session's location, mode = which
    /// process to attach) and the service-owned scrollback; the PTY open + fork happen **outside**
    /// the state lock so a slow spawn never blocks other clients. An AI-CLI session resumes its prior
    /// `claude` session by id; a Regular session spawns the platform shell.
    ///
    /// `launch` selects the AI-CLI spawn mode: [`LaunchMode::Fresh`] for a brand-new session
    /// (`SessionCreate` — `claude --session-id`), [`LaunchMode::Resume`] for bringing an existing
    /// durable session back (`SessionStart` — `claude --resume`). Ignored for a Regular/shell primary.
    ///
    /// Resolves `env_include` in the session's own directory (FR-012b, BUG-003).
    pub fn start_session(&self, id: SessionId, launch: LaunchMode) -> io::Result<()> {
        if self.live_session(id).is_some() {
            return Ok(()); // already running — Start is idempotent
        }

        let plan = {
            let inner = self.lock();
            let scrollback = inner.catalog.settings_wire().scrollback_lines;
            inner
                .catalog
                .workspace()
                .sessions
                .iter()
                .find_map(|(project, sessions)| {
                    sessions
                        .iter()
                        .find(|s| s.id == id && !s.archived)
                        .map(|s| SpawnPlan {
                            cwd: s.location.cwd(project),
                            mode: s.mode,
                            provider: s.provider,
                            resumable: matches!(
                                s.lifecycle,
                                SessionLifecycle::InterruptedResumable
                            ),
                            scrollback,
                        })
                })
        };
        let Some(plan) = plan else {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                "no such session in the catalog",
            ));
        };

        // FR-010 / T076: is the CLI actually there? Checked again *here*, at launch, and not only
        // when the choice was offered — the offer-time check answers a different question at a
        // different moment, and a CLI can be uninstalled between the two (research R11).
        //
        // Reported rather than attempted: spawning a binary that is not on `PATH` fails with an
        // `ENOENT` the user cannot act on, and it would spend the crash-loop budget doing it.
        if plan.mode == TerminalMode::AiCli {
            let provider = plan.provider.provider();
            if !provider.is_available() {
                let reason = missing_cli_reason(
                    provider.display_name(),
                    &std::env::var("MICOLD_IMAGE_REFERENCE").unwrap_or_default(),
                    launch,
                );
                tracing::warn!(session = %id.0, cli = provider.command(), "AI CLI not on PATH; not starting");
                self.lock().start_failures.insert(id, reason.clone());
                return Err(io::Error::new(io::ErrorKind::NotFound, reason));
            }

            // FR-008 / T046: the CLI is installed, but is the conversation still there? A row the
            // user is clicking Start on was offered because a conversation existed at startup; it
            // can be deleted afterwards, by the CLI or by hand, and nothing tells us.
            //
            // Gated on `resumable` — the record having been marked at startup — for two reasons.
            // It is the only state in which absence is *news*: a session created but never used
            // has no recorded conversation either, and refusing that would break `Fresh`-adjacent
            // resumes of sessions the daemon never saw a transcript for. And it keeps the check to
            // one stat() on the path a user actually clicks.
            //
            // Reported, not silently started fresh: reusing the id for a new conversation would
            // put the user in an empty session wearing the old one's title (spec clarification
            // 2026-08-16).
            if launch == LaunchMode::Resume && plan.resumable {
                let gone = match provider.config_dir() {
                    Some(config) => !provider.has_recorded_conversation(&config, &plan.cwd, id.0),
                    // An unresolvable config directory is ignorance, not evidence of absence —
                    // attempt the resume and let the CLI answer.
                    None => false,
                };
                if gone {
                    let reason = format!(
                        "{} no longer has this conversation. Close this session, or start a new one.",
                        provider.display_name()
                    );
                    tracing::warn!(
                        session = %id.0,
                        cli = provider.command(),
                        "no recorded conversation to resume; not starting"
                    );
                    self.lock().start_failures.insert(id, reason.clone());
                    return Err(io::Error::new(io::ErrorKind::NotFound, reason));
                }
            }
        }

        let size = self.desired_size(id);
        let cwd = plan.cwd.clone();
        let spawned = match plan.mode {
            TerminalMode::AiCli => {
                let mut spec = LaunchSpec {
                    cwd: plan.cwd.clone(),
                    session_id: id.0,
                    provider: plan.provider,
                    mode: launch,
                    env: self.ai_cli_env_for(&plan.cwd, plan.provider),
                };
                // The hook settings file follows the provider's `activity_source`, not the terminal
                // mode: it is `claude`'s mechanism, and `copilot` has no `--settings` flag to hand
                // it to (feature 026, T016a).
                let activity = self.activity_launch_for(id, &mut spec);
                PtySession::spawn_ai_cli(
                    id,
                    &spec,
                    plan.scrollback,
                    size,
                    &activity,
                    &self.terminal_colors,
                )
            }
            TerminalMode::Regular => PtySession::spawn_shell(
                id,
                &plan.cwd,
                &self.env_include_vars_for(&plan.cwd),
                plan.scrollback,
                size,
                &self.terminal_colors,
            ),
        };
        // A refused spawn is recorded like the refusals above (`010` BUG-001, FR-012). Returned
        // bare, it left the record in the state it was created in, so the catalog broadcast that
        // follows every failed start said `starting…` for good. The one refusal a user causes from
        // outside the application — the session's folder renamed or deleted — gets a sentence that
        // names it; anything else is passed through, since it is still better read than lost.
        let session = match spawned {
            Ok(session) => session,
            Err(err) => {
                let reason = if err.kind() == io::ErrorKind::NotFound && !cwd.is_dir() {
                    format!(
                        "This session's folder no longer exists: {}. Restore it, or close this \
                         session.",
                        cwd.display()
                    )
                } else {
                    format!("Couldn't start this session: {err}")
                };
                tracing::warn!(session = %id.0, error = %err, "session spawn refused; not starting");
                self.lock().start_failures.insert(id, reason.clone());
                return Err(io::Error::new(err.kind(), reason));
            }
        };
        // Session-start event with the launch reason (FR-045). No terminal content — id + mode only.
        tracing::info!(session = %id.0, mode = ?plan.mode, ?launch, "session started");
        // It started, so whatever was wrong before is no longer true.
        self.lock().start_failures.remove(&id);
        self.register_session(session);
        // The durable record has to learn that the process exists (FR-006d, BUG-011). Nothing else
        // will tell it: the live registry above is a separate map, and `overlay_live_summaries`
        // projects activity/title/input-serial onto the wire snapshot but not `lifecycle`. Without
        // this the record keeps whatever it held before the spawn — `InterruptedResumable` for a
        // resume, `Starting` for a session created and never advanced — for the whole life of the
        // process, and the client dutifully renders it: `restart` offered for a running agent.
        //
        // Marked here rather than in the `ClientMsg::SessionStart` handler because this is the one
        // place that knows a process now exists; `SessionCreate` and the restart path reach it too.
        // Not persisted, deliberately — lifecycle is runtime state (FR-021) and every durable
        // session loads `Idle`.
        let owner = self.lock().catalog.mark_session_running(id);
        if owner.is_some() {
            // Outside the lock: `broadcast_catalog` takes it. And unconditionally rather than only
            // on a reply, which is what `spawn_session_start` used to do — `SessionStart` carries
            // no reply, so a resume changed the world and announced nothing.
            self.broadcast_catalog();
        }
        Ok(())
    }

    /// Adopt a freshly-spawned session into the live registry as its `Primary` process (attached by
    /// default), starting its input log at serial `0`. Returns the shared handle.
    pub fn register_session(&self, session: PtySession) -> Arc<PtySession> {
        let pty = Arc::new(session);
        let id = pty.id();
        let mut procs = HashMap::new();
        procs.insert(SessionProcess::Primary, new_proc(Arc::clone(&pty), id));
        let mut inner = self.lock();
        inner.sessions.insert(
            id,
            LiveSession {
                procs,
                attached: SessionProcess::Primary,
                input: InputReceiver::new(),
                activity: Activity::new(),
                last_title: None,
                name_stale: true,
                event_log: None,
                respawned_at: None,
            },
        );
        // A fresh FSM starts at `Unknown`, so a retained `Ended` from the previous run would be
        // shadowed anyway — but only for as long as this entry lives. Dropping it here is what
        // stops the next stop-then-start from re-showing the *old* run's ending (BUG-018).
        inner.ended.remove(&id);
        pty
    }

    /// Open the event-log tail for a session this daemon has just started (feature 026, T064 —
    /// FR-018, FR-019).
    ///
    /// Called from the `SessionStart` path after [`Self::start_session`] succeeds, and by nothing
    /// else. **That call site is the enforcement** of "only a supervised session is watched": a
    /// session discovered under FR-014 never reaches this code, so a project holding hundreds of
    /// them schedules no observation work at all (SC-006, SC-009).
    ///
    /// A no-op for a provider whose activity source is not `EventLog` — `claude` pushes to the
    /// loopback hook receiver instead, and that mechanism is untouched by this feature.
    ///
    /// Everything it needs comes from the session's own durable record, so nothing here re-decides
    /// which CLI a session runs. A failure to open the watch is logged and dropped: the session
    /// runs, its badge stays `Unknown`, and nothing else is affected — the same posture as a hook
    /// settings file that could not be written.
    ///
    /// **Blocking** (it resolves a config directory and registers a watch), so the caller runs it
    /// off the async runtime, alongside the spawn it follows.
    pub fn open_event_log_tail(self: &Arc<Self>, id: SessionId) {
        // The provider and cwd from the record, under the lock and nothing more.
        let Some((provider, cwd)) = ({
            let inner = self.lock();
            inner
                .catalog
                .workspace()
                .sessions
                .iter()
                .find_map(|(project, sessions)| {
                    sessions
                        .iter()
                        .find(|s| s.id == id)
                        .map(|s| (s.provider, s.location.cwd(project)))
                })
        }) else {
            return;
        };

        let provider = provider.provider();
        let Some(config_dir) = provider.config_dir() else {
            return;
        };
        // Each log-shaped source brings its own vocabulary. Pi's log is written by the component
        // this daemon injects at spawn (feature 029, FR-012c); with that component declined
        // (FR-012e) nothing writes it, so no watch is opened and the badge stays `Unknown`.
        let (path, map): (PathBuf, crate::event_log::LineMapper) =
            match provider.activity_source(&config_dir, &cwd, id.0) {
                ActivitySource::EventLog { path } => (path, crate::activity::copilot_event),
                ActivitySource::Extension { log } => {
                    if !self.lock().catalog.pi_activity_component() {
                        return;
                    }
                    (log, crate::activity::pi_event)
                }
                ActivitySource::Hooks | ActivitySource::None => return,
            };

        let state = Arc::clone(self);
        match crate::event_log::EventLogTail::open(path, map, move |event| {
            // A direct push, not a queued one: the badge moves as the line lands, and SC-005's
            // one-second budget is spent on the platform's notification latency rather than on a
            // cadence of ours.
            //
            // Announcing it is part of that, and the part this originally left out (T086): moving
            // the signal only changes what a *snapshot* would say, and no client takes one on its
            // own. Every other producer of this change pushes — the hook receiver on the same
            // `bool` (`hooks.rs`), `drain_signals` by returning it to the supervisor tick — and
            // there is no tick behind a watch, so without this the badge waited for an unrelated
            // broadcast. Guarded on the `bool` for the same reason they are: the event vocabulary
            // is full of lines that are deliberately no-ops, and a snapshot per line of
            // `events.jsonl` would be a broadcast storm for a badge that never moves.
            if state.note_activity(id, event) {
                state.broadcast_catalog();
            }
        }) {
            Ok(tail) => {
                if let Some(live) = self.lock().sessions.get_mut(&id) {
                    live.event_log = Some(tail);
                }
            }
            Err(err) => tracing::warn!(
                session = %id.0,
                %err,
                "could not watch the session's event log; activity will be Unknown"
            ),
        }
    }

    /// Remove a session (all its processes) from the live registry, returning **every** removed
    /// process handle so the caller can `kill()` them outside the lock. The removed [`LiveSession`]
    /// — whose `Proc` drops each kill+reader-join the PTYs — is dropped **outside** the state lock
    /// so that blocking teardown never stalls other clients (module invariant; mirrors
    /// `close_shell`).
    ///
    /// Returns all processes rather than just the primary (BUG-001) — see `remove_live_by_ids` for
    /// why `Drop` alone is not enough, and note that a session opened straight into Regular Terminal
    /// mode has no primary at all, so the old signature returned nothing for it.
    pub fn remove_session(&self, session: SessionId) -> Vec<Arc<PtySession>> {
        let removed = self.lock().sessions.remove(&session);
        removed
            .map(|s| s.procs.values().map(|p| Arc::clone(&p.pty)).collect())
            .unwrap_or_default()
    }

    /// Supervise every live session whose primary process has exited (US4, FR-005): apply the
    /// crash-loop policy to the catalog, respawn on `Restart`, and drop the live process on
    /// `Stop`/`GiveUp`. Runs on a timer regardless of whether any client is attached — the identity
    /// between attended and unattended handling is the whole point of this user story.
    ///
    /// Locking discipline (module invariant): the policy is applied and respawn plans gathered under
    /// the lock; the blocking PTY spawn and the old-process teardown happen **off** the lock. Returns
    /// the distinct projects whose catalog lifecycle changed, so the caller broadcasts one
    /// `CatalogChanged` per affected project.
    pub fn supervise_exited_sessions(&self) -> Vec<PathBuf> {
        self.supervise_exited_sessions_at(micold_core::clock::now())
    }

    /// [`Self::supervise_exited_sessions`] with the tick's clock reading supplied, so the restart
    /// stability window (`RESTART_STABLE_AFTER`, `005` BUG-004) is tested with injected readings
    /// rather than ten-second sleeps. Every reading within one daemon must come from the same clock.
    pub fn supervise_exited_sessions_at(&self, now: micold_core::clock::Uptime) -> Vec<PathBuf> {
        // Phase 1 — under the lock: classify exits, apply the policy, gather the follow-up work.
        let scrollback;
        let mut changed: Vec<PathBuf> = Vec::new();
        let mut to_drop: Vec<SessionId> = Vec::new();
        let mut to_respawn: Vec<(SessionId, PathBuf, TerminalMode, AiCli)> = Vec::new();
        {
            let mut inner = self.lock();
            scrollback = inner.catalog.settings_wire().scrollback_lines;
            // Partition live primaries into those that have exited and those still alive. The alive
            // set is captured *before* this tick's respawns, so a process respawned this tick is not
            // in it; each carries the reading of its last respawn, for the survivor check below.
            let mut exited: Vec<(SessionId, ExitOutcome)> = Vec::new();
            let mut alive: Vec<(SessionId, Option<micold_core::clock::Uptime>)> = Vec::new();
            for (id, live) in &inner.sessions {
                let Some(proc) = live.procs.get(&SessionProcess::Primary) else {
                    continue;
                };
                if proc.pty.is_alive() {
                    alive.push((*id, live.respawned_at));
                } else {
                    // A reaped-but-unclassifiable exit is treated as a crash so supervision still
                    // runs rather than the session lingering as a dead-but-alive entry.
                    exited.push((
                        *id,
                        proc.pty
                            .exit_outcome()
                            .unwrap_or_else(|| ExitOutcome::crashed("exit status not observed")),
                    ));
                }
            }
            for (id, outcome) in exited {
                match inner.catalog.supervise_session_exit(id, outcome) {
                    Some((project, SupervisionAction::Restart, cwd, mode, provider)) => {
                        // Session exit + restart-attempt event, with the reason (FR-045).
                        tracing::warn!(session = %id.0, reason = "unexpected exit", "session crashed; restarting");
                        changed.push(project);
                        to_respawn.push((id, cwd, mode, provider));
                    }
                    Some((project, SupervisionAction::GiveUp, _, _, _)) => {
                        tracing::error!(session = %id.0, reason = "crash loop", "session gave up after repeated crashes (Failed)");
                        changed.push(project.to_path_buf());
                        Self::note_ended(&mut inner, id, "crash loop");
                        to_drop.push(id);
                    }
                    Some((project, SupervisionAction::Stop, _, _, _)) => {
                        tracing::info!(session = %id.0, reason = "clean exit", "session stopped");
                        changed.push(project.to_path_buf());
                        Self::note_ended(&mut inner, id, "clean exit");
                        to_drop.push(id);
                    }
                    // Session already gone from the catalog (closed concurrently) — just reap the
                    // orphaned live entry, no catalog change to broadcast.
                    None => to_drop.push(id),
                }
            }
            // Shell instances that have stopped since the last tick (`012` FR-008, BUG-003). They
            // are deliberately NOT removed from `procs` — the PTY holds the final screen the user
            // is still looking at — so nothing else marks the transition, and `overlay_live_summaries`
            // would go on reporting the same absence without ever announcing it. Announced once per
            // instance: the marker is what stops a dead shell re-broadcasting on every tick.
            let mut newly_dead: Vec<(SessionId, ShellInstanceId)> = Vec::new();
            for (id, live) in &inner.sessions {
                for (key, proc) in &live.procs {
                    if let SessionProcess::Shell(instance) = key {
                        if !proc.pty.is_alive()
                            && !inner.announced_dead_shells.contains(&(*id, *instance))
                        {
                            newly_dead.push((*id, *instance));
                        }
                    }
                }
            }
            for (id, instance) in newly_dead {
                inner.announced_dead_shells.insert((id, instance));
                if let Some((project, _)) = inner.catalog.workspace().find_session(id) {
                    tracing::info!(session = %id.0, instance = instance.0, "shell instance exited");
                    changed.push(project.to_path_buf());
                }
            }
            // Survivors: a session still alive while marked `Restarting` is healthy again once its
            // respawn has stayed up for `RESTART_STABLE_AFTER` — reset it to `Running`, which clears
            // the crash-loop counter (closes the L5 gap). Not sooner: a respawn seen alive one tick
            // after spawning can still be a CLI failing a second into its startup, and resetting it
            // then restarted that forever (`005` BUG-004). A session never respawned has nothing to
            // wait out.
            for (id, respawned_at) in alive {
                let stable =
                    respawned_at.is_none_or(|at| now.saturating_sub(at) >= RESTART_STABLE_AFTER);
                if !stable {
                    continue;
                }
                if let Some(project) = inner.catalog.mark_running_if_restarting(id) {
                    tracing::info!(session = %id.0, reason = "restart survived", "session recovered; running");
                    changed.push(project.to_path_buf());
                }
            }
        }
        // Phase 2 — off the lock: tear down stopped/failed processes (blocking kill+join in Drop).
        for id in to_drop {
            self.remove_session(id);
        }
        // Phase 3 — off the lock: respawn restart-eligible sessions.
        for (id, cwd, mode, provider) in to_respawn {
            self.respawn_primary(id, cwd, mode, provider, scrollback, now);
        }
        changed.sort();
        changed.dedup();
        changed
    }

    /// End a session's activity FSM and keep its verdict past the live entry (`010` BUG-018).
    ///
    /// Called from the two supervision outcomes that stop a session for good — a clean exit and a
    /// crash-loop give-up — immediately before the live entry is dropped, and given the same one-word
    /// `reason` the tick logs there, so the badge and the log agree about what happened. The
    /// detailed sentence for a give-up is not duplicated here: it lives on
    /// [`SessionLifecycle::Failed`], which the client already reads (BUG-017), and two copies of one
    /// sentence are two things that can disagree.
    ///
    /// The FSM is still the producer — this applies [`ActivityEvent::Ended`] to it and keeps what
    /// it returns, so `Ended`'s absorbing rule decides the result rather than this function.
    fn note_ended(inner: &mut Inner, id: SessionId, reason: &str) {
        let Some(live) = inner.sessions.get_mut(&id) else {
            return;
        };
        live.activity.apply(ActivityEvent::Ended {
            reason: reason.to_string(),
        });
        let signal = live.activity.signal().clone();
        inner.ended.insert(id, signal);
    }

    /// Apply an activity [`ActivityEvent`] to a live session's FSM (US2, T046). Returns `true` if the
    /// derived signal changed, so the caller can push a `CatalogChanged` reflecting the new badge.
    /// A no-op (returns `false`) for an unknown/not-live session — a hook for a session the daemon is
    /// not hosting reports nothing, matching invariant H1 (never invent state).
    pub fn note_activity(&self, session: SessionId, event: ActivityEvent) -> bool {
        let mut inner = self.lock();
        let Some(live) = inner.sessions.get_mut(&session) else {
            return false;
        };
        let before = live.activity.signal().clone();
        live.activity.apply(event);
        let changed = live.activity.signal() != &before;
        live.name_stale |= changed;
        changed
    }

    /// Drain each live session's out-of-band terminal signals into runtime state (US2, T046/T047):
    /// the latest OSC-0 title (debounced against `last_title`) and the braille-spinner edge (fed to
    /// the FSM as Working-only evidence, H1a).
    ///
    /// Cheap and lock-only — it reads atomics/`Mutex<Option<String>>` already populated by the
    /// reader thread, never blocking I/O. **That is why it returns the name changes instead of
    /// recording them** (feature 029, research R2): the supervisor calls this directly on its async
    /// task, and persisting a name means writing a project's state file, which is blocking I/O and
    /// does not belong on the async runtime — let alone on the 250 ms tick path while the state
    /// lock is held. The write is the caller's job, in its own `spawn_blocking` hop, and only when
    /// there is something to write. See [`Self::record_observed_names`].
    pub fn drain_signals(&self) -> DrainedSignals {
        let mut out = DrainedSignals::default();
        let mut guard = self.lock();
        let inner = &mut *guard;
        let workspace = inner.catalog.workspace();
        for (id, live) in inner.sessions.iter_mut() {
            // A spinner glyph seen since the last drain is positive `Working` evidence.
            if let Some(proc) = live.procs.get(&live.attached) {
                if proc.pty.signals().take_spinner() {
                    let before = live.activity.signal().clone();
                    live.activity.apply(ActivityEvent::SpinnerObserved);
                    if live.activity.signal() != &before {
                        out.changed = true;
                        // An activity change here re-arms the live name lookup, exactly as
                        // `note_activity` does for a hook (feature 032, FR-010, C6.3b, research
                        // R9). Without it, a spinner drained *before* the `UserPromptSubmit` hook
                        // leaves the hook with nothing to change, and the session's first turn
                        // waits for the end of the turn to be read — well outside the minute US3
                        // promises.
                        live.name_stale = true;
                    }
                }
            }
            // The name comes from the conversation, so from the AI CLI and nothing else (feature
            // 029, FR-011): the `Primary` of an `AiCli` session. A shell tab is attached to the
            // same session but titles itself `user@host: ~/dir`, and a Regular Terminal session's
            // primary *is* a shell — neither has a conversation to name. And only the part of the
            // title the CLI means as the name counts: not its product name, not its decoration
            // (FR-004). Nor the title a Windows console gives itself, its executable's path.
            let title = workspace
                .find_session(*id)
                .filter(|(_, session)| session.mode == TerminalMode::AiCli)
                .and_then(|(project, session)| {
                    let proc = live.procs.get(&SessionProcess::Primary)?;
                    let title = proc.pty.signals().title()?;
                    if crate::activity::is_console_default_title(&title) {
                        return None;
                    }
                    session
                        .provider
                        .provider()
                        .name_in_terminal_title(&title, &session.location.cwd(project))
                });
            // Debounced: only a real change is a push — and only a real change is a durable
            // write. A spinner cycling through glyph frames on an otherwise stable title produces
            // one change here, not thirty, because the glyph was stripped before the title
            // reached `signals`.
            if title != live.last_title {
                if let Some(name) = title.as_deref() {
                    out.names.push((*id, name.to_string()));
                }
                live.last_title = title;
                out.changed = true;
            }
        }
        out
    }

    /// Record the names [`Self::drain_signals`] observed, durably (feature 029, FR-003).
    ///
    /// **Blocking**: each write persists a project's state file, so this runs in the supervisor's
    /// `spawn_blocking` hop and never on the async runtime.
    ///
    /// A failed write is logged and the rest of the batch continues — one session's bad write does
    /// not abandon the others — and nothing is surfaced to the client. That is deliberate and is
    /// what FR-009 asks for: the label is already updated in memory, so the user sees the right
    /// name either way, and a read-only data directory is not a session failure. `WireLifecycle::
    /// Failed` is about the session's *process*. It is the same posture `adopt_discovered_sessions`
    /// takes, for the same reason.
    pub fn record_observed_names(&self, changes: &[(SessionId, String)]) {
        if changes.is_empty() {
            return;
        }
        let mut inner = self.lock();
        for (id, name) in changes {
            if let Err(err) = inner.catalog.record_session_name(*id, name) {
                tracing::warn!(
                    session = %id.0,
                    %err,
                    "could not persist the session's name; it is still shown"
                );
            }
        }
    }

    /// Respawn a session's primary process after a crash and swap it into the live registry. The
    /// session stays `Restarting { attempts }` — the policy set that, and only an explicit healthy
    /// signal clears it — so a process that dies again right away keeps advancing toward `Failed`.
    ///
    /// On a spawn *failure* (the binary is gone, or the working directory is — `010` BUG-012), the
    /// dead entry is left exactly where it is. That is not an oversight: `supervise_exited_sessions`
    /// classifies by `pty.is_alive()` over the live registry, so a dead primary left in place is
    /// re-observed by the very next tick and counted like any other crash, once per tick, until the
    /// budget runs out and the give-up path drops it *and* announces the change. Counting the
    /// failure here instead and dropping the entry — which this did until `010` BUG-024 — took the
    /// session out of the only collection anything walks, stranding it at `Restarting` with no
    /// process behind it for the life of the daemon.
    fn respawn_primary(
        &self,
        id: SessionId,
        cwd: PathBuf,
        mode: TerminalMode,
        provider: AiCli,
        scrollback: usize,
        now: micold_core::clock::Uptime,
    ) {
        // The viewer's pane did not change size because the process died — come back at the size the
        // session was last given, not at the seed (FR-020a, `006` SC-011).
        let size = self.desired_size(id);
        let spawned = match mode {
            TerminalMode::AiCli => {
                let mut spec = LaunchSpec {
                    env: self.ai_cli_env_for(&cwd, provider),
                    cwd,
                    session_id: id.0,
                    provider,
                    mode: LaunchMode::Resume,
                };
                let activity = self.activity_launch_for(id, &mut spec);
                PtySession::spawn_ai_cli(
                    id,
                    &spec,
                    scrollback,
                    size,
                    &activity,
                    &self.terminal_colors,
                )
            }
            TerminalMode::Regular => PtySession::spawn_shell(
                id,
                &cwd,
                &self.env_include_vars_for(&cwd),
                scrollback,
                size,
                &self.terminal_colors,
            ),
        };
        match spawned {
            Ok(session) => {
                // Swap in the fresh process; the old (dead) one is dropped off the lock. The session
                // stays `Restarting { attempts }` (set by the policy) — it is NOT reset to `Running`
                // here, so a process that crashes again right after respawn keeps advancing the
                // crash-loop counter toward `Failed`. `now` is recorded as the respawn's reading:
                // the tick resets the counter only once the process has outlived
                // `RESTART_STABLE_AFTER` from it (`005` BUG-004).
                let _old = self.swap_primary(id, session, now);
            }
            Err(_) => {
                // Couldn't even respawn. Leave the dead primary in the registry: the next tick sees
                // a session whose process is not alive, which is precisely the case supervision is
                // written for, and counts one crash for it like any other. Removing it here would
                // be the last anything ever heard of this session (BUG-024).
            }
        }
    }

    /// Replace a session's `Primary` process with `session`, returning the displaced [`Proc`] so the
    /// caller drops it **off** the lock. If the session vanished meanwhile (closed concurrently), the
    /// freshly-spawned process is torn down off the lock instead of leaking.
    fn swap_primary(
        &self,
        id: SessionId,
        session: PtySession,
        respawned_at: micold_core::clock::Uptime,
    ) -> Option<Proc> {
        let pty = Arc::new(session);
        let mut inner = self.lock();
        let Some(live) = inner.sessions.get_mut(&id) else {
            drop(inner);
            // `pty` drops here, now that the lock is released: its Drop kills + joins off-lock.
            return None;
        };
        live.respawned_at = Some(respawned_at);
        let old = live
            .procs
            .insert(SessionProcess::Primary, new_proc(pty, id));
        drop(inner);
        old
    }

    /// The *attached* process's PTY handle, if the daemon is hosting the session.
    pub fn live_session(&self, session: SessionId) -> Option<Arc<PtySession>> {
        let inner = self.lock();
        let live = inner.sessions.get(&session)?;
        live.procs.get(&live.attached).map(|p| Arc::clone(&p.pty))
    }

    /// Every one of a session's process PTY handles (for a resize that must reach all of them, not
    /// just the attached one). Empty if the session isn't live.
    pub fn session_ptys(&self, session: SessionId) -> Vec<Arc<PtySession>> {
        self.lock()
            .sessions
            .get(&session)
            .map(|l| l.procs.values().map(|p| Arc::clone(&p.pty)).collect())
            .unwrap_or_default()
    }

    /// Resize a session to the client's visible grid (`ClientMsg::SessionResize`).
    ///
    /// The size is **recorded first**, whether or not the session has any process yet, and every
    /// spawn site seeds from that record (FR-020a; BUG-003 of `006-real-terminal-emulator`). This
    /// method used to be the `SessionResize` arm's inline loop over the live PTYs, which meant a
    /// size arriving before the process — routine, since `SessionStart` is spawned and may wait on a
    /// 60 s environment-include script — was applied to an empty list and lost, leaving the session
    /// at the 100×30 seed until an unrelated window resize produced another one.
    ///
    /// Every one of the session's processes is then resized, not just the attached one, so a later
    /// attach-switch shows a correctly-sized grid too. PTY resizes happen outside the state lock
    /// (the handles are cloned `Arc`s).
    pub fn resize_session(&self, session: SessionId, cols: u16, rows: u16) {
        // A degenerate size is not a size: `PtySession::resize` rejects a zero dimension, and
        // recording one would seed a spawn with it. A client reporting one keeps whatever it last
        // reported for real.
        if cols > 0 && rows > 0 {
            self.lock().sizes.insert(session, (cols, rows));
        }
        for pty in self.session_ptys(session) {
            if let Err(err) = pty.resize(cols, rows) {
                tracing::warn!(session = %session.0, %err, "resize failed");
            }
            // Force the stream to re-frame at the new size even for a process that doesn't redraw
            // on SIGWINCH (the framer treats a size change as structural → full frame).
            pty.signals().mark_dirty();
        }
    }

    /// The size this session's next process should be spawned at — whatever a client last reported
    /// for it (FR-020a). `None` for a session no client has ever sized, where the supervisor's own
    /// seed applies.
    fn desired_size(&self, session: SessionId) -> Option<(u16, u16)> {
        self.lock().sizes.get(&session).copied()
    }

    /// The *attached* process's framer (used by the view-stream task and the scrollback handler).
    pub fn session_framer(&self, session: SessionId) -> Option<Arc<Mutex<Framer>>> {
        let inner = self.lock();
        let live = inner.sessions.get(&session)?;
        live.procs
            .get(&live.attached)
            .map(|p| Arc::clone(&p.framer))
    }

    /// Attach `process` (make it the streamed + driven one). Returns the new attached process's
    /// `(pty, framer)` so the caller can restart the view stream, or `None` if it isn't live.
    pub fn attach_process(
        &self,
        session: SessionId,
        process: SessionProcess,
    ) -> Option<(Arc<PtySession>, Arc<Mutex<Framer>>)> {
        let mut inner = self.lock();
        let live = inner.sessions.get_mut(&session)?;
        if !live.procs.contains_key(&process) {
            return None;
        }
        live.attached = process;
        let p = live.procs.get(&process)?;
        Some((Arc::clone(&p.pty), Arc::clone(&p.framer)))
    }

    /// Open (spawn) a shell instance for a session (feature 011). Idempotent — an already-open
    /// instance is a no-op. The shell shares the session's id (for its frames) but has its own PTY,
    /// input log, and framer. cwd/scrollback come from the catalog; the PTY fork happens outside the
    /// lock.
    pub fn open_shell(&self, session: SessionId, instance: ShellInstanceId) -> io::Result<()> {
        let key = SessionProcess::Shell(instance);
        if self
            .lock()
            .sessions
            .get(&session)
            .is_some_and(|l| l.procs.contains_key(&key))
        {
            return Ok(());
        }
        let (cwd, scrollback) = {
            let inner = self.lock();
            let scrollback = inner.catalog.settings_wire().scrollback_lines;
            let cwd = inner
                .catalog
                .workspace()
                .sessions
                .iter()
                .find_map(|(project, sessions)| {
                    sessions
                        .iter()
                        .find(|s| s.id == session)
                        .map(|s| s.location.cwd(project))
                });
            (cwd, scrollback)
        };
        let Some(cwd) = cwd else {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                "no such session in the catalog",
            ));
        };
        let env = self.env_include_vars_for(&cwd);
        // A second terminal for a session is displayed in the same pane as the first, so it starts
        // at the same size (FR-020a, `006` SC-011).
        let size = self.desired_size(session);
        let pty = Arc::new(PtySession::spawn_shell(
            session,
            &cwd,
            &env,
            scrollback,
            size,
            &self.terminal_colors,
        )?);
        let mut inner = self.lock();
        match inner.sessions.get_mut(&session) {
            Some(live) => {
                live.procs.insert(key, new_proc(pty, session));
            }
            // The session has no live entry — its AI CLI exited, failed to start, or has not been
            // relaunched since the service restarted. Create one around this shell rather than
            // dropping it: registering conditionally meant the spawned `pty` fell out of scope
            // here, its `Drop` killed the child that had just started, and this still returned
            // `Ok(())` — so switching such a session to Regular Terminal mode silently did nothing
            // at all (BUG-001, feature 010-regular-terminal-mode).
            //
            // The AI CLI is deliberately NOT started as a side effect: FR-003 makes the shell
            // depend only on there not already being one, and resuming a conversation the user did
            // not ask for is what switching *to* AI CLI mode is for (FR-005).
            //
            // A `LiveSession` with no `Primary` is sound: teardown removes the whole entry and each
            // `Proc`'s `Drop` kills its child, so nothing leaks, and supervision (which only ever
            // restarts a primary) skips it — shell instances never auto-restart anyway.
            None => {
                let mut procs = HashMap::new();
                procs.insert(key, new_proc(pty, session));
                inner.sessions.insert(
                    session,
                    LiveSession {
                        procs,
                        attached: key,
                        input: InputReceiver::new(),
                        activity: Activity::new(),
                        last_title: None,
                        // A shell-only session has no conversation to name.
                        name_stale: false,
                        // A shell-only session has no AI CLI, so there is nothing to tail.
                        event_log: None,
                        respawned_at: None,
                    },
                );
            }
        }
        Ok(())
    }

    /// Close (kill) a session's shell instance. If it was the attached one, attachment falls back to
    /// `Primary`; the new attached `(pty, framer)` is returned so the caller can restart the stream.
    /// The killed process is dropped *outside* the state lock (its teardown may block).
    pub fn close_shell(
        &self,
        session: SessionId,
        instance: ShellInstanceId,
    ) -> Option<(Arc<PtySession>, Arc<Mutex<Framer>>)> {
        let key = SessionProcess::Shell(instance);
        let (_removed, reattach) = {
            let mut inner = self.lock();
            // A closed instance can be opened again under the same id (that is what restart is), so
            // its death must be announceable a second time (BUG-003).
            inner.announced_dead_shells.remove(&(session, instance));
            let live = inner.sessions.get_mut(&session)?;
            let removed = live.procs.remove(&key);
            let reattach = if live.attached == key {
                live.attached = SessionProcess::Primary;
                live.procs
                    .get(&SessionProcess::Primary)
                    .map(|p| (Arc::clone(&p.pty), Arc::clone(&p.framer)))
            } else {
                None
            };
            (removed, reattach)
        };
        // `_removed` (the killed process) drops here, outside the lock.
        reattach
    }

    /// Drive one input batch into a session's PTY, enforcing the append-only input contract (G2,
    /// protocol.md §7). The [`InputReceiver`] classifies the serial under the lock; the actual PTY
    /// write happens *after* the lock is dropped so it can never stall the shared state.
    ///
    /// - `Apply` — the expected next serial: the bytes are written to the PTY.
    /// - `Lost` — a gap (only possible across a reconnect): surfaced loudly, then the arrived bytes
    ///   are still written, because dropping input that *did* arrive would compound the loss.
    /// - `Stale` — a duplicate/reordered serial: dropped and never written, so the log is never
    ///   reordered or coalesced.
    ///
    /// While a start for this session is in flight (BUG-009, T125), the session does not exist yet
    /// — so input for it is *held*, in arrival order, and replayed by [`Self::finish_start`] the
    /// moment it does. Without this, spawning the start off the connection loop would trade a
    /// visible disconnect for silently swallowed keystrokes, which §7 forbids and BUG-006 already
    /// demonstrated the cost of. Classification is deliberately deferred with the bytes: replaying
    /// in arrival order puts every serial through [`InputReceiver`] exactly as it would have been
    /// had the start been instant.
    pub fn session_input(&self, session: SessionId, serial: u64, bytes: &[u8]) {
        {
            let mut inner = self.lock();
            if let Some(held) = inner.starting.get_mut(&session) {
                held.push((serial, bytes.to_vec()));
                return;
            }
        }
        self.apply_input(session, serial, bytes);
    }

    /// Mark a start as in flight, so input for `session` is held rather than dropped (T125).
    /// Idempotent — a redundant `SessionStart` must not discard input already held.
    pub fn begin_start(&self, session: SessionId) {
        self.lock().starting.entry(session).or_default();
    }

    /// End the in-flight start and replay whatever was typed while it ran, in arrival order (T125).
    ///
    /// Drains repeatedly rather than once: the connection loop keeps appending while the marker is
    /// set, so only an empty buffer observed *under the lock* proves nothing more can arrive before
    /// the marker goes. That last observation removes the marker in the same critical section, so
    /// there is no window in which an input could take the direct path and overtake a held one.
    /// Writes happen outside the lock, as everywhere else.
    pub fn finish_start(&self, session: SessionId) {
        loop {
            let batch = {
                let mut inner = self.lock();
                match inner.starting.get_mut(&session) {
                    Some(held) if held.is_empty() => {
                        inner.starting.remove(&session);
                        return;
                    }
                    Some(held) => std::mem::take(held),
                    None => return, // never begun, or already finished
                }
            };
            for (serial, bytes) in batch {
                self.apply_input(session, serial, &bytes);
            }
        }
    }

    /// The per-session gate serializing starts (T125). Spawning removed the serialization the
    /// connection loop used to provide, and two concurrent starts would both observe "not live" and
    /// spawn a process each. Taken *inside* the spawned task, never on the loop.
    pub fn session_gate(&self, session: SessionId) -> Arc<tokio::sync::Mutex<()>> {
        Arc::clone(self.lock().session_gates.entry(session).or_default())
    }

    fn apply_input(&self, session: SessionId, serial: u64, bytes: &[u8]) {
        let resolved = {
            let mut inner = self.lock();
            inner.sessions.get_mut(&session).and_then(|live| {
                // Classify against the *session-level* input log (matches the client's per-session
                // stamper), then write to whichever process is attached.
                let outcome = live.input.accept(serial);
                // Read *after* `accept`: a stale serial leaves the high-water mark unmoved, so this
                // is the serial the client should have sent — the number a diagnostic needs (T113).
                let expected = live.input.expected();
                let attached = live.attached;
                live.procs
                    .get(&attached)
                    .map(|p| (outcome, expected, Arc::clone(&p.pty)))
            })
        };

        let Some((outcome, expected, pty)) = resolved else {
            // Input for a session the daemon is not hosting: nothing to write. Loud, not silent —
            // this means the client's view diverged from the daemon's (a bug or a lost session).
            tracing::warn!(session = %session.0, "dropping input for an unknown session");
            return;
        };

        match outcome {
            InputOutcome::Apply => {}
            InputOutcome::Lost { missing } => {
                tracing::warn!(
                    session = %session.0,
                    missing,
                    serial,
                    "input loss detected across a reconnect; applying the arrived bytes and resyncing"
                );
            }
            InputOutcome::Stale => {
                // `warn!`, not `debug!` (T113/FR-045a, BUG-006): this branch discards user
                // keystrokes, and at the shipped `MICOLD_LOG=info` a `debug!` here made a total,
                // user-visible loss of input produce no diagnostic anywhere — not in the journal,
                // and not in the FR-046 recent-errors ring, which captures WARN and above. Session
                // id and serials only, never the bytes (FR-047).
                tracing::warn!(
                    session = %session.0,
                    serial,
                    expected,
                    "dropping stale/duplicate input; these keystrokes are discarded"
                );
                return;
            }
        }

        if let Err(err) = pty.write_input(bytes) {
            tracing::warn!(session = %session.0, %err, "failed to write input to the PTY");
        }
    }

    /// The number of currently connected clients (test/observability).
    ///
    /// Reads [`Self::presence`] rather than the length of the client table, so that this and the
    /// idle rule cannot disagree (feature 028, lifecycle contract §2.5: *exactly one* presence
    /// count). The table's length tracked the same two transitions and so agreed by construction —
    /// but two things that agree by construction drift the moment one of them grows a third
    /// mutator, and the drift would show up as a daemon stopping with a window open.
    pub fn client_count(&self) -> usize {
        self.presence().connected()
    }

    /// Whether `project` currently has an attachment (test/observability).
    pub fn is_attached(&self, project: &Path) -> bool {
        self.lock().attachments.contains_key(project)
    }
}
