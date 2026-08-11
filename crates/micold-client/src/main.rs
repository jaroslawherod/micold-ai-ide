//! Micold AI IDE — GUI binary entry point.
//!
//! Adapts the render-free core (`micold_client::app`) to the iced runtime. All state
//! transitions live in the core and are unit-tested there; this layer renders state, performs
//! the feature's I/O at the boundary (filesystem scans, git worktree ops via `GitCli`), talks to
//! the session daemon over its connection (`micold_client::daemon`), and holds the gui-only
//! runtime that cannot live in the pure (Clone/Eq) core `State` — the per-session grid caches,
//! the input stamper, and the daemon outbox.

use iced::time::every;
use iced::{Subscription, Task};
mod shell;

use crate::shell::capabilities::Capabilities;
use micold_client::app::{Message, State};
use micold_client::features::session::SelectKind;
use micold_client::features::worktree_form::{
    BranchSource, ResolutionState, WorktreeForm, WorktreeFormStatus,
};
use micold_client::features::Outcome;
use micold_client::grid::GridCache;
use micold_client::input::SessionInputStamper;
use micold_client::overlay::registry::Closing;
use micold_client::selection::{self, Anchor, SelectGranularity, Selection};
use micold_core::env_include::{self, EnvIncludeOutcome, EnvIncludeResolver, EnvIncludeSnapshot};
use micold_core::frame_probe::{
    FrameProbe, ProbeConfig, Scene, SceneFacts, ENV_VAR as FRAME_PROBE_ENV,
    SCENE_ENV_VAR as FRAME_PROBE_SCENE_ENV,
};
use micold_core::fs_scan::FolderBrowser;
use micold_core::git::Git;
use micold_core::os_theme::OsThemeProbe;
use micold_core::protocol::grid::LineId;
use micold_core::protocol::messages::{
    CatalogSnapshot, ClientMsg, DaemonMsg, OperationResult, SessionProcess, WireLifecycle,
};
use micold_core::selector::{Selector, SelectorStatus};
use micold_core::session::{
    Session, SessionId, SessionLabel, SessionLifecycle, SessionLocation, ShellInstanceId,
    TerminalMode,
};
use micold_core::settings::Settings;

use micold_core::theme::{observe_system_scheme, SystemScheme};
use micold_core::worktree::Worktree;
use micold_core::worktree::{BranchOrigin, CreateMode};
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, OnceLock};
use std::time::{Duration, Instant};

/// How often the OS light/dark preference is polled while the window has input focus
/// (research R4).
const OS_THEME_POLL: Duration = Duration::from_millis(500);

/// How often the OS theme is polled while the window is unfocused. Coarser than
/// [`OS_THEME_POLL`], but never suspended: `window_focused` is *input* focus, and an unfocused
/// window is usually still on screen (second monitor, side-by-side), so suspending the poll
/// left a fully visible window showing the wrong theme indefinitely. Changing the OS theme also
/// means leaving the app, which is exactly what unfocuses it. Kept at 1s so SC-003's
/// "within 1 second" holds whether or not the window happens to hold focus.
const BACKGROUND_OS_THEME_POLL: Duration = Duration::from_secs(1);

/// The binary's application state: the pure core plus gui-only runtime handles.
/// A mutating RPC the client has sent to the daemon and is awaiting a reply for (T055). Tracked per
/// correlation `req` so the reply can be matched, a duplicate submission avoided, and — if the
/// connection drops before the reply arrives — the user can be told the outcome is unknown (FR-031/035).
#[derive(Debug, Clone)]
enum PendingOp {
    /// A `SessionCreate`; on success the daemon-assigned session is selected + viewed. Further
    /// variants (project/session-delete) are added as each mutation domain is migrated.
    CreateSession,
    DeleteSession,
    WorktreeCreate(String),
    WorktreeDelete(String),
    /// A read-only `BranchPreflight` (feature 016). Carries what the reply needs to continue:
    /// the project it was asked about, the derived names, whether the branch came from the
    /// picker, and the remote the user named by picking that row — so nothing is recomputed when
    /// the answer lands. `project` is what makes a reply that outlived its form (cancelled, or
    /// the user switched project) detectable instead of acted upon.
    BranchPreflight {
        project: PathBuf,
        names: micold_core::naming::DerivedNames,
        picked: bool,
        preferred_remote: Option<String>,
    },
    /// A read-only `BranchList` for the existing-branch picker (feature 016).
    BranchList {
        project: PathBuf,
    },
    WorktreeRename(String),
    ProjectAdd,
    ProjectRemove,
    ProjectRename,
    /// A `SettingsSet` (FR-012a/FR-012b, BUG-003/T100): the service echoes the persisted result
    /// back as `SettingsChanged` to every connected client (including this one), which is what
    /// actually applies it — this variant exists only so a failure reaches the user and a
    /// disconnect-before-reply resolves to "unknown" like every other mutating RPC (T055).
    SettingsSet,
}

impl PendingOp {
    /// A short verb phrase for an error / unknown-outcome notification ("create the session …").
    fn describe(&self) -> String {
        match self {
            PendingOp::CreateSession => "create the session".into(),
            PendingOp::DeleteSession => "delete the session".into(),
            PendingOp::WorktreeCreate(d) => format!("create the worktree \"{d}\""),
            PendingOp::BranchPreflight { .. } => "check the branch".into(),
            PendingOp::BranchList { .. } => "list the branches".into(),
            PendingOp::WorktreeDelete(d) => format!("delete the worktree \"{d}\""),
            PendingOp::WorktreeRename(d) => format!("rename the worktree \"{d}\""),
            PendingOp::ProjectAdd => "add the project".into(),
            PendingOp::ProjectRemove => "remove the project".into(),
            PendingOp::ProjectRename => "rename the project".into(),
            PendingOp::SettingsSet => "update the settings".into(),
        }
    }
}

struct App {
    core: State,
    /// Every service capability, chosen once at boot (feature 021, T049 — FR-018).
    ///
    /// Held here because four of the eleven sites this replaced were inside `update_inner`, which
    /// takes `&mut App` and nothing else. Cheap to clone (seven `Arc`s), which is what lets the
    /// folder-listing task take a capability instead of constructing one.
    caps: Capabilities,
    /// Per-session renderable grid caches, fed by daemon `GridFrame`s (never Clone/Eq). The client
    /// no longer owns any PTY — sessions live in the daemon (feature 010).
    grids: HashMap<SessionId, GridCache>,
    /// Per-session monotonic input stamper: turns key bytes into ordered `SessionInput` (G2). Held
    /// here (long-lived) so a session's serial is never reset by a daemon detach/reattach.
    stamper: SessionInputStamper,
    /// The active `LineId`-anchored text selection on the displayed session, or `None`.
    selection: Option<Selection>,
    /// How far the displayed session's view is scrolled up into scrollback (0 = live bottom).
    display_offset: usize,
    /// The configured terminal scrollback limit (feature 006), loaded from settings and applied
    /// to newly spawned sessions.
    scrollback_lines: usize,
    /// The overlay currently fading out (rendered from this snapshot until its fade completes),
    /// or `None` when no overlay is leaving.
    dismissing: Option<Closing>,
    /// Whether the OS window currently has input focus (idle-CPU fix). Gates the
    /// terminal/OS-theme poll subscriptions: `true` until the first `Unfocused` event,
    /// which matches iced's behavior of not emitting an initial `Focused` on launch.
    window_focused: bool,
    /// The terminal pane's last-known `(cols, rows)`, reported by `Message::TerminalResized`.
    /// Seeds newly-spawned sessions so they fill the pane immediately instead of starting at the
    /// hardcoded default and waiting for the next window resize to reconcile (bugfix: new
    /// terminal not starting fullscreen).
    last_grid: Option<(u16, u16)>,
    /// The current environment-include settings (feature 011), loaded from settings and mirrored
    /// here the same way `scrollback_lines` is — so the Settings form can be seeded/saved without
    /// re-reading the settings file.
    env_include_enabled: bool,
    env_include_script_path: String,
    env_include_timeout_secs: u64,
    /// Per-directory cache of resolved environment-include snapshots (data-model.md; BUG-002).
    /// Keyed by the directory the sourcing subprocess ran in: a version-manager hook (mise, asdf,
    /// nvm, pyenv, rbenv, …) computes its `PATH` contribution from the sourcing shell's own cwd,
    /// so one directory-agnostic snapshot can never be correct for more than one project.
    /// Sessions launched in the same directory share that directory's cached entry; sessions in
    /// different directories resolve (and cache) independently. Never persisted (FR-008).
    env_include_cache: HashMap<PathBuf, EnvIncludeSnapshot>,
    /// The outcome of the most recently *attempted* resolution — at boot, on a Settings save, or
    /// on a session restart — regardless of which directory it was for (FR-013, BUG-002). A cache
    /// hit in `env_include_vars_for` does NOT update this, since no new attempt was made. Shown as
    /// a single "last attempt" status in Settings, independent of the per-directory `vars` cache
    /// used for merging into a session's own spawn call site.
    env_include_last_outcome: EnvIncludeOutcome,
    /// Handle for sending `ClientMsg`s to the daemon while connected (feature 010). `None` before
    /// the first connect and after a disconnect. The connection itself lives in the
    /// [`micold_client::daemon::connection`] subscription; this is only the send side.
    daemon: Option<micold_client::daemon::Outbox>,
    /// The last catalog snapshot the daemon sent (welcome or `CatalogChanged`). Not yet rendered —
    /// the sidebar/session-list retarget onto it lands with the render switch (T042).
    daemon_catalog: Option<micold_core::protocol::messages::CatalogSnapshot>,
    /// Projects this window has been displaced from by another window's takeover (US5, FR-024),
    /// keyed by project path → the taking-over window's identity. A displaced project is read-only
    /// here (input suppressed, a banner shown) until the user takes it back or reconnects. Cleared
    /// on a fresh connect. Empty in the common single-window case.
    displaced: HashMap<PathBuf, String>,
    /// Whether the daemon connection is currently down (between a `DaemonDisconnected` and the next
    /// `DaemonConnected`). Drives the stale-content banner (FR-027). `daemon.is_none()` also implies
    /// this, but the flag is explicit for clarity at the render site.
    disconnected: bool,
    /// A pending contract-version mismatch (US6, FR-021/022): `(client_version, daemon_version,
    /// daemon_build)`. `Some` while the running daemon's contract differs from ours — drives the
    /// version-mismatch banner and its "restart service" action. Cleared on a successful connect.
    version_mismatch: Option<(u32, u32, String)>,
    /// A pending same-contract build mismatch (US6, FR-022a, BUG-002): `(client_build,
    /// daemon_build)`. `Some` while the running daemon's package version differs from ours despite a
    /// matching wire contract — drives the build-mismatch banner and its "restart service" action.
    /// Cleared on a successful connect. Mutually exclusive with `version_mismatch` in practice (the
    /// handshake reports at most one refusal reason per attempt), but kept as its own field rather
    /// than folded into one enum so each clears independently of the other's precedence in
    /// `connection_status`.
    build_mismatch: Option<(String, String)>,
    /// Correlation-id counter for the client's mutating RPCs (FR-009).
    next_req: u64,
    /// In-flight mutating RPCs keyed by `req` (T055). Lets a reply be matched, a duplicate
    /// submission suppressed, and an in-flight op resolved as *unknown* if the connection drops.
    pending_ops: HashMap<u64, PendingOp>,
    /// The frame-time measurement run, when one was asked for (feature 018, FR-039b — T000z/T076a).
    /// `None` for every ordinary launch, which is what keeps this out of the way of the running
    /// application: with no run configured, [`view`] does not read the clock and [`subscription`]
    /// does not ask for frames.
    ///
    /// `RefCell` because the samples are taken in `view`, which only ever gets `&App`.
    probe: Option<RefCell<FrameProbe>>,
    /// Whether the reference scene has been composed and verified. Until it is, the probe records
    /// nothing — frames spent building the scene are not frames of the scene.
    scene_ready: bool,
    /// Frames spent so far trying to compose the scene, against [`SCENE_COMPOSE_BUDGET`].
    scene_frames: usize,
    /// Ripples the last traversal found mid-animation, for [`Scene::Full`]'s half of the check.
    ///
    /// Observed rather than assumed. The probe asks for a ripple every frame of a `full` run, but
    /// what it records is what the traversal *found* — a scene that claimed a ripple because it had
    /// requested one would put a baseline figure in the full slot, which is the specific mistake
    /// `Scene::check` exists to prevent.
    ///
    /// Shared with the traversal rather than delivered as a message, because a message would make
    /// the probe compose an extra view per frame and count it. See `material::ripple_pulse`.
    ripples_animating: Arc<AtomicUsize>,
    /// Counted frames on which a ripple was animating, against `Scene::RIPPLE_COVERAGE`.
    ///
    /// A count over the run rather than a per-frame check, because the ripple is the one element of
    /// the scene that legitimately blinks — it settles and is pressed again on the frame after.
    /// `Cell` because it is tallied from `view`, which only ever gets `&App`.
    scene_ripple_frames: std::cell::Cell<usize>,
}

/// The measurement run this process was asked for, or `None` for an ordinary launch.
///
/// Parsed once and cached — the value is read from `main`, `boot` and `subscription`, and a run
/// that changed shape between them would be a measurement of nothing in particular. Every decision
/// about what the environment means lives in [`ProbeConfig::from_env_value`], under test in
/// `micold-core/tests/frame_probe.rs`; this only reads the variable and reports a refusal.
fn probe_config() -> Option<ProbeConfig> {
    static CONFIG: OnceLock<Option<ProbeConfig>> = OnceLock::new();
    *CONFIG.get_or_init(|| {
        let raw = std::env::var(FRAME_PROBE_ENV).ok();
        match ProbeConfig::from_env_value(raw.as_deref()) {
            Ok(config) => config,
            // Refused rather than ignored: a typo here would otherwise record an ordinary session
            // as a measurement run, and the resulting figure would be wrong in a way nothing in
            // the recorded procedure could catch.
            Err(message) => {
                eprintln!("{message}");
                std::process::exit(2);
            }
        }
    })
}

/// The reference scene this process was asked to compose and measure, or `None`.
///
/// Parsed once and cached, for the same reason as [`probe_config`]: it is read from several places
/// and a scene that changed shape between them would be a measurement of nothing in particular.
fn probe_scene() -> Option<Scene> {
    static SCENE: OnceLock<Option<Scene>> = OnceLock::new();
    *SCENE.get_or_init(|| {
        let raw = std::env::var(FRAME_PROBE_SCENE_ENV).ok();
        match Scene::from_env_value(raw.as_deref()) {
            Ok(scene) => scene,
            Err(message) => {
                eprintln!("{message}");
                std::process::exit(2);
            }
        }
    })
}

/// Where the composed context menu is opened. Fixed, because "a context menu open over a dialog"
/// has to mean the same thing in all three of §B8's figures — a menu opened by hand lands somewhere
/// slightly different every time, and the difference is in the figure without being in the record.
const SCENE_MENU_AT: (u16, u16) = (400, 300);

/// How many frames the scene gets to compose before the run gives up.
///
/// Generous: it covers spawning the daemon, connecting, creating a session and letting the process
/// come up. Bounded because the alternative is a probe that sits forever in front of a half-composed
/// window with no output at all.
const SCENE_COMPOSE_BUDGET: usize = 3_000;

/// What the window is currently showing, for [`Scene::check`].
///
/// Reads state and counts; decides nothing. Every judgement about whether these add up to the
/// reference scene lives in `micold_core::frame_probe`, under test.
fn scene_facts(app: &App) -> SceneFacts {
    let running_sessions = app
        .core
        .workspace
        .active
        .as_ref()
        .map(|p| app.core.workspace.running_session_count(p))
        .unwrap_or(0);
    SceneFacts {
        worktrees: app.core.worktrees.len(),
        running_sessions,
        dialog_open: micold_client::overlay::registry::open_dialog(&app.core).is_some(),
        context_menu_open: app.core.terminal_context_menu.is_some(),
        ripple_animating: app.ripples_animating.load(Ordering::Relaxed) > 0,
    }
}

/// Keep a ripple running and report how many there are (FR-039b).
///
/// Issued on every frame of a [`Scene::Full`] run — during composition *and* during measurement.
/// A ripple lives about half a second, so one pressed while the scene was being composed would
/// have settled long before the 300th counted frame, and most of the run would be measuring the
/// baseline under the full scene's name. The traversal presses only what it finds idle, so this
/// keeps exactly one going rather than restarting it.
fn pulse_ripples(found: Arc<AtomicUsize>) -> Task<Message> {
    // `discard`, so the traversal yields no message: see `material::ripple_pulse` for why a message
    // here would corrupt the very figure this scene exists to produce.
    iced::advanced::widget::operate(micold_client::ui::ripple_pulse(found)).discard()
}

/// Drive the window toward the reference scene (FR-039b).
///
/// Called after every update while a scene run is in flight. Each step is idempotent except the
/// session create, which is guarded on there being neither a running session nor one already in
/// flight — without that guard this would create a new session on every frame.
fn compose_scene(app: &mut App) -> Task<Message> {
    let facts = scene_facts(app);
    let mut steps = Vec::new();

    // The three elements are composed independently. Sequencing them behind the session — the one
    // step that needs the daemon, and by far the slowest — meant a daemon that never connected
    // silently blocked the dialog and the menu too, and the run gave up reporting all three missing
    // when only one of them was actually stuck.
    if facts.running_sessions == 0 {
        let creating = app
            .pending_ops
            .values()
            .any(|op| matches!(op, PendingOp::CreateSession));
        // Guarded, unlike the other two: a session create is not idempotent, and an unguarded one
        // here would start a fresh session on every frame.
        if !creating && app.daemon.is_some() {
            steps.push(Task::done(Message::SessionStartRequested {
                location: SessionLocation::Default,
            }));
        }
    }
    if !facts.dialog_open {
        steps.push(Task::done(Message::AboutOpened));
    }
    if !facts.context_menu_open {
        let (x, y) = SCENE_MENU_AT;
        steps.push(Task::done(Message::TerminalContextMenuOpened { x, y }));
    }

    Task::batch(steps)
}

/// Report the completed run and end the process (feature 018, FR-039b).
///
/// Exiting is the point: the run has the frames it asked for, and every further frame is the
/// operator reading the summary rather than the scene being measured. The figure goes to stderr so
/// it survives being piped, and is shaped by [`micold_core::frame_probe::Summary::report_line`] so
/// all three of §B8's slots are written to the same precision.
fn report_probe_and_exit(probe: &FrameProbe, app: &App) -> ! {
    match probe.summary() {
        Some(summary) => {
            // The ripple is checked over the whole run rather than per frame, so this is the first
            // point it can be judged — and the last point at which refusing still costs nothing.
            if let Some(scene) = probe_scene() {
                if let Err(why) =
                    scene.check_ripple_coverage(app.scene_ripple_frames.get(), summary.frames)
                {
                    eprintln!("frame probe: {why}");
                    std::process::exit(5);
                }
            }
            eprintln!("frame probe: {}", summary.report_line());
            std::process::exit(0)
        }
        // Unreachable while the run only ends on `is_complete`, which requires at least one counted
        // frame. Stated rather than unwrapped so a future change to that rule cannot turn an empty
        // run into a panic on the way out.
        None => {
            eprintln!("frame probe: no frames were counted; nothing to report.");
            std::process::exit(1)
        }
    }
}

/// Resolve the environment-include snapshot for `cwd` from the given settings values.
///
/// A thin call into the core since T046: the short-circuit and the sourcing both live beside the
/// engine now, and this is the shell picking the real resolver — the one decision that is the
/// shell's to make (FR-017).
fn resolve_env_include(
    resolver: &dyn EnvIncludeResolver,
    enabled: bool,
    script_path: &str,
    timeout_secs: u64,
    cwd: &Path,
) -> EnvIncludeSnapshot {
    env_include::snapshot_for(
        resolver,
        enabled,
        script_path,
        Duration::from_secs(timeout_secs),
        cwd,
    )
}

/// The directory to use whenever a single representative directory is needed synchronously
/// (boot, a Settings save) rather than a specific session's own directory (BUG-002): the active
/// session's own directory if there is one (most relevant to what the user is currently looking
/// at), else the active project's root, else the app process's own current directory.
fn default_resolution_cwd(core: &State) -> PathBuf {
    if let Some(id) = core.active_session {
        if let Some((cwd, _, _)) = session_cwd_mode_and_active_shell(core, id) {
            return cwd;
        }
    }
    if let Some(repo) = core.workspace.active.clone() {
        return repo;
    }
    std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
}

/// Force a fresh re-source of the environment-include script for `cwd`'s cache entry, updating
/// `env_include_last_outcome` to this attempt's outcome (feature 011 FR-007, BUG-002). Called on
/// `TerminalRestartRequested` for the restarted session's own directory (leaving every other
/// cached directory untouched, since only this one needs a fresh attempt), and from
/// `Message::SettingsSaved`'s handler after it clears the whole cache (every cached directory is
/// stale once the enabled/path/timeout settings themselves changed) — the two refresh triggers
/// the spec's Clarifications name.
fn refresh_env_include(app: &mut App, cwd: &Path) {
    let snapshot = resolve_env_include(
        app.caps.env_include(),
        app.env_include_enabled,
        &app.env_include_script_path,
        app.env_include_timeout_secs,
        cwd,
    );
    app.env_include_last_outcome = snapshot.outcome.clone();
    app.env_include_cache.insert(cwd.to_path_buf(), snapshot);
}

impl Drop for App {
    /// On shutdown, disconnect cleanly (`Goodbye`) — the daemon keeps every session running so it
    /// survives the UI closing (FR-001). The client owns no process to kill.
    fn drop(&mut self) {
        if let Some(d) = &self.daemon {
            d.send(ClientMsg::Goodbye);
        }
    }
}

impl App {
    /// The displayed session's grid cache, if any (routes through `active_session`).
    fn attached_grid(&self) -> Option<&GridCache> {
        self.grids.get(&self.core.active_session?)
    }
}

pub fn main() -> iced::Result {
    shell::startup::run()
}

/// Resolve a session's working directory from `repo` (the project root) and its
/// [`SessionLocation`] (feature 010, research.md R2). Thin wrapper around
/// [`SessionLocation::cwd`] — the pure library method is the single authoritative
/// implementation of the `Worktree`/`Default` decision, so it's also what
/// `tests/session_default_location.rs` and `tests/session_title_sync.rs` call directly,
/// rather than each hand-copying this match. All five cwd-resolution call sites in this
/// binary go through this wrapper.
fn session_cwd_for_location(repo: &Path, location: &SessionLocation) -> PathBuf {
    location.cwd(repo)
}

fn update(app: &mut App, message: Message) -> Task<Message> {
    // Snapshot the open dialog BEFORE the reducer runs: closing a dialog clears the state it was
    // drawn from synchronously in the core, so this snapshot is the only way to keep rendering it
    // while it fades out (FR-002/FR-006/FR-012).
    // Which dialog, not merely that one is open: switching straight from one to another has to
    // read as a change here too. Since T037 there is no enum to compare, and the identity the
    // snapshot already carries is the right thing to ask anyway.
    let snapshot_before = Closing::of(&app.core);
    let dialog_before = snapshot_before.as_ref().map(Closing::id);

    let task = update_inner(app, message);

    // Feature 024: an armed reveal scrolls its row into view once there *is* a row to scroll to.
    //
    // Here rather than in the arms that arm it, because arming is a consequence of `active_session`
    // changing and the row may not exist for another frame or two — the incoming project's worktree
    // list is discovered asynchronously, and the viewport reports its height only once laid out.
    // Draining on any message is what lets the scroll wait for both without either arm knowing.
    let task = match reveal_scroll(app) {
        Some(scroll) => Task::batch([task, scroll]),
        None => task,
    };

    // Hand a closing overlay's snapshot to the renderer so its exit has something to draw (US1).
    // The transition itself belongs to `material::Modal`, which reports back with
    // `OverlayTransitionFinished` once it is over; the snapshot is released there.
    let dialog_after =
        micold_client::overlay::registry::open_dialog(&app.core).map(|open| open.id());
    if dialog_before != dialog_after {
        app.dismissing = if dialog_after.is_none() {
            snapshot_before
        } else {
            None
        };
    }

    let Some(scene) = probe_scene() else {
        return task;
    };
    // A scene run (FR-039b). Drive the window toward the scene, and do not let the probe count a
    // single frame until the scene it claims to be measuring is actually the one on screen.
    // The full scene is the baseline *plus a ripple mid-animation*, so the ripple has to outlive
    // composition — see `pulse_ripples`.
    let pulse = if scene == Scene::Full {
        Some(pulse_ripples(Arc::clone(&app.ripples_animating)))
    } else {
        None
    };
    if app.scene_ready {
        return match pulse {
            Some(pulse) => Task::batch([task, pulse]),
            None => task,
        };
    }
    app.scene_frames += 1;
    if scene.check(&scene_facts(app)).is_ok() {
        app.scene_ready = true;
        eprintln!("frame probe: {scene:?} scene composed; measuring.");
        return match pulse {
            Some(pulse) => Task::batch([task, pulse]),
            None => task,
        };
    }
    if app.scene_frames > SCENE_COMPOSE_BUDGET {
        // Loud, and with the reason: an unattended run that quietly measured a half-composed window
        // would produce a figure that looks exactly like a good one.
        let why = scene
            .check(&scene_facts(app))
            .expect_err("the budget is only exceeded while the check is failing");
        eprintln!("frame probe: gave up composing the scene after {SCENE_COMPOSE_BUDGET} frames.");
        eprintln!("{why}");
        std::process::exit(3);
    }
    let mut steps = vec![task, compose_scene(app)];
    steps.extend(pulse);
    Task::batch(steps)
}

/// Drain an armed reveal into a scroll, once there is something to scroll to (feature 024, §6.4).
///
/// Three conditions, and the arm survives all three failing:
///
/// - nothing is armed — the ordinary case, and the reason this is cheap to call on every message;
/// - the projection holds no row for the current session, because the worktree list has not
///   arrived yet (research R7) — scrolling now would use an offset computed from the wrong rows;
/// - the viewport has not been laid out, so its height is still `0` — which means "unknown", never
///   "nothing fits" (contract §6.3).
///
/// Once it does drain, the offset may still be `None`: the row was already fully visible, and
/// FR-009 says a reveal that did not need to move the list must not move it.
fn reveal_scroll(app: &mut App) -> Option<Task<Message>> {
    if !app.core.pending_reveal_scroll
        || app.core.sidebar_viewport_height == 0
        || !app.core.current_session_is_listed()
    {
        return None;
    }
    app.core.pending_reveal_scroll = false;
    let offset = app.core.reveal_scroll_offset()?;
    Some(iced::widget::operation::scroll_to(
        micold_client::ui::SIDEBAR_SCROLL_ID.clone(),
        iced::widget::scrollable::AbsoluteOffset {
            x: 0.0,
            y: offset as f32,
        },
    ))
}

fn update_inner(app: &mut App, message: Message) -> Task<Message> {
    match message {
        // ---- Feature 010: daemon connection lifecycle (binary-owned runtime state) ----
        Message::DaemonConnected {
            outbox,
            catalog,
            settings,
        } => {
            // A fresh connection resyncs from authoritative state (FR-028): clear the transient
            // disconnected/displaced flags. If a project is still held by another window, the
            // re-attach below is refused and the displaced state is re-established from that reply.
            app.disconnected = false;
            app.displaced.clear();
            app.version_mismatch = None;
            app.build_mismatch = None;
            // The daemon is the single writer of settings + sessions; adopt what it reports
            // (FR-012a/FR-012b) — including environment-include, which this client's own
            // boot-time local read may predate (e.g. another window changed it while this one was
            // still starting up). Re-source env-include under the now-authoritative values.
            app.scrollback_lines = settings.scrollback_lines;
            app.env_include_enabled = settings.env_include_enabled;
            app.env_include_script_path = settings.env_include_script_path;
            app.env_include_timeout_secs = settings.env_include_timeout_secs;
            app.env_include_cache.clear();
            let cwd = default_resolution_cwd(&app.core);
            refresh_env_include(app, &cwd);
            reconcile_catalog(&mut app.core, &catalog, false);
            // Adopt the daemon's per-session input position (FR-028a, T111). This process may be a
            // *new* client attached to sessions it did not start — after a package upgrade, or a
            // plain quit-and-reopen — in which case its stamper is empty and starting those counters
            // at 0 would put them behind the daemon, which then discards every keystroke as stale
            // (BUG-006). Part of the same resync as the flags and settings above: the daemon's
            // position is authoritative state, so re-read it rather than assume continuity.
            app.stamper.seed_from_catalog(&catalog);
            app.daemon_catalog = Some(catalog);
            // Attach to the active project and view its active session so the daemon starts
            // streaming grid frames for it (FR-011/FR-016).
            if let Some(project) = app.core.workspace.active_project().map(|p| p.path.clone()) {
                outbox.send(ClientMsg::Attach {
                    project: project.clone(),
                    force: false,
                });
                outbox.send(ClientMsg::SetViewedSession {
                    project,
                    session: app.core.active_session,
                });
            }
            app.daemon = Some(outbox);
            Task::none()
        }
        Message::DaemonEvent(event) => {
            match event {
                DaemonMsg::CatalogChanged { catalog } => {
                    reconcile_catalog(&mut app.core, &catalog, true);
                    // Sessions can appear after connect — created in another window, or resumed —
                    // so seed here too (T111). Absent-only, so this never disturbs a counter this
                    // client is already driving.
                    app.stamper.seed_from_catalog(&catalog);
                    app.daemon_catalog = Some(catalog);
                }
                // A settings mutation reached the service — this client's own `SettingsSet` echoed
                // back, or another window's (FR-011). Sync every service-owned field and re-source
                // env-include, exactly like the local-save path below does for its own change
                // (T100): the enabled/path/timeout settings may have changed, so every previously
                // cached directory's snapshot is stale.
                DaemonMsg::SettingsChanged { settings } => {
                    app.scrollback_lines = settings.scrollback_lines;
                    app.env_include_enabled = settings.env_include_enabled;
                    app.env_include_script_path = settings.env_include_script_path;
                    app.env_include_timeout_secs = settings.env_include_timeout_secs;
                    app.env_include_cache.clear();
                    let cwd = default_resolution_cwd(&app.core);
                    refresh_env_include(app, &cwd);
                }
                // Fetched scrollback: resolve + insert into the session's grid cache (FR-016/017).
                DaemonMsg::ScrollbackResponse {
                    session,
                    lines,
                    styles,
                    hyperlinks,
                    ..
                } => {
                    if let Some(grid) = app.grids.get_mut(&session) {
                        grid.apply_scrollback(&lines, &styles, &hyperlinks);
                    }
                }
                // A mutating request we correlated resolved. For most ops the resulting state has
                // already arrived via the `CatalogChanged` push (reconcile_catalog), so there is
                // nothing to do; a `SessionCreate` additionally names the daemon-assigned id so we
                // select + view it.
                DaemonMsg::OperationOk { req, result } => match app.pending_ops.remove(&req) {
                    Some(PendingOp::CreateSession) => {
                        if let OperationResult::SessionCreated { session } = result {
                            app.core.update(Message::SessionSelected(session));
                            view_and_start(app, session);
                            // No follow-up focus message: `SessionSelected` focuses the terminal in
                            // the reducer and nothing releases it on the same click any more
                            // (feature 023). The re-assertion this replaced existed to win a race
                            // against a `TerminalFocusReleased` published by the very click that
                            // selected — the shape FR-008a forbids, since it puts the keyboard
                            // somewhere the user did not ask for, however briefly.
                        }
                    }
                    // A worktree create succeeded: close the form. The worktree itself arrives via
                    // the `CatalogChanged` push (reconcile), so the constructed value here is only to
                    // reuse `WorktreeCreated`'s form-closing logic (it dedups by dir_name).
                    Some(PendingOp::WorktreeCreate(dir_name)) => {
                        if let Some(repo) = app.core.workspace.active.clone() {
                            let path = repo.join(".claude/worktrees").join(&dir_name);
                            app.core.update(Message::WorktreeCreated(
                                micold_core::worktree::Worktree {
                                    dir_name,
                                    path,
                                    branch: None,
                                    status: micold_core::worktree::WorktreeStatus::Valid,
                                },
                            ));
                        }
                    }
                    // Feature 016: the pre-flight answer decides what happens next. A free name
                    // creates straight away (FR-025 — no extra prompt); anything else either
                    // resolves itself (the user already named the branch by picking it) or opens
                    // the reuse/overwrite prompt.
                    //
                    // The answer is only acted on while the form that asked for it is still open,
                    // still editing, and still pointed at the same project: cancelling the form
                    // (or switching project) while the RPC is in flight must not go on to create
                    // a worktree the user backed out of.
                    Some(PendingOp::BranchPreflight {
                        project: asked_for,
                        names,
                        picked,
                        preferred_remote,
                    }) => {
                        if let OperationResult::BranchPreflight { situation } = result {
                            let form_open = app
                                .core
                                .worktree_form
                                .as_ref()
                                .is_some_and(|f| f.status == WorktreeFormStatus::Editing);
                            if let Some(project) = app
                                .core
                                .workspace
                                .active
                                .clone()
                                .filter(|p| form_open && *p == asked_for)
                            {
                                match &situation {
                                    micold_core::worktree::BranchSituation::Free => {
                                        send_worktree_create(
                                            app,
                                            project,
                                            names,
                                            CreateMode::NewBranch,
                                        );
                                    }
                                    // Picking a branch IS the intent to use it, so an available
                                    // candidate needs no prompt (contract branch-picker.md §5). It can
                                    // never mean overwrite.
                                    _ if picked => {
                                        match WorktreeForm::mode_for(
                                            &situation,
                                            preferred_remote.as_deref(),
                                        ) {
                                            Some(mode) => {
                                                send_worktree_create(app, project, names, mode)
                                            }
                                            None => app.core.update(
                                                Message::AddWorktreeConflictDetected(situation),
                                            ),
                                        }
                                    }
                                    _ => app
                                        .core
                                        .update(Message::AddWorktreeConflictDetected(situation)),
                                }
                            }
                        }
                    }
                    // Same staleness guard: a listing for a project that is no longer the active
                    // one must not populate the picker of a form opened on a different repo.
                    Some(PendingOp::BranchList { project: asked_for }) => {
                        if let OperationResult::BranchList { candidates } = result {
                            if app.core.workspace.active.as_deref() == Some(asked_for.as_path()) {
                                app.core
                                    .update(Message::AddWorktreeBranchesListed(candidates));
                            }
                        }
                    }
                    // Feature 013 (FR-015): the worktree directory and its sessions are already
                    // gone by this point (that half always succeeds here) — a failed branch
                    // deletion is reported as a distinct, non-blocking notice rather than
                    // silently discarded, so choosing "delete the branch" that git then refuses
                    // (e.g. unreachable commits) doesn't look like it silently kept the branch.
                    Some(PendingOp::WorktreeDelete(dir)) => {
                        if let OperationResult::WorktreeDeleted {
                            branch_delete_failed,
                            leftovers,
                        } = result
                        {
                            if branch_delete_failed {
                                app.core.notify_error(format!(
                                    "The worktree \"{dir}\" was removed, but its branch could not \
                                     be deleted (it may hold commits not present elsewhere)."
                                ));
                            }
                            // FR-023c/FR-023d: partial success. Lead with what *did* happen —
                            // the worktree is gone — so this does not read as a failed delete,
                            // then name the paths and their owner, which is the only part the
                            // user can act on. A bare error code named nothing and left them
                            // with a tree of tens of thousands of files to search (BUG-002).
                            if !leftovers.is_empty() {
                                app.core.notify_error(format!(
                                    "The worktree \"{dir}\" was removed, but {}. You can delete \
                                     {} yourself once you have permission to.",
                                    describe_leftovers(&leftovers),
                                    if leftovers.len() == 1 { "it" } else { "them" },
                                ));
                            }
                        }
                    }
                    _ => {}
                },
                // FR-024: a stage push names the step in flight. Peeked, not removed — the
                // operation is still running and its terminal reply still needs the pending op.
                DaemonMsg::OperationProgress { req, stage, detail } => {
                    if matches!(
                        app.pending_ops.get(&req),
                        Some(PendingOp::WorktreeCreate(_))
                    ) {
                        app.core
                            .update(Message::WorktreeCreateStageChanged(stage, detail));
                    }
                }
                DaemonMsg::OperationError {
                    req,
                    message,
                    detail,
                    ..
                } => {
                    match app.pending_ops.remove(&req) {
                        // A failed worktree create shows in the form (keeps it open to retry), not a
                        // toast — mirroring the old local-create failure path. `detail` carries git's
                        // own stderr verbatim (feature 010, FR-006/SC-003): for a submodule fetch
                        // failure this is normally the only place that names which submodule failed
                        // and why (auth/network/unreachable commit) — `message` alone is the generic
                        // "git failed to create the worktree".
                        Some(PendingOp::WorktreeCreate(_)) => {
                            app.core.update(Message::WorktreeCreateFailed(
                                worktree_create_error_text(message, detail),
                            ));
                        }
                        // Feature 016: both branch queries back the open form, so their failures
                        // belong on its own error line. A notification would be raised into the
                        // surface the modal's scrim covers — invisible — and for the listing the
                        // empty picker would then wrongly claim the repository has no branches.
                        Some(PendingOp::BranchPreflight { .. }) => {
                            app.core.worktree_error =
                                Some(format!("Could not check the branch: {message}"));
                        }
                        Some(PendingOp::BranchList { .. }) => {
                            app.core.worktree_error =
                                Some(format!("Could not list branches: {message}"));
                        }
                        Some(op) => app
                            .core
                            .notify_error(format!("Couldn't {}: {message}", op.describe())),
                        None => {}
                    }
                }
                // Diagnostics replies (Phase 10, FR-046): surface as notices.
                DaemonMsg::LogLocation { path, sink, .. } => {
                    let where_ = match path {
                        Some(p) => format!("a file at {}", p.display()),
                        None => format!("{sink:?}"),
                    };
                    app.core
                        .notify_info(format!("The session service logs to {where_}."));
                }
                DaemonMsg::RecentErrors { entries, .. } => {
                    if entries.is_empty() {
                        app.core
                            .notify_info("The session service reports no recent errors.");
                    } else {
                        let latest = entries.last().unwrap();
                        app.core.notify_error(format!(
                            "The session service reported {} recent issue(s); most recent: [{}] {}",
                            entries.len(),
                            latest.level,
                            latest.message
                        ));
                    }
                }
                // Another window took over a project we held (US5, FR-024). Mark it read-only here —
                // input is suppressed and a "take over" banner is shown — but never terminate.
                DaemonMsg::Displaced { project, by } => {
                    app.displaced.insert(project, by);
                }
                // A (re)attach was refused. `ProjectBusy` means another window holds it: surface the
                // same take-over banner as a live displacement, naming the current holder.
                DaemonMsg::Refused {
                    reason:
                        micold_core::protocol::messages::RefusalReason::ProjectBusy {
                            project,
                            holder,
                            ..
                        },
                } => {
                    app.displaced.insert(project, holder);
                }
                // An attach this window asked for was accepted (FR-024a). This is the fact that
                // falsifies a recorded displacement: the daemon decides who holds a project, and it
                // has just confirmed we do. Clearing here — rather than only on a full reconnect or
                // the banner's "Take over" button — is what lets a window that was once refused go
                // back to a project after the holder released it and simply type into it. Without
                // it, `displaced` is a latch: written by every refusal, cleared by almost nothing,
                // so the window renders a project it owns while suppressing its own input above a
                // banner naming a window that may have exited (BUG-007).
                //
                // `sessions` is deliberately ignored. It is built from `DaemonState::sessions_for`,
                // the raw durable projection with **no** live overlay, so its `activity` is always
                // `Unknown`, its labels lag the terminal title, and its `input_serial` is `0` even
                // for a session the daemon has been driving for hours — adopting it would re-create
                // BUG-006. The authoritative view arrives immediately after, as the `CatalogChanged`
                // that `refresh_worktrees_and_send` sends on the heels of every `Attached`, and that
                // one *is* overlaid.
                DaemonMsg::Attached { project, .. } => {
                    app.displaced.remove(&project);
                }
                // Other control messages (Pong) are consumed as their flows land.
                _ => {}
            }
            Task::none()
        }
        Message::DaemonGridFrame(frame) => {
            // Feed the frame into the session's grid cache; the pane renders from it (T042).
            let session = frame.session;
            let (old_top, new_top, oldest) = {
                let cache = app.grids.entry(session).or_default();
                let old = cache.viewport_top().0;
                cache.apply(&frame);
                (old, cache.viewport_top().0, cache.oldest_available().0)
            };
            // Hold a scrolled-back view in place as new output advances the viewport: without this,
            // `line_at_row = viewport_top - display_offset + row` would slide the shown lines toward
            // the live bottom on every output tick (FR-016). Only the displayed session, only while
            // scrolled up; clamp to the retained history.
            if app.core.active_session == Some(session)
                && app.display_offset > 0
                && new_top > old_top
            {
                let advanced = (new_top - old_top) as usize;
                let history = (new_top - oldest).max(0) as usize;
                app.display_offset = (app.display_offset + advanced).min(history);
            }
            Task::none()
        }
        Message::DaemonDisconnected => {
            app.daemon = None;
            // Content on screen is now stale; the banner says so (FR-027). The subscription is
            // already auto-reconnecting with backoff.
            app.disconnected = true;
            // Any request still in flight will never get a reply on this connection (`req`s are
            // per-connection). Resolve each to an explicit *unknown* outcome — never a silent
            // success or failure — and reconcile against authoritative state on reconnect
            // (FR-031/035). The daemon applied its mutation atomically before replying, so the fresh
            // welcome catalog is the source of truth for whether it actually took effect.
            for (_req, op) in app.pending_ops.drain() {
                app.core.notify_error(format!(
                    "The session service disconnected before confirming the request to {} — \
                     it may or may not have taken effect; reconnecting will show the current state.",
                    op.describe()
                ));
            }
            Task::none()
        }
        Message::DaemonConnectFailed(reason) => {
            app.disconnected = true;
            app.core
                .notify_error(format!("Could not connect to the session daemon: {reason}"));
            Task::none()
        }
        // The user chose to take the active project back after being displaced (FR-024): re-attach
        // with force, which displaces the current holder, and re-view its active session.
        Message::ConnectionTakeoverRequested => {
            if let (Some(project), Some(d)) =
                (app.core.workspace.active.clone(), app.daemon.clone())
            {
                app.displaced.remove(&project);
                d.send(ClientMsg::Attach {
                    project: project.clone(),
                    force: true,
                });
                d.send(ClientMsg::SetViewedSession {
                    project,
                    session: app.core.active_session,
                });
            }
            Task::none()
        }
        // The daemon refused us on a contract mismatch (US6, FR-021): record it so the banner can
        // name both versions and offer the restart action. The connection subscription keeps
        // retrying in the background; each retry re-sets this identically until the user acts.
        Message::DaemonVersionMismatch {
            client,
            daemon,
            daemon_build,
        } => {
            app.version_mismatch = Some((client, daemon, daemon_build));
            Task::none()
        }
        // Same contract, different package version (US6, FR-022a, BUG-002): record it so the banner
        // can name both builds and offer the restart action, distinct from a contract mismatch.
        Message::DaemonBuildMismatch {
            client_build,
            daemon_build,
        } => {
            app.build_mismatch = Some((client_build, daemon_build));
            Task::none()
        }
        // "Restart service" (FR-022/022a): stop the mismatched daemon by its recorded pid. A
        // mismatched client can't send it a control message, so termination is the version-agnostic
        // stop. Once it exits, the auto-reconnect loop finds nothing listening and spawns a matching
        // daemon; previously-live sessions then reload as interrupted-resumable (FR-006a). Live
        // processes are lost — we say so — but the durable sessions survive.
        Message::ConnectionRestartServiceRequested => {
            app.version_mismatch = None;
            app.build_mismatch = None;
            app.core.notify_info(
                "Restarting the session service — running processes are stopped, but your \
                 sessions are preserved and can be resumed.",
            );
            Task::perform(
                async {
                    tokio::task::spawn_blocking(|| {
                        let endpoint = micold_core::endpoint::resolve()?;
                        micold_core::spawn::stop_running_daemon(&endpoint)
                    })
                    .await
                    .map_err(|e| e.to_string())?
                    .map_err(|e: std::io::Error| e.to_string())
                },
                |r: Result<bool, String>| match r {
                    Ok(_) => Message::NoOp,
                    Err(e) => Message::DaemonConnectFailed(format!(
                        "could not stop the mismatched service: {e}"
                    )),
                },
            )
        }
        // Make sessions survive logout (US7, FR-038; Linux only). Runs off-thread — it spawns
        // `loginctl`/`systemctl` — and reports the outcome as a toast. Never enabled by install.
        Message::LogoutSurvivalRequested => Task::perform(
            async {
                tokio::task::spawn_blocking(|| {
                    let endpoint = micold_core::endpoint::resolve().map_err(|e| {
                        micold_core::logout_survival::SurvivalOutcome::Failed(e.to_string())
                    })?;
                    Ok(micold_core::logout_survival::enable(&endpoint))
                })
                .await
                .unwrap_or_else(|e| {
                    Err(micold_core::logout_survival::SurvivalOutcome::Failed(
                        e.to_string(),
                    ))
                })
            },
            |r: Result<
                micold_core::logout_survival::SurvivalOutcome,
                micold_core::logout_survival::SurvivalOutcome,
            >| {
                let outcome = r.unwrap_or_else(|e| e);
                Message::LogoutSurvivalOutcome(outcome.user_message())
            },
        ),
        Message::LogoutSurvivalOutcome(message) => {
            app.core.notify_info(message);
            Task::none()
        }
        // Ask the daemon where it logs and for its recent errors (Phase 10, FR-046). The replies
        // arrive as `LogLocation`/`RecentErrors` events, shown as notices. Uncorrelated: only the
        // latest answer matters, so no pending-op bookkeeping is needed.
        Message::DiagnosticsRequested => {
            if let Some(d) = &app.daemon {
                let req = app.next_req;
                app.next_req += 2;
                d.send(ClientMsg::LogLocationRequest { req });
                d.send(ClientMsg::RecentErrorsRequest {
                    req: req + 1,
                    limit: 20,
                });
            } else {
                app.core
                    .notify_error("Not connected to the session service — no diagnostics to show.");
            }
            Task::none()
        }

        // The closing dialog has finished animating out; its snapshot has served its purpose.
        Message::OverlayTransitionFinished => {
            app.dismissing = None;
            Task::none()
        }
        Message::ProjectSelectorOpened => {
            let dir = start_dir();
            app.core.clear_for_dialog();
            app.core.selector = Some(Selector::open_at(dir.clone()));
            scan_task(app.caps.browser(), dir)
        }
        Message::SelectorNavigatedInto(_) | Message::SelectorNavigatedUp => {
            app.core.update(message);
            match &app.core.selector {
                Some(selector) if selector.status == SelectorStatus::Loading => {
                    scan_task(app.caps.browser(), selector.current_dir.clone())
                }
                _ => Task::none(),
            }
        }
        // Open the chosen folder as a project — but only if it is a git repository (FR-001a).
        Message::FolderChosen(path) => {
            // Close the picker BEFORE the git gate. Notifications render inside `base`, which
            // every modal wraps behind its scrim, so a refusal reported while the selector was
            // still open would be dimmed out of view.
            app.core.selector = None;
            if !app.caps.git().is_repo_root(&path) {
                app.core.update(Message::ProjectOpenRefused(
                    "Only git repositories can be opened as projects.".to_string(),
                ));
                return Task::none();
            }
            // Switch without tearing down the outgoing project's sessions (feature 008, BS-1).
            // `open_or_activate` moves `active` to the new project, so capture the outgoing
            // foreground FIRST (I1), then finish the switch bookkeeping for the new project. The
            // in-memory `open_or_activate` gives instant UI; local git discovery seeds the worktree
            // list until the daemon's post-attach refresh reconciles it (T055).
            let previous = app.core.workspace.active.clone();
            app.core.record_foreground();
            app.core
                .workspace
                .open_or_activate(path.clone(), app.caps.scanner());
            app.core.restore_after_activation(&path);
            app.core
                .set_worktrees(discover_worktrees(app.caps.git(), &path));
            app.core.worktree_error = None;
            log_foreground_choice(app, &path);
            // The daemon is the single writer: tell it to learn this project (persist + discover),
            // and switch this client's attachment to it. No local `persist()`, no local
            // transcript-reconcile — sessions come from the daemon catalog via reconcile_catalog.
            let add_path = path.clone();
            send_op(app, PendingOp::ProjectAdd, move |req| {
                ClientMsg::ProjectAdd {
                    req,
                    path: add_path,
                }
            });
            switch_daemon_attachment(app, previous, &path);
            Task::none()
        }
        Message::KnownProjectReopened(path) => {
            app.core.workspace.refresh_availability(app.caps.scanner());
            // Non-destructive switch: keep the outgoing project's sessions running in the
            // background and restore the target project's foreground (feature 008, BS-1/BS-3).
            let previous = app.core.workspace.active.clone();
            if app.core.switch_active(&path) {
                app.core
                    .set_worktrees(discover_worktrees(app.caps.git(), &path));
                log_foreground_choice(app, &path);
                // Already a known project (no ProjectAdd); just move the daemon attachment.
                switch_daemon_attachment(app, previous, &path);
            }
            Task::none()
        }
        // Project rename (feature 001, FR-017): the daemon is the single writer, so route it through
        // the `ProjectRename` RPC (T055). The pure-core update applies it in memory (instant feedback
        // + validation + closes the overlay); the daemon persists it and reconciles other windows.
        // No local `persist()`.
        Message::RenameConfirmed => {
            let draft = app
                .core
                .rename_draft
                .as_ref()
                .map(|d| (d.path.clone(), d.text.trim().to_string()));
            app.core.update(Message::RenameConfirmed);
            // Only send if the pure update accepted it (a rejected name leaves the draft in place).
            if app.core.rename_draft.is_none() {
                if let Some((path, display_name)) = draft {
                    if !display_name.is_empty() {
                        send_op(app, PendingOp::ProjectRename, move |req| {
                            ClientMsg::ProjectRename {
                                req,
                                path,
                                display_name,
                            }
                        });
                    }
                }
            }
            Task::none()
        }
        // Forget a project (feature 014): route through the daemon's `ProjectRemove`, which stops the
        // project's sessions, drops its records, and deletes its per-project state file (FR-005/010),
        // then broadcasts the pruned catalog (T055). The pure reducer drops the record + clears the
        // active pointer in memory for instant feedback; nothing inside the project folder is touched.
        Message::ProjectForgetConfirmed => {
            if let Some(path) = app.core.forget_target.clone() {
                app.grids.retain(|id, _| {
                    !app.core
                        .workspace
                        .session_ids_of_project(&path)
                        .contains(id)
                });
                let remove_path = path.clone();
                send_op(app, PendingOp::ProjectRemove, move |req| {
                    ClientMsg::ProjectRemove {
                        req,
                        path: remove_path,
                    }
                });
                // Release this client's attachment on the project it is forgetting.
                if let Some(d) = &app.daemon {
                    d.send(ClientMsg::Detach { project: path });
                }
            }
            app.core.update(Message::ProjectForgetConfirmed);
            Task::none()
        }
        // Worktree rename (feature 008, FR-014/FR-015): the daemon is the single writer of the
        // display-name override, so route it through the `WorktreeRename` RPC (T055). The pure-core
        // update still applies it in memory for instant feedback (validated + closes the overlay);
        // the daemon persists it and reconciles a second window via `CatalogChanged`. No local
        // `persist()` — the daemon owns the durable file now.
        Message::WorktreeRenameConfirmed => {
            let draft = app
                .core
                .worktree_rename_draft
                .as_ref()
                .map(|d| (d.dir_name.clone(), d.text.trim().to_string()));
            let project = app.core.workspace.active.clone();
            app.core.update(Message::WorktreeRenameConfirmed);
            // Only send if the pure update accepted it (a rejected name leaves the draft in place).
            if app.core.worktree_rename_draft.is_none() {
                if let (Some((dir_name, display_name)), Some(project)) = (draft, project) {
                    if !display_name.is_empty() {
                        send_op(
                            app,
                            PendingOp::WorktreeRename(dir_name.clone()),
                            move |req| ClientMsg::WorktreeRename {
                                req,
                                project,
                                dir_name,
                                display_name,
                            },
                        );
                    }
                }
            }
            Task::none()
        }
        Message::ThemePreferenceChanged(_) | Message::ThemeModeCycled => {
            app.core.update(message);
            shell::persist::persist_settings(app.caps.settings(), &mut app.core);
            Task::none()
        }
        // Validate the form, then create the worktree (incl. any submodule fetch) via git,
        // off the update() thread so a slow fetch doesn't freeze the UI (feature 010,
        // research R4). AddWorktreeSubmitted/WorktreeCreated/WorktreeCreateFailed keep their
        // existing meaning; WorktreeCreateStarted is dispatched first so the form can show it.
        // Submitting classifies the target branch first (feature 016, FR-001). A free name
        // creates immediately, exactly as before; anything else becomes a decision for the user
        // rather than the dead-end "a branch with that name already exists" error.
        Message::AddWorktreeSubmitted => {
            app.core.update(Message::AddWorktreeSubmitted);
            let Some(form) = app.core.worktree_form.clone() else {
                return Task::none();
            };
            if form.status != WorktreeFormStatus::Editing || form.resolution.is_prompting() {
                return Task::none(); // create in flight, or a prompt is already open.
            }
            // The form stays `Editing` while the pre-flight RPC is in flight (there is nothing to
            // show yet and the answer may be a prompt, not a create), so `status` alone does not
            // stop a second submit. Without this a double-click sends two pre-flights, both come
            // back `Free`, and two `WorktreeCreate`s race for the same directory.
            if app
                .pending_ops
                .values()
                .any(|op| matches!(op, PendingOp::BranchPreflight { .. }))
            {
                return Task::none();
            }
            let Ok(names) = form.preview() else {
                return Task::none(); // validation error already recorded by the reducer
            };
            let Some(project) = app.core.workspace.active.clone() else {
                return Task::none();
            };
            // Feature 016: classify the name before creating anything. Git lives on the daemon
            // now, so pre-flight is an RPC — the reply decides whether this becomes a create or a
            // prompt. `PendingOp` carries what the answer needs so nothing is recomputed.
            let picked = form.source == BranchSource::Existing;
            // The remote the user named by picking that specific row, so a branch that exists on
            // several remotes tracks the one they chose (spec Edge Cases).
            let preferred_remote = form.selected_branch.as_ref().and_then(|c| match &c.origin {
                BranchOrigin::Remote { remote } => Some(remote.clone()),
                BranchOrigin::Local => None,
            });
            let (branch, dir_name) = (names.branch.clone(), names.dir_name.clone());
            let asked_for = project.clone();
            send_op(
                app,
                PendingOp::BranchPreflight {
                    project: asked_for,
                    names,
                    picked,
                    preferred_remote,
                },
                move |req| ClientMsg::BranchPreflight {
                    req,
                    project,
                    branch,
                    dir_name,
                },
            );
            Task::none()
        }
        // The user answered the prompt: create under the mode they chose. Overwrite cannot arrive
        // here — it only ever comes through the confirmation below (FR-005).
        //
        // Both arms check the state the reducer requires BEFORE letting it clear the prompt: the
        // reducer refuses transitions it considers illegal, and acting anyway would run the
        // create the reducer just declined to acknowledge — an `Overwrite` that never passed the
        // destructive confirmation, in the worst case.
        Message::AddWorktreeResolutionChosen(mode) => {
            let answering = app.core.worktree_form.as_ref().is_some_and(|f| {
                matches!(f.resolution, ResolutionState::Choosing { .. })
                    && !matches!(mode, CreateMode::Overwrite)
            });
            app.core
                .update(Message::AddWorktreeResolutionChosen(mode.clone()));
            if !answering {
                return Task::none();
            }
            start_resolved_create(app, mode)
        }
        Message::AddWorktreeOverwriteConfirmed => {
            let confirmed = app.core.worktree_form.as_ref().is_some_and(|f| {
                matches!(f.resolution, ResolutionState::ConfirmingOverwrite { .. })
            });
            app.core.update(Message::AddWorktreeOverwriteConfirmed);
            if !confirmed {
                return Task::none();
            }
            start_resolved_create(app, CreateMode::Overwrite)
        }
        // Switching to the existing-branch picker lists what the repository already has
        // (feature 016, FR-011). The daemon reads local ref storage only — nothing is fetched.
        Message::AddWorktreeSourceChanged(source) => {
            app.core.update(Message::AddWorktreeSourceChanged(source));
            if source != BranchSource::Existing {
                return Task::none();
            }
            let Some(project) = app.core.workspace.active.clone() else {
                return Task::none();
            };
            let asked_for = project.clone();
            send_op(
                app,
                PendingOp::BranchList { project: asked_for },
                move |req| ClientMsg::BranchList { req, project },
            );
            Task::none()
        }
        // Start a new session at a location — a worktree or the project root ("Default",
        // feature 010): spawn `claude` and stream it (FR-010/012/013). A `Default` location
        // never creates, modifies, or removes a worktree (FR-002) — it simply runs in `repo`
        // itself, so this arm never calls into `micold_core::worktree`.
        Message::SessionStartRequested { location } => {
            // Correlated create: the daemon owns the id + catalog. The new session arrives via the
            // `CatalogChanged` push (reconciled into the core) and is selected + focused when the
            // `OperationOk { SessionCreated }` reply names its id.
            if let Some(project) = app.core.workspace.active.clone() {
                let worktree_dir = match &location {
                    SessionLocation::Worktree(dir) => dir.clone(),
                    SessionLocation::Default => String::new(),
                };
                send_op(app, PendingOp::CreateSession, |req| {
                    ClientMsg::SessionCreate {
                        req,
                        project,
                        worktree_dir,
                    }
                });
            }
            Task::none()
        }
        // Selecting a session reattaches/resumes whichever process its persisted mode selects
        // (FR-005, FR-011) — an Idle AI CLI session resumes via `claude --resume` (FR-023a); a
        // session last left in Regular mode gets a fresh shell instead.
        Message::SessionSelected(id) => {
            app.core.update(Message::SessionSelected(id));
            // View the selected session (the daemon streams its grid), resuming it if idle — the
            // same sequence `view_and_start` performs for every other path that displays a session,
            // called rather than repeated so the pane size that now precedes the start (BUG-003,
            // FR-014a) cannot be added to one copy and not the other.
            view_and_start(app, id);
            // Nothing further: the reducer's `SessionSelected` arm focuses the terminal (FR-011),
            // and selecting from the sidebar no longer releases it, so there is no race left to
            // win with a follow-up message (feature 023, research R3).
            Task::none()
        }
        // Close a session: kill both its processes (AI CLI and shell, feature 010 FR-014) and
        // drop the runtime handles. The pure core archives (not deletes) the record (FR-015a,
        // bugfix BUG-003); here we additionally record the durable, provider-side suppression
        // marker (FR-020c) so a still-existing `claude` transcript is never reconstructed by
        // reconciliation on a later project open.
        // Close (archive) a session: route through the daemon's `SessionDelete`, which archives it
        // durably (anti-resurrection marker) and stops its process (T055). The pure-core update
        // archives the record in memory for instant feedback; the daemon reconciles other windows.
        Message::SessionCloseRequested(id) => {
            app.grids.remove(&id);
            // Release the input counter too (T114): ids are unique UUIDs so it can never be reused,
            // and a session being archived will take no more input. Never on a mere detach — the
            // counter must survive a reconnect for loss detection to hold.
            app.stamper.forget(id);
            send_op(app, PendingOp::DeleteSession, move |req| {
                ClientMsg::SessionDelete { req, session: id }
            });
            app.core.update(Message::SessionCloseRequested(id));
            Task::none()
        }
        // Permanently remove a session (bugfix BUG-003, FR-015c): the same daemon `SessionDelete` —
        // the daemon has no hard-delete, so a remove is an archive with a durable tombstone, which
        // also suppresses any future reconciliation (FR-020c). The pure core drops the record.
        Message::SessionRemoveConfirmed => {
            if let Some(id) = app.core.session_remove_target {
                app.grids.remove(&id);
                app.stamper.forget(id); // T114, as in the close path above.
                send_op(app, PendingOp::DeleteSession, move |req| {
                    ClientMsg::SessionDelete { req, session: id }
                });
            }
            app.core.update(Message::SessionRemoveConfirmed);
            Task::none()
        }
        // Switch the active session's terminal between AI CLI and Regular modes (feature 010,
        // FR-001–FR-004, FR-010): flip the mode, then reattach/spawn whichever process it now
        // selects. Neither process is ever killed as a side effect (FR-006) — the previously-
        // attached one simply stops being displayed/written to and keeps running in the
        // background (research R6).
        Message::TerminalModeToggled => {
            app.core.update(Message::TerminalModeToggled);
            if let Some(id) = app.core.active_session {
                // Entering Regular with no instance yet: lazily open the session's first one
                // (feature 011 FR-007), spawning it on the daemon.
                let needs_first_shell =
                    app.core.workspace.find_session(id).is_some_and(|(_, s)| {
                        s.mode == TerminalMode::Regular && s.shells.is_empty()
                    });
                if needs_first_shell {
                    let shell_id = app
                        .core
                        .workspace
                        .find_session_mut(id)
                        .map(|(_, s)| s.open_shell_instance());
                    if let (Some(shell_id), Some(d)) = (shell_id, &app.daemon) {
                        d.send(ClientMsg::SessionOpenShell {
                            session: id,
                            instance: shell_id,
                        });
                    }
                }
                attach_current_process(app, id);
            }
            Task::none()
        }
        // Manually restart the active session's currently-attached, not-running process
        // (FR-013) — the shell never auto-restarts, so this is its only path back; also covers
        // an Idle/Failed AI CLI, which previously had no explicit affordance. Also re-sources the
        // environment-include script fresh (feature 011, FR-007) — the spec's Clarifications name
        // this restart control as a manual-retry path for a previously-failed script, alongside
        // the Settings-save refresh trigger. Unlike the passive reattach callers below, this is
        // also a direct user restart request, so it must cover a Regular Terminal instance that
        // has already `Exited` — `explicit_restart = true` lets `ensure_attached_process`'s
        // `Regular` branch spawn it, the same case `Message::ShellInstanceRestartRequested`
        // handles for a background instance.
        Message::TerminalRestartRequested => {
            if let Some(id) = app.core.active_session {
                // Re-source fresh for this session's own directory only (BUG-002) — other
                // cached directories are untouched, since only this one needs a new attempt.
                if let Some((cwd, _, _)) = session_cwd_mode_and_active_shell(&app.core, id) {
                    refresh_env_include(app, &cwd);
                }
                view_and_start(app, id);
            }
            Task::none()
        }
        // Manually restart one specific Regular Terminal instance (feature 011, FR-010) —
        // independent of `active_shell`, so a background instance can be restarted without first
        // switching to it. A no-op if that instance's process is already running (idempotent,
        // mirrors `ensure_attached_process`'s reattach-for-free check). Addressed by the
        // originating `SessionId` (not `app.core.active_session`) so this can't misapply to a
        // same-numbered instance of a different session if the active session changed in the
        // same message batch.
        Message::ShellInstanceRestartRequested(id, shell_id) => {
            if let Some(d) = &app.daemon {
                d.send(ClientMsg::SessionRestartShell {
                    session: id,
                    instance: shell_id,
                });
            }
            app.core
                .update(Message::ShellInstanceRestartRequested(id, shell_id));
            Task::none()
        }
        // Open an additional Regular Terminal instance for the active session (feature 011,
        // FR-001–FR-003, FR-007; contracts/shell-instance-lifecycle.md) — the "+" bottom-bar
        // control or the Ctrl+Shift+T/Cmd+Shift+T shortcut. A no-op outside Regular mode (FR-019
        // edge case: the control/shortcut does nothing, and does not switch modes). Unlike
        // `ensure_attached_process` (spawn-if-absent/reattach), this always opens a brand-new
        // instance, even if one is already running.
        Message::ShellInstanceOpenRequested => {
            if let Some(id) = app.core.active_session {
                if let Some((_cwd, TerminalMode::Regular, _)) =
                    session_cwd_mode_and_active_shell(&app.core, id)
                {
                    let shell_id = {
                        let Some((_, session)) = app.core.workspace.find_session_mut(id) else {
                            return Task::none();
                        };
                        session.open_shell_instance()
                    };
                    if let Some(d) = &app.daemon {
                        d.send(ClientMsg::SessionOpenShell {
                            session: id,
                            instance: shell_id,
                        });
                    }
                    attach_current_process(app, id);
                }
            }
            Task::none()
        }
        // Close an individual Regular Terminal instance (feature 011, FR-011–FR-013,
        // FR-018-consistent teardown) — kills and removes only that one `RuntimeTerminal`,
        // leaving sibling instances and the AI CLI process untouched. If this was the session's
        // last instance, the pure reducer flips `mode` back to `AiCli` (FR-013); reattach the AI
        // CLI process via the same shared path the primary toggle already uses (a no-op if it's
        // already attached). Addressed by the originating `SessionId` (not
        // `app.core.active_session`) — see `Message::ShellInstanceSelected`'s doc comment.
        Message::ShellInstanceCloseRequested(id, shell_id) => {
            if let Some(d) = &app.daemon {
                d.send(ClientMsg::SessionCloseShell {
                    session: id,
                    instance: shell_id,
                });
            }
            // Core close reassigns active_shell / reverts mode to AiCli when the last one closes.
            app.core
                .update(Message::ShellInstanceCloseRequested(id, shell_id));
            // Re-attach whatever process the session now shows (a sibling instance, or the primary).
            attach_current_process(app, id);
            Task::none()
        }
        // Switch which Regular-terminal instance is shown (feature 011 FR-004): select it in the
        // core, then attach that process on the daemon so its grid streams.
        Message::ShellInstanceSelected(id, shell_id) => {
            app.core
                .update(Message::ShellInstanceSelected(id, shell_id));
            attach_current_process(app, id);
            Task::none()
        }
        // Stream live keystrokes/paste to the displayed session's currently-ATTACHED process
        // (FR-007/FR-008), but only while that process is Running (FR-012a, feature 010 extends
        // the write-gate to the shell): input to a non-running process is discarded, not
        // buffered.
        Message::TerminalBytes(bytes) => {
            // A window displaced from the active project is read-only: it MUST send zero further
            // input (FR-024). Bail before stamping so no serial is consumed (a consumed-but-unsent
            // serial would be an unrecoverable gap in the input log, G2).
            if active_project_displaced(app) {
                return Task::none();
            }
            if let Some(id) = app.core.active_session {
                // The daemon owns process liveness: it routes input to the session's attached
                // process and drops it harmlessly if that process isn't running. Gating on a
                // client-side lifecycle field is wrong now (the client no longer tracks process
                // state, and the daemon never marks the catalog session Running), so we send
                // whenever connected. Input is stamped with a monotonic per-session serial (G2).
                let msg = app.stamper.stamp(id, bytes);
                if let Some(d) = &app.daemon {
                    d.send(msg);
                }
                // Any live keystroke means the view is at the live bottom again.
                app.display_offset = 0;
            }
            Task::none()
        }
        // Mouse text selection on the displayed session's grid, anchored to absolute `LineId`s so
        // new output can't corrupt it (FR-013/FR-018).
        Message::TerminalSelectStart { col, line, kind } => {
            if let Some(id) = app.core.active_session {
                if let Some(grid) = app.grids.get(&id) {
                    let anchor = Anchor::new(row_line_id(grid, app.display_offset, line), col);
                    let gran = match kind {
                        SelectKind::Simple => SelectGranularity::Char,
                        SelectKind::Semantic => SelectGranularity::Word,
                        SelectKind::Lines => SelectGranularity::Line,
                    };
                    let sel =
                        Selection::start(anchor, gran, |id| grid.line(id).map(|l| l.text.clone()));
                    app.selection = Some(sel);
                }
            }
            Task::none()
        }
        Message::TerminalSelectUpdate { col, line } => {
            if let Some(id) = app.core.active_session {
                if let (Some(grid), Some(sel)) = (app.grids.get(&id), app.selection.as_mut()) {
                    let anchor = Anchor::new(row_line_id(grid, app.display_offset, line), col);
                    sel.update(anchor, |id| grid.line(id).map(|l| l.text.clone()));
                }
            }
            Task::none()
        }
        Message::TerminalSelectCleared => {
            app.selection = None;
            Task::none()
        }
        // Reflow the displayed session's daemon PTY + grid to the visible size (FR-014/FR-015).
        Message::TerminalResized { cols, rows } => {
            // Remember the pane's live size so the next started session starts at it too.
            app.last_grid = Some((cols, rows));
            if let (Some(id), Some(d)) = (app.core.active_session, &app.daemon) {
                d.send(ClientMsg::SessionResize {
                    session: id,
                    cols,
                    rows,
                });
            }
            Task::none()
        }
        // Scroll the displayed session's scrollback view (FR-016). Offset is clamped to the cached
        // history; deeper history is fetched from the daemon on demand (see `request_scrollback`).
        Message::TerminalScrolled(delta) => {
            scroll_view(app, |off, history| {
                (off as i32 + delta).clamp(0, history as i32) as usize
            });
            Task::none()
        }
        // Scroll to an absolute offset (scrollbar drag). Resolve against the LIVE offset at apply
        // time so a burst of batched drag messages converges (drag flicker fix, FR-016).
        Message::TerminalScrolledTo(target) => {
            scroll_view(app, |off, history| {
                let delta = micold_client::ui::target_offset_delta(off, target);
                (off as i32 + delta).clamp(0, history as i32) as usize
            });
            Task::none()
        }
        // Copy the current selection to the system clipboard (FR-013). Also closes the menu.
        Message::TerminalCopyRequested => {
            app.core.update(Message::TerminalContextMenuClosed);
            selection_copy_request(app).map_or_else(Task::none, interpret)
        }
        // Paste the system clipboard into the displayed session's PTY (FR-013). The read is async;
        // its result flows back through `TerminalBytes`, which honours the Running write-gate.
        Message::TerminalPasteRequested => {
            app.core.update(Message::TerminalContextMenuClosed);
            iced::clipboard::read()
                .map(|c| Message::TerminalBytes(c.unwrap_or_default().into_bytes()))
        }
        // Copy arbitrary displayed text (e.g. a worktree name) to the system clipboard, so
        // labels the app itself doesn't make selectable are still reachable cross-application.
        // Also closes the worktree context menu, mirroring its other actions (idempotent if
        // the text wasn't copied from that menu).
        // The text is the request: the view named it (`ui::worktree_menu_items` asks the worktree
        // feature for the display name), so there is nothing left here to decide and no second
        // emitter to write. Translating it is all the shell does.
        Message::TextCopyRequested(text) => {
            app.core.update(Message::WorktreeMenuDismissed);
            interpret(Outcome::ClipboardWrite(text))
        }
        // Poll terminals: feed streamed bytes into the VT emulators, then detect unexpected
        // exits and apply the crash-restart policy (FR-012, FR-022).
        // Open Settings: let the reducer show the overlay, then seed the draft with the current
        // scrollback value (FR-019/FR-020).
        Message::SettingsOpened => {
            app.core.update(Message::SettingsOpened);
            if let Some(draft) = app.core.settings_draft.as_mut() {
                draft.scrollback_lines = app.scrollback_lines.to_string();
                draft.env_include_enabled = app.env_include_enabled;
                draft.env_include_script_path = app.env_include_script_path.clone();
                draft.env_include_timeout = app.env_include_timeout_secs.to_string();
            }
            Task::none()
        }
        // Save Settings: validate the scrollback and environment-include timeout fields; on
        // success persist + apply + refresh + close, on failure keep the form open with an error
        // (FR-020/FR-021; environment-include: FR-014, contracts/settings-ui.md).
        Message::SettingsSaved => {
            let Some(draft) = app.core.settings_draft.clone() else {
                return Task::none();
            };

            let scrollback_min = micold_core::settings::MIN_SCROLLBACK_LINES;
            let scrollback_max = micold_core::settings::MAX_SCROLLBACK_LINES;
            let scrollback_lines = match draft.scrollback_lines.trim().parse::<usize>() {
                Ok(n) if (scrollback_min..=scrollback_max).contains(&n) => n,
                Ok(_) => {
                    if let Some(d) = app.core.settings_draft.as_mut() {
                        d.error = Some(format!(
                            "Enter a number between {scrollback_min} and {scrollback_max}."
                        ));
                    }
                    return Task::none();
                }
                Err(_) => {
                    if let Some(d) = app.core.settings_draft.as_mut() {
                        d.error = Some("Enter a whole number of lines.".to_string());
                    }
                    return Task::none();
                }
            };

            let timeout_min = micold_core::settings::MIN_ENV_INCLUDE_TIMEOUT_SECS;
            let timeout_max = micold_core::settings::MAX_ENV_INCLUDE_TIMEOUT_SECS;
            let env_include_timeout_secs = match draft.env_include_timeout.trim().parse::<u64>() {
                Ok(t) if (timeout_min..=timeout_max).contains(&t) => t,
                Ok(_) => {
                    if let Some(d) = app.core.settings_draft.as_mut() {
                        d.error = Some(format!(
                            "Enter a timeout between {timeout_min} and {timeout_max} seconds."
                        ));
                    }
                    return Task::none();
                }
                Err(_) => {
                    if let Some(d) = app.core.settings_draft.as_mut() {
                        d.error = Some("Enter a whole number of seconds.".to_string());
                    }
                    return Task::none();
                }
            };

            app.scrollback_lines = scrollback_lines;
            app.env_include_enabled = draft.env_include_enabled;
            app.env_include_script_path = draft.env_include_script_path;
            app.env_include_timeout_secs = env_include_timeout_secs;
            if let Some(store) = app.caps.settings() {
                if let Err(err) = store.save(&Settings {
                    theme: app.core.theme_pref,
                    scrollback_lines,
                    env_include_enabled: app.env_include_enabled,
                    env_include_script_path: app.env_include_script_path.clone(),
                    env_include_timeout_secs,
                }) {
                    app.core
                        .notify_error(format!("Couldn't save your settings: {err}"));
                }
            }
            // Also ask a connected daemon to apply the service-owned fields (scrollback,
            // FR-012a; environment-include, FR-012b) so the change takes effect immediately for
            // every session the daemon spawns — not just after its next restart re-reads the file
            // this save just wrote (T100). Silently skipped while disconnected: unlike every other
            // `send_op` caller, saving settings already has a fully-functional local-only path (the
            // write above), so there's no "can't do this at all without a daemon" error to raise —
            // the next daemon boot picks up the file regardless.
            if let Some(daemon) = &app.daemon {
                let req = app.next_req;
                app.next_req += 1;
                daemon.send(ClientMsg::SettingsSet {
                    req,
                    scrollback_lines: Some(scrollback_lines),
                    env_include_enabled: Some(app.env_include_enabled),
                    env_include_script_path: Some(app.env_include_script_path.clone()),
                    env_include_timeout_secs: Some(env_include_timeout_secs),
                });
                app.pending_ops.insert(req, PendingOp::SettingsSet);
            }
            // The enabled/path/timeout settings themselves changed, so every previously cached
            // directory's snapshot is stale (BUG-002) — clear all of them, then eagerly re-source
            // one representative directory so Settings shows fresh feedback immediately; every
            // other directory lazily re-resolves the next time a session in it launches.
            app.env_include_cache.clear();
            let cwd = default_resolution_cwd(&app.core);
            refresh_env_include(app, &cwd);
            app.core.update(Message::SettingsSaved); // closes the overlay
            Task::none()
        }
        Message::TerminalTick => {
            // Obsolete under the daemon: output arrives as streamed grid frames, titles arrive via
            // the daemon (Event::Title), and the daemon supervises/restarts processes. The emitting
            // poll subscription is gone; this no-op keeps `Message` exhaustive. TODO: drop the variant.
            Task::none()
        }
        Message::WindowFocusChanged(focused) => {
            app.window_focused = focused;
            // **Nothing here touches terminal focus, and that is the implementation of
            // FR-013–FR-015** (feature 023). Coming back to the window must leave the keyboard
            // exactly where it was, and it does — not because anything is saved and restored, but
            // because `State::terminal_focused()` is derived from state that a window focus change
            // does not write. The spec names a "suspended holder"; it has no runtime existence.
            //
            // So resist adding a restore here. A rule that hands the terminal the keyboard on
            // return would take it from a half-typed dialog field and would undo a release the
            // user made on purpose. `window_focus_changes_no_focus_term` in
            // `tests/terminal_focus.rs` fails if this arm starts writing one.

            // Re-detect on the way back in rather than waiting out the next poll tick: coming
            // back from the OS theme settings is the single most likely moment for the app's
            // idea of the scheme to be stale (003 FR-006).
            if focused {
                app.core
                    .update(Message::SystemThemeChanged(detect_system_scheme()));
            }
            Task::none()
        }
        // Confirmed worktree delete (feature 008, FR-020): terminate the worktree's session
        // processes, remove its git worktree + branch + directory, then drop the records and
        // persist. Ordered per `CleanupStep`; every git step is idempotent (FR-023).
        // Worktree delete (feature 008/013): route through the daemon, which removes the worktree
        // (git off its runtime), and — gated on the removal actually succeeding — archives the
        // worktree's sessions and broadcasts the new catalog (T055). A failed delete surfaces as an
        // `OperationError` and the worktree reappears on the next reconcile, so the optimistic local
        // drop below self-heals. `stop_sessions: true` mirrors the old behaviour (it always stopped
        // the worktree's sessions first).
        //
        // NOTE: the daemon keeps the branch (no keep/delete wire flag yet), so the confirm dialog's
        // "delete branch" choice is currently a no-op — branch deletion needs a wire field (deferred).
        Message::WorktreeDeleteConfirmed => {
            let target = app.core.worktree_delete_target.clone();
            if let (Some(dir), Some(project)) = (target, app.core.workspace.active.clone()) {
                // Drop this path's cached env-include snapshot (BUG-002): a worktree recreated for
                // the same branch reuses the exact path, and a stale snapshot would linger forever.
                let cwd =
                    session_cwd_for_location(&project, &SessionLocation::Worktree(dir.clone()));
                app.env_include_cache.remove(&cwd);
                let (p, d) = (project, dir.clone());
                // Feature 013 (FR-011/FR-012): the user's explicit keep/delete choice from the
                // confirm dialog, defaulting to "delete the branch" (`worktree_delete_keep_branch`
                // defaults to `false`).
                let delete_branch = !app.core.worktree_delete_keep_branch;
                send_op(app, PendingOp::WorktreeDelete(dir), move |req| {
                    ClientMsg::WorktreeDelete {
                        req,
                        project: p,
                        dir_name: d,
                        stop_sessions: true,
                        delete_branch,
                    }
                });
            }
            // Optimistically drop the records + dismiss the dialog; the daemon's `CatalogChanged`
            // reconciles the truth (re-adding the worktree on a failed delete).
            app.core.update(Message::WorktreeDeleteConfirmed);
            Task::none()
        }
        other => {
            app.core.update(other);
            Task::none()
        }
    }
}

fn view(app: &App) -> iced::Element<'_, Message> {
    let Some(probe) = &app.probe else {
        return render(app);
    };
    // A scene run does not start counting until the scene is verified composed — frames spent
    // building it are not frames of it.
    if probe_scene().is_some() && !app.scene_ready {
        return render(app);
    }
    // A measurement run (FR-039b). Time the composition of this frame, and end the process once the
    // run has the frames it asked for.
    let started = Instant::now();
    let element = render(app);
    let elapsed = started.elapsed();

    // Outside the timed span, deliberately: this is the probe's own bookkeeping and belongs in the
    // figure no more than the borrow below does.
    if let Some(scene) = probe_scene() {
        // The scene has to still *be* the scene. `Scene::check` stopped being asked the moment it
        // first passed, which left every counted frame measured against whatever the window drifted
        // into — and produced a `full` figure that landed in one of two clusters 60% apart
        // depending on whether it drifted (T083, FR-039b).
        if let Err(why) = scene.check_still_composed(&scene_facts(app)) {
            eprintln!("frame probe: {why}");
            std::process::exit(4);
        }
        if app.ripples_animating.load(Ordering::Relaxed) > 0 {
            app.scene_ripple_frames
                .set(app.scene_ripple_frames.get() + 1);
        }
    }

    let mut probe = probe.borrow_mut();
    probe.record(elapsed);
    if probe_config().is_some_and(|config| config.is_complete(&probe)) {
        report_probe_and_exit(&probe, app);
    }
    element
}

/// Compose the frame. Separated from [`view`] so the measurement run times exactly this and nothing
/// of its own bookkeeping.
///
/// **What the figure covers.** This is the cost of *composing* the frame — building the widget tree
/// from state — on the CPU. It is not the cost of presenting it: layout, draw and GPU work all
/// happen after this returns, and are not in the number. The alternative, timing the interval
/// between presented frames, measures the display's refresh rate rather than the scene for any
/// scene that renders faster than one vsync, which is every scene worth comparing here. §B8 records
/// this limitation alongside the figures.
fn render(app: &App) -> iced::Element<'_, Message> {
    // Render the displayed session from its daemon-streamed grid cache + the client-side selection
    // and scroll offset (feature 010). The daemon is the single source of screen state.
    micold_client::ui::view(
        &app.core,
        app.attached_grid(),
        app.selection.as_ref(),
        app.display_offset,
        app.dismissing.as_ref(),
        &app.env_include_last_outcome,
        &connection_status(app),
    )
}

/// Whether this window has been displaced from the active project (US5, FR-024) — the read-only
/// condition that suppresses input.
fn active_project_displaced(app: &App) -> bool {
    app.core
        .workspace
        .active
        .as_ref()
        .is_some_and(|p| app.displaced.contains_key(p))
}

/// The connection state for the status banner (US5/US6). Precedence: a contract mismatch (US6) wins
/// — it blocks every connection and has the most specific action — then a same-contract build
/// mismatch (US6, FR-022a), then a per-project takeover (US5), then a plain disconnect. Each names
/// the situation and offers a concrete action.
/// Resolve this window's connection facts and let the feature decide which one the banner shows.
///
/// The precedence lives in `features::connection` so it is testable without a window; what is left
/// here is the one thing that needs the shell: turning the active project into a displacement.
fn connection_status(app: &App) -> micold_client::features::connection::ConnectionStatus {
    let displaced_by = app
        .core
        .workspace
        .active
        .as_ref()
        .and_then(|project| app.displaced.get(project))
        .map(String::as_str);

    micold_client::features::connection::connection_status(
        app.version_mismatch.as_ref(),
        app.build_mismatch.as_ref(),
        displaced_by,
        app.disconnected,
    )
}

fn theme(app: &App) -> iced::Theme {
    micold_client::ui::theme(app.core.color_scheme())
}

/// The absolute [`LineId`] shown at viewport `row` of `grid`, accounting for scrollback `offset`.
fn row_line_id(grid: &GridCache, offset: usize, row: u16) -> LineId {
    LineId(grid.viewport_top().0 - offset as i64 + row as i64)
}

/// Update the displayed session's scroll offset via `f(current, history)`, then fetch any revealed
/// scrollback the cache doesn't yet hold from the daemon (`ScrollbackRequest`). `history` is the
/// daemon's full retained depth (`viewport_top - oldest_available`), so the view can scroll into
/// history; un-fetched lines render blank until the `ScrollbackResponse` fills them (FR-016/017).
fn scroll_view(app: &mut App, f: impl FnOnce(usize, usize) -> usize) {
    let Some(id) = app.core.active_session else {
        return;
    };
    let (vt, new_off, need_from) = {
        let Some(grid) = app.grids.get(&id) else {
            return;
        };
        let vt = grid.viewport_top().0;
        let oldest = grid.oldest_available().0;
        let history = (vt - oldest).max(0) as usize;
        let new_off = f(app.display_offset, history);
        // The visible window's top line; find the lowest un-cached line in [top, viewport_top).
        let top = (vt - new_off as i64).max(oldest);
        let rows = grid.rows() as i64;
        let mut need_from = None;
        let mut lid = top;
        while lid < vt && lid < top + rows {
            if grid.line(LineId(lid)).is_none() {
                need_from = Some(lid);
                break;
            }
            lid += 1;
        }
        (vt, new_off, need_from)
    };
    app.display_offset = new_off;
    if let Some(from) = need_from {
        let req = app.next_req;
        app.next_req += 1;
        if let Some(d) = &app.daemon {
            d.send(ClientMsg::ScrollbackRequest {
                session: id,
                req,
                ranges: vec![LineId(from)..LineId(vt)],
            });
        }
    }
}

/// View a session on the daemon — start/resume it and stream its grid — resetting the local
/// selection and scroll for the newly-displayed session.
fn view_and_start(app: &mut App, id: SessionId) {
    app.selection = None;
    app.display_offset = 0;
    if let (Some(project), Some(d)) = (app.core.workspace.active.clone(), &app.daemon) {
        send_pane_size(app, id);
        d.send(ClientMsg::SessionStart { session: id });
        d.send(ClientMsg::SetViewedSession {
            project,
            session: Some(id),
        });
    }
}

/// Tell the daemon what size to start `id` at, **before** its `SessionStart` (BUG-003, FR-014a).
///
/// The pane widget only publishes `Message::TerminalResized` when its own size *changes*, so a
/// session started into a window the user is not resizing is never told anything — it used to come
/// up at the daemon's 100×30 spawn seed and stay there until the next window resize. `App::last_grid`
/// is the last size the pane published; stating it here is what makes it a size the *next* session
/// starts at rather than only one the current session was corrected to. Ordered before the start so
/// the daemon has it recorded when the spawn reads it (the daemon also honours it if it arrives
/// afterwards — `010` FR-020a — but only the ordering makes the spawn itself right).
///
/// A no-op before the first frame has laid out a pane (`last_grid` is `None`), where the daemon's
/// own default correctly applies.
fn send_pane_size(app: &App, id: SessionId) {
    if let (Some((cols, rows)), Some(d)) = (app.last_grid, &app.daemon) {
        d.send(ClientMsg::SessionResize {
            session: id,
            cols,
            rows,
        });
    }
}

/// Map a wire lifecycle back to the domain one (inverse of the daemon's `wire_lifecycle`).
/// `InterruptedResumable` — a session the daemon found durably-running after a restart, never
/// auto-relaunched — is carried through as its own state so the sidebar/status can present it
/// distinctly and its select action resumes it (FR-006a).
fn wire_to_lifecycle(w: &WireLifecycle) -> SessionLifecycle {
    match w {
        WireLifecycle::Idle => SessionLifecycle::Idle,
        WireLifecycle::InterruptedResumable => SessionLifecycle::InterruptedResumable,
        WireLifecycle::Starting => SessionLifecycle::Starting,
        WireLifecycle::Running => SessionLifecycle::Running,
        WireLifecycle::Restarting { attempts } => SessionLifecycle::Restarting {
            attempts: *attempts,
        },
        WireLifecycle::Failed { .. } => SessionLifecycle::Failed,
    }
}

/// Send a correlated mutating RPC to the daemon: allocate a `req`, record the pending op (so the
/// reply can be matched and a disconnect can resolve it as unknown), and send the message `build`s.
/// A no-op that notifies the user when there is no daemon connection (T055).
fn send_op(app: &mut App, op: PendingOp, build: impl FnOnce(u64) -> ClientMsg) {
    let Some(daemon) = &app.daemon else {
        app.core.notify_error(format!(
            "Not connected to the session service — can't {} right now.",
            op.describe()
        ));
        return;
    };
    let req = app.next_req;
    app.next_req += 1;
    daemon.send(build(req));
    app.pending_ops.insert(req, op);
}

/// Move this client's daemon attachment from `old` to `new` on a project switch: release the old
/// (so another window can take it), attach the new, and set the viewed session — so the daemon
/// streams grid frames and discovers worktrees for the project now in focus (T055). A no-op when
/// disconnected; the initial attach on connect is handled by `DaemonConnected`.
/// Append a line to the client's own log, beside the daemon's (`micold-client.log`).
///
/// The client has no logging framework and this does not add one: a single appended line, opened
/// and closed per call, on a path that already exists for the daemon. It is here rather than in the
/// reducer because the reducer is render-free and does no I/O, and it writes to a file rather than
/// stderr because how the application was launched should not decide whether a diagnostic survives.
///
/// Silently does nothing if the directory cannot be resolved or the file cannot be opened. A
/// diagnostic that can itself fail the thing it is diagnosing is worse than no diagnostic.
fn log_line(message: &str) {
    let Some(dirs) = directories::ProjectDirs::from("", "", "micold-ai-ide") else {
        return;
    };
    let dir = dirs.data_dir();
    if std::fs::create_dir_all(dir).is_err() {
        return;
    }
    if let Ok(mut file) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(dir.join("micold-client.log"))
    {
        use std::io::Write;
        let _ = writeln!(file, "{message}");
    }
}

/// Record which session entering `path` landed on, and why (feature 008 FR-003).
///
/// Written at every project switch because "it forgot which session I was on" is a report with
/// four distinct causes, and the one hardest to see from outside — a resolve looking under a key
/// nothing is filed under — is indistinguishable from the others in the UI. The known keys are
/// logged alongside for exactly that case: if the sidebar lists sessions the resolve cannot find,
/// the two keys are printed side by side and the mismatch is the answer.
fn log_foreground_choice(app: &App, path: &Path) {
    let choice = &app.core.last_foreground_choice;
    let keys: Vec<String> = app
        .core
        .workspace
        .sessions
        .keys()
        .map(|k| k.display().to_string())
        .collect();
    log_line(&format!(
        "switch: entered {} -> active_session={:?} choice={:?} resolve_key={} session_keys={:?}",
        path.display(),
        app.core.active_session,
        choice,
        micold_core::project::canonicalize_best_effort(path).display(),
        keys,
    ));
}

fn switch_daemon_attachment(app: &App, old: Option<PathBuf>, new: &Path) {
    let Some(daemon) = &app.daemon else {
        return;
    };
    if let Some(old) = old {
        if old != new {
            daemon.send(ClientMsg::Detach { project: old });
        }
    }
    daemon.send(ClientMsg::Attach {
        project: new.to_path_buf(),
        force: false,
    });
    daemon.send(ClientMsg::SetViewedSession {
        project: new.to_path_buf(),
        session: app.core.active_session,
    });
}

/// Phrase the surviving paths for a partial-success delete notice (FR-023d, BUG-002).
///
/// Names the owner when the platform reported one, because that is what tells the user *why* the
/// app could not remove it and what they need in order to: "owned by another user (uid 0)" points
/// straight at a container that wrote build output as root, where a bare path alone would read as
/// an unexplained failure. Long lists are truncated — the report is already capped, and naming a
/// couple of blockers plus a count is what a person can act on.
fn describe_leftovers(leftovers: &[micold_core::worktree::Leftover]) -> String {
    const NAMED: usize = 2;
    let named = leftovers
        .iter()
        .take(NAMED)
        .map(|l| match l.foreign_uid {
            Some(uid) => format!("{} (owned by another user, uid {uid})", l.path.display()),
            None => l.path.display().to_string(),
        })
        .collect::<Vec<_>>()
        .join(", ");
    match leftovers.len().saturating_sub(NAMED) {
        0 => format!("these paths could not be removed: {named}"),
        rest => format!("these paths could not be removed: {named}, and {rest} more"),
    }
}

/// Reconcile the client's core session state from the daemon's authoritative catalog snapshot
/// (FR-011). The daemon owns sessions now, so each project's session list is made to mirror the
/// snapshot: existing sessions have their lifecycle + label updated; sessions the daemon reports
/// but the client lacks are added; sessions the daemon no longer reports (archived/removed) are
/// dropped. A dangling `active_session` pointer is cleared.
fn reconcile_catalog(core: &mut State, snapshot: &CatalogSnapshot, sync_worktrees: bool) {
    // Mirror the daemon's project list into the client (T055). Add projects the daemon reports that
    // the client lacks (e.g. opened in another window), and adopt the daemon's display name for known
    // ones. Deliberately NOT a full mirror: projects are not *removed* here — a `CatalogChanged` that
    // predates this client's own in-flight `ProjectAdd` must not drop the project it just opened, and
    // an ephemeral (non-persisting) daemon reporting an empty catalog must not wipe the list. Forget
    // drops the record locally (optimistically) and durably on the daemon.
    for snap in &snapshot.projects {
        if let Some(existing) = core
            .workspace
            .projects
            .iter_mut()
            .find(|p| p.path == snap.path)
        {
            existing.display_name = snap.display_name.clone();
        } else {
            let availability = if snap.available {
                micold_core::project::Availability::Available
            } else {
                micold_core::project::Availability::Unavailable
            };
            let mut project = micold_core::project::Project::new(
                snap.path.clone(),
                snap.is_git_repo,
                availability,
            );
            project.display_name = snap.display_name.clone();
            core.workspace.projects.push(project);
        }
    }
    // Sessions observed transitioning into `Restarting` this reconciliation (feature 008,
    // FR-011/SC-007) — collected here and applied after the loop below, since
    // `note_background_restart` needs `&mut core` while `list` still holds `core.workspace`
    // borrowed. `note_background_restart` itself no-ops for the active project's session, so
    // background-ness isn't checked here.
    let mut newly_restarting: Vec<SessionId> = Vec::new();
    for project in &snapshot.projects {
        let list = core
            .workspace
            .sessions
            .entry(project.path.clone())
            .or_default();
        let snap_ids: HashSet<SessionId> = project.sessions.iter().map(|s| s.id).collect();
        for summary in &project.sessions {
            let lifecycle = wire_to_lifecycle(&summary.lifecycle);
            if let Some(existing) = list.iter_mut().find(|s| s.id == summary.id) {
                if !matches!(existing.lifecycle, SessionLifecycle::Restarting { .. })
                    && matches!(lifecycle, SessionLifecycle::Restarting { .. })
                {
                    newly_restarting.push(existing.id);
                }
                existing.lifecycle = lifecycle;
                existing.activity = summary.activity.clone();
                // Adopt the daemon's title only when it has a real one. The daemon now overlays the
                // live OSC-0 title onto the summary (T047), but a summary can still be `Pending`
                // before the first title arrives; don't let that clobber a title already learned.
                if let SessionLabel::Named(_) = summary.title {
                    existing.label = summary.title.clone();
                }
            } else {
                let location = summary
                    .worktree_dir
                    .clone()
                    .map(SessionLocation::Worktree)
                    .unwrap_or(SessionLocation::Default);
                let mut s = Session::restored(
                    summary.id,
                    location,
                    summary.title.clone(),
                    TerminalMode::AiCli,
                );
                s.lifecycle = lifecycle;
                s.activity = summary.activity.clone();
                list.push(s);
            }
        }
        // Drop sessions the daemon no longer reports (archived/removed on its side).
        list.retain(|s| snap_ids.contains(&s.id));
    }
    for id in newly_restarting {
        core.note_background_restart(id);
    }
    // Mirror the active project's worktrees from the daemon's git discovery into the render state
    // (the sidebar reads `core.worktrees` + `worktree_names`). Only on `CatalogChanged` pushes, not
    // the initial welcome: the welcome's worktree cache is empty until the post-attach refresh, so
    // syncing it would briefly blank the list boot-time local discovery had populated (T055).
    if sync_worktrees {
        if let Some(active) = core.workspace.active.clone() {
            if let Some(project) = snapshot.projects.iter().find(|p| p.path == active) {
                let root = active.join(".claude/worktrees");
                core.set_worktrees(
                    project
                        .worktrees
                        .iter()
                        .map(|w| micold_core::worktree::Worktree {
                            dir_name: w.dir_name.clone(),
                            path: root.join(&w.dir_name),
                            branch: w.branch.clone(),
                            status: wire_to_worktree_status(w.status),
                        })
                        .collect(),
                );
                // Mirror display-name overrides from the catalog (a second window sees a rename).
                let names: std::collections::BTreeMap<String, String> = project
                    .worktrees
                    .iter()
                    .filter(|w| w.display_name != w.dir_name)
                    .map(|w| (w.dir_name.clone(), w.display_name.clone()))
                    .collect();
                if names.is_empty() {
                    core.workspace.worktree_names.remove(&active);
                } else {
                    core.workspace.worktree_names.insert(active, names);
                }
            }
        }
    }
    // Clear a dangling active-session pointer if its session is gone.
    //
    // Feature 024: through `set_current_session`, like every other app-initiated clear, so the row
    // the vanished session was in is committed open rather than snapping shut under the user
    // (FR-001c). Nothing is armed: there is no session to scroll to.
    if let Some(id) = core.active_session {
        if core.workspace.find_session(id).is_none() {
            core.set_current_session(None);
        }
    }
}

/// Project the wire [`WorktreeStatus`] back onto the client's core status enum (T055). The inverse of
/// the daemon's mapping; `Locked`/`Prunable` both collapse to `Invalid` (the client renders both as
/// an unusable/removable worktree).
fn wire_to_worktree_status(
    status: micold_core::protocol::messages::WorktreeStatus,
) -> micold_core::worktree::WorktreeStatus {
    use micold_core::protocol::messages::WorktreeStatus as Wire;
    use micold_core::worktree::WorktreeStatus as Core;
    match status {
        Wire::Clean => Core::Valid,
        Wire::Missing => Core::Missing,
        Wire::Locked | Wire::Prunable => Core::Invalid,
    }
}

/// Which daemon process the session currently shows (feature 011): its `Primary` (the AI CLI, or a
/// persisted Regular session's own shell) unless a specific Regular-terminal instance is selected.
fn session_process(session: &Session) -> SessionProcess {
    match (session.mode, session.active_shell) {
        (TerminalMode::Regular, Some(sid)) if !session.shells.is_empty() => {
            SessionProcess::Shell(sid)
        }
        _ => SessionProcess::Primary,
    }
}

/// Tell the daemon which of a session's processes to attach (stream + drive), based on its current
/// mode + active shell, and reset the local view (selection + scroll) for the switch. Called
/// whenever the attached process changes: mode toggle, instance select/open/close.
fn attach_current_process(app: &mut App, id: SessionId) {
    let process = app
        .core
        .workspace
        .find_session(id)
        .map(|(_, s)| session_process(s));
    app.selection = None;
    app.display_offset = 0;
    if let (Some(process), Some(d)) = (process, &app.daemon) {
        d.send(ClientMsg::SessionAttachProcess {
            session: id,
            process,
        });
    }
}

/// Perform a feature's effect request (feature 021, T045 — FR-015a, contract C3).
///
/// Translation and nothing else: one arm per variant, no branch that could have gone the other
/// way. What reaches the clipboard, and whether anything should, was decided by the feature that
/// emitted the request — which is the whole point of expressing the request instead of the call.
fn interpret(outcome: Outcome) -> Task<Message> {
    match outcome {
        Outcome::ClipboardWrite(text) => iced::clipboard::write(text),
    }
}

/// Ask the selection feature what copying it should put on the clipboard.
///
/// Only the grid lookup is here — finding the displayed session's cached lines is reading the
/// shell's own data, not a rule about copying. Without a grid there is nothing to resolve the
/// selection against, so there is no selection to offer, which is what `selected_text` said by
/// returning an empty string before the request had a type.
fn selection_copy_request(app: &App) -> Option<Outcome> {
    let grid = app.core.active_session.and_then(|id| app.grids.get(&id))?;
    selection::copy_request(app.selection.as_ref(), |id| {
        grid.line(id).map(|l| l.text.clone())
    })
}

/// How often the snackbar's countdown ticks while one is visible.
///
/// Coarse on purpose: the durations it serves are 4s and 10s, so a quarter-second tick is
/// imperceptible in the dismissal and costs four wake-ups a second instead of sixty. It runs only
/// while a notification is on screen.
const SNACKBAR_TICK: std::time::Duration = std::time::Duration::from_millis(250);

fn subscription(app: &App) -> Subscription<Message> {
    // Event-driven (not a poll): reports actual OS focus changes, so it costs nothing while
    // the window sits idle either focused or not (idle-CPU fix).
    // Resize events are rare, so this costs nothing at idle; it keeps `window_size` current for
    // context-menu clamping (feature 015).
    let mut subs = vec![
        micold_client::ui::subscription(&app.core),
        window_focus_events(),
        // The daemon connection: one long-lived socket to the session host (feature 010, T041).
        micold_client::daemon::connection(),
        iced::window::resize_events().map(|(_id, size)| Message::WindowResized {
            width: size.width.max(0.0) as u16,
            height: size.height.max(0.0) as u16,
        }),
    ];
    // Always polled — see [`BACKGROUND_OS_THEME_POLL`]. Only the cadence follows focus.
    subs.push(os_theme_poll(os_theme_poll_interval(app.window_focused)));
    // The snackbar's clock, subscribed **only while something is on screen** (FR-032a, SC-017).
    // A timer that ran at rest would hold the loop awake for the life of the process to count down
    // a notification that does not exist; `Queue::is_active` is what keeps it off.
    if app.core.notify.is_active() {
        subs.push(
            iced::time::every(SNACKBAR_TICK)
                .map(|_| Message::NotificationsAdvanced(SNACKBAR_TICK.as_millis() as u32)),
        );
    }
    // The terminal output poll is gone — the daemon streams grid frames over the connection. Worktree
    // create now runs on the daemon too, so there is no local progress buffer to drain (T055).
    // No animation clock. Every transition is played by the widget that owns it, and a widget
    // that is moving asks the runtime for the next frame itself — so the idle window schedules
    // nothing at all, rather than ticking 60 times a second to advance tracks that have all
    // arrived (FR-014, FR-025).
    // Track the pointer ONLY while the project switcher is open (feature 015), so a right-click
    // on a row can anchor its context menu at the cursor. Scoping it this way keeps the idle
    // window free of per-mouse-move redraws — the switcher is a brief, deliberate interaction.
    if app.core.project_switcher_open {
        subs.push(cursor_move_events());
    }
    // A measurement run, and only a measurement run, drives the window continuously (FR-039b): the
    // scene has to be re-composed for there to be anything to time. `window::frames()` yields once
    // per presented frame, and the `NoOp` it maps to is enough to make the runtime compose the next
    // one — so this needs no `request_redraw` of its own, and 017's single sanctioned frame-request
    // path (`ui/cdk/motion.rs`) stays the only one.
    //
    // Idle quiescence (SC-017, FR-039a) is unaffected because the branch is unreachable without the
    // environment variable; `tests/frame_probe_glue.rs` is what keeps that true.
    if probe_config().is_some() {
        subs.push(iced::window::frames().map(|_| Message::NoOp));
    }
    Subscription::batch(subs)
}

/// The OS theme poll interval for this tick. Unlike the terminal poll this is never `None`:
/// suspending it while unfocused is what let a visible window keep the wrong theme (003 FR-006 /
/// SC-003). Both cadences satisfy SC-003's one-second bound.
fn os_theme_poll_interval(window_focused: bool) -> Duration {
    if window_focused {
        OS_THEME_POLL
    } else {
        BACKGROUND_OS_THEME_POLL
    }
}

/// Subscribes to raw OS window events and keeps only focus changes, translating them into
/// [`Message::WindowFocusChanged`]. Every other window event (resize, move, redraw, ...) is
/// discarded before it ever reaches `update`.
/// Subscribes to raw pointer events and keeps only cursor moves, translating them into
/// [`Message::CursorMoved`] (feature 015). Only subscribed while the project switcher is open —
/// see [`subscription`] — since its sole purpose is anchoring a row's right-click menu.
fn cursor_move_events() -> Subscription<Message> {
    iced::event::listen_with(cursor_move_message)
}

/// The `listen_with` callback backing [`cursor_move_events`]; a free function (rather than a
/// closure) so it can be unit-tested directly. Negative coordinates (the pointer leaving the
/// window on some platforms) clamp to 0 rather than wrapping around the `u16` cast.
fn cursor_move_message(
    event: iced::Event,
    _status: iced::event::Status,
    _window: iced::window::Id,
) -> Option<Message> {
    match event {
        iced::Event::Mouse(iced::mouse::Event::CursorMoved { position }) => {
            Some(Message::CursorMoved {
                x: position.x.max(0.0) as u16,
                y: position.y.max(0.0) as u16,
            })
        }
        _ => None,
    }
}

fn window_focus_events() -> Subscription<Message> {
    iced::event::listen_with(window_focus_message)
}

/// The `listen_with` callback backing [`window_focus_events`]; a free function (rather than a
/// closure) so it can be unit-tested directly.
fn window_focus_message(
    event: iced::Event,
    _status: iced::event::Status,
    _window: iced::window::Id,
) -> Option<Message> {
    match event {
        iced::Event::Window(iced::window::Event::Focused) => {
            Some(Message::WindowFocusChanged(true))
        }
        iced::Event::Window(iced::window::Event::Unfocused) => {
            Some(Message::WindowFocusChanged(false))
        }
        _ => None,
    }
}

/// The session's cwd (worktree or project root) + current `TerminalMode` + `active_shell`, for a
/// session of the active project (feature 010/011).
fn session_cwd_mode_and_active_shell(
    core: &State,
    id: SessionId,
) -> Option<(PathBuf, TerminalMode, Option<ShellInstanceId>)> {
    let repo = core.workspace.active.clone()?;
    let session = core.active_sessions().iter().find(|s| s.id == id)?;
    let cwd = session_cwd_for_location(&repo, &session.location);
    Some((cwd, session.mode, session.active_shell))
}

/// Discover the active project's worktrees from git + the filesystem (FR-018/018a). Delegates to the
/// shared `micold_core::worktree::discover` so the client and daemon can never diverge in how a
/// worktree is discovered.
fn discover_worktrees(git: &dyn Git, repo: &Path) -> Vec<Worktree> {
    micold_core::worktree::discover(git, repo)
}

/// Send the create for a resolved mode, re-deriving the names from the form (feature 016).
///
/// Shared by both resolution answers so `Overwrite` and the non-destructive modes take exactly one
/// path to the daemon. The daemon re-verifies the mode against a fresh pre-flight before touching
/// anything (FR-009), so a branch that changed while the prompt was open fails cleanly rather than
/// acting on a stale answer.
fn start_resolved_create(app: &mut App, mode: CreateMode) -> Task<Message> {
    let Some(form) = app.core.worktree_form.clone() else {
        return Task::none();
    };
    // Same double-submit guard `AddWorktreeSubmitted` applies: the answer buttons stop being
    // rendered once the prompt resolves, but two clicks can queue two messages before the next
    // render, and the reducer's second pass is a no-op — only this check stops the second one
    // from launching a concurrent create of the same worktree.
    if form.status != WorktreeFormStatus::Editing {
        return Task::none();
    }
    let Ok(names) = form.preview() else {
        return Task::none();
    };
    let Some(project) = app.core.workspace.active.clone() else {
        return Task::none();
    };
    send_worktree_create(app, project, names, mode);
    Task::none()
}

/// Hand a fully-resolved create to the daemon and put the form into its in-progress state.
fn send_worktree_create(
    app: &mut App,
    project: PathBuf,
    names: micold_core::naming::DerivedNames,
    mode: CreateMode,
) {
    app.core
        .update(Message::WorktreeCreateStarted(mode.clone()));
    let (branch, dir_name) = (names.branch, names.dir_name);
    // The mode is not duplicated here: `WorktreeCreateStarted` above already put it on the form,
    // which is where the stage label reads it from (FR-024).
    send_op(
        app,
        PendingOp::WorktreeCreate(dir_name.clone()),
        move |req| ClientMsg::WorktreeCreate {
            req,
            project,
            branch,
            dir_name,
            mode,
        },
    );
}

fn map_system_scheme(mode: dark_light::Mode) -> SystemScheme {
    match mode {
        dark_light::Mode::Dark => SystemScheme::Dark,
        dark_light::Mode::Light => SystemScheme::Light,
        dark_light::Mode::Unspecified => SystemScheme::Unspecified,
    }
}

/// Query the OS for its current light/dark preference (FR-005). `dark_light::detect()`'s Linux
/// backend has a hardcoded 25 ms D-Bus timeout and returns `Err` under CPU contention with no
/// relation to the actual OS preference — the caller falls this back to the last-known scheme
/// via `theme::observe_system_scheme` rather than `SystemScheme::Unspecified` (FR-021; BUG-001).
/// Deliberately takes no arguments (bugfix, found by `run` sanity check, 2026-07-23): it used to
/// take `last_known: SystemScheme` and apply the fallback itself, but that meant
/// `os_theme_poll`'s `Subscription::map` closure had to *capture* `last_known` to call it — and
/// iced panics on boot if a subscription's mapping closure captures anything, since a capturing
/// closure can't have the stable identity iced needs to avoid restarting the underlying timer
/// every frame. The fallback now happens in the reducer (`Message::SystemThemeChanged`,
/// `src/app.rs`), which already has the previous scheme in `self.system_scheme`.
fn detect_system_scheme() -> Result<SystemScheme, ()> {
    SystemThemeProbe.detect()
}

/// The real [`OsThemeProbe`] (feature 021, T047): the codebase's only direct operating-system
/// branch, now behind the capability.
///
/// Here rather than in the core, where the trait and its fake live, because `dark-light` is a
/// client dependency and `micold-core` deliberately has none on it — that is why
/// [`SystemScheme`] mirrors `dark_light::Mode` instead of re-exporting it. Moving the call into
/// the core to "isolate the OS branch" would have put the OS crate in the render-free half, which
/// is the opposite of isolating it. The shell owns the concrete implementation; that is FR-017.
struct SystemThemeProbe;

impl OsThemeProbe for SystemThemeProbe {
    fn detect(&self) -> Result<SystemScheme, ()> {
        dark_light::detect().map(map_system_scheme).map_err(|_| ())
    }
}

fn os_theme_poll(interval: Duration) -> Subscription<Message> {
    every(interval).map(|_instant| Message::SystemThemeChanged(detect_system_scheme()))
}

/// The worktree-creation failure text shown in the form (feature 010, FR-006/SC-003): appends
/// `detail` (the daemon's `OperationError.detail`, git's own stderr verbatim) to `message` when
/// present and non-blank. For a submodule fetch failure, `message` alone is the generic "git
/// failed to create the worktree" — `detail` is normally the only place that names which
/// submodule failed and why (auth/network/unreachable commit).
fn worktree_create_error_text(message: String, detail: Option<String>) -> String {
    match detail {
        Some(detail) if !detail.trim().is_empty() => format!("{message}: {}", detail.trim()),
        _ => message,
    }
}

fn start_dir() -> PathBuf {
    directories::UserDirs::new()
        .map(|dirs| dirs.home_dir().to_path_buf())
        .unwrap_or_else(|| PathBuf::from(std::path::MAIN_SEPARATOR_STR))
}

fn scan_task(browser: Arc<dyn FolderBrowser + Send + Sync>, dir: PathBuf) -> Task<Message> {
    Task::perform(async move { scan(&*browser, dir) }, |message| message)
}

fn scan(browser: &dyn FolderBrowser, dir: PathBuf) -> Message {
    match browser.list_subdirs(&dir) {
        Ok(entries) => Message::SelectorListingReady(entries),
        Err(error) => Message::SelectorListingFailed(error.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use micold_client::features::settings::SettingsDraft;
    use micold_core::protocol::messages::{ActivitySignal, ProjectSnapshot, SessionSummary};

    // Convergence fix (retrofit session, 2026-07-27): the daemon's OperationError.detail (git's
    // own stderr, e.g. naming which submodule failed and why) was destructured with `..` and
    // silently discarded — the worktree-creation form only ever showed the generic
    // "git failed to create the worktree" message, never the diagnostic FR-006/SC-003 requires.
    #[test]
    fn worktree_create_error_appends_a_non_blank_detail() {
        assert_eq!(
            worktree_create_error_text(
                "git failed to create the worktree".to_string(),
                Some(
                    "fatal: could not read Username for 'https://example.com': terminal prompts disabled"
                        .to_string()
                ),
            ),
            "git failed to create the worktree: fatal: could not read Username for \
             'https://example.com': terminal prompts disabled"
        );
    }

    #[test]
    fn worktree_create_error_falls_back_to_message_when_detail_is_absent_or_blank() {
        assert_eq!(
            worktree_create_error_text("git failed to create the worktree".to_string(), None),
            "git failed to create the worktree"
        );
        assert_eq!(
            worktree_create_error_text(
                "git failed to create the worktree".to_string(),
                Some("   ".to_string())
            ),
            "git failed to create the worktree"
        );
    }

    fn summary(id: SessionId, title: &str, lifecycle: WireLifecycle) -> SessionSummary {
        summary_at(id, title, lifecycle, 0)
    }

    fn summary_at(
        id: SessionId,
        title: &str,
        lifecycle: WireLifecycle,
        input_serial: u64,
    ) -> SessionSummary {
        SessionSummary {
            id,
            worktree_dir: None,
            title: SessionLabel::Named(title.into()),
            lifecycle,
            activity: ActivitySignal::Unknown,
            input_serial,
        }
    }

    // --- T111 / FR-028a: seeding the stamper from the daemon's authoritative position (BUG-006) ---

    #[test]
    fn seeding_adopts_the_daemons_position_for_a_session_this_client_never_drove() {
        // The restarted-UI case: the stamper is empty because the process is new, but the session
        // has been taking input for a while. Its first keystroke must be stamped where the daemon
        // expects, not at 0 — which is what made every pre-existing session read-only.
        let id = SessionId::new();
        let snapshot = snapshot_with("/p", vec![summary_at(id, "s", WireLifecycle::Running, 40)]);
        let mut stamper = SessionInputStamper::new();

        stamper.seed_from_catalog(&snapshot);

        let ClientMsg::SessionInput { serial, .. } = stamper.stamp(id, b"x".to_vec()) else {
            panic!("stamp must produce SessionInput");
        };
        assert_eq!(
            serial, 40,
            "the first keystroke resumes at the daemon's mark"
        );
    }

    #[test]
    fn seeding_never_rewinds_a_counter_this_client_is_already_driving() {
        // A snapshot is a moment behind: input this client has already stamped may still be in
        // flight, so its counter is legitimately *ahead*. Adopting the older number would re-mint
        // serials the daemon has applied — the duplicate `Stale` exists to reject.
        let id = SessionId::new();
        let mut stamper = SessionInputStamper::new();
        for _ in 0..3 {
            stamper.stamp(id, b"x".to_vec());
        }

        let snapshot = snapshot_with("/p", vec![summary_at(id, "s", WireLifecycle::Running, 1)]);
        stamper.seed_from_catalog(&snapshot);

        let ClientMsg::SessionInput { serial, .. } = stamper.stamp(id, b"x".to_vec()) else {
            panic!("stamp must produce SessionInput");
        };
        assert_eq!(
            serial, 3,
            "the live counter continues; the stale snapshot is ignored"
        );
    }

    #[test]
    fn a_session_the_daemon_is_not_hosting_seeds_at_zero() {
        // No live entry means no `InputReceiver`, so the catalog's default stands. The client and
        // the daemon are both at 0, which is exactly in step.
        let id = SessionId::new();
        let snapshot = snapshot_with(
            "/p",
            vec![summary_at(id, "s", WireLifecycle::InterruptedResumable, 0)],
        );
        let mut stamper = SessionInputStamper::new();

        stamper.seed_from_catalog(&snapshot);

        let ClientMsg::SessionInput { serial, .. } = stamper.stamp(id, b"x".to_vec()) else {
            panic!("stamp must produce SessionInput");
        };
        assert_eq!(serial, 0);
    }

    fn snapshot_with(path: &str, sessions: Vec<SessionSummary>) -> CatalogSnapshot {
        CatalogSnapshot {
            schema_version: 1,
            last_active: Some(PathBuf::from(path)),
            projects: vec![ProjectSnapshot {
                path: PathBuf::from(path),
                display_name: "demo".into(),
                is_git_repo: true,
                available: true,
                worktrees: Vec::new(),
                sessions,
            }],
        }
    }

    #[test]
    fn reconcile_adds_updates_and_drops_sessions_from_the_snapshot() {
        let path = "/repo/demo";
        let mut core = State::default();

        // First snapshot: one Running session — added to the core.
        let a = SessionId::new();
        reconcile_catalog(
            &mut core,
            &snapshot_with(path, vec![summary(a, "A", WireLifecycle::Running)]),
            false,
        );
        let list = core.workspace.sessions.get(&PathBuf::from(path)).unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].id, a);
        assert_eq!(list[0].lifecycle, SessionLifecycle::Running);

        // Second snapshot: A is now Idle, and a new session B appears — A updated, B added.
        let b = SessionId::new();
        reconcile_catalog(
            &mut core,
            &snapshot_with(
                path,
                vec![
                    summary(a, "A", WireLifecycle::Idle),
                    summary(b, "B", WireLifecycle::Running),
                ],
            ),
            false,
        );
        let list = core.workspace.sessions.get(&PathBuf::from(path)).unwrap();
        assert_eq!(list.len(), 2);
        assert_eq!(
            list.iter().find(|s| s.id == a).unwrap().lifecycle,
            SessionLifecycle::Idle,
            "existing session's lifecycle is reconciled"
        );

        // Third snapshot: only B remains (A archived/removed on the daemon) — A is dropped, and a
        // dangling active pointer to A is cleared.
        core.active_session = Some(a);
        reconcile_catalog(
            &mut core,
            &snapshot_with(path, vec![summary(b, "B", WireLifecycle::Running)]),
            false,
        );
        let list = core.workspace.sessions.get(&PathBuf::from(path)).unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].id, b);
        assert_eq!(core.active_session, None, "dangling active pointer cleared");
    }

    // Convergence fix (retrofit session, 2026-07-27): a session transitioning to `Restarting` in
    // a background (inactive) project's snapshot must raise the FR-011/SC-007 return notice.
    // `State::note_background_restart` existed and was unit-tested in isolation
    // (`tests/background_restart.rs`), but nothing called it from the daemon-driven reconcile
    // path after feature 010 moved supervision into the daemon — so no background restart was
    // ever actually detected or notified.
    #[test]
    fn reconcile_detects_a_background_restart_and_arms_the_return_notice() {
        let mut core = State::default();
        core.workspace.active = Some(PathBuf::from("/b")); // /a is the background project

        let a = SessionId::new();
        reconcile_catalog(
            &mut core,
            &snapshot_with("/a", vec![summary(a, "A", WireLifecycle::Running)]),
            false,
        );
        assert!(core.restarted_while_inactive.is_empty());

        // /a's session crashes and the daemon starts restarting it, while /a is still inactive.
        reconcile_catalog(
            &mut core,
            &snapshot_with(
                "/a",
                vec![summary(a, "A", WireLifecycle::Restarting { attempts: 1 })],
            ),
            false,
        );
        assert!(
            core.restarted_while_inactive.contains(&a),
            "a background session's transition into Restarting must be detected and marked"
        );

        // A further Restarting snapshot (still retrying) must not re-mark or duplicate anything.
        reconcile_catalog(
            &mut core,
            &snapshot_with(
                "/a",
                vec![summary(a, "A", WireLifecycle::Restarting { attempts: 2 })],
            ),
            false,
        );
        assert_eq!(core.restarted_while_inactive.len(), 1);

        // Returning to /a fires the return notice (mirrors `background_restart.rs`).
        core.record_foreground();
        assert!(core.switch_active(Path::new("/a")));
        let visible = core
            .notify
            .visible()
            .expect("the return notice reached the queue");
        assert_eq!(visible.level, micold_core::notify::Level::Info);
        assert_eq!(
            visible.message,
            "A background session was restarted while you were away."
        );
    }

    #[test]
    fn maps_dark_light_mode_onto_system_scheme() {
        assert_eq!(
            map_system_scheme(dark_light::Mode::Dark),
            SystemScheme::Dark
        );
        assert_eq!(
            map_system_scheme(dark_light::Mode::Light),
            SystemScheme::Light
        );
        assert_eq!(
            map_system_scheme(dark_light::Mode::Unspecified),
            SystemScheme::Unspecified
        );
    }

    /// 003 FR-006 / SC-003: the theme poll must keep running while unfocused. It used to be
    /// dropped entirely, so a visible-but-unfocused window kept the wrong theme indefinitely —
    /// and leaving the app to change the OS theme is what unfocuses it in the first place.
    #[test]
    fn fr_006_os_theme_poll_never_stops_while_unfocused() {
        assert_eq!(os_theme_poll_interval(true), OS_THEME_POLL);
        assert_eq!(os_theme_poll_interval(false), BACKGROUND_OS_THEME_POLL);
    }

    /// SC-003 bounds the update at one second whether or not the window holds focus.
    #[test]
    fn sc_003_both_theme_poll_cadences_stay_within_one_second() {
        assert!(os_theme_poll_interval(true) <= Duration::from_secs(1));
        assert!(os_theme_poll_interval(false) <= Duration::from_secs(1));
    }

    /// Regression: `Subscription::map` requires a zero-sized (non-capturing) closure. Threading
    /// `last_known` through the closure captured it and crashed the app on startup under iced
    /// 0.13's `debug_assert!`; since 0.14 the same mistake is a `const {}` compile error, so this
    /// test now only pins the construction path — the capture itself can no longer reach runtime.
    #[test]
    fn os_theme_poll_builds_with_a_non_capturing_closure() {
        let _ = os_theme_poll(OS_THEME_POLL);
    }

    fn dummy_status() -> iced::event::Status {
        iced::event::Status::Ignored
    }

    /// Feature 015: cursor moves become `CursorMoved` so a switcher row's right-click can anchor
    /// its menu at the pointer; every other event is discarded before it reaches `update`.
    #[test]
    fn cursor_move_events_map_position_and_ignore_others() {
        let at = |x: f32, y: f32| {
            cursor_move_message(
                iced::Event::Mouse(iced::mouse::Event::CursorMoved {
                    position: iced::Point::new(x, y),
                }),
                dummy_status(),
                iced::window::Id::unique(),
            )
        };
        assert_eq!(
            at(412.0, 233.0),
            Some(Message::CursorMoved { x: 412, y: 233 })
        );
        // Off-window negatives clamp to 0 instead of wrapping the u16 cast.
        assert_eq!(at(-5.0, -1.0), Some(Message::CursorMoved { x: 0, y: 0 }));
        // Unrelated events are dropped.
        assert_eq!(
            cursor_move_message(
                iced::Event::Mouse(iced::mouse::Event::CursorLeft),
                dummy_status(),
                iced::window::Id::unique(),
            ),
            None
        );
    }

    #[test]
    fn window_focus_message_maps_focused_and_unfocused() {
        assert_eq!(
            window_focus_message(
                iced::Event::Window(iced::window::Event::Focused),
                dummy_status(),
                iced::window::Id::unique()
            ),
            Some(Message::WindowFocusChanged(true))
        );
        assert_eq!(
            window_focus_message(
                iced::Event::Window(iced::window::Event::Unfocused),
                dummy_status(),
                iced::window::Id::unique()
            ),
            Some(Message::WindowFocusChanged(false))
        );
    }

    #[test]
    fn window_focus_message_ignores_other_window_events() {
        assert_eq!(
            window_focus_message(
                iced::Event::Window(iced::window::Event::Closed),
                dummy_status(),
                iced::window::Id::unique()
            ),
            None
        );
        assert_eq!(
            window_focus_message(
                iced::Event::Window(iced::window::Event::RedrawRequested(
                    iced::time::Instant::now()
                )),
                dummy_status(),
                iced::window::Id::unique()
            ),
            None
        );
    }

    #[test]
    fn update_inner_applies_window_focus_changed() {
        let mut app = App {
            caps: Capabilities::real(),
            core: State::default(),
            grids: HashMap::new(),
            stamper: SessionInputStamper::new(),
            selection: None,
            display_offset: 0,
            scrollback_lines: micold_core::settings::DEFAULT_SCROLLBACK_LINES,
            dismissing: None,
            window_focused: true,
            last_grid: None,
            env_include_enabled: micold_core::settings::DEFAULT_ENV_INCLUDE_ENABLED,
            env_include_script_path: String::new(),
            env_include_timeout_secs: micold_core::settings::DEFAULT_ENV_INCLUDE_TIMEOUT_SECS,
            env_include_cache: HashMap::new(),
            env_include_last_outcome: EnvIncludeOutcome::Disabled,
            daemon: None,
            daemon_catalog: None,
            displaced: HashMap::new(),
            disconnected: false,
            version_mismatch: None,
            build_mismatch: None,
            next_req: 0,
            pending_ops: HashMap::new(),
            probe: None,
            scene_ready: false,
            scene_frames: 0,
            ripples_animating: Arc::new(AtomicUsize::new(0)),
            scene_ripple_frames: std::cell::Cell::new(0),
        };

        let _ = update_inner(&mut app, Message::WindowFocusChanged(false));
        assert!(!app.window_focused);

        let _ = update_inner(&mut app, Message::WindowFocusChanged(true));
        assert!(app.window_focused);
    }

    #[test]
    fn terminal_resized_remembers_the_pane_size_for_future_spawns() {
        // Reproduces the reported bug: a freshly spawned session used to always start at the
        // hardcoded INIT_ROWS x INIT_COLS default, filling only that fixed area until the next
        // window resize reconciled it. `TerminalResized` (published by the pane widget whenever
        // its live size changes) must now be remembered on `App` so `spawn_pty` call sites can
        // seed new sessions at the pane's actual current size instead.
        let mut app = App {
            caps: Capabilities::real(),
            core: State::default(),
            grids: HashMap::new(),
            stamper: SessionInputStamper::new(),
            selection: None,
            display_offset: 0,
            scrollback_lines: micold_core::settings::DEFAULT_SCROLLBACK_LINES,
            dismissing: None,
            window_focused: true,
            last_grid: None,
            env_include_enabled: micold_core::settings::DEFAULT_ENV_INCLUDE_ENABLED,
            env_include_script_path: String::new(),
            env_include_timeout_secs: micold_core::settings::DEFAULT_ENV_INCLUDE_TIMEOUT_SECS,
            env_include_cache: HashMap::new(),
            env_include_last_outcome: EnvIncludeOutcome::Disabled,
            daemon: None,
            daemon_catalog: None,
            displaced: HashMap::new(),
            disconnected: false,
            version_mismatch: None,
            build_mismatch: None,
            next_req: 0,
            pending_ops: HashMap::new(),
            probe: None,
            scene_ready: false,
            scene_frames: 0,
            ripples_animating: Arc::new(AtomicUsize::new(0)),
            scene_ripple_frames: std::cell::Cell::new(0),
        };
        assert_eq!(app.last_grid, None);

        let _ = update_inner(
            &mut app,
            Message::TerminalResized {
                cols: 220,
                rows: 60,
            },
        );
        assert_eq!(app.last_grid, Some((220, 60)));

        let _ = update_inner(
            &mut app,
            Message::TerminalResized {
                cols: 180,
                rows: 45,
            },
        );
        assert_eq!(app.last_grid, Some((180, 45)));
    }

    /// T061 (BUG-003, `006-real-terminal-emulator` FR-014a): displaying a session must state the
    /// pane's size *before* starting it, so the daemon spawns the process at that size instead of
    /// its 100×30 seed.
    ///
    /// The pane widget publishes `TerminalResized` only when its own size changes, so a session
    /// started into a window nobody is resizing was never told anything at all. The neighbouring
    /// `terminal_resized_remembers_the_pane_size_for_future_spawns` pins that `App::last_grid` is
    /// *stored*; nothing pinned that anything reads it, and nothing did — this closes that gap by
    /// asserting the message order on the wire.
    #[test]
    fn displaying_a_session_states_the_pane_size_before_starting_it() {
        let (tx, mut rx) = iced::futures::channel::mpsc::unbounded();
        let mut app = base_app();
        app.daemon = Some(micold_client::daemon::Outbox::new(tx));
        app.core.workspace.active = Some(std::path::PathBuf::from("/tmp/project"));
        let id = SessionId::new();
        app.last_grid = Some((220, 60));

        let _ = update_inner(&mut app, Message::SessionSelected(id));

        match rx.try_recv() {
            Ok(ClientMsg::SessionResize {
                session,
                cols,
                rows,
            }) => {
                assert_eq!(session, id);
                assert_eq!((cols, rows), (220, 60));
            }
            other => panic!("expected the size first, got {other:?}"),
        }
        assert!(
            matches!(
                rx.try_recv(),
                Ok(ClientMsg::SessionStart { session }) if session == id
            ),
            "the start must follow the size, not precede it"
        );
    }

    /// The pane has not been laid out yet (nothing has published a size): the start goes out alone
    /// and the daemon's own default applies. A zero-size guess here would be worse than no guess.
    #[test]
    fn displaying_a_session_before_the_pane_has_a_size_sends_only_the_start() {
        let (tx, mut rx) = iced::futures::channel::mpsc::unbounded();
        let mut app = base_app();
        app.daemon = Some(micold_client::daemon::Outbox::new(tx));
        app.core.workspace.active = Some(std::path::PathBuf::from("/tmp/project"));
        let id = SessionId::new();
        assert_eq!(app.last_grid, None);

        let _ = update_inner(&mut app, Message::SessionSelected(id));

        assert!(
            matches!(
                rx.try_recv(),
                Ok(ClientMsg::SessionStart { session }) if session == id
            ),
            "the first message is the start itself"
        );
    }

    /// Builds an `App` with every field at a neutral default, so each test only spells out the
    /// fields it actually varies (mirrors the literal-construction pattern the other tests in this
    /// module already use, factored out because T100's tests need several variants of it).
    fn base_app() -> App {
        App {
            caps: Capabilities::real(),
            core: State::default(),
            grids: HashMap::new(),
            stamper: SessionInputStamper::new(),
            selection: None,
            display_offset: 0,
            scrollback_lines: micold_core::settings::DEFAULT_SCROLLBACK_LINES,
            dismissing: None,
            window_focused: true,
            last_grid: None,
            env_include_enabled: micold_core::settings::DEFAULT_ENV_INCLUDE_ENABLED,
            env_include_script_path: String::new(),
            env_include_timeout_secs: micold_core::settings::DEFAULT_ENV_INCLUDE_TIMEOUT_SECS,
            env_include_cache: HashMap::new(),
            env_include_last_outcome: EnvIncludeOutcome::Disabled,
            daemon: None,
            daemon_catalog: None,
            displaced: HashMap::new(),
            disconnected: false,
            version_mismatch: None,
            build_mismatch: None,
            next_req: 0,
            pending_ops: HashMap::new(),
            probe: None,
            scene_ready: false,
            scene_frames: 0,
            ripples_animating: Arc::new(AtomicUsize::new(0)),
            scene_ripple_frames: std::cell::Cell::new(0),
        }
    }

    // --- T117 / FR-024a / SC-021: the read-only state must end when the daemon says we hold the
    // project (BUG-007) -----------------------------------------------------------------------

    /// An `App` holding `project` as its active project, with nothing else varied.
    fn app_on_project(project: &Path) -> App {
        let mut app = base_app();
        app.core.workspace.active = Some(project.to_path_buf());
        app
    }

    fn feed(app: &mut App, msg: DaemonMsg) {
        let _ = update_inner(app, Message::DaemonEvent(msg));
    }

    #[test]
    fn an_accepted_attach_ends_the_read_only_state_a_refusal_started() {
        // The reported sequence: this window is refused a project another window holds, the holder
        // later releases it, and the window reaches the project again by an ordinary switch — which
        // sends a *non-forced* `Attach`. The daemon accepts. Before the fix, `Attached` fell into
        // the catch-all arm, so the window rendered a project it owned while refusing to type into
        // it, above a takeover banner naming a window that may have exited.
        let project = PathBuf::from("/repo/demo");
        let mut app = app_on_project(&project);

        feed(
            &mut app,
            DaemonMsg::Refused {
                reason: micold_core::protocol::messages::RefusalReason::ProjectBusy {
                    project: project.clone(),
                    holder: "other-window".into(),
                    since_secs: 12,
                },
            },
        );
        assert!(
            active_project_displaced(&app),
            "a ProjectBusy refusal must make the window read-only (FR-023)"
        );

        feed(
            &mut app,
            DaemonMsg::Attached {
                project: project.clone(),
                sessions: vec![],
            },
        );

        assert!(
            !active_project_displaced(&app),
            "an accepted attach means the daemon says we hold it — input must flow again (FR-024a)"
        );
        assert_eq!(
            connection_status(&app),
            micold_client::features::connection::ConnectionStatus::Connected,
            "and no takeover affordance may be offered for a project we hold (SC-021)"
        );
    }

    #[test]
    fn an_accepted_attach_ends_the_read_only_state_a_takeover_started() {
        // The same fix from the other direction: this window *was* displaced by a real takeover
        // rather than refused up front. Once the taker releases the project and this window's own
        // attach is accepted, it is writable again — via the ordinary attach path, with no need to
        // press "Take over" and force a displacement of nobody.
        let project = PathBuf::from("/repo/demo");
        let mut app = app_on_project(&project);

        feed(
            &mut app,
            DaemonMsg::Displaced {
                project: project.clone(),
                by: "other-window".into(),
            },
        );
        assert!(
            active_project_displaced(&app),
            "a takeover makes us read-only (FR-024)"
        );

        feed(
            &mut app,
            DaemonMsg::Attached {
                project: project.clone(),
                sessions: vec![],
            },
        );
        assert!(!active_project_displaced(&app));
    }

    #[test]
    fn an_accepted_attach_clears_only_that_project() {
        // The map is per-project and must stay that way: being handed back one project says nothing
        // about another that a different window still holds.
        let mine = PathBuf::from("/repo/mine");
        let theirs = PathBuf::from("/repo/theirs");
        let mut app = app_on_project(&mine);

        for p in [&mine, &theirs] {
            feed(
                &mut app,
                DaemonMsg::Displaced {
                    project: p.clone(),
                    by: "other-window".into(),
                },
            );
        }

        feed(
            &mut app,
            DaemonMsg::Attached {
                project: mine.clone(),
                sessions: vec![],
            },
        );

        assert!(
            !app.displaced.contains_key(&mine),
            "the attached project is cleared"
        );
        assert!(
            app.displaced.contains_key(&theirs),
            "a project we did not attach to is untouched"
        );
    }

    #[test]
    fn a_refusal_after_an_attach_makes_the_window_read_only_again() {
        // The flag must move in both directions, not just the new one. Clearing on `Attached` must
        // not make the state sticky the other way: if we are later refused — because another window
        // took the project while we were away — the banner comes back.
        let project = PathBuf::from("/repo/demo");
        let mut app = app_on_project(&project);

        feed(
            &mut app,
            DaemonMsg::Attached {
                project: project.clone(),
                sessions: vec![],
            },
        );
        assert!(!active_project_displaced(&app));

        feed(
            &mut app,
            DaemonMsg::Displaced {
                project: project.clone(),
                by: "other-window".into(),
            },
        );
        assert!(
            active_project_displaced(&app),
            "the flag is a cache of daemon-reported ownership, not a latch in either direction"
        );
    }

    /// T100 (BUG-003 follow-up, FR-012a/FR-012b): saving Settings while connected to a daemon must
    /// ask it to apply the service-owned fields too — not just write `settings.json` locally — so
    /// the change takes effect for that daemon's already-running sessions immediately, rather than
    /// only after its next restart.
    #[test]
    fn settings_saved_sends_settings_set_to_a_connected_daemon() {
        let (tx, mut rx) = iced::futures::channel::mpsc::unbounded();
        let mut app = base_app();
        app.daemon = Some(micold_client::daemon::Outbox::new(tx));
        app.core.settings_draft = Some(SettingsDraft {
            scrollback_lines: "20000".into(),
            env_include_enabled: false,
            env_include_script_path: "/tmp/does-not-exist.sh".into(),
            env_include_timeout: "15".into(),
            error: None,
        });

        let _ = update_inner(&mut app, Message::SettingsSaved);

        match rx.try_recv() {
            Ok(ClientMsg::SettingsSet {
                scrollback_lines,
                env_include_enabled,
                env_include_script_path,
                env_include_timeout_secs,
                ..
            }) => {
                assert_eq!(scrollback_lines, Some(20_000));
                assert_eq!(env_include_enabled, Some(false));
                assert_eq!(
                    env_include_script_path,
                    Some("/tmp/does-not-exist.sh".to_string())
                );
                assert_eq!(env_include_timeout_secs, Some(15));
            }
            other => panic!("expected a queued SettingsSet, got {other:?}"),
        }
        // Exactly one — a save must not double-send.
        assert!(rx.try_recv().is_err(), "no second message queued");
    }

    /// The disconnected case is not an error: settings-saving already has a fully working local-only
    /// path (the direct `settings.json` write above the daemon-send in `Message::SettingsSaved`), so
    /// there is nothing to notify the user about — unlike every other `send_op`-routed mutation, which
    /// has no such standalone path.
    #[test]
    fn settings_saved_is_a_silent_no_op_toward_the_daemon_when_disconnected() {
        let mut app = base_app();
        assert!(app.daemon.is_none());
        app.core.settings_draft = Some(SettingsDraft {
            scrollback_lines: "20000".into(),
            env_include_enabled: true,
            env_include_script_path: String::new(),
            env_include_timeout: "15".into(),
            error: None,
        });

        let _ = update_inner(&mut app, Message::SettingsSaved);

        assert_eq!(
            app.scrollback_lines, 20_000,
            "the local field still updates"
        );
        assert!(app.pending_ops.is_empty(), "nothing was queued to send");
    }

    /// T100: a fresh connect (or reconnect) must adopt the daemon's authoritative env-include
    /// settings too, not just scrollback — the daemon is the single source of truth for both
    /// (FR-012a/FR-012b), and this client's own boot-time local read may already be stale relative
    /// to it (e.g. another window changed a setting first).
    #[test]
    fn daemon_connected_adopts_the_authoritative_env_include_settings() {
        let (tx, _rx) = iced::futures::channel::mpsc::unbounded();
        let mut app = base_app();
        app.env_include_enabled = true;
        app.env_include_script_path = "/tmp/stale-local-path.sh".into();
        app.env_include_timeout_secs = 10;

        let _ = update_inner(
            &mut app,
            Message::DaemonConnected {
                outbox: micold_client::daemon::Outbox::new(tx),
                catalog: snapshot_with("/repo/demo", Vec::new()),
                settings: micold_core::protocol::messages::DaemonSettings {
                    scrollback_lines: 12_345,
                    env_include_enabled: false,
                    env_include_script_path: "/authoritative/from-daemon.sh".into(),
                    env_include_timeout_secs: 30,
                },
            },
        );

        assert_eq!(app.scrollback_lines, 12_345);
        assert!(!app.env_include_enabled);
        assert_eq!(app.env_include_script_path, "/authoritative/from-daemon.sh");
        assert_eq!(app.env_include_timeout_secs, 30);
    }

    /// T100: a `SettingsChanged` push (this client's own `SettingsSet` echoed back, or another
    /// window's) must sync every service-owned field, not just scrollback — the whole point of
    /// sending `SettingsSet` at all is that the change takes effect without a restart.
    #[test]
    fn settings_changed_event_syncs_env_include_fields() {
        let mut app = base_app();
        app.env_include_enabled = true;
        app.env_include_script_path = "/tmp/before.sh".into();
        app.env_include_timeout_secs = 10;

        let _ = update_inner(
            &mut app,
            Message::DaemonEvent(DaemonMsg::SettingsChanged {
                settings: micold_core::protocol::messages::DaemonSettings {
                    scrollback_lines: 5_000,
                    env_include_enabled: false,
                    env_include_script_path: "/tmp/after.sh".into(),
                    env_include_timeout_secs: 45,
                },
            }),
        );

        assert_eq!(app.scrollback_lines, 5_000);
        assert!(!app.env_include_enabled);
        assert_eq!(app.env_include_script_path, "/tmp/after.sh");
        assert_eq!(app.env_include_timeout_secs, 45);
    }

    #[test]
    fn connection_status_orders_mismatch_over_displaced_over_disconnected() {
        // `connection_status` is decision/branching logic (Constitution I) picking which of five
        // mutually-possible states wins — pins the precedence directly rather than relying on it
        // only being exercised incidentally elsewhere (convergence finding F1, BUG-002).
        use micold_client::features::connection::ConnectionStatus;

        let mut app = App {
            caps: Capabilities::real(),
            core: State::default(),
            grids: HashMap::new(),
            stamper: SessionInputStamper::new(),
            selection: None,
            display_offset: 0,
            scrollback_lines: micold_core::settings::DEFAULT_SCROLLBACK_LINES,
            dismissing: None,
            window_focused: true,
            last_grid: None,
            env_include_enabled: micold_core::settings::DEFAULT_ENV_INCLUDE_ENABLED,
            env_include_script_path: String::new(),
            env_include_timeout_secs: micold_core::settings::DEFAULT_ENV_INCLUDE_TIMEOUT_SECS,
            env_include_cache: HashMap::new(),
            env_include_last_outcome: EnvIncludeOutcome::Disabled,
            daemon: None,
            daemon_catalog: None,
            displaced: HashMap::new(),
            disconnected: false,
            version_mismatch: None,
            build_mismatch: None,
            next_req: 0,
            pending_ops: HashMap::new(),
            probe: None,
            scene_ready: false,
            scene_frames: 0,
            ripples_animating: Arc::new(AtomicUsize::new(0)),
            scene_ripple_frames: std::cell::Cell::new(0),
        };

        assert_eq!(connection_status(&app), ConnectionStatus::Connected);

        app.disconnected = true;
        assert_eq!(connection_status(&app), ConnectionStatus::Disconnected);

        let project = PathBuf::from("/repo/demo");
        app.core.workspace.active = Some(project.clone());
        app.displaced.insert(project.clone(), "other-window".into());
        assert_eq!(
            connection_status(&app),
            ConnectionStatus::Displaced {
                by: "other-window".into()
            },
            "a takeover must win over a plain disconnect"
        );

        app.build_mismatch = Some(("client-1".into(), "daemon-0".into()));
        assert_eq!(
            connection_status(&app),
            ConnectionStatus::BuildMismatch {
                client_build: "client-1".into(),
                daemon_build: "daemon-0".into(),
            },
            "a same-contract build mismatch must win over a takeover"
        );

        app.version_mismatch = Some((2, 1, "daemon-0".into()));
        assert_eq!(
            connection_status(&app),
            ConnectionStatus::VersionMismatch {
                client: 2,
                daemon: 1,
                daemon_build: "daemon-0".into(),
            },
            "a wire-contract mismatch must win over a same-contract build mismatch"
        );
    }
}
