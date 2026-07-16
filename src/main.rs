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
use micold_ai_ide::app::{Message, Overlay, State};
use micold_ai_ide::fs_scan::{FolderScanner, StdFolderScanner};
use micold_ai_ide::git::{Git, GitCli};
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

/// How often the OS light/dark preference is polled (research R4).
const OS_THEME_POLL: Duration = Duration::from_millis(500);

/// How often live terminals are polled for streamed output + process-exit detection.
const TERMINAL_POLL: Duration = Duration::from_millis(120);

/// The animation clock tick interval (~60fps).
const ANIM_TICK: Duration = Duration::from_millis(16);
/// Per-tick progress step for fades (~90ms) and the sidebar slide (~120ms).
const FADE_STEP: f32 = 0.18;
const SLIDE_STEP: f32 = 0.14;

/// The binary's application state: the pure core plus gui-only runtime handles.
struct App {
    core: State,
    /// Live PTY sessions, keyed by session id (never part of the pure core — not Clone/Eq).
    terminals: HashMap<SessionId, RuntimeTerminal>,
    /// Overflow-menu fade progress (0=hidden, 1=shown).
    menu_anim: f32,
    /// Sidebar slide progress (0=collapsed, 1=expanded).
    sidebar_anim: f32,
    /// Main-view fade progress (reset to 0 to fade in when the content changes).
    main_anim: f32,
    /// Identity of the current main content, to detect changes that trigger a fade.
    main_key: String,
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

/// Move `current` toward `target` by at most `step`.
fn approach(current: f32, target: f32, step: f32) -> f32 {
    if current < target {
        (current + step).min(target)
    } else {
        (current - step).max(target)
    }
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

pub fn main() -> iced::Result {
    iced::application("Micold AI IDE", update, view)
        .theme(theme)
        .default_font(iced::Font::DEFAULT)
        .font(ui::MATERIAL_SYMBOLS_BYTES)
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
    if let Some(store) = JsonFileSettingsStore::default_location() {
        core.theme_pref = store.load().settings.theme;
    }
    core.system_scheme = detect_system_scheme();
    // If a project is already active from a previous run, discover its worktrees.
    if let Some(repo) = core.workspace.active.clone() {
        core.worktrees = discover_worktrees(&repo);
    }
    let sidebar_anim = if core.sidebar_hidden { 0.0 } else { 1.0 };
    let main_key = main_content_key(&core);
    (
        App {
            core,
            terminals: HashMap::new(),
            menu_anim: 0.0,
            sidebar_anim,
            main_anim: 1.0,
            main_key,
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
        let _ = store.save(&Settings {
            theme: core.theme_pref,
        });
    }
}

fn update(app: &mut App, message: Message) -> Task<Message> {
    let task = update_inner(app, message);
    // Trigger a main-view fade-in whenever the displayed content changes.
    let key = main_content_key(&app.core);
    if key != app.main_key {
        app.main_key = key;
        app.main_anim = 0.0;
    }
    task
}

fn update_inner(app: &mut App, message: Message) -> Task<Message> {
    match message {
        // Advance the animation clock toward each target.
        Message::AnimationTick => {
            let menu_target = if app.core.help_menu_open { 1.0 } else { 0.0 };
            let sidebar_target = if app.core.sidebar_hidden { 0.0 } else { 1.0 };
            app.menu_anim = approach(app.menu_anim, menu_target, FADE_STEP);
            app.sidebar_anim = approach(app.sidebar_anim, sidebar_target, SLIDE_STEP);
            app.main_anim = approach(app.main_anim, 1.0, FADE_STEP);
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
                match spawn_pty(&launch_spec(&cwd, id, LaunchMode::Fresh)) {
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
                    if let Ok(rt) = spawn_pty(&launch_spec(&cwd, id, LaunchMode::Resume)) {
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
        // Send the input line to the active session's PTY (FR-014).
        Message::TerminalLineSubmitted => {
            if let Some(id) = app.core.active_session {
                if let Some(rt) = app.terminals.get_mut(&id) {
                    let mut line = app.core.terminal_input.clone();
                    line.push('\n');
                    let _ = rt.write(line.as_bytes());
                }
            }
            app.core.update(Message::TerminalLineSubmitted);
            // The user interacted, so `claude` now has a conversation — persist it so it can
            // be resumed after a restart (FR-020; empty sessions are still pruned by persist).
            persist(&app.core);
            Task::none()
        }
        // Poll terminals: feed streamed bytes into the VT emulators, then detect unexpected
        // exits and apply the crash-restart policy (FR-012, FR-022).
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
    // Supply the active session's interpreted terminal screen to the pane.
    let output = app
        .core
        .active_session
        .and_then(|id| app.terminals.get(&id))
        .map(|rt| rt.screen_text());
    let anim = ui::Anim {
        menu: app.menu_anim,
        sidebar: app.sidebar_anim,
        main: app.main_anim,
    };
    ui::view(&app.core, output.as_deref(), anim)
}

fn theme(app: &App) -> iced::Theme {
    ui::style::theme(app.core.color_scheme())
}

fn subscription(app: &App) -> Subscription<Message> {
    let mut subs = vec![ui::subscription(&app.core), os_theme_poll()];
    if !app.terminals.is_empty() {
        subs.push(every(TERMINAL_POLL).map(|_| Message::TerminalTick));
    }
    // Run the animation clock only while something is actually animating.
    let menu_target = if app.core.help_menu_open { 1.0 } else { 0.0 };
    let sidebar_target = if app.core.sidebar_hidden { 0.0 } else { 1.0 };
    let animating = (app.menu_anim - menu_target).abs() > f32::EPSILON
        || (app.sidebar_anim - sidebar_target).abs() > f32::EPSILON
        || app.main_anim < 1.0;
    if animating {
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
            if let Ok(rt) = spawn_pty(&launch_spec(&cwd, id, LaunchMode::Resume)) {
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
