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
use crate::session::AiCli;
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
    /// Which AI CLI a new session runs when nothing is chosen for it (feature 026, FR-003).
    ///
    /// **Not validated against availability.** A default naming an uninstalled CLI is kept, not
    /// silently rewritten: FR-004 asks the application to *tell* the user, and a preference that
    /// quietly repaired itself would also lose the user's choice across a temporary `PATH` problem
    /// (research R11).
    #[serde(default)]
    pub default_ai_cli: AiCli,
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
            default_ai_cli: AiCli::default(),
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
/// What one [`SettingsStore::update`] saw and did.
///
/// The base status is the part a plain `io::Result` cannot carry, and it is the part BUG-025
/// needed: the reset had to be attributed by elimination from the bytes left on disk, because
/// neither log recorded that a settings write had happened at all, let alone what document it
/// merged into (T162).
#[derive(Debug)]
pub struct SettingsWrite {
    /// The status of the read this write used as its base.
    pub base: LoadStatus,
    /// What the write itself did.
    pub result: io::Result<()>,
}

impl SettingsWrite {
    /// One log line naming the writer, the process, the base, and the outcome.
    ///
    /// Formatted here in the library rather than at each call site so it can be asserted on — the
    /// same reason `attach_log_line` lives in the client's library. A diagnostic nothing tests is
    /// a diagnostic that quietly stops being written, which is the failure this task is about.
    pub fn log_line(&self, writer: &str) -> String {
        let pid = std::process::id();
        let base = match self.base {
            LoadStatus::Loaded => "loaded",
            LoadStatus::Missing => "missing",
            LoadStatus::Recovered => "recovered",
        };
        match &self.result {
            Ok(()) => format!("settings write: writer={writer} pid={pid} base={base} result=ok"),
            Err(err) => {
                format!(
                    "settings write: writer={writer} pid={pid} base={base} result=failed err={err}"
                )
            }
        }
    }
}

pub trait SettingsStore {
    /// Load settings. Never fails: a missing or corrupt file yields [`Settings::default`]
    /// with the corresponding [`LoadStatus`] (FR-019).
    fn load(&self) -> SettingsOutcome;

    /// Persist settings. Writes atomically (temp file + rename).
    fn save(&self, settings: &Settings) -> io::Result<()>;

    /// Read the stored settings, apply `change` to them, and write the result back.
    ///
    /// **The only way a writer should persist a change** (FR-010a, FR-010c). Both writers of this
    /// file — the client for its own fields, the service for its five — must merge into the
    /// document that is on disk at save time rather than into a copy of their own, and must not
    /// merge into a base that a failed read produced. Doing that as `load()` then `save()` at each
    /// call site is what BUG-025 was: the status beside the settings is easy to drop, and every
    /// caller dropped it.
    fn update(&self, change: &mut dyn FnMut(&mut Settings)) -> io::Result<()> {
        self.update_reporting(change).result
    }

    /// [`SettingsStore::update`], plus the base status the write used — for the two call sites
    /// that log it (T162). The behaviour is identical; only the report is wider.
    fn update_reporting(&self, change: &mut dyn FnMut(&mut Settings)) -> SettingsWrite {
        // Held across the read *and* the write (FR-010b, T154). Two writers whose read-modify-write
        // cycles interleave both merge into the document from before the other's change, so the
        // later writer writes the earlier one's change away — a perfectly valid file that is
        // missing a setting the user was told had been saved. Staging each write separately (T153)
        // makes the file parseable; only mutual exclusion makes it complete.
        let guard = match self.begin_exclusive() {
            Ok(guard) => guard,
            // No base was read, so there is no honest status to report. `Missing` would claim a
            // first run; the error itself says what happened.
            Err(err) => {
                return SettingsWrite {
                    base: LoadStatus::Missing,
                    result: Err(err),
                }
            }
        };
        let _guard = guard;
        let outcome = self.load();
        // A read that did not return the stored document must not be the base of a write, and a
        // read failure must not destroy the file it failed to read (FR-010c, T159).
        //
        // The test is the file, not the status. `Missing` is a first run — there is nothing to
        // lose and defaults are the right base. `Recovered` after a *corrupt* file is also safe:
        // `load` has already moved that file aside to `.bak`, so nothing survives for this write
        // to destroy. `Recovered` for a file that is still sitting there is the dangerous one —
        // present, never corrupt, merely unreadable this time — and writing defaults over it
        // replaces the whole `daemon` block, because the merge preserves top-level keys only.
        if outcome.status != LoadStatus::Loaded && self.stored_document_exists() {
            // Phrased to read correctly inside the client's "Couldn't save your settings: {err}"
            // wrapper and the daemon's operation-error frame (T160) — the message a user sees is
            // the whole point of refusing rather than quietly writing defaults.
            return SettingsWrite {
                base: outcome.status,
                result: Err(io::Error::other(
                    "the settings file could not be read, and saving now would replace it with \
                     defaults",
                )),
            };
        }
        let base = outcome.status;
        let mut settings = outcome.settings;
        change(&mut settings);
        SettingsWrite {
            base,
            result: self.save(&settings),
        }
    }

    /// Whether a stored document is still present after [`SettingsStore::load`] has run.
    ///
    /// Only [`SettingsStore::update`] uses this, to tell a read failure that lost nothing from one
    /// that is about to. Defaults to `false`: an in-memory store has no document on disk that a
    /// save could destroy.
    fn stored_document_exists(&self) -> bool {
        false
    }

    /// Where [`SettingsStore::load`] preserves a file it could not parse, for a store that has
    /// such a place.
    ///
    /// Exists so the recovery can be *reported* naming the file (T158): "your settings were reset"
    /// is an apology, and the same sentence with the path is something the user can act on.
    /// `None` for a store with nothing on disk to preserve.
    fn recovery_path(&self) -> Option<PathBuf> {
        None
    }

    /// Take exclusive access for the duration of one [`SettingsStore::update`], released when the
    /// returned guard drops (FR-010b).
    ///
    /// Defaults to no guard at all: an in-memory store has one copy of the settings behind its own
    /// `Mutex`, and no second process can reach it. Only a store backed by a shared file needs
    /// this, and only that implementation pays for it.
    fn begin_exclusive(&self) -> io::Result<Box<dyn Send>> {
        Ok(Box::new(()))
    }
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
    /// Missing in pre-026 files → defaults to `ClaudeCode`, which is the right answer for every
    /// settings file written before this feature (feature 026, FR-003).
    ///
    /// `settings_version` deliberately does **not** move for this. The `#[serde(default)]`
    /// argument that spares `schema_version` in `store.rs` (research R8) spares this third version
    /// number for the same reason: a bump would cost a migration path to express a default.
    #[serde(default)]
    default_ai_cli: AiCli,
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
            default_ai_cli: settings.default_ai_cli,
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
                // The same read-repairs-what-the-user-did-not-choose rule, applied to the image
                // namespace this feature retired. Correcting `DEFAULT_IMAGE` only reached users
                // with no `sandbox` block on disk; this reaches the ones who have one (FR-024).
                daemon.sandbox.image.repair_retired_namespace();
                daemon
            },
            // Not clamped, and not checked against availability: unlike the two numbers above,
            // there is no invalid value to repair — only a CLI that may not be installed today,
            // which is the user's choice to keep (research R11).
            default_ai_cli: self.default_ai_cli,
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

    /// Path of the temporary file used for one atomic write. Unique per call — see
    /// [`crate::store`]'s `temp_path_for`, which this delegates to, for why deriving it from the
    /// target alone is what made two writers of one settings file destroy each other (T153,
    /// BUG-025).
    fn temp_path(&self) -> PathBuf {
        crate::store::temp_path_for(&self.path)
    }

    fn backup_path(&self) -> PathBuf {
        self.path.with_extension("json.bak")
    }

    /// The sidecar whose file lock serialises read-modify-write cycles on `settings.json`.
    ///
    /// A sidecar rather than the settings file itself: the write replaces that file by rename, so
    /// a lock held on it would be a lock on an inode no later writer will ever open. The sidecar
    /// is never renamed and never deleted, so every writer that resolves the same settings path
    /// resolves the same lock inode — including the daemon inside the sandbox, where the bind
    /// mount makes the container path and the host path one file.
    fn lock_path(&self) -> PathBuf {
        self.path.with_extension("json.lock")
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
    /// Blocks until the lock is ours (T154, T156). Waiting is the right answer over failing: the
    /// critical section is one small read and one small write, the competing writer is about to
    /// release, and a save that returns an error the user must act on — for a conflict the app
    /// resolves by waiting a millisecond — would be noise. What FR-010b forbids is the third
    /// option, proceeding *without* the lock; a lock that cannot be opened at all is a real
    /// failure and is returned as one, which stops the save before anything is written.
    fn begin_exclusive(&self) -> io::Result<Box<dyn Send>> {
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let lock = std::fs::OpenOptions::new()
            .create(true)
            .read(true)
            .write(true)
            .truncate(false)
            .open(self.lock_path())?;
        lock.lock()?;
        Ok(Box::new(lock))
    }

    fn recovery_path(&self) -> Option<PathBuf> {
        Some(self.backup_path())
    }

    /// After a load, the file is still there for every failure except the corrupt one, which
    /// `load` has moved aside to `.bak`. That is exactly the line [`SettingsStore::update`] needs
    /// (FR-010c, T159).
    fn stored_document_exists(&self) -> bool {
        self.path.exists()
    }

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

        // Atomic write: temp file in the same directory, then rename over the target. The
        // staging path is this write's alone (T153); sharing one across writers is what produced
        // the truncated documents in BUG-025.
        let temp = self.temp_path();
        crate::store::write_then_rename(&temp, &self.path, &json)
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
    /// What [`SettingsStore::recovery_path`] answers; `None` for a store that kept nothing.
    recovery_path: Option<PathBuf>,
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

    /// A recovered store that kept the unreadable file at `preserved` — the shape
    /// [`JsonFileSettingsStore`] leaves behind, without naming it outside the shell's single
    /// assembly point (FR-018).
    pub fn recovered_keeping(preserved: PathBuf) -> Self {
        let fake = Self::recovered();
        fake.inner.lock().expect("fake lock").recovery_path = Some(preserved);
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

    fn recovery_path(&self) -> Option<PathBuf> {
        self.inner.lock().expect("fake lock").recovery_path.clone()
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
