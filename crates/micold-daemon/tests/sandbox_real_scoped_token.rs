//! #1542 (FR-008, SC-002) — Part C steps 4-8, 10 and 11 of `specs/1002-sandbox-scoped-token/
//! quickstart.md`, run against a real container daemon and a fake GitHub.
//!
//! The token is checked in-process by the core's `pat::check` against a loopback fake GitHub API
//! (`fake_github.rs`, shared with the client's tests) through the client's own `UreqTransport`.
//! The result is granted to the daemon in the container, whose helper answers for a local smart-HTTP
//! git remote (`MICOLD_GIT_CREDENTIAL_BASE`), bound on every interface and reached from the
//! container by name. Step 7's `/hooks` 403 is a real-GitHub fact and stays manual.
//!
//! Step 11 (a kind pod) is follow-up #1760 and not covered here; the ignored test below marks it.
//!
//! Behind `sandbox-real-runtime` (Principle VI). `mise run image && mise run test-sandbox`.

#![cfg(all(feature = "sandbox-real-runtime", unix))]

#[allow(dead_code)]
#[path = "../../micold-client/tests/support/fake_github.rs"]
mod fake_github;
#[path = "support/git_http.rs"]
mod git_http;
mod sandbox_real_support;

use std::time::{Duration, SystemTime, UNIX_EPOCH};

use fake_github::FakeGithubApi;
use futures_util::SinkExt;
use micold_client::pat_source::UreqTransport;
use micold_core::github::GithubRepo;
use micold_core::local_time::rfc3339_utc;
use micold_core::protocol::codec::Frame;
use micold_core::protocol::messages::ClientMsg;
use micold_core::scoped_credential::git_env::git_env_with_helper;
use micold_core::scoped_credential::pat;
use micold_core::scoped_credential::{SecretToken, UnixTime};
use sandbox_real_support::{cli_out, git, seed, start_sandbox, wait_for_accept, SandboxSpec};

const CONTAINER: &str = "micold-scoped-token-probe";
const NETWORK: &str = "micold-scoped-token-probe-net";
/// Not 7727, and not a neighbour's: a developer's own sandbox must not be disturbed.
const PORT: u16 = 17743;
const TOKEN: &str = "github_pat_FAKE_scratch0001";
const REPO: &str = "you/scratch";
/// What the container calls the host: `--add-host` maps it to the runtime's host gateway.
const HOST_NAME: &str = "micold-host";

fn repo() -> GithubRepo {
    GithubRepo::from_remote_url(&format!("https://github.com/{REPO}")).expect("a GitHub repository")
}

fn now() -> UnixTime {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

/// The `github-authentication-token-expiration` header value GitHub sends for a token expiring at `at`.
fn expiry_header(at: UnixTime) -> String {
    rfc3339_utc(at as i64)
        .replace('T', " ")
        .replace('Z', " UTC")
}

/// `(exit status, stdout+stderr)` of `script` run in the container as `user`.
fn sh(user: &str, script: &str) -> (i32, String) {
    let out = cli_out(&[
        "exec",
        "-u",
        user,
        CONTAINER,
        "sh",
        "-c",
        &format!("({script}) 2>&1; echo EXIT=$?"),
    ]);
    let (body, code) = out
        .rsplit_once("EXIT=")
        .expect("the script reports its status");
    (code.trim().parse().expect("a status"), body.to_string())
}

async fn grant(conn: &mut micold_core::connect::DaemonConnection, expires_at: UnixTime) {
    conn.send(Frame::Control(ClientMsg::GitCredentialGrant {
        repo: repo(),
        token: SecretToken::new(TOKEN),
        expires_at,
    }))
    .await
    .expect("send the grant");
}

async fn revoke(conn: &mut micold_core::connect::DaemonConnection) {
    conn.send(Frame::Control(ClientMsg::GitCredentialRevoke {
        repo: repo(),
    }))
    .await
    .expect("send the revoke");
}

/// Wait until the credential file of the repository exists (or is gone) in the container.
fn wait_file(user: &str, present: bool) {
    let path = format!("/run/micold/git/{REPO}");
    let script = if present {
        format!("test -s {path}")
    } else {
        format!("test ! -e {path}")
    };
    for _ in 0..100 {
        if sh(user, &script).0 == 0 {
            return;
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    panic!(
        "/run/micold/git/{REPO} did not become {}",
        if present { "present" } else { "absent" }
    );
}

#[tokio::test]
async fn sandbox_real_scoped_token_container_steps_4_to_8_and_10() {
    let dir = tempfile::tempdir().unwrap();

    // The fake GitHub API: the token reaches only you/scratch and expires in 30 days.
    let api = FakeGithubApi::start();
    let expiry = now() + 30 * 86_400;
    api.probe(REPO, true, Some(&expiry_header(expiry)));
    let checked = pat::check(
        &UreqTransport::default(),
        &api.base(),
        &repo(),
        &SecretToken::new(TOKEN),
        now(),
    )
    .expect("the fake accepts the token");
    assert_eq!(checked, expiry, "the check reports the fake's expiry");

    // The git host: a bare you/scratch.git behind basic auth with the token, on every interface.
    let backend = git_http::git_http_backend().expect("`git http-backend` is installed");
    let projects = dir.path().join("remote");
    let bare = projects.join("you").join("scratch.git");
    std::fs::create_dir_all(&bare).unwrap();
    git(&bare, &["init", "-q", "--bare", "-b", "main"]);
    let port = git_http::serve("0.0.0.0", projects, backend, TOKEN);
    let base = format!("http://{HOST_NAME}:{port}");

    // The container's project: a clone of GitHub's you/scratch as far as the daemon can tell.
    let data = dir.path().join("data");
    let project = dir.path().join("project");
    std::fs::create_dir_all(&project).unwrap();
    git(&project, &["init", "-q", "-b", "main"]);
    git(
        &project,
        &[
            "remote",
            "add",
            "origin",
            &format!("https://github.com/{REPO}"),
        ],
    );
    seed(&data, &project, "scoped-token");
    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".into());
    let _sandbox = start_sandbox(&SandboxSpec {
        container: CONTAINER,
        network: NETWORK,
        port: PORT,
        data_home: &data,
        project: &project,
        home: &home,
        survive_logout: false,
        extra: &[
            "--add-host".to_string(),
            format!("{HOST_NAME}:host-gateway"),
        ],
    });
    let (mut conn, _) = wait_for_accept(PORT).await;

    // The daemon's installed binary is the helper; its owner is the user the credential files and
    // the sessions belong to.
    let (_, daemon) = sh(
        "root",
        "find / -type f -name micold-daemon -path '*/.micold/remote/*' 2>/dev/null | head -1",
    );
    let daemon = daemon.trim().to_string();
    assert!(
        !daemon.is_empty(),
        "the provisioned daemon is in the container"
    );
    let (_, user) = sh("root", &format!("stat -c %U {daemon}"));
    let user = user.trim().to_string();
    let mut env = git_env_with_helper(&base, &daemon);
    env.insert("MICOLD_GIT_CREDENTIAL_BASE".into(), base.clone());
    for (k, v) in [
        ("GIT_AUTHOR_NAME", "t"),
        ("GIT_AUTHOR_EMAIL", "t@example.com"),
        ("GIT_COMMITTER_NAME", "t"),
        ("GIT_COMMITTER_EMAIL", "t@example.com"),
    ] {
        env.insert(k.into(), v.into());
    }
    let exports: String = env
        .iter()
        .map(|(k, v)| format!("export {k}='{v}'; "))
        .collect();
    let git_in = |script: &str| sh(&user, &format!("{exports}{script}"));
    let url = format!("{base}/{REPO}.git");

    // Steps 4 and 5: the credential is granted; a clone, a push and a pull succeed ----------------
    grant(&mut conn, expiry).await;
    wait_file(&user, true);
    let (code, out) = git_in(&format!(
        "rm -rf /tmp/a /tmp/b && git clone -q {url} /tmp/a && cd /tmp/a && echo one >a.txt && \
         git add . && git commit -q -m one && git push -q origin HEAD:main && \
         git clone -q {url} /tmp/b && cd /tmp/a && echo two >b.txt && git add . && \
         git commit -q -m two && git push -q origin HEAD:main && cd /tmp/b && \
         git pull -q origin main && test -f b.txt"
    ));
    assert_eq!(code, 0, "push and pull with the scoped credential: {out}");
    // An SSH-form remote of the forge is rewritten to the same HTTPS and authenticates the same way.
    let (code, out) = git_in("cd /tmp/a && git ls-remote git@github.com:you/scratch.git");
    assert_eq!(code, 0, "an ssh remote is rewritten: {out}");
    assert!(out.contains("refs/heads/main"), "{out}");

    // No token and no SSH agent in a session's environment, and no ~/.ssh ------------------------
    let (_, env_out) = sh(&user, "env");
    for name in ["SSH_AUTH_SOCK", "GH_TOKEN", "GITHUB_TOKEN"] {
        assert!(!env_out.contains(name), "the environment names {name}");
    }
    assert!(!env_out.contains(TOKEN), "the environment holds the token");
    let (code, out) = sh(&user, "test ! -e \"$HOME/.ssh\"");
    assert_eq!(code, 0, "~/.ssh exists: {out}");

    // Steps 6 and 7: another repository gets nothing; the token file holds the token --------------
    let (_, filled) = git_in(&format!(
        "printf 'protocol=http\\nhost={HOST_NAME}:{port}\\npath=you/other\\n\\n' | git credential fill"
    ));
    assert!(
        !filled.contains("password="),
        "another repository got a credential: {filled}"
    );
    let (_, filled) = git_in(&format!(
        "printf 'protocol=http\\nhost={HOST_NAME}:{port}\\npath=you/scratch\\n\\n' | git credential fill"
    ));
    assert!(filled.contains(&format!("password={TOKEN}")), "{filled}");
    let (_, file) = sh(&user, &format!("cat /run/micold/git/{REPO}"));
    assert!(
        file.contains(&format!("token={TOKEN}")),
        "the token file: {file}"
    );

    // Step 8: removed, push fails; pasted again, push succeeds, no daemon restart ------------------
    revoke(&mut conn).await;
    wait_file(&user, false);
    let push = "cd /tmp/a && echo three >c.txt && git add . && git commit -q -m three && \
                git push -q origin HEAD:main";
    let (code, out) = git_in(push);
    assert_ne!(code, 0, "push succeeded without a credential: {out}");
    grant(&mut conn, expiry).await;
    wait_file(&user, true);
    let (code, out) = git_in("cd /tmp/a && git push -q origin HEAD:main");
    assert_eq!(code, 0, "push after the token is pasted again: {out}");

    // Step 10: the token expires; push fails until a new one is pasted -----------------------------
    grant(&mut conn, now() + 6).await;
    // The helper stops answering for the repository once the grant's expiry passes (the file stays
    // until the client revokes it, so "gone" is the helper's refusal, not the file's absence).
    // Poll for that refusal rather than sleeping a guessed time.
    let deadline = std::time::Instant::now() + Duration::from_secs(60);
    loop {
        let (_, filled) = git_in(&format!(
            "printf 'protocol=http\\nhost={HOST_NAME}:{port}\\npath=you/scratch\\n\\n' | git credential fill"
        ));
        if !filled.contains("password=") {
            break;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "the helper still answers 60 s after the grant expired: {filled}"
        );
        std::thread::sleep(Duration::from_millis(500));
    }
    let (code, out) = git_in("cd /tmp/a && echo four >d.txt && git add . && git commit -q -m four && git push -q origin HEAD:main");
    assert_ne!(code, 0, "push succeeded with an expired credential: {out}");
    revoke(&mut conn).await;
    wait_file(&user, false);
    grant(&mut conn, expiry).await;
    wait_file(&user, true);
    let (code, out) = git_in("cd /tmp/a && git push -q origin HEAD:main");
    assert_eq!(code, 0, "push after a new token is pasted: {out}");
}

/// Step 11: a kind pod. Reported skipped: the pod harness is `micold-client`'s `kube_real_*` suite.
#[test]
#[ignore = "step 11: the kind pod harness lives in micold-client's kube_real_* suite"]
fn sandbox_real_scoped_token_kind_step_11_is_skipped() {
    eprintln!(
        "SKIPPED sandbox_real_scoped_token step 11: the kind pod harness lives in micold-client's \
         kube_real_* suite (mise run test-kube); not reachable from this crate"
    );
}
