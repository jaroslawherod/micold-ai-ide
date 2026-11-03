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
