//! Micold AI IDE — GUI binary entry point.
//!
//! Adapts the render-free core (`micold_client::app`) to the iced runtime. All state
//! transitions live in the core and are unit-tested there; this layer renders state, performs
//! the feature's I/O at the boundary (filesystem scans, git worktree ops via `GitCli`), talks to
//! the session daemon over its connection (`micold_client::daemon`), and holds the gui-only
//! runtime that cannot live in the pure (Clone/Eq) core `State` — the per-session grid caches,
//! the input stamper, and the daemon outbox.

// A release build on Windows opens no console window of its own, for the app or for the daemon it
// starts (feature 030, FR-005). Debug builds keep the console so `cargo run` output stays visible.
#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

use iced::Task;
mod shell;

use crate::shell::capabilities::Capabilities;
use crate::shell::daemon_sync::PendingOp;
use micold_client::app::{Message, State};
use micold_client::features::help::Msg as HelpMsg;
use micold_client::features::project::Msg as ProjectMsg;
use micold_client::features::session::Msg as SessionMsg;
use micold_client::features::session::SelectKind;
use micold_client::features::worktree::Msg as WorktreeMsg;
use micold_client::features::worktree_form::Msg as FormMsg;
use micold_client::grid::GridCache;
use micold_client::input::SessionInputStamper;
use micold_client::overlay::registry::Closing;
use micold_client::selection::{Anchor, SelectGranularity, Selection};
use micold_core::env_include::{EnvIncludeOutcome, EnvIncludeSnapshot};
use micold_core::frame_probe::{
    FrameProbe, ProbeConfig, Scene, SceneFacts, ENV_VAR as FRAME_PROBE_ENV,
    SCENE_ENV_VAR as FRAME_PROBE_SCENE_ENV,
};
use micold_core::protocol::grid::LineId;
use micold_core::protocol::messages::ClientMsg;
use micold_core::session::{SessionId, SessionLocation, ShellInstanceId, TerminalMode};

use micold_core::theme::observe_system_scheme;
use std::cell::RefCell;
use std::collections::HashMap;
use std::ops::Range;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, OnceLock};
use std::time::Instant;

/// The binary's application state: the pure core plus gui-only runtime handles.
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
    /// The terminal pane's last-known `(cols, rows)`, reported by `Message::Session(SessionMsg::TerminalResized)`.
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
    /// Projects this window may not write to because another window holds them, keyed by project
    /// path → who holds it and which of the two events said so (US5, FR-023/FR-024): a takeover
    /// this window lost, or an attach it was refused. Read-only here (input suppressed, a banner
    /// shown) until the user takes it back or reconnects. Cleared on a fresh connect. Empty in the
    /// common single-window case.
    ///
    /// The cause is stored rather than derived because it is not recoverable afterwards — the
    /// banner would otherwise have to describe one event in the words of the other, which is what
    /// it did (`010` BUG-023).
    displaced: HashMap<PathBuf, micold_client::features::connection::Hold>,
    /// Whether the daemon connection is currently down (between a `DaemonDisconnected` and the next
    /// `DaemonConnected`). Drives the stale-content banner (FR-027). `daemon.is_none()` also implies
    /// this, but the flag is explicit for clarity at the render site.
    disconnected: bool,
    /// Where the daemon runs, resolved once at boot from the settings the shell loaded.
    ///
    /// Held here rather than read by the connection subscription, because the shell is the single
    /// place that chooses a real settings store (FR-017/FR-018) — a rule
    /// `tests/no_concrete_implementations.rs` enforces, and which caught the first version of this.
    placement: micold_client::daemon::Placement,
    /// The sandbox's state, when the daemon is placed in one (feature 027).
    ///
    /// Always present, `Disabled` for the host placement — so the persistent-notice check is one
    /// call rather than an `Option` every render site has to remember to unwrap.
    sandbox: micold_client::features::sandbox::Sandbox,
    /// What a restart of the sandbox would run, when there is a sandbox to restart (R9).
    ///
    /// `None` for the host placement. Its project list is refreshed from the daemon's catalog, so
    /// a restart shares the projects registered *now* rather than the ones registered at boot
    /// (M-4).
    sandbox_boot: Option<shell::sandbox::BootPlan>,
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
    /// Scrollback ranges asked for and not yet answered, keyed by `req` (`010` BUG-021).
    ///
    /// A scroll gesture computes the same un-cached run on every wheel notch, because nothing it
    /// asked for has arrived yet. Without this, each notch sent that run again; three seconds of
    /// scrolling queued hundreds of overlapping ranges the daemon then ground through in silence.
    /// A range recorded here is one [`GridCache::needed_scrollback`] treats as already on its way,
    /// so a request covers a viewport of travel and the notches in between send nothing.
    ///
    /// Entries are removed by the matching `ScrollbackResponse`, and dropped wholesale on
    /// disconnect (`req`s are per-connection) and with the session's grid.
    scrollback_inflight: HashMap<u64, (SessionId, Range<LineId>)>,
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
        worktrees: app.core.worktree.worktrees.len(),
        running_sessions,
        dialog_open: micold_client::overlay::registry::open_dialog(&app.core).is_some(),
        context_menu_open: app.core.session.terminal_context_menu.is_some(),
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
            steps.push(Task::done(Message::Session(SessionMsg::StartRequested {
                location: SessionLocation::Default,
                provider: app.core.session.provider_for_start(None),
            })));
        }
    }
    if !facts.dialog_open {
        steps.push(Task::done(Message::Help(HelpMsg::AboutOpened)));
    }
    if !facts.context_menu_open {
        let (x, y) = SCENE_MENU_AT;
        steps.push(Task::done(Message::Session(
            SessionMsg::TerminalContextMenuOpened { x, y },
        )));
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
        self.grids.get(&self.core.session.active?)
    }
}

pub fn main() -> iced::Result {
    // Held for the whole process: the installer's `AppMutex` check sees the app while it runs.
    let _running = micold_core::process::announce_running();
    // Before anything connects or spawns (packaging contract §2.7). An upgrade from a release that
    // shipped the units leaves a per-user enablement behind that dpkg cannot reach, and a stale
    // enablement plus a unit file the upgrade deleted is a socket activation that starts nothing.
    // Silent, and free when there is nothing to clear — see `shell::legacy_units`.
    shell::legacy_units::disable_legacy_units();
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

    // Feature 026 FR-002d: the same shape, one scroll region over. Drained here for the same
    // reason — the viewport's width arrives with layout, not with the selection.
    let task = match tab_reveal_scroll(app) {
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
    if !app.core.sidebar.pending_reveal_scroll
        || app.core.sidebar.viewport_height == 0
        || !app.core.current_session_is_listed()
    {
        if app.core.sidebar.pending_reveal_scroll && micold_client::reveal_trace::enabled() {
            micold_client::reveal_trace::line(format_args!(
                "armed, waiting: viewport_h={} listed={}",
                app.core.sidebar.viewport_height,
                app.core.current_session_is_listed()
            ));
        }
        return None;
    }
    app.core.sidebar.pending_reveal_scroll = false;
    let offset = app.core.reveal_scroll_offset();
    if micold_client::reveal_trace::enabled() {
        match offset {
            Some(y) => micold_client::reveal_trace::line(format_args!(
                "drained: viewport_h={} scroll_offset={} -> scrolling to {y}",
                app.core.sidebar.viewport_height, app.core.sidebar.scroll_offset,
            )),
            None => micold_client::reveal_trace::line(format_args!(
                "drained: viewport_h={} scroll_offset={} -> no scroll, the row is already visible \
                 (FR-009)",
                app.core.sidebar.viewport_height, app.core.sidebar.scroll_offset,
            )),
        }
    }
    let offset = offset?;
    // Record where the list was sent, rather than waiting to be told (BUG-002).
    //
    // `scroll_offset` is a mirror of the scrollable's position whose only writer is
    // `Message::Sidebar(SidebarMsg::Scrolled)` — and the rendering stack publishes that from `notify_viewport`,
    // which returns *without publishing* whenever the content fits the viewport
    // (`iced_widget/src/scrollable.rs`). A reveal in a project whose sidebar fits therefore moves
    // the list silently, and the mirror keeps the offset of the project before it. The next
    // arrival then measures its row against a position the panel is nowhere near, concludes the
    // row is already visible, and consumes the arm under FR-009 — the deepest row in a 30-location
    // list left below the fold with nothing to move it.
    //
    // The application does not need to be told what it just did. `offset` is already clamped into
    // this list's own scrollable range by `scroll_target`, so it is the position the panel will
    // hold whether or not a notification follows.
    app.core.sidebar.scroll_offset = offset;
    Some(iced::widget::operation::scroll_to(
        micold_client::ui::SIDEBAR_SCROLL_ID.clone(),
        iced::widget::scrollable::AbsoluteOffset {
            x: 0.0,
            y: offset as f32,
        },
    ))
}

/// Scroll the marked tab into view, once there is a viewport to scroll it in (feature 026 FR-002d).
///
/// Deferred for the same two reasons [`reveal_scroll`] is: the viewport reports its width only once
/// laid out, and `0` there means "unknown", never "nothing fits" — nothing is scrolled on a guess.
///
/// The offset may still be `None` once it drains: the marked tab was already fully visible, and
/// FR-002d's whole point is that a user may scroll away from it by hand. A reveal that fired on
/// every selection would yank them back each time, including on selections made with the mode
/// toggle rather than with the strip.
fn tab_reveal_scroll(app: &mut App) -> Option<Task<Message>> {
    if !app.core.session.pending_tab_reveal || app.core.session.tab_strip_viewport_width == 0 {
        return None;
    }
    app.core.session.pending_tab_reveal = false;
    let index = micold_client::ui::terminal::marked_tab_index(&app.core)?;
    let offset = micold_client::ui::terminal::scroll_into_view(
        index,
        app.core.session.tab_strip_scroll_offset as f32,
        app.core.session.tab_strip_viewport_width as f32,
    )?;
    Some(iced::widget::operation::scroll_to(
        micold_client::ui::terminal::TAB_STRIP_SCROLL_ID.clone(),
        iced::widget::scrollable::AbsoluteOffset { x: offset, y: 0.0 },
    ))
}

fn update_inner(app: &mut App, message: Message) -> Task<Message> {
    match message {
        // ---- Feature 010: daemon connection lifecycle (binary-owned runtime state) ----
        // Twelve arms until T011. All twelve were effects, so all twelve are `shell/connection.rs`
        // now (contract M2) and the routing decision is stated once, next to them.
        Message::Connection(msg) => shell::connection::update(app, msg),
        // ---- Feature 027: the session service inside a container ----
        // Six arms until T011, and the same story as the twelve above: every one of them was an
        // effect or a write to the binary-owned `app.sandbox`, so all six are
        // `shell/sandbox.rs` now (contract M2).
        Message::Sandbox(msg) => shell::sandbox::update(app, msg),
        // Feature 027, FR-030. The one thing the reducer cannot do: focus belongs to the widget
        // tree, so moving it is an operation issued from here. Every input in the application
        // already implements iced's `Focusable` — what was missing was anyone asking.
        // The move itself, then the second clause of FR-030: a control the traversal reached
        // below the fold is focused-but-invisible until something scrolls to it, and iced's focus
        // operations never look at a scrollable. Chained rather than batched, because the scroll
        // has to read the focus the move just set.
        Message::FocusMoved { forward } => {
            let moved = if forward {
                iced::widget::operation::focus_next()
            } else {
                iced::widget::operation::focus_previous()
            };
            moved.chain(micold_client::ui::scroll_focused_into_view())
        }

        // The closing dialog has finished animating out; its snapshot has served its purpose.
        Message::OverlayTransitionFinished => {
            app.dismissing = None;
            Task::none()
        }
        Message::Project(ProjectMsg::SelectorOpened) => {
            shell::workspace::on_project_selector_opened(app)
        }
        Message::Project(
            msg @ (ProjectMsg::SelectorNavigatedInto(_) | ProjectMsg::SelectorNavigatedUp),
        ) => shell::workspace::on_selector_navigated(app, msg),
        Message::Project(ProjectMsg::FolderChosen(path)) => {
            shell::workspace::on_folder_chosen(app, path)
        }
        Message::Project(ProjectMsg::Reopened(path)) => {
            shell::workspace::on_known_project_reopened(app, path)
        }
        Message::Project(ProjectMsg::SwitcherToggled) => shell::workspace::on_switcher_toggled(app),
        Message::Project(ProjectMsg::RenameConfirmed) => {
            shell::daemon_sync::on_rename_confirmed(app)
        }
        Message::Project(ProjectMsg::ForgetConfirmed) => {
            shell::daemon_sync::on_project_forget_confirmed(app)
        }
        Message::Worktree(WorktreeMsg::RenameConfirmed) => {
            shell::daemon_sync::on_worktree_rename_confirmed(app)
        }
        // Feature 029 FR-020. The same split `RenameConfirmed` establishes: the pure reducer
        // applies the record, this half makes it durable.
        Message::Worktree(WorktreeMsg::ClaimRequested(dir_name)) => {
            shell::daemon_sync::on_worktree_claim_requested(app, dir_name)
        }
        Message::WorktreeForm(FormMsg::Submitted) => {
            shell::daemon_sync::on_add_worktree_submitted(app)
        }
        Message::WorktreeForm(FormMsg::ResolutionChosen(mode)) => {
            shell::daemon_sync::on_add_worktree_resolution_chosen(app, mode)
        }
        Message::WorktreeForm(FormMsg::OverwriteConfirmed) => {
            shell::daemon_sync::on_add_worktree_overwrite_confirmed(app)
        }
        Message::WorktreeForm(FormMsg::SourceChanged(source)) => {
            shell::daemon_sync::on_add_worktree_source_changed(app, source)
        }
        Message::Session(SessionMsg::StartRequested { location, provider }) => {
            shell::daemon_sync::on_session_start_requested(app, location, provider)
        }
        // The override list is opening: refresh the availability set first (feature 026, T014a).
        // This and `Settings(Opened)` are the two named events research R11 means by "when the
        // choice is offered" — the set is never re-probed per frame, which would be a `PATH`
        // lookup per render and exactly the scheduled work SC-006 forbids.
        Message::Session(SessionMsg::StartMenuOpened {
            location,
            unavailable_default,
        }) => {
            shell::daemon_sync::ask_cli_availability(app);
            app.core
                .update(Message::Session(SessionMsg::StartMenuOpened {
                    location,
                    unavailable_default,
                }));
            Task::none()
        }
        Message::Session(SessionMsg::Selected(id)) => {
            shell::daemon_sync::on_session_selected(app, id)
        }
        Message::Session(SessionMsg::CloseRequested(id)) => {
            shell::daemon_sync::on_session_close_requested(app, id)
        }
        Message::Session(SessionMsg::RemoveConfirmed) => {
            shell::daemon_sync::on_session_remove_confirmed(app)
        }
        Message::Session(SessionMsg::TerminalAiCliSelected(id)) => {
            shell::daemon_sync::on_terminal_ai_cli_selected(app, id)
        }
        Message::Session(SessionMsg::TerminalRestartRequested) => {
            shell::daemon_sync::on_terminal_restart_requested(app)
        }
        Message::Session(SessionMsg::ShellInstanceRestartRequested(id, shell_id)) => {
            shell::daemon_sync::on_shell_instance_restart_requested(app, id, shell_id)
        }
        Message::Session(SessionMsg::ShellInstanceOpenRequested) => {
            shell::daemon_sync::on_shell_instance_open_requested(app)
        }
        Message::Session(SessionMsg::ShellInstanceCloseRequested(id, shell_id)) => {
            shell::daemon_sync::on_shell_instance_close_requested(app, id, shell_id)
        }
        Message::Session(SessionMsg::ShellInstanceSelected(id, shell_id)) => {
            shell::daemon_sync::on_shell_instance_selected(app, id, shell_id)
        }
        Message::Session(SessionMsg::TerminalBytes(bytes)) => {
            shell::daemon_sync::on_terminal_bytes(app, bytes)
        }
        // Mouse text selection on the displayed session's grid, anchored to absolute `LineId`s so
        // new output can't corrupt it (FR-013/FR-018).
        Message::Session(SessionMsg::TerminalSelectStart { col, line, kind }) => {
            if let Some(id) = app.core.session.active {
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
        Message::Session(SessionMsg::TerminalSelectUpdate { col, line }) => {
            if let Some(id) = app.core.session.active {
                if let (Some(grid), Some(sel)) = (app.grids.get(&id), app.selection.as_mut()) {
                    let anchor = Anchor::new(row_line_id(grid, app.display_offset, line), col);
                    sel.update(anchor, |id| grid.line(id).map(|l| l.text.clone()));
                }
            }
            Task::none()
        }
        Message::Session(SessionMsg::TerminalSelectCleared) => {
            app.selection = None;
            Task::none()
        }
        Message::Session(SessionMsg::TerminalResized { cols, rows }) => {
            shell::daemon_sync::on_terminal_resized(app, cols, rows)
        }
        // Scroll the displayed session's scrollback view (FR-016). Offset is clamped to the cached
        // history; deeper history is fetched from the daemon on demand (see `request_scrollback`).
        Message::Session(SessionMsg::TerminalScrolled(delta)) => {
            scroll_view(app, |off, history| {
                (off as i32 + delta).clamp(0, history as i32) as usize
            });
            Task::none()
        }
        // Scroll to an absolute offset (scrollbar drag). Resolve against the LIVE offset at apply
        // time so a burst of batched drag messages converges (drag flicker fix, FR-016).
        Message::Session(SessionMsg::TerminalScrolledTo(target)) => {
            scroll_view(app, |off, history| {
                let delta = micold_client::ui::target_offset_delta(off, target);
                (off as i32 + delta).clamp(0, history as i32) as usize
            });
            Task::none()
        }
        Message::Session(SessionMsg::TerminalCopyRequested) => {
            shell::clipboard::on_copy_requested(app)
        }
        Message::Session(SessionMsg::TerminalPasteRequested) => {
            shell::clipboard::on_paste_requested(app)
        }
        Message::Worktree(WorktreeMsg::TextCopyRequested(text)) => {
            shell::clipboard::on_text_copy_requested(app, text)
        }
        Message::Settings(msg) => shell::settings::update(app, msg),
        Message::Session(SessionMsg::TerminalTick) => {
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
            shell::os_theme::redetect_on_focus(app, focused);
            Task::none()
        }
        Message::Worktree(WorktreeMsg::DeleteConfirmed) => {
            shell::daemon_sync::on_worktree_delete_confirmed(app)
        }
        Message::Worktree(WorktreeMsg::IncludeRequested(path)) => {
            shell::daemon_sync::on_worktree_include_requested(app, path)
        }
        Message::Worktree(WorktreeMsg::ExcludeRequested(dir)) => {
            shell::daemon_sync::on_worktree_exclude_requested(app, dir)
        }
        Message::Worktree(WorktreeMsg::RefreshRequested) => {
            shell::daemon_sync::on_worktree_refresh_requested(app)
        }
        // The bounded wait fired. Routed here rather than falling through to the reducer because
        // whether *this* timer still names the live request is a question about correlation, and
        // correlation lives in the shell.
        Message::Worktree(WorktreeMsg::RefreshTimedOut(req)) => {
            shell::daemon_sync::on_worktree_refresh_timed_out(app, req)
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
        &app.sandbox,
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
    let hold = app
        .core
        .workspace
        .active
        .as_ref()
        .and_then(|project| app.displaced.get(project));

    micold_client::features::connection::connection_status(
        app.version_mismatch.as_ref(),
        app.build_mismatch.as_ref(),
        hold,
        // Not listening *yet* is not disconnected: the sandbox view shows the stage, and a banner
        // saying the service is gone would call a working bring-up broken (FR-036b).
        app.disconnected && !app.sandbox.state.is_coming_up(),
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
/// scrollback the cache doesn't yet hold and hasn't already asked for (`ScrollbackRequest`).
/// `history` is the daemon's full retained depth (`viewport_top - oldest_available`), so the view
/// can scroll into history; un-fetched lines render blank until the `ScrollbackResponse` fills them
/// (FR-016/017).
///
/// **Which range to ask for is [`GridCache::needed_scrollback`]'s decision, not this function's**
/// (`010` BUG-021). It used to be made here, from the revealed line to the live tail, and that is
/// the range whose size grows with scroll depth. Moving it left this function with the two things
/// that genuinely need the shell — the correlation id and the socket.
fn scroll_view(app: &mut App, f: impl FnOnce(usize, usize) -> usize) {
    let Some(id) = app.core.session.active else {
        return;
    };
    let inflight: Vec<Range<LineId>> = app
        .scrollback_inflight
        .values()
        .filter(|(session, _)| *session == id)
        .map(|(_, range)| range.clone())
        .collect();
    let (new_off, needed) = {
        let Some(grid) = app.grids.get(&id) else {
            return;
        };
        let history = (grid.viewport_top().0 - grid.oldest_available().0).max(0) as usize;
        let new_off = f(app.display_offset, history);
        (new_off, grid.needed_scrollback(new_off, &inflight))
    };
    app.display_offset = new_off;
    if let Some(range) = needed {
        let req = app.next_req;
        app.next_req += 1;
        if let Some(d) = &app.daemon {
            d.send(ClientMsg::ScrollbackRequest {
                session: id,
                req,
                ranges: vec![range.clone()],
            });
            // Recorded only once it is actually on the wire. A range marked in flight with no
            // request behind it is a range nothing will ever answer, and those rows would stay
            // blank for as long as the view stayed there.
            app.scrollback_inflight.insert(req, (id, range));
        }
    }
}

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
    let choice = &app.core.session.last_foreground_choice;
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
        app.core.session.active,
        choice,
        micold_core::project::canonicalize_best_effort(path).display(),
        keys,
    ));
}

/// Perform a feature's effect request (feature 021, T045 — FR-015a, contract C3).
///
/// Translation and nothing else: one arm per variant, no branch that could have gone the other
/// way. What reaches the clipboard, and whether anything should, was decided by the feature that
/// emitted the request — which is the whole point of expressing the request instead of the call.
/// Ask the selection feature what copying it should put on the clipboard.
///
/// Only the grid lookup is here — finding the displayed session's cached lines is reading the
/// shell's own data, not a rule about copying. Without a grid there is nothing to resolve the
/// selection against, so there is no selection to offer, which is what `selected_text` said by
/// returning an empty string before the request had a type.
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

// The binary's unit tests live beside it; `crate::tests::base_app` is shared with `shell/*` tests.
#[cfg(test)]
#[path = "main_tests.rs"]
pub(crate) mod tests;
