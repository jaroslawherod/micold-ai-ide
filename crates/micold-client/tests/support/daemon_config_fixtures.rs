//! One Settings fixture per daemon kind for the daemon-configuration renderer tests (#1525).

#![allow(dead_code)]

use micold_client::daemon_config_render::{DaemonSettings, ProjectSetting};
use micold_core::daemon_config::{IdleStopSetting, Kind, Listen, SocketSpec};

pub const KINDS: [Kind; 4] = [Kind::Host, Kind::Ssh, Kind::Container, Kind::Kubernetes];

pub fn golden(kind: Kind) -> String {
    let path = format!(
        "{}/tests/golden/daemon_config/{}.toml",
        env!("CARGO_MANIFEST_DIR"),
        kind.as_str()
    );
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{path}: {e}"))
}

pub fn settings(kind: Kind) -> DaemonSettings {
    let project = |path: &str, remote: Option<&str>| ProjectSetting {
        path: path.into(),
        remote: remote.map(str::to_string),
    };
    match kind {
        Kind::Host => DaemonSettings {
            data_dir: Some("/home/me/.local/share/micold-ai-ide".into()),
            time_zone: Some("Europe/Warsaw".into()),
            idle_stop: Some(IdleStopSetting::Default),
            log: Some("info".into()),
            clone_root: Some("/home/me/src".into()),
            projects: vec![project(
                "/home/me/src/app",
                Some("https://github.com/acme/app.git"),
            )],
            ..DaemonSettings::new("Host", Listen::Socket(SocketSpec::Auto))
        },
        Kind::Ssh => DaemonSettings {
            home: Some("/home/dev".into()),
            idle_stop: Some(IdleStopSetting::Secs(900)),
            log: Some("debug".into()),
            clone_root: Some("/home/dev/repos".into()),
            projects: vec![
                project("/home/dev/repos/app", None),
                project(
                    "/home/dev/repos/lib",
                    Some("https://github.com/acme/lib.git"),
                ),
            ],
            ..DaemonSettings::new(
                "Remote box",
                Listen::Socket(SocketSpec::Path("/run/user/1000/micold.sock".into())),
            )
        },
        Kind::Container => DaemonSettings {
            image: Some("ghcr.io/acme/micold:1".into()),
            idle_stop: Some(IdleStopSetting::Default),
            client_token_file: Some("/run/micold/secrets/client-token".into()),
            ssh_dir: Some("/run/micold/ssh".into()),
            clone_root: Some("/workspace".into()),
            ..DaemonSettings::new("Sandbox", Listen::Tcp("0.0.0.0:7000".parse().unwrap()))
        },
        Kind::Kubernetes => DaemonSettings {
            time_zone: Some("UTC".into()),
            idle_stop: Some(IdleStopSetting::Off),
            client_token_file: Some("/run/micold/secrets/client-token".into()),
            clone_root: Some("/data/repos".into()),
            ..DaemonSettings::new("Pod", Listen::Tcp("0.0.0.0:7000".parse().unwrap()))
        },
    }
}
