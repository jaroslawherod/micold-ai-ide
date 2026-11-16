//! `config check` on files with and without `[database]` (#1948, US4): it passes or fails naming
//! the key and the path, and never prints the URL (FR-008).

use std::ffi::OsString;
use std::path::Path;

use micold_daemon::config_check;

const HEAD: &str =
    "schema = 1\n[daemon]\nname = \"t\"\nkind = \"host\"\n[listen]\nsocket = \"auto\"\n";

/// Runs `config check <file> --files` on `HEAD` + `body`: the status, stdout and stderr.
fn check(dir: &Path, body: &str) -> (i32, String, String) {
    let file = dir.join("daemon.toml");
    std::fs::write(&file, format!("{HEAD}{body}")).unwrap();
    let args: Vec<OsString> = vec!["check".into(), file.into(), "--files".into()];
    let (mut out, mut err) = (Vec::new(), Vec::new());
    let status = config_check::run(&args, &mut out, &mut err);
    (
        status,
        String::from_utf8_lossy(&out).into_owned(),
        String::from_utf8_lossy(&err).into_owned(),
    )
}

fn url_file(dir: &Path, content: Option<&str>) -> String {
    let path = dir.join("db-url");
    if let Some(content) = content {
        std::fs::write(&path, content).unwrap();
        std::fs::set_permissions(&path, std::os::unix::fs::PermissionsExt::from_mode(0o600))
            .unwrap();
    }
    path.display().to_string()
}

#[test]
fn no_database_table_passes_and_prints_nothing_about_it() {
    let dir = tempfile::tempdir().unwrap();
    let (status, out, err) = check(dir.path(), "");
    assert_eq!((status, err.as_str()), (0, ""));
    assert!(!out.contains("database"), "{out}");
}

#[test]
fn a_readable_url_file_passes_and_the_view_never_shows_the_url() {
    let dir = tempfile::tempdir().unwrap();
    let path = url_file(
        dir.path(),
        Some("postgres://user:hunter2@db.internal/orch\n"),
    );
    let (status, out, err) = check(dir.path(), &format!("[database]\nurl_file = \"{path}\"\n"));
    assert_eq!((status, err.as_str()), (0, ""));
    assert!(
        out.contains(&format!("database.url_file = {path} (readable")),
        "{out}"
    );
    assert!(out.contains("database.pool_size = 5"), "{out}");
    for secret in ["hunter2", "db.internal", "user:"] {
        assert!(!out.contains(secret), "{out}");
    }
}

#[test]
fn a_missing_empty_or_foreign_url_file_fails_naming_key_and_path() {
    let dir = tempfile::tempdir().unwrap();
    let path = url_file(dir.path(), None);
    let body = format!("[database]\nurl_file = \"{path}\"\n");
    let (status, _, err) = check(dir.path(), &body);
    assert_eq!(status, 2);
    assert!(
        err.contains(&format!("database.url_file {path}: missing")),
        "{err}"
    );

    url_file(dir.path(), Some(" \n"));
    let (status, _, err) = check(dir.path(), &body);
    assert_eq!(status, 2);
    assert!(
        err.contains(&format!("database.url_file {path}: empty")),
        "{err}"
    );

    url_file(dir.path(), Some("mysql://u:hunter2@db.internal/x"));
    let (status, _, err) = check(dir.path(), &body);
    assert_eq!(status, 2);
    assert!(err.contains("\"mysql\""), "{err}");
    assert!(
        !err.contains("hunter2") && !err.contains("db.internal"),
        "{err}"
    );
}

#[test]
fn a_missing_ca_file_or_url_key_fails_naming_the_key() {
    let dir = tempfile::tempdir().unwrap();
    let path = url_file(dir.path(), Some("postgres://h/d"));
    let body = format!("[database]\nurl_file = \"{path}\"\nca_file = \"/nonexistent/ca.pem\"\n");
    let (status, _, err) = check(dir.path(), &body);
    assert_eq!(status, 2);
    assert!(
        err.contains("database.ca_file /nonexistent/ca.pem: missing"),
        "{err}"
    );

    let (status, _, err) = check(dir.path(), "[database]\npool_size = 3\n");
    assert_eq!(status, 2);
    assert!(err.contains("database.url_file"), "{err}");
}
