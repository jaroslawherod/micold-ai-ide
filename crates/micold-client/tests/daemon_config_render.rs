//! `render(&DaemonSettings, Kind)` writes one golden file per kind, and the file parses back to the
//! Settings it was rendered from (#1525, US1, FR-005).

use crate::fixtures;

use fixtures::{golden, settings, KINDS};
use micold_client::daemon_config_render::{render, DaemonSettings, ProjectSetting};
use micold_core::daemon_config::{parse, Kind, Listen, SocketSpec};
use std::path::Path;

#[test]
fn each_kind_renders_byte_for_byte_to_its_golden_file() {
    for kind in KINDS {
        assert_eq!(
            render(&settings(kind), kind),
            golden(kind),
            "{kind:?} must render byte for byte to tests/golden/daemon_config/{}.toml",
            kind.as_str()
        );
    }
}

/// The Settings a rendered file says, read back through the daemon's own parser.
fn read_back(text: &str, kind: Kind) -> DaemonSettings {
    let config =
        parse(text, "daemon.toml").unwrap_or_else(|e| panic!("{kind:?}: {}\n{text}", e[0]));
    DaemonSettings {
        name: config.daemon.name,
        data_dir: config.daemon.data_dir,
        home: config.daemon.home,
        time_zone: config.daemon.time_zone,
        image: config.daemon.image,
        idle_stop: config.daemon.idle_stop,
        log: config.daemon.log,
        listen: Some(config.listen.and_then(|l| l.listen()).expect("a listener")),
        client_token_file: config
            .auth
            .and_then(|a| a.client_token_file)
            .map(|s| s.path().to_path_buf()),
        ssh_dir: config.ssh.and_then(|s| s.dir),
        clone_root: config.storage.and_then(|s| s.clone_root),
        projects: config
            .projects
            .into_iter()
            .map(|p| ProjectSetting {
                path: p.path,
                remote: p.remote,
            })
            .collect(),
        startup: None,
        github_tokens: vec![],
        git_config_file: None,
        extras: Default::default(),
    }
}

#[test]
fn the_rendered_file_parses_back_to_its_settings() {
    for kind in KINDS {
        let s = settings(kind);
        assert_eq!(
            read_back(&render(&s, kind), kind),
            s,
            "{kind:?}: every setting survives the file"
        );
    }
}

#[test]
fn a_value_that_needs_quoting_survives_the_file() {
    // An apostrophe with a backslash cannot be a literal string; a control character and an empty
    // value are escaped or kept, never dropped or broken (R4).
    let awkward = [
        ("apostrophe and backslash", r"it's a\b"),
        ("control character", "bell\u{7}tab\there"),
        ("backslash and control character", "a\\b\u{7}"),
        ("empty", ""),
        ("double quote", "say \"hi\""),
    ];
    for (what, value) in awkward {
        let mut s = settings(Kind::Host);
        s.name = value.into();
        s.time_zone = Some(value.into());
        s.log = Some(value.into());
        if !value.is_empty() {
            // Paths must be absolute; the awkward part follows the root.
            s.data_dir = Some(format!("/{value}").into());
            s.clone_root = Some(format!("/{value}/src").into());
            s.projects[0].path = format!("/{value}/src/app").into();
            s.projects[0].remote = Some(value.into());
        }
        assert_eq!(
            read_back(&render(&s, Kind::Host), Kind::Host),
            s,
            "{what}: {value:?}"
        );
    }
}

#[test]
fn a_windows_path_is_a_literal_string_and_a_quote_is_escaped() {
    let mut s = settings(Kind::Host);
    s.listen = Some(Listen::Socket(SocketSpec::Auto));
    s.data_dir = Some(r"C:\Users\me\data".into());
    s.name = "A \"quoted\" name".into();
    let text = render(&s, Kind::Host);
    assert!(text.contains(r"data_dir = 'C:\Users\me\data'"), "{text}");
    assert!(text.contains(r#"name = "A \"quoted\" name""#), "{text}");
}

#[test]
fn a_project_added_at_runtime_renders_in_the_next_file() {
    let kind = micold_core::daemon_config::Kind::Host;
    let mut s = settings(kind);
    s.add_runtime_project("/home/me/src/added".as_ref());
    // Adding it again changes nothing: the file is rewritten only when it differs.
    let once = render(&s, kind);
    s.add_runtime_project("/home/me/src/added".as_ref());
    assert_eq!(render(&s, kind), once);
    assert_eq!(once, fixtures::golden_named("host_project_added"));
    parse(&once, "daemon.toml").unwrap_or_else(|e| panic!("{}", e[0]));
}

/// #1528: a pod's file (contract §3.4) validates, names the mounts the pod has and the zone the
/// pod's `TZ` has, and keeps the idle stop off that the old `MICOLD_IDLE_STOP` gave it.
#[test]
fn a_pods_file_validates_and_names_the_pods_mounts() {
    let settings = DaemonSettings::for_pod(
        "k8s-dev",
        "ghcr.io/acme/basic:1",
        Some("Europe/Berlin"),
        false,
    );
    let text = micold_client::daemon_config_render::prepare(&settings, Kind::Kubernetes)
        .unwrap_or_else(|e| panic!("{}", e[0]));
    assert_eq!(text, include_str!("golden/daemon_config/pod.toml"));
    assert!(!text.contains("MICOLD_"), "{text}");
}

/// #1530: with the shared git configuration mounted, the pod's file names the mounted copy.
#[test]
fn a_pods_file_names_the_mounted_git_config() {
    let settings = DaemonSettings::for_pod("k8s-dev", "ghcr.io/acme/basic:1", None, true);
    let text = micold_client::daemon_config_render::prepare(&settings, Kind::Kubernetes)
        .unwrap_or_else(|e| panic!("{}", e[0]));
    assert!(
        text.contains("config_file = \"/run/micold/secrets/startup/gitconfig\""),
        "{text}"
    );
}

#[test]
fn the_sandbox_extras_render_and_parse_back() {
    use micold_core::daemons::{SessionExtras, SessionFileSetting};
    let mut s = settings(Kind::Container);
    s.extras = SessionExtras {
        ssh_key_file: Some("/keys/id".into()),
        known_hosts_file: Some("/keys/known".into()),
        signing_key_file: Some("/keys/sign".into()),
        ca_file: Some("/etc/corp-ca.pem".into()),
        files: vec![SessionFileSetting {
            file: "/secrets/npmrc".into(),
            target: ".npmrc".into(),
            mode: Some("0640".into()),
        }],
    };
    let text = render(&s, Kind::Container);
    let config = parse(&text, "daemon.toml").unwrap_or_else(|e| panic!("{}\n{text}", e[0]));
    assert_eq!(
        config.git.ssh_key_file.unwrap().path(),
        Path::new("/keys/id")
    );
    assert_eq!(
        config.git.known_hosts_file.as_deref(),
        Some(Path::new("/keys/known"))
    );
    assert_eq!(
        config.git.signing_key_file.unwrap().path(),
        Path::new("/keys/sign")
    );
    assert_eq!(
        config.network.unwrap().ca_file.as_deref(),
        Some(Path::new("/etc/corp-ca.pem"))
    );
    let file = &config.session.files[0];
    assert_eq!(
        (
            file.file.as_path(),
            file.as_path.as_path(),
            file.mode.as_deref()
        ),
        (
            Path::new("/secrets/npmrc"),
            Path::new(".npmrc"),
            Some("0640")
        )
    );
    // Nothing set: none of the tables appears.
    let plain = render(&settings(Kind::Container), Kind::Container);
    assert!(!plain.contains("[git]") && !plain.contains("[network]") && !plain.contains("session"));
}
