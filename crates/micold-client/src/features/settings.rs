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

use std::path::PathBuf;

use crate::features::session::CliAvailability;
use crate::features::window::FieldId;
use micold_core::attention::{NotificationKind, NotificationKinds};
use micold_core::cli_reason::{explain, AttemptDir, Explanation};
use micold_core::issue_types::{
    default_mapping, validate_mapping, LabelTypeEntry, MappingErrorKind,
};
use micold_core::mcp::policy::CrossSessionAccess;
use micold_core::naming::ConventionalType;
use micold_core::required::{RequiredMarks, Requirement};
use micold_core::sandbox::image::{ImageChoice, ImageSourceProblem, OwnImage, OwnImageDraft};
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
    /// What this window knows of each daemon's connection (feature 491): its state and, while
    /// connected, the handle that sends to it. One daemon failing changes no other's entry; ops are
    /// routed by binding and nothing falls back to another daemon. Rows read their daemon status
    /// from here, so a state change reaches them in the reducer step that applied it (SC-002).
    pub links: crate::links::DaemonLinks,
    /// The container runtime is not installed on this machine (set by the sandbox probe).
    pub runtime_missing: bool,
    /// In-progress Settings edit, present only while the Settings view is shown (feature 006).
    pub settings_draft: Option<SettingsDraft>,
    /// Whether the Jira integration is on, as the service last reported it (feature 533, FR-022).
    /// Every Jira control and request is gated on it through `micold_core::jira::jira_visible`.
    pub jira_enabled: bool,
    /// Each project's Jira connection, as the service last reported it (feature 533, FR-022).
    pub jira_connections: std::collections::BTreeMap<String, micold_core::jira::JiraConnection>,
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
    /// The daemon registry as last stored (feature 491). Seeded at boot from the settings file and
    /// re-read after a save; a worktree's binding resolves against it.
    pub daemons: micold_core::daemons::DaemonRegistry,
    /// The daemon a worktree without a stored binding runs on (feature 491, FR-017).
    pub legacy_default_daemon: Option<micold_core::daemons::DaemonId>,
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
    /// Every session directory whose environment-include resolution by the service failed, as the
    /// latest catalog snapshot the client applied reported it (011 FR-022, BUG-454). Replaced, not
    /// merged, on each snapshot, so a directory the service stopped reporting disappears here too.
    /// The Environment page lists them after the note on the last resolution.
    pub env_include_failures: Vec<micold_core::protocol::messages::EnvIncludeFailure>,
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
    /// The configured daemons: their state, and add, edit and remove (feature 491, FR-010).
    Daemons,
    /// How an issue's labels choose a worktree's type (feature 034, FR-018).
    GithubIssues,
    /// The Jira integration's switch and the open project's connection (feature 533, FR-002).
    Jira,
}

impl SettingsSection {
    /// Every section, in rail order.
    pub const ALL: &'static [SettingsSection] = &[
        SettingsSection::Appearance,
        SettingsSection::Terminal,
        SettingsSection::Environment,
        SettingsSection::Daemons,
        SettingsSection::GithubIssues,
        SettingsSection::Jira,
    ];

    /// The section's name, as the rail shows it.
    pub fn label(self) -> &'static str {
        match self {
            SettingsSection::Appearance => "Appearance",
            SettingsSection::Terminal => "Terminal",
            SettingsSection::Environment => "Environment",
            SettingsSection::Daemons => "Daemons",
            SettingsSection::GithubIssues => "GitHub",
            SettingsSection::Jira => "Jira",
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
            SettingsSection::Daemons => Icon::Daemons,
            SettingsSection::GithubIssues => Icon::IssueMapping,
            SettingsSection::Jira => Icon::JiraIssues,
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
    /// Whether the service saves terminal history to disk (feature 041, FR-026). Holds the value
    /// only; the default-on comes from `Settings`.
    pub save_terminal_history: bool,
}

/// The Environment section's fields (feature 011).
#[derive(Debug, Clone, PartialEq, Eq)]
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
    /// Whether a session that needs attention raises a desktop notification (feature 039,
    /// FR-026). One switch for every AI CLI, service-owned like the two above it; the default-on
    /// comes from `Settings`.
    pub desktop_notifications: bool,
    /// Which notification kinds are on (feature 613, FR-009). Held whole and sent whole on save
    /// (S3); kept as they are while `desktop_notifications` is off (FR-012).
    pub notification_kinds: NotificationKinds,
    /// The long-task threshold, in seconds as typed (feature 613, FR-025, S5). Text, as the
    /// timeout above is: only a save judges it (S6).
    pub long_task_threshold_secs: String,
    /// **Show Claude plan usage** (feature 490, FR-001). Service-owned like the switches above;
    /// the default-on comes from `Settings`.
    pub plan_usage_enabled: bool,
    /// The plan-usage warning threshold, as typed (feature 490, FR-005). Text, as the thresholds
    /// above are: only a save judges it.
    pub plan_usage_warn_percent: String,
    /// Whether agents may read and type into other sessions (feature 034, FR-016). A closed choice
    /// of three values, like the default CLI above: nothing to validate on save.
    pub cross_session_access: CrossSessionAccess,
}

/// Every field at its type's default, except the plan-usage pair (feature 490): on and "80", the
/// stored defaults, so a draft built without the stored settings neither turns the feature off nor
/// refuses a threshold nobody typed.
impl Default for EnvironmentDraft {
    fn default() -> Self {
        Self {
            enabled: Default::default(),
            script_path: Default::default(),
            timeout_secs: Default::default(),
            default_ai_cli: Default::default(),
            pi_activity_component: Default::default(),
            tool_server_enabled: Default::default(),
            desktop_notifications: Default::default(),
            notification_kinds: Default::default(),
            long_task_threshold_secs: Default::default(),
            plan_usage_enabled: micold_core::plan_usage::DEFAULT_PLAN_USAGE_ENABLED,
            plan_usage_warn_percent: micold_core::plan_usage::WarnPercent::DEFAULT.to_string(),
            cross_session_access: Default::default(),
        }
    }
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
    /// The switch **Show pull request status on worktrees** (feature 040, FR-029). Off until the
    /// user turns it on (FR-030).
    pub pr_status_enabled: bool,
}

impl Default for GithubDraft {
    /// The default table, as a never-edited mapping reads (FR-021).
    fn default() -> Self {
        Self {
            entries: default_mapping(),
            pr_status_enabled: false,
        }
    }
}

/// An API token as typed. Its `Debug` redacts, so a draft, a message or a log line holding it
/// never shows it (FR-009).
#[derive(Clone, Default, PartialEq, Eq)]
pub struct TokenInput(String);

impl TokenInput {
    /// What the user typed.
    pub fn new(text: String) -> Self {
        Self(text)
    }

    /// The text, for the masked field and for validation.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Debug for TokenInput {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("TokenInput(<redacted>)")
    }
}

/// The question raised when the switch is turned off over a stored connection (US2 AS5, US3 AS4).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeepOrRemovePrompt {
    /// The project whose connection is at stake; the prompt names it.
    pub project: std::path::PathBuf,
}

/// The answer to a [`KeepOrRemovePrompt`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeepOrRemove {
    /// Leave the connection and its token stored.
    Keep,
    /// Delete them.
    Remove,
}

/// The Jira section (feature 533, FR-002, FR-004, FR-006, FR-010).
///
/// Holds the switch, the open project and the connection stored for it, and the connection form
/// as typed. With the switch off the page renders only the switch and its description (FR-001).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct JiraDraft {
    /// The switch **Enable Jira integration**. Off until the user turns it on (FR-002).
    pub enabled: bool,
    /// The open project, when one is open.
    pub project: Option<std::path::PathBuf>,
    /// The connection stored for [`Self::project`], when there is one.
    pub connection: Option<micold_core::jira::JiraConnection>,
    /// **Site URL**, as typed.
    pub site: String,
    /// **Project key**, as typed.
    pub project_key: String,
    /// **Account email**, as typed.
    pub email: String,
    /// **API token**, as typed; blank keeps a stored one.
    pub token: TokenInput,
    /// Whether a token file exists for the project (FR-010); the token itself is never held.
    pub token_stored: bool,
    /// **Remove connection** was pressed; the save deletes the connection and its token.
    pub remove_requested: bool,
    /// Keep-or-remove question awaiting an answer; nothing is applied meanwhile.
    pub prompt: Option<KeepOrRemovePrompt>,
    /// The user answered **Keep** to the question, so the save does not raise it again.
    pub kept: bool,
}

impl JiraDraft {
    /// The status line under the switch (FR-004); `None` while the switch is off.
    pub fn status(&self) -> Option<String> {
        if !self.enabled {
            return None;
        }
        Some(match (&self.project, &self.connection) {
            (None, _) => "Open a project to connect it to Jira.".to_owned(),
            (Some(_), None) => "Enabled. This project has no Jira connection yet.".to_owned(),
            (Some(_), Some(c)) => {
                format!("Enabled. Connected to {} - {}.", c.site, c.project_key)
            }
        })
    }

    fn typed(&self) -> (&str, &str, &str) {
        (&self.site, &self.project_key, &self.email)
    }

    fn fill_from(&mut self, connection: Option<&micold_core::jira::JiraConnection>) {
        let (site, key, email) = connection
            .map(|c| {
                (
                    c.site.to_string(),
                    c.project_key.to_string(),
                    c.email.to_string(),
                )
            })
            .unwrap_or_default();
        self.site = site;
        self.project_key = key;
        self.email = email;
        self.token = TokenInput::default();
    }

    /// Point the form at `project` and the connection `connections` holds for it; `token_stored`
    /// is the probe's answer for that project. Typed text is kept unless the form still shows
    /// the previous connection untouched.
    pub fn follow(
        &mut self,
        project: Option<std::path::PathBuf>,
        connections: &std::collections::BTreeMap<String, micold_core::jira::JiraConnection>,
        token_stored: bool,
    ) {
        let connection = project
            .as_deref()
            .and_then(|p| connections.get(&micold_core::jira::connection_key(p)))
            .cloned();
        let previous = self.connection.as_ref().map(|c| {
            (
                c.site.to_string(),
                c.project_key.to_string(),
                c.email.to_string(),
            )
        });
        let untouched = match previous {
            Some((site, key, email)) => {
                self.typed() == (site.as_str(), key.as_str(), email.as_str())
                    && self.token.as_str().is_empty()
            }
            None => self.typed() == ("", "", "") && self.token.as_str().is_empty(),
        };
        let moved = self.project != project;
        if (untouched || moved) && !self.remove_requested {
            self.fill_from(connection.as_ref());
        }
        if moved {
            self.remove_requested = false;
            self.prompt = None;
            self.kept = false;
        }
        self.token_stored = token_stored && !self.remove_requested;
        self.connection = connection;
        self.project = project;
    }

    /// An edit of a connection field: a pending removal or question no longer describes the form.
    fn edited(&mut self) {
        self.remove_requested = false;
        self.prompt = None;
    }

    /// Whether the connection form is validated at all: Jira is on, a project is open, no removal
    /// is pending and the form holds input or a stored connection (the cases [`SettingsDraft::validate`]
    /// does not answer with `Keep`).
    fn validated(&self) -> bool {
        self.enabled && self.project.is_some() && !self.remove_requested && self.has_input()
    }

    /// Whether the form holds typed connection input worth validating.
    fn has_input(&self) -> bool {
        self.connection.is_some() || self.typed() != ("", "", "") || !self.token.as_str().is_empty()
    }
}

/// The legacy daemon block the settings file still carries (feature 027, FR-028); no page edits it (804).
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
    /// The reference of "My own image", as typed (feature 806).
    pub image_reference: String,
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
    /// Terminal (feature 041).
    pub save_terminal_history: bool,
    /// Environment.
    pub tool_server_enabled: bool,
    /// Environment.
    pub desktop_notifications: bool,
    /// Environment: the kind switches (feature 613).
    pub notification_kinds: NotificationKinds,
    /// Environment: the long-task threshold, in 10–3600 seconds (feature 613).
    pub long_task_threshold_secs: u64,
    /// Environment: **Show Claude plan usage** (feature 490).
    pub plan_usage_enabled: bool,
    /// Environment: the plan-usage warning threshold, 50–100 (feature 490).
    pub plan_usage_warn_percent: u8,
    /// Environment.
    pub cross_session_access: CrossSessionAccess,
    /// The legacy daemon block, kept for the migration; no page edits it (804).
    pub daemon: DaemonConfig,
    /// GitHub issues: the label-to-type mapping (feature 034).
    pub issue_label_types: Vec<LabelTypeEntry>,
    /// GitHub: whether pull request status is read (feature 040, FR-029).
    pub pr_status_enabled: bool,
    /// Jira: whether the integration is on (feature 533, FR-002).
    pub jira_enabled: bool,
    /// Jira: what the save does to the open project's connection (feature 533, US3).
    pub jira: micold_core::jira::ConnectionAction,
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
            save_terminal_history: self.save_terminal_history,
            tool_server_enabled: self.tool_server_enabled,
            desktop_notifications: self.desktop_notifications,
            cross_session_access: self.cross_session_access,
            daemon: self.daemon,
            issue_label_types: self.issue_label_types,
            pr_status_enabled: self.pr_status_enabled,
            jira_enabled: self.jira_enabled,
            // Not in the form yet (feature 533): written by their own change, never by a save.
            jira_connections: std::collections::BTreeMap::new(),
            notification_kinds: self.notification_kinds,
            long_task_threshold_secs: self.long_task_threshold_secs,
            // Not in the form (feature 482): service-owned; the save keeps the stored value.
            diff_layout: micold_core::settings::DiffLayout::Unified,
            // Not in the form (feature 491): the registry is service-owned; a save sets only
            // the changed fields on the stored settings (`set_changed`) and keeps the registry.
            daemons: Vec::new(),
            next_daemon_id: 0,
            legacy_default_daemon: None,
            plan_usage_enabled: self.plan_usage_enabled,
            plan_usage_warn_percent: self.plan_usage_warn_percent,
        }
    }
}

impl State {
    /// The daemon a location without a stored binding runs on: the legacy default, else id 1 (what
    /// a pre-feature `daemon` block migrated to).
    pub fn legacy_daemon(&self) -> micold_core::daemons::DaemonId {
        self.legacy_default_daemon
            .unwrap_or(micold_core::daemons::DaemonId(1))
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
    /// The legacy daemon block, mirrored for the migration; no page edits it (804).
    pub daemon: DaemonDraft,
    /// GitHub issues (feature 034).
    pub github: GithubDraft,
    /// Jira (feature 533).
    pub jira: JiraDraft,
    /// The last validation failure shown after a rejected save.
    pub error: Option<FieldError>,
    /// The settings the page was opened with, which a save compares against so that it changes
    /// only what the user changed (BUG-570, FR-026a). `None` for a draft not seeded from stored
    /// settings: every field then counts as changed.
    pub baseline: Option<Settings>,
    /// Which fields the user has left and whether a save was attempted, for the required errors
    /// (feature 937). Fresh when the page opens, dropped with the draft.
    pub required: RequiredMarks<FieldId>,
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
pub(crate) const MAX_MIB: u64 = 1024 * 1024;
/// The kernel's own `pid_max` ceiling on 64-bit Linux.
const MAX_PIDS: u32 = 4_194_304;

/// The image a form stands for: `Basic` stays `Basic`; "My own image" is its reference and
/// archive texts validated (feature 806, FR-003). The archive wins when both are set.
pub fn image_choice(
    kind: &ImageChoice,
    reference: &str,
    archive: &str,
) -> Result<ImageChoice, ImageSourceProblem> {
    match kind {
        ImageChoice::Basic => Ok(ImageChoice::Basic),
        ImageChoice::Own(_) => {
            let archive = archive.trim();
            OwnImageDraft {
                reference: reference.to_string(),
                archive: (!archive.is_empty()).then(|| PathBuf::from(archive)),
            }
            .validate()
            .map(ImageChoice::Own)
        }
    }
}

/// A sandbox profile's path and limits as the text fields show them (page and Daemons form).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ProfileTexts {
    /// The archive path; empty when none.
    pub image_path: String,
    /// The reference of "My own image"; empty when none.
    pub image_reference: String,
    /// Cores; empty when unset.
    pub cpus: String,
    /// MiB; empty when unset.
    pub memory_mib: String,
    /// Processes; empty when unset.
    pub pids: String,
    /// MiB; empty when unset.
    pub storage_mib: String,
}

impl ProfileTexts {
    /// What `profile` shows: what the user would type to reproduce it.
    pub fn of(profile: &SandboxProfile) -> Self {
        let b = &profile.budget;
        Self {
            image_path: match &profile.image {
                ImageChoice::Own(OwnImage::Archive(p)) => p.display().to_string(),
                _ => String::new(),
            },
            image_reference: match &profile.image {
                ImageChoice::Own(OwnImage::Reference(r)) => r.clone(),
                _ => String::new(),
            },
            cpus: b.cpus_milli.map(cores_text).unwrap_or_default(),
            memory_mib: b
                .memory_bytes
                .map(|v| v.as_mib().to_string())
                .unwrap_or_default(),
            pids: b.pids.map(|p| p.to_string()).unwrap_or_default(),
            storage_mib: b
                .storage_bytes
                .map(|v| v.as_mib().to_string())
                .unwrap_or_default(),
        }
    }
}

/// Which of the four sandbox limits a text was about (feature 804): the page and the Daemons form
/// each map it to their own field.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LimitField {
    /// The processor limit.
    Cpus,
    /// The memory limit.
    Memory,
    /// The process-count limit.
    Pids,
    /// The writable-storage limit.
    Storage,
}

impl LimitField {
    fn field_id(self) -> FieldId {
        match self {
            Self::Cpus => FieldId::SettingsCpuLimit,
            Self::Memory => FieldId::SettingsMemoryLimit,
            Self::Pids => FieldId::SettingsPidLimit,
            Self::Storage => FieldId::SettingsStorageLimit,
        }
    }
}

/// Parse the four typed limits. An empty text is *unset* (rule RB-2); a value outside the
/// documented range is refused with the range that would be accepted (FR-016). The one copy of
/// this parsing, used by the Settings page and the Daemons form.
pub fn parse_budget(
    cpus: &str,
    memory_mib: &str,
    pids: &str,
    storage_mib: &str,
) -> Result<micold_core::sandbox::ResourceBudget, (LimitField, String)> {
    Ok(micold_core::sandbox::ResourceBudget {
        cpus_milli: parse_cores(cpus)?,
        memory_bytes: parse_mib(memory_mib, LimitField::Memory, MIN_MEMORY, MAX_MIB)?,
        pids: parse_count(pids, MIN_PIDS, MAX_PIDS)?,
        storage_bytes: parse_mib(storage_mib, LimitField::Storage, MIN_STORAGE, MAX_MIB)?,
    })
}

pub(crate) fn parse_cores(text: &str) -> Result<Option<MilliCpus>, (LimitField, String)> {
    let text = text.trim();
    if text.is_empty() {
        return Ok(None);
    }
    let min = cores_text(MIN_MILLI_CPUS);
    match text.parse::<f64>() {
        Ok(c) if c.is_finite() && c >= f64::from(MIN_MILLI_CPUS.0) / 1000.0 && c <= MAX_CORES => {
            Ok(Some(MilliCpus((c * 1000.0).round() as u32)))
        }
        Ok(_) => Err((
            LimitField::Cpus,
            format!("Enter between {min} and {MAX_CORES:.0} cores, or leave it empty to use the runtime's default."),
        )),
        Err(_) => Err((
            LimitField::Cpus,
            "Enter a number of cores, like 2 or 1.5.".to_string(),
        )),
    }
}

pub(crate) fn parse_mib(
    text: &str,
    field: LimitField,
    min: Bytes,
    max_mib: u64,
) -> Result<Option<Bytes>, (LimitField, String)> {
    let text = text.trim();
    if text.is_empty() {
        return Ok(None);
    }
    let min_mib = min.as_mib();
    match text.parse::<u64>() {
        Ok(m) if (min_mib..=max_mib).contains(&m) => Ok(Some(Bytes::from_mib(m))),
        Ok(_) => Err((
            field,
            format!(
                "Enter between {min_mib} and {max_mib} MiB, or leave it empty to use the \
                 runtime's default."
            ),
        )),
        Err(_) => Err((field, "Enter a whole number of mebibytes.".to_string())),
    }
}

fn parse_count(text: &str, min: u32, max: u32) -> Result<Option<u32>, (LimitField, String)> {
    let text = text.trim();
    if text.is_empty() {
        return Ok(None);
    }
    match text.parse::<u32>() {
        Ok(n) if (min..=max).contains(&n) => Ok(Some(n)),
        Ok(_) => Err((
            LimitField::Pids,
            format!(
                "Enter between {min} and {max} processes, or leave it empty to use the \
                 runtime's default."
            ),
        )),
        Err(_) => Err((
            LimitField::Pids,
            "Enter a whole number of processes.".to_string(),
        )),
    }
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

    /// Parse and range-check every section together.
    ///
    /// Total: it reports, and never panics, for anything that can be typed. The order the fields
    /// are checked in is the rail's order, so a form with two bad values reports the first one a
    /// user reading top to bottom would reach.
    pub fn validate(&self) -> Result<ValidSettings, FieldError> {
        let scrollback_lines = self.scrollback()?;
        let env_include_timeout_secs = self.timeout()?;
        let long_task_threshold_secs = self.long_task_threshold()?;
        let plan_usage_warn_percent = self.plan_usage_warn()?;

        let mut profile = self.daemon.profile.clone();
        profile.image = image_choice(
            &profile.image,
            &self.daemon.image_reference,
            &self.daemon.image_path,
        )
        .map_err(|problem| FieldError {
            field: FieldId::DaemonImage,
            section: SettingsSection::Daemons,
            message: problem.message(),
        })?;
        profile.budget = self.budget()?;
        self.mapping()?;
        let jira = self.jira_action()?;

        Ok(ValidSettings {
            theme: self.appearance.theme,
            scrollback_lines,
            env_include_enabled: self.environment.enabled,
            env_include_script_path: self.environment.script_path.clone(),
            env_include_timeout_secs,
            default_ai_cli: self.environment.default_ai_cli,
            pi_activity_component: self.environment.pi_activity_component,
            save_terminal_history: self.terminal.save_terminal_history,
            tool_server_enabled: self.environment.tool_server_enabled,
            desktop_notifications: self.environment.desktop_notifications,
            notification_kinds: self.environment.notification_kinds,
            long_task_threshold_secs,
            plan_usage_enabled: self.environment.plan_usage_enabled,
            plan_usage_warn_percent,
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
            pr_status_enabled: self.github.pr_status_enabled,
            jira_enabled: self.jira.enabled,
            jira,
        })
    }

    /// The fields `section` refuses empty, in display order, with whether each is empty now
    /// (feature 937, FR-002). Mirrors [`Self::validate`]'s emptiness rules, trims included: the
    /// scrollback, timeout, long-task threshold and warn percentage always; one label per GitHub
    /// mapping entry; the Jira connection's four fields only while that form is validated, and its
    /// token only when none is stored. Appearance and Daemons list nothing (the daemon form has its
    /// own list).
    pub fn required_fields(&self, section: SettingsSection) -> Vec<(FieldId, Requirement)> {
        let text = |id, value: &str| (id, Requirement::text(true, value));
        match section {
            SettingsSection::Appearance | SettingsSection::Daemons => Vec::new(),
            SettingsSection::Terminal => {
                vec![text(
                    FieldId::SettingsScrollback,
                    &self.terminal.scrollback_lines,
                )]
            }
            SettingsSection::Environment => {
                let e = &self.environment;
                vec![
                    text(
                        FieldId::SettingsLongTaskThreshold,
                        &e.long_task_threshold_secs,
                    ),
                    text(FieldId::SettingsPlanUsageWarn, &e.plan_usage_warn_percent),
                    text(FieldId::SettingsEnvIncludeTimeout, &e.timeout_secs),
                ]
            }
            SettingsSection::GithubIssues => self
                .github
                .entries
                .iter()
                .enumerate()
                .map(|(i, entry)| text(FieldId::IssueMappingLabel(i), &entry.label))
                .collect(),
            SettingsSection::Jira => {
                let j = &self.jira;
                if !j.validated() {
                    return Vec::new();
                }
                vec![
                    text(FieldId::SettingsJiraSite, &j.site),
                    text(FieldId::SettingsJiraKey, &j.project_key),
                    text(FieldId::SettingsJiraEmail, &j.email),
                    (
                        FieldId::SettingsJiraToken,
                        Requirement::text(!j.token_stored, j.token.as_str()),
                    ),
                ]
            }
        }
    }

    /// Forget the fields the user left among the mapping labels: the entries were removed,
    /// moved or replaced, so an index no longer names the entry it did (feature 937).
    fn forget_mapping_marks(&mut self) {
        self.required
            .forget(|id| matches!(id, FieldId::IssueMappingLabel(_)));
    }

    /// The connection form, checked last because its section is last in the rail (FR-006).
    fn jira_action(&self) -> Result<micold_core::jira::ConnectionAction, FieldError> {
        use micold_core::jira::{
            ApiToken, ConnectionAction, FieldProblem, JiraConnection, JiraField,
        };
        let j = &self.jira;
        let Some(project) = j.project.clone() else {
            return Ok(ConnectionAction::Keep);
        };
        // A removal wins over the switch: answering **Remove** to the question arrives with the
        // switch already off.
        if j.remove_requested {
            return Ok(if j.connection.is_some() {
                ConnectionAction::Remove { project }
            } else {
                ConnectionAction::Keep
            });
        }
        if !j.enabled || !j.has_input() {
            return Ok(ConnectionAction::Keep);
        }
        let refuse = |problem: FieldProblem| FieldError {
            field: match problem.field {
                JiraField::Site => FieldId::SettingsJiraSite,
                JiraField::ProjectKey => FieldId::SettingsJiraKey,
                JiraField::Email => FieldId::SettingsJiraEmail,
                JiraField::Token => FieldId::SettingsJiraToken,
            },
            section: SettingsSection::Jira,
            message: problem.reason.to_owned(),
        };
        let connection = JiraConnection::parse(&j.site, &j.project_key, &j.email)
            .map_err(|problems| refuse(problems[0]))?;
        let token = if j.token.as_str().trim().is_empty() && j.token_stored {
            None
        } else {
            Some(ApiToken::parse(j.token.as_str()).map_err(refuse)?)
        };
        if token.is_none() && j.connection.as_ref() == Some(&connection) {
            return Ok(ConnectionAction::Keep);
        }
        Ok(ConnectionAction::Save {
            project,
            connection,
            token,
        })
    }

    /// Whether saving now must first ask the Keep-or-Remove question: the switch goes off over a
    /// stored connection that was not already answered (US2 AS5).
    pub fn needs_keep_or_remove(&self) -> bool {
        let j = &self.jira;
        let was_on = self.baseline.as_ref().is_none_or(|b| b.jira_enabled);
        !j.enabled && was_on && j.connection.is_some() && !j.remove_requested && !j.kept
    }

    /// Raise the question and show the section it is in; nothing is applied until it is answered.
    pub fn raise_keep_or_remove(&mut self) {
        if let Some(project) = self.jira.project.clone() {
            self.jira.prompt = Some(KeepOrRemovePrompt { project });
            self.section = SettingsSection::Jira;
        }
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
        let d = &self.daemon;
        parse_budget(&d.cpus, &d.memory_mib, &d.pids, &d.storage_mib).map_err(|(field, message)| {
            FieldError {
                field: field.field_id(),
                section: SettingsSection::Daemons,
                message,
            }
        })
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

    /// The long-task threshold as typed, parsed and range-checked (feature 613, S6, FR-026).
    fn long_task_threshold(&self) -> Result<u64, FieldError> {
        let min = micold_core::settings::MIN_LONG_TASK_THRESHOLD_SECS;
        let max = micold_core::settings::MAX_LONG_TASK_THRESHOLD_SECS;
        let reject = |message: String| FieldError {
            field: FieldId::SettingsLongTaskThreshold,
            section: SettingsSection::Environment,
            message,
        };
        match self
            .environment
            .long_task_threshold_secs
            .trim()
            .parse::<u64>()
        {
            Ok(t) if (min..=max).contains(&t) => Ok(t),
            Ok(_) => Err(reject(format!(
                "Enter a threshold between {min} and {max} seconds."
            ))),
            Err(_) => Err(reject("Enter a whole number of seconds.".to_string())),
        }
    }

    /// The plan-usage threshold as typed, refused outside 50–100 with a message naming the range
    /// (feature 490, FR-005).
    fn plan_usage_warn(&self) -> Result<u8, FieldError> {
        micold_core::plan_usage::WarnPercent::parse(&self.environment.plan_usage_warn_percent)
            .map(micold_core::plan_usage::WarnPercent::get)
            .map_err(|message| FieldError {
                field: FieldId::SettingsPlanUsageWarn,
                section: SettingsSection::Environment,
                message,
            })
    }

    /// Bring an open page up to `now`, the settings as stored after another window saved
    /// (BUG-475, FR-026b): each field the user has not edited since the page opened shows its value
    /// in `now`, and a field they edited keeps their edit. "Edited" is judged against the page as
    /// opened ([`Self::baseline`]), which then becomes `now`, so a later save still sends only the
    /// fields the user changed (FR-026a) and a later refresh judges against what the page shows.
    ///
    /// A draft with no baseline was not seeded from stored settings and is left as it is. The
    /// runtime's capabilities and the sandbox's sign-in report are not settings and are untouched.
    pub fn refresh_untouched(&mut self, now: &Settings) {
        let Some(opened) = self.baseline.as_ref().map(Self::from_settings) else {
            return;
        };
        let fresh = Self::from_settings(now);
        // Named in full so that a field added to the form fails to compile here until this says
        // whether another window's save refreshes it.
        let Self {
            section: _,
            appearance: AppearanceDraft { theme: _ },
            terminal:
                TerminalDraft {
                    scrollback_lines: _,
                    save_terminal_history: _,
                },
            environment:
                EnvironmentDraft {
                    enabled: _,
                    script_path: _,
                    timeout_secs: _,
                    default_ai_cli: _,
                    pi_activity_component: _,
                    tool_server_enabled: _,
                    desktop_notifications: _,
                    notification_kinds: _,
                    long_task_threshold_secs: _,
                    plan_usage_enabled: _,
                    plan_usage_warn_percent: _,
                    cross_session_access: _,
                },
            daemon:
                DaemonDraft {
                    placement: _,
                    profile: _,
                    image_path: _,
                    image_reference: _,
                    cpus: _,
                    memory_mib: _,
                    pids: _,
                    storage_mib: _,
                    capabilities: _,
                    unshared_sign_in: _,
                },
            github:
                GithubDraft {
                    entries: _,
                    pr_status_enabled: _,
                },
            jira:
                JiraDraft {
                    enabled: _,
                    project: _,
                    connection: _,
                    site: _,
                    project_key: _,
                    email: _,
                    token: _,
                    token_stored: _,
                    remove_requested: _,
                    prompt: _,
                    kept: _,
                },
            error: _,
            baseline: _,
            required: _,
        } = &fresh;
        macro_rules! refresh {
            ($($part:ident . $field:ident),+ $(,)?) => {$(
                if self.$part.$field == opened.$part.$field {
                    self.$part.$field = fresh.$part.$field.clone();
                }
            )+};
        }
        refresh!(
            appearance.theme,
            terminal.scrollback_lines,
            terminal.save_terminal_history,
            environment.enabled,
            environment.script_path,
            environment.timeout_secs,
            environment.default_ai_cli,
            environment.pi_activity_component,
            environment.tool_server_enabled,
            environment.desktop_notifications,
            environment.notification_kinds,
            environment.long_task_threshold_secs,
            environment.plan_usage_enabled,
            environment.plan_usage_warn_percent,
            environment.cross_session_access,
            daemon.placement,
            daemon.profile,
            daemon.image_path,
            daemon.image_reference,
            daemon.cpus,
            daemon.memory_mib,
            daemon.pids,
            daemon.storage_mib,
            github.entries,
            github.pr_status_enabled,
            jira.enabled,
        );
        self.baseline = Some(
            fresh
                .validate()
                .map(ValidSettings::into_settings)
                .unwrap_or_else(|_| now.clone()),
        );
    }

    /// Seed the draft from what is stored, so the form opens showing the current values.
    pub fn from_settings(settings: &Settings) -> Self {
        let profile_texts = ProfileTexts::of(&settings.daemon.sandbox);
        Self {
            section: SettingsSection::default(),
            appearance: AppearanceDraft {
                theme: settings.theme,
            },
            terminal: TerminalDraft {
                scrollback_lines: settings.scrollback_lines.to_string(),
                save_terminal_history: settings.save_terminal_history,
            },
            environment: EnvironmentDraft {
                enabled: settings.env_include_enabled,
                script_path: settings.env_include_script_path.clone(),
                timeout_secs: settings.env_include_timeout_secs.to_string(),
                default_ai_cli: settings.default_ai_cli,
                pi_activity_component: settings.pi_activity_component,
                tool_server_enabled: settings.tool_server_enabled,
                desktop_notifications: settings.desktop_notifications,
                notification_kinds: settings.notification_kinds,
                long_task_threshold_secs: settings.long_task_threshold_secs.to_string(),
                plan_usage_enabled: settings.plan_usage_enabled,
                plan_usage_warn_percent: micold_core::settings::clamp_plan_usage_warn_percent(
                    u64::from(settings.plan_usage_warn_percent),
                )
                .to_string(),
                cross_session_access: settings.cross_session_access,
            },
            daemon: DaemonDraft {
                placement: settings.daemon.placement,
                profile: settings.daemon.sandbox.clone(),
                // An unset limit seeds an *empty* field rather than a zero or the word
                // "unlimited": empty is what the user types to unset it (rule RB-2).
                image_path: profile_texts.image_path,
                image_reference: profile_texts.image_reference,
                cpus: profile_texts.cpus,
                memory_mib: profile_texts.memory_mib,
                pids: profile_texts.pids,
                storage_mib: profile_texts.storage_mib,
                // Seeded by the shell from the sandbox's state, which is where the probe's answer
                // lands — `Settings` has never heard of it and must not learn.
                capabilities: None,
                unshared_sign_in: None,
            },
            github: GithubDraft {
                entries: settings.issue_label_types.clone(),
                pr_status_enabled: settings.pr_status_enabled,
            },
            jira: JiraDraft {
                enabled: settings.jira_enabled,
                ..JiraDraft::default()
            },
            error: None,
            baseline: Some(settings.clone()),
            required: RequiredMarks::default(),
        }
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
    /// The daemon registry was read from the stored settings: at boot, and after a save that moved
    /// the single entry with the placement (feature 491).
    DaemonsLoaded {
        /// The registry as stored.
        daemons: micold_core::daemons::DaemonRegistry,
        /// The daemon a worktree without a binding runs on.
        legacy_default: Option<micold_core::daemons::DaemonId>,
    },
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
    /// Terminal: the **Save terminal history** checkbox (feature 041, FR-026).
    SaveTerminalHistoryToggled(bool),
    /// The Settings **Let AI sessions manage worktrees and sessions** switch was toggled
    /// (feature 034, FR-004).
    ToolServerToggled(bool),
    /// The Settings **Desktop notifications** switch was toggled (feature 039, FR-026).
    DesktopNotificationsToggled(bool),
    /// A Settings notification kind switch was toggled (feature 613, S3).
    NotificationKindToggled(NotificationKind, bool),
    /// The Settings long-task threshold field changed (feature 613, S5).
    LongTaskThresholdChanged(String),
    /// Environment: **Show Claude plan usage** was toggled (feature 490, FR-001).
    PlanUsageToggled(bool),
    /// Environment: the plan-usage warning threshold was edited (feature 490, FR-005).
    PlanUsageWarnChanged(String),
    /// The Settings **Let agents read and type into other sessions** select changed
    /// (feature 034, FR-016).
    CrossSessionAccessChanged(CrossSessionAccess),
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
    /// The pull request status switch was toggled (feature 040, FR-029).
    PrStatusToggled(bool),
    /// The **Enable Jira integration** switch was toggled (feature 533, FR-002).
    JiraEnabledToggled(bool),
    /// **Site URL** was edited (feature 533, FR-006).
    JiraSiteChanged(String),
    /// **Project key** was edited.
    JiraKeyChanged(String),
    /// **Account email** was edited.
    JiraEmailChanged(String),
    /// **API token** was edited; the text never shows in `Debug` (FR-009).
    JiraTokenChanged(TokenInput),
    /// **Remove connection** was pressed (FR-010).
    JiraRemoveRequested,
    /// The Keep-or-Remove question was answered (US2 AS5, US3 AS4).
    JiraKeepOrRemoveChosen(KeepOrRemove),
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
        Msg::SaveTerminalHistoryToggled(on) => save_terminal_history_toggled(state, on),
        Msg::ToolServerToggled(on) => tool_server_toggled(state, on),
        Msg::DesktopNotificationsToggled(on) => desktop_notifications_toggled(state, on),
        Msg::NotificationKindToggled(kind, on) => notification_kind_toggled(state, kind, on),
        Msg::LongTaskThresholdChanged(text) => long_task_threshold_changed(state, text),
        Msg::PlanUsageToggled(on) => plan_usage_toggled(state, on),
        Msg::PlanUsageWarnChanged(text) => plan_usage_warn_changed(state, text),
        Msg::CrossSessionAccessChanged(access) => cross_session_access_changed(state, access),
        Msg::PlacementMoved(kind) => placement_in_force_changed(state, kind),
        Msg::DaemonsLoaded {
            daemons,
            legacy_default,
        } => registry_loaded(&mut state.settings, daemons, legacy_default),
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
        Msg::IssueMappingRemoved(index) => {
            edit_mapping(state, |entries| {
                if index < entries.len() {
                    entries.remove(index);
                }
            });
            forget_mapping_marks(state);
        }
        Msg::IssueMappingMoved(index, direction) => {
            edit_mapping(state, |entries| move_entry(entries, index, direction));
            forget_mapping_marks(state);
        }
        Msg::IssueMappingDefaultsRestored => {
            edit_mapping(state, |entries| *entries = default_mapping());
            forget_mapping_marks(state);
        }
        Msg::PrStatusToggled(on) => edit(state, |draft| draft.github.pr_status_enabled = on),
        Msg::JiraEnabledToggled(on) => edit(state, |draft| {
            draft.jira.enabled = on;
            draft.jira.prompt = None;
            draft.jira.kept = false;
        }),
        Msg::JiraSiteChanged(text) => edit(state, |draft| {
            draft.jira.edited();
            draft.jira.site = text;
        }),
        Msg::JiraKeyChanged(text) => edit(state, |draft| {
            draft.jira.edited();
            draft.jira.project_key = text;
        }),
        Msg::JiraEmailChanged(text) => edit(state, |draft| {
            draft.jira.edited();
            draft.jira.email = text;
        }),
        Msg::JiraTokenChanged(text) => edit(state, |draft| {
            draft.jira.edited();
            draft.jira.token = text;
        }),
        Msg::JiraRemoveRequested => edit(state, |draft| {
            draft.jira.fill_from(None);
            draft.jira.token_stored = false;
            draft.jira.remove_requested = true;
        }),
        Msg::JiraKeepOrRemoveChosen(choice) => edit(state, |draft| {
            draft.jira.prompt = None;
            match choice {
                KeepOrRemove::Keep => draft.jira.kept = true,
                KeepOrRemove::Remove => {
                    draft.jira.fill_from(None);
                    draft.jira.token_stored = false;
                    draft.jira.remove_requested = true;
                }
            }
        }),
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

/// Terminal: the **Save terminal history** checkbox was toggled (feature 041, FR-026).
pub fn save_terminal_history_toggled(state: &mut crate::app::State, on: bool) {
    edit(state, |draft| draft.terminal.save_terminal_history = on);
}

/// Environment: bind new sessions to the service's tool server (feature 034, FR-004).
pub fn tool_server_toggled(state: &mut crate::app::State, on: bool) {
    edit(state, |draft| draft.environment.tool_server_enabled = on);
}

/// Environment: raise a desktop notification when a session needs attention (feature 039,
/// FR-026).
pub fn desktop_notifications_toggled(state: &mut crate::app::State, on: bool) {
    edit(state, |draft| draft.environment.desktop_notifications = on);
}

/// Environment: one notification kind's switch was toggled (feature 613, S3). The others keep
/// their values.
pub fn notification_kind_toggled(state: &mut crate::app::State, kind: NotificationKind, on: bool) {
    edit(state, |draft| {
        draft.environment.notification_kinds.set(kind, on)
    });
}

/// Environment: the long-task threshold was edited (feature 613, S5).
pub fn long_task_threshold_changed(state: &mut crate::app::State, text: String) {
    edit(state, |draft| {
        draft.environment.long_task_threshold_secs = text
    });
}

/// Environment: **Show Claude plan usage** was toggled (feature 490, FR-001).
pub fn plan_usage_toggled(state: &mut crate::app::State, on: bool) {
    edit(state, |draft| draft.environment.plan_usage_enabled = on);
}

/// Environment: the plan-usage warning threshold was edited (feature 490, FR-005).
pub fn plan_usage_warn_changed(state: &mut crate::app::State, text: String) {
    edit(state, |draft| {
        draft.environment.plan_usage_warn_percent = text
    });
}

/// Environment: whether agents may read and type into other sessions (feature 034, FR-016).
pub fn cross_session_access_changed(state: &mut crate::app::State, access: CrossSessionAccess) {
    edit(state, |draft| {
        draft.environment.cross_session_access = access
    });
}

/// GitHub issues: one edit to the mapping (feature 034, FR-018). An index past the end — a
/// message from a row that is gone — changes nothing.
fn edit_mapping(state: &mut crate::app::State, change: impl FnOnce(&mut Vec<LabelTypeEntry>)) {
    edit(state, |draft| change(&mut draft.github.entries));
}

/// The mapping's indices shifted: drop the left marks that named them (feature 937).
fn forget_mapping_marks(state: &mut crate::app::State) {
    if let Some(draft) = &mut state.settings.settings_draft {
        draft.forget_mapping_marks();
    }
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
}

/// Take the daemon registry and the legacy default from stored settings (feature 491), at boot
/// and after a save moved the single entry with the placement.
fn registry_loaded(
    state: &mut State,
    daemons: micold_core::daemons::DaemonRegistry,
    legacy_default: Option<micold_core::daemons::DaemonId>,
) {
    state.daemons = daemons;
    state.legacy_default_daemon = legacy_default;
}

/// The [`Msg::DaemonsLoaded`] a shell sends for stored settings (feature 491).
pub fn daemons_loaded(stored: &Settings) -> Msg {
    Msg::DaemonsLoaded {
        daemons: micold_core::daemons::DaemonRegistry::new(
            stored.daemons.clone(),
            stored.next_daemon_id,
        ),
        legacy_default: stored.legacy_default_daemon,
    }
}

/// The service is now running in `kind` (FR-035b).
///
/// Reported by the shell once a move has actually been applied — not when it is chosen, and not
/// when it is saved. Everything that tells the user where their sessions are reads this.
pub fn placement_in_force_changed(state: &mut crate::app::State, kind: PlacementKind) {
    state.settings.placement_in_force = kind;
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

/// The lines for each session directory whose own resolution by the service failed (011 FR-022,
/// BUG-454), shown right after [`script_path_notice`]'s: per directory, 011's category label
/// naming the directory, then what the script printed there when it printed anything.
///
/// `failures` is the service's list from the latest catalog snapshot; `last` is the outcome the
/// note above reports. A directory whose outcome equals it is folded into that note rather than
/// repeated, so an absolute path missing everywhere still reads as one note.
pub fn directory_failure_lines(
    failures: &[micold_core::protocol::messages::EnvIncludeFailure],
    last: &micold_core::env_include::EnvIncludeOutcome,
) -> Vec<NoticeLine> {
    failures
        .iter()
        .filter(|failure| failure.outcome != *last)
        .flat_map(|failure| {
            let mut lines = lines_011(&failure.outcome, None);
            if let Some(NoticeLine::Caution(category)) = lines.first_mut() {
                *category = format!("{category} in {}", failure.dir.display());
            }
            lines
        })
        .collect()
}

// --- Where a CLI is missing, and what to say about it (027 FR-023b, 037 FR-006) ---

/// The sentence shown where an image is chosen and where a CLI is chosen, when a session would not
/// find one of the CLIs (027 FR-023b, 037 FR-006): the reason and the action
/// [`micold_core::cli_reason::explain`] gives for the home directory's environment state, joined
/// as `{reason} {action}`. `None` when there is nothing to say.
///
/// The `None` cases:
///
/// - **the service has not answered yet.** Not "nothing is available" — the app has not asked, or
///   the reply is in flight. Saying anything here would be a guess, and a guess that names a
///   *specific* CLI as missing is worse than silence.
/// - **everything is present.** The absence of a notice is the whole of "this is fine"; a green
///   "all CLIs present" line is a second thing to read on every visit to a form that is not about
///   AI CLIs.
/// - **the answer carries no environment state** (037 FR-011, contract W5). A CLI is missing but
///   why is not known, and a reason made up here could send the user to change the wrong setting.
///
/// The sentence never presents this as the application failing. It says what a session would not
/// find and which setting or install changes that, because that is where the user can act. The
/// answer is always the home directory's here, so the attempt it reports is named as the home
/// directory's (037 FR-004a).
pub fn missing_cli_notice(availability: Option<&CliAvailability>) -> Option<String> {
    let availability = availability?;
    let Explanation { reason, action } = explain(
        &availability.missing(),
        availability.env?,
        availability.place(),
        AttemptDir::Home,
    )?;
    Some(format!("{reason} {action}"))
}

/// The `SettingsSet` that changes the diff layout and leaves every other setting as it is
/// (feature 482, R12): the layout is service-owned, so the daemon stores it and echoes it to every
/// window as `SettingsChanged`.
pub fn diff_layout_set(
    req: u64,
    layout: micold_core::settings::DiffLayout,
) -> micold_core::protocol::messages::ClientMsg {
    micold_core::protocol::messages::ClientMsg::SettingsSet {
        req,
        scrollback_lines: None,
        env_include_enabled: None,
        env_include_script_path: None,
        env_include_timeout_secs: None,
        default_ai_cli: None,
        pi_activity_component: None,
        save_terminal_history: None,
        tool_server_enabled: None,
        cross_session_access: None,
        pr_status_enabled: None,
        jira_enabled: None,
        jira_connection: None,
        desktop_notifications: None,
        notification_kinds: None,
        long_task_threshold_secs: None,
        diff_layout: Some(layout),
        plan_usage_enabled: None,
        plan_usage_warn_percent: None,
    }
}

#[cfg(test)]
mod parse_budget_tests {
    //! Feature 804, T003: the one parser of the four typed limits.

    use super::*;

    #[test]
    fn empty_texts_are_unset() {
        assert_eq!(
            parse_budget("", " ", "", ""),
            Ok(micold_core::sandbox::ResourceBudget {
                cpus_milli: None,
                memory_bytes: None,
                pids: None,
                storage_bytes: None,
            })
        );
    }

    #[test]
    fn valid_texts_parse_to_the_budget() {
        let b = parse_budget("1.5", "2048", "512", "4096").unwrap();
        assert_eq!(b.cpus_milli, Some(MilliCpus(1500)));
        assert_eq!(b.memory_bytes, Some(Bytes::from_mib(2048)));
        assert_eq!(b.pids, Some(512));
        assert_eq!(b.storage_bytes, Some(Bytes::from_mib(4096)));
    }

    #[test]
    fn each_refusal_names_its_limit() {
        for (args, field) in [
            (("abc", "", "", ""), LimitField::Cpus),
            (("", "1", "", ""), LimitField::Memory),
            (("", "", "x", ""), LimitField::Pids),
            (("", "", "", "1"), LimitField::Storage),
        ] {
            let (got, message) = parse_budget(args.0, args.1, args.2, args.3).unwrap_err();
            assert_eq!(got, field, "{message}");
            assert!(!message.is_empty());
        }
    }

    /// FR-016: a value below a limit's minimum is refused by its own field and the message names
    /// the accepted range, which is not what text that is not a number says.
    #[test]
    fn a_limit_below_its_minimum_names_the_accepted_range() {
        for (args, field, range) in [
            (("0.001", "", "", ""), LimitField::Cpus, "Enter between"),
            (("", "1", "", ""), LimitField::Memory, "MiB"),
            (("", "", "1", ""), LimitField::Pids, "Enter between"),
            (("", "", "", "1"), LimitField::Storage, "MiB"),
        ] {
            let (got, message) = parse_budget(args.0, args.1, args.2, args.3).unwrap_err();
            assert_eq!(got, field, "{message}");
            assert!(message.contains(range), "{field:?}: {message}");
            assert!(
                message.chars().any(|c| c.is_ascii_digit()),
                "{field:?}: {message}"
            );
        }
    }

    #[test]
    fn text_that_is_not_a_number_says_so_rather_than_naming_a_range() {
        for (args, field) in [
            (("abc", "", "", ""), LimitField::Cpus),
            (("", "x", "", ""), LimitField::Memory),
            (("", "", "x", ""), LimitField::Pids),
            (("", "", "", "x"), LimitField::Storage),
        ] {
            let (got, message) = parse_budget(args.0, args.1, args.2, args.3).unwrap_err();
            assert_eq!(got, field);
            assert!(!message.contains("between"), "{field:?}: {message}");
        }
    }

    #[test]
    fn a_whole_core_count_has_no_decimal_point() {
        let b = parse_budget("2", "", "", "").unwrap();
        assert_eq!(b.cpus_milli, Some(MilliCpus(2000)));
        assert_eq!(cores_text(MilliCpus(2000)), "2");
    }
}
