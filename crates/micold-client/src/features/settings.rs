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
//! Twenty-five transitions in [`Msg`]: the theme's two that apply outside the draft
//! (`ThemePreferenceChanged`, `SystemThemeChanged`), the view's navigation
//! (`Opened`, `SectionShown`, `RailToggled`), the four sections' nineteen field edits, and the two
//! ways out (`Saved`, `Cancelled`).
//!
//! [`update`] routes all of them and is pure (data-model.md §1.1 shape A), but the feature's entry
//! shape is **B**: `main.rs` has one `Message::Settings` arm and it goes to `shell/settings.rs`,
//! because four of them additionally need `settings.json` written or read back
//! (`Opened`, `Saved`, `ThemePreferenceChanged`). The rest reach [`update`]
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

use crate::features::session::{AvailabilitySource, CliAvailability};
use crate::features::window::FieldId;
use crate::overlay::registry::Registered;
use crate::overlay::{DismissalRules, FloatingSurface, SurfaceId};
use micold_core::issue_types::{
    default_mapping, validate_mapping, LabelTypeEntry, MappingErrorKind,
};
use micold_core::mcp::policy::CrossSessionAccess;
use micold_core::naming::ConventionalType;
use micold_core::overlay::Layer;
use micold_core::sandbox::placement::PlacementKind;
use micold_core::sandbox::runtime::RuntimeCapabilities;
use micold_core::sandbox::{
    Bytes, MilliCpus, SandboxProfile, MIN_MEMORY, MIN_MILLI_CPUS, MIN_PIDS, MIN_STORAGE,
};
use micold_core::script_path_check::CheckedScriptPath;
use micold_core::session::AiCli;
use micold_core::settings::{DaemonConfig, Settings};
use micold_core::theme::{SystemScheme, ThemePreference};
use micold_core::typeahead::Direction;

/// What this feature remembers (feature 028, contract S1).
///
/// The fields keep the names they had as flat members of `app::State`, and the reducers below
/// spell the root's type `crate::app::State` now that `State` here means this struct.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct State {
    /// In-progress Settings edit, present only while the Settings view is shown (feature 006).
    pub settings_draft: Option<SettingsDraft>,
    /// Whether the Settings rail is showing icons alone (feature 027, FR-026c).
    ///
    /// Here rather than on [`SettingsDraft`], and that placement is the requirement: FR-026d calls
    /// this view state, not a setting. On the draft it would be reverted by Cancel — so closing the
    /// rail to read a page and then abandoning an edit would reopen it — written to disk on Save,
    /// and subject to FR-029's save-together rule, which would make "the rail is closed" something
    /// the user could fail to save. It outlives the draft and dies with the process.
    pub settings_rail_collapsed: bool,
    /// The last light/dark scheme reported by the OS poll (transient, not persisted).
    pub system_scheme: SystemScheme,
    /// How the app chooses its theme (persisted); defaults to following the OS (FR-005).
    pub theme_pref: ThemePreference,
    /// Where sessions are running **now** (FR-035b; BUG-003).
    ///
    /// Not the draft's `daemon.placement`, which is what the user is *choosing*, and not the
    /// stored one either: after an accepted fallback (FR-035a) the file says container and the
    /// service is on the host, and the honest answer to "where do my sessions run" is the host.
    /// Seeded at boot from the placement the shell resolved and moved only when a save actually
    /// moves the service, so a Cancel cannot revert it — reverting it would mean the note lying
    /// about a container that is still running.
    pub placement_in_force: PlacementKind,
    /// The placement change the user is being asked to confirm, while the question is open
    /// (FR-032; BUG-003). `None` the rest of the time, which is what closes the dialog.
    pub pending_placement: Option<PendingPlacementChange>,
    /// What the latest check of the stored environment-include script path found (spec 035).
    ///
    /// Not a setting and never persisted (FR-010): it describes the file as it is now, and is
    /// re-checked each time Settings opens (FR-009).
    pub script_check: ScriptCheck,
    /// The sequence number of the latest check started. A result for any other number is stale,
    /// so a slow answer about an older path can never overwrite a newer one (spec 035 S4).
    pub script_check_seq: u64,
    /// The latest check a save started and has not yet reported on. Separate from
    /// [`Self::script_check_seq`], which gates only what is shown: a save's result must still be
    /// reported after a newer check has replaced it on the page (spec 035 S5).
    pub script_check_save_seq: Option<u64>,
}

/// Why a script path check was started (spec 035, contracts/settings-indication.md §1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CheckOrigin {
    /// Settings was shown, or another window changed the settings while it was shown.
    Opened,
    /// This window saved Settings.
    Saved,
}

/// Where the check of the stored script path stands (spec 035, data-model.md).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum ScriptCheck {
    /// No check yet, or the path is blank (FR-011).
    #[default]
    Idle,
    /// A check is in flight. `last` is the previous answer, still shown so a re-check does not
    /// blank the page.
    Pending {
        /// The check's sequence number.
        seq: u64,
        /// The previous `Done` answer, if there was one.
        last: Option<CheckedScriptPath>,
    },
    /// The latest applied answer.
    Done(CheckedScriptPath),
}

/// A placement change that has been asked about and not yet answered (FR-032).
///
/// Carries both ends because the dialog names both: "sessions will move from the host to a
/// container" is answerable, "the placement will change" is not. `from` is the placement in force
/// rather than the stored one for the same reason [`State::placement_in_force`] exists.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PendingPlacementChange {
    /// Where sessions run now.
    pub from: PlacementKind,
    /// Where this save would move them.
    pub to: PlacementKind,
}

/// What a save has to do about where the service runs (FR-032a, FR-033a; BUG-003).
///
/// The shape is deliberately `survival_step`'s: that decision — arrange for sessions to outlive a
/// logout, or don't — is the other setting in this form whose value has to be *acted on* rather
/// than written, and it already established the rule this one needs. Act on a change, and only on
/// a change.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlacementStep {
    /// The save leaves sessions where they are. Write it and move nothing.
    Leave,
    /// The save moves the service. Ask first, and write nothing until the answer comes back.
    Confirm {
        /// Where sessions run now.
        from: PlacementKind,
        /// Where this save would move them.
        to: PlacementKind,
    },
}

/// Whether saving `saved` moves a service currently running at `in_force` (FR-032a).
///
/// Pure, and the whole of "only when it actually differs": a user who opens Settings, changes a
/// scrollback size and presses Save must not be asked about the session service, and a user who
/// re-selects the placement they already have has not asked for a restart either.
pub fn placement_step(in_force: PlacementKind, saved: PlacementKind) -> PlacementStep {
    if in_force == saved {
        PlacementStep::Leave
    } else {
        PlacementStep::Confirm {
            from: in_force,
            to: saved,
        }
    }
}

/// The line under the placement select: where sessions run **now** (FR-035b).
///
/// It takes the placement in force and nothing else, and that signature is the fix. The note used
/// to read the draft — the same value the select directly above it already shows — so the instant
/// a user chose *In a container* it told them they were currently in one, about a service still
/// running on the host. A control cannot report the state of the world by echoing the question.
/// What the confirmation tells the user they are about to lose (FR-033).
///
/// Pure and here rather than inside the dialog's view, because it is the only part of that dialog
/// anyone can be wrong about: the count, its plural, and whether a move with nothing running still
/// has something to warn about. A view returning an `Element` can be looked at; a `String` can be
/// asserted.
///
/// `live_sessions` is how many sessions the application currently knows are running — all of them,
/// across every project, because the service that hosts them is one process and the move stops it.
pub fn placement_move_consequence(
    from: PlacementKind,
    to: PlacementKind,
    live_sessions: usize,
) -> String {
    let ends = match live_sessions {
        // Nothing running, so nothing is lost, and saying "0 sessions stop" would be a warning
        // about an absence — the kind that teaches people to click through dialogs unread.
        0 => String::new(),
        1 => " Your 1 session stops and becomes resumable.".to_string(),
        n => format!(" Your {n} sessions stop and become resumable."),
    };

    format!(
        "Sessions run {} now. Saving stops the session service and starts it again {}.{ends} Your \
         projects and their history are untouched.",
        from.label().to_lowercase(),
        to.label().to_lowercase(),
    )
}

pub fn placement_note(in_force: PlacementKind) -> String {
    format!("Currently {}.", in_force.label().to_lowercase())
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
    /// How an issue's labels choose a worktree's type (feature 034, FR-018).
    GithubIssues,
}

impl SettingsSection {
    /// Every section, in rail order.
    pub const ALL: &'static [SettingsSection] = &[
        SettingsSection::Appearance,
        SettingsSection::Terminal,
        SettingsSection::Environment,
        SettingsSection::Daemon,
        SettingsSection::GithubIssues,
    ];

    /// The section's name, as the rail shows it.
    pub fn label(self) -> &'static str {
        match self {
            SettingsSection::Appearance => "Appearance",
            SettingsSection::Terminal => "Terminal",
            SettingsSection::Environment => "Environment",
            SettingsSection::Daemon => "Session service",
            SettingsSection::GithubIssues => "GitHub issues",
        }
    }

    /// The section's icon, as the rail shows it beside — or instead of — the name (FR-026b).
    ///
    /// On the section rather than on the rail, for the same reason [`Self::label`] is: the rail is
    /// one presentation of these, and a collapsed rail is a second. A glyph chosen inside a view
    /// would be that view's, and the next surface to list sections would pick its own.
    pub fn icon(self) -> crate::icons::Icon {
        use crate::icons::Icon;
        match self {
            // The theme is the section's whole content today, and the sun is the glyph the app bar
            // used for it before the duplicate was removed (FR-026e).
            SettingsSection::Appearance => Icon::LightMode,
            SettingsSection::Terminal => Icon::RegularTerminal,
            // The environment section is where the default agent is chosen, which is the part of it
            // a user is looking for when they come here.
            SettingsSection::Environment => Icon::AiCli,
            SettingsSection::Daemon => Icon::SessionService,
            SettingsSection::GithubIssues => Icon::IssueMapping,
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
    /// Whether a Pi session is started with this application's activity component loaded
    /// (feature 029, FR-012e). One application-wide switch — there is no per-project or
    /// per-session form of it. Holds the value only; the default-on comes from `Settings`.
    pub pi_activity_component: bool,
    /// Whether new sessions are bound to the service's tool server (feature 034, FR-004).
    /// Application-wide and service-owned like the Pi switch above it; the default-on comes from
    /// `Settings`.
    pub tool_server_enabled: bool,
    /// Whether agents may read and type into other sessions (feature 034, FR-016). A closed choice
    /// of three values, like the default CLI above: nothing to validate on save.
    pub cross_session_access: CrossSessionAccess,
}

/// The label-to-type mapping the draft carries (feature 034, FR-016).
///
/// Seeded from what is stored and written back whole by a save, so that a save changing anything
/// else keeps the mapping. The GitHub issues section edits it (US3); labels are held as typed, and
/// only [`SettingsDraft::validate`] judges them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GithubDraft {
    /// The mapping, in order.
    pub entries: Vec<LabelTypeEntry>,
}

impl Default for GithubDraft {
    /// The default table, as a never-edited mapping reads (FR-021).
    fn default() -> Self {
        Self {
            entries: default_mapping(),
        }
    }
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
    /// The host path of a shared sign-in that the running sandbox's container does not mount, when
    /// the bring-up reported one (FR-004g, BUG-008).
    ///
    /// Not persisted and not a setting, for the same reason as [`Self::capabilities`]: it is a fact
    /// about the sandbox that is running, seeded from its state when the page opens.
    pub unshared_sign_in: Option<String>,
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
    /// Environment.
    pub pi_activity_component: bool,
    /// Environment.
    pub tool_server_enabled: bool,
    /// Environment.
    pub cross_session_access: CrossSessionAccess,
    /// Session service.
    pub daemon: DaemonConfig,
    /// GitHub issues: the label-to-type mapping (feature 034).
    pub issue_label_types: Vec<LabelTypeEntry>,
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
            pi_activity_component: self.pi_activity_component,
            tool_server_enabled: self.tool_server_enabled,
            cross_session_access: self.cross_session_access,
            daemon: self.daemon,
            issue_label_types: self.issue_label_types,
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
    /// GitHub issues (feature 034).
    pub github: GithubDraft,
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
        self.mapping()?;

        Ok(ValidSettings {
            theme: self.appearance.theme,
            scrollback_lines,
            env_include_enabled: self.environment.enabled,
            env_include_script_path: self.environment.script_path.clone(),
            env_include_timeout_secs,
            default_ai_cli: self.environment.default_ai_cli,
            pi_activity_component: self.environment.pi_activity_component,
            tool_server_enabled: self.environment.tool_server_enabled,
            cross_session_access: self.environment.cross_session_access,
            daemon: DaemonConfig {
                placement: self.daemon.placement,
                sandbox: profile,
            },
            // Stored trimmed: the spaces around a label are not part of it (they are ignored when it
            // is matched and compared), so they are not written back.
            issue_label_types: self
                .github
                .entries
                .iter()
                .map(|entry| LabelTypeEntry {
                    label: entry.label.trim().to_string(),
                    type_: entry.type_,
                })
                .collect(),
        })
    }

    /// The label-to-type mapping, checked last because its section is last in the rail (FR-019).
    fn mapping(&self) -> Result<(), FieldError> {
        validate_mapping(&self.github.entries).map_err(|error| FieldError {
            field: FieldId::IssueMappingLabel(error.index),
            section: SettingsSection::GithubIssues,
            message: match error.kind {
                MappingErrorKind::Blank => "Enter a label, or remove this entry.".to_string(),
                // Named by the label it repeats: the rows carry no numbers to point at.
                MappingErrorKind::Duplicate { of } => format!(
                    "“{}” is already mapped above.",
                    self.github.entries[of].label.trim()
                ),
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
                pi_activity_component: settings.pi_activity_component,
                tool_server_enabled: settings.tool_server_enabled,
                cross_session_access: settings.cross_session_access,
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
                unshared_sign_in: None,
            },
            github: GithubDraft {
                entries: settings.issue_label_types.clone(),
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
    /// The theme preference was set from outside the Settings form (FR-007, FR-008). The shell
    /// persists the updated preference afterward.
    ///
    /// No control emits it since feature 027 removed the app bar's theme cycle (FR-026e), and it is
    /// kept deliberately: it is the *contract* for a live theme change — apply it, and carry it into
    /// an open draft rather than letting Save write the draft's stale copy back (BUG-001). Deleting
    /// it would delete that rule along with it, and the rule is what a second writer would need to
    /// obey the day one is added again.
    ThemePreferenceChanged(ThemePreference),
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
    /// Collapse the Settings rail to its icons, or reopen it (feature 027, FR-026c/d).
    ///
    /// Deliberately not a `SettingsDraft` field: it is view state, so Cancel must not revert it and
    /// Save must not write it to disk. See [`State::settings_rail_collapsed`].
    RailToggled,
    /// The Settings theme picker changed (feature 027, FR-027).
    ///
    /// Distinct from [`Msg::ThemePreferenceChanged`], which the app bar's cycle button emits and
    /// which applies immediately from outside the form. This one edits the draft and takes effect on save, like every
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
    /// The Settings **Pi activity component** switch was toggled (feature 029, FR-012e).
    PiActivityComponentToggled(bool),
    /// The Settings **Let AI sessions manage worktrees and sessions** switch was toggled
    /// (feature 034, FR-004).
    ToolServerToggled(bool),
    /// The Settings **Let agents read and type into other sessions** select changed
    /// (feature 034, FR-016).
    CrossSessionAccessChanged(CrossSessionAccess),
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
    /// Saving would move sessions, so the user is asked first (feature 027, FR-032; BUG-003).
    ///
    /// Raised by the shell rather than by a control: whether a save is a *change* is a comparison
    /// against the placement in force, and the save path is the only thing holding both sides of
    /// it. Everything about it is pure from here on.
    PlacementChangeRequested {
        /// Where sessions run now.
        from: PlacementKind,
        /// Where the save would move them.
        to: PlacementKind,
    },
    /// The user agreed to move the service (feature 027, FR-033a; BUG-003). The shell performs the
    /// deferred save and restarts the service in the new placement.
    PlacementChangeConfirmed,
    /// The user declined the move, or dismissed the question (FR-032b; BUG-003).
    ///
    /// The whole save goes with it: the surface stays open and the draft keeps every edit, because
    /// a declined restart is not a declined form.
    PlacementChangeCancelled,
    /// The service is now running here (FR-035b; BUG-003).
    ///
    /// Reported by the shell after a move it has actually performed — the save path, and the
    /// accepted fallback of FR-035a, which moves sessions to the host without any save at all.
    /// Both have to reach the same field, which is why this is a message rather than an assignment
    /// inside the one of them that happened to be written first.
    PlacementMoved(PlacementKind),
    /// A mapping entry's label was edited (feature 034, FR-018).
    IssueMappingLabelChanged(usize, String),
    /// A mapping entry's type was picked (feature 034, FR-018).
    IssueMappingTypeChanged(usize, ConventionalType),
    /// **Add entry**: append a blank label typed `feat` (feature 034, FR-018).
    IssueMappingAdded,
    /// A mapping entry's delete button (feature 034, FR-018).
    IssueMappingRemoved(usize),
    /// A mapping entry's move up (`Prev`) or move down (`Next`) button (feature 034, FR-017).
    IssueMappingMoved(usize, Direction),
    /// **Restore defaults** (feature 034, FR-018, FR-021).
    IssueMappingDefaultsRestored,
    /// Save the Settings form (validated + persisted by the shell) (FR-020, FR-021).
    Saved,
    /// Dismiss the Settings form without saving (Cancel or Esc).
    Cancelled,
    /// A check of the stored script path is starting (spec 035 FR-009). Raised by the shell,
    /// which then runs the check off the UI thread.
    ScriptPathCheckStarted {
        /// Why it was started.
        origin: CheckOrigin,
        /// The stored path it checks.
        path: String,
        /// The stored enabled flag at the start (research R8).
        enabled: bool,
    },
    /// A check of the stored script path finished (spec 035 FR-009).
    ScriptPathChecked {
        /// The sequence number [`Msg::ScriptPathCheckStarted`] gave it.
        seq: u64,
        /// Why it was started.
        origin: CheckOrigin,
        /// What was found, or `None` for a blank path.
        result: Option<CheckedScriptPath>,
    },
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
        Msg::SystemThemeChanged(detected) => system_theme_changed(state, detected),
        Msg::Opened => opened(state),
        Msg::SectionShown(section) => section_shown(state, section),
        Msg::RailToggled => rail_toggled(state),
        Msg::ThemeChanged(theme) => theme_changed(state, theme),
        Msg::ScrollbackChanged(text) => scrollback_changed(state, text),
        Msg::EnvIncludeEnabledToggled(enabled) => env_include_enabled_toggled(state, enabled),
        Msg::EnvIncludePathChanged(text) => env_include_path_changed(state, text),
        Msg::EnvIncludeTimeoutChanged(text) => env_include_timeout_changed(state, text),
        Msg::DefaultAiCliChanged(which) => default_ai_cli_changed(state, which),
        Msg::PiActivityComponentToggled(on) => pi_activity_component_toggled(state, on),
        Msg::ToolServerToggled(on) => tool_server_toggled(state, on),
        Msg::CrossSessionAccessChanged(access) => cross_session_access_changed(state, access),
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
        Msg::PlacementChangeRequested { from, to } => placement_change_requested(state, from, to),
        Msg::PlacementChangeConfirmed => placement_change_confirmed(state),
        Msg::PlacementChangeCancelled => placement_change_cancelled(state),
        Msg::PlacementMoved(kind) => placement_in_force_changed(state, kind),
        Msg::IssueMappingLabelChanged(index, label) => edit_mapping(state, |entries| {
            if let Some(entry) = entries.get_mut(index) {
                entry.label = label;
            }
        }),
        Msg::IssueMappingTypeChanged(index, type_) => edit_mapping(state, |entries| {
            if let Some(entry) = entries.get_mut(index) {
                entry.type_ = type_;
            }
        }),
        Msg::IssueMappingAdded => edit_mapping(state, |entries| {
            entries.push(LabelTypeEntry {
                label: String::new(),
                type_: ConventionalType::Feat,
            })
        }),
        Msg::IssueMappingRemoved(index) => edit_mapping(state, |entries| {
            if index < entries.len() {
                entries.remove(index);
            }
        }),
        Msg::IssueMappingMoved(index, direction) => {
            edit_mapping(state, |entries| move_entry(entries, index, direction))
        }
        Msg::IssueMappingDefaultsRestored => {
            edit_mapping(state, |entries| *entries = default_mapping())
        }
        Msg::Saved => saved(state),
        Msg::Cancelled => cancelled(state),
        Msg::ScriptPathCheckStarted {
            origin,
            path,
            enabled,
        } => script_path_check_started(state, origin, &path, enabled),
        Msg::ScriptPathChecked {
            seq,
            origin,
            result,
        } => return script_path_checked(state, seq, origin, result),
    }
    Vec::new()
}

/// The theme mode was advanced one step (feature 003, FR-005).
/// A theme preference was set from outside the Settings form, and applies immediately.
///
/// Pure state change; the shell persists it at the I/O boundary (FR-009).
///
/// # Why this survives the control that used it (BUG-001)
///
/// The theme was the one setting with two writers: the app bar's cycle, which applied at once, and
/// the Appearance section, which drafts. Harmless while Settings was a 420dp modal covering the
/// bar — the menu could not be reached while the form was open. FR-026 made Settings a
/// full-surface view with the bar still on screen, so both became reachable at once, and a draft
/// seeded when the view opened still said what the theme was *then*: cycling the menu and pressing
/// Save reverted the theme the user had just chosen and could see applied.
///
/// FR-026e has since removed that second control, which dissolves the condition. This stays as the
/// rule any future one must obey: apply, and carry the newer value into an open draft — rather
/// than the form giving up drafting, because Cancel must still discard an Appearance edit. And
/// deliberately **not** via [`edit`]: a choice made outside the form is not the user acting on the
/// form, so it must not clear a validation error they are being asked to fix (FR-029) — that would
/// empty the message and leave the rejected field unexplained.
pub fn theme_preference_changed(state: &mut crate::app::State, pref: ThemePreference) {
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

/// The Settings rail was collapsed to its icons, or reopened (feature 027, FR-026c/d).
///
/// Not routed through [`edit`], and not touching the draft at all: FR-026d makes this view state,
/// so it must not mark the form edited, must not clear a validation error the user is being asked
/// to fix, and must survive both Save and Cancel. See [`State::settings_rail_collapsed`].
///
pub fn rail_toggled(state: &mut crate::app::State) {
    state.settings.settings_rail_collapsed = !state.settings.settings_rail_collapsed;
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

/// Environment: the **Pi activity component** switch was toggled (feature 029, FR-012e).
pub fn pi_activity_component_toggled(state: &mut crate::app::State, on: bool) {
    edit(state, |draft| draft.environment.pi_activity_component = on);
}

/// Environment: bind new sessions to the service's tool server (feature 034, FR-004).
pub fn tool_server_toggled(state: &mut crate::app::State, on: bool) {
    edit(state, |draft| draft.environment.tool_server_enabled = on);
}

/// Environment: whether agents may read and type into other sessions (feature 034, FR-016).
pub fn cross_session_access_changed(state: &mut crate::app::State, access: CrossSessionAccess) {
    edit(state, |draft| {
        draft.environment.cross_session_access = access
    });
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

/// GitHub issues: one edit to the mapping (feature 034, FR-018). An index past the end — a
/// message from a row that is gone — changes nothing.
fn edit_mapping(state: &mut crate::app::State, change: impl FnOnce(&mut Vec<LabelTypeEntry>)) {
    edit(state, |draft| change(&mut draft.github.entries));
}

/// Swap the entry at `index` with its neighbour: `Prev` is up, `Next` is down (FR-017). The first
/// entry cannot move up and the last cannot move down.
fn move_entry(entries: &mut [LabelTypeEntry], index: usize, direction: Direction) {
    let other = match direction {
        Direction::Prev => index.checked_sub(1),
        Direction::Next => index.checked_add(1),
    };
    if let Some(other) = other.filter(|o| *o < entries.len() && index < entries.len()) {
        entries.swap(index, other);
    }
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
///
/// The pending question goes with the draft. It cannot normally outlive one — a save that asks
/// does not reach here until the answer comes back — but a question about a form that no longer
/// exists is a dialog with nothing behind it, and leaving it representable is how it would appear.
pub fn saved(state: &mut crate::app::State) {
    state.settings.settings_draft = None;
    state.settings.pending_placement = None;
}

/// A check of the stored script path is starting (spec 035, contract S1).
///
/// It takes the next sequence number, so only its own answer will be shown, and keeps the previous
/// answer on the page meanwhile: a re-check that blanked the notice for the moment it ran would
/// make the page flicker on every open. A save's check is also marked as the one to report on.
///
/// The previous answer is kept only while it is about the same stored path and enabled flag: after
/// another window's save it would describe what is no longer stored.
pub fn script_path_check_started(
    state: &mut crate::app::State,
    origin: CheckOrigin,
    path: &str,
    enabled: bool,
) {
    let settings = &mut state.settings;
    settings.script_check_seq += 1;
    let seq = settings.script_check_seq;
    let last = match std::mem::take(&mut settings.script_check) {
        ScriptCheck::Done(checked) => Some(checked),
        ScriptCheck::Pending { last, .. } => last,
        ScriptCheck::Idle => None,
    }
    .filter(|checked| checked.path == path && checked.enabled == enabled);
    settings.script_check = ScriptCheck::Pending { seq, last };
    if origin == CheckOrigin::Saved {
        settings.script_check_save_seq = Some(seq);
    }
}

/// A check of the stored script path finished (spec 035, contracts S2–S7).
///
/// Shown only if it is the latest check started; an older one's answer is about a path the user
/// may since have changed (S4). `None` is a blank path, which has nothing to show (FR-011).
///
/// A save's own check also answers for that save (S5–S7): Save closed Settings, so a path it left
/// missing or unreadable is reported by a notification, whether or not a newer open has since taken
/// over the page. Only the latest save's check reports, and only once.
pub fn script_path_checked(
    state: &mut crate::app::State,
    seq: u64,
    origin: CheckOrigin,
    result: Option<CheckedScriptPath>,
) -> Vec<crate::features::Outcome> {
    let settings = &mut state.settings;
    let reports = origin == CheckOrigin::Saved && settings.script_check_save_seq == Some(seq);
    if reports {
        settings.script_check_save_seq = None;
    }
    let notice = result
        .as_ref()
        .filter(|_| reports)
        .and_then(save_notice)
        .map(crate::features::notifications::info);
    if seq == settings.script_check_seq {
        settings.script_check = match result {
            Some(checked) => ScriptCheck::Done(checked),
            None => ScriptCheck::Idle,
        };
    }
    notice.into_iter().collect()
}

/// The notification a save posts when it leaves a path that is not a readable file (spec 035
/// FR-004, research R6), or `None` when the check found nothing to report.
///
/// There is no "Settings saved." prefix: the notice is posted for the path, not for the write.
fn save_notice(checked: &CheckedScriptPath) -> Option<String> {
    use micold_core::script_path_check::ScriptPathState;
    let path = &checked.path;
    match checked.state {
        ScriptPathState::NotFound { tilde: false } => Some(format!(
            "The environment-include script was not found: {path}"
        )),
        ScriptPathState::NotFound { tilde: true } => Some(format!(
            "The environment-include script was not found: {path} \
             (~ is not expanded; use a full path)"
        )),
        ScriptPathState::NotReadable => Some(format!(
            "The environment-include script is not a readable file: {path}"
        )),
        ScriptPathState::Present | ScriptPathState::Relative | ScriptPathState::Unchecked => None,
    }
}

/// The form was dismissed without saving.
pub fn cancelled(state: &mut crate::app::State) {
    state.settings.settings_draft = None;
    state.settings.pending_placement = None;
}

// --- Moving the service the form is about (BUG-003 — FR-032, FR-032b, FR-033a, FR-035b) ---

/// The save wants to move sessions; ask before anything is applied (FR-032).
///
/// Nothing else happens here, and that is the requirement rather than an omission: FR-032b makes
/// the confirmation a gate on the *whole* save, so the draft is untouched, the view stays open
/// behind the dialog, and not one field has been written when this returns.
pub fn placement_change_requested(
    state: &mut crate::app::State,
    from: PlacementKind,
    to: PlacementKind,
) {
    state.settings.pending_placement = Some(PendingPlacementChange { from, to });
}

/// The user agreed to the move (FR-033a).
///
/// The reducer's half is to stop asking. Performing the deferred save and restarting the service
/// in the new placement is an effect, so it belongs to `shell/persist.rs`, which reads the pending
/// change before dispatching this.
pub fn placement_change_confirmed(state: &mut crate::app::State) {
    state.settings.pending_placement = None;
}

/// The user declined the move (FR-032b).
///
/// Only the question is dropped. The draft is deliberately *not* reverted to the placement in
/// force: the user chose a placement and then declined to apply it now, which is a decision to
/// reconsider rather than an edit to undo, and silently resetting the select would leave them
/// looking at a form that disagrees with what they just did.
pub fn placement_change_cancelled(state: &mut crate::app::State) {
    state.settings.pending_placement = None;
}

/// The service is now running in `kind` (FR-035b).
///
/// Reported by the shell once a move has actually been applied — not when it is chosen, and not
/// when it is saved. Everything that tells the user where their sessions are reads this.
pub fn placement_in_force_changed(state: &mut crate::app::State, kind: PlacementKind) {
    state.settings.placement_in_force = kind;
}

/// The confirm-move dialog, as a floating surface (FR-032; BUG-003).
///
/// The first surface this feature registers since Settings stopped being one (FR-026). That is not
/// a reversal: Settings is a *view* because it is a destination the user navigates to, and this is
/// a dialog because it is a question about an action already taken, with no way past it but an
/// answer. Being a registered surface is what puts it above the view in the z-order and gives
/// Escape somewhere to go that is not "close the form".
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ConfirmPlacementDialog;

impl FloatingSurface for ConfirmPlacementDialog {
    fn id(&self) -> SurfaceId {
        SurfaceId::new("confirm_placement")
    }

    fn layer(&self) -> Layer {
        Layer::Dialog
    }

    fn dismissal(&self) -> DismissalRules {
        // Dismissing is declining, and declining leaves the whole save unapplied (FR-032b) — so
        // the scrim and Escape both reach the same reducer the Cancel button does, and none of the
        // three can leave a half-applied save behind.
        DismissalRules::for_layer(Layer::Dialog)
            .cancelled_by(crate::app::Message::Settings(Msg::PlacementChangeCancelled))
    }
}

impl Registered for ConfirmPlacementDialog {
    fn open_in(state: &crate::app::State) -> Option<Self> {
        state
            .settings
            .pending_placement
            .map(|_| ConfirmPlacementDialog)
    }
}

// --- What the Environment page says about the script path (spec 035, FR-002, FR-005) ---

/// One line under the Environment page's timeout field, in page order (spec 035).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NoticeLine {
    /// A warning, rendered through `ui::settings::caution`.
    Caution(String),
    /// An explanation, rendered through `ui::settings::note`.
    Note(String),
}

/// The feature is off, and says what that means for a path that names nothing (spec 035, OFF).
const NOTICE_OFF: &str = "Environment include is off, so no script is sourced. Turning it on \
                          will not source one until this path names a readable file.";
/// The feature is on, and cannot source a script until the path names a readable file (ON).
const NOTICE_ON: &str = "Environment include is on, but the script cannot be sourced until this \
                         path names a readable file.";
/// `~` is taken literally, as resolution takes it (TILDE).
const NOTICE_TILDE: &str = "~ is not expanded. Use a full path.";
/// A relative path is not checked (REL).
const NOTICE_RELATIVE: &str =
    "Relative path: whether the script is found depends on each session's directory.";
/// The check had no answer within [`SCRIPT_PATH_CHECK_BOUND`] (HUNG).
///
/// [`SCRIPT_PATH_CHECK_BOUND`]: micold_core::script_path_check::SCRIPT_PATH_CHECK_BOUND
const NOTICE_HUNG: &str =
    "No answer within 2 seconds. The file may be on a drive that is not responding.";
// NOTICE_HUNG states the bound in words; a change to the bound must change the sentence too.
const _: () = assert!(micold_core::script_path_check::SCRIPT_PATH_CHECK_BOUND.as_secs() == 2);

/// Every line shown below the Environment page's timeout field (spec 035,
/// contracts/settings-indication.md §2): what the check found about the stored path, then 011's
/// note about the last resolution attempt.
///
/// Pure, beside [`missing_cli_notice`], for the reason that one is: the view can be looked at, and
/// these lines can be asserted.
///
/// The same path reads the same whether the feature is on or off (SC-003); only the note about its
/// effect on sessions changes. With the feature on, 011's "Script not found" is merged into the
/// path's caution rather than repeated (FR-005).
pub fn script_path_notice(
    check: &ScriptCheck,
    last: &micold_core::env_include::EnvIncludeOutcome,
) -> Vec<NoticeLine> {
    use micold_core::env_include::EnvIncludeOutcome;
    use micold_core::script_path_check::ScriptPathState;

    let checked = match check {
        ScriptCheck::Done(checked) => checked,
        // A re-check keeps showing the previous answer, so the notice does not blank.
        ScriptCheck::Pending {
            last: Some(checked),
            ..
        } => checked,
        ScriptCheck::Idle | ScriptCheck::Pending { last: None, .. } => {
            return lines_011(last, None)
        }
    };
    let path = &checked.path;
    let last_011 = || lines_011(last, Some(path));
    // What a path that names no readable file means for sessions, in the feature's state.
    let effect = || {
        NoticeLine::Note(
            if checked.enabled {
                NOTICE_ON
            } else {
                NOTICE_OFF
            }
            .to_string(),
        )
    };
    // 011's lines follow the path's with the feature on, except a `MissingScript` line, which the
    // path's caution already says (N3, N7).
    let after_on = || {
        if checked.enabled && *last != EnvIncludeOutcome::MissingScript {
            last_011()
        } else {
            Vec::new()
        }
    };
    match checked.state {
        ScriptPathState::NotFound { tilde } => {
            let mut lines = vec![NoticeLine::Caution(format!("Script not found: {path}"))];
            if tilde {
                lines.push(NoticeLine::Note(NOTICE_TILDE.to_string()));
            }
            lines.push(effect());
            lines.extend(after_on());
            lines
        }
        ScriptPathState::NotReadable => {
            let mut lines = vec![
                NoticeLine::Caution(format!("Not a readable file: {path}")),
                effect(),
            ];
            lines.extend(after_on());
            lines
        }
        ScriptPathState::Relative => {
            let mut lines = vec![NoticeLine::Note(NOTICE_RELATIVE.to_string())];
            lines.extend(last_011());
            lines
        }
        ScriptPathState::Unchecked => {
            let mut lines = vec![
                NoticeLine::Caution(format!("Couldn't check the script path: {path}")),
                NoticeLine::Note(NOTICE_HUNG.to_string()),
            ];
            lines.extend(last_011());
            lines
        }
        // FR-014: the file is there now, but the attempt in force could not find it.
        ScriptPathState::Present
            if checked.enabled && *last == EnvIncludeOutcome::MissingScript =>
        {
            vec![
                NoticeLine::Caution("The last attempt could not find the script".to_string()),
                NoticeLine::Note(format!(
                    "{path} exists now. Save Settings or restart a session to source it."
                )),
            ]
        }
        ScriptPathState::Present => last_011(),
    }
}

/// Feature 011's lines for the most recent resolution attempt (011 FR-012/FR-013): the failure
/// category, then its diagnostic when there is one. Nothing when it succeeded or the feature is
/// off. `MissingScript` names the path when a check has named one (spec 035 FR-005).
fn lines_011(
    outcome: &micold_core::env_include::EnvIncludeOutcome,
    path: Option<&str>,
) -> Vec<NoticeLine> {
    use micold_core::env_include::EnvIncludeOutcome;

    let (category, diagnostic) = match outcome {
        EnvIncludeOutcome::Disabled | EnvIncludeOutcome::Success => return Vec::new(),
        EnvIncludeOutcome::MissingScript => {
            let category = match path {
                Some(path) => format!("Script not found: {path}"),
                None => "Script not found".to_string(),
            };
            return vec![NoticeLine::Caution(category)];
        }
        EnvIncludeOutcome::NonZeroExit { diagnostic, .. } => {
            ("Exited with an error", &**diagnostic)
        }
        EnvIncludeOutcome::TimedOut { diagnostic } => ("Timed out", &**diagnostic),
    };
    let mut lines = vec![NoticeLine::Caution(category.to_string())];
    if !diagnostic.is_empty() {
        lines.push(NoticeLine::Note(diagnostic.to_string()));
    }
    lines
}

// --- Where a CLI is missing, and what to say about it (feature 027, FR-023b) ---

/// The sentence shown where an image is chosen and where a CLI is chosen, when the place sessions
/// run is missing one (FR-023b). `None` when there is nothing to say.
///
/// Two of the three `None` cases are the interesting ones:
///
/// - **the service has not answered yet.** Not "nothing is available" — the app has not asked, or
///   the reply is in flight. Saying anything here would be a guess, and a guess that names a
///   *specific* CLI as missing is worse than silence.
/// - **everything is present.** The absence of a notice is the whole of "this is fine"; a green
///   "all CLIs present" line is a second thing to read on every visit to a form that is not about
///   AI CLIs.
///
/// The sentence never presents this as the application failing. It is a fact about the machine
/// sessions run on, phrased as what that machine would have to provide, because that is where the
/// user can act. FR-023b is explicit that it belongs *here*, at the two points of choice, and not
/// at session start — by then the user has committed to something the app already knew would not
/// work.
pub fn missing_cli_notice(availability: Option<&CliAvailability>) -> Option<String> {
    let availability = availability?;
    let missing = availability.missing();
    let names = name_list(&missing)?;
    let verb = if missing.len() == 1 {
        "isn't"
    } else {
        "aren't"
    };
    Some(match &availability.source {
        AvailabilitySource::Image(reference) => format!(
            "{names} {verb} in {reference}. Sessions run in that image, so it has to provide any \
             AI CLI you want to use."
        ),
        AvailabilitySource::ThisComputer => {
            format!("{names} {verb} installed on this computer, which is where sessions run.")
        }
    })
}

/// "Claude Code", "Claude Code and GitHub Copilot", "a, b and c" — `None` for an empty list.
///
/// Written out rather than `join(", ")` because this goes in a sentence, and a comma-separated
/// list reads as a field value rather than as prose.
fn name_list(clis: &[AiCli]) -> Option<String> {
    let (last, rest) = clis.split_last()?;
    if rest.is_empty() {
        return Some(last.to_string());
    }
    let leading: Vec<String> = rest.iter().map(ToString::to_string).collect();
    Some(format!("{} and {last}", leading.join(", ")))
}
