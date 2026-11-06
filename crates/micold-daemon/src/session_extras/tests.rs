use std::collections::BTreeMap;
use std::path::Path;
use std::process::Command;

use micold_core::daemon_config::DaemonConfig;
use micold_core::process::no_window;

use super::*;

/// A configuration of `kind` with `tail` after the `[daemon]` table; `dir` holds the files.
fn config(dir: &Path, kind: &str, tail: &str) -> DaemonConfig {
    let mut text = format!("schema = 1\n[daemon]\nname = \"t\"\nkind = \"{kind}\"\n");
    if kind == "container" {
        let client = dir.join("client");
        write(&client, "x", 0o600);
        text.push_str(&format!(
            "image = \"img\"\n[listen]\ntcp = \"127.0.0.1:7000\"\n[auth]\nclient_token_file = \"{}\"\n",
            client.display()
        ));
    }
    text.push_str(tail);
    crate::config_load::validate_text(&text, "daemon.toml")
        .unwrap_or_else(|e| panic!("{e:?}"))
        .0
}

fn write(path: &Path, text: &str, mode: u32) {
    std::fs::write(path, text).unwrap();
    #[cfg(unix)]
    std::fs::set_permissions(path, std::os::unix::fs::PermissionsExt::from_mode(mode)).unwrap();
}

fn vars(edits: &[EnvEdit]) -> BTreeMap<String, String> {
    edits
        .iter()
        .filter_map(|e| match e {
            EnvEdit::Set(k, v) => Some((k.to_string_lossy().into(), v.to_string_lossy().into())),
            EnvEdit::Remove(_) => None,
        })
        .collect()
}

fn git_section(dir: &Path) -> String {
    let (key, hosts) = (dir.join("id"), dir.join("known"));
    write(&key, "k", 0o600);
    write(&hosts, "h", 0o644);
    format!(
        "[git]\nssh_key_file = \"{}\"\nknown_hosts_file = \"{}\"\n",
        key.display(),
        hosts.display()
    )
}

#[test]
fn a_sandbox_session_ssh_command_names_the_key_and_the_known_hosts() {
    let dir = tempfile::tempdir().unwrap();
    let cfg = config(dir.path(), "container", &git_section(dir.path()));
    let env = extras(&cfg).env.into_iter().collect::<BTreeMap<_, _>>();
    let want = format!(
        "ssh -i {} -o UserKnownHostsFile={} -o IdentitiesOnly=yes",
        dir.path().join("id").display(),
        dir.path().join("known").display()
    );
    assert_eq!(env["GIT_SSH_COMMAND"], want);
}

#[test]
fn a_host_daemon_leaves_the_users_git_and_keys_alone() {
    let dir = tempfile::tempdir().unwrap();
    let cfg = config(dir.path(), "host", "");
    assert_eq!(extras(&cfg), Extras::default());
}

#[test]
fn signing_follows_the_key_and_is_off_without_it() {
    let dir = tempfile::tempdir().unwrap();
    let off = extras(&config(dir.path(), "container", ""));
    assert!(off.git.contains(&("commit.gpgsign".into(), "false".into())));
    assert!(off.git.contains(&("tag.gpgsign".into(), "false".into())));
    let key = dir.path().join("sign");
    write(&key, "k", 0o600);
    let on = extras(&config(
        dir.path(),
        "container",
        &format!("[git]\nsigning_key_file = \"{}\"\n", key.display()),
    ));
    assert_eq!(
        on.git,
        vec![("user.signingkey".to_string(), key.to_string_lossy().into())]
    );
}

#[test]
fn git_entries_join_the_block_already_there() {
    let base = vec![
        EnvEdit::Set("GIT_CONFIG_COUNT".into(), "2".into()),
        EnvEdit::Set("GIT_CONFIG_KEY_0".into(), "a".into()),
        EnvEdit::Set("GIT_CONFIG_VALUE_0".into(), "1".into()),
    ];
    let extra = Extras {
        env: vec![("X".into(), "y".into())],
        git: vec![("k".into(), "v".into())],
    };
    let env = vars(&add_extras(base, extra.clone()));
    assert_eq!(env["GIT_CONFIG_COUNT"], "3");
    assert_eq!(
        (&env["GIT_CONFIG_KEY_2"], &env["GIT_CONFIG_VALUE_2"]),
        (&"k".to_string(), &"v".to_string())
    );
    assert_eq!(env["X"], "y");
    // No block yet: one starts.
    assert_eq!(
        vars(&add_extras(Vec::new(), extra))["GIT_CONFIG_COUNT"],
        "1"
    );
}

/// Run `git commit` in a fresh repository whose global configuration asks for SSH signing,
/// with the environment `edits` give; true when the commit succeeds. `None` without git or
/// ssh-keygen.
fn commit_with(dir: &Path, edits: &[EnvEdit]) -> Option<bool> {
    let repo = dir.join("repo");
    std::fs::create_dir_all(&repo).ok()?;
    let global = dir.join("gitconfig");
    write(
        &global,
        "[user]\nname = t\nemail = t@example.com\n[gpg]\nformat = ssh\n[commit]\ngpgsign = true\n",
        0o600,
    );
    let git = |args: &[&str]| {
        let mut cmd = Command::new("git");
        no_window(&mut cmd)
            .current_dir(&repo)
            .args(args)
            .env("GIT_CONFIG_GLOBAL", &global)
            .env("GIT_CONFIG_SYSTEM", "/dev/null");
        for (k, v) in vars(edits) {
            cmd.env(k, v);
        }
        cmd.output()
    };
    git(&["init", "-q"]).ok()?.status.success().then_some(())?;
    Some(
        git(&["commit", "-q", "--allow-empty", "-m", "m"])
            .ok()?
            .status
            .success(),
    )
}

fn ssh_keygen(path: &Path) -> Option<()> {
    no_window(&mut Command::new("ssh-keygen"))
        .args(["-q", "-t", "ed25519", "-N", "", "-f"])
        .arg(path)
        .status()
        .ok()?
        .success()
        .then_some(())
}

#[test]
fn a_signed_commit_succeeds_with_the_key_and_an_unsigned_one_without() {
    let dir = tempfile::tempdir().unwrap();
    let key = dir.path().join("sign");
    if ssh_keygen(&key).is_none() {
        return;
    }
    let with = config(
        dir.path(),
        "container",
        &format!("[git]\nsigning_key_file = \"{}\"\n", key.display()),
    );
    let Some(signed) = commit_with(dir.path(), &add_extras(Vec::new(), extras(&with))) else {
        return;
    };
    assert!(signed, "a commit signed with the delivered key");
    let log = no_window(&mut Command::new("git"))
        .current_dir(dir.path().join("repo"))
        .args(["cat-file", "-p", "HEAD"])
        .output()
        .unwrap();
    assert!(String::from_utf8_lossy(&log.stdout).contains("gpgsig"));
    // The same global configuration asks for signing and no key arrived: it would fail unless off.
    let without = config(dir.path(), "container", "");
    std::fs::remove_dir_all(dir.path().join("repo")).unwrap();
    assert_eq!(
        commit_with(dir.path(), &add_extras(Vec::new(), extras(&without))),
        Some(true)
    );
    std::fs::remove_dir_all(dir.path().join("repo")).unwrap();
    assert_eq!(
        commit_with(dir.path(), &[]),
        Some(false),
        "signing is on without it"
    );
}

#[test]
fn the_ca_file_reaches_every_tool_that_reads_one() {
    let dir = tempfile::tempdir().unwrap();
    let ca = dir.path().join("ca.pem");
    write(&ca, "pem", 0o644);
    let cfg = config(
        dir.path(),
        "host",
        &format!("[network]\nca_file = \"{}\"\n", ca.display()),
    );
    let env = extras(&cfg).env.into_iter().collect::<BTreeMap<_, _>>();
    for name in CA_VARIABLES {
        assert_eq!(env[name], ca.to_string_lossy(), "{name}");
    }
    assert_eq!(Egress::of(&cfg).ca_file.as_deref(), Some(ca.as_path()));
}

#[test]
fn the_daemon_takes_its_proxy_from_session_env_or_a_secret_file() {
    let dir = tempfile::tempdir().unwrap();
    let secret = dir.path().join("proxy");
    write(&secret, "http://user:pw@proxy.corp:3128\n", 0o600);
    let cfg = config(
        dir.path(),
        "host",
        &format!(
            "[session.env]\nNO_PROXY = \"localhost,.corp\"\n[session.secret_env]\nHTTPS_PROXY = \"{}\"\n",
            secret.display()
        ),
    );
    let egress = Egress::of(&cfg);
    assert_eq!(
        egress.https_proxy.as_deref(),
        Some("http://user:pw@proxy.corp:3128")
    );
    assert!(egress.bypasses_proxy("api.corp"));
    assert!(egress.bypasses_proxy("LOCALHOST"));
    assert!(!egress.bypasses_proxy("api.github.com"));
    let none = Egress::of(&config(dir.path(), "host", ""));
    assert_eq!(none, Egress::default());
}

fn file(from: &Path, to: &str, mode: Option<&str>) -> SessionFile {
    SessionFile {
        file: from.to_path_buf(),
        as_path: to.into(),
        mode: mode.map(str::to_string),
    }
}

#[test]
fn a_session_file_lands_in_the_home_with_its_mode() {
    let dir = tempfile::tempdir().unwrap();
    let (src, home) = (dir.path().join("npmrc"), dir.path().join("home"));
    write(&src, "//registry/:_authToken=x", 0o400);
    let files = [
        file(&src, ".npmrc", None),
        file(&src, "a/b/.pypirc", Some("0640")),
    ];
    for _ in 0..2 {
        assert!(deliver_files(&files, &home).is_empty());
    }
    let mode = |p: &str| {
        use std::os::unix::fs::PermissionsExt;
        std::fs::metadata(home.join(p))
            .unwrap()
            .permissions()
            .mode()
            & 0o777
    };
    assert_eq!(
        std::fs::read_to_string(home.join(".npmrc")).unwrap(),
        "//registry/:_authToken=x"
    );
    assert_eq!((mode(".npmrc"), mode("a/b/.pypirc")), (0o600, 0o640));
}

#[test]
fn a_target_outside_the_home_or_in_a_cli_config_dir_is_refused() {
    let dir = tempfile::tempdir().unwrap();
    let (src, home) = (dir.path().join("f"), dir.path().join("home"));
    write(&src, "x", 0o600);
    for bad in [
        "../escape",
        "/etc/passwd",
        ".claude/settings.json",
        ".config/opencode/a",
        ".Codex/x",
    ] {
        let failures = deliver_files(&[file(&src, bad, None)], &home);
        assert_eq!(failures.len(), 1, "{bad}");
    }
    assert!(!home.exists() || std::fs::read_dir(&home).unwrap().next().is_none());
    assert!(!dir.path().join("escape").exists());
}
