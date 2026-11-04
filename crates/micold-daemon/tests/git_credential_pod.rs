//! #1528: the daemon of a pod has no `MICOLD_*` variable, only a `daemon.toml` whose `daemon.kind`
//! is `kubernetes`. Installing settings is per process (the first call wins), so this is its own
//! test binary, apart from `git_credential_grant.rs`, whose host-process tests need none installed.

#[path = "support/conn.rs"]
mod conn;
#[path = "support/env.rs"]
mod env_support;

use std::process::Command;
use std::sync::Arc;
use std::time::Duration;

use conn::connect_and_attach;
use env_support::Env;
use futures_util::StreamExt;
use micold_core::protocol::codec::Frame;
use micold_core::protocol::messages::DaemonMsg;
use micold_daemon::catalog::Catalog;
use micold_daemon::state::DaemonState;

#[tokio::test]
async fn a_daemon_run_with_only_the_pods_file_announces_targets() {
    let dir = tempfile::tempdir().unwrap();
    let pod = micold_core::kube::credentials::daemon_env(&Default::default());
    assert!(pod.keys().all(|k| !k.starts_with("MICOLD_")), "{pod:?}");
    let _env = Env::set(&[
        ("MICOLD_GIT_CREDENTIAL_ROOT", None),
        ("MICOLD_IMAGE_REFERENCE", None),
        (micold_core::kube::credentials::POD_ENV, None),
        ("MICOLD_TOKEN_PATH", None),
        ("GIT_CONFIG_GLOBAL", Some("/dev/null".into())),
        ("GIT_CONFIG_NOSYSTEM", Some("1".into())),
    ]);
    let mut settings = micold_daemon::settings::DaemonSettings::resolve(None, &|_| None);
    settings.kind = Some(micold_core::daemon_config::Kind::Kubernetes);
    micold_daemon::settings::install(settings);

    let path = dir.path().join("a");
    std::fs::create_dir_all(&path).unwrap();
    for args in [
        &["init", "-q", "-b", "main"][..],
        &["remote", "add", "origin", "https://github.com/o/r"],
    ] {
        assert!(Command::new("git")
            .arg("-C")
            .arg(&path)
            .args(args)
            .status()
            .unwrap()
            .success());
    }
    let state = Arc::new(DaemonState::new(Catalog::ephemeral()));
    state.add_project(&path).unwrap();
    let mut client = connect_and_attach(&state, &path).await;
    let found = tokio::time::timeout(Duration::from_secs(20), async {
        loop {
            if let Frame::Control(DaemonMsg::GitCredentialTargets { targets, .. }) =
                client.next().await.expect("open").expect("frame")
            {
                return targets;
            }
        }
    })
    .await
    .expect("the targets arrive");
    assert_eq!(found.len(), 1, "{found:?}");
}
