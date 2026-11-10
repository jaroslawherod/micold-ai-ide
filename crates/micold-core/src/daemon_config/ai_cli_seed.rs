//! What a host or SSH daemon seeds into a CLI's isolated config dir beside the delivered sign-in
//! (#1532 M3, research D15 rows 4, 6, 14): the user's `.claude.json` (trust and MCP servers kept,
//! `primaryApiKey` dropped, never copied back), `settings_file` and `place` as **copies** — never
//! links or read-only mounts, so a CLI can record its own state in them (Codex's folder trust in
//! `config.toml`) — and the `expiresAt` of a delivered sign-in. `use_machine_sign_in` opts a CLI
//! out of all of it.

use std::path::{Path, PathBuf};

use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};

use super::ai_cli::{host_config_dir, place_target, settings_file_name};
use super::{AiCli, CliId};

/// The Claude Code file that holds per-project trust, MCP servers and the onboarding flag.
const CLAUDE_JSON: &str = ".claude.json";
/// The key `.claude.json` keeps an API key in; a delivered sign-in replaces it, so it is never
/// seeded.
const PRIMARY_API_KEY: &str = "primaryApiKey";
/// Where the stamps that say which source version a copy was made from live.
const STAMP_DIR: &str = ".micold-seeded";

/// Whether `cli` opts out of every delivered, isolated sign-in and config (`use_machine_sign_in`,
/// host and SSH only; off when unset, so a daemon whose file never named it migrates with it off).
pub fn uses_machine_sign_in(cli: &AiCli) -> bool {
    cli.use_machine_sign_in == Some(true)
}

/// Whether container and pod sessions must mark Claude Code's onboarding done (T097): its entry
/// has a `sign_in_file`, `token_file` or `api_key_file`, so a credential is delivered whatever the
/// sandbox's credential shares say.
pub fn claude_onboarding_wanted(clis: &[AiCli]) -> bool {
    clis.iter().any(|c| {
        c.id == CliId::ClaudeCode
            && (c.sign_in_file.is_some() || c.token_file.is_some() || c.api_key_file.is_some())
    })
}

/// The directory the CLI reads its own config from inside [`host_config_dir`]: OpenCode appends an
/// `opencode` folder to the variable's value, the others read the value itself.
fn config_dir(data_dir: &Path, cli: CliId) -> PathBuf {
    let dir = host_config_dir(data_dir, cli);
    if cli == CliId::Opencode {
        dir.join("opencode")
    } else {
        dir
    }
}

/// Seeds `<config dir>/.claude.json` from the user's own file, once. Nothing happens when the
/// target exists (unless `reseed`, Settings' "Seed again from this computer") or the user has no
/// parseable object there. `primaryApiKey` is dropped; nothing is ever written to `user_file`.
fn seed_claude_json(dir: &Path, user_file: &Path, reseed: bool) -> std::io::Result<()> {
    let target = dir.join(CLAUDE_JSON);
    if !reseed && target.symlink_metadata().is_ok() {
        return Ok(());
    }
    let Ok(text) = std::fs::read_to_string(user_file) else {
        return Ok(());
    };
    let Ok(serde_json::Value::Object(mut record)) = serde_json::from_str(&text) else {
        return Ok(());
    };
    record.remove(PRIMARY_API_KEY);
    let out = serde_json::to_string_pretty(&serde_json::Value::Object(record))
        .map_err(std::io::Error::other)?;
    remove_link(&target)?;
    std::fs::write(&target, out)
}

/// Removes `path` when it is a link, so a following write replaces the link, never the file it
/// names (a read-through link made for the user's own settings must not be written through).
fn remove_link(path: &Path) -> std::io::Result<()> {
    match path.symlink_metadata() {
        Ok(m) if m.file_type().is_symlink() => std::fs::remove_file(path),
        _ => Ok(()),
    }
}

/// Replaces every link between `config_dir` and `dir` with a real directory, so a copy made below
/// it never lands in the user's own files a read-through link (`agents`, `commands`) names.
fn unlink_ancestors(config_dir: &Path, dir: &Path) -> std::io::Result<()> {
    let Ok(relative) = dir.strip_prefix(config_dir) else {
        return Ok(());
    };
    let mut current = config_dir.to_path_buf();
    for part in relative.components() {
        current.push(part);
        if current
            .symlink_metadata()
            .is_ok_and(|m| m.file_type().is_symlink())
        {
            std::fs::remove_file(&current)?;
            std::fs::create_dir(&current)?;
        }
    }
    Ok(())
}

/// What identifies one version of `source`: its length and modification time.
fn stamp_of(source: &Path) -> std::io::Result<String> {
    let meta = std::fs::metadata(source)?;
    let nanos = meta
        .modified()
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map_or(0, |d| d.as_nanos());
    Ok(format!("{}:{nanos}", meta.len()))
}

/// Copies `source` to `target` when it is new or `source` changed since the last copy; otherwise
/// leaves `target` alone, so what the CLI wrote there survives.
fn seed_copy(config_dir: &Path, source: &Path, target: &Path) -> std::io::Result<()> {
    let stamp = stamp_of(source)?;
    let name = target
        .strip_prefix(config_dir)
        .unwrap_or(target)
        .to_string_lossy()
        .replace(['/', '\\'], "_");
    let stamp_file = config_dir.join(STAMP_DIR).join(name);
    let is_link = target
        .symlink_metadata()
        .is_ok_and(|m| m.file_type().is_symlink());
    if !is_link
        && target.exists()
        && std::fs::read_to_string(&stamp_file).is_ok_and(|seen| seen == stamp)
    {
        return Ok(());
    }
    if let Some(parent) = target.parent() {
        unlink_ancestors(config_dir, parent)?;
        std::fs::create_dir_all(parent)?;
    }
    remove_link(target)?;
    std::fs::copy(source, target)?;
    if let Some(parent) = stamp_file.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&stamp_file, stamp)
}

/// Seeds `cli`'s isolated config dir under `data_dir` (the one `prepare_host_config_dir` built):
/// Claude's `.claude.json` from `user_claude_json`, then `settings_file` and each `place` as
/// copies. A no-op with `use_machine_sign_in`. Individual copies that fail are reported together
/// after the others were tried, so one bad source does not drop the rest (FR-019).
pub fn seed_host_config_dir(
    data_dir: &Path,
    cli: &AiCli,
    user_claude_json: Option<&Path>,
    reseed: bool,
) -> std::io::Result<()> {
    if uses_machine_sign_in(cli) {
        return Ok(());
    }
    let dir = config_dir(data_dir, cli.id);
    std::fs::create_dir_all(&dir)?;
    let mut failures = Vec::new();
    let mut note = |what: &str, result: std::io::Result<()>| {
        if let Err(e) = result {
            failures.push(format!("{what}: {e}"));
        }
    };
    if let (CliId::ClaudeCode, Some(user)) = (cli.id, user_claude_json) {
        note(CLAUDE_JSON, seed_claude_json(&dir, user, reseed));
    }
    if let (Some(source), Some(name)) = (&cli.settings_file, settings_file_name(cli.id)) {
        note(name, seed_copy(&dir, source, &dir.join(name)));
    }
    for place in &cli.place {
        match place_target(&dir, &place.as_path) {
            Some(target) => note(
                &place.as_path.display().to_string(),
                seed_copy(&dir, &place.file, &target),
            ),
            None => note(
                &place.as_path.display().to_string(),
                Err(std::io::Error::other("escapes the config dir")),
            ),
        }
    }
    if failures.is_empty() {
        Ok(())
    } else {
        Err(std::io::Error::other(failures.join("; ")))
    }
}

/// When a delivered sign-in's access token runs out, in unix seconds, read from the sign-in
/// file's `text` (FR-040): Claude's `claudeAiOauth.expiresAt` (milliseconds) and the `exp` claim of
/// Codex's `tokens.access_token`, decoded for its hint only and never verified. `None` for a CLI or
/// a file with no such hint.
pub fn sign_in_expires_at_secs(cli: CliId, text: &str) -> Option<u64> {
    let value: serde_json::Value = serde_json::from_str(text).ok()?;
    match cli {
        CliId::ClaudeCode => Some(value.get("claudeAiOauth")?.get("expiresAt")?.as_u64()? / 1000),
        CliId::Codex => {
            let token = value.get("tokens")?.get("access_token")?.as_str()?;
            let bytes = URL_SAFE_NO_PAD.decode(token.split('.').nth(1)?).ok()?;
            serde_json::from_slice::<serde_json::Value>(&bytes)
                .ok()?
                .get("exp")?
                .as_u64()
        }
        CliId::Opencode | CliId::Copilot | CliId::Pi => None,
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

    /// T096: seeded once, `primaryApiKey` dropped, the user's file never written.
    #[test]
    fn claude_json_is_seeded_once_without_the_api_key_and_never_copied_back() {
        let tmp = tempfile::tempdir().unwrap();
        let user = tmp.path().join("user.json");
        let text = r#"{"primaryApiKey":"sk-x","projects":{"/p":{"hasTrustDialogAccepted":true}},"mcpServers":{"m":{}}}"#;
        std::fs::write(&user, text).unwrap();
        let data = tmp.path().join("data");
        let claude = cli(CliId::ClaudeCode);

        seed_host_config_dir(&data, &claude, Some(&user), false).unwrap();
        let seeded = data.join("ai/claude_code/.claude.json");
        let v: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&seeded).unwrap()).unwrap();
        assert!(v.get("primaryApiKey").is_none());
        assert_eq!(v["projects"]["/p"]["hasTrustDialogAccepted"], true);
        assert!(v["mcpServers"].is_object());

        // The CLI edits its copy; a second session must not overwrite it, nor touch the user's.
        std::fs::write(&seeded, r#"{"edited":1}"#).unwrap();
        seed_host_config_dir(&data, &claude, Some(&user), false).unwrap();
        assert_eq!(std::fs::read_to_string(&seeded).unwrap(), r#"{"edited":1}"#);
        assert_eq!(std::fs::read_to_string(&user).unwrap(), text);

        // "Seed again from this computer".
        seed_host_config_dir(&data, &claude, Some(&user), true).unwrap();
        assert!(std::fs::read_to_string(&seeded)
            .unwrap()
            .contains("mcpServers"));
    }

    #[test]
    fn a_missing_or_unparseable_user_file_seeds_nothing() {
        let tmp = tempfile::tempdir().unwrap();
        let claude = cli(CliId::ClaudeCode);
        seed_host_config_dir(tmp.path(), &claude, Some(&tmp.path().join("none")), false).unwrap();
        let bad = tmp.path().join("bad.json");
        std::fs::write(&bad, "not json").unwrap();
        seed_host_config_dir(tmp.path(), &claude, Some(&bad), false).unwrap();
        assert!(!tmp.path().join("ai/claude_code/.claude.json").exists());
    }

    /// T098: copies, re-copied only when the source changed, so the CLI's own edits survive.
    #[test]
    fn settings_file_and_place_are_copies_recopied_only_when_the_source_changes() {
        let tmp = tempfile::tempdir().unwrap();
        let source = tmp.path().join("config.toml");
        let agent = tmp.path().join("review.md");
        std::fs::write(&source, "model = \"a\"\n").unwrap();
        std::fs::write(&agent, "agent").unwrap();
        let mut codex = cli(CliId::Codex);
        codex.settings_file = Some(source.clone());
        codex.place = vec![Place {
            file: agent,
            as_path: PathBuf::from("agents/review.md"),
        }];
        let data = tmp.path().join("data");
        let dir = data.join("ai/codex");

        seed_host_config_dir(&data, &codex, None, false).unwrap();
        let copy = dir.join("config.toml");
        assert_eq!(std::fs::read_to_string(&copy).unwrap(), "model = \"a\"\n");
        assert_eq!(
            std::fs::read_to_string(dir.join("agents/review.md")).unwrap(),
            "agent"
        );
        assert!(
            !copy.symlink_metadata().unwrap().file_type().is_symlink(),
            "a copy, never a link or a mount"
        );

        // Codex records folder trust in its copy; an unchanged source leaves it.
        std::fs::write(
            &copy,
            "model = \"a\"\n[projects.\"/p\"]\ntrust_level = \"trusted\"\n",
        )
        .unwrap();
        seed_host_config_dir(&data, &codex, None, false).unwrap();
        assert!(std::fs::read_to_string(&copy).unwrap().contains("trusted"));

        // A changed source replaces it.
        std::fs::write(&source, "model = \"bb\"\n").unwrap();
        seed_host_config_dir(&data, &codex, None, false).unwrap();
        assert_eq!(std::fs::read_to_string(&copy).unwrap(), "model = \"bb\"\n");
    }

    /// Review A F1: a `place` below a linked directory never writes into the user's directory.
    #[cfg(unix)]
    #[test]
    fn a_place_below_a_linked_directory_leaves_the_users_directory_alone() {
        let tmp = tempfile::tempdir().unwrap();
        let users = tmp.path().join("users-agents");
        std::fs::create_dir_all(&users).unwrap();
        let file = tmp.path().join("f");
        std::fs::write(&file, "x").unwrap();
        let data = tmp.path().join("data");
        let dir = data.join("ai/claude_code");
        std::fs::create_dir_all(&dir).unwrap();
        std::os::unix::fs::symlink(&users, dir.join("agents")).unwrap();
        let mut claude = cli(CliId::ClaudeCode);
        claude.place = vec![Place {
            file,
            as_path: PathBuf::from("agents/review.md"),
        }];
        seed_host_config_dir(&data, &claude, None, false).unwrap();
        assert!(dir.join("agents/review.md").exists());
        assert_eq!(std::fs::read_dir(&users).unwrap().count(), 0);
    }

    /// Review A F2: prepare (which relinks the user's settings) then seed, twice: the CLI's own
    /// edit of the copy survives the second session.
    #[cfg(unix)]
    #[test]
    fn the_clis_edit_of_a_settings_copy_survives_the_next_prepare() {
        use crate::daemon_config::ai_cli::prepare_host_config_dir;
        let tmp = tempfile::tempdir().unwrap();
        let live = tmp.path().join("live");
        std::fs::create_dir_all(&live).unwrap();
        std::fs::write(live.join("config.toml"), "users").unwrap();
        let sign_in = tmp.path().join("auth.json");
        std::fs::write(&sign_in, "{}").unwrap();
        let source = tmp.path().join("cfg.toml");
        std::fs::write(&source, "configured").unwrap();
        let mut codex = cli(CliId::Codex);
        codex.settings_file = Some(source);
        let data = tmp.path().join("data");
        let copy = data.join("ai/codex/config.toml");
        for session in 0..2 {
            prepare_host_config_dir(&data, CliId::Codex, &sign_in, Some(&live)).unwrap();
            seed_host_config_dir(&data, &codex, None, false).unwrap();
            if session == 0 {
                std::fs::write(&copy, "configured\ntrusted").unwrap();
            }
        }
        assert_eq!(
            std::fs::read_to_string(&copy).unwrap(),
            "configured\ntrusted"
        );
        assert_eq!(
            std::fs::read_to_string(live.join("config.toml")).unwrap(),
            "users"
        );
    }

    /// A settings copy replaces a read-through link; it never writes through it.
    #[cfg(unix)]
    #[test]
    fn a_settings_copy_replaces_a_link_without_writing_through_it() {
        let tmp = tempfile::tempdir().unwrap();
        let users = tmp.path().join("users.json");
        std::fs::write(&users, "user").unwrap();
        let source = tmp.path().join("cfg.json");
        std::fs::write(&source, "configured").unwrap();
        let data = tmp.path().join("data");
        let dir = data.join("ai/claude_code");
        std::fs::create_dir_all(&dir).unwrap();
        std::os::unix::fs::symlink(&users, dir.join("settings.json")).unwrap();
        let mut claude = cli(CliId::ClaudeCode);
        claude.settings_file = Some(source);
        seed_host_config_dir(&data, &claude, None, false).unwrap();
        assert_eq!(
            std::fs::read_to_string(dir.join("settings.json")).unwrap(),
            "configured"
        );
        assert_eq!(std::fs::read_to_string(&users).unwrap(), "user");
    }

    #[test]
    fn a_place_that_escapes_is_reported_and_the_rest_still_seeded() {
        let tmp = tempfile::tempdir().unwrap();
        let file = tmp.path().join("f");
        std::fs::write(&file, "x").unwrap();
        let mut claude = cli(CliId::ClaudeCode);
        claude.place = vec![
            Place {
                file: file.clone(),
                as_path: PathBuf::from("../escape"),
            },
            Place {
                file,
                as_path: PathBuf::from("ok.md"),
            },
        ];
        let data = tmp.path().join("data");
        assert!(seed_host_config_dir(&data, &claude, None, false).is_err());
        assert!(data.join("ai/claude_code/ok.md").exists());
        assert!(!data.join("ai/escape").exists());
    }

    #[test]
    fn onboarding_is_wanted_for_any_configured_claude_credential() {
        let mut claude = cli(CliId::ClaudeCode);
        assert!(!claude_onboarding_wanted(&[claude.clone()]));
        claude.token_file = Some(super::super::SecretPath::new("/t"));
        assert!(claude_onboarding_wanted(&[claude.clone()]));
        let mut codex = cli(CliId::Codex);
        codex.api_key_file = claude.token_file.clone();
        assert!(!claude_onboarding_wanted(&[codex]));
    }

    /// T099: `use_machine_sign_in` seeds nothing and is off unless set.
    #[test]
    fn use_machine_sign_in_is_off_by_default_and_seeds_nothing_when_on() {
        let tmp = tempfile::tempdir().unwrap();
        let user = tmp.path().join("u.json");
        std::fs::write(&user, "{}").unwrap();
        let mut claude = cli(CliId::ClaudeCode);
        assert!(!uses_machine_sign_in(&claude));
        claude.use_machine_sign_in = Some(true);
        assert!(uses_machine_sign_in(&claude));
        seed_host_config_dir(tmp.path(), &claude, Some(&user), true).unwrap();
        assert!(!tmp.path().join("ai").exists());
    }

    /// T100: `expiresAt` of each delivered sign-in.
    #[test]
    fn a_sign_ins_expiry_is_read_per_cli() {
        let claude = r#"{"claudeAiOauth":{"expiresAt":1700000000000}}"#;
        assert_eq!(
            sign_in_expires_at_secs(CliId::ClaudeCode, claude),
            Some(1_700_000_000)
        );
        let payload = URL_SAFE_NO_PAD.encode(r#"{"exp":1800000000}"#);
        let codex = format!(r#"{{"tokens":{{"access_token":"h.{payload}.s"}}}}"#);
        assert_eq!(
            sign_in_expires_at_secs(CliId::Codex, &codex),
            Some(1_800_000_000)
        );
        assert_eq!(sign_in_expires_at_secs(CliId::Opencode, claude), None);
        assert_eq!(sign_in_expires_at_secs(CliId::ClaudeCode, "{}"), None);
    }
}
