//! The daemon loads its configuration file (#1524, US1, contract R8, research D3 and D11).
//!
//! The pure parts (which file, which value wins) run through `config_load` and `DaemonSettings`
//! with an injected environment; what only a process shows (the exit status, the message) runs the
//! real binary.

use std::collections::HashMap;
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use micold_core::daemon_config::Kind;
use micold_daemon::config_load::{
    load_startup, parse_args, DefaultLocations, LoadError, StartupArgs,
};
use micold_daemon::settings::DaemonSettings;

const DAEMON_BIN: &str = env!("CARGO_BIN_EXE_micold-daemon");

fn config_text(name: &str, kind: &str, extra_daemon: &str, rest: &str) -> String {
    format!("schema = 1\n[daemon]\nname = \"{name}\"\nkind = \"{kind}\"\n{extra_daemon}\n{rest}\n")
}

/// A container or pod daemon must say how it listens (V3).
const SANDBOX_LISTEN: &str =
    "[listen]\ntcp = \"127.0.0.1:7000\"\n[auth]\nclient_token_file = \"/run/micold/secrets/token\"\n";

/// The sandbox rest with a token file that exists and is owner-only, for a load that checks files.
fn sandbox_rest(dir: &Path) -> String {
    let token = put(dir, "token", "t\n");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&token, std::fs::Permissions::from_mode(0o600)).unwrap();
    }
    SANDBOX_LISTEN.replace("/run/micold/secrets/token", &token.display().to_string())
}

fn put(dir: &Path, relative: &str, text: &str) -> PathBuf {
    let file = dir.join(relative);
    std::fs::create_dir_all(file.parent().unwrap()).unwrap();
    std::fs::write(&file, text).unwrap();
    file
}

fn defaults(dir: &Path) -> DefaultLocations {
    DefaultLocations {
        host: Some(dir.join("host/daemon.toml")),
        ssh: Some(dir.join("ssh/daemon.toml")),
        sandbox: dir.join("sandbox/daemon.toml"),
    }
}

fn env_of(pairs: &[(&str, &str)]) -> impl Fn(&str) -> Option<OsString> {
    let map: HashMap<String, OsString> = pairs
        .iter()
        .map(|(k, v)| ((*k).to_string(), OsString::from(*v)))
        .collect();
    move |name| map.get(name).cloned()
}

fn args(list: &[&str]) -> StartupArgs {
    let list: Vec<OsString> = list.iter().map(OsString::from).collect();
    parse_args(&list).expect("valid arguments")
}

fn name_of(startup: &micold_daemon::config_load::Startup) -> String {
    startup.file.as_ref().unwrap().config.daemon.name.clone()
}

// --- T014 ---

#[test]
fn the_config_flag_wins_over_every_default_location() {
    let dir = tempfile::tempdir().unwrap();
    put(
        dir.path(),
        "host/daemon.toml",
        &config_text("from-default", "host", "", ""),
    );
    let chosen = put(
        dir.path(),
        "elsewhere/mine.toml",
        &config_text("from-flag", "host", "", ""),
    );
    let startup = load_startup(
        &args(&["--config", chosen.to_str().unwrap()]),
        &env_of(&[]),
        &defaults(dir.path()),
    )
    .unwrap();
    assert_eq!(name_of(&startup), "from-flag");
    assert_eq!(startup.file.unwrap().path, chosen);
}

#[test]
fn each_kind_reads_its_own_default_location() {
    let dir = tempfile::tempdir().unwrap();
    put(
        dir.path(),
        "host/daemon.toml",
        &config_text("host-file", "host", "", ""),
    );
    put(
        dir.path(),
        "ssh/daemon.toml",
        &config_text("ssh-file", "ssh", "", ""),
    );
    put(
        dir.path(),
        "sandbox/daemon.toml",
        &config_text("container-file", "container", "", &sandbox_rest(dir.path())),
    );
    for (kind, expected) in [
        ("host", "host-file"),
        ("ssh", "ssh-file"),
        ("container", "container-file"),
    ] {
        let startup = load_startup(
            &args(&["--kind", kind]),
            &env_of(&[]),
            &defaults(dir.path()),
        )
        .unwrap();
        assert_eq!(name_of(&startup), expected, "--kind {kind}");
    }
}

#[test]
fn without_a_kind_the_first_default_location_that_has_a_file_is_used() {
    let dir = tempfile::tempdir().unwrap();
    put(
        dir.path(),
        "ssh/daemon.toml",
        &config_text("ssh-file", "ssh", "", ""),
    );
    let startup = load_startup(&args(&[]), &env_of(&[]), &defaults(dir.path())).unwrap();
    assert_eq!(name_of(&startup), "ssh-file");
    assert_eq!(startup.settings.kind, Some(Kind::Ssh));
}

#[test]
fn a_file_of_another_kind_than_asked_for_is_refused() {
    let dir = tempfile::tempdir().unwrap();
    put(
        dir.path(),
        "ssh/daemon.toml",
        &config_text("x", "host", "", ""),
    );
    let error = load_startup(
        &args(&["--kind", "ssh"]),
        &env_of(&[]),
        &defaults(dir.path()),
    )
    .unwrap_err();
    assert!(error.to_string().contains("daemon.kind"), "{error}");
}

#[test]
fn no_file_is_fine_for_a_host_and_falls_back_to_the_environment() {
    let dir = tempfile::tempdir().unwrap();
    let startup = load_startup(
        &args(&["--kind", "host"]),
        &env_of(&[("MICOLD_REPOS_ROOT", "/srv/clones")]),
        &defaults(dir.path()),
    )
    .unwrap();
    assert!(startup.file.is_none());
    assert_eq!(startup.settings.clone_root, Some("/srv/clones".into()));
    let startup = load_startup(&args(&[]), &env_of(&[]), &defaults(dir.path())).unwrap();
    assert!(startup.file.is_none(), "no kind and no file is a host");
}

#[test]
fn no_file_is_fatal_for_the_other_kinds_and_the_message_names_the_place() {
    let dir = tempfile::tempdir().unwrap();
    for kind in ["ssh", "container", "kubernetes"] {
        let error = load_startup(
            &args(&["--kind", kind]),
            &env_of(&[]),
            &defaults(dir.path()),
        )
        .unwrap_err();
        let text = error.to_string();
        assert!(
            text.contains("daemon.toml") && text.contains(kind),
            "{kind}: {text}"
        );
    }
}

#[test]
fn a_config_flag_naming_a_missing_file_is_fatal_for_every_kind() {
    let dir = tempfile::tempdir().unwrap();
    let missing = dir.path().join("nope.toml");
    let error = load_startup(
        &args(&["--config", missing.to_str().unwrap(), "--kind", "host"]),
        &env_of(&[]),
        &defaults(dir.path()),
    )
    .unwrap_err();
    assert!(error.to_string().contains("nope.toml"), "{error}");
}

#[test]
fn an_invalid_file_is_fatal_and_the_message_has_file_line_and_column() {
    let dir = tempfile::tempdir().unwrap();
    let file = put(dir.path(), "bad.toml", "schema = 1\n[daemon]\nname = 3\n");
    let error = load_startup(
        &args(&["--config", file.to_str().unwrap()]),
        &env_of(&[]),
        &defaults(dir.path()),
    )
    .unwrap_err();
    assert!(
        matches!(error, LoadError::Invalid(_)),
        "an invalid file is its own error: {error}"
    );
    assert!(error.to_string().contains("bad.toml:"), "{error}");
}

#[test]
fn unknown_or_incomplete_arguments_are_refused() {
    for bad in [&["--config"][..], &["--kind", "mainframe"], &["--wat"]] {
        let list: Vec<OsString> = bad.iter().map(OsString::from).collect();
        assert!(parse_args(&list).is_err(), "{bad:?}");
    }
}

#[test]
fn a_failed_start_with_the_config_flag_exits_non_zero_and_says_why_on_stderr() {
    let dir = tempfile::tempdir().unwrap();
    let missing = dir.path().join("nope.toml");
    let output = Command::new(DAEMON_BIN)
        .arg("--config")
        .arg(&missing)
        .env("HOME", dir.path())
        .env("XDG_DATA_HOME", dir.path().join("data"))
        .env("XDG_RUNTIME_DIR", dir.path().join("run"))
        .env_remove("MICOLD_DAEMON_CONFIG")
        .stdin(Stdio::null())
        .output()
        .unwrap();
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("fatal") && stderr.contains("nope.toml"),
        "{stderr}"
    );
}

// --- T088 ---

#[test]
fn a_nested_daemon_refuses_the_default_location_and_names_the_flag() {
    let dir = tempfile::tempdir().unwrap();
    put(
        dir.path(),
        "host/daemon.toml",
        &config_text("outer", "host", "", ""),
    );
    let error = load_startup(
        &args(&[]),
        &env_of(&[("MICOLD_DAEMON_CONFIG", "/run/micold/config/daemon.toml")]),
        &defaults(dir.path()),
    )
    .unwrap_err();
    assert!(matches!(error, LoadError::Nested), "{error}");
    assert!(error.to_string().contains("--config"), "{error}");
    // With the flag it starts, whatever the variable says.
    let own = put(
        dir.path(),
        "own.toml",
        &config_text("inner", "host", "", ""),
    );
    let startup = load_startup(
        &args(&["--config", own.to_str().unwrap()]),
        &env_of(&[("MICOLD_DAEMON_CONFIG", "/x")]),
        &defaults(dir.path()),
    )
    .unwrap();
    assert_eq!(name_of(&startup), "inner");
}

#[test]
fn the_real_binary_of_a_nested_daemon_exits_naming_the_flag() {
    let dir = tempfile::tempdir().unwrap();
    let output = Command::new(DAEMON_BIN)
        .env("MICOLD_DAEMON_CONFIG", "/run/micold/config/daemon.toml")
        .env("HOME", dir.path())
        .env("XDG_DATA_HOME", dir.path().join("data"))
        .env("XDG_RUNTIME_DIR", dir.path().join("run"))
        .stdin(Stdio::null())
        .output()
        .unwrap();
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("--config"), "{stderr}");
}

// --- T015: the file wins over the variables ---

const FILE_SETS_EVERYTHING: &str = r#"
schema = 1
[daemon]
name = "all"
kind = "container"
data_dir = "/file/data"
home = "/file/home"
time_zone = "Europe/Berlin"
image = "file/image:1"
idle_stop = 90
log = "debug"
[listen]
tcp = "127.0.0.1:7001"
[auth]
client_token_file = "/file/token"
[storage]
clone_root = "/file/repos"
"#;

const ENV_SETS_EVERYTHING: &[(&str, &str)] = &[
    ("MICOLD_DATA_DIR", "/env/data"),
    ("MICOLD_HOME", "/env/home"),
    ("TZ", "UTC"),
    ("MICOLD_IMAGE_REFERENCE", "env/image:2"),
    ("MICOLD_IDLE_STOP", "5m"),
    ("MICOLD_LOG", "trace"),
    ("MICOLD_LISTEN_ADDR", "0.0.0.0:9"),
    ("MICOLD_TOKEN_PATH", "/env/token"),
    ("MICOLD_REPOS_ROOT", "/env/repos"),
];

fn parsed(text: &str) -> micold_core::daemon_config::DaemonConfig {
    micold_core::daemon_config::diagnose(text, "t.toml")
        .0
        .unwrap_or_else(|| panic!("the fixture must be valid"))
}

#[test]
fn every_value_the_file_sets_beats_the_variable_for_it() {
    let config = parsed(FILE_SETS_EVERYTHING);
    let s = DaemonSettings::resolve(Some(&config), &env_of(ENV_SETS_EVERYTHING));
    assert_eq!(s.kind, Some(Kind::Container));
    assert_eq!(s.data_dir, Some("/file/data".into()));
    assert_eq!(s.home, Some("/file/home".into()));
    assert_eq!(s.time_zone.as_deref(), Some("Europe/Berlin"));
    assert_eq!(s.image, "file/image:1");
    assert_eq!(s.idle_stop.as_deref(), Some("90s"));
    assert_eq!(s.log.as_deref(), Some("debug"));
    assert_eq!(s.listen_tcp.as_deref(), Some("127.0.0.1:7001"));
    assert_eq!(s.token_file, Some("/file/token".into()));
    assert_eq!(s.clone_root, Some("/file/repos".into()));
}

#[test]
fn every_value_the_file_leaves_out_comes_from_the_variable() {
    let config = parsed(&config_text("bare", "host", "", ""));
    let s = DaemonSettings::resolve(Some(&config), &env_of(ENV_SETS_EVERYTHING));
    assert_eq!(s.data_dir, Some("/env/data".into()));
    assert_eq!(s.home, Some("/env/home".into()));
    assert_eq!(s.time_zone.as_deref(), Some("UTC"));
    assert_eq!(s.image, "env/image:2");
    assert_eq!(s.idle_stop.as_deref(), Some("5m"));
    assert_eq!(s.log.as_deref(), Some("trace"));
    assert_eq!(s.listen_tcp.as_deref(), Some("0.0.0.0:9"));
    assert_eq!(s.token_file, Some("/env/token".into()));
    assert_eq!(s.clone_root, Some("/env/repos".into()));
}

#[test]
fn with_no_file_the_variables_are_all_there_is() {
    let s = DaemonSettings::resolve(None, &env_of(ENV_SETS_EVERYTHING));
    assert_eq!(s.kind, None);
    assert_eq!(s.image, "env/image:2");
    assert_eq!(s.idle_stop.as_deref(), Some("5m"));
    let none = DaemonSettings::resolve(None, &env_of(&[]));
    assert_eq!(none, DaemonSettings::default());
}

#[test]
fn a_socket_listener_in_the_file_overrides_a_tcp_address_in_the_environment() {
    let config = parsed(&config_text(
        "sock",
        "host",
        "",
        "[listen]\nsocket = \"auto\"\n",
    ));
    let s = DaemonSettings::resolve(
        Some(&config),
        &env_of(&[("MICOLD_LISTEN_ADDR", "0.0.0.0:9")]),
    );
    assert_eq!(s.listen_tcp, None);
}

#[test]
fn the_idle_stop_words_map_to_the_notation_the_idle_rule_reads() {
    for (written, expected) in [("\"off\"", "off"), ("\"default\"", ""), ("45", "45s")] {
        let config = parsed(&config_text(
            "i",
            "host",
            &format!("idle_stop = {written}"),
            "",
        ));
        let s = DaemonSettings::resolve(Some(&config), &env_of(&[("MICOLD_IDLE_STOP", "9s")]));
        assert_eq!(s.idle_stop.as_deref(), Some(expected), "{written}");
    }
}

// --- T015a ---

#[test]
fn the_credential_root_follows_the_kind_whatever_the_sandbox_inference_says() {
    use micold_daemon::git_credential::credential_root_of;
    let with = |kind: &str, extra: &str| {
        let rest = if matches!(kind, "container" | "kubernetes") {
            SANDBOX_LISTEN
        } else {
            ""
        };
        let config = parsed(&config_text("k", kind, extra, rest));
        DaemonSettings::resolve(Some(&config), &env_of(&[]))
    };
    // A container or pod daemon has a root even with no image, token or pod variable.
    assert!(credential_root_of(&with("container", "")).is_some());
    assert!(credential_root_of(&with("kubernetes", "")).is_some());
    // A host (or SSH) daemon has none, even when the inference would say sandbox.
    let image = "image = \"x/y:1\"";
    let host = DaemonSettings::resolve(
        Some(&parsed(&config_text("h", "host", image, ""))),
        &env_of(&[("MICOLD_TOKEN_PATH", "/t")]),
    );
    assert!(!host.image.is_empty() && host.token_file.is_some());
    assert_eq!(credential_root_of(&host), None);
    assert_eq!(credential_root_of(&with("ssh", "")), None);
}

#[test]
fn without_a_file_the_old_inference_still_decides() {
    use micold_daemon::git_credential::credential_root_of;
    let image = DaemonSettings::resolve(None, &env_of(&[("MICOLD_IMAGE_REFERENCE", "x/y:1")]));
    assert!(credential_root_of(&image).is_some());
    assert_eq!(
        credential_root_of(&DaemonSettings::resolve(None, &env_of(&[]))),
        None
    );
}
