//! Persistence boundary for application settings (currently just the theme preference).
//!
//! Fronted as a trait so logic stays testable without the real user data directory
//! (Constitution Principle I). The production `JsonFileSettingsStore` reuses the pattern in
//! `store.rs`: `serde_json` + `directories`, an atomic temp-file+rename write, and a
//! missing/corrupt file that degrades to defaults rather than crashing (Principle IV;
//! FR-019). On-disk format is the durable contract in
//! `specs/003-material-design-layout/contracts/settings-schema.md`.

use crate::store::LoadStatus;
use crate::theme::ThemePreference;
use serde::{Deserialize, Serialize};
use std::io;
use std::path::PathBuf;

/// The current on-disk settings schema version (see the settings-schema contract). Bumped to
/// `2` in feature 006 when `scrollback_lines` was added (missing field still defaults on read).
const SETTINGS_VERSION: u32 = 2;

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

/// The persisted application settings document.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Settings {
    /// How the app selects its theme.
    #[serde(default)]
    pub theme: ThemePreference,
    /// Per-session embedded-terminal scrollback limit in lines (feature 006, FR-020/FR-021).
    #[serde(default = "default_scrollback")]
    pub scrollback_lines: usize,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            theme: ThemePreference::default(),
            scrollback_lines: DEFAULT_SCROLLBACK_LINES,
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
}

impl StoredSettings {
    fn from_settings(settings: &Settings) -> Self {
        Self {
            settings_version: SETTINGS_VERSION,
            theme: settings.theme,
            scrollback_lines: settings.scrollback_lines,
        }
    }

    fn into_settings(self) -> Settings {
        Settings {
            theme: self.theme,
            scrollback_lines: clamp_scrollback(self.scrollback_lines),
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
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let stored = StoredSettings::from_settings(settings);
        let json = serde_json::to_string_pretty(&stored)
            .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))?;

        // Atomic write: temp file in the same directory, then rename over the target.
        let temp = self.temp_path();
        std::fs::write(&temp, json)?;
        std::fs::rename(&temp, &self.path)?;
        Ok(())
    }
}
