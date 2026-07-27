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
use micold_client::app::{
    BranchSource, ClosingOverlay, Message, Overlay, ResolutionState, SelectKind, State,
    WorktreeForm, WorktreeFormStatus,
};
use micold_client::grid::GridCache;
use micold_client::input::SessionInputStamper;
use micold_client::motion::Animator;
use micold_client::selection::{Anchor, SelectGranularity, Selection};
use micold_client::ui::MotionKey;
use micold_core::env_include::{self, EnvIncludeOutcome};
use micold_core::fs_scan::{FolderScanner, StdFolderScanner};
use micold_core::git::{Git, GitCli};
use micold_core::protocol::grid::LineId;
use micold_core::protocol::messages::{
    CatalogSnapshot, ClientMsg, DaemonMsg, OperationResult, SessionProcess, WireLifecycle,
};
use micold_core::provider::{AiCliProvider, ClaudeProvider};
use micold_core::selector::{Selector, SelectorStatus};
use micold_core::session::{
    Session, SessionId, SessionLabel, SessionLifecycle, SessionLocation, ShellInstanceId,
    TerminalMode,
};
use micold_core::settings::{JsonFileSettingsStore, Settings, SettingsStore};
use micold_core::store::{JsonFileStore, ProjectStore};
use micold_core::theme::{observe_system_scheme, SystemScheme};
use micold_core::worktree::Worktree;
use micold_core::worktree::{BranchOrigin, CreateMode};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::time::Duration;

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

/// The animation clock tick interval (~60fps).
const ANIM_TICK: Duration = Duration::from_millis(16);

// Animation timing as legible durations (not opaque per-frame steps); the per-tick step the
// `Animator` consumes is derived via `step()` below (FR-013).
/// Overlay fade-in — Material Design 3 "medium" duration; clearly perceptible (the prior ~90ms
/// was imperceptible).
const OVERLAY_ENTER: Duration = Duration::from_millis(250);
/// Overlay fade-out — ~0.8× the enter (Material convention: exits are quicker).
const OVERLAY_EXIT: Duration = Duration::from_millis(200);
/// Overflow-menu fade — preserves the prior feel (was step 0.18 ≈ 90ms).
const MENU_FADE: Duration = Duration::from_millis(90);
/// Main-view fade — preserves the prior feel (was step 0.18 ≈ 90ms).
const MAIN_FADE: Duration = Duration::from_millis(90);
/// Sidebar slide — preserves the prior feel (was step 0.14 ≈ 114ms).
const SIDEBAR_SLIDE: Duration = Duration::from_millis(114);
/// Resize-handle hover highlight — preserves the prior gentle ~0.8s ramp (was step 0.02).
const HANDLE_HOVER: Duration = Duration::from_millis(800);
/// Hover-revealed row actions fade (feature 008) — quick, so the icons feel responsive.
const ROW_ACTIONS_FADE: Duration = Duration::from_millis(120);

/// Convert an animation `duration` into the per-tick progress step the [`Animator`] advances by
/// on each [`ANIM_TICK`], clamped to `(0, 1]`.
fn step(duration: Duration) -> f32 {
    (ANIM_TICK.as_secs_f32() / duration.as_secs_f32()).clamp(f32::EPSILON, 1.0)
}

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
        }
    }
}

struct App {
    core: State,
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
    /// The single shared animation driver for all UI motion (menu / sidebar / main / handle /
    /// overlay). Replaces the former per-animation `*_anim` fields (FR-007).
    motion: Animator<MotionKey>,
    /// Identity of the current main content, to detect changes that trigger a fade.
    main_key: String,
    /// Whether the pointer is over the sidebar resize handle (drives its hover highlight).
    handle_hovered: bool,
    /// The overlay currently fading out (rendered from this snapshot until its fade completes),
    /// or `None` when no overlay is leaving.
    dismissing: Option<ClosingOverlay>,
    /// Per-worktree hover-reveal fade tracks (feature 008), keyed by a hash of `dir_name` so each
    /// row's action icons fade in/out independently (hovering one while another fades out).
    row_fx: Animator<u64>,
    /// The worktree hovered on the previous update, to detect hover-enter/leave transitions and
    /// start the corresponding fade.
    prev_hovered: Option<String>,
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
    /// Correlation-id counter for the client's mutating RPCs (FR-009).
    next_req: u64,
    /// In-flight mutating RPCs keyed by `req` (T055). Lets a reply be matched, a duplicate
    /// submission suppressed, and an in-flight op resolved as *unknown* if the connection drops.
    pending_ops: HashMap<u64, PendingOp>,
}

/// The result of a single resolution attempt for one directory (feature 011, data-model.md).
/// `vars` is empty for every non-`Success` outcome.
struct EnvIncludeSnapshot {
    /// Resolved variables. Vestigial on the client now that the daemon resolves env at spawn time
    /// (T053); kept so the Settings resolution path is unchanged. TODO: move env-include to the daemon.
    #[allow(dead_code)]
    vars: Vec<(String, String)>,
    outcome: EnvIncludeOutcome,
}

/// Resolve the environment-include snapshot for `cwd` from the given settings values,
/// short-circuiting to `Disabled` (no subprocess spawned) when the feature is off or the path is
/// blank — mirrors the spec's Edge Cases and contracts/env-include-resolution.md's Non-goals (the
/// engine itself never decides whether to run). Shared by every resolution call site so they all
/// apply the exact same short-circuit + resolution logic.
fn resolve_env_include(
    enabled: bool,
    script_path: &str,
    timeout_secs: u64,
    cwd: &Path,
) -> EnvIncludeSnapshot {
    if !enabled || script_path.trim().is_empty() {
        return EnvIncludeSnapshot {
            vars: Vec::new(),
            outcome: EnvIncludeOutcome::Disabled,
        };
    }
    let (vars, outcome) = env_include::resolve(
        Path::new(script_path),
        cwd,
        Duration::from_secs(timeout_secs),
    );
    EnvIncludeSnapshot { vars, outcome }
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
        app.env_include_enabled,
        &app.env_include_script_path,
        app.env_include_timeout_secs,
        cwd,
    );
    app.env_include_last_outcome = snapshot.outcome.clone();
    app.env_include_cache.insert(cwd.to_path_buf(), snapshot);
}

/// Identity of the main content area, used to trigger a fade when it changes.
fn main_content_key(core: &State) -> String {
    if core.workspace.active_project().is_none() {
        "none".to_string()
    } else if let Some(id) = core.active_session {
        format!("s:{id}")
    } else {
        "project".to_string()
    }
}

/// The desired `(target, duration)` for each state-driven motion key, derived from the current
/// state. Used both to advance the animator (in [`apply_motion_targets`]) and to decide whether
/// the animation clock should run (in [`motion_animating`]), so the two never disagree.
///
/// `MotionKey::Overlay` is intentionally absent: it is driven by the overlay open/close
/// lifecycle in `update`, not by steady-state.
fn motion_targets(app: &App) -> [(MotionKey, f32, Duration); 5] {
    [
        (
            MotionKey::Menu,
            if app.core.help_menu_open { 1.0 } else { 0.0 },
            MENU_FADE,
        ),
        (
            MotionKey::Sidebar,
            if app.core.sidebar_hidden { 0.0 } else { 1.0 },
            SIDEBAR_SLIDE,
        ),
        (MotionKey::Main, 1.0, MAIN_FADE),
        (
            MotionKey::HandleHover,
            if app.handle_hovered { 1.0 } else { 0.0 },
            HANDLE_HOVER,
        ),
        (
            MotionKey::SidebarFilter,
            if app.core.sidebar_filter_open {
                1.0
            } else {
                0.0
            },
            MENU_FADE,
        ),
    ]
}

/// Push the current state-derived targets into the animator before ticking.
fn apply_motion_targets(app: &mut App) {
    for (key, target, duration) in motion_targets(app) {
        app.motion.to(key, target, step(duration));
    }
}

/// The overlay fade's target: fully shown while an overlay is open, fully hidden otherwise
/// (including while a just-closed overlay is fading out — the snapshot in `App::dismissing`
/// only supplies render data, it does not change the target).
fn overlay_motion_target(app: &App) -> f32 {
    if app.core.overlay == Overlay::None {
        0.0
    } else {
        1.0
    }
}

/// Whether any animation still needs to advance — gates the animation clock (FR-014). Compares
/// each key's current value against its freshly derived target so a just-reset track (e.g. the
/// main-view fade reset to 0 on a content change) is detected even before the animator's stored
/// target is refreshed. `MotionKey::Overlay` is handled by [`overlay_motion_target`].
fn motion_animating(app: &App) -> bool {
    let steady = motion_targets(app)
        .iter()
        .any(|(key, target, _)| (app.motion.get(*key) - target).abs() > f32::EPSILON);
    let overlay =
        (app.motion.get(MotionKey::Overlay) - overlay_motion_target(app)).abs() > f32::EPSILON;
    steady || overlay || app.row_fx.animating()
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

/// The app window icon as raw 64x64 RGBA (generated from `assets/icon/icon.svg` by
/// `assets/icon/generate.py`). Embedded directly so no runtime image decoder is needed.
const ICON_RGBA: &[u8] = include_bytes!("../../../assets/icon/icon-64.rgba");

/// Window settings carrying the app icon (taskbar / titlebar). On Linux the window app-id /
/// WM_CLASS is set to match `StartupWMClass` in the `.desktop` entry so the running window groups
/// under the launcher icon.
fn window_settings() -> iced::window::Settings {
    let icon = iced::window::icon::from_rgba(ICON_RGBA.to_vec(), 64, 64).ok();
    #[allow(unused_mut)]
    let mut settings = iced::window::Settings {
        icon,
        ..Default::default()
    };
    #[cfg(target_os = "linux")]
    {
        settings.platform_specific.application_id = "micold-ai-ide".to_string();
    }
    settings
}

pub fn main() -> iced::Result {
    iced::application("Micold AI IDE", update, view)
        .theme(theme)
        .default_font(iced::Font::DEFAULT)
        .font(micold_client::ui::MATERIAL_SYMBOLS_BYTES)
        .window(window_settings())
        .subscription(subscription)
        .run_with(boot)
}

fn boot() -> (App, Task<Message>) {
    let mut core = State::default();
    if let Some(store) = JsonFileStore::default_location() {
        core.workspace = store.load().workspace;
        core.workspace
            .refresh_availability(&StdFolderScanner::new());
        // Drop any leftover empty sessions so a restart never resumes a nonexistent
        // conversation (bug fix; see spec Clarifications 2026-07-16).
        prune_empty_sessions(&mut core.workspace);
    }
    let mut scrollback_lines = micold_core::settings::DEFAULT_SCROLLBACK_LINES;
    let mut env_include_enabled = micold_core::settings::DEFAULT_ENV_INCLUDE_ENABLED;
    let mut env_include_script_path = Settings::default().env_include_script_path;
    let mut env_include_timeout_secs = micold_core::settings::DEFAULT_ENV_INCLUDE_TIMEOUT_SECS;
    if let Some(store) = JsonFileSettingsStore::default_location() {
        let loaded = store.load().settings;
        core.theme_pref = loaded.theme;
        scrollback_lines = loaded.scrollback_lines;
        env_include_enabled = loaded.env_include_enabled;
        env_include_script_path = loaded.env_include_script_path;
        env_include_timeout_secs = loaded.env_include_timeout_secs;
    }
    let boot_cwd = default_resolution_cwd(&core);
    let boot_snapshot = resolve_env_include(
        env_include_enabled,
        &env_include_script_path,
        env_include_timeout_secs,
        &boot_cwd,
    );
    let env_include_last_outcome = boot_snapshot.outcome.clone();
    let mut env_include_cache = HashMap::new();
    env_include_cache.insert(boot_cwd, boot_snapshot);
    core.system_scheme = observe_system_scheme(detect_system_scheme(), core.system_scheme);
    // If a project is already active from a previous run, discover its worktrees for the initial
    // render. Session recovery from transcripts is now the daemon's responsibility (it owns
    // sessions); the client adopts them from the welcome catalog on connect (T055).
    if let Some(repo) = core.workspace.active.clone() {
        core.set_worktrees(discover_worktrees(&repo));
    }
    let mut motion = Animator::new();
    motion.set(MotionKey::Menu, 0.0);
    motion.set(
        MotionKey::Sidebar,
        if core.sidebar_hidden { 0.0 } else { 1.0 },
    );
    motion.set(MotionKey::Main, 1.0);
    motion.set(MotionKey::HandleHover, 0.0);
    motion.set(MotionKey::Overlay, 0.0);
    motion.set(MotionKey::SidebarFilter, 0.0);
    let main_key = main_content_key(&core);
    (
        App {
            core,
            grids: HashMap::new(),
            stamper: SessionInputStamper::new(),
            selection: None,
            display_offset: 0,
            scrollback_lines,
            motion,
            main_key,
            handle_hovered: false,
            dismissing: None,
            row_fx: Animator::new(),
            prev_hovered: None,
            window_focused: true,
            last_grid: None,
            env_include_enabled,
            env_include_script_path,
            env_include_timeout_secs,
            env_include_cache,
            env_include_last_outcome,
            daemon: None,
            daemon_catalog: None,
            displaced: HashMap::new(),
            disconnected: false,
            version_mismatch: None,
            next_req: 0,
            pending_ops: HashMap::new(),
        },
        // Ask for the initial window size up front: `resize_events` only fires on *changes*, so
        // without this the first context menu before any resize would have nothing to clamp
        // against (feature 015).
        iced::window::get_latest()
            .and_then(iced::window::get_size)
            .map(|size| Message::WindowResized {
                width: size.width.max(0.0) as u16,
                height: size.height.max(0.0) as u16,
            }),
    )
}

/// Persist the catalog. Empty sessions — those `claude` never recorded a conversation for —
/// are NOT preserved, so a restart never tries to resume a nonexistent session (bug fix; see
/// spec Clarifications 2026-07-16). A save failure is non-fatal (Principle IV) but is surfaced
/// to the user rather than silently discarded (FR-012b, bugfix 002/BUG-001) — the mutation that
/// triggered this persist stays in memory regardless; only the next restart is at risk.
fn persist(core: &mut State) {
    if let Some(store) = JsonFileStore::default_location() {
        let mut to_save = core.workspace.clone();
        prune_empty_sessions(&mut to_save);
        if let Err(err) = store.save(&to_save) {
            core.notify_error(format!("Couldn't save your changes: {err}"));
        }
    }
}

/// Remove sessions that have no `claude` conversation on disk (empty sessions).
fn prune_empty_sessions(workspace: &mut micold_core::workspace::Workspace) {
    for (project_path, sessions) in workspace.sessions.iter_mut() {
        sessions.retain(|s| session_has_conversation(project_path, s));
    }
    workspace
        .sessions
        .retain(|_, sessions| !sessions.is_empty());
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

/// Whether the AI CLI provider has recorded a conversation transcript for this session
/// (research R6, FR-020a). Routed through the provider seam (FR-024, bugfix BUG-002).
/// Cwd site 1/5 (research.md R2).
fn session_has_conversation(project_path: &Path, session: &micold_core::session::Session) -> bool {
    let provider = ClaudeProvider;
    let cwd = session_cwd_for_location(project_path, &session.location);
    let Some(config) = provider.config_dir() else {
        // Cannot determine the provider config dir — do not drop the session on uncertainty.
        return true;
    };
    provider.has_recorded_conversation(&config, &cwd, session.id.0)
}

fn persist_settings(core: &mut State) {
    if let Some(store) = JsonFileSettingsStore::default_location() {
        // Preserve the persisted scrollback limit (feature 006) and environment-include settings
        // (feature 011) when saving a theme change — this function only ever changes `theme`.
        let existing = store.load().settings;
        if let Err(err) = store.save(&Settings {
            theme: core.theme_pref,
            ..existing
        }) {
            core.notify_error(format!("Couldn't save your settings: {err}"));
        }
    }
}

fn update(app: &mut App, message: Message) -> Task<Message> {
    // Capture the open overlay + its draft BEFORE the reducer runs: closing an overlay clears
    // its draft synchronously in the core, so this snapshot is the only way to keep rendering it
    // while it fades out (FR-002/FR-006/FR-012).
    let overlay_before = app.core.overlay;
    let snapshot_before = capture_overlay(app);

    let task = update_inner(app, message);

    // Drive the overlay fade on open/close transitions (US1).
    let overlay_after = app.core.overlay;
    if overlay_before != overlay_after {
        if overlay_after == Overlay::None {
            // Closed (Cancel / Esc / successful submit): fade the snapshot out.
            app.dismissing = snapshot_before;
            app.motion.to(MotionKey::Overlay, 0.0, step(OVERLAY_EXIT));
        } else {
            // Opened (or switched to a different overlay): fade the new one in from hidden.
            app.dismissing = None;
            app.motion.set(MotionKey::Overlay, 0.0);
            app.motion.to(MotionKey::Overlay, 1.0, step(OVERLAY_ENTER));
        }
    }

    // Trigger a main-view fade-in whenever the displayed content changes.
    let key = main_content_key(&app.core);
    if key != app.main_key {
        app.main_key = key;
        app.motion.set(MotionKey::Main, 0.0);
    }

    // On a hover-enter/leave transition, animate the row's action icons: fade the newly hovered
    // row in (from hidden) and the previously hovered row out (feature 008).
    if app.core.hovered_worktree != app.prev_hovered {
        let s = step(ROW_ACTIONS_FADE);
        if let Some(old) = &app.prev_hovered {
            app.row_fx
                .to(micold_client::ui::worktree_fx_key(old), 0.0, s);
        }
        if let Some(new) = &app.core.hovered_worktree {
            let key = micold_client::ui::worktree_fx_key(new);
            // Start from hidden so it animates in (unless it's mid-fade-out and re-hovered).
            if app.row_fx.get(key) <= f32::EPSILON {
                app.row_fx.set(key, 0.0);
            }
            app.row_fx.to(key, 1.0, s);
        }
        app.prev_hovered = app.core.hovered_worktree.clone();
    }

    task
}

/// Snapshot the currently open overlay (and its draft) so it can be rendered while it fades out.
/// Returns `None` when no overlay is open or its draft is unexpectedly absent.
fn capture_overlay(app: &App) -> Option<ClosingOverlay> {
    match app.core.overlay {
        Overlay::None => None,
        Overlay::About => Some(ClosingOverlay::About),
        Overlay::ProjectSelector => app.core.selector.clone().map(ClosingOverlay::Selector),
        Overlay::RenameProject => app.core.rename_draft.clone().map(ClosingOverlay::Rename),
        Overlay::AddWorktree => app
            .core
            .worktree_form
            .clone()
            .map(|form| ClosingOverlay::Worktree(form, app.core.worktree_error.clone())),
        Overlay::Settings => app
            .core
            .settings_draft
            .clone()
            .map(ClosingOverlay::Settings),
        Overlay::ConfirmWorktreeDelete => app
            .core
            .worktree_delete_target
            .clone()
            .map(ClosingOverlay::ConfirmDelete),
        Overlay::RenameWorktree => app
            .core
            .worktree_rename_draft
            .clone()
            .map(ClosingOverlay::WorktreeRename),
        Overlay::ConfirmSessionRemove => app
            .core
            .session_remove_target
            .and_then(|id| app.core.workspace.find_session(id))
            .map(|(_, session)| {
                ClosingOverlay::ConfirmSessionRemove(session.label.display().to_string())
            }),
        Overlay::ConfirmForgetProject => app.core.forget_target.clone().map(|path| {
            let display_name = app
                .core
                .workspace
                .projects
                .iter()
                .find(|p| p.path == path)
                .map(|p| p.display_name.clone())
                .unwrap_or_else(|| micold_core::project::default_display_name(&path));
            let running = app.core.workspace.running_session_count(&path);
            ClosingOverlay::ConfirmForget(display_name, running)
        }),
    }
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
            // The daemon is the single writer of settings + sessions; adopt what it reports.
            app.scrollback_lines = settings.scrollback_lines;
            reconcile_catalog(&mut app.core, &catalog, false);
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
                    app.daemon_catalog = Some(catalog);
                }
                DaemonMsg::SettingsChanged { settings } => {
                    app.scrollback_lines = settings.scrollback_lines;
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
                            return Task::done(Message::TerminalFocused);
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
                    _ => {}
                },
                // FR-024: a stage push names the step in flight. Peeked, not removed — the
                // operation is still running and its terminal reply still needs the pending op.
                DaemonMsg::OperationProgress { req, stage } => {
                    if matches!(
                        app.pending_ops.get(&req),
                        Some(PendingOp::WorktreeCreate(_))
                    ) {
                        app.core.update(Message::WorktreeCreateStageChanged(stage));
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
                // Other control messages (Pong, Attached) are consumed as their flows land.
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
        // "Restart service" (FR-022): stop the mismatched daemon by its recorded pid. A mismatched
        // client can't send it a control message, so termination is the version-agnostic stop. Once
        // it exits, the auto-reconnect loop finds nothing listening and spawns a matching daemon;
        // previously-live sessions then reload as interrupted-resumable (FR-006a). Live processes are
        // lost — we say so — but the durable sessions survive.
        Message::ConnectionRestartServiceRequested => {
            app.version_mismatch = None;
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

        // Advance every animation toward its target via the shared driver.
        Message::AnimationTick => {
            apply_motion_targets(app);
            app.motion.tick();
            app.row_fx.tick();
            // Once the leaving overlay has fully faded, release its snapshot.
            if app.dismissing.is_some() && app.motion.get(MotionKey::Overlay) <= 0.001 {
                app.dismissing = None;
            }
            Task::none()
        }
        // Pointer entered/left the sidebar resize handle; the hover highlight animates via the
        // animation clock.
        Message::SidebarHandleHovered(hovered) => {
            app.handle_hovered = hovered;
            Task::none()
        }
        Message::ProjectSelectorOpened => {
            let dir = start_dir();
            app.core.selector = Some(Selector::open_at(dir.clone()));
            app.core.open_overlay(Overlay::ProjectSelector);
            scan_task(dir)
        }
        Message::SelectorNavigatedInto(_) | Message::SelectorNavigatedUp => {
            app.core.update(message);
            match &app.core.selector {
                Some(selector) if selector.status == SelectorStatus::Loading => {
                    scan_task(selector.current_dir.clone())
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
            app.core.overlay = Overlay::None;
            if !GitCli::new().is_repo_root(&path) {
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
                .open_or_activate(path.clone(), &StdFolderScanner::new());
            app.core.restore_after_activation(&path);
            app.core.set_worktrees(discover_worktrees(&path));
            app.core.worktree_error = None;
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
            app.core
                .workspace
                .refresh_availability(&StdFolderScanner::new());
            // Non-destructive switch: keep the outgoing project's sessions running in the
            // background and restore the target project's foreground (feature 008, BS-1/BS-3).
            let previous = app.core.workspace.active.clone();
            if app.core.switch_active(&path) {
                app.core.set_worktrees(discover_worktrees(&path));
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
            persist_settings(&mut app.core);
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
            // View the selected session (the daemon streams its grid), resuming it if idle.
            app.selection = None;
            app.display_offset = 0;
            if let (Some(project), Some(d)) = (app.core.workspace.active.clone(), &app.daemon) {
                d.send(ClientMsg::SessionStart { session: id });
                d.send(ClientMsg::SetViewedSession {
                    project,
                    session: Some(id),
                });
            }
            // BUG-001: auto-focus the selected session's terminal (FR-010/FR-010a). Selecting from
            // the sidebar is a click *outside* the pane, so a currently-focused pane also publishes
            // `TerminalFocusReleased` for the same click. Re-assert focus via a follow-up message,
            // which is delivered *after* the current event batch drains — so the focus wins
            // regardless of the intra-batch order of `SessionSelected` vs `TerminalFocusReleased`.
            Task::done(Message::TerminalFocused)
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
                persist(&mut app.core);
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
                    persist(&mut app.core);
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
            persist(&mut app.core);
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
            let content = selected_text(app);
            if content.is_empty() {
                Task::none()
            } else {
                iced::clipboard::write(content)
            }
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
        Message::TextCopyRequested(text) => {
            app.core.update(Message::WorktreeMenuDismissed);
            iced::clipboard::write(text)
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
            if let Some(store) = JsonFileSettingsStore::default_location() {
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
                send_op(app, PendingOp::WorktreeDelete(dir), move |req| {
                    ClientMsg::WorktreeDelete {
                        req,
                        project: p,
                        dir_name: d,
                        stop_sessions: true,
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
    // Render the displayed session from its daemon-streamed grid cache + the client-side selection
    // and scroll offset (feature 010). The daemon is the single source of screen state.
    micold_client::ui::view(
        &app.core,
        app.attached_grid(),
        app.selection.as_ref(),
        app.display_offset,
        &app.motion,
        app.dismissing.as_ref(),
        &app.row_fx,
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
/// — it blocks every connection and has the most specific action — then a per-project takeover (US5),
/// then a plain disconnect. Each names the situation and offers a concrete action.
fn connection_status(app: &App) -> micold_client::ui::ConnectionStatus {
    use micold_client::ui::ConnectionStatus;
    if let Some((client, daemon, daemon_build)) = &app.version_mismatch {
        return ConnectionStatus::VersionMismatch {
            client: *client,
            daemon: *daemon,
            daemon_build: daemon_build.clone(),
        };
    }
    if let Some(project) = app.core.workspace.active.as_ref() {
        if let Some(by) = app.displaced.get(project) {
            return ConnectionStatus::Displaced { by: by.clone() };
        }
    }
    if app.disconnected {
        ConnectionStatus::Disconnected
    } else {
        ConnectionStatus::Connected
    }
}

fn theme(app: &App) -> iced::Theme {
    micold_client::ui::style::theme(app.core.color_scheme())
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
        d.send(ClientMsg::SessionStart { session: id });
        d.send(ClientMsg::SetViewedSession {
            project,
            session: Some(id),
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
    if let Some(id) = core.active_session {
        if core.workspace.find_session(id).is_none() {
            core.active_session = None;
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

/// The selected text of the displayed session, or empty when nothing is selected.
fn selected_text(app: &App) -> String {
    let Some(id) = app.core.active_session else {
        return String::new();
    };
    let (Some(grid), Some(sel)) = (app.grids.get(&id), app.selection.as_ref()) else {
        return String::new();
    };
    sel.text(|id| grid.line(id).map(|l| l.text.clone()))
}

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
    // The terminal output poll is gone — the daemon streams grid frames over the connection. Worktree
    // create now runs on the daemon too, so there is no local progress buffer to drain (T055).
    // Run the animation clock only while something is actually animating (FR-014).
    if motion_animating(app) {
        subs.push(every(ANIM_TICK).map(|_| Message::AnimationTick));
    }
    // Track the pointer ONLY while the project switcher is open (feature 015), so a right-click
    // on a row can anchor its context menu at the cursor. Scoping it this way keeps the idle
    // window free of per-mouse-move redraws — the switcher is a brief, deliberate interaction.
    if app.core.project_switcher_open {
        subs.push(cursor_move_events());
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
fn discover_worktrees(repo: &Path) -> Vec<Worktree> {
    micold_core::worktree::discover(&GitCli::new(), repo)
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
    dark_light::detect().map(map_system_scheme).map_err(|_| ())
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

fn scan_task(dir: PathBuf) -> Task<Message> {
    Task::perform(async move { scan(dir) }, |message| message)
}

fn scan(dir: PathBuf) -> Message {
    match StdFolderScanner::new().list_subdirs(&dir) {
        Ok(entries) => Message::SelectorListingReady(entries),
        Err(error) => Message::SelectorListingFailed(error.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use micold_client::app::{NoticeLevel, Notification};
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
        SessionSummary {
            id,
            worktree_dir: None,
            title: SessionLabel::Named(title.into()),
            lifecycle,
            activity: ActivitySignal::Unknown,
        }
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
        assert_eq!(
            core.notifications,
            vec![Notification {
                level: NoticeLevel::Info,
                message: "A background session was restarted while you were away.".to_string(),
            }]
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

    /// Regression: iced 0.13's `Subscription::map` asserts the closure is zero-sized and panics
    /// at construction otherwise. Threading `last_known` through the closure captured it and
    /// crashed the app on startup; the poll now emits a unit message, so building it must not
    /// panic.
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
            core: State::default(),
            grids: HashMap::new(),
            stamper: SessionInputStamper::new(),
            selection: None,
            display_offset: 0,
            scrollback_lines: micold_core::settings::DEFAULT_SCROLLBACK_LINES,
            motion: Animator::new(),
            main_key: main_content_key(&State::default()),
            handle_hovered: false,
            dismissing: None,
            row_fx: Animator::new(),
            prev_hovered: None,
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
            next_req: 0,
            pending_ops: HashMap::new(),
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
            core: State::default(),
            grids: HashMap::new(),
            stamper: SessionInputStamper::new(),
            selection: None,
            display_offset: 0,
            scrollback_lines: micold_core::settings::DEFAULT_SCROLLBACK_LINES,
            motion: Animator::new(),
            main_key: main_content_key(&State::default()),
            handle_hovered: false,
            dismissing: None,
            row_fx: Animator::new(),
            prev_hovered: None,
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
            next_req: 0,
            pending_ops: HashMap::new(),
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
}
