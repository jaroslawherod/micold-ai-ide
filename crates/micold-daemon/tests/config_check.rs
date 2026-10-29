//! `micold-daemon config check` (#1523, contracts/daemon-config.md §4): exit 0 with the resolved
//! view, exit 2 with `file:line:column: message` per error, never a secret's content.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

const DAEMON_BIN: &str = env!("CARGO_BIN_EXE_micold-daemon");

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../micold-core/tests/fixtures/daemon_config")
        .join(name)
}

fn check(args: &[&str]) -> Output {
    Command::new(DAEMON_BIN)
        .arg("config")
        .arg("check")
        .args(args)
        .output()
        .unwrap()
}

fn text(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

#[test]
fn a_valid_file_exits_zero_and_prints_the_resolved_view() {
    let expected = [
        (
            "host.toml",
            ["daemon: This computer (host)", "listen: socket auto"],
        ),
        (
            "ssh.toml",
            [
                "daemon: build-box (ssh)",
                "storage.clone_root: /home/jaro/src",
            ],
        ),
        (
            "container.toml",
            [
                "daemon: podman-sandbox (container)",
                "session.env: RUST_LOG=info",
            ],
        ),
        (
            "kubernetes.toml",
            [
                "daemon: k8s-dev (kubernetes)",
                "storage.clone_root: /data/repos",
            ],
        ),
    ];
    for (name, lines) in expected {
        let file = fixture(name);
        let output = check(&[file.to_str().unwrap()]);
        assert_eq!(
            output.status.code(),
            Some(0),
            "{name}: {}",
            text(&output.stderr)
        );
        let stdout = text(&output.stdout);
        assert!(stdout.contains("schema: 1"), "{name}: {stdout}");
        for line in lines {
            assert!(
                stdout.contains(line),
                "{name}: missing `{line}` in {stdout}"
            );
        }
    }
}

#[test]
fn a_warning_only_file_exits_zero_and_prints_the_resolved_view() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("daemon.toml");
    std::fs::write(
        &file,
        "schema = 1\n[daemon]\nname = \"t\"\nkind = \"host\"\n\
         [[ai.cli]]\nid = \"pi\"\nversion = \"1.2.3\"\n",
    )
    .unwrap();
    let output = check(&[file.to_str().unwrap()]);
    let stderr = text(&output.stderr);
    assert!(stderr.contains("warning: "), "{stderr}");
    assert_eq!(output.status.code(), Some(0), "{stderr}");
    assert!(text(&output.stdout).contains("daemon: t (host)"));
}

#[test]
fn the_resolved_view_fills_defaults_and_marks_missing_secrets() {
    let file = fixture("container.toml");
    let output = check(&[file.to_str().unwrap()]);
    let stdout = text(&output.stdout);
    assert!(stdout.contains("idle_stop: default"), "{stdout}");
    assert!(stdout.contains("(missing)"), "{stdout}");
    assert!(
        stdout.contains("github.token acme-org: host github.com"),
        "{stdout}"
    );
}

#[test]
fn an_invalid_file_exits_two_with_a_positioned_error_and_prints_no_view() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("daemon.toml");
    std::fs::write(
        &file,
        "schema = 1\n[daemon]\nname = \"t\"\nkind = \"host\"\nlistne = 1\n",
    )
    .unwrap();
    let output = check(&[file.to_str().unwrap()]);
    assert_eq!(output.status.code(), Some(2));
    let stderr = text(&output.stderr);
    assert!(
        stderr.contains(&format!(
            "{}:5:1: unknown key \"listne\" in [daemon]",
            file.display()
        )),
        "{stderr}"
    );
    assert!(text(&output.stdout).is_empty());
}

#[test]
fn every_semantic_error_is_printed() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("daemon.toml");
    std::fs::write(
        &file,
        "schema = 1\n[daemon]\nname = \"t\"\nkind = \"host\"\n[session.env]\nPATH = \"x\"\n\
         [[projects]]\npath = \"rel\"\n",
    )
    .unwrap();
    let output = check(&[file.to_str().unwrap()]);
    assert_eq!(output.status.code(), Some(2));
    let stderr = text(&output.stderr);
    assert!(stderr.contains("`PATH` is reserved"), "{stderr}");
    assert!(stderr.contains("must be an absolute path"), "{stderr}");
}

#[test]
fn a_newer_schema_exits_two_naming_both_versions() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("daemon.toml");
    std::fs::write(&file, "schema = 7\n").unwrap();
    let output = check(&[file.to_str().unwrap()]);
    assert_eq!(output.status.code(), Some(2));
    let stderr = text(&output.stderr);
    assert!(
        stderr.contains("schema 7") && stderr.contains("schema 1"),
        "{stderr}"
    );
}

#[test]
fn a_kind_that_differs_from_the_file_is_an_error() {
    let file = fixture("host.toml");
    let output = check(&[file.to_str().unwrap(), "--kind", "container"]);
    assert_eq!(output.status.code(), Some(2));
    assert!(text(&output.stderr).contains("daemon.kind"));
    let output = check(&[file.to_str().unwrap(), "--kind", "host"]);
    assert_eq!(output.status.code(), Some(0));
}

#[test]
fn files_flag_refuses_a_missing_required_file() {
    let file = fixture("host.toml");
    let output = check(&[file.to_str().unwrap(), "--files"]);
    assert_eq!(output.status.code(), Some(2));
    assert!(text(&output.stderr).contains("startup.sh does not exist"));
}

#[test]
fn a_missing_config_file_exits_two() {
    let output = check(&["/nonexistent/daemon.toml"]);
    assert_eq!(output.status.code(), Some(2));
    assert!(text(&output.stderr).contains("cannot read"));
}

#[cfg(unix)]
#[test]
fn a_secrets_content_is_never_printed() {
    use std::os::unix::fs::PermissionsExt;
    let dir = tempfile::tempdir().unwrap();
    let secret = dir.path().join("tok");
    std::fs::write(&secret, "s3cr3t-value\n").unwrap();
    std::fs::set_permissions(&secret, std::fs::Permissions::from_mode(0o600)).unwrap();
    let file = dir.path().join("daemon.toml");
    std::fs::write(
        &file,
        format!(
            "schema = 1\n[daemon]\nname = \"t\"\nkind = \"host\"\n\
             [[github.tokens]]\nname = \"a\"\ntoken_file = \"{}\"\n",
            secret.display()
        ),
    )
    .unwrap();
    let output = check(&[file.to_str().unwrap(), "--files"]);
    assert_eq!(output.status.code(), Some(0), "{}", text(&output.stderr));
    let stdout = text(&output.stdout);
    assert!(
        stdout.contains(&format!("{} (readable, 0600)", secret.display())),
        "{stdout}"
    );
    assert!(!stdout.contains("s3cr3t-value") && !text(&output.stderr).contains("s3cr3t-value"));
}

#[test]
fn a_bad_command_prints_usage_and_exits_two() {
    let output = Command::new(DAEMON_BIN)
        .args(["config", "nope"])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(2));
    assert!(text(&output.stderr).contains("usage:"));
}

#[test]
fn bad_arguments_print_a_usage_message_and_exit_two() {
    let file = fixture("host.toml");
    let file = file.to_str().unwrap();
    let cases: [&[&str]; 4] = [
        &[file, "--kind", "bogus"],
        &[file, "--kind"],
        &[file, "--bogus"],
        &[file, file],
    ];
    for args in cases {
        let output = check(args);
        let stderr = text(&output.stderr);
        assert_eq!(output.status.code(), Some(2), "{args:?}: {stderr}");
        assert!(stderr.contains("kind"), "{args:?}: {stderr}");
        assert!(text(&output.stdout).is_empty(), "{args:?}");
    }
}
