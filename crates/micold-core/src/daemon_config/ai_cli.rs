//! How one parsed `[[ai.cli]]` entry maps onto its CLI's launch and placement (contract
//! "`[[ai.cli]]`", FR-013/FR-013a-c/FR-013e, data-model V7–V11): which env variable `api_base_url`
//! and `api_key_file` set, that Claude Code's key reaches it through `apiKeyHelper` instead of an
//! env variable, and where `settings_file`/`place` land under the CLI's config dir.
//!
//! Pure: nothing here reads a file or spawns anything. `crates/micold-daemon/src/state.rs` reads
//! `api_key_file`'s content and builds the actual launch; this module only says which variable
//! that content belongs in, and what `args`/`env`/`model` this entry adds on top of the CLI's own
//! — never a replacement (`validate.rs`'s V7–V10 already refuse what a CLI cannot take).

use std::path::{Path, PathBuf};

use super::{AiCli, CliId};

/// The endpoint variable `api_base_url` sets, for the CLIs that have one (contract row
/// `api_base_url`). `None` for a CLI `validate.rs`'s V7 already refuses the key for.
pub fn api_base_url_variable(cli: CliId) -> Option<&'static str> {
    match cli {
        CliId::ClaudeCode => Some("ANTHROPIC_BASE_URL"),
        CliId::Codex => Some("OPENAI_BASE_URL"),
        CliId::Copilot | CliId::Pi | CliId::Opencode => None,
    }
}

/// The variable `api_key_file`'s content is read into at session start, for the CLIs that take
/// their key that way. `None` for Claude Code, whose key reaches it through `apiKeyHelper`
/// instead ([`uses_api_key_helper`]) — never through this or any other variable (contract row
/// `api_key_file`: "Never on the command line"; FR-013, SC-008) — and for a CLI `validate.rs`'s V7
/// already refuses `api_key_file` for (Copilot, Pi).
pub fn api_key_env_variable(cli: CliId) -> Option<&'static str> {
    match cli {
        CliId::Codex | CliId::Opencode => Some("OPENAI_API_KEY"),
        CliId::ClaudeCode | CliId::Copilot | CliId::Pi => None,
    }
}

/// Whether `cli`'s `api_key_file` reaches the session through `apiKeyHelper` in the per-session
/// settings (contract row `api_key_file`), read on each use rather than once at session start, so
/// a replaced key file is picked up on the CLI's next call (FR-013, SC-008). Only Claude Code has
/// this mechanism today.
pub fn uses_api_key_helper(cli: CliId) -> bool {
    cli == CliId::ClaudeCode
}

/// The variable the daemon hands Claude Code's launched session the configured `api_key_file`'s
/// **path** in (never its content — `apiKeyHelper` reads that itself, on each use): the `ai-key
/// claude_code` subcommand `apiKeyHelper` names (`crates/micold-daemon/src/ai_key.rs`) is a short
/// subprocess with no daemon state of its own, so it reads this from the environment it inherits
/// from the session that spawned it, rather than re-finding the daemon's configuration file.
pub const CLAUDE_API_KEY_FILE_VAR: &str = "MICOLD_CLAUDE_API_KEY_FILE";

/// The extra launch args and env one configured `[[ai.cli]]` entry contributes, on top of the
/// CLI's own (contract rows `args`, `model`, `api_base_url`, `env`): additions, never a
/// replacement. `api_key` is `api_key_file`'s already-read content, when the entry sets one and
/// the CLI does not use `apiKeyHelper` for it ([`uses_api_key_helper`]) — reading the file is
/// `crates/micold-daemon/src/state.rs`'s job, not this pure module's.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LaunchOverlay {
    /// Appended after the CLI's own launch args, before the tool-server binding (contract row
    /// `args`).
    pub args: Vec<String>,
    /// Added to the CLI's launch environment, after `launch_env` (contract row `env`).
    pub env: Vec<(String, String)>,
    /// The default model, for the one CLI with a model argument (contract row `model`).
    pub model: Option<String>,
}

/// Build `cli`'s [`LaunchOverlay`]. `api_key` is `cli.api_key_file`'s content, already read, or
/// `None` when there is no file or the CLI reads it through `apiKeyHelper` instead.
pub fn launch_overlay(cli: &AiCli, api_key: Option<&str>) -> LaunchOverlay {
    let mut env = Vec::new();
    if let Some(url) = &cli.api_base_url {
        if let Some(var) = api_base_url_variable(cli.id) {
            env.push((var.to_string(), url.clone()));
        }
    }
    if let (Some(key), Some(var)) = (api_key, api_key_env_variable(cli.id)) {
        env.push((var.to_string(), key.to_string()));
    }
    env.extend(cli.env.iter().map(|(k, v)| (k.clone(), v.clone())));
    LaunchOverlay {
        args: cli.args.clone(),
        env,
        model: (cli.id == CliId::ClaudeCode)
            .then(|| cli.model.clone())
            .flatten(),
    }
}

/// `place.as`'s target under `config_dir`, or `None` when it would escape (contract row `place`;
/// `validate.rs`'s V10 already refuses an escaping `place.as` at parse time, so a config that
/// reaches here is already known good — this is the belt to that suspenders, for a caller with no
/// validated [`super::DaemonConfig`] of its own to lean on).
pub fn place_target(config_dir: &Path, as_path: &Path) -> Option<PathBuf> {
    if is_absolute_on_any_platform(as_path)
        || as_path
            .components()
            .any(|c| matches!(c, std::path::Component::ParentDir))
    {
        return None;
    }
    Some(config_dir.join(as_path))
}

/// Whether `path` is absolute on *this* host (`Path::is_absolute`) or would be on another one:
/// a leading `/` or `\`, or a drive letter (`C:...`). The config is authored once and placed by
/// whichever platform's daemon reads it, so a path that only one platform's `is_absolute` would
/// catch is still an escape to guard against (T062's "`place.as` cannot escape the CLI's config
/// dir" does not say "on this platform only" — the Windows CI run that first found this treated a
/// leading-slash `/absolute` as relative, since `Path::is_absolute` is Windows's own rule, not
/// POSIX's).
fn is_absolute_on_any_platform(path: &Path) -> bool {
    if path.is_absolute() {
        return true;
    }
    let s = path.to_string_lossy();
    if s.starts_with('/') || s.starts_with('\\') {
        return true;
    }
    let bytes = s.as_bytes();
    bytes.len() >= 2 && bytes[0].is_ascii_alphabetic() && bytes[1] == b':'
}

/// `settings_file`'s file name under the CLI's config dir in the sandbox home (contract row
/// `settings_file`): Claude `settings.json`, Codex `config.toml` (`$CODEX_HOME` already *is* the
/// config dir there), OpenCode `opencode.json`. `None` for Copilot and Pi, which the contract
/// marks C,K-unsupported — `validate.rs`'s V10 refuses `settings_file` for them on no kind at all,
/// so this only names the file for the CLIs that can have one.
pub fn settings_file_name(cli: CliId) -> Option<&'static str> {
    match cli {
        CliId::ClaudeCode => Some("settings.json"),
        CliId::Codex => Some("config.toml"),
        CliId::Opencode => Some("opencode.json"),
        CliId::Copilot | CliId::Pi => None,
    }
}

/// The environment variable a host or SSH session's config dir is pointed at through (T065,
/// research D9's per-CLI table): `CLAUDE_CONFIG_DIR`, `CODEX_HOME`, OpenCode's `XDG_DATA_HOME`
/// (one level above its own `opencode` subdirectory — [`host_config_dir`] accounts for that).
/// `None` for Copilot and Pi, which have no shareable sign-in to isolate
/// (`validate.rs`'s V7 already refuses `sign_in_file` for them).
pub fn host_config_dir_variable(cli: CliId) -> Option<&'static str> {
    match cli {
        CliId::ClaudeCode => Some("CLAUDE_CONFIG_DIR"),
        CliId::Codex => Some("CODEX_HOME"),
        CliId::Opencode => Some("XDG_DATA_HOME"),
        CliId::Copilot | CliId::Pi => None,
    }
}

/// `<data_dir>/ai/<id>` (Claude, Codex), or `<data_dir>/ai/opencode/data` for OpenCode, whose
/// [`host_config_dir_variable`] is `XDG_DATA_HOME` — a directory *above* the `opencode` subfolder
/// OpenCode itself appends, not the config dir itself (research D9; `provider.rs`'s own
/// `config_dir()` joins `"opencode"` onto `XDG_DATA_HOME` the same way).
pub fn host_config_dir(data_dir: &Path, cli: CliId) -> PathBuf {
    match cli {
        CliId::Opencode => data_dir.join("ai").join("opencode").join("data"),
        _ => data_dir.join("ai").join(cli.as_str()),
    }
}

/// The delivered sign-in's file name under [`host_config_dir`] (research D9's per-CLI table):
/// Claude's own `.credentials.json`, Codex's and OpenCode's shared `auth.json` — exactly the
/// basename each provider's own `sandbox_auth_file`/`config_dir` already uses, so the session sees
/// its sign-in exactly where it always looks. `None` for a CLI with no shareable sign-in
/// (Copilot, Pi).
pub fn host_sign_in_file_name(cli: CliId) -> Option<&'static str> {
    match cli {
        CliId::ClaudeCode => Some(".credentials.json"),
        CliId::Codex | CliId::Opencode => Some("auth.json"),
        CliId::Copilot | CliId::Pi => None,
    }
}

/// The files and directories linked read-through from the CLI's own live config dir into its
/// isolated host/SSH one (research D9: "The daemon links the user's own settings files... into
/// it (read-through links, never copies back)"), so only the sign-in differs. OpenCode has none
/// named today (Issue 11 found no per-CLI settings file worth isolating from, beyond the sign-in
/// itself); Copilot and Pi have no isolated config dir at all ([`host_config_dir_variable`]).
pub fn host_linked_extras(cli: CliId) -> &'static [&'static str] {
    match cli {
        CliId::ClaudeCode => &["settings.json", "CLAUDE.md", "agents", "commands"],
        CliId::Codex => &["config.toml", "AGENTS.md"],
        CliId::Opencode | CliId::Copilot | CliId::Pi => &[],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::daemon_config::Place;
    use std::collections::BTreeMap;

    fn cli(id: CliId) -> AiCli {
        AiCli {
            id,
            enabled: None,
            version: None,
            args: Vec::new(),
            model: None,
            api_base_url: None,
            env: BTreeMap::new(),
            sign_in_file: None,
            token_file: None,
            api_key_file: None,
            settings_file: None,
            place: Vec::new(),
            use_machine_sign_in: None,
        }
    }

    #[test]
    fn model_only_overrides_claude_codes_default() {
        let mut claude = cli(CliId::ClaudeCode);
        claude.model = Some("opus".to_string());
        assert_eq!(
            launch_overlay(&claude, None).model,
            Some("opus".to_string())
        );

        let mut codex = cli(CliId::Codex);
        codex.model = None; // refused by V7 for any other CLI; nothing to carry here.
        assert_eq!(launch_overlay(&codex, None).model, None);
    }

    #[test]
    fn api_base_url_maps_to_the_clis_own_endpoint_variable() {
        let mut claude = cli(CliId::ClaudeCode);
        claude.api_base_url = Some("https://proxy.example/v1".to_string());
        assert_eq!(
            launch_overlay(&claude, None).env,
            vec![(
                "ANTHROPIC_BASE_URL".to_string(),
                "https://proxy.example/v1".to_string()
            )]
        );

        let mut codex = cli(CliId::Codex);
        codex.api_base_url = Some("https://proxy.example/v1".to_string());
        assert_eq!(
            launch_overlay(&codex, None).env,
            vec![(
                "OPENAI_BASE_URL".to_string(),
                "https://proxy.example/v1".to_string()
            )]
        );
    }

    #[test]
    fn claude_codes_api_key_never_reaches_env_it_uses_api_key_helper_instead() {
        assert!(uses_api_key_helper(CliId::ClaudeCode));
        let mut claude = cli(CliId::ClaudeCode);
        claude.api_key_file = Some(super::super::SecretPath::new("/secrets/claude.key"));
        // Even handed the key's content, Claude Code's overlay carries no env for it: the daemon
        // never puts the key on the command line or in the environment for this CLI (SC-008).
        assert_eq!(launch_overlay(&claude, Some("sk-test")).env, Vec::new());
    }

    #[test]
    fn codexs_api_key_reaches_openai_api_key_at_session_start() {
        assert!(!uses_api_key_helper(CliId::Codex));
        let codex = cli(CliId::Codex);
        assert_eq!(
            launch_overlay(&codex, Some("sk-test")).env,
            vec![("OPENAI_API_KEY".to_string(), "sk-test".to_string())]
        );
    }

    #[test]
    fn configured_args_and_env_are_additions_carried_verbatim() {
        let mut claude = cli(CliId::ClaudeCode);
        claude.args = vec!["--verbose".to_string()];
        claude.env.insert("FOO".to_string(), "bar".to_string());
        let overlay = launch_overlay(&claude, None);
        assert_eq!(overlay.args, vec!["--verbose".to_string()]);
        assert_eq!(overlay.env, vec![("FOO".to_string(), "bar".to_string())]);
    }

    #[test]
    fn place_as_stays_inside_the_config_dir() {
        let dir = Path::new("/home/user/.claude");
        assert_eq!(
            place_target(dir, Path::new("agents/review.md")),
            Some(dir.join("agents/review.md"))
        );
        assert_eq!(place_target(dir, Path::new("../escape")), None);
        assert_eq!(place_target(dir, Path::new("/absolute")), None);
    }

    #[test]
    fn place_as_rejects_an_absolute_path_foreign_to_the_host_platform() {
        // A leading `/` or `\`, and a drive letter, are all absolute on some platform the
        // daemon runs on even when `Path::is_absolute` (host-specific) says otherwise — e.g.
        // `/absolute` is relative by Windows's own rule, `C:\escape` is relative by POSIX's.
        let dir = Path::new("/home/user/.claude");
        assert_eq!(place_target(dir, Path::new("/absolute")), None);
        assert_eq!(place_target(dir, Path::new(r"\absolute")), None);
        assert_eq!(place_target(dir, Path::new(r"C:\escape")), None);
        assert_eq!(place_target(dir, Path::new("C:/escape")), None);
    }

    #[test]
    fn place_entries_round_trip_through_the_parsed_type() {
        let place = Place {
            file: PathBuf::from("/src/mcp.json"),
            as_path: PathBuf::from("mcp.json"),
        };
        let dir = Path::new("/home/user/.codex");
        assert_eq!(
            place_target(dir, &place.as_path),
            Some(dir.join("mcp.json"))
        );
    }

    #[test]
    fn settings_file_names_match_each_clis_own_user_settings() {
        assert_eq!(settings_file_name(CliId::ClaudeCode), Some("settings.json"));
        assert_eq!(settings_file_name(CliId::Codex), Some("config.toml"));
        assert_eq!(settings_file_name(CliId::Opencode), Some("opencode.json"));
        assert_eq!(settings_file_name(CliId::Copilot), None);
        assert_eq!(settings_file_name(CliId::Pi), None);
    }

    #[test]
    fn host_config_dir_is_data_dir_ai_id_except_opencodes_data_level() {
        let data_dir = Path::new("/var/lib/micold");
        assert_eq!(
            host_config_dir(data_dir, CliId::ClaudeCode),
            data_dir.join("ai").join("claude_code")
        );
        assert_eq!(
            host_config_dir(data_dir, CliId::Codex),
            data_dir.join("ai").join("codex")
        );
        assert_eq!(
            host_config_dir(data_dir, CliId::Opencode),
            data_dir.join("ai").join("opencode").join("data")
        );
    }

    #[test]
    fn host_config_dir_variable_and_sign_in_file_name_cover_only_the_clis_with_a_sign_in() {
        for cli in [CliId::ClaudeCode, CliId::Codex, CliId::Opencode] {
            assert!(host_config_dir_variable(cli).is_some());
            assert!(host_sign_in_file_name(cli).is_some());
        }
        for cli in [CliId::Copilot, CliId::Pi] {
            assert_eq!(host_config_dir_variable(cli), None);
            assert_eq!(host_sign_in_file_name(cli), None);
            assert!(host_linked_extras(cli).is_empty());
        }
    }
}
