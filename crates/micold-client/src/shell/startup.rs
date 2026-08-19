//! Starting the process: the window, the runtime, and the first `App` (feature 021, T050 —
//! FR-019a).
//!
//! FR-019a asks the shell to be divided by the **external system each part addresses**. This part
//! addresses the *window system and the iced runtime*: it registers fonts, sets the app icon and
//! the Linux `WM_CLASS`, hands iced its three functions, and assembles the first `App` before any
//! of that is on screen.
//!
//! # `main` stayed behind, and only `main`
//!
//! Rust requires the entry point at the binary crate root, so `main.rs` keeps a two-line
//! `fn main()` that calls [`run`]. Everything the task named as `main` — the probe notice, the
//! font registration, the window settings, the runtime handshake — is here; what is left there is
//! the signature Rust insists on.
//!
//! # Nothing was relocated alongside, because there was nothing to relocate
//!
//! T050 names FR-027's relocation clause, which exists so a moved function's inline tests move
//! with it rather than being dropped. `boot`, `window_settings` and `main` had no inline tests:
//! `boot` reads the user's real data directory and opens a window, which is why. What made it
//! testable is [`Capabilities`], and the first tests of anything `boot` does landed at T049
//! against the provider port.
//!
//! # What still lives in `main.rs` that this calls
//!
//! `boot` composes six functions that address other systems — persistence, the environment-include
//! subprocess, git discovery, the OS theme — and those move in T051–T054. Until then it reaches
//! back up to the crate root for them, which is why the imports below are `crate::`.

use std::cell::RefCell;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::AtomicUsize;
use std::sync::Arc;

use iced::Task;
use micold_client::app::{Message, State};
use micold_client::input::SessionInputStamper;
use micold_core::sandbox::placement::PlacementKind;
use micold_core::settings::Settings;

use crate::shell::capabilities::Capabilities;
use crate::shell::env_include::{default_resolution_cwd, resolve_env_include};
use crate::shell::os_theme::detect_system_scheme;
use crate::shell::persist::prune_empty_sessions;
use crate::shell::workspace::discover_worktrees;
use crate::{observe_system_scheme, probe_config, theme, update, view, App};

/// The app window icon as raw 64x64 RGBA (generated from `assets/icon/icon.svg` by
/// `assets/icon/generate.py`). Embedded directly so no runtime image decoder is needed.
const ICON_RGBA: &[u8] = include_bytes!("../../../../assets/icon/icon-64.rgba");

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

pub fn run() -> iced::Result {
    // Resolve the measurement run before opening a window, so a malformed `MICOLD_FRAME_PROBE`
    // is refused at the terminal the operator is looking at rather than after the UI is up.
    if let Some(config) = probe_config() {
        eprintln!(
            "frame probe: measuring {} frames after {} warm-up, then exiting.",
            config.frames, config.warm_up
        );
    }
    iced::application(boot, update, view)
        .title("Micold AI IDE")
        .theme(theme)
        // Roboto is the application's typeface, so the interface looks the same on every platform
        // instead of inheriting whatever UI font the OS provides (FR-008). Both weights are
        // registered because the type scale uses 400 and 500 and the matcher can only choose from
        // what it has been given.
        .default_font(micold_client::ui::ROBOTO)
        .font(micold_client::ui::ROBOTO_REGULAR_BYTES)
        .font(micold_client::ui::ROBOTO_MEDIUM_BYTES)
        // Registering fonts does not disable fallback: text outside a registered font's coverage —
        // a worktree named in Japanese, say — still resolves through the system's own font list
        // rather than rendering missing-glyph boxes (FR-013). Roboto covers Latin and the symbols
        // the interface composes its own strings from; user data is not so constrained.
        .font(micold_client::ui::MATERIAL_SYMBOLS_BYTES)
        .window(window_settings())
        .subscription(crate::shell::subscriptions::subscription)
        .run()
}

fn boot() -> (App, Task<Message>) {
    // The single assembly point (FR-018). Everything below takes what it needs from `caps`.
    let caps = Capabilities::real();
    let mut core = State::default();
    if let Some(store) = caps.projects() {
        core.workspace = store.load().workspace;
        core.workspace.refresh_availability(caps.scanner());
        // Drop any leftover empty sessions so a restart never resumes a nonexistent
        // conversation (bug fix; see spec Clarifications 2026-07-16).
        prune_empty_sessions(caps.provider(), &mut core.workspace);
    }
    let mut scrollback_lines = micold_core::settings::DEFAULT_SCROLLBACK_LINES;
    let mut env_include_enabled = micold_core::settings::DEFAULT_ENV_INCLUDE_ENABLED;
    let mut env_include_script_path = Settings::default().env_include_script_path;
    let mut env_include_timeout_secs = micold_core::settings::DEFAULT_ENV_INCLUDE_TIMEOUT_SECS;
    if let Some(store) = caps.settings() {
        let loaded = store.load().settings;
        core.theme_pref = loaded.theme;
        scrollback_lines = loaded.scrollback_lines;
        env_include_enabled = loaded.env_include_enabled;
        env_include_script_path = loaded.env_include_script_path;
        env_include_timeout_secs = loaded.env_include_timeout_secs;
    }
    // Feature 027: where the daemon runs. Read here rather than in the connection subscription
    // because the *bring-up* is what the user watches, and it starts before the first dial.
    let placement = caps
        .settings()
        .map(|store| store.load().settings.daemon.placement)
        .unwrap_or_default();
    let sandbox_state = micold_client::features::sandbox::Sandbox::for_placement(placement);
    let sandbox_profile = caps
        .settings()
        .map(|store| store.load().settings.daemon.sandbox)
        .unwrap_or_default();
    let state_dir = directories::ProjectDirs::from("", "", "micold-ai-ide")
        .map(|d| d.data_dir().to_path_buf())
        .unwrap_or_default();
    let resolved_placement = micold_client::daemon::Placement {
        kind: placement,
        state_dir: state_dir.clone(),
        strict_fingerprint: sandbox_profile.image.refuses_fingerprint_mismatch(),
    };
    // The mount set is the registered projects, read from the store this boot already loaded.
    let projects_for_sandbox: Vec<PathBuf> = core
        .workspace
        .projects
        .iter()
        .map(|p| p.path.clone())
        .collect();

    let boot_cwd = default_resolution_cwd(&core);
    let boot_snapshot = resolve_env_include(
        caps.env_include(),
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
        core.set_worktrees(discover_worktrees(caps.git(), &repo));
        // Feature 025: land on the session this project was last showing, rather than on the
        // project overview. The memory came from the store above, beside that project's sessions.
        //
        // The same function a project switch calls, deliberately. It resolves the memory, applies
        // it, and gives the terminal the keyboard — all three wanted here — and everything else it
        // does is already true at boot: `default_expanded` and `show_agent_worktrees` are their
        // defaults, and `arm_notice` finds nothing because no session has restarted yet. A
        // launch-only path would be a second implementation of a sequence the switch already
        // performs, and the two would drift (research R5).
        //
        // Ordering: after `prune_empty_sessions` above, so a memory naming a session with no
        // conversation on disk resolves to nothing rather than to a session about to be dropped.
        core.restore_after_activation(&repo);
    }
    (
        App {
            core,
            caps,
            grids: HashMap::new(),
            stamper: SessionInputStamper::new(),
            selection: None,
            display_offset: 0,
            scrollback_lines,
            dismissing: None,
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
            placement: resolved_placement,
            sandbox: sandbox_state,
            version_mismatch: None,
            build_mismatch: None,
            next_req: 0,
            pending_ops: HashMap::new(),
            probe: probe_config().map(|config| RefCell::new(config.probe())),
            scene_ready: false,
            scene_frames: 0,
            ripples_animating: Arc::new(AtomicUsize::new(0)),
            scene_ripple_frames: std::cell::Cell::new(0),
        },
        Task::batch([
            // Ask for the initial window size up front: `resize_events` only fires on *changes*, so
            // without this the first context menu before any resize would have nothing to clamp
            // against (feature 015).
            iced::window::latest()
                .and_then(iced::window::size)
                .map(|size| Message::WindowResized {
                    width: size.width.max(0.0) as u16,
                    height: size.height.max(0.0) as u16,
                }),
            // And, when the daemon is sandboxed, start bringing it up. Batched rather than
            // sequenced: the window has no reason to wait on a container image.
            sandbox_boot(placement, sandbox_profile, state_dir, projects_for_sandbox),
        ]),
    )
}

/// Start the sandbox, if that is where the daemon lives (feature 027).
///
/// Runs on a blocking thread, because every step of it shells out to a container runtime and the
/// image acquisition can take minutes — on the render thread that would freeze the window for the
/// whole of it, which is the opposite of SC-004's "continuous progress".
///
/// The projects come from the host's own `projects.json`, read a moment ago at boot. That is why
/// the daemon's state directory is bind-mounted rather than held in a runtime-managed volume: the
/// mount set has to be known *before* the sandbox exists, and only the daemon owns the project
/// list.
fn sandbox_boot(
    placement: PlacementKind,
    profile: micold_core::sandbox::SandboxProfile,
    state_dir: PathBuf,
    projects: Vec<PathBuf>,
) -> Task<Message> {
    if placement != PlacementKind::LocalSandbox {
        return Task::none();
    }

    Task::future(async move {
        let outcome = tokio::task::spawn_blocking(move || {
            let facts = crate::shell::sandbox::HostFacts::gather(state_dir);
            // Progress is dropped here rather than streamed: a `Task::future` yields one message,
            // and threading a channel through boot for the sake of the first release's progress bar
            // would buy less than the settings view (US3) will when it renders this properly.
            crate::shell::sandbox::start(
                &profile,
                &projects,
                &facts,
                crate::shell::sandbox::control_port(),
                &mut |_| {},
            )
            .map(|ready| ready.started)
        })
        .await;

        match outcome {
            Ok(Ok(started)) => Message::SandboxStarted(Box::new(started)),
            Ok(Err(failure)) => Message::SandboxFailed(Box::new(failure)),
            Err(join) => {
                Message::SandboxFailed(Box::new(micold_core::sandbox::lifecycle::Failure {
                    stage: micold_core::sandbox::lifecycle::Stage::Starting,
                    error: micold_core::sandbox::runtime::RuntimeError::Unknown {
                        stderr: join.to_string(),
                    },
                }))
            }
        }
    })
}
