//! The Settings draft — one in-progress edit, seen through four sections (feature 021 T018;
//! feature 027 US3).
//!
//! Every text field is held as text, even the two that are numbers: the form holds what the user
//! has typed, so a half-typed "12" is representable without being a valid scrollback limit.
//! Parsing happens once, on save, in [`SettingsDraft::validate`].
//!
//! # The split this module used to record is closed
//!
//! It said: "the validation that turns these strings into settings — the range checks and their
//! error messages — lives in `main.rs`'s `Message::SettingsSaved` arm rather than beside the type
//! it validates, which is what FR-001 asks against."
//!
//! The sectioned view is what made that untenable rather than merely untidy. A rejected save now
//! has to name the *section* holding the offending field, so the user is shown the control the
//! message is about — and the shell arm had no idea which section a field belonged to, nor any
//! business knowing. That knowledge is a property of the draft's shape, so the validation came
//! here with it. What is left in the shell is the part that was always the shell's: writing the
//! file, telling a connected daemon, and re-sourcing the environment.

use std::collections::BTreeSet;
use std::path::PathBuf;

use crate::app::FieldId;
use micold_core::sandbox::placement::PlacementKind;
use micold_core::sandbox::SandboxProfile;
use micold_core::settings::{DaemonConfig, Settings};
use micold_core::theme::ThemePreference;

/// One page of the Settings view (FR-026).
///
/// Ordered as the rail shows them: the two that describe the window the user is looking at, then
/// the two that describe what runs underneath it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum SettingsSection {
    /// Theme.
    #[default]
    Appearance,
    /// The embedded terminal.
    Terminal,
    /// The environment sourced into each session.
    Environment,
    /// Where the session service runs and what it may reach (FR-028).
    Daemon,
}

impl SettingsSection {
    /// Every section, in rail order.
    pub const ALL: &'static [SettingsSection] = &[
        SettingsSection::Appearance,
        SettingsSection::Terminal,
        SettingsSection::Environment,
        SettingsSection::Daemon,
    ];

    /// The section's name, as the rail shows it.
    pub fn label(self) -> &'static str {
        match self {
            SettingsSection::Appearance => "Appearance",
            SettingsSection::Terminal => "Terminal",
            SettingsSection::Environment => "Environment",
            SettingsSection::Daemon => "Session service",
        }
    }

    /// Its position in the rail, which is what [`SectionList::selected`] is given.
    ///
    /// [`SectionList::selected`]: crate::ui::material::SectionList::selected
    pub fn index(self) -> usize {
        Self::ALL
            .iter()
            .position(|s| *s == self)
            .expect("every section is in ALL")
    }
}

/// The Appearance section's fields.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AppearanceDraft {
    /// Light, dark, or follow the system.
    pub theme: ThemePreference,
}

/// The Terminal section's fields.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TerminalDraft {
    /// The editable scrollback limit (parsed and range-checked on save).
    pub scrollback_lines: String,
}

/// The Environment section's fields (feature 011).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct EnvironmentDraft {
    /// Whether a script is sourced before each session starts (FR-001).
    pub enabled: bool,
    /// The script to source (FR-002).
    pub script_path: String,
    /// Its timeout, in seconds as text (FR-003).
    pub timeout_secs: String,
}

/// The Session service section's fields (feature 027, FR-028).
///
/// The sandbox profile is held whole rather than field by field, so that a setting this section
/// does not render yet — the limits and the network posture, which arrive with US4 — survives a
/// save instead of being reset to its default by a draft that had never heard of it.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DaemonDraft {
    /// Where the service runs (FR-001).
    pub placement: PlacementKind,
    /// Everything about the sandbox, whether or not it is the selected placement.
    pub profile: SandboxProfile,
    /// The archive to import, as typed. Text rather than the profile's `Option<PathBuf>` for the
    /// same reason the numbers are text: an empty field is a state the user passes through.
    pub image_path: String,
}

/// A rejected save: what was wrong, where the control is, and what to say.
///
/// The section is the part a modal never needed. With one page, "Enter a number between 100 and
/// 100000" was a complete report; with four, it names a field the user may not be able to see.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FieldError {
    /// The control the message is about.
    pub field: FieldId,
    /// The section that control is in, so reporting can show it (FR-029).
    pub section: SettingsSection,
    /// What to tell the user, naming the accepted range where there is one.
    pub message: String,
}

/// The draft, parsed and in range — what a save applies.
///
/// A separate type from [`Settings`] because it is not one: the daemon half is assembled from a
/// profile the draft edited and a placement it chose, and returning `Settings` would need the
/// theme's own persistence path to agree about who writes the file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidSettings {
    /// Appearance.
    pub theme: ThemePreference,
    /// Terminal.
    pub scrollback_lines: usize,
    /// Environment.
    pub env_include_enabled: bool,
    /// Environment.
    pub env_include_script_path: String,
    /// Environment.
    pub env_include_timeout_secs: u64,
    /// Session service.
    pub daemon: DaemonConfig,
}

impl ValidSettings {
    /// The persisted shape, ready to write.
    pub fn into_settings(self) -> Settings {
        Settings {
            theme: self.theme,
            scrollback_lines: self.scrollback_lines,
            env_include_enabled: self.env_include_enabled,
            env_include_script_path: self.env_include_script_path,
            env_include_timeout_secs: self.env_include_timeout_secs,
            daemon: self.daemon,
        }
    }
}

/// In-progress Settings state, present only while the Settings view is shown.
///
/// **One draft, four views of it.** The sections are pages over this single value, not four forms:
/// navigating away from a section and back must not revert what was typed, and a save applies
/// every section at once (US3 scenario 2). Both follow from there being one of these.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SettingsDraft {
    /// The section on screen.
    pub section: SettingsSection,
    /// Appearance.
    pub appearance: AppearanceDraft,
    /// Terminal.
    pub terminal: TerminalDraft,
    /// Environment.
    pub environment: EnvironmentDraft,
    /// Session service.
    pub daemon: DaemonDraft,
    /// The last validation failure shown after a rejected save.
    pub error: Option<FieldError>,
}

impl SettingsDraft {
    /// Show `section`. Nothing else changes — in particular nothing is reseeded, which is the
    /// whole of US3 scenario 2.
    pub fn show(&mut self, section: SettingsSection) {
        self.section = section;
    }

    /// Any field changed, so a previous rejection no longer describes the form.
    ///
    /// One method rather than a line in each control's reducer arm: the rule is "editing clears
    /// the error", and stating it per field is how one field ends up not clearing it.
    pub fn edited(&mut self) {
        self.error = None;
    }

    /// Show a rejected save's failure, and the section holding the field it is about (FR-029).
    pub fn report(&mut self, error: FieldError) {
        self.section = error.section;
        self.error = Some(error);
    }

    /// Whether the user has shared any host credential with the sandbox (FR-004c).
    ///
    /// Asked by the rail, which marks the section, and by the section itself. One question with
    /// one answer, so the badge and the summary cannot disagree.
    pub fn shares_credentials(&self) -> bool {
        !self.daemon.profile.credentials.is_empty()
    }

    /// Parse and range-check every section together.
    ///
    /// Total: it reports, and never panics, for anything that can be typed. The order the fields
    /// are checked in is the rail's order, so a form with two bad values reports the first one a
    /// user reading top to bottom would reach.
    pub fn validate(&self) -> Result<ValidSettings, FieldError> {
        let scrollback_lines = self.scrollback()?;
        let env_include_timeout_secs = self.timeout()?;

        let mut profile = self.daemon.profile.clone();
        profile.image.path = {
            let trimmed = self.daemon.image_path.trim();
            (!trimmed.is_empty()).then(|| PathBuf::from(trimmed))
        };

        Ok(ValidSettings {
            theme: self.appearance.theme,
            scrollback_lines,
            env_include_enabled: self.environment.enabled,
            env_include_script_path: self.environment.script_path.clone(),
            env_include_timeout_secs,
            daemon: DaemonConfig {
                placement: self.daemon.placement,
                sandbox: profile,
            },
        })
    }

    fn scrollback(&self) -> Result<usize, FieldError> {
        let min = micold_core::settings::MIN_SCROLLBACK_LINES;
        let max = micold_core::settings::MAX_SCROLLBACK_LINES;
        let reject = |message: String| FieldError {
            field: FieldId::SettingsScrollback,
            section: SettingsSection::Terminal,
            message,
        };
        match self.terminal.scrollback_lines.trim().parse::<usize>() {
            Ok(n) if (min..=max).contains(&n) => Ok(n),
            Ok(_) => Err(reject(format!("Enter a number between {min} and {max}."))),
            Err(_) => Err(reject("Enter a whole number of lines.".to_string())),
        }
    }

    fn timeout(&self) -> Result<u64, FieldError> {
        let min = micold_core::settings::MIN_ENV_INCLUDE_TIMEOUT_SECS;
        let max = micold_core::settings::MAX_ENV_INCLUDE_TIMEOUT_SECS;
        let reject = |message: String| FieldError {
            field: FieldId::SettingsEnvIncludeTimeout,
            section: SettingsSection::Environment,
            message,
        };
        match self.environment.timeout_secs.trim().parse::<u64>() {
            Ok(t) if (min..=max).contains(&t) => Ok(t),
            Ok(_) => Err(reject(format!(
                "Enter a timeout between {min} and {max} seconds."
            ))),
            Err(_) => Err(reject("Enter a whole number of seconds.".to_string())),
        }
    }

    /// Seed the draft from what is stored, so the form opens showing the current values.
    pub fn from_settings(settings: &Settings) -> Self {
        Self {
            section: SettingsSection::default(),
            appearance: AppearanceDraft {
                theme: settings.theme,
            },
            terminal: TerminalDraft {
                scrollback_lines: settings.scrollback_lines.to_string(),
            },
            environment: EnvironmentDraft {
                enabled: settings.env_include_enabled,
                script_path: settings.env_include_script_path.clone(),
                timeout_secs: settings.env_include_timeout_secs.to_string(),
            },
            daemon: DaemonDraft {
                placement: settings.daemon.placement,
                profile: settings.daemon.sandbox.clone(),
                image_path: settings
                    .daemon
                    .sandbox
                    .image
                    .path
                    .as_ref()
                    .map(|p| p.display().to_string())
                    .unwrap_or_default(),
            },
            error: None,
        }
    }

    /// The credentials the user has shared, in a stable order (rule N-2).
    pub fn shared_credentials(&self) -> &BTreeSet<micold_core::sandbox::CredentialShare> {
        &self.daemon.profile.credentials
    }
}
