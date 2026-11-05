//! Host delivery (#1526, M1a, T025/T027/U21-U22): `deliver_host` writes `<data_dir>/daemon.toml`,
//! mode `0600`, only when it changed. The `spawn_detached_daemon` env regression (U23) lives in its
//! own target (`daemon_config_delivery_host_env.rs`): it sets process env vars, which must not run
//! concurrently with the rest of this suite (see `tests/main.rs`'s header comment).

use micold_client::daemon_config_delivery::deliver_host;
use micold_client::daemon_config_render::DaemonSettings;
use micold_core::daemon_config::{Listen, SocketSpec};

fn settings() -> DaemonSettings {
    DaemonSettings::new("Host", Listen::Socket(SocketSpec::Auto))
}

#[test]
fn first_call_writes_the_file_mode_0600() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("daemon.toml");

    let wrote = deliver_host(&settings(), dir.path()).unwrap();
    assert!(wrote, "first call must write");
    assert!(path.is_file());

    // A golden text, not `prepare`: the function `deliver_host` itself calls (I5-9).
    let golden = include_str!("fixtures/daemon_config_delivery_host.golden.toml");
    assert_eq!(std::fs::read_to_string(&path).unwrap(), golden);

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = std::fs::metadata(&path).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o600);
    }
}

#[test]
fn second_call_with_unchanged_settings_is_a_no_op() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("daemon.toml");

    assert!(deliver_host(&settings(), dir.path()).unwrap());
    let written_at = std::fs::metadata(&path).unwrap().modified().unwrap();

    let wrote_again = deliver_host(&settings(), dir.path()).unwrap();
    assert!(!wrote_again, "unchanged settings must not rewrite");
    assert_eq!(
        std::fs::metadata(&path).unwrap().modified().unwrap(),
        written_at,
        "file must be untouched"
    );
}

#[test]
fn a_settings_change_writes_again() {
    let dir = tempfile::tempdir().unwrap();

    assert!(deliver_host(&settings(), dir.path()).unwrap());

    let mut changed = settings();
    changed.add_runtime_project(std::path::Path::new("/home/me/src/app"));

    assert!(
        deliver_host(&changed, dir.path()).unwrap(),
        "a changed project list must rewrite"
    );
    let path = dir.path().join("daemon.toml");
    let expected = micold_client::daemon_config_render::prepare(
        &changed,
        micold_core::daemon_config::Kind::Host,
    )
    .unwrap();
    assert_eq!(std::fs::read_to_string(&path).unwrap(), expected);
}

/// I5-6 (mutant: no `create_dir_all`): a first run, when the data dir does not exist yet.
#[test]
fn a_data_dir_that_does_not_exist_yet_is_created() {
    let dir = tempfile::tempdir().unwrap();
    let data_dir = dir.path().join("not").join("yet");

    assert!(deliver_host(&settings(), &data_dir).unwrap());
    assert!(data_dir.join("daemon.toml").is_file());
}

/// I5-7 (decision: the host path corrects a hand-widened mode, as the SSH path does): an
/// unchanged file left `0644` is not rewritten but is made `0600` again.
#[cfg(unix)]
#[test]
fn an_unchanged_file_with_a_widened_mode_is_made_private_again() {
    use std::os::unix::fs::PermissionsExt;
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("daemon.toml");
    assert!(deliver_host(&settings(), dir.path()).unwrap());
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o644)).unwrap();

    assert!(
        !deliver_host(&settings(), dir.path()).unwrap(),
        "the content is unchanged"
    );
    let mode = std::fs::metadata(&path).unwrap().permissions().mode() & 0o777;
    assert_eq!(mode, 0o600);
}

/// #1533 M4 (FR-011, US5.4): a host daemon's start script and `env_file` are written beside its
/// `daemon.toml`, the file names those copies, and neither the script text nor a secret is in it.
#[test]
fn the_script_and_env_file_are_delivered_beside_the_config_and_not_into_it() {
    use micold_client::daemon_config_render::StartupTable;
    use micold_core::daemons::{StartupScript, StartupSettings};

    let app = tempfile::tempdir().unwrap();
    let data = tempfile::tempdir().unwrap();
    let script = app.path().join("mine.sh");
    let env = app.path().join("mine.env");
    std::fs::write(&script, "echo SCRIPT_BODY\n").unwrap();
    std::fs::write(&env, "TOKEN=SECRET_VALUE\n").unwrap();
    let mut table = StartupSettings::new(StartupScript::File(script.clone()));
    table.env_file = Some(env);
    let mut settings = settings();
    settings.startup = Some(StartupTable {
        script_file: script,
        settings: table,
    });

    assert!(deliver_host(&settings, data.path()).unwrap());

    assert_eq!(
        std::fs::read_to_string(data.path().join("startup.sh")).unwrap(),
        "echo SCRIPT_BODY\n"
    );
    assert_eq!(
        std::fs::read_to_string(data.path().join("startup.env")).unwrap(),
        "TOKEN=SECRET_VALUE\n"
    );
    let config = std::fs::read_to_string(data.path().join("daemon.toml")).unwrap();
    let delivered = data.path().join("startup.sh");
    assert!(config.contains(&format!("script_file = \"{}\"", delivered.display())));
    assert!(!config.contains("SCRIPT_BODY") && !config.contains("SECRET_VALUE"));
    assert!(
        !deliver_host(&settings, data.path()).unwrap(),
        "an unchanged delivery writes nothing"
    );
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = |n: &str| {
            std::fs::metadata(data.path().join(n))
                .unwrap()
                .permissions()
                .mode()
                & 0o777
        };
        assert_eq!((mode("startup.sh"), mode("startup.env")), (0o600, 0o600));
    }
}

/// A script that cannot be read stops the delivery with its path, instead of a daemon that starts
/// without it.
#[test]
fn an_unreadable_script_stops_the_delivery_naming_it() {
    use micold_client::daemon_config_render::StartupTable;
    use micold_core::daemons::{StartupScript, StartupSettings};

    let data = tempfile::tempdir().unwrap();
    let missing = data.path().join("gone.sh");
    let mut settings = settings();
    settings.startup = Some(StartupTable {
        script_file: missing.clone(),
        settings: StartupSettings::new(StartupScript::File(missing)),
    });
    let err = deliver_host(&settings, data.path()).unwrap_err();
    assert!(err.to_string().contains("gone.sh"), "{err}");
    assert!(!data.path().join("daemon.toml").exists());
}

// --- container delivery (#1527, T031, FR-005) --------------------------------------------------

mod container {
    use micold_client::daemon_config_delivery::deliver_container;
    use micold_core::sandbox::image::{ImageChoice, OwnImage};
    use micold_core::sandbox::{DeliveryMount, SandboxProfile};
    use std::path::Path;

    fn profile(survive_logout: bool) -> SandboxProfile {
        SandboxProfile {
            image: ImageChoice::Own(OwnImage::Reference("acme/ssh:1".into())),
            survive_logout,
            ..SandboxProfile::default()
        }
    }

    #[test]
    fn the_file_carries_what_the_create_arguments_no_longer_do_and_is_valid() {
        let dir = tempfile::tempdir().unwrap();
        let delivery = DeliveryMount::beside(&dir.path().join("state"));
        let home = Path::new("/home/jaro");

        assert!(deliver_container("box", &profile(true), home, &delivery, None).unwrap());
        let text = std::fs::read_to_string(delivery.config.join("daemon.toml")).unwrap();
        let (config, errors) = micold_core::daemon_config::diagnose(&text, "daemon.toml");
        assert!(errors.is_empty(), "{errors:?}\n{text}");
        let config = config.unwrap();
        let ssh_dir = config.ssh.clone().and_then(|s| s.dir);
        let daemon = config.daemon;
        assert_eq!(daemon.kind, micold_core::daemon_config::Kind::Container);
        assert_eq!(
            daemon.data_dir.as_deref(),
            Some(Path::new("/var/lib/micold-ai-ide"))
        );
        assert_eq!(daemon.home.as_deref(), Some(home));
        // The entrypoint reads `[ssh] dir`; the value is the one the container mounts the keys at,
        // not an unset table (which resolves to None).
        assert_eq!(
            ssh_dir.as_deref(),
            Some(Path::new(micold_core::sandbox::argv::CONTAINER_SSH_DIR))
        );
        assert_eq!(daemon.image.as_deref(), Some("acme/ssh:1"));
        assert_eq!(
            daemon.idle_stop,
            Some(micold_core::daemon_config::IdleStopSetting::Off)
        );
        assert!(delivery.secrets.is_dir(), "the secrets folder is made too");
    }

    #[test]
    fn the_keep_running_opt_in_off_leaves_the_idle_window_alone_and_a_repeat_writes_nothing() {
        let dir = tempfile::tempdir().unwrap();
        let delivery = DeliveryMount::beside(&dir.path().join("state"));
        let home = Path::new("/home/jaro");

        assert!(deliver_container("box", &profile(false), home, &delivery, None).unwrap());
        let text = std::fs::read_to_string(delivery.config.join("daemon.toml")).unwrap();
        assert!(!text.contains("idle_stop"), "{text}");
        assert!(!deliver_container("box", &profile(false), home, &delivery, None).unwrap());
        assert!(deliver_container("box", &profile(true), home, &delivery, None).unwrap());
    }

    fn startup_in(app: &Path, with_env: bool) -> micold_client::daemon_config_render::StartupTable {
        use micold_core::daemons::{StartupScript, StartupSettings};
        let script = app.join("mine.sh");
        std::fs::write(&script, "echo SCRIPT_BODY\n").unwrap();
        let mut settings = StartupSettings::new(StartupScript::File(script.clone()));
        if with_env {
            let env = app.join("mine.env");
            std::fs::write(&env, "TOKEN=SECRET_VALUE\n").unwrap();
            settings.env_file = Some(env);
        }
        micold_client::daemon_config_render::StartupTable {
            script_file: script,
            settings,
        }
    }

    /// #1533 M5 (FR-011, US1.2, US5.4): the script goes into the folder mounted at
    /// `/run/micold/config` and the `env_file` into the one mounted at `/run/micold/secrets`; the
    /// file names the paths inside the container, and holds neither the script nor a secret.
    #[test]
    fn the_script_and_env_file_go_to_the_config_and_secrets_folders() {
        let dir = tempfile::tempdir().unwrap();
        let delivery = DeliveryMount::beside(&dir.path().join("state"));
        let startup = startup_in(dir.path(), true);

        deliver_container(
            "box",
            &profile(false),
            Path::new("/home/jaro"),
            &delivery,
            Some(&startup),
        )
        .unwrap();

        assert_eq!(
            std::fs::read_to_string(delivery.config.join("startup.sh")).unwrap(),
            "echo SCRIPT_BODY\n"
        );
        assert_eq!(
            std::fs::read_to_string(delivery.secrets.join("startup.env")).unwrap(),
            "TOKEN=SECRET_VALUE\n"
        );
        let text = std::fs::read_to_string(delivery.config.join("daemon.toml")).unwrap();
        assert!(
            text.contains("script_file = \"/run/micold/config/startup.sh\""),
            "{text}"
        );
        assert!(
            text.contains("env_file = \"/run/micold/secrets/startup.env\""),
            "{text}"
        );
        assert!(
            !text.contains("SCRIPT_BODY") && !text.contains("SECRET_VALUE"),
            "{text}"
        );
        let (_, errors) = micold_core::daemon_config::diagnose(&text, "daemon.toml");
        assert!(errors.is_empty(), "{errors:?}\n{text}");
    }

    /// A script taken away leaves no stale copy of it or of its secret in the folders.
    #[test]
    fn a_daemon_without_a_script_leaves_no_stale_copy() {
        let dir = tempfile::tempdir().unwrap();
        let delivery = DeliveryMount::beside(&dir.path().join("state"));
        let home = Path::new("/home/jaro");
        let startup = startup_in(dir.path(), true);
        deliver_container("box", &profile(false), home, &delivery, Some(&startup)).unwrap();

        deliver_container("box", &profile(false), home, &delivery, None).unwrap();

        assert!(!delivery.config.join("startup.sh").exists());
        assert!(!delivery.secrets.join("startup.env").exists());
    }

    #[test]
    fn an_unreadable_script_stops_the_delivery_naming_it() {
        let dir = tempfile::tempdir().unwrap();
        let delivery = DeliveryMount::beside(&dir.path().join("state"));
        let mut startup = startup_in(dir.path(), false);
        startup.settings.script =
            micold_core::daemons::StartupScript::File(dir.path().join("gone.sh"));
        startup.script_file = dir.path().join("gone.sh");

        let err = deliver_container(
            "box",
            &profile(false),
            Path::new("/home/jaro"),
            &delivery,
            Some(&startup),
        )
        .unwrap_err();
        assert!(err.to_string().contains("gone.sh"), "{err}");
    }
}

/// #1533 M6 (FR-011, US4.4): a pod's script and `env_file` become the config map's and the startup
/// secret's text, the rendered file names their mounts, and neither the script nor a secret value
/// is in it. An unreadable script stops the start naming it.
#[test]
fn a_pods_script_and_env_file_become_texts_and_the_file_names_their_mounts() {
    use micold_client::daemon_config_delivery::pod_startup;
    use micold_client::daemon_config_render::{prepare, StartupTable};
    use micold_core::daemon_config::Kind;
    use micold_core::daemons::{StartupScript, StartupSettings};

    let app = tempfile::tempdir().unwrap();
    let script = app.path().join("mine.sh");
    let env = app.path().join("mine.env");
    std::fs::write(&script, "echo SCRIPT_BODY\n").unwrap();
    std::fs::write(&env, "TOKEN=SECRET_VALUE\n").unwrap();
    let mut table = StartupSettings::new(StartupScript::File(script.clone()));
    table.env_file = Some(env);
    let mut settings = DaemonSettings::for_pod("Pod", "img:1", None);
    settings.startup = Some(StartupTable {
        script_file: script,
        settings: table,
    });

    let delivered = pod_startup(&settings).unwrap();
    assert_eq!(delivered.script.as_deref(), Some("echo SCRIPT_BODY\n"));
    assert_eq!(delivered.env.as_deref(), Some("TOKEN=SECRET_VALUE\n"));
    let config = prepare(&delivered.settings, Kind::Kubernetes).unwrap();
    assert!(config.contains("script_file = \"/run/micold/config/startup.sh\""));
    assert!(config.contains("env_file = \"/run/micold/secrets/startup/startup.env\""));
    assert!(!config.contains("SCRIPT_BODY") && !config.contains("SECRET_VALUE"));

    let none = pod_startup(&DaemonSettings::for_pod("Pod", "img:1", None)).unwrap();
    assert_eq!((none.script, none.env), (None, None));

    let mut missing = settings.clone();
    missing.startup.as_mut().unwrap().script_file = app.path().join("gone.sh");
    let err = pod_startup(&missing).unwrap_err();
    assert!(err.to_string().contains("gone.sh"), "{err}");

    let big = app.path().join("big.sh");
    std::fs::write(&big, vec![b'#'; 512 * 1024 + 1]).unwrap();
    let mut large = settings.clone();
    large.startup.as_mut().unwrap().script_file = big;
    let err = pod_startup(&large).unwrap_err();
    assert!(err.to_string().contains("big.sh"), "{err}");
}
