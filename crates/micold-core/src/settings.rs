//! Persistence boundary for application settings (currently just the theme preference).
//!
//! Fronted as a trait so logic stays testable without the real user data directory
//! (Constitution Principle I). The production `JsonFileSettingsStore` reuses the pattern in
//! `store.rs`: `serde_json` + `directories`, an atomic temp-file+rename write, and a
//! missing/corrupt file that degrades to defaults rather than crashing (Principle IV;
//! FR-019). On-disk format is the durable contract in
//! `specs/003-material-design-layout/contracts/settings-schema.md`.

use crate::attention::{NotificationKinds, LONG_TASK_THRESHOLD};
use crate::issue_types::{default_mapping, LabelTypeEntry};
use crate::jira::{JiraBaseUrl, JiraConnection};
use crate::mcp::policy::CrossSessionAccess;
use crate::sandbox::placement::PlacementKind;
use crate::sandbox::{BudgetViolation, SandboxProfile};
use crate::session::{AiCli, SessionId};
use crate::setup::SetupCommand;
use crate::store::LoadStatus;
use crate::theme::ThemePreference;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
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

/// Shortest accepted long-task threshold, in seconds (feature 613, FR-026).
pub const MIN_LONG_TASK_THRESHOLD_SECS: u64 = 10;
/// Longest accepted long-task threshold, in seconds (FR-026).
pub const MAX_LONG_TASK_THRESHOLD_SECS: u64 = 3600;

/// The long-task threshold when a file predates it or cannot be read: [`LONG_TASK_THRESHOLD`],
/// the one definition of the default (FR-025, contract C6).
fn default_long_task_threshold_secs() -> u64 {
    LONG_TASK_THRESHOLD.as_secs()
}

/// Clamp a long-task threshold into 10–3600 seconds (FR-026): read from the file, or sent to the
/// service. Settings refuses an out-of-range value on save instead.
pub fn clamp_long_task_threshold(secs: u64) -> u64 {
    secs.clamp(MIN_LONG_TASK_THRESHOLD_SECS, MAX_LONG_TASK_THRESHOLD_SECS)
}

fn default_plan_usage_enabled() -> bool {
    crate::plan_usage::DEFAULT_PLAN_USAGE_ENABLED
}

fn default_warn_percent() -> u8 {
    crate::plan_usage::WarnPercent::DEFAULT
}

fn default_plan_usage_warn_percent() -> u64 {
    u64::from(crate::plan_usage::WarnPercent::DEFAULT)
}

/// Clamp a plan-usage warning threshold into 50–100 percent (feature 490, FR-005): read from the
/// file, or sent to the service. Settings refuses an out-of-range value on save instead.
pub fn clamp_plan_usage_warn_percent(percent: u64) -> u8 {
    crate::plan_usage::WarnPercent::clamped(percent).get()
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

/// How the Changes view lays out a diff (feature 482, R12). Unified until the user picks
/// side by side; the choice is kept for the next file and the next start (US1 s4).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DiffLayout {
    /// One column, removed lines above added ones.
    #[default]
    Unified,
    /// The old version on the left, the new on the right.
    SideBySide,
}

/// One project's Jira connection changing (feature 533). Sent alone so two windows saving
/// different projects never overwrite each other's map.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct JiraConnectionChange {
    /// The project, as the wire names it elsewhere.
    pub project: String,
    /// What happens to its connection.
    pub change: JiraChange,
}

/// What a [`JiraConnectionChange`] does.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum JiraChange {
    /// Store this connection, replacing any.
    Upsert(JiraConnection),
    /// Remove the project's connection.
    Remove,
}

impl JiraConnectionChange {
    /// Apply the change to a connections map.
    pub fn apply(&self, map: &mut BTreeMap<String, JiraConnection>) {
        match &self.change {
            JiraChange::Upsert(connection) => {
                map.insert(self.project.clone(), connection.clone());
            }
            JiraChange::Remove => {
                map.remove(&self.project);
            }
        }
    }
}

/// One project's Jira base URL changing (feature 1122, D4). Sent alone, as a
/// [`JiraConnectionChange`] is, so two windows saving different projects never overwrite each
/// other's map.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct JiraBaseUrlChange {
    /// The project, keyed as its Jira connection is ([`crate::jira::connection_key`]).
    pub project: String,
    /// The new value; `None` unsets the project's base URL.
    pub url: Option<JiraBaseUrl>,
}

impl JiraBaseUrlChange {
    /// Apply the change to a base-URL map: set or remove only this project's entry.
    pub fn apply(&self, map: &mut BTreeMap<String, JiraBaseUrl>) {
        match &self.url {
            Some(url) => {
                map.insert(self.project.clone(), url.clone());
            }
            None => {
                map.remove(&self.project);
            }
        }
    }
}

/// One project's setup command changing (feature 1289), shaped as [`JiraBaseUrlChange`]: one
/// project's entry, so a save never overwrites another project's.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SetupCommandChange {
    /// The project, keyed as its Jira connection is ([`crate::jira::connection_key`]).
    pub project: String,
    /// The new command; `None` clears the project's command.
    pub command: Option<SetupCommand>,
}

impl SetupCommandChange {
    /// Apply the change to a setup-command map: set or remove only this project's entry.
    pub fn apply(&self, map: &mut BTreeMap<String, SetupCommand>) {
        match &self.command {
            Some(command) => {
                map.insert(self.project.clone(), command.clone());
            }
            None => {
                map.remove(&self.project);
            }
        }
    }
}

/// The persisted application settings document.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Settings {
    /// How the app selects its theme.
    #[serde(default)]
    pub theme: ThemePreference,
    /// Whether the Overview's full-screen zoom (feature 970, FR-008) and Overview card focus and
    /// expansion (feature 1299) switch in one step instead of animating. Off by default; a file
    /// written before the key existed reads as off. The operating system's reduced-motion
    /// preference is not read.
    #[serde(default)]
    pub reduce_motion: bool,
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
    /// The registry of session daemons (feature 491, FR-001). Authoritative when stored; a
    /// document without it is migrated once from [`Self::daemon`].
    #[serde(default)]
    pub daemons: Vec<crate::daemons::DaemonEntry>,
    /// The id the next added daemon receives; never reused.
    #[serde(default)]
    pub next_daemon_id: u32,
    /// The daemon a worktree with no stored binding runs on. Written once by the migration.
    #[serde(default)]
    pub legacy_default_daemon: Option<crate::daemons::DaemonId>,
    /// Which AI CLI a new session runs when nothing is chosen for it (feature 026, FR-003).
    ///
    /// **Not validated against availability.** A default naming an uninstalled CLI is kept, not
    /// silently rewritten: FR-004 asks the application to *tell* the user, and a preference that
    /// quietly repaired itself would also lose the user's choice across a temporary `PATH` problem
    /// (research R11).
    #[serde(default)]
    pub default_ai_cli: AiCli,
    /// Template for the first prompt of a new worktree's session (feature 1161, FR-001). Stored
    /// verbatim, newlines kept; text that fails `PromptTemplate::parse` is kept too (FR-013) and
    /// treated as empty by whoever renders it. Service-owned beside `default_ai_cli`, because the
    /// MCP tools render it in the service. Empty means no default prompt.
    #[serde(default)]
    pub default_prompt_template: String,
    /// Whether a Pi session is started with this application's activity component loaded into it
    /// (feature 029, FR-012e). On by default, so the badge works unconfigured.
    ///
    /// Application-wide on purpose: the component does not vary by project or session, so neither
    /// does the decision about it. Off, Pi launches without it and the badge reads `Unknown` — the
    /// user's choice, not a fault (FR-012f). Service-owned beside `default_ai_cli`, because the
    /// service is what spawns the session and so the only side that acts on it.
    #[serde(default = "default_pi_activity_component")]
    pub pi_activity_component: bool,
    /// Whether the service saves terminal output to disk so a service restart keeps it
    /// (feature 041, FR-026). On by default; off, the saved histories are deleted and nothing more
    /// is written. Service-owned: the service is what writes the files.
    #[serde(default = "default_save_terminal_history")]
    pub save_terminal_history: bool,
    /// Whether new sessions are bound to the service's tool server (feature 034, FR-004).
    #[serde(default = "default_tool_server_enabled")]
    pub tool_server_enabled: bool,
    /// Whether agents may read and type into other sessions of their project (feature 034,
    /// FR-016). `Auto` by default. Service-owned like the binding toggle, but read on every tool
    /// request rather than at spawn, so a change applies to sessions already running.
    #[serde(default)]
    pub cross_session_access: CrossSessionAccess,
    /// The ordered label-to-type mapping an issue pick reads (feature 034, FR-016, FR-017).
    ///
    /// Read leniently, as the stored document is: an entry that does not parse is dropped and the
    /// rest kept; a value that is not a list reads as the default table (see `known_entries`).
    #[serde(default = "default_mapping", deserialize_with = "known_entries")]
    pub issue_label_types: Vec<LabelTypeEntry>,
    /// Whether pull request status is read from GitHub and shown on worktree rows (feature 040,
    /// FR-029). Off until the user turns it on (FR-030): nothing is sent to GitHub for this
    /// feature before then. Service-owned like the binding toggle, so every window follows it.
    #[serde(default)]
    pub pr_status_enabled: bool,
    /// Whether new sessions are guarded to their own worktree (feature 1210, FR-008). On by
    /// default; off, sessions launched afterwards get no `PreToolUse` guard and no
    /// `permissions.deny` rules. Service-owned: the spawn reads it, and running sessions keep
    /// the state they launched with.
    #[serde(default = "default_worktree_guard_enabled")]
    pub worktree_guard_enabled: bool,
    /// Whether the Jira integration is on (feature 533, FR-002). Off until the user turns it on:
    /// while off nothing is sent to Jira and no Jira control is shown (FR-001). Service-owned so
    /// every window follows one switch (FR-022).
    #[serde(default)]
    pub jira_enabled: bool,
    /// Each project's Jira connection, keyed by the project path as the wire names it (feature
    /// 533, FR-006). Never holds a token. An entry that no longer parses is dropped on read.
    #[serde(default, deserialize_with = "crate::jira::known_values")]
    pub jira_connections: BTreeMap<String, JiraConnection>,
    /// Each project's Jira base URL, which its Overview cards build Jira issue links on (feature
    /// 1122, FR-010), keyed as `jira_connections` is. Independent of the connection. An entry
    /// that no longer parses is dropped on read: that project reads as unset.
    #[serde(
        default,
        deserialize_with = "crate::jira::known_values",
        skip_serializing_if = "BTreeMap::is_empty"
    )]
    pub jira_base_urls: BTreeMap<String, JiraBaseUrl>,
    /// Each project's setup command, run in every new worktree of that project (feature 1289,
    /// FR-009, contract C3), keyed as `jira_connections` is. Entered in the app, never read from
    /// the repository. The client writes it; a daemon's service-settings write keeps what is on
    /// disk. A blank entry is dropped on read: that project has no command.
    #[serde(
        default,
        deserialize_with = "crate::jira::known_values",
        skip_serializing_if = "BTreeMap::is_empty"
    )]
    pub setup_commands: BTreeMap<String, SetupCommand>,
    /// Whether the GitHub issues integration is on (feature 1102, FR-001). On by default, and in a
    /// file written before the switch (FR-007). While off, no GitHub issue request is made and the
    /// New worktree form offers no GitHub issue source. Service-owned so every window follows one
    /// switch (FR-008).
    #[serde(default = "default_github_issues_enabled")]
    pub github_issues_enabled: bool,
    /// Whether a session that comes to await input while not in view raises a desktop
    /// notification (feature 039, FR-026). One switch for every AI CLI (FR-028). Service-owned,
    /// so every window follows it at once; unread marks do not depend on it (FR-017).
    #[serde(default = "default_desktop_notifications")]
    pub desktop_notifications: bool,
    /// Which kinds of event notify while `desktop_notifications` is on (feature 613, FR-010,
    /// FR-011). Service-owned like the master switch.
    #[serde(default)]
    pub notification_kinds: NotificationKinds,
    /// How long, in seconds, a turn must last for its end to be **Long task finished** rather
    /// than **Turn finished** (feature 613, FR-025). 10–3600, 60 by default. Service-owned like
    /// the switches, and read at every turn end, so a change applies without a restart (FR-013).
    #[serde(default = "default_long_task_threshold_secs")]
    pub long_task_threshold_secs: u64,
    /// How the Changes view lays out a diff (feature 482, US1 s4, R12). Service-owned so every
    /// window and the next start keep the last choice.
    #[serde(default)]
    pub diff_layout: DiffLayout,
    /// Whether the Claude plan's usage is read from the sessions the app runs and shown (feature
    /// 490, FR-001, FR-002). Service-owned: the daemon writes each session's settings file from it.
    #[serde(default = "default_plan_usage_enabled")]
    pub plan_usage_enabled: bool,
    /// The plan-usage warning threshold, a whole percent from 50 to 100 (feature 490, FR-005).
    #[serde(default = "default_warn_percent")]
    pub plan_usage_warn_percent: u8,
    /// The sessions whose children the user folded away in the sidebar (feature 551, FR-011,
    /// FR-017). Stored as *folded*, so an origin never folded is expanded with no extra rule. Never
    /// pruned: a boot sees an empty or partial list before the first daemon snapshot, and pruning
    /// then would erase a fold the user left. Client-owned: the daemon never reads it.
    #[serde(default)]
    pub folded_origins: BTreeSet<SessionId>,
}

/// Reads the stored cross-session option. A value this build does not know (a mistyped hand edit,
/// or one a later build wrote) reads as `Off`, the value that allows nothing: it must not fail the
/// whole document, which would bring the option back as `Auto` (M6 review A, F6).
fn cross_session_access_or_off<'de, D>(deserializer: D) -> Result<CrossSessionAccess, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = serde_json::Value::deserialize(deserializer)?;
    Ok(serde_json::from_value(value).unwrap_or(CrossSessionAccess::Off))
}

/// The binding's default when a file predates it: on (FR-004).
fn default_tool_server_enabled() -> bool {
    true
}

/// The guard's default when a file predates it: on (feature 1210, FR-008).
fn default_worktree_guard_enabled() -> bool {
    true
}

/// The switch's default when a file predates it: on (FR-012e).
fn default_pi_activity_component() -> bool {
    true
}

/// The switch's default when a file predates it: on (feature 041, FR-026, FR-029).
fn default_save_terminal_history() -> bool {
    true
}

/// The GitHub issues switch when a file predates it: on (feature 1102, FR-007).
fn default_github_issues_enabled() -> bool {
    true
}

/// Desktop notifications when a file predates the switch, or cannot be read: on (feature 039,
/// FR-026).
fn default_desktop_notifications() -> bool {
    true
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            theme: ThemePreference::default(),
            reduce_motion: false,
            scrollback_lines: DEFAULT_SCROLLBACK_LINES,
            env_include_enabled: DEFAULT_ENV_INCLUDE_ENABLED,
            env_include_script_path: default_env_include_script_path_string(),
            env_include_timeout_secs: DEFAULT_ENV_INCLUDE_TIMEOUT_SECS,
            daemon: DaemonConfig::default(),
            daemons: migrated_registry(&DaemonConfig::default()).0,
            next_daemon_id: 2,
            legacy_default_daemon: Some(crate::daemons::DaemonId(1)),
            default_ai_cli: AiCli::default(),
            default_prompt_template: String::new(),
            pi_activity_component: default_pi_activity_component(),
            save_terminal_history: default_save_terminal_history(),
            tool_server_enabled: default_tool_server_enabled(),
            cross_session_access: CrossSessionAccess::default(),
            issue_label_types: default_mapping(),
            pr_status_enabled: false,
            worktree_guard_enabled: default_worktree_guard_enabled(),
            jira_enabled: false,
            jira_connections: BTreeMap::new(),
            jira_base_urls: BTreeMap::new(),
            setup_commands: BTreeMap::new(),
            github_issues_enabled: default_github_issues_enabled(),
            desktop_notifications: default_desktop_notifications(),
            notification_kinds: NotificationKinds::default(),
            long_task_threshold_secs: default_long_task_threshold_secs(),
            diff_layout: DiffLayout::default(),
            plan_usage_enabled: default_plan_usage_enabled(),
            plan_usage_warn_percent: crate::plan_usage::WarnPercent::DEFAULT,
            folded_origins: BTreeSet::new(),
        }
    }
}

/// The registry a pre-feature `daemon` block migrates to: one daemon, the legacy default.
fn migrated_registry(
    config: &DaemonConfig,
) -> (
    Vec<crate::daemons::DaemonEntry>,
    u32,
    Option<crate::daemons::DaemonId>,
) {
    let entry = crate::daemons::migrated_entry(config);
    let id = entry.id;
    (vec![entry], id.0 + 1, Some(id))
}

impl Settings {
    /// The registry these settings hold.
    pub fn registry(&self) -> crate::daemons::DaemonRegistry {
        crate::daemons::DaemonRegistry::new(self.daemons.clone(), self.next_daemon_id)
    }

    /// Store a registry back (its entries and the id counter).
    pub fn set_registry(&mut self, registry: crate::daemons::DaemonRegistry) {
        self.next_daemon_id = registry.next_id();
        self.daemons = registry.entries().to_vec();
    }

    /// Set the single-daemon `daemon` block and keep the registry entry that stands for it
    /// (the legacy default, else the first) in step, so the block still written for rollback
    /// never disagrees with the registry. The container name and port of an entry that already is
    /// a container are kept.
    pub fn set_single_daemon(&mut self, config: DaemonConfig) {
        use crate::daemons::{ContainerSettings, DaemonRuntime};
        // A default that names a removed entry leaves the block alone: removing a daemon must
        // not re-target the single-daemon block at whatever is first.
        let target = match self.legacy_default_daemon {
            Some(id) => Some(id),
            None => self.daemons.first().map(|d| d.id),
        };
        if let Some(entry) = target.and_then(|id| self.daemons.iter_mut().find(|d| d.id == id)) {
            if entry.runtime.is_startable() {
                entry.runtime = match config.placement {
                    PlacementKind::HostProcess => DaemonRuntime::Host,
                    PlacementKind::LocalSandbox => {
                        let (container_name, port) = match &entry.runtime {
                            DaemonRuntime::Container(c) => (c.container_name.clone(), c.port),
                            _ => (
                                crate::sandbox::CONTAINER_NAME.to_string(),
                                crate::endpoint::DEFAULT_SANDBOX_PORT,
                            ),
                        };
                        DaemonRuntime::Container(ContainerSettings {
                            profile: config.sandbox.clone(),
                            container_name,
                            port,
                            migration: None,
                        })
                    }
                };
            }
        }
        self.daemon = config;
    }

    /// Bring the legacy `daemon` block in step with the registry entry that stands for it (the
    /// legacy default, else the first): the inverse of [`Self::set_single_daemon`], run after the
    /// Daemons form saved that entry so the block kept for rollback does not disagree.
    ///
    /// A container entry sets `LocalSandbox` and takes its profile; a host entry sets
    /// `HostProcess` and keeps the block's profile; an SSH (or unknown) entry leaves the block
    /// alone.
    pub fn sync_legacy_daemon(&mut self) {
        use crate::daemons::DaemonRuntime;
        let target = self
            .legacy_default_daemon
            .filter(|id| self.daemons.iter().any(|d| d.id == *id))
            .or_else(|| self.daemons.first().map(|d| d.id));
        let Some(entry) = target.and_then(|id| self.daemons.iter().find(|d| d.id == id)) else {
            return;
        };
        match &entry.runtime {
            DaemonRuntime::Container(c) => {
                self.daemon.placement = PlacementKind::LocalSandbox;
                self.daemon.sandbox = c.profile.clone();
            }
            DaemonRuntime::Host => self.daemon.placement = PlacementKind::HostProcess,
            _ => {}
        }
    }
}

#[cfg(test)]
mod sync_legacy_daemon_tests {
    //! Feature 804, T001: the legacy block follows the registry entry the Daemons form saved.

    use super::*;
    use crate::daemons::DaemonRuntime;
    use crate::sandbox::runtime::RuntimeKind;
    use crate::ssh::SshSettings;

    fn with_runtime(runtime: DaemonRuntime) -> Settings {
        let mut s = Settings::default();
        s.daemons[0].runtime = runtime;
        s
    }

    #[test]
    fn a_container_entry_sets_local_sandbox_and_its_profile() {
        let profile = SandboxProfile {
            runtime: RuntimeKind::Podman,
            survive_logout: true,
            ..SandboxProfile::default()
        };
        let mut s = with_runtime(DaemonRuntime::Container(
            crate::daemons::ContainerSettings {
                profile: profile.clone(),
                container_name: "c".into(),
                port: 1234,
                migration: None,
            },
        ));
        s.daemon.placement = PlacementKind::HostProcess;
        s.sync_legacy_daemon();
        assert_eq!(s.daemon.placement, PlacementKind::LocalSandbox);
        assert_eq!(s.daemon.sandbox, profile);
    }

    #[test]
    fn a_host_entry_sets_host_process_and_keeps_the_profile() {
        let mut s = with_runtime(DaemonRuntime::Host);
        s.daemon.sandbox.survive_logout = true;
        let kept = s.daemon.sandbox.clone();
        s.sync_legacy_daemon();
        assert_eq!(s.daemon.placement, PlacementKind::HostProcess);
        assert_eq!(s.daemon.sandbox, kept);
    }

    #[test]
    fn an_ssh_entry_leaves_the_block_untouched() {
        let target = crate::ssh::SshTarget::new("box", None, None, None).unwrap();
        let mut s = with_runtime(DaemonRuntime::Ssh(SshSettings { target }));
        s.daemon.sandbox.survive_logout = true;
        let before = s.daemon.clone();
        s.sync_legacy_daemon();
        assert_eq!(s.daemon, before);
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

/// The on-disk shape. Unknown fields are ignored on read and **every** field takes its serde
/// default when the document omits it — both give forward compatibility (settings-schema contract).
#[derive(Debug, Serialize, Deserialize)]
struct StoredSettings {
    /// Missing → `0`, the version number no build has ever written (003 BUG-003).
    ///
    /// This field is the one thing in the document nothing reads: no migration and no gate
    /// consumes it, and `into_settings` drops it. Without a default it was nonetheless the one
    /// field a document had to carry, so a settings file written by hand — the one case where the
    /// number cannot be guessed — was rejected as corrupt, moved to `settings.json.bak`, and
    /// replaced by `Settings::default()` with nothing said.
    ///
    /// `0` rather than [`SETTINGS_VERSION`] because it is the honest answer: the document named no
    /// version, and saying so is better than recording a claim that this build wrote it.
    ///
    /// The one hazard in that choice belongs to whoever writes the first migration gate, so it is
    /// written down here rather than left to be found: `0` means *unknown*, not *older than v1*.
    /// The obvious gate, `if settings_version < SETTINGS_VERSION { migrate }`, would run every
    /// migration ever written over a hand-written document that is already in the current shape.
    /// A gate must treat `0` as "no version stated" and leave the document alone.
    #[serde(default)]
    settings_version: u32,
    #[serde(default)]
    theme: ThemePreference,
    /// Missing in files written before feature 970 → off.
    #[serde(default)]
    reduce_motion: bool,
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
    /// Feature 491: absent in every file written before it, which is the migration's gate: a
    /// document that has the key (even an empty list) is authoritative and never migrated again.
    /// `settings_version` does not move, by the argument the other additive fields record.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    daemons: Option<Vec<crate::daemons::DaemonEntry>>,
    #[serde(default)]
    next_daemon_id: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    legacy_default_daemon: Option<crate::daemons::DaemonId>,
    /// Missing in pre-026 files → defaults to `ClaudeCode`, which is the right answer for every
    /// settings file written before this feature (feature 026, FR-003).
    ///
    /// `settings_version` deliberately does **not** move for this. The `#[serde(default)]`
    /// argument that spares `schema_version` in `store.rs` (research R8) spares this third version
    /// number for the same reason: a bump would cost a migration path to express a default.
    #[serde(default)]
    default_ai_cli: AiCli,
    /// Missing in files written before 1161 → empty. Additive, so `settings_version` stays.
    #[serde(default)]
    default_prompt_template: String,
    /// Missing in pre-029 files → on, the requirement's default (FR-012e). Additive and defaulted,
    /// so `settings_version` does not move for it either.
    #[serde(default = "default_pi_activity_component")]
    pi_activity_component: bool,
    /// Missing in pre-041 files → on (FR-026, FR-029). Additive and defaulted, so
    /// `settings_version` does not move for it either.
    #[serde(default = "default_save_terminal_history")]
    save_terminal_history: bool,
    /// Missing in pre-034 files → on, the requirement's default (FR-004). Additive and defaulted,
    /// so `settings_version` does not move for it either.
    #[serde(default = "default_tool_server_enabled")]
    tool_server_enabled: bool,
    /// Missing in files written before the option → `Auto`, the requirement's default (FR-016).
    /// Additive and defaulted, so `settings_version` does not move for it either. A value this
    /// build does not know reads as `Off`: see [`cross_session_access_or_off`].
    #[serde(default, deserialize_with = "cross_session_access_or_off")]
    cross_session_access: CrossSessionAccess,
    /// Missing in pre-034 files → the default table (FR-021); `[]` stays `[]`. An entry naming a
    /// type this build does not know (or that does not parse at all) is dropped on read and the
    /// rest kept (R10), and a value that is not a list reads as the default table: one hand edit
    /// must not send the whole document to `.bak`. Additive and defaulted, so
    /// `settings_version` does not move for it.
    #[serde(default = "default_mapping", deserialize_with = "known_entries")]
    issue_label_types: Vec<LabelTypeEntry>,
    /// Missing in pre-040 files → off, the requirement's default (FR-030). Additive and
    /// defaulted, so `settings_version` does not move for it either.
    #[serde(default)]
    pr_status_enabled: bool,
    /// Missing in files written before feature 1210 → on (FR-008). Additive and defaulted, so
    /// `settings_version` does not move for it either.
    #[serde(default = "default_worktree_guard_enabled")]
    worktree_guard_enabled: bool,
    /// Missing in files written before feature 533 → off (FR-002). Additive and defaulted, so
    /// `settings_version` does not move for it either.
    #[serde(default)]
    jira_enabled: bool,
    /// Missing → none; an entry that no longer parses is dropped and the rest kept (FR-006).
    #[serde(default, deserialize_with = "crate::jira::known_values")]
    jira_connections: BTreeMap<String, JiraConnection>,
    /// Missing in files written before feature 1122 → none; an entry that no longer parses is
    /// dropped and the rest kept (FR-010). Written only when not empty. Additive and defaulted, so
    /// `settings_version` does not move.
    #[serde(
        default,
        deserialize_with = "crate::jira::known_values",
        skip_serializing_if = "BTreeMap::is_empty"
    )]
    jira_base_urls: BTreeMap<String, JiraBaseUrl>,
    /// Missing in files written before feature 1289 → none; a blank entry is dropped and the rest
    /// kept. Written only when not empty. Additive and defaulted, so `settings_version` does not
    /// move.
    #[serde(
        default,
        deserialize_with = "crate::jira::known_values",
        skip_serializing_if = "BTreeMap::is_empty"
    )]
    setup_commands: BTreeMap<String, SetupCommand>,
    /// Missing in files written before feature 1102 → on (FR-007). Additive and defaulted, so
    /// `settings_version` does not move.
    #[serde(default = "default_github_issues_enabled")]
    github_issues_enabled: bool,
    /// Missing in files written before feature 039 → on, the requirement's default (FR-026).
    /// Additive and defaulted, so `settings_version` does not move for it either.
    #[serde(default = "default_desktop_notifications")]
    desktop_notifications: bool,
    /// Missing in files written before feature 613 → the default kinds; a missing field inside
    /// takes that kind's default (FR-010). Additive and defaulted, so `settings_version` does not
    /// move for it either.
    #[serde(default)]
    notification_kinds: NotificationKinds,
    /// Missing in files written before feature 613's threshold → 60 (FR-025); out of 10–3600 →
    /// clamped on read (FR-026). Additive and defaulted, so `settings_version` does not move.
    #[serde(default = "default_long_task_threshold_secs")]
    long_task_threshold_secs: u64,
    /// Missing in files written before feature 482 → unified (R12). Additive and defaulted, so
    /// `settings_version` does not move for it either.
    #[serde(default)]
    diff_layout: DiffLayout,
    /// Missing in files written before feature 490 → on (FR-001). Additive and defaulted, so
    /// `settings_version` does not move.
    #[serde(default = "default_plan_usage_enabled")]
    plan_usage_enabled: bool,
    /// Missing → 80; out of 50–100 → clamped on read (FR-005). Wide on disk so a hand-edited
    /// 300 is clamped rather than failing the whole document.
    #[serde(default = "default_plan_usage_warn_percent")]
    plan_usage_warn_percent: u64,
    #[serde(default)]
    folded_origins: BTreeSet<SessionId>,
}

/// The mapping's entries that parse, in order; an entry with an unknown `type` token is skipped
/// (R10). A value that is not a list at all reads as the default table, as an absent one does: one
/// bad hand edit must not send the whole document to `.bak`.
fn known_entries<'de, D>(deserializer: D) -> Result<Vec<LabelTypeEntry>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    Ok(match serde_json::Value::deserialize(deserializer)? {
        serde_json::Value::Array(entries) => entries
            .into_iter()
            .filter_map(|entry| serde_json::from_value(entry).ok())
            .collect(),
        _ => default_mapping(),
    })
}

impl StoredSettings {
    fn from_settings(settings: &Settings) -> Self {
        Self {
            settings_version: SETTINGS_VERSION,
            theme: settings.theme,
            reduce_motion: settings.reduce_motion,
            scrollback_lines: settings.scrollback_lines,
            env_include_enabled: settings.env_include_enabled,
            env_include_script_path: settings.env_include_script_path.clone(),
            env_include_timeout_secs: settings.env_include_timeout_secs,
            daemon: settings.daemon.clone(),
            daemons: Some(settings.daemons.clone()),
            next_daemon_id: settings.next_daemon_id,
            legacy_default_daemon: settings.legacy_default_daemon,
            default_ai_cli: settings.default_ai_cli,
            default_prompt_template: settings.default_prompt_template.clone(),
            pi_activity_component: settings.pi_activity_component,
            save_terminal_history: settings.save_terminal_history,
            tool_server_enabled: settings.tool_server_enabled,
            cross_session_access: settings.cross_session_access,
            issue_label_types: settings.issue_label_types.clone(),
            pr_status_enabled: settings.pr_status_enabled,
            worktree_guard_enabled: settings.worktree_guard_enabled,
            jira_enabled: settings.jira_enabled,
            jira_connections: settings.jira_connections.clone(),
            jira_base_urls: settings.jira_base_urls.clone(),
            setup_commands: settings.setup_commands.clone(),
            github_issues_enabled: settings.github_issues_enabled,
            desktop_notifications: settings.desktop_notifications,
            notification_kinds: settings.notification_kinds,
            long_task_threshold_secs: settings.long_task_threshold_secs,
            diff_layout: settings.diff_layout,
            plan_usage_enabled: settings.plan_usage_enabled,
            plan_usage_warn_percent: u64::from(settings.plan_usage_warn_percent),
            folded_origins: settings.folded_origins.clone(),
        }
    }

    fn into_settings(self) -> Settings {
        let (daemons, next_daemon_id, legacy_default_daemon) = match self.daemons {
            Some(daemons) => {
                let floor = daemons.iter().map(|d| d.id.0 + 1).max().unwrap_or(1);
                (
                    daemons,
                    self.next_daemon_id.max(floor).max(1),
                    self.legacy_default_daemon,
                )
            }
            // R5: a pre-feature document becomes a registry of one daemon, once.
            None => migrated_registry(&self.daemon),
        };
        Settings {
            theme: self.theme,
            reduce_motion: self.reduce_motion,
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
            daemons,
            next_daemon_id,
            legacy_default_daemon,
            // Not clamped, and not checked against availability: unlike the two numbers above,
            // there is no invalid value to repair — only a CLI that may not be installed today,
            // which is the user's choice to keep (research R11).
            default_ai_cli: self.default_ai_cli,
            default_prompt_template: self.default_prompt_template,
            pi_activity_component: self.pi_activity_component,
            save_terminal_history: self.save_terminal_history,
            tool_server_enabled: self.tool_server_enabled,
            cross_session_access: self.cross_session_access,
            issue_label_types: self.issue_label_types,
            pr_status_enabled: self.pr_status_enabled,
            worktree_guard_enabled: self.worktree_guard_enabled,
            jira_enabled: self.jira_enabled,
            jira_connections: self.jira_connections,
            jira_base_urls: self.jira_base_urls,
            setup_commands: self.setup_commands,
            github_issues_enabled: self.github_issues_enabled,
            desktop_notifications: self.desktop_notifications,
            notification_kinds: self.notification_kinds,
            long_task_threshold_secs: clamp_long_task_threshold(self.long_task_threshold_secs),
            diff_layout: self.diff_layout,
            plan_usage_enabled: self.plan_usage_enabled,
            plan_usage_warn_percent: clamp_plan_usage_warn_percent(self.plan_usage_warn_percent),
            folded_origins: self.folded_origins,
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
        crate::paths::data_subdir("settings.json").map(Self::at)
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
        Some(crate::store::backup_path_for(&self.path))
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
                let _ = std::fs::rename(&self.path, crate::store::backup_path_for(&self.path));
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
        crate::store::write_atomic(&self.path, &json)
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

#[cfg(test)]
mod desktop_notifications_tests {
    //! Feature 039, story 4: the **Desktop notifications** switch as it is stored (FR-026, FR-028).

    use super::*;

    fn store_in(dir: &tempfile::TempDir) -> JsonFileSettingsStore {
        JsonFileSettingsStore::at(dir.path().join("settings.json"))
    }

    /// U42 (FR-026, US4-1, Edge: settings file unreadable): the defaults have the switch on.
    #[test]
    fn desktop_notifications_are_on_by_default() {
        assert!(
            Settings::default().desktop_notifications,
            "a fresh installation, and a settings file that cannot be read, notify"
        );
    }

    /// U43 (FR-026): a file written before the switch existed reads as on.
    #[test]
    fn a_settings_file_without_the_field_reads_as_on() {
        let dir = tempfile::tempdir().unwrap();
        let store = store_in(&dir);
        std::fs::write(
            dir.path().join("settings.json"),
            r#"{ "settings_version": 4, "theme": "dark", "tool_server_enabled": false }"#,
        )
        .unwrap();

        let outcome = store.load();

        assert_eq!(outcome.status, LoadStatus::Loaded);
        assert!(
            outcome.settings.desktop_notifications,
            "the user of an older file never turned notifications off"
        );
    }

    /// U44 (FR-026, US4-4): off survives a save and a load.
    #[test]
    fn desktop_notifications_off_survives_a_save_and_load() {
        let dir = tempfile::tempdir().unwrap();
        let store = store_in(&dir);

        store
            .save(&Settings {
                desktop_notifications: false,
                ..Settings::default()
            })
            .unwrap();
        // Read the file too: with a stub that always loads `false`, the load alone would pass.
        let written: serde_json::Value =
            serde_json::from_slice(&std::fs::read(dir.path().join("settings.json")).unwrap())
                .unwrap();

        assert_eq!(
            written.get("desktop_notifications"),
            Some(&serde_json::Value::Bool(false)),
            "the file holds the user's choice"
        );
        assert!(
            !store.load().settings.desktop_notifications,
            "loading the file does not turn notifications back on"
        );
    }

    /// U45 (FR-028, US4-6): one key names notifications, it is a switch, and no key holds a value
    /// per AI CLI.
    #[test]
    fn the_stored_settings_hold_one_notification_key_and_none_per_ai_cli() {
        let dir = tempfile::tempdir().unwrap();
        let store = store_in(&dir);
        store.save(&Settings::default()).unwrap();
        let written: serde_json::Value =
            serde_json::from_slice(&std::fs::read(dir.path().join("settings.json")).unwrap())
                .unwrap();
        let document = written.as_object().expect("settings.json is an object");

        let naming_notifications: Vec<&str> = document
            .keys()
            .map(String::as_str)
            .filter(|key| key.contains("notif"))
            .collect();
        // Feature 613 adds the per-kind switches beside the master switch (FR-011); neither is
        // per AI CLI.
        assert_eq!(
            naming_notifications,
            ["desktop_notifications", "notification_kinds"],
            "one master switch and one set of kinds, for every AI CLI"
        );
        assert!(
            document["desktop_notifications"].is_boolean(),
            "the switch is on or off, not a value per AI CLI"
        );
        for cli in ["claude", "copilot", "pi_coding", "pi-coding"] {
            assert!(
                !document
                    .keys()
                    .any(|key| key.contains("notif") && key.contains(cli)),
                "no notification key names the AI CLI `{cli}`"
            );
        }
    }
}

#[cfg(test)]
mod notification_kinds_tests {
    //! Feature 613: the per-kind switches as they are stored (FR-010, US2.8).

    use super::*;
    use crate::attention::{NotificationKind, NotificationKinds};

    fn store_in(dir: &tempfile::TempDir) -> JsonFileSettingsStore {
        JsonFileSettingsStore::at(dir.path().join("settings.json"))
    }

    fn load_document(document: &str) -> Settings {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("settings.json"), document).unwrap();
        let outcome = store_in(&dir).load();
        assert_eq!(outcome.status, LoadStatus::Loaded, "the document loads");
        outcome.settings
    }

    #[test]
    fn a_fresh_installation_has_the_default_kinds() {
        assert_eq!(
            Settings::default().notification_kinds,
            NotificationKinds::default(),
            "FR-010"
        );
    }

    #[test]
    fn a_file_from_before_the_feature_keeps_its_master_switch_and_gets_the_default_kinds() {
        for master in [false, true] {
            let settings = load_document(&format!(
                r#"{{ "settings_version": 4, "theme": "dark", "desktop_notifications": {master} }}"#
            ));
            assert_eq!(
                settings.desktop_notifications, master,
                "the stored master switch is kept (FR-010, US2.8)"
            );
            assert_eq!(
                settings.notification_kinds,
                NotificationKinds::default(),
                "an older file gets the default kinds (FR-010)"
            );
        }
    }

    #[test]
    fn a_kinds_object_missing_a_field_gets_that_fields_default_and_keeps_the_others() {
        let settings = load_document(
            r#"{ "settings_version": 4, "notification_kinds": { "needs_permission": false, "session_error": false, "turn_finished": true } }"#,
        );
        assert_eq!(
            settings.notification_kinds,
            NotificationKinds {
                needs_permission: false,
                session_error: false,
                long_task_finished: true,
                turn_finished: true,
            },
            "only the missing field takes its default"
        );
    }

    #[test]
    fn the_kinds_survive_a_save_and_load() {
        let dir = tempfile::tempdir().unwrap();
        let store = store_in(&dir);
        let mut kinds = NotificationKinds::default();
        for kind in NotificationKind::ALL {
            kinds.set(kind, !kind.default_on());
        }
        store
            .save(&Settings {
                notification_kinds: kinds,
                ..Settings::default()
            })
            .unwrap();
        assert_eq!(
            store.load().settings.notification_kinds,
            kinds,
            "all four values come back as saved"
        );
    }

    #[test]
    fn an_unreadable_file_falls_back_to_the_default_kinds() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("settings.json"), "{ not json").unwrap();
        let outcome = store_in(&dir).load();
        assert_ne!(outcome.status, LoadStatus::Loaded, "the file does not load");
        assert_eq!(
            outcome.settings.notification_kinds,
            NotificationKinds::default(),
            "Edge Cases: settings file unreadable"
        );
    }
}

#[cfg(test)]
mod reduce_motion_tests {
    //! Feature 970, story 3: the **Reduce motion** switch as it is stored (FR-008).

    use super::*;

    fn store_in(dir: &tempfile::TempDir) -> JsonFileSettingsStore {
        JsonFileSettingsStore::at(dir.path().join("settings.json"))
    }

    /// FR-008: off by default, and a file written before the key existed loads with it off and
    /// every other field as before.
    #[test]
    fn reduce_motion_defaults_off() {
        assert!(
            !Settings::default().reduce_motion,
            "a fresh installation animates"
        );

        let dir = tempfile::tempdir().unwrap();
        let store = store_in(&dir);
        std::fs::write(
            dir.path().join("settings.json"),
            r#"{ "settings_version": 4, "theme": "dark", "desktop_notifications": false }"#,
        )
        .unwrap();

        let outcome = store.load();

        assert_eq!(outcome.status, LoadStatus::Loaded);
        assert!(
            !outcome.settings.reduce_motion,
            "an older file never asked for it"
        );
        assert_eq!(
            outcome.settings,
            Settings {
                theme: crate::theme::ThemePreference::Dark,
                desktop_notifications: false,
                ..Settings::default()
            },
            "every other field reads as it did"
        );
    }

    /// FR-008: on survives a save and a load, and the file holds the choice.
    #[test]
    fn reduce_motion_round_trips() {
        let dir = tempfile::tempdir().unwrap();
        let store = store_in(&dir);

        store
            .save(&Settings {
                reduce_motion: true,
                ..Settings::default()
            })
            .unwrap();
        let written: serde_json::Value =
            serde_json::from_slice(&std::fs::read(dir.path().join("settings.json")).unwrap())
                .unwrap();

        assert_eq!(
            written.get("reduce_motion"),
            Some(&serde_json::Value::Bool(true)),
            "the file holds the user's choice"
        );
        assert!(store.load().settings.reduce_motion, "and loads back on");
    }
}

#[cfg(test)]
mod setup_commands_tests {
    //! Feature 1289, A14: each project's setup command is kept in `settings.json` (FR-009,
    //! contract C3), additively, so `settings_version` does not move.

    use super::*;

    fn store_in(dir: &tempfile::TempDir) -> JsonFileSettingsStore {
        JsonFileSettingsStore::at(dir.path().join("settings.json"))
    }

    fn written(dir: &tempfile::TempDir) -> serde_json::Value {
        serde_json::from_slice(&std::fs::read(dir.path().join("settings.json")).unwrap()).unwrap()
    }

    fn command(text: &str) -> SetupCommand {
        SetupCommand::parse(text).unwrap()
    }

    #[test]
    fn a_setup_command_round_trips_through_the_file() {
        let dir = tempfile::tempdir().unwrap();
        let store = store_in(&dir);
        let settings = Settings {
            setup_commands: BTreeMap::from([(
                "/home/me/src/app".to_owned(),
                command("mise trust && mise install"),
            )]),
            ..Settings::default()
        };

        store.save(&settings).unwrap();

        assert_eq!(
            written(&dir).get("setup_commands"),
            Some(&serde_json::json!({"/home/me/src/app": "mise trust && mise install"}))
        );
        assert_eq!(store.load().settings, settings);
    }

    #[test]
    fn no_setup_command_writes_no_key() {
        let dir = tempfile::tempdir().unwrap();
        store_in(&dir).save(&Settings::default()).unwrap();
        assert_eq!(written(&dir).get("setup_commands"), None);
    }

    #[test]
    fn a_file_without_the_key_loads_no_commands_and_keeps_its_version() {
        let dir = tempfile::tempdir().unwrap();
        let store = store_in(&dir);
        std::fs::write(
            dir.path().join("settings.json"),
            r#"{ "settings_version": 4, "theme": "dark" }"#,
        )
        .unwrap();

        let outcome = store.load();

        assert_eq!(outcome.status, LoadStatus::Loaded);
        assert!(outcome.settings.setup_commands.is_empty());
        store.save(&outcome.settings).unwrap();
        assert_eq!(
            written(&dir).get("settings_version"),
            Some(&serde_json::json!(SETTINGS_VERSION))
        );
        assert_eq!(
            SETTINGS_VERSION, 4,
            "an additive field does not move the version"
        );
    }

    #[test]
    fn a_blank_command_in_the_file_is_dropped_and_the_rest_kept() {
        let dir = tempfile::tempdir().unwrap();
        let store = store_in(&dir);
        std::fs::write(
            dir.path().join("settings.json"),
            r#"{ "settings_version": 4, "setup_commands": { "/a": "  ", "/b": "make" } }"#,
        )
        .unwrap();

        let outcome = store.load();

        assert_eq!(
            outcome.status,
            LoadStatus::Loaded,
            "one bad entry is not a bad file"
        );
        assert_eq!(
            outcome.settings.setup_commands,
            BTreeMap::from([("/b".to_owned(), command("make"))])
        );
    }

    #[test]
    fn a_change_sets_or_removes_one_project_only() {
        let mut map = BTreeMap::from([("/other".to_owned(), command("make"))]);

        SetupCommandChange {
            project: "/app".to_owned(),
            command: Some(command("mise install")),
        }
        .apply(&mut map);
        assert_eq!(map.get("/app"), Some(&command("mise install")));

        SetupCommandChange {
            project: "/app".to_owned(),
            command: None,
        }
        .apply(&mut map);
        assert_eq!(map.get("/app"), None);
        assert_eq!(
            map.get("/other"),
            Some(&command("make")),
            "another project's entry stays"
        );
    }
}
