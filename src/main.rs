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
    Message, Overlay, RenameDraft, SettingsDraft, State, WorktreeForm, WorktreeRenameDraft,
};
use micold_ai_ide::fs_scan::{FolderScanner, StdFolderScanner};
use micold_ai_ide::git::{Git, GitCli};
use micold_ai_ide::motion::Animator;
use micold_ai_ide::provider::{AiCliProvider, ClaudeProvider};
use micold_ai_ide::selector::{Selector, SelectorStatus};
use micold_ai_ide::session::{
    RestartDecision, Session, SessionId, SessionLabel, SessionLifecycle, SessionLocation,
};
use micold_ai_ide::settings::{JsonFileSettingsStore, Settings, SettingsStore};
use micold_ai_ide::store::{JsonFileStore, ProjectStore};
use micold_ai_ide::terminal::{LaunchMode, LaunchSpec};
use micold_ai_ide::theme::SystemScheme;
use micold_ai_ide::worktree::{
    create_worktree, parse_worktrees, reconcile, remove_worktree, CreateError, Worktree,
};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::Duration;
use ui::terminal::{spawn_pty, RuntimeTerminal};
use ui::MotionKey;

/// How often the OS light/dark preference is polled (research R4).
const OS_THEME_POLL: Duration = Duration::from_millis(500);

/// How often live terminals are polled for streamed output + process-exit detection.
const TERMINAL_POLL: Duration = Duration::from_millis(120);

/// How often live terminals are polled while the window is unfocused (idle-CPU fix). Coarser
/// than [`TERMINAL_POLL`] rather than fully suspended: a fully-suspended poll would also stall
/// crash detection/auto-restart and title-sync indefinitely, and let `RuntimeTerminal`'s PTY
/// output buffer grow unbounded for as long as the window stays backgrounded (code review
/// findings). This still cuts the tick rate — and its full-`view()` rebuild cost — by ~17x
/// while backgrounded.
const BACKGROUND_TERMINAL_POLL: Duration = Duration::from_secs(2);

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
                    &launch_spec(&cwd, id, LaunchMode::Fresh),
                    app.scrollback_lines,
                ) {
                    Ok(rt) => {
                        app.terminals.insert(id, rt);
                        app.core.update(Message::SessionStarted(session));
                        app.core.update(Message::SessionRunning(id));
                        persist(&app.core);
                        started = true;
                    }
                    Err(err) => {
                        app.core.worktree_error = Some(format!("Could not start session: {err}"));
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
            // BUG-001: auto-focus the selected session's terminal (FR-010/FR-010a). Selecting from
            // the sidebar is a click *outside* the pane, so a currently-focused pane also publishes
            // `TerminalFocusReleased` for the same click. Re-assert focus via a follow-up message,
            // which is delivered *after* the current event batch drains — so the focus wins
            // regardless of the intra-batch order of `SessionSelected` vs `TerminalFocusReleased`.
            Task::done(Message::TerminalFocused)
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
        // Scroll to an absolute offset (scrollbar drag). Resolve the delta against the LIVE offset
        // here at apply time so a burst of batched drag messages converges instead of accumulating
        // stale relative deltas (drag flicker fix, FR-016).
        Message::TerminalScrolledTo(target) => {
            if let Some(rt) = app
                .core
                .active_session
                .and_then(|id| app.terminals.get_mut(&id))
            {
                rt.scroll(ui::target_offset_delta(rt.display_offset(), target));
            }
            Task::none()
        }
        // Copy the current selection to the system clipboard (FR-013). Also closes the menu.
        Message::TerminalCopyRequested => {
            app.core.update(Message::TerminalContextMenuClosed);
            let content = app
                .core
                .active_session
                .and_then(|id| app.terminals.get(&id))
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
            sync_session_titles(app);
            handle_process_exits(app);
            Task::none()
        }
        Message::WindowFocusChanged(focused) => {
            app.window_focused = focused;
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
                // Terminate this worktree's running sessions first.
                for id in app.core.sessions_in_worktree(&dir) {
                    if let Some(mut rt) = app.terminals.remove(&id) {
                        let _ = rt.kill();
                    }
                }
                if let Some(wt) = wt {
                    let _ = remove_worktree(&GitCli::new(), &repo, &wt.path, wt.branch.as_deref());
                    let _ = std::fs::remove_dir_all(&wt.path);
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
    // Supply the active session's live terminal runtime to the colour-rendering pane (feature 006).
    let terminal = app
        .core
        .active_session
        .and_then(|id| app.terminals.get(&id));
    ui::view(
        &app.core,
        terminal,
        &app.motion,
        app.dismissing.as_ref(),
        &app.row_fx,
    )
}

fn theme(app: &App) -> iced::Theme {
    ui::style::theme(app.core.color_scheme())
}

fn subscription(app: &App) -> Subscription<Message> {
    // Event-driven (not a poll): reports actual OS focus changes, so it costs nothing while
    // the window sits idle either focused or not (idle-CPU fix).
    let mut subs = vec![ui::subscription(&app.core), window_focus_events()];
    // The OS theme poll only matters while the window is visible to look at.
    if app.window_focused {
        subs.push(os_theme_poll());
    }
    if let Some(interval) = terminal_poll_interval(!app.terminals.is_empty(), app.window_focused) {
        subs.push(every(interval).map(|_| Message::TerminalTick));
    }
    // Run the animation clock only while something is actually animating (FR-014).
    if motion_animating(app) {
        subs.push(every(ANIM_TICK).map(|_| Message::AnimationTick));
    }
    Subscription::batch(subs)
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
    let mut exited: Vec<SessionId> = Vec::new();
    for (id, rt) in app.terminals.iter_mut() {
        if rt.has_exited() {
            exited.push(*id);
        }
    }

    for id in exited {
        app.terminals.remove(&id);
        // Resolve the exited session in ANY project — a background session of an inactive
        // project must still be handled, not silently dropped (feature 008, BS-6).
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
                &launch_spec(&cwd, id, LaunchMode::Resume),
                app.scrollback_lines,
            ) {
                app.terminals.insert(id, rt);
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
}

/// The session's cwd (worktree or project root) + current lifecycle, for a session of the
/// active project. Cwd site 4/5 (research.md R2).
fn session_cwd(core: &State, id: SessionId) -> Option<(PathBuf, SessionLifecycle)> {
    let repo = core.workspace.active.clone()?;
    let session = core.active_sessions().iter().find(|s| s.id == id)?;
    let cwd = session_cwd_for_location(&repo, &session.location);
    Some((cwd, session.lifecycle))
}

/// The session's cwd (worktree or project root) + current lifecycle, for a session in ANY
/// project (feature 008). Unlike [`session_cwd`], this resolves sessions of inactive projects
/// too, so the crash-loop guard applies to background sessions (BS-6). Cwd site 5/5
/// (research.md R2).
fn session_cwd_any(core: &State, id: SessionId) -> Option<(PathBuf, SessionLifecycle)> {
    let (project, session) = core.workspace.find_session(id)?;
    let cwd = session_cwd_for_location(project, &session.location);
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
        };

        let _ = update_inner(&mut app, Message::WindowFocusChanged(false));
        assert!(!app.window_focused);

        let _ = update_inner(&mut app, Message::WindowFocusChanged(true));
        assert!(app.window_focused);
    }
}
