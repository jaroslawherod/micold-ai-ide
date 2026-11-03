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

    let expected = micold_client::daemon_config_render::prepare(
        &settings(),
        micold_core::daemon_config::Kind::Host,
    )
    .unwrap();
    assert_eq!(std::fs::read_to_string(&path).unwrap(), expected);

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
