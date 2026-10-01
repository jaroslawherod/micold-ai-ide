//! Whether a CLI would first ask the user to trust a session's folder (feature 034, FR-017,
//! research R12; U234–U236, U223–U225).
//!
//! Claude Code and Copilot show a trust question in a folder they have not trusted yet, and their
//! output settles on it (quickstart §B3, `evidence/m3-real-cli.md` finding 4). A first prompt typed
//! then would answer that question, so the service reads each CLI's own trust record, read-only,
//! before it types anything. A trusted folder trusts every folder below it.

use std::path::Path;

use micold_core::mcp::binding::ConfigLocations;
use micold_core::mcp::trust::would_ask_trust;
use micold_core::provider::FolderTrust;
use micold_core::session::AiCli;

fn locations(home: &Path) -> ConfigLocations {
    ConfigLocations {
        home: Some(home.to_path_buf()),
        claude_config_dir: None,
        copilot_config_dir: Some(home.join(".copilot")),
    }
}

fn claude_record(home: &Path, project: &Path, accepted: bool) {
    let doc = serde_json::json!({
        "projects": { project.to_str().unwrap(): { "hasTrustDialogAccepted": accepted } }
    });
    std::fs::write(home.join(".claude.json"), doc.to_string()).unwrap();
}

fn copilot_record(home: &Path, body: &str) {
    std::fs::create_dir_all(home.join(".copilot")).unwrap();
    std::fs::write(home.join(".copilot/config.json"), body).unwrap();
}

/// U234: each CLI names the record it keeps; Pi never asks.
#[test]
fn each_cli_names_its_trust_record() {
    assert_eq!(
        AiCli::ClaudeCode.provider().folder_trust(),
        FolderTrust::ClaudeProjects
    );
    assert_eq!(
        AiCli::Copilot.provider().folder_trust(),
        FolderTrust::CopilotTrustedFolders
    );
    assert_eq!(AiCli::Pi.provider().folder_trust(), FolderTrust::NeverAsks);
}

/// U235: Claude trusts a project it accepted, and every worktree below it.
#[test]
fn claude_does_not_ask_below_an_accepted_project() {
    let home = tempfile::tempdir().unwrap();
    let project = tempfile::tempdir().unwrap();
    claude_record(home.path(), project.path(), true);
    let worktree = project.path().join(".claude/worktrees/feat-x");
    for cwd in [project.path(), worktree.as_path()] {
        assert!(
            !would_ask_trust(FolderTrust::ClaudeProjects, &locations(home.path()), cwd),
            "{}",
            cwd.display()
        );
    }
}

/// U236: Claude asks with no record, no file, or `hasTrustDialogAccepted: false`, and a sibling
/// whose name merely starts with the project's is not below it.
#[test]
fn claude_asks_without_an_accepted_record() {
    let home = tempfile::tempdir().unwrap();
    let project = tempfile::tempdir().unwrap();
    let at = locations(home.path());
    assert!(
        would_ask_trust(FolderTrust::ClaudeProjects, &at, project.path()),
        "no file"
    );
    claude_record(home.path(), project.path(), false);
    assert!(
        would_ask_trust(FolderTrust::ClaudeProjects, &at, project.path()),
        "false"
    );
    claude_record(home.path(), project.path(), true);
    let sibling = format!("{}-other", project.path().display());
    assert!(
        would_ask_trust(FolderTrust::ClaudeProjects, &at, Path::new(&sibling)),
        "a path prefix is not an ancestor"
    );
}

/// U223: with `CLAUDE_CONFIG_DIR` set, its `.claude.json` is the record, not the home one.
#[test]
fn claude_reads_the_record_under_its_config_dir() {
    let home = tempfile::tempdir().unwrap();
    let config = tempfile::tempdir().unwrap();
    let project = tempfile::tempdir().unwrap();
    claude_record(home.path(), project.path(), true);
    let at = ConfigLocations {
        claude_config_dir: Some(config.path().to_path_buf()),
        ..locations(home.path())
    };
    assert!(would_ask_trust(
        FolderTrust::ClaudeProjects,
        &at,
        project.path()
    ));
    claude_record(config.path(), project.path(), true);
    assert!(!would_ask_trust(
        FolderTrust::ClaudeProjects,
        &at,
        project.path()
    ));
}

/// U224: Copilot trusts a folder in `trustedFolders` and everything below it; its `config.json`
/// starts with `//` comment lines.
#[test]
fn copilot_does_not_ask_below_a_trusted_folder() {
    let home = tempfile::tempdir().unwrap();
    let project = tempfile::tempdir().unwrap();
    let at = locations(home.path());
    let worktree = project.path().join(".claude/worktrees/feat-x");
    assert!(
        would_ask_trust(FolderTrust::CopilotTrustedFolders, &at, &worktree),
        "no file"
    );
    copilot_record(
        home.path(),
        &format!(
            "// User settings belong in settings.json.\n// This file is managed automatically.\n{}",
            serde_json::json!({ "trustedFolders": ["/elsewhere", project.path()] })
        ),
    );
    assert!(!would_ask_trust(
        FolderTrust::CopilotTrustedFolders,
        &at,
        &worktree
    ));
    assert!(!would_ask_trust(
        FolderTrust::CopilotTrustedFolders,
        &at,
        project.path()
    ));
    let sibling = format!("{}-other", project.path().display());
    assert!(would_ask_trust(
        FolderTrust::CopilotTrustedFolders,
        &at,
        Path::new(&sibling)
    ));
}

/// U225: Pi has no trust question, so nothing is read and it never asks.
#[test]
fn pi_never_asks() {
    let at = ConfigLocations::default();
    assert!(!would_ask_trust(
        FolderTrust::NeverAsks,
        &at,
        Path::new("/nowhere")
    ));
}
