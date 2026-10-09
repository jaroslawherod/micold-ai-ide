//! `OpenCodeProvider` — the OpenCode profile of the AI CLI seam, fresh start only (feature 488, M1, T003).
//!
//! Every derivation is pure, so the provider is testable without the CLI installed.

use micold_core::provider::{
    ActivitySource, AiCliProvider, FolderTrust, InputReadiness, OpenCodeProvider, ToolServerSupport,
};
use micold_core::session::AiCli;
use micold_core::terminal::LaunchMode;
use std::path::Path;
use uuid::Uuid;

fn id() -> Uuid {
    Uuid::parse_str("44444444-5555-4555-8555-666666666666").unwrap()
}

#[test]
fn identity_is_opencode() {
    assert_eq!(OpenCodeProvider.command(), "opencode");
    assert_eq!(OpenCodeProvider.display_name(), "OpenCode");
    assert_eq!(OpenCodeProvider.id(), AiCli::OpenCode);
    assert_eq!(AiCli::OpenCode.tool_name(), "opencode");
    assert_eq!(AiCli::OpenCode.provider().command(), "opencode");
}

#[test]
fn a_fresh_start_passes_no_arguments() {
    assert!(OpenCodeProvider
        .launch_args(id(), LaunchMode::Fresh)
        .is_empty());
}

#[test]
fn availability_follows_the_path_it_is_given() {
    let dir = tempfile::tempdir().unwrap();
    let path = std::env::join_paths([dir.path()]).unwrap();
    assert!(!OpenCodeProvider.is_available(&path));
    std::fs::write(dir.path().join("opencode"), "").unwrap();
    assert!(OpenCodeProvider.is_available(&path));
    assert!(!OpenCodeProvider.is_available(std::ffi::OsStr::new("")));
}

#[test]
fn an_empty_store_contributes_nothing() {
    let cwd = Path::new("/work");
    let base = Path::new("/nowhere");
    assert!(OpenCodeProvider.recorded_session_ids(base, cwd).is_empty());
    assert!(!OpenCodeProvider.has_recorded_conversation(base, cwd, id()));
    assert_eq!(OpenCodeProvider.read_title(base, cwd, id()), None);
    assert_eq!(OpenCodeProvider.read_label(base, cwd, id()), None);
    assert_eq!(
        OpenCodeProvider.name_in_terminal_title("OpenCode", cwd),
        None
    );
}

#[test]
fn the_seam_answers_for_activity_tools_readiness_and_trust() {
    assert_eq!(
        OpenCodeProvider.activity_source(Path::new("/b"), Path::new("/w"), id()),
        ActivitySource::None
    );
    assert!(matches!(
        OpenCodeProvider.tool_server_support(),
        ToolServerSupport::Unsupported { .. }
    ));
    assert_eq!(
        OpenCodeProvider.input_readiness(),
        InputReadiness::OutputSettled
    );
    assert_eq!(OpenCodeProvider.folder_trust(), FolderTrust::NeverAsks);
    assert_eq!(
        OpenCodeProvider.launch_env(),
        vec![("OPENCODE_DISABLE_AUTOUPDATE".to_string(), "1".to_string())]
    );
}

#[test]
fn closing_a_session_leaves_a_marker_outside_the_cli_store() {
    let base = tempfile::tempdir().unwrap();
    let cwd = Path::new("/work");
    assert!(!OpenCodeProvider.is_archived(base.path(), cwd, id()));
    OpenCodeProvider
        .mark_archived(base.path(), cwd, id())
        .unwrap();
    assert!(OpenCodeProvider.is_archived(base.path(), cwd, id()));
    assert!(!OpenCodeProvider.is_archived(base.path(), cwd, Uuid::from_u128(7)));
}
