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
use micold_ai_ide::app::{Message, Overlay, RenameDraft, SettingsDraft, State, WorktreeForm};
use micold_ai_ide::fs_scan::{FolderScanner, StdFolderScanner};
use micold_ai_ide::git::{Git, GitCli};
use micold_ai_ide::motion::Animator;
use micold_ai_ide::selector::{Selector, SelectorStatus};
use micold_ai_ide::session::{RestartDecision, Session, SessionId, SessionLifecycle};
use micold_ai_ide::settings::{JsonFileSettingsStore, Settings, SettingsStore};
use micold_ai_ide::store::{JsonFileStore, ProjectStore};
use micold_ai_ide::terminal::{LaunchMode, LaunchSpec};
use micold_ai_ide::theme::SystemScheme;
use micold_ai_ide::worktree::{create_worktree, parse_worktrees, reconcile, CreateError, Worktree};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::Duration;
use ui::terminal::{spawn_pty, RuntimeTerminal};
use ui::MotionKey;

/// How often the OS light/dark preference is polled (research R4).
const OS_THEME_POLL: Duration = Duration::from_millis(500);

/// How often live terminals are polled for streamed output + process-exit detection.
const TERMINAL_POLL: Duration = Duration::from_millis(120);

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
}

/// The binary's application state: the pure core plus gui-only runtime handles.
struct App {
    core: State,
    /// Live PTY sessions, keyed by session id (never part of the pure core — not Clone/Eq).
    terminals: HashMap<SessionId, RuntimeTerminal>,
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
fn motion_targets(app: &App) -> [(MotionKey, f32, Duration); 4] {
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
    steady || overlay
}

impl App {
    /// Stop the outgoing project's sessions on close/switch (FR-023): kill their processes and
    /// mark the persisted records `Idle` so reopening the project can resume them (FR-023a).
    fn stop_active_project_sessions(&mut self) {
        for (_, mut rt) in self.terminals.drain() {
            let _ = rt.kill();
        }
        if let Some(path) = self.core.workspace.active.clone() {
            if let Some(list) = self.core.workspace.sessions.get_mut(&path) {
                for session in list {
                    session.stop_for_project_change();
                }
            }
        }
        self.core.active_session = None;
    }
}

impl Drop for App {
    /// Kill every child `claude` process on shutdown so none are orphaned (research R5, T057).
    fn drop(&mut self) {
        for rt in self.terminals.values_mut() {
            let _ = rt.kill();
        }
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
    if let Some(store) = JsonFileSettingsStore::default_location() {
        let loaded = store.load().settings;
        core.theme_pref = loaded.theme;
        scrollback_lines = loaded.scrollback_lines;
    }
    core.system_scheme = detect_system_scheme();
    // If a project is already active from a previous run, discover its worktrees.
    if let Some(repo) = core.workspace.active.clone() {
        core.worktrees = discover_worktrees(&repo);
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

/// Whether `claude` has recorded a conversation transcript for this session (research R6).
/// The transcript lives at `<claude>/projects/<encoded-cwd>/<session-id>.jsonl`, where
/// `<encoded-cwd>` is the worktree path with every non-alphanumeric char replaced by `-`.
fn session_has_conversation(
    project_path: &Path,
    session: &micold_ai_ide::session::Session,
) -> bool {
    let cwd = project_path
        .join(".claude/worktrees")
        .join(&session.worktree_dir);
    let Some(base) = claude_config_dir() else {
        // Cannot determine the claude dir — do not drop the session on uncertainty.
        return true;
    };
    let encoded: String = cwd
        .to_string_lossy()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect();
    base.join("projects")
        .join(encoded)
        .join(format!("{}.jsonl", session.id))
        .exists()
}

/// The `claude` config directory: `$CLAUDE_CONFIG_DIR` or `~/.claude`.
fn claude_config_dir() -> Option<PathBuf> {
    if let Ok(dir) = std::env::var("CLAUDE_CONFIG_DIR") {
        if !dir.is_empty() {
            return Some(PathBuf::from(dir));
        }
    }
    directories::UserDirs::new().map(|d| d.home_dir().join(".claude"))
}

fn persist_settings(core: &State) {
    if let Some(store) = JsonFileSettingsStore::default_location() {
        // Preserve the persisted scrollback limit (feature 006) when saving a theme change.
        let scrollback_lines = store.load().settings.scrollback_lines;
        let _ = store.save(&Settings {
            theme: core.theme_pref,
            scrollback_lines,
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
    }
}

fn update_inner(app: &mut App, message: Message) -> Task<Message> {
    match message {
        // Advance every animation toward its target via the shared driver.
        Message::AnimationTick => {
            apply_motion_targets(app);
            app.motion.tick();
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
            app.core.overlay = Overlay::ProjectSelector;
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
            app.core.selector = None;
            app.core.overlay = Overlay::None;
            if !GitCli::new().is_repo_root(&path) {
                app.core.update(Message::ProjectOpenRefused(
                    "Only git repositories can be opened as projects.".to_string(),
                ));
                return Task::none();
            }
            // Stop the outgoing project's sessions before switching (FR-023).
            app.stop_active_project_sessions();
            app.core
                .workspace
                .open_or_activate(path.clone(), &StdFolderScanner::new());
            app.core.worktrees = discover_worktrees(&path);
            app.core.worktree_error = None;
            persist(&app.core);
            Task::none()
        }
        Message::KnownProjectReopened(path) => {
            app.core
                .workspace
                .refresh_availability(&StdFolderScanner::new());
            // Stop the outgoing project's sessions before switching (FR-023).
            app.stop_active_project_sessions();
            if app.core.workspace.activate(&path) {
                app.core.worktrees = discover_worktrees(&path);
                persist(&app.core);
            }
            Task::none()
        }
        Message::RenameConfirmed => {
            app.core.update(Message::RenameConfirmed);
            persist(&app.core);
            Task::none()
        }
        Message::ThemePreferenceChanged(_) | Message::ThemeModeCycled => {
            app.core.update(message);
            persist_settings(&app.core);
            Task::none()
        }
        // Validate the form, then create the worktree via git (FR-006/006b).
        Message::AddWorktreeSubmitted => {
            app.core.update(Message::AddWorktreeSubmitted);
            let Some(form) = app.core.worktree_form.clone() else {
                return Task::none();
            };
            let Ok(names) = form.preview() else {
                return Task::none(); // validation error already recorded by the reducer
            };
            let Some(repo) = app.core.workspace.active.clone() else {
                return Task::none();
            };
            match create(&repo, &names) {
                Ok(worktree) => app.core.update(Message::WorktreeCreated(worktree)),
                Err(err) => app
                    .core
                    .update(Message::WorktreeCreateFailed(describe_create_error(err))),
            }
            Task::none()
        }
        // Start a new session on a worktree: spawn `claude` and stream it (FR-010/012/013).
        Message::SessionStartRequested { worktree_dir } => {
            if let Some(repo) = app.core.workspace.active.clone() {
                let cwd = repo.join(".claude/worktrees").join(&worktree_dir);
                let session = Session::start_new(&worktree_dir);
                let id = session.id;
                match spawn_pty(
                    &launch_spec(&cwd, id, LaunchMode::Fresh),
                    app.scrollback_lines,
                ) {
                    Ok(rt) => {
                        app.terminals.insert(id, rt);
                        app.core.update(Message::SessionStarted(session));
                        app.core.update(Message::SessionRunning(id));
                        persist(&app.core);
                    }
                    Err(err) => {
                        app.core.worktree_error = Some(format!("Could not start session: {err}"));
                    }
                }
            }
            Task::none()
        }
        // Selecting an Idle (restored) session resumes it via `claude --resume` (FR-023a).
        Message::SessionSelected(id) => {
            app.core.update(Message::SessionSelected(id));
            if !app.terminals.contains_key(&id) {
                if let Some((cwd, _)) = session_cwd(&app.core, id) {
                    if let Ok(rt) = spawn_pty(
                        &launch_spec(&cwd, id, LaunchMode::Resume),
                        app.scrollback_lines,
                    ) {
                        app.terminals.insert(id, rt);
                        app.core.update(Message::SessionRunning(id));
                    }
                }
            }
            Task::none()
        }
        // Close a session: kill its process and drop the runtime handle (FR-015a).
        Message::SessionCloseRequested(id) => {
            if let Some(mut rt) = app.terminals.remove(&id) {
                let _ = rt.kill();
            }
            app.core.update(Message::SessionCloseRequested(id));
            persist(&app.core);
            Task::none()
        }
        // Stream live keystrokes/paste to the displayed session's PTY (FR-008), but only while
        // it is Running (FR-012a): input to a non-running process is discarded, not buffered.
        Message::TerminalBytes(bytes) => {
            if let Some(id) = app.core.active_session {
                let running = app
                    .core
                    .active_sessions()
                    .iter()
                    .find(|s| s.id == id)
                    .map(|s| micold_ai_ide::app::should_write_to(s.lifecycle))
                    .unwrap_or(false);
                if running {
                    if let Some(rt) = app.terminals.get_mut(&id) {
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
            if let Some(rt) = app
                .core
                .active_session
                .and_then(|id| app.terminals.get_mut(&id))
            {
                rt.selection_start(col, line, kind);
            }
            Task::none()
        }
        Message::TerminalSelectUpdate { col, line } => {
            if let Some(rt) = app
                .core
                .active_session
                .and_then(|id| app.terminals.get_mut(&id))
            {
                rt.selection_update(col, line);
            }
            Task::none()
        }
        Message::TerminalSelectCleared => {
            if let Some(rt) = app
                .core
                .active_session
                .and_then(|id| app.terminals.get_mut(&id))
            {
                rt.selection_clear();
            }
            Task::none()
        }
        // Reflow the displayed session's PTY + grid to the visible size (FR-014/FR-015).
        Message::TerminalResized { cols, rows } => {
            if let Some(rt) = app
                .core
                .active_session
                .and_then(|id| app.terminals.get_mut(&id))
            {
                let _ = rt.resize(cols, rows);
            }
            Task::none()
        }
        // Scroll the displayed session's local scrollback (FR-016).
        Message::TerminalScrolled(delta) => {
            if let Some(rt) = app
                .core
                .active_session
                .and_then(|id| app.terminals.get_mut(&id))
            {
                rt.scroll(delta);
            }
            Task::none()
        }
        // Poll terminals: feed streamed bytes into the VT emulators, then detect unexpected
        // exits and apply the crash-restart policy (FR-012, FR-022).
        // Open Settings: let the reducer show the overlay, then seed the draft with the current
        // scrollback value (FR-019/FR-020).
        Message::SettingsOpened => {
            app.core.update(Message::SettingsOpened);
            if let Some(draft) = app.core.settings_draft.as_mut() {
                draft.scrollback_lines = app.scrollback_lines.to_string();
            }
            Task::none()
        }
        // Save Settings: validate the scrollback field; on success persist + apply and close, on
        // failure keep the form open with an error (FR-020/FR-021).
        Message::SettingsSaved => {
            let parsed = app
                .core
                .settings_draft
                .as_ref()
                .and_then(|d| d.scrollback_lines.trim().parse::<usize>().ok());
            let min = micold_ai_ide::settings::MIN_SCROLLBACK_LINES;
            let max = micold_ai_ide::settings::MAX_SCROLLBACK_LINES;
            match parsed {
                Some(n) if (min..=max).contains(&n) => {
                    app.scrollback_lines = n;
                    if let Some(store) = JsonFileSettingsStore::default_location() {
                        let _ = store.save(&Settings {
                            theme: app.core.theme_pref,
                            scrollback_lines: n,
                        });
                    }
                    app.core.update(Message::SettingsSaved); // closes the overlay
                }
                Some(_) => {
                    if let Some(draft) = app.core.settings_draft.as_mut() {
                        draft.error = Some(format!("Enter a number between {min} and {max}."));
                    }
                }
                None => {
                    if let Some(draft) = app.core.settings_draft.as_mut() {
                        draft.error = Some("Enter a whole number of lines.".to_string());
                    }
                }
            }
            Task::none()
        }
        Message::TerminalTick => {
            for rt in app.terminals.values_mut() {
                rt.pump();
            }
            handle_process_exits(app);
            Task::none()
        }
        other => {
            app.core.update(other);
            Task::none()
        }
    }
}

fn view(app: &App) -> iced::Element<'_, Message> {
    // Supply the active session's live terminal runtime to the colour-rendering pane (feature 006).
    let terminal = app
        .core
        .active_session
        .and_then(|id| app.terminals.get(&id));
    ui::view(&app.core, terminal, &app.motion, app.dismissing.as_ref())
}

fn theme(app: &App) -> iced::Theme {
    ui::style::theme(app.core.color_scheme())
}

fn subscription(app: &App) -> Subscription<Message> {
    let mut subs = vec![ui::subscription(&app.core), os_theme_poll()];
    if !app.terminals.is_empty() {
        subs.push(every(TERMINAL_POLL).map(|_| Message::TerminalTick));
    }
    // Run the animation clock only while something is actually animating (FR-014).
    if motion_animating(app) {
        subs.push(every(ANIM_TICK).map(|_| Message::AnimationTick));
    }
    Subscription::batch(subs)
}

/// Detect processes that exited unexpectedly and apply the crash-loop guard (FR-022/022a).
fn handle_process_exits(app: &mut App) {
    let mut exited: Vec<SessionId> = Vec::new();
    for (id, rt) in app.terminals.iter_mut() {
        if rt.has_exited() {
            exited.push(*id);
        }
    }

    for id in exited {
        app.terminals.remove(&id);
        let Some((cwd, lifecycle)) = session_cwd(&app.core, id) else {
            continue;
        };
        // An intentional stop (Idle) is not a crash — do not auto-restart.
        if lifecycle == SessionLifecycle::Idle {
            continue;
        }
        let decision = with_session(&mut app.core, id, |s| s.on_unexpected_exit());
        if decision == Some(RestartDecision::Resume) {
            if let Ok(rt) = spawn_pty(
                &launch_spec(&cwd, id, LaunchMode::Resume),
                app.scrollback_lines,
            ) {
                app.terminals.insert(id, rt);
                app.core.update(Message::SessionRunning(id));
            }
        }
    }
}

/// Run `f` against the active project's session `id`, returning its result.
fn with_session<R>(
    core: &mut State,
    id: SessionId,
    f: impl FnOnce(&mut Session) -> R,
) -> Option<R> {
    let path = core.workspace.active.clone()?;
    core.workspace
        .sessions
        .get_mut(&path)?
        .iter_mut()
        .find(|s| s.id == id)
        .map(f)
}

/// The worktree cwd + current lifecycle for a session of the active project.
fn session_cwd(core: &State, id: SessionId) -> Option<(PathBuf, SessionLifecycle)> {
    let repo = core.workspace.active.clone()?;
    let session = core.active_sessions().iter().find(|s| s.id == id)?;
    let cwd = repo.join(".claude/worktrees").join(&session.worktree_dir);
    Some((cwd, session.lifecycle))
}

/// Build a launch spec for a session in a worktree (claude-cli.md).
fn launch_spec(cwd: &Path, id: SessionId, mode: LaunchMode) -> LaunchSpec {
    LaunchSpec {
        cwd: cwd.to_path_buf(),
        session_id: id.0,
        mode,
        env: vec![("TERM".to_string(), "xterm-256color".to_string())],
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
fn create(
    repo: &Path,
    names: &micold_ai_ide::naming::DerivedNames,
) -> Result<Worktree, CreateError> {
    let git = GitCli::new();
    let root = repo.join(".claude/worktrees");
    let target = root.join(&names.dir_name);
    let _ = std::fs::create_dir_all(&root);
    let target_exists = target.exists() && dir_nonempty(&target);
    let result = create_worktree(&git, repo, &target, names, target_exists);
    if result.is_err() {
        // CleanupStep::RemoveDir (the fs half of the rollback plan).
        let _ = std::fs::remove_dir_all(&target);
    }
    result
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

fn os_theme_poll() -> Subscription<Message> {
    every(OS_THEME_POLL).map(|_instant| Message::SystemThemeChanged(detect_system_scheme()))
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
}
