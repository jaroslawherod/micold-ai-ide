//! The connection's shell operations (feature 533, US3, contracts/jira-connection.md §4): the
//! token file is written only when a new token was typed, owner-only, and deleted on removal; the
//! probe is a plain existence check.

#[path = "../src/shell/jira.rs"]
#[allow(dead_code)]
mod jira;

use jira::{apply, remove_token, token_stored, write_token};
use micold_core::jira::{ApiToken, ConnectionAction, JiraConnection};
use micold_core::settings::JiraChange;
use micold_core::store::jira_token_path;
use std::path::{Path, PathBuf};

const SENTINEL: &str = "sentinel-token-value-9f3a";

fn connection() -> JiraConnection {
    JiraConnection::parse("https://acme.atlassian.net", "ABC", "me@acme.com").unwrap()
}

fn save(token: Option<&str>) -> ConnectionAction {
    ConnectionAction::Save {
        project: PathBuf::from("/work/app"),
        connection: connection(),
        token: token.map(|t| ApiToken::parse(t).unwrap()),
    }
}

fn dir() -> (tempfile::TempDir, PathBuf) {
    let root = tempfile::tempdir().unwrap();
    let dir = root.path().join("jira");
    (root, dir)
}

#[test]
fn saving_with_a_new_token_writes_the_file_then_returns_the_upsert() {
    let (_root, dir) = dir();
    let change = apply(Some(&dir), &save(Some(SENTINEL))).unwrap().unwrap();
    assert_eq!(change.project, "/work/app");
    assert_eq!(change.change, JiraChange::Upsert(connection()));
    let file = jira_token_path(&dir, Path::new("/work/app"));
    assert_eq!(std::fs::read_to_string(file).unwrap(), SENTINEL);
    assert!(token_stored(&dir, Path::new("/work/app")));
}

#[test]
fn saving_without_a_new_token_leaves_the_file_alone() {
    let (_root, dir) = dir();
    let project = Path::new("/work/app");
    write_token(&dir, project, &ApiToken::parse("old").unwrap()).unwrap();
    apply(Some(&dir), &save(None)).unwrap().unwrap();
    let file = jira_token_path(&dir, project);
    assert_eq!(std::fs::read_to_string(file).unwrap(), "old");

    let (_root2, empty) = self::dir();
    apply(Some(&empty), &save(None)).unwrap().unwrap();
    assert!(!empty.exists(), "no token typed, nothing written");
}

#[test]
fn removing_deletes_the_file_and_tolerates_a_missing_one() {
    let (_root, dir) = dir();
    let project = Path::new("/work/app");
    write_token(&dir, project, &ApiToken::parse(SENTINEL).unwrap()).unwrap();
    let remove = ConnectionAction::Remove {
        project: project.to_path_buf(),
    };
    let change = apply(Some(&dir), &remove).unwrap().unwrap();
    assert_eq!(change.change, JiraChange::Remove);
    assert!(!token_stored(&dir, project));
    // Already gone: still a removal.
    assert!(apply(Some(&dir), &remove).unwrap().is_some());
    remove_token(&dir, project).unwrap();
}

#[test]
fn keep_does_nothing() {
    let (_root, dir) = dir();
    assert_eq!(apply(Some(&dir), &ConnectionAction::Keep), Ok(None));
    assert!(!dir.exists());
}

#[test]
fn a_failed_write_reports_without_the_token() {
    let root = tempfile::tempdir().unwrap();
    let blocker = root.path().join("jira");
    std::fs::write(&blocker, b"a file where the directory should be").unwrap();
    let error = apply(Some(&blocker), &save(Some(SENTINEL))).unwrap_err();
    assert!(!error.contains(SENTINEL), "{error}");
}

#[cfg(unix)]
#[test]
fn the_token_file_is_owner_only_in_an_owner_only_directory() {
    use std::os::unix::fs::PermissionsExt;
    let (_root, dir) = dir();
    let project = Path::new("/work/app");
    write_token(&dir, project, &ApiToken::parse(SENTINEL).unwrap()).unwrap();
    let mode = |p: &Path| std::fs::metadata(p).unwrap().permissions().mode() & 0o777;
    assert_eq!(mode(&dir), 0o700);
    assert_eq!(mode(&jira_token_path(&dir, project)), 0o600);
}
