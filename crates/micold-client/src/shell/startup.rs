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
use std::sync::atomic::AtomicUsize;
use std::sync::Arc;

use iced::Task;
use micold_client::app::{Message, State};
use micold_client::input::SessionInputStamper;
use micold_core::settings::Settings;

use crate::shell::capabilities::Capabilities;
use crate::shell::persist::prune_empty_sessions;
use crate::{
    default_resolution_cwd, detect_system_scheme, discover_worktrees, observe_system_scheme,
    probe_config, resolve_env_include, subscription, theme, update, view, App,
};

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
        .subscription(subscription)
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
        // Ask for the initial window size up front: `resize_events` only fires on *changes*, so
        // without this the first context menu before any resize would have nothing to clamp
        // against (feature 015).
        iced::window::latest()
            .and_then(iced::window::size)
            .map(|size| Message::WindowResized {
                width: size.width.max(0.0) as u16,
                height: size.height.max(0.0) as u16,
            }),
    )
}
