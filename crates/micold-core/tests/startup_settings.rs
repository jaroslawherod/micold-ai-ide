//! The start script's settings on a daemon entry (#1533, FR-010, US5): serde defaults match the
//! schema, an inline script is a marker and never text, and no kind delivers a script yet.

use micold_core::daemon_config::{Kind, OnFailure, RunPolicy};
use micold_core::daemons::{start_script_delivered, FileSettings, StartupScript, StartupSettings};

#[test]
fn the_serde_defaults_match_the_schema() {
    let s: StartupSettings = serde_json::from_str(r#"{"script":"inline"}"#).unwrap();
    assert_eq!(s.script, StartupScript::Inline);
    assert_eq!(s.timeout_secs, None);
    assert_eq!(s.on_failure, OnFailure::Refuse);
    assert_eq!(s.run, RunPolicy::EveryStart);
    assert!(s.export_env);
    assert_eq!(s.shell, None);
    assert_eq!(s.env_file, None);
    assert_eq!(s, StartupSettings::new(StartupScript::Inline));
}

#[test]
fn an_unknown_policy_is_refused_with_its_name() {
    for text in [
        r#"{"script":"inline","on_failure":"ignore"}"#,
        r#"{"script":"inline","run":"sometimes"}"#,
    ] {
        let err = serde_json::from_str::<StartupSettings>(text).unwrap_err();
        assert!(err.to_string().contains("unknown variant"), "{err}");
    }
}

#[test]
fn the_entry_holds_the_inline_marker_and_no_text() {
    let file = FileSettings {
        startup: Some(StartupSettings::new(StartupScript::Inline)),
        ..FileSettings::default()
    };
    let json = serde_json::to_string(&file).unwrap();
    assert!(json.contains(r#""script":"inline""#), "{json}");
    let back: FileSettings = serde_json::from_str(&json).unwrap();
    assert_eq!(back, file);
    assert!(!FileSettings::default().is_empty() || json.contains("startup"));
    assert_eq!(
        serde_json::to_string(&FileSettings::default()).unwrap(),
        "{}",
        "an entry without a script keeps its file unchanged"
    );
}

#[test]
fn no_kind_delivers_a_script_yet() {
    for kind in [Kind::Host, Kind::Ssh, Kind::Container, Kind::Kubernetes] {
        assert!(!start_script_delivered(kind), "{kind:?}");
    }
}
