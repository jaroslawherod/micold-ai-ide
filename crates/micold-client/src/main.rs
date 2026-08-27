//! Micold AI IDE — GUI binary entry point.
//!
//! Adapts the render-free core (`micold_client::app`) to the iced runtime. All state
//! transitions live in the core and are unit-tested there; this layer renders state, performs
//! the feature's I/O at the boundary (filesystem scans, git worktree ops via `GitCli`), talks to
//! the session daemon over its connection (`micold_client::daemon`), and holds the gui-only
//! runtime that cannot live in the pure (Clone/Eq) core `State` — the per-session grid caches,
//! the input stamper, and the daemon outbox.

use iced::Task;
mod shell;

use crate::shell::capabilities::Capabilities;
use crate::shell::daemon_sync::PendingOp;
use micold_client::app::{Message, State};
use micold_client::features::session::SelectKind;
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
                provider: app.core.provider_for_start(None),
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
    if !app.core.pending_reveal_scroll
        || app.core.sidebar_viewport_height == 0
        || !app.core.current_session_is_listed()
    {
        if app.core.pending_reveal_scroll && micold_client::reveal_trace::enabled() {
            micold_client::reveal_trace::line(format_args!(
                "armed, waiting: viewport_h={} listed={}",
                app.core.sidebar_viewport_height,
                app.core.current_session_is_listed()
            ));
        }
        return None;
    }
    app.core.pending_reveal_scroll = false;
    let offset = app.core.reveal_scroll_offset();
    if micold_client::reveal_trace::enabled() {
        match offset {
            Some(y) => micold_client::reveal_trace::line(format_args!(
                "drained: viewport_h={} scroll_offset={} -> scrolling to {y}",
                app.core.sidebar_viewport_height, app.core.sidebar_scroll_offset,
            )),
            None => micold_client::reveal_trace::line(format_args!(
                "drained: viewport_h={} scroll_offset={} -> no scroll, the row is already visible \
                 (FR-009)",
                app.core.sidebar_viewport_height, app.core.sidebar_scroll_offset,
            )),
        }
    }
    let offset = offset?;
    // Record where the list was sent, rather than waiting to be told (BUG-002).
    //
    // `sidebar_scroll_offset` is a mirror of the scrollable's position whose only writer is
    // `Message::SidebarScrolled` — and the rendering stack publishes that from `notify_viewport`,
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
    app.core.sidebar_scroll_offset = offset;
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
    if !app.core.pending_tab_reveal || app.core.tab_strip_viewport_width == 0 {
        return None;
    }
    app.core.pending_tab_reveal = false;
    let index = micold_client::ui::terminal::marked_tab_index(&app.core)?;
    let offset = micold_client::ui::terminal::scroll_into_view(
        index,
        app.core.tab_strip_scroll_offset as f32,
        app.core.tab_strip_viewport_width as f32,
    )?;
    Some(iced::widget::operation::scroll_to(
        micold_client::ui::terminal::TAB_STRIP_SCROLL_ID.clone(),
        iced::widget::scrollable::AbsoluteOffset { x: offset, y: 0.0 },
    ))
}

fn update_inner(app: &mut App, message: Message) -> Task<Message> {
    match message {
        // ---- Feature 010: daemon connection lifecycle (binary-owned runtime state) ----
        Message::DaemonConnected {
            outbox,
            catalog,
            settings,
        } => shell::daemon_sync::on_connected(app, outbox, catalog, settings),
        Message::DaemonEvent(event) => shell::daemon_sync::on_daemon_event(app, event),
        Message::DaemonGridFrame(frame) => shell::daemon_sync::on_grid_frame(app, frame),
        Message::DaemonDisconnected => shell::daemon_sync::on_disconnected(app),
        Message::DaemonConnectFailed(reason) => shell::daemon_sync::on_connect_failed(app, reason),
        // Feature 027. The sandbox's outcome is recorded, never acted on: a failure must not start
        // a session somewhere else, and a success needs no prompting because the connection
        // subscription is already retrying against the loopback port.
        Message::SandboxStarted(started) => {
            app.sandbox.started(*started);
            Task::none()
        }
        Message::SandboxFailed(failure) => {
            // Recorded and left standing. The banner draws it for as long as it lasts — the queue
            // would show it once, for four seconds, while the sandbox stayed broken (FR-035b, S-3),
            // which is exactly the edge case the spec calls out.
            app.sandbox.failed(*failure);
            Task::none()
        }
        Message::SandboxDiagnostics(lines) => {
            match lines.last() {
                // Phrased like the connected path's `RecentErrors` answer, because from the user's
                // side it is the same question — only the route it took to be answered differs.
                Some(latest) => app.core.notify_error(format!(
                    "The session service logged {} line(s) inside the sandbox; most recent: {}",
                    lines.len(),
                    latest.trim()
                )),
                None => app.core.notify_info(
                    "The sandbox is there, but the session service inside it has logged nothing.",
                ),
            }
            Task::none()
        }
        Message::SandboxLost => {
            app.sandbox.container_lost(shell::sandbox::CONTAINER_NAME);
            Task::none()
        }
        // The one edge back into bring-up, and it is here because a person pressed something
        // (R9, FR-035a). Both of these are user actions; neither is ever sent by the application
        // to itself.
        Message::SandboxRestartRequested => {
            match app.sandbox_boot.clone() {
                Some(plan)
                    if app
                        .sandbox
                        .restart(micold_core::sandbox::lifecycle::RestartRequested) =>
                {
                    // Going back to the sandbox has to take the connection with it. If a fallback
                    // was in force, `placement.kind` is the host process; left there, the sandbox
                    // would come up and every session would still be running outside it — the
                    // banner saying "sandboxed" over an unconfined shell, which is the one thing
                    // FR-035b exists to prevent.
                    app.placement.kind =
                        micold_core::sandbox::placement::PlacementKind::LocalSandbox;
                    shell::sandbox::boot(plan)
                }
                _ => Task::none(),
            }
        }
        Message::SandboxFallbackAccepted => {
            if let Some(offer) = app.sandbox.fallback_offer() {
                // Consent is not the whole of it. `daemon::connection` dials from `app.placement`,
                // and for `LocalSandbox` it deliberately never falls back to a host process
                // (FR-035, and the comment in `daemon.rs` says so). So the *only* thing that can
                // turn accepted consent into a working service is moving the placement here —
                // without this the user presses "Run without it for now", the banner changes, and
                // the client goes on dialling a port nothing is listening on, forever.
                //
                // In memory only: nothing writes it back to the settings store, which is what
                // makes the choice last for this occurrence alone (FR-035a).
                if app.sandbox.accept_fallback(offer) {
                    app.placement.kind =
                        micold_core::sandbox::placement::PlacementKind::HostProcess;
                }
            }
            Task::none()
        }
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
        Message::ConnectionTakeoverRequested => shell::daemon_sync::on_takeover_requested(app),
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
        Message::ConnectionRestartServiceRequested => {
            shell::service_control::on_restart_service_requested(app)
        }
        Message::LogoutSurvivalRequested => shell::service_control::on_logout_survival_requested(),
        Message::LogoutSurvivalOutcome(message) => {
            shell::service_control::on_logout_survival_outcome(app, message)
        }
        Message::DiagnosticsRequested => shell::daemon_sync::on_diagnostics_requested(app),

        // The closing dialog has finished animating out; its snapshot has served its purpose.
        Message::OverlayTransitionFinished => {
            app.dismissing = None;
            Task::none()
        }
        Message::ProjectSelectorOpened => shell::workspace::on_project_selector_opened(app),
        Message::SelectorNavigatedInto(_) | Message::SelectorNavigatedUp => {
            shell::workspace::on_selector_navigated(app, message)
        }
        Message::FolderChosen(path) => shell::workspace::on_folder_chosen(app, path),
        Message::KnownProjectReopened(path) => {
            shell::workspace::on_known_project_reopened(app, path)
        }
        Message::RenameConfirmed => shell::daemon_sync::on_rename_confirmed(app),
        Message::ProjectForgetConfirmed => shell::daemon_sync::on_project_forget_confirmed(app),
        Message::WorktreeRenameConfirmed => shell::daemon_sync::on_worktree_rename_confirmed(app),
        Message::ThemePreferenceChanged(_) | Message::ThemeModeCycled => {
            shell::persist::on_theme_changed(app, message)
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
        Message::SessionStartRequested { location, provider } => {
            shell::daemon_sync::on_session_start_requested(app, location, provider)
        }
        // The override list is opening: refresh the availability set first (feature 026, T014a).
        // This and `SettingsOpened` are the two named events research R11 means by "when the choice
        // is offered" — the set is never re-probed per frame, which would be a `PATH` lookup per
        // render and exactly the scheduled work SC-006 forbids.
        Message::SessionStartMenuOpened(location) => {
            app.core.available_providers = app.caps.available_providers();
            app.core.update(Message::SessionStartMenuOpened(location));
            Task::none()
        }
        Message::SessionSelected(id) => shell::daemon_sync::on_session_selected(app, id),
        Message::SessionCloseRequested(id) => {
            shell::daemon_sync::on_session_close_requested(app, id)
        }
        Message::SessionRemoveConfirmed => shell::daemon_sync::on_session_remove_confirmed(app),
        Message::TerminalAiCliSelected(id) => {
            shell::daemon_sync::on_terminal_ai_cli_selected(app, id)
        }
        Message::TerminalRestartRequested => shell::daemon_sync::on_terminal_restart_requested(app),
        Message::ShellInstanceRestartRequested(id, shell_id) => {
            shell::daemon_sync::on_shell_instance_restart_requested(app, id, shell_id)
        }
        Message::ShellInstanceOpenRequested => {
            shell::daemon_sync::on_shell_instance_open_requested(app)
        }
        Message::ShellInstanceCloseRequested(id, shell_id) => {
            shell::daemon_sync::on_shell_instance_close_requested(app, id, shell_id)
        }
        Message::ShellInstanceSelected(id, shell_id) => {
            shell::daemon_sync::on_shell_instance_selected(app, id, shell_id)
        }
        Message::TerminalBytes(bytes) => shell::daemon_sync::on_terminal_bytes(app, bytes),
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
        Message::TerminalResized { cols, rows } => {
            shell::daemon_sync::on_terminal_resized(app, cols, rows)
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
        Message::TerminalCopyRequested => shell::clipboard::on_copy_requested(app),
        Message::TerminalPasteRequested => shell::clipboard::on_paste_requested(app),
        Message::TextCopyRequested(text) => shell::clipboard::on_text_copy_requested(app, text),
        Message::SettingsOpened => shell::persist::on_settings_opened(app),
        Message::SettingsSaved => shell::persist::on_settings_saved(app),
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
            shell::os_theme::redetect_on_focus(app, focused);
            Task::none()
        }
        Message::WorktreeDeleteConfirmed => shell::daemon_sync::on_worktree_delete_confirmed(app),
        Message::WorktreeIncludeRequested(path) => {
            shell::daemon_sync::on_worktree_include_requested(app, path)
        }
        Message::WorktreeExcludeRequested(dir) => {
            shell::daemon_sync::on_worktree_exclude_requested(app, dir)
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
    let Some(id) = app.core.active_session else {
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

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    // The catalog fixtures moved to `shell/daemon_sync.rs` with the reconcile tests that are
    // mostly what they were for; the stamper-seeding tests below still build a snapshot, so they
    // import them back rather than keep a second copy.
    use crate::shell::daemon_sync::tests::{snapshot_with, summary, summary_at};
    use micold_client::features::settings::{EnvironmentDraft, SettingsDraft, TerminalDraft};
    use micold_core::session::AiCli;
    // These tests drive whole messages through `update_inner`, which is this file's dispatcher, so
    // they stay here even though what they assert about is the daemon's: they are tests of the
    // routing reaching the right arm as much as of the arm itself.
    use micold_core::protocol::messages::{DaemonMsg, WireLifecycle};

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
            placement: micold_client::daemon::Placement::default(),
            sandbox: micold_client::features::sandbox::Sandbox::default(),
            sandbox_boot: None,
            version_mismatch: None,
            build_mismatch: None,
            next_req: 0,
            scrollback_inflight: HashMap::new(),
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
            placement: micold_client::daemon::Placement::default(),
            sandbox: micold_client::features::sandbox::Sandbox::default(),
            sandbox_boot: None,
            version_mismatch: None,
            build_mismatch: None,
            next_req: 0,
            scrollback_inflight: HashMap::new(),
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

    // --- BUG-002 (feature 025): the restored session is started, not only viewed ----------------
    //
    // Deciding which session to display and asking the daemon to run it are two halves of one act,
    // and only `view_and_start` performed the second. `restore_after_activation` is client state
    // only — it resolves the memory, reveals the row, takes the keyboard, and says nothing to the
    // daemon. So the launch made a session current that the daemon was not hosting, and
    // `SetViewedSession` had no stream to open. BUG-001 made that screen say so; these make it not
    // happen.
    //
    // The seam had no tests at all: the plan reasoned `boot()` was glue because the decision
    // (*which* session) lives in the tested reducer. What the client then sends is a second
    // decision, and this is where it is now pinned.

    /// Settings that change nothing and source no environment script — these tests are about which
    /// session messages go out, and env-include would reach for the filesystem on the way.
    fn quiet_settings() -> micold_core::protocol::messages::DaemonSettings {
        micold_core::protocol::messages::DaemonSettings {
            scrollback_lines: micold_core::settings::DEFAULT_SCROLLBACK_LINES,
            env_include_enabled: false,
            env_include_script_path: String::new(),
            env_include_timeout_secs: micold_core::settings::DEFAULT_ENV_INCLUDE_TIMEOUT_SECS,
            default_ai_cli: AiCli::ClaudeCode,
        }
    }

    /// Drive a daemon connection for `app` and return every `ClientMsg` it sent, in order.
    fn connect(
        app: &mut App,
        catalog: micold_core::protocol::messages::CatalogSnapshot,
    ) -> Vec<ClientMsg> {
        let (tx, mut rx) = iced::futures::channel::mpsc::unbounded();
        let _ = update_inner(
            app,
            Message::DaemonConnected {
                outbox: micold_client::daemon::Outbox::new(tx),
                catalog,
                settings: quiet_settings(),
            },
        );
        let mut sent = Vec::new();
        while let Ok(msg) = rx.try_recv() {
            sent.push(msg);
        }
        sent
    }

    /// The container's log is an *answer*, and an empty one is an answer too (FR-038).
    ///
    /// A user asks for diagnostics because something is wrong. Two outcomes are useful — "here is
    /// what it said" and "it is running and has said nothing" — and neither is silence, which is
    /// what the arm did before it existed.
    #[test]
    fn the_containers_log_is_reported_whether_or_not_it_has_anything_in_it() {
        let mut app = base_app();
        let _ = update_inner(
            &mut app,
            Message::SandboxDiagnostics(vec![
                "starting session service".into(),
                "bind /work/demo: permission denied".into(),
            ]),
        );
        let said = app.core.notify.visible().expect("a notice was raised");
        assert!(
            said.message.contains("permission denied"),
            "the most recent line is the one worth showing: {}",
            said.message
        );

        let mut empty = base_app();
        let _ = update_inner(&mut empty, Message::SandboxDiagnostics(Vec::new()));
        let said = empty
            .core
            .notify
            .visible()
            .expect("an empty log still gets an answer");
        assert!(
            said.message.contains("logged nothing"),
            "an empty log should say so rather than say nothing: {}",
            said.message
        );
    }

    /// Connecting must **start** the restored session, not only view it (FR-004a, contract §3.3a).
    ///
    /// `InterruptedResumable` is deliberate rather than incidental: it is what every durable session
    /// is after a restart, which makes this the ordinary launch and not an edge case.
    #[test]
    fn connecting_starts_the_restored_session_rather_than_only_viewing_it() {
        let project = PathBuf::from("/repo/demo");
        let id = SessionId::new();
        let mut app = base_app();
        app.core.workspace.active = Some(project.clone());
        // What `boot()`'s `restore_after_activation` leaves behind: the session is already current
        // when the connection arrives, chosen from the memory loaded off disk.
        app.core.active_session = Some(id);

        let sent = connect(
            &mut app,
            snapshot_with(
                "/repo/demo",
                vec![summary(
                    id,
                    "left off here",
                    WireLifecycle::InterruptedResumable,
                )],
            ),
        );

        let attach = sent
            .iter()
            .position(|m| matches!(m, ClientMsg::Attach { project: p, .. } if *p == project))
            .expect("the project is attached first");
        let start = sent
            .iter()
            .position(|m| matches!(m, ClientMsg::SessionStart { session } if *session == id))
            .expect("the restored session must be started, or no frame will ever arrive for it");
        let view = sent
            .iter()
            .position(
                |m| matches!(m, ClientMsg::SetViewedSession { session: Some(s), .. } if *s == id),
            )
            .expect("and it must still be the session the daemon streams");
        assert!(
            attach < start,
            "nothing about a session precedes the attach"
        );
        assert!(
            start < view,
            "the start precedes the view, the order `view_and_start` already establishes"
        );
    }

    /// Exactly one start, naming the restored session (SC-005a, contract §3.3b).
    ///
    /// This is the bound that replaces FR-004's prohibition. Resuming the one session the user is
    /// being shown is the feature; resuming every session the application happens to remember would
    /// be a launch that spawns a process per project, which is what "restoring starts nothing" was
    /// really protecting against.
    #[test]
    fn connecting_starts_only_the_session_it_restores() {
        let project = PathBuf::from("/repo/demo");
        let restored = SessionId::new();
        let sibling = SessionId::new();
        let elsewhere = SessionId::new();
        let mut app = base_app();
        app.core.workspace.active = Some(project.clone());
        app.core.active_session = Some(restored);

        let mut catalog = snapshot_with(
            "/repo/demo",
            vec![
                summary(restored, "restored", WireLifecycle::InterruptedResumable),
                summary(sibling, "same project", WireLifecycle::Idle),
            ],
        );
        let mut other = snapshot_with(
            "/repo/other",
            vec![summary(
                elsewhere,
                "another project entirely",
                WireLifecycle::InterruptedResumable,
            )],
        );
        catalog.projects.append(&mut other.projects);

        let sent = connect(&mut app, catalog);

        let started: Vec<SessionId> = sent
            .iter()
            .filter_map(|m| match m {
                ClientMsg::SessionStart { session } => Some(*session),
                _ => None,
            })
            .collect();
        assert_eq!(
            started,
            vec![restored],
            "one start, naming the restored session — not its neighbours, not another project's"
        );
    }

    /// With nothing remembered, a launch starts nothing at all (FR-007, SC-005a).
    ///
    /// The overview is a legitimate place to land, and landing there must stay free of side effects:
    /// FR-007 forbids choosing a session on the user's behalf, and starting one would be that choice
    /// made twice over.
    #[test]
    fn connecting_with_no_remembered_session_starts_nothing() {
        let project = PathBuf::from("/repo/demo");
        let unchosen = SessionId::new();
        let mut app = base_app();
        app.core.workspace.active = Some(project.clone());
        assert_eq!(
            app.core.active_session, None,
            "the memory resolved to nothing"
        );

        let sent = connect(
            &mut app,
            snapshot_with(
                "/repo/demo",
                vec![summary(unchosen, "not chosen", WireLifecycle::Idle)],
            ),
        );

        assert!(
            !sent
                .iter()
                .any(|m| matches!(m, ClientMsg::SessionStart { .. })),
            "landing on the project overview must not run anything (FR-007)"
        );
        assert!(
            sent.iter()
                .any(|m| matches!(m, ClientMsg::SetViewedSession { session: None, .. })),
            "the daemon is still told that no session is viewed"
        );
    }

    /// BUG-002 (024): a drained reveal must record where it sent the list, because the list will
    /// not always tell us.
    ///
    /// `sidebar_scroll_offset` is a mirror of the scrollable's position, and its only writer is
    /// `Message::SidebarScrolled` — which iced publishes from `notify_viewport`, and
    /// `notify_viewport` returns without publishing when the content fits the viewport
    /// (`iced_widget/src/scrollable.rs`). So a reveal that runs in a project whose sidebar fits —
    /// the short project you pass through on the way somewhere — moves the list and is never told,
    /// and the mirror keeps the *previous* project's offset.
    ///
    /// What that costs is the whole feature: on the next arrival the drain measures the row against
    /// an offset the list is nowhere near, decides it is already visible, and consumes the arm
    /// under FR-009. Reproduced on screen 2026-08-20 — the panel sat at the top with the marked row
    /// 1,968px below the fold, and the trace read `scroll_offset=734 -> no scroll, the row is
    /// already visible` immediately followed by `the scrollable reports offset 0`.
    #[test]
    fn a_drained_reveal_records_where_it_sent_the_list() {
        use micold_core::project::{Availability, Project};
        use micold_core::session::{AiCli, Session};
        use micold_core::worktree::{Worktree, WorktreeStatus};

        let mut app = base_app();
        let path = PathBuf::from("/repo");
        app.core.workspace.projects.push(Project {
            path: path.clone(),
            display_name: "repo".to_string(),
            is_git_repo: true,
            availability: Availability::Available,
        });
        app.core.workspace.active = Some(path.clone());
        app.core.worktrees = vec![Worktree {
            dir_name: "only".to_string(),
            path: PathBuf::from("/repo/.claude/worktrees/only"),
            branch: Some("feat/only".to_string()),
            status: WorktreeStatus::Valid,
            included: false,
        }];
        let session = Session::start_new(
            SessionLocation::Worktree("only".to_string()),
            AiCli::ClaudeCode,
        );
        let id = session.id;
        app.core.workspace.sessions.insert(path, vec![session]);
        app.core.active_session = Some(id);
        // Everything here fits: one location in a tall panel. This is the project that scrolls
        // without reporting.
        app.core.sidebar_viewport_height = 400;
        // Left behind by the project we just came from, whose list was long enough to scroll.
        app.core.sidebar_scroll_offset = 734;
        app.core.pending_reveal_scroll = true;

        assert!(
            reveal_scroll(&mut app).is_some(),
            "precondition: the reveal drains and asks for a scroll — 734 is not where this \
             one-location list can be"
        );

        assert_eq!(
            app.core.sidebar_scroll_offset, 0,
            "the reveal knows where it sent the list, so it must not wait to be told: leaving 734 \
             here is BUG-002, and the next arrival measures its row against a position the panel \
             left long ago"
        );
    }

    /// Builds an `App` with every field at a neutral default, so each test only spells out the
    /// fields it actually varies (mirrors the literal-construction pattern the other tests in this
    /// module already use, factored out because T100's tests need several variants of it).
    pub(crate) fn base_app() -> App {
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
            placement: micold_client::daemon::Placement::default(),
            sandbox: micold_client::features::sandbox::Sandbox::default(),
            sandbox_boot: None,
            version_mismatch: None,
            build_mismatch: None,
            next_req: 0,
            scrollback_inflight: HashMap::new(),
            pending_ops: HashMap::new(),
            probe: None,
            scene_ready: false,
            scene_frames: 0,
            ripples_animating: Arc::new(AtomicUsize::new(0)),
            scene_ripple_frames: std::cell::Cell::new(0),
        }
    }

    // --- FR-035a: an accepted fallback has to reach the connection, not only the banner --------

    /// An `App` configured for the sandbox, with the sandbox failed and a boot plan to restart.
    fn app_with_a_failed_sandbox() -> App {
        use micold_core::sandbox::lifecycle::{Failure, Stage};
        let mut app = base_app();
        app.placement.kind = micold_core::sandbox::placement::PlacementKind::LocalSandbox;
        app.sandbox = micold_client::features::sandbox::Sandbox {
            state: micold_core::sandbox::lifecycle::SandboxState::Failed(Failure {
                stage: Stage::Probing,
                error: micold_core::sandbox::runtime::RuntimeError::NotInstalled {
                    kind: micold_core::sandbox::runtime::RuntimeKind::Docker,
                },
            }),
            ..micold_client::features::sandbox::Sandbox::default()
        };
        app.sandbox_boot = Some(shell::sandbox::BootPlan {
            profile: micold_core::sandbox::SandboxProfile::default(),
            state_dir: PathBuf::from("/tmp/micold-test-state"),
            projects: Vec::new(),
        });
        app
    }

    #[test]
    fn accepting_the_fallback_moves_the_connection_to_a_host_process() {
        // The defect this was written for: `SandboxFallbackAccepted` used to record consent and
        // return `Task::none()`, and nothing else changed. But `daemon::connection` dials from
        // `app.placement`, and its `LocalSandbox` arm never falls back to a host process by design
        // (FR-035) — so the user pressed "Run without it for now", the banner said they were
        // running unsandboxed, and the client kept dialling a port nothing was listening on. The
        // offer worked as a statement and not as a service.
        let mut app = app_with_a_failed_sandbox();

        let _ = update_inner(&mut app, Message::SandboxFallbackAccepted);

        assert!(
            app.sandbox.fallback.is_some(),
            "the consent must be recorded, or the persistent notice has nothing to show"
        );
        assert_eq!(
            app.placement.kind,
            micold_core::sandbox::placement::PlacementKind::HostProcess,
            "accepting the fallback must move the connection to a host process — the subscription              is keyed on this value, and only changing it makes the client dial somewhere a daemon              can actually be"
        );
    }

    #[test]
    fn a_fallback_that_was_not_on_offer_leaves_the_connection_where_it_is() {
        // The other half: consent is not a thing the application may take on its own behalf
        // (FR-035). A running sandbox offers no fallback, so nothing here may move the placement —
        // otherwise a stray message would quietly unsandbox a working session.
        let mut app = base_app();
        app.placement.kind = micold_core::sandbox::placement::PlacementKind::LocalSandbox;
        app.sandbox = micold_client::features::sandbox::Sandbox {
            state: micold_core::sandbox::lifecycle::SandboxState::Running(
                micold_core::sandbox::runtime::ContainerId("x".into()),
            ),
            ..micold_client::features::sandbox::Sandbox::default()
        };

        let _ = update_inner(&mut app, Message::SandboxFallbackAccepted);

        assert!(app.sandbox.fallback.is_none());
        assert_eq!(
            app.placement.kind,
            micold_core::sandbox::placement::PlacementKind::LocalSandbox,
            "a fallback nobody offered must not move a running sandbox's connection"
        );
    }

    #[test]
    fn trying_the_sandbox_again_brings_the_connection_back_with_it() {
        // The return leg. Without it the restart succeeds, the container comes up, and every
        // session keeps running on the host process the fallback moved us to — a banner claiming
        // containment over an unconfined shell, which is precisely what FR-035b forbids.
        let mut app = app_with_a_failed_sandbox();
        let _ = update_inner(&mut app, Message::SandboxFallbackAccepted);
        assert_eq!(
            app.placement.kind,
            micold_core::sandbox::placement::PlacementKind::HostProcess,
            "precondition: the fallback moved us off the sandbox"
        );

        let _ = update_inner(&mut app, Message::SandboxRestartRequested);

        assert_eq!(
            app.placement.kind,
            micold_core::sandbox::placement::PlacementKind::LocalSandbox,
            "asking for the sandbox back must point the connection back at it"
        );
    }

    // --- T117 / FR-024a / SC-021: the read-only state must end when the daemon says we hold the
    // project (BUG-007) -----------------------------------------------------------------------

    /// A *different* window: same build as this one, its own instance. Every gate below that
    /// speaks of "another window" means this — a second process, which is what the daemon reports
    /// when the takeover is real. `010` BUG-022 is the case where it is not.
    fn other_window() -> micold_core::protocol::messages::ClientIdentity {
        micold_core::protocol::messages::ClientIdentity::new(
            "other-window",
            micold_core::protocol::messages::ClientInstance {
                pid: 4242,
                nonce: "a-second-process".into(),
            },
        )
    }

    /// This window, as the daemon would name it back to us.
    fn this_window() -> micold_core::protocol::messages::ClientIdentity {
        micold_core::protocol::messages::ClientIdentity::new(
            "micold-ai-ide/test",
            micold_core::protocol::messages::ClientInstance::current(),
        )
    }

    /// An `App` holding `project` as its active project, with nothing else varied.
    fn app_on_project(project: &Path) -> App {
        let mut app = base_app();
        app.core.workspace.active = Some(project.to_path_buf());
        app
    }

    fn feed(app: &mut App, msg: DaemonMsg) {
        let _ = update_inner(app, Message::DaemonEvent(msg));
    }

    // --- `010` BUG-021: what a scroll gesture costs -------------------------------------------

    /// A grid sitting at the live tail: `rows` visible lines at `viewport_top`, no history cached.
    fn at_the_tail(session: SessionId, viewport_top: i64, rows: u16) -> GridCache {
        use micold_core::protocol::grid::{
            GridFrame, WireCursor, WireCursorShape, WireLine, WireStyle,
        };
        let mut cache = GridCache::new();
        cache.apply(&GridFrame {
            session,
            seq: 1,
            generation: 1,
            full: true,
            viewport_top: LineId(viewport_top),
            oldest_available: LineId(0),
            cols: 80,
            rows,
            cursor: WireCursor {
                line: LineId(viewport_top),
                col: 0,
                shape: WireCursorShape::Block,
                visible: true,
                blinking: false,
            },
            styles: vec![WireStyle {
                fg: micold_core::protocol::grid::WireColor::Named(7),
                bg: micold_core::protocol::grid::WireColor::Named(0),
                flags: 0,
                underline_color: None,
            }],
            hyperlinks: Vec::new(),
            lines: (0..rows as i64)
                .map(|i| WireLine {
                    id: LineId(viewport_top + i),
                    text: "x".into(),
                    runs: vec![micold_core::protocol::grid::StyleRun { len: 1, style: 0 }],
                    extras: Vec::new(),
                    wrapped: false,
                })
                .collect(),
            mode: 0,
            input_serial: None,
        });
        cache
    }

    /// An `App` scrolled-ready: one session, its grid at the tail, and a socket to read.
    fn app_at_the_tail() -> (
        App,
        iced::futures::channel::mpsc::UnboundedReceiver<ClientMsg>,
    ) {
        let (tx, rx) = iced::futures::channel::mpsc::unbounded();
        let mut app = base_app();
        app.daemon = Some(micold_client::daemon::Outbox::new(tx));
        let id = SessionId::new();
        app.core.active_session = Some(id);
        app.grids.insert(id, at_the_tail(id, 4000, 69));
        (app, rx)
    }

    fn scrollback_ranges(
        rx: &mut iced::futures::channel::mpsc::UnboundedReceiver<ClientMsg>,
    ) -> Vec<Range<LineId>> {
        let mut out = Vec::new();
        while let Ok(msg) = rx.try_recv() {
            if let ClientMsg::ScrollbackRequest { ranges, .. } = msg {
                out.extend(ranges);
            }
        }
        out
    }

    /// `010` BUG-021, measured the way the report measured it: not one request, but a gesture.
    ///
    /// The unit gates in `grid.rs` pin the shape of a single range. This one drives 150 wheel
    /// notches of two lines each through the real dispatcher against a real `Outbox` and reads what
    /// went on the wire. Before the fix that was 150 requests — one per notch, because no answer
    /// had arrived to cache — each running from the revealed line to the live tail, summing to tens
    /// of thousands of lines for a gesture that revealed three hundred. The daemon served every one
    /// of them serially under the session's terminal lock, which is why a three-second scroll left
    /// it saturated for fifteen seconds afterwards and the pane blank throughout.
    ///
    /// Both numbers below are ceilings with slack in them, deliberately: what is load-bearing is
    /// that the cost of a gesture tracks the lines it *reveals*, not the notches it takes or the
    /// depth it reaches. An exact count would pin the prefetch size, which is a tuning decision.
    #[test]
    fn a_scroll_gesture_asks_once_per_viewport_not_once_per_notch() {
        let (mut app, mut rx) = app_at_the_tail();

        for _ in 0..150 {
            let _ = update_inner(&mut app, Message::TerminalScrolled(2));
        }

        let asked = scrollback_ranges(&mut rx);
        assert_eq!(app.display_offset, 300, "the gesture went 300 lines back");
        assert!(
            asked.len() <= 6,
            "150 notches over 300 lines sent {} requests; one per viewport of travel is ~5",
            asked.len()
        );
        let lines: i64 = asked.iter().map(|r| r.end.0 - r.start.0).sum();
        assert!(
            lines <= 4 * 300,
            "a gesture revealing 300 lines asked the daemon for {lines}"
        );
        assert!(
            asked.iter().all(|r| r.end.0 - r.start.0 <= 2 * 69),
            "every request is bounded by the viewport, however deep the gesture went: {asked:?}"
        );
    }

    /// The gap probe D found, and the reason it is worth writing down: an in-flight record is
    /// released by its *answer*, and removing that release broke nothing.
    ///
    /// Every other gate here stops at the request. None of them let an answer arrive and then
    /// scrolled again, so nothing said the record was ever cleared — and a record that is never
    /// cleared is a range this view will never ask for twice. That is harmless while the answer
    /// carries the lines (the cache then holds them, and `held` is true either way) and permanent
    /// when it does not: the daemon trimmed them, or the session went away, and those rows stay
    /// blank for as long as the window stays there. A storm traded for a hole.
    #[test]
    fn a_range_whose_answer_brought_nothing_is_asked_for_again() {
        let (mut app, mut rx) = app_at_the_tail();
        let session = app
            .core
            .active_session
            .expect("the fixture views a session");

        let _ = update_inner(&mut app, Message::TerminalScrolled(2));
        let asked = scrollback_ranges(&mut rx);
        assert_eq!(
            asked.len(),
            1,
            "one notch into un-cached history, one request"
        );
        let req = *app
            .scrollback_inflight
            .keys()
            .next()
            .expect("and it is on the record as in flight");

        feed(
            &mut app,
            DaemonMsg::ScrollbackResponse {
                session,
                req,
                oldest_available: LineId(0),
                newest: LineId(4068),
                lines: Vec::new(),
                styles: Vec::new(),
                hyperlinks: Vec::new(),
                more: false,
            },
        );

        let _ = update_inner(&mut app, Message::TerminalScrolled(0));
        assert_eq!(
            scrollback_ranges(&mut rx),
            asked,
            "the answer brought none of that range, so it is not on its way and must be asked \
             for again"
        );
    }

    /// The in-flight record is what stops the storm, so it must never outlive its answer.
    ///
    /// `req`s are per-connection: a request outstanding when the socket drops will never be
    /// answered on the next one. Keeping the record would suppress exactly the request that would
    /// have filled those rows, and the pane would stay blank for as long as the view stayed there —
    /// trading a storm for a permanent hole.
    #[test]
    fn a_disconnect_releases_the_ranges_that_will_never_be_answered() {
        let (mut app, _rx) = app_at_the_tail();
        let _ = update_inner(&mut app, Message::TerminalScrolled(2));
        assert!(
            !app.scrollback_inflight.is_empty(),
            "the gesture must have left a request outstanding for this to be about anything"
        );

        let _ = shell::daemon_sync::on_disconnected(&mut app);

        assert!(
            app.scrollback_inflight.is_empty(),
            "a range asked for on a dead connection is not on its way"
        );
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
                    holder: other_window(),
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

    /// `010` BUG-023: the refusal and the takeover are one state and must not be one sentence.
    ///
    /// Both leave the window read-only with the same take-over button, which is why the refusal was
    /// folded onto `displaced` in the first place. What the fold also did was tell a window that had
    /// merely been turned away that another window "took over this project" — the opposite event,
    /// described in the past tense, about something that never happened to it. The distinction has
    /// to survive as far as the banner, so the banner is chosen from the status and the status is
    /// what this pins.
    #[test]
    fn a_refused_attach_reaches_the_banner_as_a_refusal_not_a_takeover() {
        use micold_client::features::connection::ConnectionStatus;

        let project = PathBuf::from("/repo/demo");

        let mut refused = app_on_project(&project);
        feed(
            &mut refused,
            DaemonMsg::Refused {
                reason: micold_core::protocol::messages::RefusalReason::ProjectBusy {
                    project: project.clone(),
                    holder: other_window(),
                    since_secs: 12,
                },
            },
        );

        let mut taken = app_on_project(&project);
        feed(
            &mut taken,
            DaemonMsg::Displaced {
                project: project.clone(),
                by: other_window(),
            },
        );

        assert_eq!(
            connection_status(&refused),
            ConnectionStatus::ProjectBusy {
                holder: other_window().to_string()
            },
            "nobody took anything from this window — it asked and was told no"
        );
        assert_eq!(
            connection_status(&taken),
            ConnectionStatus::Displaced {
                by: other_window().to_string()
            },
            "and the window that really did lose the project keeps the sentence that fits it"
        );
        assert!(
            active_project_displaced(&refused) && active_project_displaced(&taken),
            "the difference is in what the user is told, not in what they may do: both are \
             read-only and both are resolved by the same take-over"
        );
    }

    // --- `010` BUG-022: a window must not displace itself ---------------------------------------

    /// The reported defect, end to end: one window, one project, no second window anywhere, and
    /// the window goes read-only above a banner naming its own build.
    ///
    /// The sequence is a reconnect. The keepalive declares the link dead, the outer loop dials
    /// again, the new connection attaches — and the daemon, which has not yet noticed the old
    /// socket is gone, does exactly what FR-024 says: it displaces the holder and tells it so.
    /// Both connections feed one `App`, so that `Displaced` lands after the new connection's
    /// `Attached`, which is the frame that clears the flag. The stale frame wins, and re-latches
    /// read-only on a window that just successfully attached.
    ///
    /// The pair is the point. A window that really did lose the project must still be told; what
    /// must not happen is a window being told it lost the project to itself.
    #[test]
    fn a_window_is_not_displaced_by_its_own_reconnect_but_is_by_another_window() {
        use micold_client::features::connection::ConnectionStatus;

        let project = PathBuf::from("/repo/demo");

        let mut myself = app_on_project(&project);
        feed(
            &mut myself,
            DaemonMsg::Displaced {
                project: project.clone(),
                by: this_window(),
            },
        );

        let mut other = app_on_project(&project);
        feed(
            &mut other,
            DaemonMsg::Displaced {
                project: project.clone(),
                by: other_window(),
            },
        );

        assert_eq!(
            connection_status(&myself),
            ConnectionStatus::Connected,
            "the only window the user has must keep its project when its own reconnect \
             supersedes its own dead connection (BUG-022)"
        );
        assert!(
            !active_project_displaced(&myself),
            "and it must keep typing into it — input suppression is the harm, the banner is only \
             how the user finds out about it"
        );
        assert_eq!(
            connection_status(&other),
            ConnectionStatus::Displaced {
                by: other_window().to_string()
            },
            "two genuinely different windows of one build must still displace each other, which \
             is why the identity is compared and not the build string"
        );
    }

    /// The same collision arriving as a refusal rather than a displacement — and the reason the
    /// fix is an identity and not a connection generation.
    ///
    /// When the reconnect wins the race the other way round, the daemon still has the old
    /// attachment and refuses the new connection's attach as `ProjectBusy`, naming the holder.
    /// That frame is *timely*: it arrives on the current connection, in answer to a request this
    /// window just made, so no staleness rule can discard it. Only the holder's identity says what
    /// it is — this window's own corpse — and the window can then reclaim the project instead of
    /// asking the user to take it over from themselves.
    #[test]
    fn a_refusal_by_this_windows_own_dead_connection_is_reclaimed_not_offered() {
        use micold_client::features::connection::ConnectionStatus;

        let project = PathBuf::from("/repo/demo");
        let (tx, mut rx) = iced::futures::channel::mpsc::unbounded();
        let mut app = app_on_project(&project);
        app.daemon = Some(micold_client::daemon::Outbox::new(tx));

        feed(
            &mut app,
            DaemonMsg::Refused {
                reason: micold_core::protocol::messages::RefusalReason::ProjectBusy {
                    project: project.clone(),
                    holder: this_window(),
                    since_secs: 3,
                },
            },
        );

        match rx.try_recv() {
            Ok(ClientMsg::Attach { project: p, force }) => {
                assert_eq!(p, project);
                assert!(
                    force,
                    "a non-forced retry would be refused by the same dead connection forever"
                );
            }
            other => panic!("expected a forced re-attach, got {other:?}"),
        }
        assert_eq!(
            connection_status(&app),
            ConnectionStatus::Connected,
            "and the user is never shown a take-over offer against themselves"
        );
    }

    /// The other half of that pair: a refusal naming a *different* window is still the user's
    /// decision, and forcing it silently would be a takeover nobody confirmed (FR-023).
    #[test]
    fn a_refusal_by_another_window_is_still_offered_never_forced() {
        use micold_client::features::connection::ConnectionStatus;

        let project = PathBuf::from("/repo/demo");
        let (tx, mut rx) = iced::futures::channel::mpsc::unbounded();
        let mut app = app_on_project(&project);
        app.daemon = Some(micold_client::daemon::Outbox::new(tx));

        feed(
            &mut app,
            DaemonMsg::Refused {
                reason: micold_core::protocol::messages::RefusalReason::ProjectBusy {
                    project: project.clone(),
                    holder: other_window(),
                    since_secs: 3,
                },
            },
        );

        assert!(
            rx.try_recv().is_err(),
            "nothing may be sent: taking a project from another window is a confirmed action"
        );
        assert_eq!(
            connection_status(&app),
            ConnectionStatus::ProjectBusy {
                holder: other_window().to_string()
            },
            "the window is told, and offered the take-over it must ask for"
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
                by: other_window(),
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
                    by: other_window(),
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
                by: other_window(),
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
            terminal: TerminalDraft {
                scrollback_lines: "20000".into(),
            },
            environment: EnvironmentDraft {
                enabled: false,
                script_path: "/tmp/does-not-exist.sh".into(),
                timeout_secs: "15".into(),
                default_ai_cli: AiCli::Copilot,
            },
            ..SettingsDraft::default()
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
            terminal: TerminalDraft {
                scrollback_lines: "20000".into(),
            },
            environment: EnvironmentDraft {
                enabled: true,
                script_path: String::new(),
                timeout_secs: "15".into(),
                default_ai_cli: AiCli::ClaudeCode,
            },
            ..SettingsDraft::default()
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
                    default_ai_cli: AiCli::ClaudeCode,
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
                    default_ai_cli: AiCli::ClaudeCode,
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
            placement: micold_client::daemon::Placement::default(),
            sandbox: micold_client::features::sandbox::Sandbox::default(),
            sandbox_boot: None,
            version_mismatch: None,
            build_mismatch: None,
            next_req: 0,
            scrollback_inflight: HashMap::new(),
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
        app.displaced.insert(
            project.clone(),
            micold_client::features::connection::Hold::taken_over(other_window().to_string()),
        );
        assert_eq!(
            connection_status(&app),
            ConnectionStatus::Displaced {
                by: other_window().to_string()
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
