//! The user's own status line, resolved so the relay can keep it (feature 490, data-model
//! *UserStatusLine*, *RelayConfig*; research R4; FR-013, FR-020).

use std::path::{Path, PathBuf};

use micold_core::mcp::binding::ConfigLocations;
use micold_core::plan_usage::user_status_line::{RelayConfig, UserStatusLine};

struct Fixture {
    _root: tempfile::TempDir,
    cwd: PathBuf,
    home: PathBuf,
}

fn fixture() -> Fixture {
    let root = tempfile::tempdir().unwrap();
    let cwd = root.path().join("project");
    let home = root.path().join("home");
    std::fs::create_dir_all(cwd.join(".claude")).unwrap();
    std::fs::create_dir_all(home.join(".claude")).unwrap();
    Fixture {
        _root: root,
        cwd,
        home,
    }
}

fn locations(home: &Path, claude_config_dir: Option<PathBuf>) -> ConfigLocations {
    ConfigLocations {
        home: Some(home.to_path_buf()),
        claude_config_dir,
        copilot_config_dir: None,
        codex_home: None,
    }
}

fn write(path: &Path, body: &str) {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, body).unwrap();
}

fn line(command: &str) -> String {
    format!(r#"{{"statusLine": {{"type": "command", "command": "{command}"}}}}"#)
}

fn resolved(f: &Fixture, claude_config_dir: Option<PathBuf>) -> Option<UserStatusLine> {
    UserStatusLine::resolve(&locations(&f.home, claude_config_dir), &f.cwd)
}

#[test]
fn the_local_project_file_wins_over_the_shared_one_and_the_user_one() {
    let f = fixture();
    write(&f.cwd.join(".claude/settings.local.json"), &line("local"));
    write(&f.cwd.join(".claude/settings.json"), &line("project"));
    write(&f.home.join(".claude/settings.json"), &line("user"));
    assert_eq!(resolved(&f, None).unwrap().command, "local");
}

#[test]
fn the_shared_project_file_wins_over_the_user_one() {
    let f = fixture();
    write(&f.cwd.join(".claude/settings.json"), &line("project"));
    write(&f.home.join(".claude/settings.json"), &line("user"));
    assert_eq!(resolved(&f, None).unwrap().command, "project");
}

#[test]
fn the_user_file_is_read_from_home_dot_claude() {
    let f = fixture();
    write(&f.home.join(".claude/settings.json"), &line("user"));
    assert_eq!(resolved(&f, None).unwrap().command, "user");
}

#[test]
fn claude_config_dir_replaces_home_dot_claude() {
    let f = fixture();
    let config = f.home.join("elsewhere");
    write(&f.home.join(".claude/settings.json"), &line("home"));
    write(&config.join("settings.json"), &line("config-dir"));
    assert_eq!(resolved(&f, Some(config)).unwrap().command, "config-dir");
}

#[test]
fn only_a_command_status_line_with_a_command_counts() {
    let f = fixture();
    write(
        &f.cwd.join(".claude/settings.local.json"),
        r#"{"statusLine": {"type": "static", "command": "nope"}}"#,
    );
    write(
        &f.cwd.join(".claude/settings.json"),
        r#"{"statusLine": {"type": "command", "command": ""}}"#,
    );
    write(&f.home.join(".claude/settings.json"), &line("user"));
    assert_eq!(resolved(&f, None).unwrap().command, "user");
}

#[test]
fn padding_and_refresh_interval_are_carried() {
    let f = fixture();
    write(
        &f.cwd.join(".claude/settings.json"),
        r#"{"statusLine": {"type": "command", "command": "x", "padding": 2, "refreshInterval": 5}}"#,
    );
    assert_eq!(
        resolved(&f, None),
        Some(UserStatusLine {
            command: "x".to_owned(),
            padding: Some(2),
            refresh_interval: Some(5),
        })
    );
}

#[test]
fn absent_unreadable_and_malformed_files_fall_through() {
    let f = fixture();
    write(&f.cwd.join(".claude/settings.local.json"), "{ not json");
    // A directory where a file is expected cannot be read as one.
    std::fs::create_dir_all(f.cwd.join(".claude/settings.json")).unwrap();
    write(&f.home.join(".claude/settings.json"), &line("user"));
    assert_eq!(resolved(&f, None).unwrap().command, "user");
}

#[test]
fn nothing_anywhere_names_nothing() {
    let f = fixture();
    write(&f.cwd.join(".claude/settings.json"), r#"{"model": "x"}"#);
    assert_eq!(resolved(&f, None), None);
}

#[test]
fn other_keys_of_unexpected_types_do_not_hide_the_status_line() {
    let f = fixture();
    write(
        &f.cwd.join(".claude/settings.json"),
        r#"{"env": {"A": 1, "B": [true]}, "permissions": 7, "statusLine": {"type": "command", "command": "kept"}}"#,
    );
    assert_eq!(resolved(&f, None).unwrap().command, "kept");
}

#[test]
fn relay_config_round_trips_through_serde() {
    let config = RelayConfig {
        url: "http://127.0.0.1:4242/status/0b6e4a62-6d0b-4a8e-9a43-5d2c9f0b1a77".to_owned(),
        token: "secret".to_owned(),
        user_command: Some("~/bin/line.sh".to_owned()),
    };
    let text = serde_json::to_string(&config).unwrap();
    assert_eq!(serde_json::from_str::<RelayConfig>(&text).unwrap(), config);
    let none = RelayConfig {
        user_command: None,
        ..config
    };
    let text = serde_json::to_string(&none).unwrap();
    assert_eq!(serde_json::from_str::<RelayConfig>(&text).unwrap(), none);
}

#[test]
fn claude_settings_dir_is_the_config_dir_when_set_else_home_dot_claude() {
    let home = PathBuf::from("/h");
    assert_eq!(
        locations(&home, None).claude_settings_dir(),
        Some(PathBuf::from("/h/.claude"))
    );
    assert_eq!(
        locations(&home, Some(PathBuf::from("/c"))).claude_settings_dir(),
        Some(PathBuf::from("/c"))
    );
    let nowhere = ConfigLocations {
        home: None,
        claude_config_dir: None,
        copilot_config_dir: None,
        codex_home: None,
    };
    assert_eq!(nowhere.claude_settings_dir(), None);
}

#[test]
fn an_odd_optional_value_does_not_cost_the_command() {
    let f = fixture();
    write(
        &f.cwd.join(".claude/settings.json"),
        r#"{"statusLine": {"type": "command", "command": "x", "padding": -1, "refreshInterval": 2.5}}"#,
    );
    assert_eq!(
        resolved(&f, None),
        Some(UserStatusLine {
            command: "x".to_owned(),
            padding: None,
            refresh_interval: None,
        })
    );
}
