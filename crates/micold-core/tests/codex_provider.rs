//! `CodexProvider` — the Codex profile of the AI CLI seam, fresh start only (feature 488, M1, T003).
//!
//! Every derivation is pure, so the provider is testable without the CLI installed.

use micold_core::provider::{
    ActivitySource, AiCliProvider, CodexProvider, FolderTrust, InputReadiness, ToolServerSupport,
};
use micold_core::session::AiCli;
use micold_core::terminal::LaunchMode;
use std::path::Path;
use uuid::Uuid;

fn id() -> Uuid {
    Uuid::parse_str("44444444-5555-4555-8555-666666666666").unwrap()
}

#[test]
fn identity_is_codex() {
    assert_eq!(CodexProvider.command(), "codex");
    assert_eq!(CodexProvider.display_name(), "Codex");
    assert_eq!(CodexProvider.id(), AiCli::Codex);
    assert_eq!(AiCli::Codex.tool_name(), "codex");
    assert_eq!(AiCli::Codex.provider().command(), "codex");
}

#[test]
fn a_fresh_start_passes_no_arguments() {
    assert!(CodexProvider
        .launch_args(id(), LaunchMode::Fresh)
        .is_empty());
}

#[test]
fn availability_follows_the_path_it_is_given() {
    let dir = tempfile::tempdir().unwrap();
    let path = std::env::join_paths([dir.path()]).unwrap();
    assert!(!CodexProvider.is_available(&path));
    std::fs::write(dir.path().join("codex"), "").unwrap();
    assert!(CodexProvider.is_available(&path));
    assert!(!CodexProvider.is_available(std::ffi::OsStr::new("")));
}

#[test]
fn an_empty_store_contributes_nothing() {
    let cwd = Path::new("/work");
    let base = Path::new("/nowhere");
    assert!(CodexProvider.recorded_session_ids(base, cwd).is_empty());
    assert!(!CodexProvider.has_recorded_conversation(base, cwd, id()));
    assert_eq!(CodexProvider.read_title(base, cwd, id()), None);
    assert_eq!(CodexProvider.read_label(base, cwd, id()), None);
    assert_eq!(CodexProvider.name_in_terminal_title("Codex", cwd), None);
}

#[test]
fn the_seam_answers_for_activity_tools_readiness_and_trust() {
    assert_eq!(
        CodexProvider.activity_source(Path::new("/b"), Path::new("/w"), id()),
        ActivitySource::None
    );
    assert!(matches!(
        CodexProvider.tool_server_support(),
        ToolServerSupport::Unsupported { .. }
    ));
    assert_eq!(
        CodexProvider.input_readiness(),
        InputReadiness::OutputSettled
    );
    assert_eq!(CodexProvider.folder_trust(), FolderTrust::CodexProjects);
    assert_eq!(CodexProvider.launch_env(), Vec::<(String, String)>::new());
}

#[test]
fn closing_a_session_leaves_a_marker_outside_the_cli_store() {
    let base = tempfile::tempdir().unwrap();
    let cwd = Path::new("/work");
    assert!(!CodexProvider.is_archived(base.path(), cwd, id()));
    CodexProvider.mark_archived(base.path(), cwd, id()).unwrap();
    assert!(CodexProvider.is_archived(base.path(), cwd, id()));
    assert!(!CodexProvider.is_archived(base.path(), cwd, Uuid::from_u128(7)));
}
