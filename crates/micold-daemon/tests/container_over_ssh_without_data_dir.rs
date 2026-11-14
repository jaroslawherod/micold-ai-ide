//! #1537, T086a (research D15 row 2): a container daemon started over SSH is passed no
//! `MICOLD_DATA_DIR`, and the image sets none. Its sessions still get the credential helper, the
//! HTTPS rewrite and `GIT_CONFIG_GLOBAL`, because the git setup follows `daemon.kind`, not the data
//! directory's value.

use std::collections::BTreeMap;
use std::path::Path;

use micold_core::daemon_config::Kind;
use micold_daemon::settings::{install, DaemonSettings};
use micold_daemon::supervisor::apply_env_hygiene;

fn put(dir: &Path, name: &str, text: &str) -> std::path::PathBuf {
    let file = dir.join(name);
    std::fs::write(&file, text).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&file, std::fs::Permissions::from_mode(0o600)).unwrap();
    }
    file
}

#[test]
fn a_container_session_started_over_ssh_keeps_its_git_setup_without_a_data_dir_variable() {
    // The test is about a daemon that was passed no `MICOLD_DATA_DIR`, whatever the shell exports.
    std::env::remove_var("MICOLD_DATA_DIR");
    let dir = tempfile::tempdir().unwrap();
    let api_token = put(dir.path(), "github-token", "ghp_x\n");
    let client_token = put(dir.path(), "token", "t\n");
    let identity = put(dir.path(), "gitconfig", "[user]\n\tname = x\n");
    let config = put(
        dir.path(),
        "daemon.toml",
        &format!(
            "schema = 1\n[daemon]\nname = \"c\"\nkind = \"container\"\n\
             [listen]\ntcp = \"127.0.0.1:7000\"\n[auth]\nclient_token_file = '{}'\n\
             [[github.tokens]]\nname = \"gh\"\ntoken_file = '{}'\n",
            client_token.display(),
            api_token.display()
        ),
    );
    // What `DaemonSettings::resolve` yields for a file read over SSH with no variable in sight.
    install(DaemonSettings {
        kind: Some(Kind::Container),
        config_path: Some(config),
        git_config_file: Some(identity.clone()),
        ..DaemonSettings::default()
    });

    let mut cmd = std::process::Command::new("true");
    apply_env_hygiene(&mut cmd);
    let env: BTreeMap<String, String> = cmd
        .get_envs()
        .filter_map(|(k, v)| Some((k.to_str()?.to_owned(), v?.to_str()?.to_owned())))
        .collect();

    assert_eq!(
        env.get("GIT_CONFIG_GLOBAL").map(String::as_str),
        identity.to_str(),
        "{env:?}"
    );
    let block: Vec<&String> = env
        .iter()
        .filter(|(k, _)| k.starts_with("GIT_CONFIG_KEY_") || k.starts_with("GIT_CONFIG_VALUE_"))
        .map(|(_, v)| v)
        .collect();
    assert!(
        block.iter().any(|v| v.contains("git-credential")),
        "the credential helper: {block:?}"
    );
    assert!(
        block.iter().any(|v| v.contains(".insteadOf")),
        "the HTTPS rewrite: {block:?}"
    );
}
