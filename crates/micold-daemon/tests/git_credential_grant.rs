//! #1002 M4 (contracts/protocol.md): a sandboxed daemon reports where its projects push and
//! accepts scoped credentials for exactly those repositories, writing them to its in-memory
//! credential root.

#[path = "support/conn.rs"]
mod conn;
#[path = "support/env.rs"]
mod env_support;

use std::ffi::OsString;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Arc;
use std::time::Duration;

use conn::{connect_and_attach, Client};
use env_support::Env;
use futures_util::{SinkExt, StreamExt};
use micold_core::github::GithubRepo;
use micold_core::protocol::codec::Frame;
use micold_core::protocol::messages::{ClientMsg, CredentialTarget, DaemonMsg, ErrorKind};
use micold_core::scoped_credential::{SecretToken, CLOCK_SKEW_SECS, MAX_LIFETIME_SECS};
use micold_daemon::catalog::Catalog;
use micold_daemon::state::DaemonState;

/// The environment is the process's: one test at a time.
static ONE_AT_A_TIME: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

const ROOT_ENV: &str = "MICOLD_GIT_CREDENTIAL_ROOT";

/// An expiry the daemon accepts, fixed for the whole test binary so a file's text can be compared.
fn soon() -> u64 {
    static AT: std::sync::OnceLock<u64> = std::sync::OnceLock::new();
    *AT.get_or_init(|| {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs()
            + 1800
    })
}

fn repo(url: &str) -> GithubRepo {
    GithubRepo::from_remote_url(url).unwrap()
}

fn project(dir: &Path, name: &str, origin: Option<&str>) -> PathBuf {
    let path = dir.join(name);
    std::fs::create_dir_all(&path).unwrap();
    let git = |args: &[&str]| {
        assert!(Command::new("git")
            .arg("-C")
            .arg(&path)
            .args(args)
            .status()
            .unwrap()
            .success());
    };
    git(&["init", "-q", "-b", "main"]);
    if let Some(url) = origin {
        git(&["remote", "add", "origin", url]);
    }
    path
}

fn state_with(projects: &[&Path]) -> Arc<DaemonState> {
    let state = Arc::new(DaemonState::new(Catalog::ephemeral()));
    for p in projects {
        state.add_project(p).unwrap();
    }
    state
}

async fn send(client: &mut Client, msg: ClientMsg) {
    client.send(Frame::Control(msg)).await.unwrap();
}

/// The next message matching `pick`, within 20 s.
async fn next<T>(client: &mut Client, pick: impl Fn(DaemonMsg) -> Option<T>) -> T {
    tokio::time::timeout(Duration::from_secs(20), async {
        loop {
            if let Frame::Control(m) = client.next().await.expect("open").expect("frame") {
                if let Some(found) = pick(m) {
                    return found;
                }
            }
        }
    })
    .await
    .expect("the message arrives")
}

async fn targets(client: &mut Client) -> (Vec<CredentialTarget>, Option<String>) {
    next(client, |m| match m {
        DaemonMsg::GitCredentialTargets {
            targets,
            git_too_old,
        } => Some((targets, git_too_old)),
        _ => None,
    })
    .await
}

async fn refusal(client: &mut Client) -> (ErrorKind, String) {
    next(client, |m| match m {
        DaemonMsg::OperationError {
            kind,
            message,
            req: u64::MAX,
            ..
        } => Some((kind, message)),
        _ => None,
    })
    .await
}

/// Give a credential root and wait out a failure of its own: a flushed `Ping` is not needed, the
/// grant is handled in order with what follows it.
fn root_env(root: &Path) -> Env {
    // The host's git config must not rewrite the fixtures' URLs (`url.<base>.insteadOf` turned
    // https into ssh on the dev host, #1352).
    Env::set(&[
        (ROOT_ENV, Some(root.as_os_str().to_owned())),
        ("GIT_CONFIG_GLOBAL", Some("/dev/null".into())),
        ("GIT_CONFIG_NOSYSTEM", Some("1".into())),
    ])
}

#[tokio::test]
async fn targets_follow_attach_add_and_remove_and_the_origin() {
    let _one = ONE_AT_A_TIME.lock().await;
    let dir = tempfile::tempdir().unwrap();
    let _env = root_env(&dir.path().join("creds"));
    let a = project(dir.path(), "a", Some("https://github.com/o/r"));
    let b = project(dir.path(), "b", None);
    let state = state_with(&[&a]);
    let mut client = connect_and_attach(&state, &a).await;

    let (list, old) = targets(&mut client).await;
    assert_eq!(old, None);
    assert_eq!(
        list,
        vec![CredentialTarget {
            project: a.clone(),
            push_url: Some("https://github.com/o/r".into())
        }]
    );

    send(
        &mut client,
        ClientMsg::ProjectAdd {
            req: 1,
            path: b.clone(),
        },
    )
    .await;
    let (list, _) = targets(&mut client).await;
    assert_eq!(list.len(), 2);
    assert_eq!(list.iter().find(|t| t.project == b).unwrap().push_url, None);

    // `origin` changes: a worktree refresh sees it.
    assert!(Command::new("git")
        .arg("-C")
        .arg(&a)
        .args(["remote", "set-url", "origin", "https://github.com/o/moved"])
        .status()
        .unwrap()
        .success());
    send(
        &mut client,
        ClientMsg::WorktreeRefresh {
            req: 2,
            project: a.clone(),
        },
    )
    .await;
    let (list, _) = targets(&mut client).await;
    assert_eq!(
        list.iter()
            .find(|t| t.project == a)
            .unwrap()
            .push_url
            .as_deref(),
        Some("https://github.com/o/moved")
    );

    send(
        &mut client,
        ClientMsg::ProjectRemove {
            req: 3,
            path: b.clone(),
        },
    )
    .await;
    let (list, _) = targets(&mut client).await;
    assert_eq!(list.len(), 1);
}

#[tokio::test]
async fn a_grant_is_written_atomically_with_mode_0600_and_replaced_by_the_next() {
    let _one = ONE_AT_A_TIME.lock().await;
    let dir = tempfile::tempdir().unwrap();
    let creds = dir.path().join("creds");
    let _env = root_env(&creds);
    let a = project(dir.path(), "a", Some("https://github.com/Acme/App"));
    let state = state_with(&[&a]);
    let mut client = connect_and_attach(&state, &a).await;
    let _ = targets(&mut client).await;

    let grant = |token: &str, expires_at| ClientMsg::GitCredentialGrant {
        repo: repo("https://github.com/Acme/App"),
        token: SecretToken::new(token),
        expires_at,
    };
    send(&mut client, grant("tok-1", soon())).await;
    send(
        &mut client,
        ClientMsg::GitCredentialRevoke {
            repo: repo("https://github.com/x/y"),
        },
    )
    .await;
    let file = creds.join("acme").join("app");
    let wait_for = |expected: &str| {
        let file = file.clone();
        let expected = expected.to_string();
        async move {
            for _ in 0..200 {
                if std::fs::read_to_string(&file).is_ok_and(|t| t == expected) {
                    return;
                }
                tokio::time::sleep(Duration::from_millis(50)).await;
            }
            panic!("{} never held {expected:?}", file.display());
        }
    };
    wait_for(&format!("expires_at={}\ntoken=tok-1\n", soon())).await;
    #[cfg(unix)]
    assert_eq!(
        std::fs::metadata(&file).unwrap().permissions().mode() & 0o777,
        0o600
    );

    send(&mut client, grant("tok-2", soon() + 1)).await;
    wait_for(&format!("expires_at={}\ntoken=tok-2\n", soon() + 1)).await;
    let leftovers: Vec<_> = std::fs::read_dir(file.parent().unwrap())
        .unwrap()
        .map(|e| e.unwrap().file_name())
        .collect();
    assert_eq!(
        leftovers,
        vec![OsString::from("app")],
        "no temporary file is left"
    );

    // Revoke removes it, and again is fine.
    for _ in 0..2 {
        send(
            &mut client,
            ClientMsg::GitCredentialRevoke {
                repo: repo("https://github.com/Acme/App"),
            },
        )
        .await;
    }
    for _ in 0..200 {
        if !file.exists() {
            return;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    panic!("the credential file was not removed");
}

#[tokio::test]
async fn a_grant_for_a_repository_no_project_pushes_to_is_refused() {
    let _one = ONE_AT_A_TIME.lock().await;
    let dir = tempfile::tempdir().unwrap();
    let creds = dir.path().join("creds");
    let _env = root_env(&creds);
    let a = project(dir.path(), "a", Some("https://github.com/o/r"));
    let state = state_with(&[&a]);
    let mut client = connect_and_attach(&state, &a).await;
    let _ = targets(&mut client).await;
    send(
        &mut client,
        ClientMsg::GitCredentialGrant {
            repo: repo("https://github.com/o/other"),
            token: SecretToken::new("tok-secret"),
            expires_at: soon(),
        },
    )
    .await;
    let (kind, message) = refusal(&mut client).await;
    assert_eq!(kind, ErrorKind::Refused);
    assert!(
        message.contains("o/other") && !message.contains("tok-secret"),
        "{message}"
    );
    assert!(!creds.join("o").join("other").exists());
}

#[tokio::test]
async fn a_host_process_daemon_refuses_grants_and_sends_no_targets() {
    let _one = ONE_AT_A_TIME.lock().await;
    let dir = tempfile::tempdir().unwrap();
    let _env = Env::set(&[
        (ROOT_ENV, None),
        ("MICOLD_IMAGE_REFERENCE", None),
        ("MICOLD_TOKEN_PATH", None),
    ]);
    let a = project(dir.path(), "a", Some("https://github.com/o/r"));
    let state = state_with(&[&a]);
    let mut client = connect_and_attach(&state, &a).await;
    send(
        &mut client,
        ClientMsg::GitCredentialGrant {
            repo: repo("https://github.com/o/r"),
            token: SecretToken::new("tok"),
            expires_at: soon(),
        },
    )
    .await;
    // Everything the daemon says before the refusal: no targets among it.
    let refused = tokio::time::timeout(Duration::from_secs(20), async {
        loop {
            match client.next().await.expect("open").expect("frame") {
                Frame::Control(DaemonMsg::GitCredentialTargets { .. }) => {
                    panic!("a host-process daemon sent targets")
                }
                Frame::Control(DaemonMsg::OperationError {
                    req: u64::MAX,
                    kind,
                    ..
                }) => return kind,
                _ => {}
            }
        }
    })
    .await
    .expect("refused");
    assert_eq!(refused, ErrorKind::Refused);
}

#[tokio::test]
async fn a_write_failure_names_the_repository_and_never_the_token() {
    let _one = ONE_AT_A_TIME.lock().await;
    let dir = tempfile::tempdir().unwrap();
    // The credential root is a file: no directory can be made under it.
    let root = dir.path().join("not-a-directory");
    std::fs::write(&root, "").unwrap();
    let _env = root_env(&root);
    let a = project(dir.path(), "a", Some("https://github.com/o/r"));
    let state = state_with(&[&a]);
    let mut client = connect_and_attach(&state, &a).await;
    let _ = targets(&mut client).await;
    send(
        &mut client,
        ClientMsg::GitCredentialGrant {
            repo: repo("https://github.com/o/r"),
            token: SecretToken::new("tok-secret"),
            expires_at: soon(),
        },
    )
    .await;
    let (kind, message) = refusal(&mut client).await;
    assert_eq!(kind, ErrorKind::IoFailed);
    assert!(
        message.contains("o/r") && !message.contains("tok-secret"),
        "{message}"
    );
}

// The stand-in `git` is a `sh` script on a `:`-separated PATH.
#[cfg(unix)]
#[tokio::test]
async fn a_git_older_than_2_31_marks_the_targets_and_refuses_every_grant() {
    let _one = ONE_AT_A_TIME.lock().await;
    let dir = tempfile::tempdir().unwrap();
    let creds = dir.path().join("creds");
    // A `git` that says it is 2.30.0 and otherwise is the real one.
    let real =
        String::from_utf8(Command::new("which").arg("git").output().unwrap().stdout).unwrap();
    let bin = dir.path().join("bin");
    std::fs::create_dir_all(&bin).unwrap();
    let stub = bin.join("git");
    std::fs::write(
        &stub,
        format!(
            "#!/bin/sh\nif [ \"$1\" = \"--version\" ]; then echo 'git version 2.30.0'; exit 0; fi\nexec {} \"$@\"\n",
            real.trim()
        ),
    )
    .unwrap();
    std::fs::set_permissions(&stub, std::fs::Permissions::from_mode(0o755)).unwrap();
    let a = project(dir.path(), "a", Some("https://github.com/o/r"));
    let path = format!("{}:{}", bin.display(), std::env::var("PATH").unwrap());
    let _env = Env::set(&[
        (ROOT_ENV, Some(creds.as_os_str().to_owned())),
        ("PATH", Some(path.into())),
    ]);
    let state = state_with(&[&a]);
    let mut client = connect_and_attach(&state, &a).await;
    let (list, old) = targets(&mut client).await;
    assert_eq!((list.len(), old.as_deref()), (1, Some("2.30.0")));
    send(
        &mut client,
        ClientMsg::GitCredentialGrant {
            repo: repo("https://github.com/o/r"),
            token: SecretToken::new("tok"),
            expires_at: soon(),
        },
    )
    .await;
    let (kind, message) = refusal(&mut client).await;
    assert_eq!(kind, ErrorKind::Refused);
    assert!(message.contains("2.30.0"), "{message}");
    assert!(!creds.exists());
}

#[tokio::test]
async fn a_malformed_token_or_a_stale_expiry_is_refused_and_a_path_is_not_a_repository() {
    let _one = ONE_AT_A_TIME.lock().await;
    let dir = tempfile::tempdir().unwrap();
    let creds = dir.path().join("creds");
    let _env = root_env(&creds);
    let a = project(dir.path(), "a", Some("https://github.com/o/r"));
    let state = state_with(&[&a]);
    let mut client = connect_and_attach(&state, &a).await;
    let _ = targets(&mut client).await;
    for (token, expires_at) in [
        ("a\nexpires_at=9", soon()),
        ("", soon()),
        ("tok", 5),
        ("tok", soon() + MAX_LIFETIME_SECS + CLOCK_SKEW_SECS + 60),
    ] {
        send(
            &mut client,
            ClientMsg::GitCredentialGrant {
                repo: repo("https://github.com/o/r"),
                token: SecretToken::new(token),
                expires_at,
            },
        )
        .await;
        assert_eq!(refusal(&mut client).await.0, ErrorKind::InvalidInput);
    }
    assert!(!creds.exists());
    // A peer cannot name a path: the repository does not even decode.
    let bad = serde_json::json!({"GitCredentialRevoke": {"repo": {"owner": "..", "name": "x"}}});
    assert!(serde_json::from_value::<ClientMsg>(bad).is_err());
}

#[tokio::test]
async fn a_grant_expiring_in_300_days_is_accepted() {
    let _one = ONE_AT_A_TIME.lock().await;
    let dir = tempfile::tempdir().unwrap();
    let creds = dir.path().join("creds");
    let _env = root_env(&creds);
    let a = project(dir.path(), "a", Some("https://github.com/o/r"));
    let state = state_with(&[&a]);
    let mut client = connect_and_attach(&state, &a).await;
    let _ = targets(&mut client).await;
    let expires_at = soon() + 300 * 86_400;
    send(
        &mut client,
        ClientMsg::GitCredentialGrant {
            repo: repo("https://github.com/o/r"),
            token: SecretToken::new("fake-pat"),
            expires_at,
        },
    )
    .await;
    let file = creds.join("o").join("r");
    let expected = format!("expires_at={expires_at}\ntoken=fake-pat\n");
    for _ in 0..200 {
        if std::fs::read_to_string(&file).is_ok_and(|t| t == expected) {
            return;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    panic!("{} never held the 300-day grant", file.display());
}
