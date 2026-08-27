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
//! error messages — lives in the shell's settings-save arm rather than beside the type
//! it validates, which is what FR-001 asks against."
//!
//! The sectioned view is what made that untenable rather than merely untidy. A rejected save now
//! has to name the *section* holding the offending field, so the user is shown the control the
//! message is about — and the shell arm had no idea which section a field belonged to, nor any
//! business knowing. That knowledge is a property of the draft's shape, so the validation came
//! here with it. What is left in the shell is the part that was always the shell's: writing the
//! file, telling a connected daemon, and re-sourcing the environment.
//!
//! # The vocabulary this feature declares
//!
//! Twenty-five transitions in [`Msg`]: the theme's three that apply immediately
//! (`ThemePreferenceChanged`, `ThemeModeCycled`, `SystemThemeChanged`), the view's navigation
//! (`Opened`, `SectionShown`), the four sections' nineteen field edits, and the two ways out
//! (`Saved`, `Cancelled`).
//!
//! [`update`] routes all of them and is pure (data-model.md §1.1 shape A), but the feature's entry
//! shape is **B**: `main.rs` has one `Message::Settings` arm and it goes to `shell/settings.rs`,
//! because four of them additionally need `settings.json` written or read back
//! (`Opened`, `Saved`, `ThemePreferenceChanged`, `ThemeModeCycled`). The rest reach [`update`]
//! from there through the same wrapper variant they arrived under, so the pure path is identical
//! whether or not the shell was in the way. One routing table, one place to look.
//!
//! # The state this feature remembers (feature 028, contract S1)
//!
//! Three fields in [`State`], reached as `state.settings`: `settings_draft`, the edits in flight
//! while the view is open (`None` when it is closed — its presence *is* the view being shown);
//! `theme_pref`, what the user chose; and `system_scheme`, what the desktop most recently reported.
//!
//! All three keep the names they had flat on the root (T032). `settings_draft` reads as a stutter
//! and is not one: `settings.draft` would lose that this is a draft *of the settings*, and the
//! field is distinguished from `theme_pref` beside it precisely by being the view's copy rather
//! than the live value.
//!
//! **Saved settings are not here.** These are read back from `settings.json` by `shell/persist.rs`
//! at the I/O boundary; what this struct holds is the in-flight edit and the two theme inputs the
//! resolution needs.

use std::collections::BTreeSet;
use std::path::PathBuf;

use crate::features::window::FieldId;
use micold_core::sandbox::placement::PlacementKind;
use micold_core::sandbox::runtime::RuntimeCapabilities;
use micold_core::sandbox::{
    Bytes, MilliCpus, SandboxProfile, MIN_MEMORY, MIN_MILLI_CPUS, MIN_PIDS, MIN_STORAGE,
};
use micold_core::session::AiCli;
use micold_core::settings::{DaemonConfig, Settings};
use micold_core::theme::{SystemScheme, ThemePreference};

/// What this feature remembers (feature 028, contract S1).
///
/// The fields keep the names they had as flat members of `app::State`, and the reducers below
/// spell the root's type `crate::app::State` now that `State` here means this struct.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct State {
    /// In-progress Settings edit, present only while the Settings view is shown (feature 006).
    pub settings_draft: Option<SettingsDraft>,
    /// The last light/dark scheme reported by the OS poll (transient, not persisted).
    pub system_scheme: SystemScheme,
    /// How the app chooses its theme (persisted); defaults to following the OS (FR-005).
    pub theme_pref: ThemePreference,
}

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
    /// Which AI CLI a new session runs when nothing is chosen for it (feature 026, FR-003).
    ///
    /// Not a `String`, unlike the two fields above it, and the difference is the point: those hold
    /// what the user *typed*, which may not yet be a valid setting. This is a closed enum picked
    /// from a list, so there is no half-typed state to represent and nothing to validate on save.
    ///
    /// It sits in this section rather than a fifth one because the section's own line says what it
    /// governs — "what each session inherits before the agent starts" — and which agent starts is
    /// the first thing in that sentence.
    pub default_ai_cli: AiCli,
}

/// The Session service section's fields (feature 027, FR-028).
///
/// The sandbox profile is held whole rather than field by field, so that a setting this section
/// does not render yet — the network posture, which arrives with T087 — survives a save instead of
/// being reset to its default by a draft that had never heard of it.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DaemonDraft {
    /// Where the service runs (FR-001).
    pub placement: PlacementKind,
    /// Everything about the sandbox, whether or not it is the selected placement.
    pub profile: SandboxProfile,
    /// The archive to import, as typed. Text rather than the profile's `Option<PathBuf>` for the
    /// same reason the numbers are text: an empty field is a state the user passes through.
    pub image_path: String,
    /// The processor limit, in cores as typed. Empty means *unset* — the runtime's own default —
    /// which is a different intent from any number, and is why the profile holds an `Option`
    /// (rule RB-2).
    pub cpus: String,
    /// The memory limit, in MiB as typed. Empty means unset.
    pub memory_mib: String,
    /// The process-count limit, as typed. Empty means unset.
    pub pids: String,
    /// The writable-storage limit, in MiB as typed. Empty means unset.
    pub storage_mib: String,
    /// What the selected runtime turned out to be able to enforce, when a bring-up has told us.
    ///
    /// `None` is *not yet known*, not *nothing works*: before the first bring-up the application
    /// has never run the probe, and a form that disabled every limit on that basis would be
    /// inventing a restriction. So unknown renders every limit editable, and a limit the runtime
    /// then turns out not to enforce is reported by [`reconcile`] once it runs (FR-015).
    ///
    /// Not persisted and not a setting — a fact about this machine, which is why it is seeded from
    /// the sandbox's state rather than from `Settings`.
    ///
    /// [`reconcile`]: micold_core::sandbox::runtime::reconcile
    pub capabilities: Option<RuntimeCapabilities>,
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
    /// Environment.
    pub default_ai_cli: AiCli,
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
            default_ai_cli: self.default_ai_cli,
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

/// A processor share as the field shows it: cores, with no trailing zeros.
///
/// `MilliCpus(2000)` is "2" and not "2.000". The field is round-trippable — what it shows is what
/// the user would type to reproduce the stored value — and a number padded with zeros the user
/// never typed reads as the form having edited their input.
pub fn cores_text(cpus: MilliCpus) -> String {
    let mut text = format!("{:.3}", f64::from(cpus.0) / 1000.0);
    while text.ends_with('0') {
        text.pop();
    }
    if text.ends_with('.') {
        text.pop();
    }
    text
}

/// One core per hardware thread on a machine far larger than any this will run on.
const MAX_CORES: f64 = 1024.0;
/// 1 TiB, in MiB — past any desktop, and comfortably inside `u64` once multiplied out.
const MAX_MIB: u64 = 1024 * 1024;
/// The kernel's own `pid_max` ceiling on 64-bit Linux.
const MAX_PIDS: u32 = 4_194_304;

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
        profile.budget = self.budget()?;

        Ok(ValidSettings {
            theme: self.appearance.theme,
            scrollback_lines,
            env_include_enabled: self.environment.enabled,
            env_include_script_path: self.environment.script_path.clone(),
            env_include_timeout_secs,
            default_ai_cli: self.environment.default_ai_cli,
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

    /// The four sandbox limits, parsed from what was typed.
    ///
    /// # Why an empty field is not a zero
    ///
    /// Every limit is an `Option` because *unset* and *set to some number* are different intents
    /// that have to round-trip differently (rule RB-2): unset leaves the runtime's own default,
    /// and there is no number that means that. So an empty field parses to `None` rather than
    /// being rejected — it is the way to say "do not bound this" — while a number below the
    /// documented workable minimum is refused with the range that would be accepted (FR-016).
    ///
    /// The maxima are not policy the way the minima are. `MIN_MEMORY` and its siblings are what
    /// the daemon needs to run at all, stated in `micold-core` beside the settings they bound;
    /// these ceilings only keep a typo like `1e12` from overflowing the unit conversion, so they
    /// live here with the form that parses the text.
    fn budget(&self) -> Result<micold_core::sandbox::ResourceBudget, FieldError> {
        Ok(micold_core::sandbox::ResourceBudget {
            cpus_milli: self.cores()?,
            memory_bytes: self.mib(
                &self.daemon.memory_mib,
                FieldId::SettingsMemoryLimit,
                MIN_MEMORY,
                MAX_MIB,
            )?,
            pids: self.count(
                &self.daemon.pids,
                FieldId::SettingsPidLimit,
                MIN_PIDS,
                MAX_PIDS,
            )?,
            storage_bytes: self.mib(
                &self.daemon.storage_mib,
                FieldId::SettingsStorageLimit,
                MIN_STORAGE,
                MAX_MIB,
            )?,
        })
    }

    fn reject(&self, field: FieldId, message: String) -> FieldError {
        FieldError {
            field,
            section: SettingsSection::Daemon,
            message,
        }
    }

    fn cores(&self) -> Result<Option<MilliCpus>, FieldError> {
        let text = self.daemon.cpus.trim();
        if text.is_empty() {
            return Ok(None);
        }
        let min = cores_text(MIN_MILLI_CPUS);
        match text.parse::<f64>() {
            Ok(c) if c.is_finite() && c >= f64::from(MIN_MILLI_CPUS.0) / 1000.0 && c <= MAX_CORES => {
                Ok(Some(MilliCpus((c * 1000.0).round() as u32)))
            }
            Ok(_) => Err(self.reject(
                FieldId::SettingsCpuLimit,
                format!("Enter between {min} and {MAX_CORES:.0} cores, or leave it empty to use the runtime's default."),
            )),
            Err(_) => Err(self.reject(
                FieldId::SettingsCpuLimit,
                "Enter a number of cores, like 2 or 1.5.".to_string(),
            )),
        }
    }

    fn mib(
        &self,
        text: &str,
        field: FieldId,
        min: Bytes,
        max_mib: u64,
    ) -> Result<Option<Bytes>, FieldError> {
        let text = text.trim();
        if text.is_empty() {
            return Ok(None);
        }
        let min_mib = min.as_mib();
        match text.parse::<u64>() {
            Ok(m) if (min_mib..=max_mib).contains(&m) => Ok(Some(Bytes::from_mib(m))),
            Ok(_) => Err(self.reject(
                field,
                format!(
                    "Enter between {min_mib} and {max_mib} MiB, or leave it empty to use the \
                     runtime's default."
                ),
            )),
            Err(_) => Err(self.reject(field, "Enter a whole number of mebibytes.".to_string())),
        }
    }

    fn count(
        &self,
        text: &str,
        field: FieldId,
        min: u32,
        max: u32,
    ) -> Result<Option<u32>, FieldError> {
        let text = text.trim();
        if text.is_empty() {
            return Ok(None);
        }
        match text.parse::<u32>() {
            Ok(n) if (min..=max).contains(&n) => Ok(Some(n)),
            Ok(_) => Err(self.reject(
                field,
                format!(
                    "Enter between {min} and {max} processes, or leave it empty to use the \
                     runtime's default."
                ),
            )),
            Err(_) => Err(self.reject(field, "Enter a whole number of processes.".to_string())),
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
                default_ai_cli: settings.default_ai_cli,
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
                // An unset limit seeds an *empty* field rather than a zero or the word
                // "unlimited": empty is what the user types to unset it, so what they are shown is
                // what they would have to type to reproduce it (rule RB-2).
                cpus: settings
                    .daemon
                    .sandbox
                    .budget
                    .cpus_milli
                    .map(cores_text)
                    .unwrap_or_default(),
                memory_mib: settings
                    .daemon
                    .sandbox
                    .budget
                    .memory_bytes
                    .map(|b| b.as_mib().to_string())
                    .unwrap_or_default(),
                pids: settings
                    .daemon
                    .sandbox
                    .budget
                    .pids
                    .map(|p| p.to_string())
                    .unwrap_or_default(),
                storage_mib: settings
                    .daemon
                    .sandbox
                    .budget
                    .storage_bytes
                    .map(|b| b.as_mib().to_string())
                    .unwrap_or_default(),
                // Seeded by the shell from the sandbox's state, which is where the probe's answer
                // lands — `Settings` has never heard of it and must not learn.
                capabilities: None,
            },
            error: None,
        }
    }

    /// The credentials the user has shared, in a stable order (rule N-2).
    pub fn shared_credentials(&self) -> &BTreeSet<micold_core::sandbox::CredentialShare> {
        &self.daemon.profile.credentials
    }
}

/// Everything the user can do to their settings (feature 028, FR-001).
///
/// # The variants kept their meaning and lost their prefix
///
/// The many that began with `Settings` do not any more — the type says which form (contract M1),
/// so `SettingsScrollbackChanged` is `Msg::ScrollbackChanged`. The theme variants that apply
/// immediately keep their names: `Theme` is not this feature's name, it is which setting, and
/// dropping it would leave a bare `ModeCycled` that says nothing about what mode. `ThemeChanged`
/// beside them is the *draft's* theme picker (feature 027, FR-027) and keeps the word for the
/// same reason.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Msg {
    /// The user selected a theme preference (Follow system / Light / Dark) (FR-007, FR-008).
    /// The shell persists the updated preference afterward.
    ThemePreferenceChanged(ThemePreference),
    /// Cycle the theme mode to the next one (Auto → Light → Dark → Auto) from the toolbar
    /// menu's mode toggle. The shell persists the updated preference; the menu stays open.
    ThemeModeCycled,
    /// The OS light/dark preference poll observed a (changed) scheme (FR-006). Transient;
    /// never persisted. Carries the raw detection outcome — `Err(())` for a transient failure
    /// (e.g. `dark_light::detect()` timing out under CPU load) — rather than an
    /// already-resolved `SystemScheme`, specifically so the periodic poll's `Subscription::map`
    /// closure (`os_theme_poll`, `src/main.rs`) does not need to capture the previous scheme:
    /// iced panics if a subscription's mapping closure captures state, since that breaks the
    /// stable identity it relies on to avoid restarting the underlying timer every frame.
    /// [`system_theme_changed`] applies the same last-known fallback
    /// (`theme::observe_system_scheme`) that used to be baked in at the call site instead.
    SystemThemeChanged(Result<SystemScheme, ()>),
    /// Open the Settings view (from the toolbar menu) (FR-019). The shell seeds the draft with
    /// the current values.
    Opened,
    /// The Settings view moved to another section (feature 027, FR-026).
    SectionShown(SettingsSection),
    /// The Settings theme picker changed (feature 027, FR-027).
    ///
    /// Distinct from [`Msg::ThemePreferenceChanged`], which the app bar's cycle button emits and
    /// which applies immediately. This one edits the draft and takes effect on save, like every
    /// other control on the form.
    ThemeChanged(ThemePreference),
    /// The Settings scrollback field changed.
    ScrollbackChanged(String),
    /// The Settings environment-include enabled checkbox was toggled (feature 011, FR-001).
    EnvIncludeEnabledToggled(bool),
    /// The Settings environment-include script path field changed (FR-002).
    EnvIncludePathChanged(String),
    /// The Settings environment-include timeout field changed (FR-003).
    EnvIncludeTimeoutChanged(String),
    /// The Settings **Default AI CLI** select changed (feature 026, FR-003).
    DefaultAiCliChanged(AiCli),
    /// Where the session service runs (feature 027, FR-001).
    PlacementChanged(PlacementKind),
    /// Which container runtime drives the sandbox (feature 027, FR-021).
    RuntimeChanged(micold_core::sandbox::runtime::RuntimeKind),
    /// How the sandbox image is obtained (feature 027, FR-024).
    ImageKindChanged(micold_core::sandbox::image::ImageSourceKind),
    /// The sandbox image's reference changed (feature 027, FR-024).
    ImageReferenceChanged(String),
    /// The archive an imported image is loaded from changed (feature 027, FR-024a).
    ImagePathChanged(String),
    /// One host credential's share was opted into or out of (feature 027, FR-004c).
    CredentialToggled(micold_core::sandbox::CredentialShare, bool),
    /// Whether sessions outlive the user's sign-out (feature 027, FR-014a).
    SurviveLogoutToggled(bool),
    /// Whether the sandbox may open outbound connections (feature 027, FR-017, FR-018).
    NetworkChanged(micold_core::sandbox::NetworkPosture),
    /// The sandbox's processor limit changed, in cores as typed (feature 027, FR-012).
    ///
    /// Four variants rather than one carrying which limit it is, unlike [`Msg::CredentialToggled`]:
    /// the four credentials are one control repeated over a set, while these four are four
    /// different quantities in three different units, and a single variant would only move the
    /// `match` from here into the reducer.
    CpuLimitChanged(String),
    /// The sandbox's memory limit changed, in MiB as typed (feature 027, FR-013).
    MemoryLimitChanged(String),
    /// The sandbox's process-count limit changed, as typed (feature 027, FR-014).
    PidLimitChanged(String),
    /// The sandbox's writable-storage limit changed, in MiB as typed (feature 027, FR-015).
    StorageLimitChanged(String),
    /// Save the Settings form (validated + persisted by the shell) (FR-020, FR-021).
    Saved,
    /// Dismiss the Settings form without saving (Cancel or Esc).
    Cancelled,
}

/// The pure half of this feature's reducer surface: shape A (contract M2).
///
/// Every arm is here, and every arm is pure. Four of them additionally need an effect — a write
/// to `settings.json`, or the current values read back into the draft — and that half is
/// `shell/settings.rs`'s `update`, which runs the effect and routes the rest here. Splitting by
/// effect rather than by variant is what M2 asks for: nothing about opening the view is
/// duplicated between the two, the shell simply has something extra to do afterwards.
pub fn update(state: &mut crate::app::State, msg: Msg) -> Vec<crate::features::Outcome> {
    match msg {
        Msg::ThemePreferenceChanged(pref) => theme_preference_changed(state, pref),
        Msg::ThemeModeCycled => theme_mode_cycled(state),
        Msg::SystemThemeChanged(detected) => system_theme_changed(state, detected),
        Msg::Opened => opened(state),
        Msg::SectionShown(section) => section_shown(state, section),
        Msg::ThemeChanged(theme) => theme_changed(state, theme),
        Msg::ScrollbackChanged(text) => scrollback_changed(state, text),
        Msg::EnvIncludeEnabledToggled(enabled) => env_include_enabled_toggled(state, enabled),
        Msg::EnvIncludePathChanged(text) => env_include_path_changed(state, text),
        Msg::EnvIncludeTimeoutChanged(text) => env_include_timeout_changed(state, text),
        Msg::DefaultAiCliChanged(which) => default_ai_cli_changed(state, which),
        Msg::PlacementChanged(placement) => placement_changed(state, placement),
        Msg::RuntimeChanged(runtime) => runtime_changed(state, runtime),
        Msg::ImageKindChanged(kind) => image_kind_changed(state, kind),
        Msg::ImageReferenceChanged(text) => image_reference_changed(state, text),
        Msg::ImagePathChanged(text) => image_path_changed(state, text),
        Msg::CredentialToggled(share, shared) => credential_toggled(state, share, shared),
        Msg::SurviveLogoutToggled(survive) => survive_logout_toggled(state, survive),
        Msg::NetworkChanged(posture) => network_changed(state, posture),
        Msg::CpuLimitChanged(text) => cpu_limit_changed(state, text),
        Msg::MemoryLimitChanged(text) => memory_limit_changed(state, text),
        Msg::PidLimitChanged(text) => pid_limit_changed(state, text),
        Msg::StorageLimitChanged(text) => storage_limit_changed(state, text),
        Msg::Saved => saved(state),
        Msg::Cancelled => cancelled(state),
    }
    Vec::new()
}

/// The theme mode was advanced one step (feature 003, FR-005).
///
/// The menu stays open, so repeated clicks cycle.
pub fn theme_mode_cycled(state: &mut crate::app::State) {
    apply_theme(state, state.settings.theme_pref.next());
}

/// A theme preference was chosen outright.
///
/// Pure state change; the shell persists it at the I/O boundary (FR-009).
pub fn theme_preference_changed(state: &mut crate::app::State, pref: ThemePreference) {
    apply_theme(state, pref);
}

/// Apply a theme chosen from the app bar, and carry it into an open Settings form (BUG-001).
///
/// The theme is the one setting with two writers: this menu, which applies immediately, and the
/// Appearance section, which drafts. That was harmless while Settings was a 420dp modal covering
/// the app bar — the menu could not be reached while the form was open. Feature 027 made Settings
/// a full-surface view with the app bar still on screen (FR-026), so both are now reachable at
/// once, and a draft seeded when the view opened still said what the theme was *then*: cycling the
/// menu and pressing Save reverted the theme the user had just chosen and could see applied.
///
/// The draft takes the newer value rather than the form giving up drafting, because Cancel must
/// still discard an Appearance edit. And deliberately **not** via [`edit`]: a choice made outside
/// the form is not the user acting on the form, so it must not clear a validation error they are
/// being asked to fix (FR-029) — that would empty the message and leave the rejected field
/// unexplained.
fn apply_theme(state: &mut crate::app::State, pref: ThemePreference) {
    state.settings.theme_pref = pref;
    if let Some(draft) = &mut state.settings.settings_draft {
        draft.appearance.theme = pref;
    }
}

/// The OS reported its light/dark preference (feature 003).
///
/// `observe_system_scheme` is what decides whether a detection is believed: an OS that answers
/// "unknown" must not overwrite a scheme already observed, or a single unanswered probe would
/// flip the whole UI. The rule lives in core; this arm only records its answer.
pub fn system_theme_changed(
    state: &mut crate::app::State,
    detected: Result<micold_core::theme::SystemScheme, ()>,
) {
    state.settings.system_scheme =
        micold_core::theme::observe_system_scheme(detected, state.settings.system_scheme);
}

/// Settings was opened (feature 006, FR-020; feature 027, FR-026).
///
/// The shell seeds the current values; a draft is ensured here so the reducer path alone is
/// enough to open the view in a test.
pub fn opened(state: &mut crate::app::State) {
    state.clear_for_dialog();
    if state.settings.settings_draft.is_none() {
        state.settings.settings_draft = Some(SettingsDraft::default());
    }
}

/// A section was chosen from the rail (feature 027, FR-026).
///
/// Not an `edit`: moving between sections is navigation, and it must not clear a validation error
/// the user is being asked to act on — FR-029 reports the error *in the section that owns it*, so
/// clearing it on the way there would empty the page they were sent to.
pub fn section_shown(state: &mut crate::app::State, section: SettingsSection) {
    if let Some(draft) = &mut state.settings.settings_draft {
        draft.show(section);
    }
}

/// Appearance: the theme was chosen (feature 027, FR-026).
pub fn theme_changed(state: &mut crate::app::State, theme: ThemePreference) {
    edit(state, |draft| draft.appearance.theme = theme);
}

/// Terminal: the scrollback field was edited.
pub fn scrollback_changed(state: &mut crate::app::State, text: String) {
    edit(state, |draft| draft.terminal.scrollback_lines = text);
}

/// Environment: the include toggle was flipped (feature 011).
pub fn env_include_enabled_toggled(state: &mut crate::app::State, enabled: bool) {
    edit(state, |draft| draft.environment.enabled = enabled);
}

/// Environment: the include script path was edited (feature 011).
pub fn env_include_path_changed(state: &mut crate::app::State, text: String) {
    edit(state, |draft| draft.environment.script_path = text);
}

/// Environment: the include timeout was edited (feature 011).
pub fn env_include_timeout_changed(state: &mut crate::app::State, text: String) {
    edit(state, |draft| draft.environment.timeout_secs = text);
}

/// Environment: the **Default AI CLI** select changed (feature 026, FR-003).
pub fn default_ai_cli_changed(state: &mut crate::app::State, which: AiCli) {
    edit(state, |draft| draft.environment.default_ai_cli = which);
}

/// Session service: where sessions run (feature 027, FR-001).
pub fn placement_changed(state: &mut crate::app::State, placement: PlacementKind) {
    edit(state, |draft| draft.daemon.placement = placement);
}

/// Session service: which container runtime the sandbox uses (feature 027, FR-002).
pub fn runtime_changed(
    state: &mut crate::app::State,
    runtime: micold_core::sandbox::runtime::RuntimeKind,
) {
    edit(state, |draft| draft.daemon.profile.runtime = runtime);
}

/// Session service: where the sandbox image comes from (feature 027, FR-006).
pub fn image_kind_changed(
    state: &mut crate::app::State,
    kind: micold_core::sandbox::image::ImageSourceKind,
) {
    edit(state, |draft| draft.daemon.profile.image.kind = kind);
}

/// Session service: the image's reference (feature 027, FR-006).
pub fn image_reference_changed(state: &mut crate::app::State, text: String) {
    edit(state, |draft| draft.daemon.profile.image.reference = text);
}

/// Session service: the archive an imported image is loaded from (feature 027, FR-006).
pub fn image_path_changed(state: &mut crate::app::State, text: String) {
    edit(state, |draft| draft.daemon.image_path = text);
}

/// Session service: one host credential's share opt-in (feature 027, FR-004c).
pub fn credential_toggled(
    state: &mut crate::app::State,
    share: micold_core::sandbox::CredentialShare,
    shared: bool,
) {
    edit(state, |draft| {
        // A set, so opting in twice is opting in once (rule N-2) and the order the section lists
        // them in is the order it always lists them in.
        if shared {
            draft.daemon.profile.credentials.insert(share);
        } else {
            draft.daemon.profile.credentials.remove(&share);
        }
    });
}

/// Session service: whether sessions outlive the sign-out that started them (feature 027, FR-014).
pub fn survive_logout_toggled(state: &mut crate::app::State, survive: bool) {
    edit(state, |draft| {
        draft.daemon.profile.survive_logout = survive;
    });
}

/// Session service: the sandbox's network posture (feature 027, FR-011).
pub fn network_changed(
    state: &mut crate::app::State,
    posture: micold_core::sandbox::NetworkPosture,
) {
    edit(state, |draft| draft.daemon.profile.network = posture);
}

/// Session service: the processor limit, in cores (feature 027, FR-012).
pub fn cpu_limit_changed(state: &mut crate::app::State, text: String) {
    edit(state, |draft| draft.daemon.cpus = text);
}

/// Session service: the memory limit, in MiB (feature 027, FR-013).
pub fn memory_limit_changed(state: &mut crate::app::State, text: String) {
    edit(state, |draft| draft.daemon.memory_mib = text);
}

/// Session service: the process-count limit (feature 027, FR-014).
pub fn pid_limit_changed(state: &mut crate::app::State, text: String) {
    edit(state, |draft| draft.daemon.pids = text);
}

/// Session service: the writable-storage limit, in MiB (feature 027, FR-015).
pub fn storage_limit_changed(state: &mut crate::app::State, text: String) {
    edit(state, |draft| draft.daemon.storage_mib = text);
}

/// Apply an edit to the open draft, if there is one, and clear the pending error.
///
/// Every field edit did these two things and the second was easy to forget: a stale validation
/// error left beside a field the user has since corrected is the form telling them they are wrong
/// after they have fixed it. One place, so a new field cannot omit it.
fn edit(state: &mut crate::app::State, change: impl FnOnce(&mut SettingsDraft)) {
    if let Some(draft) = &mut state.settings.settings_draft {
        change(draft);
        draft.edited();
    }
}

/// The form was saved (feature 006).
///
/// Validation and persistence happen in the shell; the reducer closes the view.
pub fn saved(state: &mut crate::app::State) {
    state.settings.settings_draft = None;
}

/// The form was dismissed without saving.
pub fn cancelled(state: &mut crate::app::State) {
    state.settings.settings_draft = None;
}
