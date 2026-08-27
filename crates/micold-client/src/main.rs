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
    /// Projects this window has been displaced from by another window's takeover (US5, FR-024),
    /// keyed by project path → the taking-over window's identity. A displaced project is read-only
    /// here (input suppressed, a banner shown) until the user takes it back or reconnects. Cleared
    /// on a fresh connect. Empty in the common single-window case.
    displaced: HashMap<PathBuf, String>,
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
        Message::Project(ProjectMsg::RenameConfirmed) => {
            shell::daemon_sync::on_rename_confirmed(app)
        }
        Message::Project(ProjectMsg::ForgetConfirmed) => {
            shell::daemon_sync::on_project_forget_confirmed(app)
        }
        Message::Worktree(WorktreeMsg::RenameConfirmed) => {
            shell::daemon_sync::on_worktree_rename_confirmed(app)
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
        Message::Session(SessionMsg::StartMenuOpened(location)) => {
            app.core.session.available_providers = app.caps.available_providers();
            app.core
                .update(Message::Session(SessionMsg::StartMenuOpened(location)));
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
    let Some(id) = app.core.session.active else {
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

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    // The catalog fixtures moved to `shell/daemon_sync.rs` with the reconcile tests that are
    // mostly what they were for; the stamper-seeding tests below still build a snapshot, so they
    // import them back rather than keep a second copy.
    use crate::shell::daemon_sync::tests::{snapshot_with, summary, summary_at};
    use micold_client::features::connection::Msg as ConnectionMsg;
    use micold_client::features::sandbox::Msg as SandboxMsg;
    use micold_client::features::settings::Msg as SettingsMsg;
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
            Message::Session(SessionMsg::TerminalResized {
                cols: 220,
                rows: 60,
            }),
        );
        assert_eq!(app.last_grid, Some((220, 60)));

        let _ = update_inner(
            &mut app,
            Message::Session(SessionMsg::TerminalResized {
                cols: 180,
                rows: 45,
            }),
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

        let _ = update_inner(&mut app, Message::Session(SessionMsg::Selected(id)));

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

        let _ = update_inner(&mut app, Message::Session(SessionMsg::Selected(id)));

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
            Message::Connection(ConnectionMsg::Connected {
                outbox: micold_client::daemon::Outbox::new(tx),
                catalog,
                settings: quiet_settings(),
            }),
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
            Message::Sandbox(SandboxMsg::Diagnostics(vec![
                "starting session service".into(),
                "bind /work/demo: permission denied".into(),
            ])),
        );
        let said = app
            .core
            .notifications
            .queue
            .visible()
            .expect("a notice was raised");
        assert!(
            said.message.contains("permission denied"),
            "the most recent line is the one worth showing: {}",
            said.message
        );

        let mut empty = base_app();
        let _ = update_inner(
            &mut empty,
            Message::Sandbox(SandboxMsg::Diagnostics(Vec::new())),
        );
        let said = empty
            .core
            .notifications
            .queue
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
        app.core.session.active = Some(id);

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
        app.core.session.active = Some(restored);

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
            app.core.session.active, None,
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
    /// `scroll_offset` is a mirror of the scrollable's position, and its only writer is
    /// `Message::Sidebar(SidebarMsg::Scrolled)` — which iced publishes from `notify_viewport`, and
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
        app.core.worktree.worktrees = vec![Worktree {
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
        app.core.session.active = Some(id);
        // Everything here fits: one location in a tall panel. This is the project that scrolls
        // without reporting.
        app.core.sidebar.viewport_height = 400;
        // Left behind by the project we just came from, whose list was long enough to scroll.
        app.core.sidebar.scroll_offset = 734;
        app.core.sidebar.pending_reveal_scroll = true;

        assert!(
            reveal_scroll(&mut app).is_some(),
            "precondition: the reveal drains and asks for a scroll — 734 is not where this \
             one-location list can be"
        );

        assert_eq!(
            app.core.sidebar.scroll_offset, 0,
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

        let _ = update_inner(&mut app, Message::Sandbox(SandboxMsg::FallbackAccepted));

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

        let _ = update_inner(&mut app, Message::Sandbox(SandboxMsg::FallbackAccepted));

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
        let _ = update_inner(&mut app, Message::Sandbox(SandboxMsg::FallbackAccepted));
        assert_eq!(
            app.placement.kind,
            micold_core::sandbox::placement::PlacementKind::HostProcess,
            "precondition: the fallback moved us off the sandbox"
        );

        let _ = update_inner(&mut app, Message::Sandbox(SandboxMsg::RestartRequested));

        assert_eq!(
            app.placement.kind,
            micold_core::sandbox::placement::PlacementKind::LocalSandbox,
            "asking for the sandbox back must point the connection back at it"
        );
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
        let _ = update_inner(app, Message::Connection(ConnectionMsg::Event(msg)));
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
        app.core.settings.settings_draft = Some(SettingsDraft {
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

        let _ = update_inner(&mut app, Message::Settings(SettingsMsg::Saved));

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
    /// path (the direct `settings.json` write above the daemon-send in
    /// `Message::Settings(SettingsMsg::Saved)`), so
    /// there is nothing to notify the user about — unlike every other `send_op`-routed mutation, which
    /// has no such standalone path.
    #[test]
    fn settings_saved_is_a_silent_no_op_toward_the_daemon_when_disconnected() {
        let mut app = base_app();
        assert!(app.daemon.is_none());
        app.core.settings.settings_draft = Some(SettingsDraft {
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

        let _ = update_inner(&mut app, Message::Settings(SettingsMsg::Saved));

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
            Message::Connection(ConnectionMsg::Connected {
                outbox: micold_client::daemon::Outbox::new(tx),
                catalog: snapshot_with("/repo/demo", Vec::new()),
                settings: micold_core::protocol::messages::DaemonSettings {
                    default_ai_cli: AiCli::ClaudeCode,
                    scrollback_lines: 12_345,
                    env_include_enabled: false,
                    env_include_script_path: "/authoritative/from-daemon.sh".into(),
                    env_include_timeout_secs: 30,
                },
            }),
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
            Message::Connection(ConnectionMsg::Event(DaemonMsg::SettingsChanged {
                settings: micold_core::protocol::messages::DaemonSettings {
                    default_ai_cli: AiCli::ClaudeCode,
                    scrollback_lines: 5_000,
                    env_include_enabled: false,
                    env_include_script_path: "/tmp/after.sh".into(),
                    env_include_timeout_secs: 45,
                },
            })),
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
