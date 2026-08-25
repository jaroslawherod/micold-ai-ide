//! Persistence boundary for application settings (currently just the theme preference).
//!
//! Fronted as a trait so logic stays testable without the real user data directory
//! (Constitution Principle I). The production `JsonFileSettingsStore` reuses the pattern in
//! `store.rs`: `serde_json` + `directories`, an atomic temp-file+rename write, and a
//! missing/corrupt file that degrades to defaults rather than crashing (Principle IV;
//! FR-019). On-disk format is the durable contract in
//! `specs/003-material-design-layout/contracts/settings-schema.md`.

use crate::sandbox::placement::PlacementKind;
use crate::sandbox::{BudgetViolation, SandboxProfile};
use crate::store::LoadStatus;
use crate::theme::ThemePreference;
use serde::{Deserialize, Serialize};
use std::io;
use std::path::{Path, PathBuf};

/// The current on-disk settings schema version (see the settings-schema contract). Bumped to
/// `2` in feature 006 when `scrollback_lines` was added (missing field still defaults on read).
/// Bumped to `3` in feature 011 when the environment-include fields were added (same
/// missing-field-defaults contract). Bumped to `4` in feature 027 for the nested `daemon` block —
/// same contract again, so a v3 file still loads without a migration step.
const SETTINGS_VERSION: u32 = 4;

/// Default per-session terminal scrollback (lines). Matches `alacritty_terminal 0.25`'s
/// `Config::scrolling_history` default (feature 006, FR-021).
pub const DEFAULT_SCROLLBACK_LINES: usize = 10_000;
/// Minimum accepted scrollback limit.
pub const MIN_SCROLLBACK_LINES: usize = 100;
/// Maximum accepted scrollback limit.
pub const MAX_SCROLLBACK_LINES: usize = 1_000_000;

fn default_scrollback() -> usize {
    DEFAULT_SCROLLBACK_LINES
}

/// Clamp a requested scrollback limit into the supported range (FR-020, FR-021).
pub fn clamp_scrollback(lines: usize) -> usize {
    lines.clamp(MIN_SCROLLBACK_LINES, MAX_SCROLLBACK_LINES)
}

/// Environment-include default: on by default (feature 011, FR-004).
pub const DEFAULT_ENV_INCLUDE_ENABLED: bool = true;
/// Environment-include default timeout, in seconds (FR-004).
pub const DEFAULT_ENV_INCLUDE_TIMEOUT_SECS: u64 = 10;
/// Minimum accepted environment-include timeout.
pub const MIN_ENV_INCLUDE_TIMEOUT_SECS: u64 = 1;
/// Maximum accepted environment-include timeout.
pub const MAX_ENV_INCLUDE_TIMEOUT_SECS: u64 = 60;

fn default_env_include_enabled() -> bool {
    DEFAULT_ENV_INCLUDE_ENABLED
}

fn default_env_include_timeout_secs() -> u64 {
    DEFAULT_ENV_INCLUDE_TIMEOUT_SECS
}

/// The platform's conventional interactive-shell startup file, joined onto `home` (feature 011,
/// research R7): `<home>/.bashrc` on Linux/macOS (sourced via bash — FR-017), or the current
/// user's PowerShell profile on Windows (FR-018). Pure and argument-driven, mirroring
/// `terminal.rs::default_shell_command` — the impure home-directory lookup happens once at the
/// call site (`default_env_include_script_path_string`), not here.
pub fn default_env_include_path(home: Option<&Path>) -> PathBuf {
    let base = home.unwrap_or_else(|| Path::new(""));
    if cfg!(windows) {
        base.join("Documents")
            .join("WindowsPowerShell")
            .join("profile.ps1")
    } else {
        base.join(".bashrc")
    }
}

fn default_env_include_script_path_string() -> String {
    let home = directories::UserDirs::new().map(|dirs| dirs.home_dir().to_path_buf());
    default_env_include_path(home.as_deref())
        .to_string_lossy()
        .into_owned()
}

/// Clamp a requested environment-include timeout into the supported range.
pub fn clamp_env_include_timeout(secs: u64) -> u64 {
    secs.clamp(MIN_ENV_INCLUDE_TIMEOUT_SECS, MAX_ENV_INCLUDE_TIMEOUT_SECS)
}

/// Everything about the session daemon: where it runs, and how the sandbox is configured when it
/// runs there (feature 027).
///
/// Nested rather than flattened with a `daemon_` prefix. The existing flat fields grew one feature
/// at a time and the root is already six keys wide; the sectioned Settings view (FR-026) makes the
/// grouping user-visible, and matching it on disk keeps the two readable together.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct DaemonConfig {
    /// Where the daemon runs. Defaults to the host process, so upgrading never moves a user into
    /// the sandbox (FR-001).
    #[serde(default)]
    pub placement: PlacementKind,
    /// The sandbox configuration, whether or not the sandbox is currently selected — a user who
    /// switches back keeps what they set.
    #[serde(default)]
    pub sandbox: SandboxProfile,
}

/// The persisted application settings document.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Settings {
    /// How the app selects its theme.
    #[serde(default)]
    pub theme: ThemePreference,
    /// Per-session embedded-terminal scrollback limit in lines (feature 006, FR-020/FR-021).
    #[serde(default = "default_scrollback")]
    pub scrollback_lines: usize,
    /// Whether environment-include is active (feature 011, FR-001/FR-004).
    #[serde(default = "default_env_include_enabled")]
    pub env_include_enabled: bool,
    /// The script path to source for environment-include (FR-002/FR-004).
    #[serde(default = "default_env_include_script_path_string")]
    pub env_include_script_path: String,
    /// How long, in seconds, sourcing the script may run before being treated as hung
    /// (FR-003/FR-004).
    #[serde(default = "default_env_include_timeout_secs")]
    pub env_include_timeout_secs: u64,
    /// Where the session daemon runs, and its sandbox profile (feature 027).
    #[serde(default)]
    pub daemon: DaemonConfig,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            theme: ThemePreference::default(),
            scrollback_lines: DEFAULT_SCROLLBACK_LINES,
            env_include_enabled: DEFAULT_ENV_INCLUDE_ENABLED,
            env_include_script_path: default_env_include_script_path_string(),
            env_include_timeout_secs: DEFAULT_ENV_INCLUDE_TIMEOUT_SECS,
            daemon: DaemonConfig::default(),
        }
    }
}

/// How a settings load resolved. Always yields usable [`Settings`]; `status` distinguishes a
/// clean first run from a recovery.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SettingsOutcome {
    /// The loaded settings (defaults on a missing or recovered file).
    pub settings: Settings,
    /// What happened during the load.
    pub status: LoadStatus,
}

/// Load and save application settings on the local filesystem (local-first).
pub trait SettingsStore {
    /// Load settings. Never fails: a missing or corrupt file yields [`Settings::default`]
    /// with the corresponding [`LoadStatus`] (FR-019).
    fn load(&self) -> SettingsOutcome;

    /// Persist settings. Writes atomically (temp file + rename).
    fn save(&self, settings: &Settings) -> io::Result<()>;
}

/// The on-disk shape. Unknown fields are ignored on read and a missing `theme` takes its
/// serde default (FollowSystem) — both give forward compatibility (settings-schema contract).
#[derive(Debug, Serialize, Deserialize)]
struct StoredSettings {
    settings_version: u32,
    #[serde(default)]
    theme: ThemePreference,
    /// Missing in v1 files → defaults to [`DEFAULT_SCROLLBACK_LINES`] (backward compatible).
    #[serde(default = "default_scrollback")]
    scrollback_lines: usize,
    /// Missing in pre-011 (v2) files → defaults to [`DEFAULT_ENV_INCLUDE_ENABLED`] (`true`).
    #[serde(default = "default_env_include_enabled")]
    env_include_enabled: bool,
    /// Missing in pre-011 (v2) files → defaults to the platform's conventional path.
    #[serde(default = "default_env_include_script_path_string")]
    env_include_script_path: String,
    /// Missing in pre-011 (v2) files → defaults to [`DEFAULT_ENV_INCLUDE_TIMEOUT_SECS`] (`10`).
    #[serde(default = "default_env_include_timeout_secs")]
    env_include_timeout_secs: u64,
    /// Missing in pre-027 (v3) files → the host placement with a default sandbox profile.
    #[serde(default)]
    daemon: DaemonConfig,
}

impl StoredSettings {
    fn from_settings(settings: &Settings) -> Self {
        Self {
            settings_version: SETTINGS_VERSION,
            theme: settings.theme,
            scrollback_lines: settings.scrollback_lines,
            env_include_enabled: settings.env_include_enabled,
            env_include_script_path: settings.env_include_script_path.clone(),
            env_include_timeout_secs: settings.env_include_timeout_secs,
            daemon: settings.daemon.clone(),
        }
    }

    fn into_settings(self) -> Settings {
        Settings {
            theme: self.theme,
            scrollback_lines: clamp_scrollback(self.scrollback_lines),
            env_include_enabled: self.env_include_enabled,
            env_include_script_path: self.env_include_script_path,
            env_include_timeout_secs: clamp_env_include_timeout(self.env_include_timeout_secs),
            daemon: {
                let mut daemon = self.daemon;
                // Clamp on read, refuse on save (rule S-7): a hand-edited file opens the app with
                // a corrected value, while the same value typed into the view is refused with the
                // accepted range named (FR-016). The two paths differ on purpose — one is the
                // user's mistake, the other is a file they may not have written.
                daemon.sandbox.budget.clamp();
                daemon
            },
        }
    }
}

/// The production [`SettingsStore`]: a JSON file in the per-user data directory.
#[derive(Debug, Clone)]
pub struct JsonFileSettingsStore {
    path: PathBuf,
}

impl JsonFileSettingsStore {
    /// A store backed by an explicit file path (used by tests with a temp directory).
    pub fn at(path: PathBuf) -> Self {
        Self { path }
    }

    /// A store at the conventional per-user location: `<data_dir>/settings.json`, sharing
    /// the same application tuple (and therefore directory) as the projects store
    /// (settings-schema contract). Returns `None` if no data directory can be determined.
    pub fn default_location() -> Option<Self> {
        directories::ProjectDirs::from("", "", "micold-ai-ide")
            .map(|dirs| Self::at(dirs.data_dir().join("settings.json")))
    }

    fn temp_path(&self) -> PathBuf {
        self.path.with_extension("json.tmp")
    }

    fn backup_path(&self) -> PathBuf {
        self.path.with_extension("json.bak")
    }

    /// The document to write: this build's fields laid over whatever is already on disk.
    ///
    /// Rule S-5, new in v4. A settings file written by a newer build and opened by an older one
    /// used to lose the newer build's keys on the next save, silently. The flat schema made that
    /// cheap to ignore — one lost boolean; the nested `daemon` block does not, because one older
    /// build opening the file would drop a whole section.
    ///
    /// Top-level keys only. Preserving unknown keys *nested inside* a block this build does know
    /// about would mean merging recursively into a shape we are simultaneously rewriting, and the
    /// result would be neither the old value nor the new one. Documented rather than attempted.
    fn merged_with_existing(&self, stored: &StoredSettings) -> io::Result<serde_json::Value> {
        let fresh = serde_json::to_value(stored)
            .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))?;

        let Some(existing) = std::fs::read_to_string(&self.path)
            .ok()
            .and_then(|text| serde_json::from_str::<serde_json::Value>(&text).ok())
        else {
            // No readable file, or one that is not JSON at all: nothing to preserve. A corrupt
            // file is already handled by `load`, which moves it aside.
            return Ok(fresh);
        };

        match (existing, fresh) {
            (serde_json::Value::Object(mut base), serde_json::Value::Object(new)) => {
                base.extend(new);
                Ok(serde_json::Value::Object(base))
            }
            (_, fresh) => Ok(fresh),
        }
    }
}

impl SettingsStore for JsonFileSettingsStore {
    fn load(&self) -> SettingsOutcome {
        let contents = match std::fs::read_to_string(&self.path) {
            Ok(contents) => contents,
            Err(err) if err.kind() == io::ErrorKind::NotFound => {
                return SettingsOutcome {
                    settings: Settings::default(),
                    status: LoadStatus::Missing,
                };
            }
            // Unreadable for any other reason: recover rather than crash (Principle IV).
            Err(_) => {
                return SettingsOutcome {
                    settings: Settings::default(),
                    status: LoadStatus::Recovered,
                };
            }
        };

        match serde_json::from_str::<StoredSettings>(&contents) {
            Ok(stored) => SettingsOutcome {
                settings: stored.into_settings(),
                status: LoadStatus::Loaded,
            },
            Err(_) => {
                // Corrupt: preserve the bad file (best-effort) and recover to defaults.
                let _ = std::fs::rename(&self.path, self.backup_path());
                SettingsOutcome {
                    settings: Settings::default(),
                    status: LoadStatus::Recovered,
                }
            }
        }
    }

    fn save(&self, settings: &Settings) -> io::Result<()> {
        // A limit below what the daemon needs to run is refused here rather than corrected, and
        // the refusal names the accepted range (FR-016, US4 scenario 5). This is deliberately the
        // opposite of what `load` does with the same value: a file the user did not write is
        // clamped and reported (rule S-7), because opening the app must not fail. A number the
        // user just typed is theirs, and silently replacing it is the application deciding it
        // knows better and saying nothing.
        //
        // Refused *before* the directory is created and before anything is written, so a rejected
        // save leaves the stored document exactly as it was.
        let violations = settings.daemon.sandbox.budget.violations();
        if !violations.is_empty() {
            let message = violations
                .iter()
                .map(BudgetViolation::message)
                .collect::<Vec<_>>()
                .join("; ");
            return Err(io::Error::new(io::ErrorKind::InvalidInput, message));
        }

        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let stored = StoredSettings::from_settings(settings);
        let json = serde_json::to_string_pretty(&self.merged_with_existing(&stored)?)
            .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))?;

        // Atomic write: temp file in the same directory, then rename over the target.
        let temp = self.temp_path();
        std::fs::write(&temp, json)?;
        std::fs::rename(&temp, &self.path)?;
        Ok(())
    }
}

// ---------------------------------------------------------------------------------------
// In-memory fake for unit tests. Public (not `#[cfg(test)]`) so integration tests in
// `tests/` can share it, matching `FakeGit` (FR-019, feature 021 T048). Pure — no disk.
// ---------------------------------------------------------------------------------------

use std::sync::Mutex;

/// An in-memory [`SettingsStore`] for tests: serves fixed settings and records every write.
///
/// `Mutex` rather than `RefCell` because the daemon boxes its settings store as `Send + Sync`.
#[derive(Debug, Default)]
pub struct FakeSettingsStore {
    inner: Mutex<FakeSettingsState>,
}

#[derive(Debug, Default)]
struct FakeSettingsState {
    settings: Settings,
    status: Option<LoadStatus>,
    /// Every settings value handed to `save`, in call order.
    saves: Vec<Settings>,
    /// When set, the next `save` fails with this kind.
    fail_next_save: Option<io::ErrorKind>,
}

impl FakeSettingsStore {
    /// A store that has never been written: defaults, [`LoadStatus::Missing`].
    pub fn new() -> Self {
        Self::default()
    }

    /// A store serving `settings` as a clean [`LoadStatus::Loaded`].
    pub fn loaded(settings: Settings) -> Self {
        let fake = Self::new();
        {
            let mut state = fake.inner.lock().expect("fake lock");
            state.settings = settings;
            state.status = Some(LoadStatus::Loaded);
        }
        fake
    }

    /// A store whose file was unparseable: defaults, [`LoadStatus::Recovered`].
    pub fn recovered() -> Self {
        let fake = Self::new();
        fake.inner.lock().expect("fake lock").status = Some(LoadStatus::Recovered);
        fake
    }

    /// Make the next [`SettingsStore::save`] fail.
    pub fn failing_save(self, kind: io::ErrorKind) -> Self {
        self.inner.lock().expect("fake lock").fail_next_save = Some(kind);
        self
    }

    /// Every settings value handed to `save`, in call order.
    pub fn saves(&self) -> Vec<Settings> {
        self.inner.lock().expect("fake lock").saves.clone()
    }
}

impl SettingsStore for FakeSettingsStore {
    fn load(&self) -> SettingsOutcome {
        let state = self.inner.lock().expect("fake lock");
        SettingsOutcome {
            settings: state.settings.clone(),
            status: state.status.unwrap_or(LoadStatus::Missing),
        }
    }

    fn save(&self, settings: &Settings) -> io::Result<()> {
        let mut state = self.inner.lock().expect("fake lock");
        if let Some(kind) = state.fail_next_save.take() {
            return Err(io::Error::new(kind, "fake: save refused"));
        }
        state.saves.push(settings.clone());
        state.settings = settings.clone();
        Ok(())
    }
}
