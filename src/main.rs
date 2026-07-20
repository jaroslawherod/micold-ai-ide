//! Micold AI IDE — GUI binary entry point.
//!
//! Adapts the render-free core (`micold_ai_ide::app`) to the iced runtime. All state
//! transitions live in the core and are unit-tested there; this layer renders state, performs
//! the feature's I/O at the boundary (filesystem scans, git worktree ops via `GitCli`, PTY
//! spawning via `portable-pty`), and holds the gui-only runtime handles that cannot live in
//! the pure (Clone/Eq) core `State` — the per-session [`RuntimeTerminal`]s.

mod ui;

use iced::time::every;
use iced::{Subscription, Task};
use micold_ai_ide::app::{
    Message, Overlay, RenameDraft, SettingsDraft, State, WorktreeForm, WorktreeFormStatus,
    WorktreeRenameDraft,
};
use micold_ai_ide::env_include::{self, EnvIncludeOutcome};
use micold_ai_ide::fs_scan::{FolderScanner, StdFolderScanner};
use micold_ai_ide::git::{Git, GitCli};
use micold_ai_ide::motion::Animator;
use micold_ai_ide::provider::{AiCliProvider, ClaudeProvider};
use micold_ai_ide::selector::{Selector, SelectorStatus};
use micold_ai_ide::session::{
    RestartDecision, Session, SessionId, SessionLabel, SessionLifecycle, SessionLocation,
    TerminalMode,
};
use micold_ai_ide::settings::{JsonFileSettingsStore, Settings, SettingsStore};
use micold_ai_ide::store::{JsonFileStore, ProjectStore};
use micold_ai_ide::terminal::{LaunchMode, LaunchSpec};
use micold_ai_ide::theme::SystemScheme;
use micold_ai_ide::worktree::{
    create_worktree, parse_worktrees, reconcile, remove_worktree, remove_worktree_dir, CreateError,
    Worktree,
};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use ui::terminal::{spawn_pty, spawn_shell_pty, RuntimeTerminal, SessionTerminals};
use ui::MotionKey;

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

/// How often live terminals are polled for streamed output + process-exit detection.
const TERMINAL_POLL: Duration = Duration::from_millis(120);

/// How often live terminals are polled while the window is unfocused (idle-CPU fix). Coarser
/// than [`TERMINAL_POLL`] rather than fully suspended: a fully-suspended poll would also stall
/// crash detection/auto-restart and title-sync indefinitely, and let `RuntimeTerminal`'s PTY
/// output buffer grow unbounded for as long as the window stays backgrounded (code review
/// findings). This still cuts the tick rate — and its full-`view()` rebuild cost — by ~17x
/// while backgrounded.
const BACKGROUND_TERMINAL_POLL: Duration = Duration::from_secs(2);

/// How often the in-flight worktree create's progress buffer is drained into the form's log.
/// Active only while a create is running, so it costs nothing the rest of the time.
const CREATE_PROGRESS_POLL: Duration = Duration::from_millis(150);

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

/// A snapshot of a just-closed overlay, kept alive by the binary so it can keep being rendered
/// while it fades out. The pure core clears the overlay + its draft synchronously on close, so
/// we capture the data here *before* the reducer runs and render from this snapshot during the
/// exit animation. Each variant carries a clone of exactly what that overlay's render function
/// needs (all `Clone`, straight from the core `State`).
enum ClosingOverlay {
    About,
    Selector(Selector),
    Rename(RenameDraft),
    Worktree(WorktreeForm, Option<String>),
    Settings(SettingsDraft),
    ConfirmDelete(String),
    WorktreeRename(WorktreeRenameDraft),
}

/// The binary's application state: the pure core plus gui-only runtime handles.
struct App {
    core: State,
    /// Live PTY sessions, keyed by session id (never part of the pure core — not Clone/Eq). Each
    /// session may hold up to two live processes — AI CLI and shell (feature 010) — of which at
    /// most one is attached to the visible pane at a time, per that session's `TerminalMode`.
    terminals: HashMap<SessionId, SessionTerminals>,
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
    /// Progress lines produced by the in-flight worktree create, shared with the worker running
    /// it. Drained into the form's log on [`CREATE_PROGRESS_POLL`] — the same
    /// shared-buffer-plus-tick idiom `RuntimeTerminal` uses to stream PTY output, since the
    /// producer here is likewise a blocking job that cannot dispatch messages itself.
    create_progress: Arc<Mutex<Vec<String>>>,
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
    /// The shared, in-memory-only resolved-environment snapshot (data-model.md). One instance per
    /// app run, applied uniformly to every session's spawn call site — never persisted (FR-008).
    env_include: EnvIncludeSnapshot,
}

/// The result of the most recently resolved (or not-yet-attempted) environment-include snapshot
/// (feature 011, data-model.md). `vars` is empty for every non-`Success` outcome.
struct EnvIncludeSnapshot {
    vars: Vec<(String, String)>,
    outcome: EnvIncludeOutcome,
}

/// Resolve the environment-include snapshot from the given settings values, short-circuiting to
/// `Disabled` (no subprocess spawned) when the feature is off or the path is blank — mirrors the
/// spec's Edge Cases and contracts/env-include-resolution.md's Non-goals (the engine itself never
/// decides whether to run). Shared by `boot()` (T013) and `refresh_env_include` (T020) so both
/// triggers apply the exact same short-circuit + resolution logic.
fn resolve_env_include(enabled: bool, script_path: &str, timeout_secs: u64) -> EnvIncludeSnapshot {
    if !enabled || script_path.trim().is_empty() {
        return EnvIncludeSnapshot {
            vars: Vec::new(),
            outcome: EnvIncludeOutcome::Disabled,
        };
    }
    let (vars, outcome) =
        env_include::resolve(Path::new(script_path), Duration::from_secs(timeout_secs));
    EnvIncludeSnapshot { vars, outcome }
}

/// Force a fresh re-source of the environment-include script from `app`'s current settings,
/// replacing `app.env_include` wholesale (feature 011, FR-007). Called on a `SettingsSaved` that
/// touched the enabled/path/timeout fields, and on `TerminalRestartRequested` for any session —
/// the two refresh triggers the spec's Clarifications name.
fn refresh_env_include(app: &mut App) {
    app.env_include = resolve_env_include(
        app.env_include_enabled,
        &app.env_include_script_path,
        app.env_include_timeout_secs,
    );
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
    /// Kill every child process (AI CLI and shell) on shutdown so none are orphaned (research
    /// R5, T057; feature 010 extends this to both processes per session).
    fn drop(&mut self) {
        for st in self.terminals.values_mut() {
            st.kill_all();
        }
    }
}

impl App {
    /// The `RuntimeTerminal` currently attached to the active session's displayed pane, if any
    /// (feature 010, FR-007) — routes through that session's current `TerminalMode` rather than
    /// assuming the AI CLI, so every keystroke/render/selection call site is automatically
    /// correct in either mode without a mode check at each call site.
    fn attached_terminal_mut(&mut self) -> Option<&mut RuntimeTerminal> {
        let id = self.core.active_session?;
        let mode = self
            .core
            .active_sessions()
            .iter()
            .find(|s| s.id == id)?
            .mode;
        self.terminals.get_mut(&id)?.attached_mut(mode)
    }

    /// Read-only counterpart of [`App::attached_terminal_mut`].
    fn attached_terminal(&self) -> Option<&RuntimeTerminal> {
        let id = self.core.active_session?;
        let mode = self
            .core
            .active_sessions()
            .iter()
            .find(|s| s.id == id)?
            .mode;
        self.terminals.get(&id)?.attached(mode)
    }
}

/// The app window icon as raw 64x64 RGBA (generated from `assets/icon/icon.svg` by
/// `assets/icon/generate.py`). Embedded directly so no runtime image decoder is needed.
const ICON_RGBA: &[u8] = include_bytes!("../assets/icon/icon-64.rgba");

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
        .font(ui::MATERIAL_SYMBOLS_BYTES)
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
    let mut scrollback_lines = micold_ai_ide::settings::DEFAULT_SCROLLBACK_LINES;
    let mut env_include_enabled = micold_ai_ide::settings::DEFAULT_ENV_INCLUDE_ENABLED;
    let mut env_include_script_path = Settings::default().env_include_script_path;
    let mut env_include_timeout_secs = micold_ai_ide::settings::DEFAULT_ENV_INCLUDE_TIMEOUT_SECS;
    if let Some(store) = JsonFileSettingsStore::default_location() {
        let loaded = store.load().settings;
        core.theme_pref = loaded.theme;
        scrollback_lines = loaded.scrollback_lines;
        env_include_enabled = loaded.env_include_enabled;
        env_include_script_path = loaded.env_include_script_path;
        env_include_timeout_secs = loaded.env_include_timeout_secs;
    }
    let env_include = resolve_env_include(
        env_include_enabled,
        &env_include_script_path,
        env_include_timeout_secs,
    );
    core.system_scheme = detect_system_scheme();
    // If a project is already active from a previous run, discover its worktrees.
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
            terminals: HashMap::new(),
            scrollback_lines,
            motion,
            main_key,
            handle_hovered: false,
            dismissing: None,
            row_fx: Animator::new(),
            prev_hovered: None,
            window_focused: true,
            create_progress: Arc::new(Mutex::new(Vec::new())),
            last_grid: None,
            env_include_enabled,
            env_include_script_path,
            env_include_timeout_secs,
            env_include,
        },
        Task::none(),
    )
}

/// Persist the catalog. Empty sessions — those `claude` never recorded a conversation for —
/// are NOT preserved, so a restart never tries to resume a nonexistent session (bug fix; see
/// spec Clarifications 2026-07-16). A save failure is non-fatal (Principle IV).
fn persist(core: &State) {
    if let Some(store) = JsonFileStore::default_location() {
        let mut to_save = core.workspace.clone();
        prune_empty_sessions(&mut to_save);
        let _ = store.save(&to_save);
    }
}

/// Remove sessions that have no `claude` conversation on disk (empty sessions).
fn prune_empty_sessions(workspace: &mut micold_ai_ide::workspace::Workspace) {
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
fn session_has_conversation(
    project_path: &Path,
    session: &micold_ai_ide::session::Session,
) -> bool {
    let provider = ClaudeProvider;
    let cwd = session_cwd_for_location(project_path, &session.location);
    let Some(config) = provider.config_dir() else {
        // Cannot determine the provider config dir — do not drop the session on uncertainty.
        return true;
    };
    provider.has_recorded_conversation(&config, &cwd, session.id.0)
}

fn persist_settings(core: &State) {
    if let Some(store) = JsonFileSettingsStore::default_location() {
        // Preserve the persisted scrollback limit (feature 006) and environment-include settings
        // (feature 011) when saving a theme change — this function only ever changes `theme`.
        let existing = store.load().settings;
        let _ = store.save(&Settings {
            theme: core.theme_pref,
            ..existing
        });
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
            app.row_fx.to(ui::worktree_fx_key(old), 0.0, s);
        }
        if let Some(new) = &app.core.hovered_worktree {
            let key = ui::worktree_fx_key(new);
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
    }
}

fn update_inner(app: &mut App, message: Message) -> Task<Message> {
    match message {
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
            // foreground FIRST (I1), then finish the switch bookkeeping for the new project.
            app.core.record_foreground();
            app.core
                .workspace
                .open_or_activate(path.clone(), &StdFolderScanner::new());
            app.core.restore_after_activation(&path);
            app.core.set_worktrees(discover_worktrees(&path));
            app.core.worktree_error = None;
            persist(&app.core);
            Task::none()
        }
        Message::KnownProjectReopened(path) => {
            app.core
                .workspace
                .refresh_availability(&StdFolderScanner::new());
            // Non-destructive switch: keep the outgoing project's sessions running in the
            // background and restore the target project's foreground (feature 008, BS-1/BS-3).
            if app.core.switch_active(&path) {
                app.core.set_worktrees(discover_worktrees(&path));
                persist(&app.core);
            }
            Task::none()
        }
        Message::RenameConfirmed => {
            app.core.update(Message::RenameConfirmed);
            persist(&app.core);
            Task::none()
        }
        // Worktree rename (feature 008, FR-014/FR-015): apply the display-name override in the
        // core, then persist it so it survives a restart. Never touches the folder or branch.
        Message::WorktreeRenameConfirmed => {
            app.core.update(Message::WorktreeRenameConfirmed);
            persist(&app.core);
            Task::none()
        }
        Message::ThemePreferenceChanged(_) | Message::ThemeModeCycled => {
            app.core.update(message);
            persist_settings(&app.core);
            Task::none()
        }
        // Validate the form, then create the worktree (incl. any submodule fetch) via git,
        // off the update() thread so a slow fetch doesn't freeze the UI (feature 010,
        // research R4). AddWorktreeSubmitted/WorktreeCreated/WorktreeCreateFailed keep their
        // existing meaning; WorktreeCreateStarted is dispatched first so the form can show it.
        Message::AddWorktreeSubmitted => {
            app.core.update(Message::AddWorktreeSubmitted);
            let Some(form) = app.core.worktree_form.clone() else {
                return Task::none();
            };
            if form.status != WorktreeFormStatus::Editing {
                return Task::none(); // a create is already in flight — no double-submit.
            }
            let Ok(names) = form.preview() else {
                return Task::none(); // validation error already recorded by the reducer
            };
            let Some(repo) = app.core.workspace.active.clone() else {
                return Task::none();
            };
            app.core.update(Message::WorktreeCreateStarted);
            // Starting a create clears the form's log, so drop anything a previous attempt left
            // buffered — otherwise its tail would be drained into the new attempt's log.
            let progress = Arc::clone(&app.create_progress);
            drain_create_progress(&progress);
            Task::perform(
                async move { create(&repo, &names, &progress).map_err(describe_create_error) },
                |result| Message::WorktreeCreationDone { result },
            )
        }
        // Start a new session at a location — a worktree or the project root ("Default",
        // feature 010): spawn `claude` and stream it (FR-010/012/013). A `Default` location
        // never creates, modifies, or removes a worktree (FR-002) — it simply runs in `repo`
        // itself, so this arm never calls into `micold_ai_ide::worktree`.
        Message::SessionStartRequested { location } => {
            let mut started = false;
            if let Some(repo) = app.core.workspace.active.clone() {
                let cwd = session_cwd_for_location(&repo, &location);
                let session = Session::start_new(location);
                let id = session.id;
                match spawn_pty(
                    &launch_spec(&cwd, id, LaunchMode::Fresh, &app.env_include.vars),
                    app.scrollback_lines,
                    app.last_grid,
                ) {
                    Ok(rt) => {
                        app.terminals.insert(
                            id,
                            SessionTerminals {
                                ai_cli: Some(rt),
                                shell: None,
                            },
                        );
                        app.core.update(Message::SessionStarted(session));
                        app.core.update(Message::SessionRunning(id));
                        persist(&app.core);
                        started = true;
                    }
                    Err(err) => {
                        // Feature 005 FR-017. Previously stored in `worktree_error`, whose only
                        // render site is inside the Add Worktree modal — not open here, so a
                        // failed spawn (typically `claude` missing from PATH) was silent.
                        app.core
                            .notify_error(format!("Could not start session: {err}"));
                    }
                }
            }
            // BUG-001: auto-focus the newly-started session's terminal (FR-010/FR-010a), using the
            // same after-the-batch follow-up as `SessionSelected` so it wins over any release
            // published by the same click that started the session.
            if started {
                Task::done(Message::TerminalFocused)
            } else {
                Task::none()
            }
        }
        // Selecting a session reattaches/resumes whichever process its persisted mode selects
        // (FR-005, FR-011) — an Idle AI CLI session resumes via `claude --resume` (FR-023a); a
        // session last left in Regular mode gets a fresh shell instead.
        Message::SessionSelected(id) => {
            app.core.update(Message::SessionSelected(id));
            ensure_attached_process(app, id);
            // BUG-001: auto-focus the selected session's terminal (FR-010/FR-010a). Selecting from
            // the sidebar is a click *outside* the pane, so a currently-focused pane also publishes
            // `TerminalFocusReleased` for the same click. Re-assert focus via a follow-up message,
            // which is delivered *after* the current event batch drains — so the focus wins
            // regardless of the intra-batch order of `SessionSelected` vs `TerminalFocusReleased`.
            Task::done(Message::TerminalFocused)
        }
        // Close a session: kill both its processes (AI CLI and shell, feature 010 FR-014) and
        // drop the runtime handles (FR-015a).
        Message::SessionCloseRequested(id) => {
            if let Some(mut st) = app.terminals.remove(&id) {
                st.kill_all();
            }
            app.core.update(Message::SessionCloseRequested(id));
            persist(&app.core);
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
                ensure_attached_process(app, id);
                persist(&app.core);
            }
            Task::none()
        }
        // Manually restart the active session's currently-attached, not-running process
        // (FR-013) — the shell never auto-restarts, so this is its only path back; also covers
        // an Idle/Failed AI CLI, which previously had no explicit affordance. Also re-sources the
        // environment-include script fresh (feature 011, FR-007) — the spec's Clarifications name
        // this restart control as a manual-retry path for a previously-failed script, alongside
        // the Settings-save refresh trigger.
        Message::TerminalRestartRequested => {
            refresh_env_include(app);
            if let Some(id) = app.core.active_session {
                ensure_attached_process(app, id);
            }
            Task::none()
        }
        // Worktree creation completed: apply progress to the form, then dispatch the result
        // (success or failure). This splits the combined result into two state transitions so
        // progress is displayed before the form closes or error shows (feature 010 follow-up).
        Message::WorktreeCreationDone { result } => {
            // Drain the tail the last poll missed BEFORE the result, so a failure's final lines
            // ("submodule update failed: …", "Rolling back…") are in the log the form keeps
            // open for diagnosis.
            let tail = drain_create_progress(&app.create_progress);
            if !tail.is_empty() {
                app.core.update(Message::WorktreeCreateLogAppended(tail));
            }
            app.core.update(Message::WorktreeCreationDone { result });
            persist(&app.core);
            Task::none()
        }
        // Tick while a create runs: hand the worker's buffered lines to the form (feature 010
        // follow-up). This is `WorktreeCreateLogAppended`'s producer — the reducer arm and its
        // tests existed, but nothing ever dispatched it, so the log only appeared in one batch
        // at completion and never rendered on success at all.
        Message::WorktreeCreateProgressPolled => {
            let lines = drain_create_progress(&app.create_progress);
            if !lines.is_empty() {
                app.core.update(Message::WorktreeCreateLogAppended(lines));
            }
            Task::none()
        }
        // Stream live keystrokes/paste to the displayed session's currently-ATTACHED process
        // (FR-007/FR-008), but only while that process is Running (FR-012a, feature 010 extends
        // the write-gate to the shell): input to a non-running process is discarded, not
        // buffered.
        Message::TerminalBytes(bytes) => {
            if let Some(id) = app.core.active_session {
                let running = app
                    .core
                    .active_sessions()
                    .iter()
                    .find(|s| s.id == id)
                    .map(|s| match s.mode {
                        TerminalMode::AiCli => micold_ai_ide::app::should_write_to(s.lifecycle),
                        TerminalMode::Regular => {
                            micold_ai_ide::app::should_write_to_shell(s.shell_lifecycle)
                        }
                    })
                    .unwrap_or(false);
                if running {
                    if let Some(rt) = app.attached_terminal_mut() {
                        let _ = rt.write(&bytes);
                    }
                    // The user interacted, so `claude` now has a conversation — persist it so it
                    // can be resumed after a restart (FR-020; empty sessions still pruned).
                    persist(&app.core);
                }
            }
            Task::none()
        }
        // Mouse text selection on the displayed session's grid (FR-013).
        Message::TerminalSelectStart { col, line, kind } => {
            if let Some(rt) = app.attached_terminal_mut() {
                rt.selection_start(col, line, kind);
            }
            Task::none()
        }
        Message::TerminalSelectUpdate { col, line } => {
            if let Some(rt) = app.attached_terminal_mut() {
                rt.selection_update(col, line);
            }
            Task::none()
        }
        Message::TerminalSelectCleared => {
            if let Some(rt) = app.attached_terminal_mut() {
                rt.selection_clear();
            }
            Task::none()
        }
        // Reflow the displayed session's PTY + grid to the visible size (FR-014/FR-015).
        Message::TerminalResized { cols, rows } => {
            // Remember the pane's live size so the next spawned session starts at it too, rather
            // than the hardcoded default (bugfix: new terminal not starting fullscreen).
            app.last_grid = Some((cols, rows));
            if let Some(rt) = app.attached_terminal_mut() {
                let _ = rt.resize(cols, rows);
            }
            Task::none()
        }
        // Scroll the displayed session's local scrollback (FR-016).
        Message::TerminalScrolled(delta) => {
            if let Some(rt) = app.attached_terminal_mut() {
                rt.scroll(delta);
            }
            Task::none()
        }
        // Scroll to an absolute offset (scrollbar drag). Resolve the delta against the LIVE offset
        // here at apply time so a burst of batched drag messages converges instead of accumulating
        // stale relative deltas (drag flicker fix, FR-016).
        Message::TerminalScrolledTo(target) => {
            if let Some(rt) = app.attached_terminal_mut() {
                rt.scroll(ui::target_offset_delta(rt.display_offset(), target));
            }
            Task::none()
        }
        // Copy the current selection to the system clipboard (FR-013). Also closes the menu.
        Message::TerminalCopyRequested => {
            app.core.update(Message::TerminalContextMenuClosed);
            let content = app
                .attached_terminal()
                .map(|rt| rt.selectable_content())
                .unwrap_or_default();
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

            let scrollback_min = micold_ai_ide::settings::MIN_SCROLLBACK_LINES;
            let scrollback_max = micold_ai_ide::settings::MAX_SCROLLBACK_LINES;
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

            let timeout_min = micold_ai_ide::settings::MIN_ENV_INCLUDE_TIMEOUT_SECS;
            let timeout_max = micold_ai_ide::settings::MAX_ENV_INCLUDE_TIMEOUT_SECS;
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
                let _ = store.save(&Settings {
                    theme: app.core.theme_pref,
                    scrollback_lines,
                    env_include_enabled: app.env_include_enabled,
                    env_include_script_path: app.env_include_script_path.clone(),
                    env_include_timeout_secs,
                });
            }
            refresh_env_include(app);
            app.core.update(Message::SettingsSaved); // closes the overlay
            Task::none()
        }
        Message::TerminalTick => {
            // Pump BOTH of a session's processes every tick, regardless of which is attached
            // (research R6) — this is what keeps a backgrounded AI CLI's crash-loop restart
            // working while Regular mode is displayed, and keeps the detached process's `Term`
            // state current so re-attaching renders instantly correct.
            for st in app.terminals.values_mut() {
                for rt in st.each_mut() {
                    rt.pump();
                }
            }
            sync_session_titles(app);
            handle_process_exits(app);
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
        Message::WorktreeDeleteConfirmed => {
            let target = app.core.worktree_delete_target.clone();
            if let (Some(dir), Some(repo)) = (target, app.core.workspace.active.clone()) {
                // Facts to remove — captured before the reducer drops them from state.
                let wt = app
                    .core
                    .worktrees
                    .iter()
                    .find(|w| w.dir_name == dir)
                    .cloned();
                // Terminate this worktree's running sessions first (both processes per
                // session, feature 010 FR-014).
                for id in app.core.sessions_in_worktree(&dir) {
                    if let Some(mut st) = app.terminals.remove(&id) {
                        st.kill_all();
                    }
                }
                if let Some(wt) = wt {
                    // FR-023: both results were previously discarded with `let _ =`, so a
                    // locked worktree, a branch checked out elsewhere, or a permission error
                    // made the row vanish from the sidebar while the branch and directory
                    // survived on disk. The reconcile below restores a truthful sidebar; these
                    // report *why* it did not go away.
                    let name = app.core.worktree_display_name(&dir);
                    match remove_worktree(&GitCli::new(), &repo, &wt.path, wt.branch.as_deref()) {
                        // Only remove the directory once git has released the worktree —
                        // deleting the working files of a still-registered worktree would
                        // leave a worse mess than the failure being reported.
                        Ok(()) => {
                            // `remove_worktree_dir` treats an already-absent directory as
                            // success — git removed it as part of releasing the worktree, so
                            // "not found" here is the happy path, not a leftover (FR-023a).
                            if let Err(err) = remove_worktree_dir(&wt.path) {
                                app.core.notify_error(format!(
                                    "Deleted worktree \"{name}\", but its folder could not be \
                                     removed: {err}. Left at {}",
                                    wt.path.display()
                                ));
                            }
                        }
                        Err(err) => {
                            app.core.notify_error(format!(
                                "Could not delete worktree \"{name}\": {err}"
                            ));
                        }
                    }
                }
                // Drop the session/worktree records in the core, then reconcile from git truth.
                app.core.update(Message::WorktreeDeleteConfirmed);
                app.core.set_worktrees(discover_worktrees(&repo));
                persist(&app.core);
            } else {
                app.core.update(Message::WorktreeDeleteConfirmed);
            }
            Task::none()
        }
        other => {
            app.core.update(other);
            Task::none()
        }
    }
}

fn view(app: &App) -> iced::Element<'_, Message> {
    // Supply the active session's currently-ATTACHED live terminal runtime to the
    // colour-rendering pane (feature 006; feature 010 routes this through `TerminalMode` rather
    // than assuming AI CLI — this is what makes FR-008 identical-real-terminal-behavior true by
    // construction: both modes render through the same pane).
    let terminal = app.attached_terminal();
    ui::view(
        &app.core,
        terminal,
        &app.motion,
        app.dismissing.as_ref(),
        &app.row_fx,
        &app.env_include.outcome,
    )
}

fn theme(app: &App) -> iced::Theme {
    ui::style::theme(app.core.color_scheme())
}

fn subscription(app: &App) -> Subscription<Message> {
    // Event-driven (not a poll): reports actual OS focus changes, so it costs nothing while
    // the window sits idle either focused or not (idle-CPU fix).
    let mut subs = vec![ui::subscription(&app.core), window_focus_events()];
    // Always polled — see [`BACKGROUND_OS_THEME_POLL`]. Only the cadence follows focus.
    subs.push(os_theme_poll(os_theme_poll_interval(app.window_focused)));
    if let Some(interval) = terminal_poll_interval(!app.terminals.is_empty(), app.window_focused) {
        subs.push(every(interval).map(|_| Message::TerminalTick));
    }
    // Drain create progress only while a create is actually in flight.
    if creating_worktree(app) {
        subs.push(every(CREATE_PROGRESS_POLL).map(|_| Message::WorktreeCreateProgressPolled));
    }
    // Run the animation clock only while something is actually animating (FR-014).
    if motion_animating(app) {
        subs.push(every(ANIM_TICK).map(|_| Message::AnimationTick));
    }
    Subscription::batch(subs)
}

/// Whether a worktree create is in flight, so its progress buffer needs draining.
fn creating_worktree(app: &App) -> bool {
    app.core
        .worktree_form
        .as_ref()
        .is_some_and(|form| form.status == WorktreeFormStatus::Creating)
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

/// The terminal poll interval for this tick, or `None` if there are no terminals to poll:
/// [`TERMINAL_POLL`] while the window has focus (redraw responsiveness matters), the coarser
/// [`BACKGROUND_TERMINAL_POLL`] while unfocused (still detects crashes, keeps titles in sync,
/// and drains the PTY buffer — just far less often than the foreground cadence).
fn terminal_poll_interval(has_terminals: bool, window_focused: bool) -> Option<Duration> {
    if !has_terminals {
        return None;
    }
    Some(if window_focused {
        TERMINAL_POLL
    } else {
        BACKGROUND_TERMINAL_POLL
    })
}

/// Subscribes to raw OS window events and keeps only focus changes, translating them into
/// [`Message::WindowFocusChanged`]. Every other window event (resize, move, redraw, ...) is
/// discarded before it ever reaches `update`.
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

/// Detect processes that exited unexpectedly and apply the crash-loop guard (FR-022/022a).
/// Reconcile each active session's sidebar label with the AI CLI provider's current session
/// title (FR-011a, SC-009, bugfix BUG-002 / completes T054). Runs on the terminal poll so the
/// label tracks the provider's name as the conversation evolves — placeholder → provider name,
/// and updated whenever the provider changes the name, so the two never stay diverged.
///
/// Best-effort I/O: a missing/unreadable transcript is a no-op and never fails the session.
/// Collect the updates first (immutable borrow of the sessions) then apply, avoiding a borrow
/// conflict with the reducer. Cwd site 3/5 (research.md R2).
fn sync_session_titles(app: &mut App) {
    let provider = ClaudeProvider;
    let Some(config) = provider.config_dir() else {
        return;
    };
    let Some(project) = app.core.workspace.active.clone() else {
        return;
    };
    let mut updates: Vec<(SessionId, String)> = Vec::new();
    for session in app.core.active_sessions() {
        if !session.is_active() {
            continue;
        }
        let cwd = session_cwd_for_location(&project, &session.location);
        if let Some(title) = provider.read_title(&config, &cwd, session.id.0) {
            if session.label != SessionLabel::Named(title.clone()) {
                updates.push((session.id, title));
            }
        }
    }
    for (id, title) in updates {
        app.core.update(Message::SessionTitleUpdated { id, title });
    }
}

fn handle_process_exits(app: &mut App) {
    // Detect each slot's exit independently (feature 010) — a session may have one, both, or
    // neither process exit in the same tick, and they're handled by entirely different policies
    // (AI CLI: crash-loop auto-restart; shell: never auto-restarted, FR-013).
    let mut ai_cli_exited: Vec<SessionId> = Vec::new();
    let mut shell_exited: Vec<SessionId> = Vec::new();
    for (id, st) in app.terminals.iter_mut() {
        if st.ai_cli.as_mut().is_some_and(|rt| rt.has_exited()) {
            ai_cli_exited.push(*id);
        }
        if st.shell.as_mut().is_some_and(|rt| rt.has_exited()) {
            shell_exited.push(*id);
        }
    }

    for id in ai_cli_exited {
        if let Some(st) = app.terminals.get_mut(&id) {
            st.ai_cli = None;
        }
        // Resolve the exited session in ANY project — a background session of an inactive
        // project must still be handled, not silently dropped (feature 008, BS-6). This is
        // unconditional on `TerminalMode` (research R6): the AI CLI's crash-loop guard applies
        // whether or not Regular mode is currently displayed.
        let Some((cwd, lifecycle)) = session_cwd_any(&app.core, id) else {
            continue;
        };
        // An intentional stop (Idle) is not a crash — do not auto-restart.
        if lifecycle == SessionLifecycle::Idle {
            continue;
        }
        let decision = app
            .core
            .workspace
            .find_session_mut(id)
            .map(|(_, s)| s.on_unexpected_exit());
        if decision == Some(RestartDecision::Resume) {
            if let Ok(rt) = spawn_pty(
                &launch_spec(&cwd, id, LaunchMode::Resume, &app.env_include.vars),
                app.scrollback_lines,
                app.last_grid,
            ) {
                app.terminals.entry(id).or_default().ai_cli = Some(rt);
                // Mark the (possibly background) session Running directly — SessionRunning only
                // reaches the active project. If it was restarted while its project is inactive,
                // record it so the user is notified on return (feature 008, BS-6/BS-7).
                if let Some((_, s)) = app.core.workspace.find_session_mut(id) {
                    s.mark_running();
                }
                app.core.note_background_restart(id);
            }
        }
    }

    // Shell exit (feature 010, FR-013): never auto-restarted, regardless of intentional exit or
    // crash — just mark `Exited` via a direct workspace mutation (mirrors the ai_cli branch's
    // cross-project-safe pattern above) so a backgrounded session's shell exit is reflected even
    // if its project isn't active. No restart decision, no crash-loop counter.
    for id in shell_exited {
        if let Some(st) = app.terminals.get_mut(&id) {
            st.shell = None;
        }
        if let Some((_, s)) = app.core.workspace.find_session_mut(id) {
            s.mark_shell_exited();
        }
    }
}

/// The session's cwd (worktree or project root) + current `TerminalMode`, for a session of the
/// active project (feature 010).
fn session_cwd_and_mode(core: &State, id: SessionId) -> Option<(PathBuf, TerminalMode)> {
    let repo = core.workspace.active.clone()?;
    let session = core.active_sessions().iter().find(|s| s.id == id)?;
    let cwd = session_cwd_for_location(&repo, &session.location);
    Some((cwd, session.mode))
}

/// Ensure the process matching `id`'s current mode is attached/running, spawning it if the
/// corresponding slot is empty (feature 010, FR-003/FR-004/FR-005/FR-011) — reattaches to an
/// already-running process for free (no spawn call at all); only reaches the network/process
/// boundary when there is genuinely nothing to reattach to. Used by `SessionSelected` (initial
/// reopen, mode-aware), `TerminalModeToggled`, and `TerminalRestartRequested`.
fn ensure_attached_process(app: &mut App, id: SessionId) {
    let Some((cwd, mode)) = session_cwd_and_mode(&app.core, id) else {
        return;
    };
    let already_attached = app
        .terminals
        .get(&id)
        .and_then(|st| st.attached(mode))
        .is_some();
    if already_attached {
        return;
    }
    // A failed spawn is reported, never dropped: discarding it here would leave the mode toggle
    // doing nothing at all with no explanation — the same silent failure fixed for
    // `SessionStartRequested` (feature 005 FR-017).
    match mode {
        TerminalMode::AiCli => {
            match spawn_pty(
                &launch_spec(&cwd, id, LaunchMode::Resume, &app.env_include.vars),
                app.scrollback_lines,
                app.last_grid,
            ) {
                Ok(rt) => {
                    app.terminals.entry(id).or_default().ai_cli = Some(rt);
                    app.core.update(Message::SessionRunning(id));
                }
                Err(err) => app
                    .core
                    .notify_error(format!("Could not start the AI CLI: {err}")),
            }
        }
        TerminalMode::Regular => {
            let env = env_include::merge_with_term(&app.env_include.vars);
            match spawn_shell_pty(&cwd, &env, app.scrollback_lines, app.last_grid) {
                Ok(rt) => {
                    app.terminals.entry(id).or_default().shell = Some(rt);
                    app.core.update(Message::ShellSessionRunning(id));
                }
                Err(err) => app
                    .core
                    .notify_error(format!("Could not start the shell: {err}")),
            }
        }
    }
}

/// The session's cwd (worktree or project root) + current lifecycle, for a session in ANY
/// project (feature 008). Unlike [`session_cwd_and_mode`] (active project only), this resolves
/// sessions of inactive projects too, so the crash-loop guard applies to background sessions
/// (BS-6). Cwd site 5/5 (research.md R2).
fn session_cwd_any(core: &State, id: SessionId) -> Option<(PathBuf, SessionLifecycle)> {
    let (project, session) = core.workspace.find_session(id)?;
    let cwd = session_cwd_for_location(project, &session.location);
    Some((cwd, session.lifecycle))
}

/// Build a launch spec for a session in a worktree (claude-cli.md). `resolved_env` is the
/// environment-include snapshot's captured variables (feature 011); merged with the hardcoded
/// `TERM` pair, which always wins on collision (FR-009).
fn launch_spec(
    cwd: &Path,
    id: SessionId,
    mode: LaunchMode,
    resolved_env: &[(String, String)],
) -> LaunchSpec {
    LaunchSpec {
        cwd: cwd.to_path_buf(),
        session_id: id.0,
        mode,
        env: env_include::merge_with_term(resolved_env),
    }
}

/// Discover the active project's worktrees from git + the filesystem (FR-018/018a).
fn discover_worktrees(repo: &Path) -> Vec<Worktree> {
    let git = GitCli::new();
    let porcelain = git.worktree_list_porcelain(repo).unwrap_or_default();
    let records = parse_worktrees(&porcelain);
    let root = repo.join(".claude/worktrees");
    let on_disk = list_dir_names(&root);
    reconcile(&records, &root, &on_disk, &|p| p.exists())
}

/// Create a branch + worktree, removing the target dir if the git step fails (FR-006/006b).
///
/// Progress lines are pushed into `progress` **as they are produced** rather than returned at
/// the end: this runs as one long blocking job (a submodule fetch can take minutes), so a log
/// only readable on completion is a log the user never sees during the wait. The UI drains this
/// buffer on [`CREATE_PROGRESS_POLL`].
fn create(
    repo: &Path,
    names: &micold_ai_ide::naming::DerivedNames,
    progress: &Arc<Mutex<Vec<String>>>,
) -> Result<Worktree, CreateError> {
    let git = GitCli::new();
    let root = repo.join(".claude/worktrees");
    let target = root.join(&names.dir_name);
    let _ = std::fs::create_dir_all(&root);
    let target_exists = target.exists() && dir_nonempty(&target);
    let result = create_worktree(&git, repo, &target, names, target_exists, &mut |line| {
        // A poisoned lock must not abort the create; the log is diagnostic, not load-bearing.
        if let Ok(mut buf) = progress.lock() {
            buf.push(line);
        }
    });
    if result.is_err() {
        // CleanupStep::RemoveDir (the fs half of the rollback plan).
        let _ = std::fs::remove_dir_all(&target);
    }
    result
}

/// Take everything buffered by the in-flight create so far, leaving the buffer empty.
fn drain_create_progress(buffer: &Arc<Mutex<Vec<String>>>) -> Vec<String> {
    buffer
        .lock()
        .map(|mut buf| std::mem::take(&mut *buf))
        .unwrap_or_default()
}

fn describe_create_error(err: CreateError) -> String {
    match err {
        CreateError::DuplicateDir => "A worktree with that name already exists.".to_string(),
        CreateError::DuplicateBranch => "A branch with that name already exists.".to_string(),
        CreateError::RolledBack(msg) => format!("Could not create the worktree: {msg}"),
    }
}

/// Directory names directly under `dir` (empty if it does not exist).
fn list_dir_names(dir: &Path) -> Vec<String> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    entries
        .flatten()
        .filter(|e| e.path().is_dir())
        .filter_map(|e| e.file_name().into_string().ok())
        .collect()
}

fn dir_nonempty(dir: &Path) -> bool {
    std::fs::read_dir(dir)
        .map(|mut it| it.next().is_some())
        .unwrap_or(false)
}

fn map_system_scheme(mode: dark_light::Mode) -> SystemScheme {
    match mode {
        dark_light::Mode::Dark => SystemScheme::Dark,
        dark_light::Mode::Light => SystemScheme::Light,
        dark_light::Mode::Unspecified => SystemScheme::Unspecified,
    }
}

fn detect_system_scheme() -> SystemScheme {
    dark_light::detect()
        .map(map_system_scheme)
        .unwrap_or(SystemScheme::Unspecified)
}

fn os_theme_poll(interval: Duration) -> Subscription<Message> {
    every(interval).map(|_instant| Message::SystemThemeChanged(detect_system_scheme()))
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

    #[test]
    fn terminal_poll_interval_is_none_without_open_terminals() {
        assert_eq!(terminal_poll_interval(false, true), None);
        assert_eq!(terminal_poll_interval(false, false), None);
    }

    #[test]
    fn terminal_poll_interval_coarsens_while_unfocused_but_keeps_polling() {
        assert_eq!(terminal_poll_interval(true, true), Some(TERMINAL_POLL));
        assert_eq!(
            terminal_poll_interval(true, false),
            Some(BACKGROUND_TERMINAL_POLL)
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

    fn dummy_status() -> iced::event::Status {
        iced::event::Status::Ignored
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
            terminals: HashMap::new(),
            scrollback_lines: micold_ai_ide::settings::DEFAULT_SCROLLBACK_LINES,
            motion: Animator::new(),
            main_key: main_content_key(&State::default()),
            handle_hovered: false,
            dismissing: None,
            row_fx: Animator::new(),
            prev_hovered: None,
            window_focused: true,
            create_progress: Arc::new(Mutex::new(Vec::new())),
            last_grid: None,
            env_include_enabled: micold_ai_ide::settings::DEFAULT_ENV_INCLUDE_ENABLED,
            env_include_script_path: String::new(),
            env_include_timeout_secs: micold_ai_ide::settings::DEFAULT_ENV_INCLUDE_TIMEOUT_SECS,
            env_include: EnvIncludeSnapshot {
                vars: Vec::new(),
                outcome: EnvIncludeOutcome::Disabled,
            },
        };

        let _ = update_inner(&mut app, Message::WindowFocusChanged(false));
        assert!(!app.window_focused);

        let _ = update_inner(&mut app, Message::WindowFocusChanged(true));
        assert!(app.window_focused);
    }

    fn test_app() -> App {
        App {
            core: State::default(),
            terminals: HashMap::new(),
            scrollback_lines: micold_ai_ide::settings::DEFAULT_SCROLLBACK_LINES,
            motion: Animator::new(),
            main_key: main_content_key(&State::default()),
            handle_hovered: false,
            dismissing: None,
            row_fx: Animator::new(),
            prev_hovered: None,
            window_focused: true,
            create_progress: Arc::new(Mutex::new(Vec::new())),
            last_grid: None,
            env_include_enabled: micold_ai_ide::settings::DEFAULT_ENV_INCLUDE_ENABLED,
            env_include_script_path: String::new(),
            env_include_timeout_secs: micold_ai_ide::settings::DEFAULT_ENV_INCLUDE_TIMEOUT_SECS,
            env_include: EnvIncludeSnapshot {
                vars: Vec::new(),
                outcome: EnvIncludeOutcome::Disabled,
            },
        }
    }

    /// Draining takes the lines and leaves the buffer empty, so a line is handed to the form
    /// exactly once — re-delivering would duplicate every line on each 150ms tick.
    #[test]
    fn draining_create_progress_takes_each_line_once() {
        let buffer = Arc::new(Mutex::new(vec!["$ git worktree add".to_string()]));

        assert_eq!(drain_create_progress(&buffer), vec!["$ git worktree add"]);
        assert!(drain_create_progress(&buffer).is_empty());

        buffer
            .lock()
            .unwrap()
            .push("Cloning into 'vendor'...".to_string());
        assert_eq!(
            drain_create_progress(&buffer),
            vec!["Cloning into 'vendor'..."]
        );
    }

    /// The poll tick is the producer `WorktreeCreateLogAppended` never had: buffered lines must
    /// reach the form's log *while* the create runs, not only when it finishes.
    #[test]
    fn polling_streams_buffered_progress_into_the_form_log() {
        let mut app = test_app();
        app.core.update(Message::AddWorktreeOpened);
        app.core.update(Message::WorktreeCreateStarted);
        app.create_progress
            .lock()
            .unwrap()
            .push("$ git submodule update --init --recursive".to_string());

        let _ = update_inner(&mut app, Message::WorktreeCreateProgressPolled);

        assert_eq!(
            app.core.worktree_form.as_ref().unwrap().log,
            vec!["$ git submodule update --init --recursive".to_string()],
            "progress must be visible while the create is still running"
        );
    }

    /// A failure's final lines are drained before the result, so the form — which stays open on
    /// failure for diagnosis — keeps them.
    #[test]
    fn completion_drains_the_tail_before_reporting_failure() {
        let mut app = test_app();
        app.core.update(Message::AddWorktreeOpened);
        app.core.update(Message::WorktreeCreateStarted);
        app.create_progress
            .lock()
            .unwrap()
            .push("submodule update failed: network error".to_string());

        let _ = update_inner(
            &mut app,
            Message::WorktreeCreationDone {
                result: Err("boom".to_string()),
            },
        );

        let form = app.core.worktree_form.as_ref().expect("form stays open");
        assert_eq!(form.log, vec!["submodule update failed: network error"]);
    }

    /// The drain tick runs only while a create is in flight — it must not add a background
    /// poll to an idle app.
    #[test]
    fn progress_polling_is_scoped_to_an_in_flight_create() {
        let mut app = test_app();
        assert!(!creating_worktree(&app), "idle app must not poll");

        app.core.update(Message::AddWorktreeOpened);
        assert!(!creating_worktree(&app), "form open but not yet submitted");

        app.core.update(Message::WorktreeCreateStarted);
        assert!(creating_worktree(&app), "create in flight");

        app.core
            .update(Message::WorktreeCreateFailed("boom".to_string()));
        assert!(!creating_worktree(&app), "create finished");
    }

    /// A previous attempt's unread tail must not bleed into the next attempt's log, which
    /// `WorktreeCreateStarted` clears.
    #[test]
    fn a_new_attempt_does_not_inherit_stale_buffered_lines() {
        let app = test_app();
        app.create_progress
            .lock()
            .unwrap()
            .push("stale line from the failed attempt".to_string());

        // What the AddWorktreeSubmitted arm does before spawning the worker.
        drain_create_progress(&app.create_progress);

        assert!(drain_create_progress(&app.create_progress).is_empty());
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
            terminals: HashMap::new(),
            scrollback_lines: micold_ai_ide::settings::DEFAULT_SCROLLBACK_LINES,
            motion: Animator::new(),
            main_key: main_content_key(&State::default()),
            handle_hovered: false,
            dismissing: None,
            row_fx: Animator::new(),
            prev_hovered: None,
            window_focused: true,
            create_progress: Arc::new(Mutex::new(Vec::new())),
            last_grid: None,
            env_include_enabled: micold_ai_ide::settings::DEFAULT_ENV_INCLUDE_ENABLED,
            env_include_script_path: String::new(),
            env_include_timeout_secs: micold_ai_ide::settings::DEFAULT_ENV_INCLUDE_TIMEOUT_SECS,
            env_include: EnvIncludeSnapshot {
                vars: Vec::new(),
                outcome: EnvIncludeOutcome::Disabled,
            },
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
