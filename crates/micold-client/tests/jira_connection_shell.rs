//! The connection's shell operations (feature 533, US3, contracts/jira-connection.md §4): the
//! token file is written only when a new token was typed, owner-only, and deleted on removal; the
//! probe is a plain existence check. Feature 1186 adds the remembered query file beside the token:
//! a read-modify-write of one field, kept unless the connection is removed (FR-013, FR-014).

#[path = "../src/shell/jira.rs"]
#[allow(dead_code)]
mod jira;

use jira::{
    apply, read_query, remember_failed, remember_query, remove_query, remove_token, token_stored,
    write_token,
};
use micold_core::jira::{ApiToken, ConnectionAction, JiraConnection};
use micold_core::jira_query::{QueryMode, RememberChange, RememberedQuery};
use micold_core::settings::JiraChange;
use micold_core::store::{jira_query_path, jira_token_path};
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

// ---- feature 1186, US3: the remembered mode and last run query (contracts/selector-source-and-storage.md §4) ----

fn query_file(dir: &Path) -> PathBuf {
    jira_query_path(dir, Path::new("/work/app"))
}

#[test]
fn a_write_changes_one_field_and_keeps_the_other() {
    let (_root, dir) = dir();
    let project = Path::new("/work/app");
    remember_query(
        &dir,
        project,
        &RememberChange::Query("priority = High".into()),
    )
    .unwrap();
    remember_query(&dir, project, &RememberChange::Mode(QueryMode::Advanced)).unwrap();
    assert_eq!(
        std::fs::read_to_string(query_file(&dir)).unwrap(),
        r#"{"mode":"advanced","query":"priority = High"}"#
    );
    // Another window's write to the other field stands: each write reads the file first.
    std::fs::write(
        query_file(&dir),
        r#"{"mode":"advanced","query":"status = Done"}"#,
    )
    .unwrap();
    remember_query(&dir, project, &RememberChange::Mode(QueryMode::Basic)).unwrap();
    let read = read_query(&dir, project);
    assert_eq!(read.mode, QueryMode::Basic);
    assert_eq!(read.query, "status = Done");
}

#[test]
fn a_missing_or_unreadable_file_reads_as_basic_and_empty() {
    let (_root, dir) = dir();
    let project = Path::new("/work/app");
    assert_eq!(read_query(&dir, project), RememberedQuery::default());
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(query_file(&dir), b"{ not json").unwrap();
    assert_eq!(read_query(&dir, project), RememberedQuery::default());
    // A write over an unreadable file starts from the default.
    remember_query(&dir, project, &RememberChange::Mode(QueryMode::Advanced)).unwrap();
    assert_eq!(
        read_query(&dir, project),
        RememberedQuery::default().with_mode(QueryMode::Advanced)
    );
}

#[cfg(unix)]
#[test]
fn the_remembered_file_is_owner_only_beside_the_token() {
    use std::os::unix::fs::PermissionsExt;
    let (_root, dir) = dir();
    remember_query(
        &dir,
        Path::new("/work/app"),
        &RememberChange::Query("priority = High".into()),
    )
    .unwrap();
    let mode = |p: &Path| std::fs::metadata(p).unwrap().permissions().mode() & 0o777;
    assert_eq!(mode(&dir), 0o700);
    assert_eq!(mode(&query_file(&dir)), 0o600);
}

#[test]
fn removing_the_connection_deletes_the_remembered_file_after_the_token() {
    let (_root, dir) = dir();
    let project = Path::new("/work/app");
    write_token(&dir, project, &ApiToken::parse(SENTINEL).unwrap()).unwrap();
    remember_query(
        &dir,
        project,
        &RememberChange::Query("priority = High".into()),
    )
    .unwrap();
    let remove = ConnectionAction::Remove {
        project: project.to_path_buf(),
    };
    apply(Some(&dir), &remove).unwrap().unwrap();
    assert!(!token_stored(&dir, project));
    assert!(!query_file(&dir).exists());
    // Already gone, or never written: still a removal.
    assert!(apply(Some(&dir), &remove).unwrap().is_some());
    remove_query(&dir, project).unwrap();
}

#[test]
fn a_failed_query_delete_reports_after_the_token_is_gone() {
    let (_root, dir) = dir();
    let project = Path::new("/work/app");
    write_token(&dir, project, &ApiToken::parse(SENTINEL).unwrap()).unwrap();
    // A directory where the file should be cannot be removed as a file.
    std::fs::create_dir_all(query_file(&dir)).unwrap();
    std::fs::write(query_file(&dir).join("x"), b"x").unwrap();
    let error = apply(
        Some(&dir),
        &ConnectionAction::Remove {
            project: project.to_path_buf(),
        },
    )
    .unwrap_err();
    assert!(!token_stored(&dir, project), "the token went first");
    assert!(error.contains("remembered Jira query"), "{error}");
}

#[test]
fn keeping_or_saving_the_connection_keeps_the_remembered_file() {
    let (_root, dir) = dir();
    let project = Path::new("/work/app");
    remember_query(
        &dir,
        project,
        &RememberChange::Query("priority = High".into()),
    )
    .unwrap();
    // Turning the integration off answers Keep (FR-014); a save changes the connection only.
    apply(Some(&dir), &ConnectionAction::Keep).unwrap();
    apply(Some(&dir), &save(Some(SENTINEL))).unwrap();
    apply(Some(&dir), &save(None)).unwrap();
    assert_eq!(read_query(&dir, project).query, "priority = High");
}

#[test]
fn a_failed_write_names_the_os_error_and_never_the_query() {
    let root = tempfile::tempdir().unwrap();
    let blocker = root.path().join("jira");
    std::fs::write(&blocker, b"a file where the directory should be").unwrap();
    let error = remember_query(
        &blocker,
        Path::new("/work/app"),
        &RememberChange::Query("summary ~ \"secret plan\"".into()),
    )
    .unwrap_err();
    let text = remember_failed(&error);
    assert!(
        text.starts_with("Couldn't remember the Jira query: "),
        "{text}"
    );
    assert!(!text.contains("secret plan"), "{text}");
}
