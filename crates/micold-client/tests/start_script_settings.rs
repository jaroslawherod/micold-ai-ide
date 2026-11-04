//! Settings → Daemons → Start script (#1533, M3, FR-010, US5): the group exists only for a kind
//! whose script is delivered, an inline script is a file beside the app's data, `[startup]` renders
//! with the delivered `script_file`, and a bad value is refused before delivery.

use micold_client::daemon_config_render::{
    inline_script_path, load_inline_script, prepare, render, save_inline_script, DaemonSettings,
    StartupTable,
};
use micold_client::features::daemons::{DraftRuntime, Section};
use micold_client::features::daemons_start::{startup_restart_pending, StartDraft, StartSource};
use micold_core::daemon_config::{parse, Kind, Listen, OnFailure, RunPolicy, SocketSpec};
use micold_core::daemons::{
    DaemonRegistry, DaemonRuntime, FileSettings, StartupScript, StartupSettings,
};

const RUNTIMES: [DraftRuntime; 4] = [
    DraftRuntime::Container,
    DraftRuntime::Kubernetes,
    DraftRuntime::Ssh,
    DraftRuntime::Host,
];

fn with_startup(settings: StartupSettings) -> DaemonSettings {
    DaemonSettings {
        startup: Some(StartupTable {
            script_file: "/data/startup.sh".into(),
            settings,
        }),
        ..DaemonSettings::new("Host", Listen::Socket(SocketSpec::Auto))
    }
}

#[test]
fn the_group_is_offered_only_for_a_kind_that_delivers() {
    for runtime in RUNTIMES {
        let delivered = matches!(
            runtime,
            DraftRuntime::Ssh | DraftRuntime::Host | DraftRuntime::Container
        );
        assert_eq!(
            Section::of(runtime).contains(&Section::StartScript),
            delivered,
            "{runtime:?}"
        );
        let hidden = Section::of_with(runtime, false);
        assert!(!hidden.contains(&Section::StartScript), "{runtime:?}");
        let shown = Section::of_with(runtime, true);
        assert_eq!(shown.last(), Some(&Section::StartScript), "{runtime:?}");
        assert_eq!(&shown[..shown.len() - 1], hidden, "{runtime:?}");
    }
}

#[test]
fn startup_renders_with_the_delivered_script_file() {
    let text = render(
        &with_startup(StartupSettings::new(StartupScript::Inline)),
        Kind::Host,
    );
    assert!(
        text.contains("\n[startup]\nscript_file = \"/data/startup.sh\"\n"),
        "{text}"
    );
    assert!(
        !text.contains("timeout_secs") && !text.contains("on_failure"),
        "{text}"
    );
    let config = parse(&text, "daemon.toml").unwrap();
    assert_eq!(
        config.startup.unwrap().script_file,
        std::path::PathBuf::from("/data/startup.sh")
    );
}

#[test]
fn every_key_renders_and_reads_back() {
    let settings = StartupSettings {
        script: StartupScript::File("/home/me/prepare.sh".into()),
        shell: Some("/bin/bash".into()),
        timeout_secs: Some(90),
        on_failure: OnFailure::Warn,
        run: RunPolicy::OncePerContainer,
        env_file: Some("/home/me/secrets.env".into()),
        export_env: false,
    };
    let text = prepare(&with_startup(settings), Kind::Host).expect("valid");
    let startup = parse(&text, "daemon.toml").unwrap().startup.unwrap();
    assert_eq!(startup.timeout_secs, Some(90));
    assert_eq!(startup.on_failure, Some(OnFailure::Warn));
    assert_eq!(startup.run, Some(RunPolicy::OncePerContainer));
    assert_eq!(startup.export_env, Some(false));
    assert_eq!(startup.shell, Some("/bin/bash".into()));
    assert_eq!(
        startup.env_file.unwrap().path(),
        std::path::Path::new("/home/me/secrets.env")
    );
    assert!(!text.contains("KEY="), "no secret value is in the file");
}

#[test]
fn a_timeout_outside_one_to_3600_is_refused_before_delivery() {
    for bad in [0, -5, 3601] {
        let mut s = StartupSettings::new(StartupScript::Inline);
        s.timeout_secs = Some(bad);
        let errors = prepare(&with_startup(s), Kind::Host).expect_err("refused");
        assert!(
            errors
                .iter()
                .any(|e| e.to_string().contains("timeout_secs")),
            "{bad}: {errors:?}"
        );
    }
    let mut ok = StartupSettings::new(StartupScript::Inline);
    ok.timeout_secs = Some(3600);
    assert!(prepare(&with_startup(ok), Kind::Host).is_ok());
}

#[test]
fn the_form_refuses_a_bad_timeout_and_an_empty_script_with_the_reason() {
    let mut start = StartDraft::default();
    assert_eq!(start.settings(), Ok(None), "no script, nothing to store");
    start.source = StartSource::File;
    assert_eq!(start.settings().unwrap_err().field, "startup_script");
    start.file = "/home/me/prepare.sh".into();
    for bad in ["0", "3601", "-1", "ten", "1.5"] {
        start.timeout = bad.into();
        let error = start.settings().unwrap_err();
        assert_eq!(error.field, "startup_timeout", "{bad}");
        assert!(error.message.contains("1 to 3600"), "{}", error.message);
    }
    start.timeout = " 300 ".into();
    let stored = start.settings().unwrap().unwrap();
    assert_eq!(stored.timeout_secs, Some(300));
    assert_eq!(
        stored.script,
        StartupScript::File("/home/me/prepare.sh".into())
    );
    start.source = StartSource::Inline;
    assert_eq!(start.settings().unwrap_err().field, "startup_script");
    start.set_text("echo hi\n".into());
    assert_eq!(
        start.settings().unwrap().unwrap().script,
        StartupScript::Inline
    );
}

#[test]
fn an_inline_script_is_a_file_in_the_app_data_and_the_entry_holds_a_marker() {
    let dir = tempfile::tempdir().unwrap();
    let mut registry = DaemonRegistry::new(Vec::new(), 1);
    let id = registry.add("Host", DaemonRuntime::Host, true).unwrap();
    let text = "echo prepared\nexport TOKEN_HINT=1\n";
    save_inline_script(dir.path(), id, text).unwrap();
    assert_eq!(
        std::fs::read_to_string(
            dir.path()
                .join("daemons")
                .join(id.0.to_string())
                .join("startup.sh")
        )
        .unwrap(),
        text
    );
    assert_eq!(
        inline_script_path(dir.path(), id).file_name().unwrap(),
        "startup.sh"
    );
    assert_eq!(load_inline_script(dir.path(), id), text);

    registry
        .set_file(
            id,
            FileSettings {
                startup: Some(StartupSettings::new(StartupScript::Inline)),
                ..FileSettings::default()
            },
        )
        .unwrap();
    let json = serde_json::to_string(registry.get(id).unwrap()).unwrap();
    assert!(json.contains(r#""script":"inline""#), "{json}");
    assert!(
        !json.contains("prepared") && !json.contains("TOKEN_HINT"),
        "{json}"
    );
}

#[test]
fn a_script_the_kind_does_not_deliver_is_left_out_of_the_file() {
    let mut registry = DaemonRegistry::new(Vec::new(), 1);
    let id = registry.add("Host", DaemonRuntime::Host, true).unwrap();
    registry
        .set_file(
            id,
            FileSettings {
                startup: Some(StartupSettings::new(StartupScript::File("/s.sh".into()))),
                ..FileSettings::default()
            },
        )
        .unwrap();
    let entry = registry.get(id).unwrap();
    let (none, _) = DaemonSettings::from_entry_delivering(entry, |_| false).unwrap();
    assert_eq!(none.startup, None, "no kind delivers");
    let (some, _) =
        DaemonSettings::from_entry_delivering(entry, |kind| kind == Kind::Host).unwrap();
    assert_eq!(
        some.startup.unwrap().script_file,
        std::path::PathBuf::from("/s.sh")
    );
}

#[test]
fn the_restart_notice_follows_the_startup_key() {
    let keys = |k: &[&str]| k.iter().map(ToString::to_string).collect::<Vec<_>>();
    assert!(startup_restart_pending(&keys(&[
        "daemon.data_dir",
        "startup"
    ])));
    assert!(startup_restart_pending(&keys(&["startup.script_file"])));
    assert!(!startup_restart_pending(&keys(&["daemon.data_dir"])));
    assert!(!startup_restart_pending(&[]));
}
