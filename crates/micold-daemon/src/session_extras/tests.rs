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
            "image = \"img\"\n[listen]\ntcp = \"127.0.0.1:7000\"\n[auth]\nclient_token_file = '{}'\n",
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
    #[cfg(not(unix))]
    let _ = mode;
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
        "[git]\nssh_key_file = '{}'\nknown_hosts_file = '{}'\n",
        key.display(),
        hosts.display()
    )
}

/// The arguments `command` gives a POSIX shell, one per line of `printf`, so quoting is checked
/// by parsing the command back rather than by comparing it with the format that built it.
#[cfg(unix)]
fn shell_args(command: &str) -> Vec<String> {
    let out = Command::new("sh")
        .arg("-c")
        .arg(format!("printf '%s\\n' {command}"))
        .output()
        .expect("sh runs");
    String::from_utf8(out.stdout)
        .unwrap()
        .lines()
        .map(str::to_string)
        .collect()
}

#[cfg(unix)]
fn ssh_command(dir: &Path, git_tail: &str) -> Option<String> {
    let cfg = config(dir, "container", git_tail);
    extras(&cfg)
        .env
        .into_iter()
        .find(|(k, _)| k == "GIT_SSH_COMMAND")
        .map(|(_, v)| v)
}

#[cfg(unix)]
#[test]
fn a_sandbox_session_ssh_command_names_the_key_and_the_known_hosts() {
    let dir = tempfile::tempdir().unwrap();
    let cmd = ssh_command(dir.path(), &git_section(dir.path())).unwrap();
    let (key, hosts) = (dir.path().join("id"), dir.path().join("known"));
    assert_eq!(
        shell_args(&cmd),
        [
            "ssh".to_string(),
            "-i".into(),
            key.display().to_string(),
            "-o".into(),
            format!("UserKnownHostsFile={}", hosts.display()),
            "-o".into(),
            "IdentitiesOnly=yes".into(),
        ]
    );
}

#[cfg(unix)]
#[test]
fn a_key_path_with_a_space_stays_one_argument() {
    let dir = tempfile::tempdir().unwrap();
    let spaced = dir.path().join("my keys");
    std::fs::create_dir(&spaced).unwrap();
    let cmd = ssh_command(dir.path(), &git_section(&spaced)).unwrap();
    let args = shell_args(&cmd);
    assert_eq!(args.len(), 7, "{args:?}");
    assert_eq!(args[2], spaced.join("id").display().to_string());
    assert_eq!(
        args[4],
        format!("UserKnownHostsFile={}", spaced.join("known").display())
    );
}

#[test]
fn the_ssh_command_needs_both_the_key_and_the_known_hosts() {
    // V18 refuses one without the other, so the half-set configurations are built by hand.
    let dir = tempfile::tempdir().unwrap();
    let full = config(dir.path(), "container", &git_section(dir.path()));
    assert!(extras(&full)
        .env
        .iter()
        .any(|(k, _)| k == "GIT_SSH_COMMAND"));
    let mut only_key = full.clone();
    only_key.git.known_hosts_file = None;
    let mut only_hosts = full;
    only_hosts.git.ssh_key_file = None;
    for cfg in [only_key, only_hosts] {
        assert!(extras(&cfg).env.iter().all(|(k, _)| k != "GIT_SSH_COMMAND"));
    }
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
        &format!("[git]\nsigning_key_file = '{}'\n", key.display()),
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
/// with the environment `edits` give; true when the commit succeeds. Panics without git.
fn commit_with(dir: &Path, edits: &[EnvEdit]) -> bool {
    let repo = dir.join("repo");
    let _ = std::fs::remove_dir_all(&repo);
    std::fs::create_dir_all(&repo).unwrap();
    let (global, system) = (dir.join("gitconfig"), dir.join("gitconfig-system"));
    write(
        &global,
        "[user]\nname = t\nemail = t@example.com\n[gpg]\nformat = ssh\n[commit]\ngpgsign = true\n",
        0o600,
    );
    write(&system, "", 0o600);
    let git = |args: &[&str]| {
        let mut cmd = Command::new("git");
        no_window(&mut cmd)
            .current_dir(&repo)
            .args(args)
            .env("GIT_CONFIG_GLOBAL", &global)
            .env("GIT_CONFIG_SYSTEM", &system);
        for (k, v) in vars(edits) {
            cmd.env(k, v);
        }
        cmd.output().expect("git is installed and runs")
    };
    assert!(git(&["init", "-q"]).status.success(), "git init");
    git(&["commit", "-q", "--allow-empty", "-m", "m"])
        .status
        .success()
}

fn ssh_keygen(path: &Path) {
    let status = no_window(&mut Command::new("ssh-keygen"))
        .args(["-q", "-t", "ed25519", "-N", "", "-f"])
        .arg(path)
        .status()
        .expect("ssh-keygen is installed (openssh) for the signing tests");
    assert!(status.success(), "ssh-keygen");
}

fn extras_edits(dir: &Path, tail: &str) -> Vec<EnvEdit> {
    add_extras(Vec::new(), extras(&config(dir, "container", tail)))
}

#[test]
fn a_signed_commit_succeeds_with_the_delivered_key() {
    let dir = tempfile::tempdir().unwrap();
    let key = dir.path().join("sign");
    ssh_keygen(&key);
    let edits = extras_edits(
        dir.path(),
        &format!("[git]\nsigning_key_file = '{}'\n", key.display()),
    );
    assert!(
        commit_with(dir.path(), &edits),
        "a commit signed with the key"
    );
    let log = no_window(&mut Command::new("git"))
        .current_dir(dir.path().join("repo"))
        .args(["cat-file", "-p", "HEAD"])
        .output()
        .unwrap();
    assert!(String::from_utf8_lossy(&log.stdout).contains("gpgsig"));
}

#[test]
fn an_unsigned_commit_succeeds_with_the_extras_when_no_key_arrived() {
    let dir = tempfile::tempdir().unwrap();
    assert!(commit_with(dir.path(), &extras_edits(dir.path(), "")));
}

#[test]
fn signing_on_without_the_extras_fails_the_commit() {
    let dir = tempfile::tempdir().unwrap();
    assert!(!commit_with(dir.path(), &[]), "signing is on without them");
}

#[test]
fn the_ca_file_reaches_every_tool_that_reads_one() {
    let dir = tempfile::tempdir().unwrap();
    let ca = dir.path().join("ca.pem");
    write(&ca, "pem", 0o644);
    let cfg = config(
        dir.path(),
        "host",
        &format!("[network]\nca_file = '{}'\n", ca.display()),
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
            "[session.env]\nNO_PROXY = \"localhost,.corp\"\n[session.secret_env]\nHTTPS_PROXY = '{}'\n",
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

#[cfg(unix)]
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

#[test]
fn a_secret_env_file_that_cannot_be_read_gives_no_proxy() {
    let dir = tempfile::tempdir().unwrap();
    let secret = dir.path().join("proxy");
    write(&secret, "http://proxy.corp:3128\n", 0o600);
    let toml = format!(
        "schema = 1\n[daemon]\nname = \"t\"\nkind = \"host\"\n[session.secret_env]\nHTTPS_PROXY = '{}'\n",
        secret.display()
    );
    let path = dir.path().join("daemon.toml");
    write(&path, &toml, 0o600);
    assert_eq!(
        Egress::load(Some(&path)).https_proxy.as_deref(),
        Some("http://proxy.corp:3128")
    );
    let cfg = config(dir.path(), "host", &toml[toml.find("[session").unwrap()..]);
    std::fs::remove_file(&secret).unwrap();
    assert_eq!(Egress::of(&cfg), Egress::default());
    assert_eq!(Egress::load(Some(&path)), Egress::default());
}

#[test]
fn a_session_file_with_a_missing_source_or_a_bad_mode_is_reported() {
    let dir = tempfile::tempdir().unwrap();
    let (src, home) = (dir.path().join("f"), dir.path().join("home"));
    write(&src, "x", 0o600);
    let missing = deliver_files(&[file(&dir.path().join("nope"), ".a", None)], &home);
    assert_eq!(missing.len(), 1, "{missing:?}");
    assert!(missing[0].contains("cannot copy"), "{missing:?}");
    assert!(!home.join(".a").exists());
    let bad = deliver_files(&[file(&src, ".b", Some("rw-"))], &home);
    assert_eq!(bad.len(), 1, "{bad:?}");
    assert!(bad[0].contains("not octal"), "{bad:?}");
    assert!(!home.join(".b").exists());
}

#[cfg(unix)]
#[test]
fn a_read_only_copy_already_there_is_replaced() {
    use std::os::unix::fs::PermissionsExt;
    let dir = tempfile::tempdir().unwrap();
    let (src, home) = (dir.path().join("f"), dir.path().join("home"));
    write(&src, "new", 0o600);
    std::fs::create_dir_all(&home).unwrap();
    write(&home.join(".rc"), "old", 0o400);
    let failures = deliver_files(&[file(&src, ".rc", Some("0400"))], &home);
    assert!(failures.is_empty(), "{failures:?}");
    assert_eq!(std::fs::read_to_string(home.join(".rc")).unwrap(), "new");
    let mode = std::fs::metadata(home.join(".rc"))
        .unwrap()
        .permissions()
        .mode();
    assert_eq!(mode & 0o777, 0o400);
}

#[test]
fn a_target_under_another_target_is_reported_and_leaves_the_first() {
    let dir = tempfile::tempdir().unwrap();
    let (src, home) = (dir.path().join("f"), dir.path().join("home"));
    write(&src, "x", 0o600);
    let failures = deliver_files(&[file(&src, "a", None), file(&src, "a/b", None)], &home);
    assert_eq!(failures.len(), 1, "{failures:?}");
    assert!(failures[0].starts_with("a/b"), "{failures:?}");
    assert_eq!(std::fs::read_to_string(home.join("a")).unwrap(), "x");
}
