//! #1760 (US2, US3; FR-003 to FR-007, SC-001 to SC-004; step 11 of the #1002 quickstart): a pod
//! daemon given the user's scoped GitHub token behind the per-daemon opt-in pushes to and pulls from
//! the repository the token is filtered to, and holds nothing else.
//!
//! The pod talks to a fake forge on the docker gateway (`kube_real_support::forge`) that accepts
//! only a secret unique to this run. Four pods, one namespace each:
//!
//! - **A** (opt-in on, a token filtered to the project's repository): a session pushes, pulls, is
//!   offered no credential for another repository or host, and its environment (read in two
//!   shells) and home hold neither the secret, `SSH_AUTH_SOCK` nor `~/.ssh`;
//! - **C** (opt-in on, only a token filtered to another repository): the token is delivered, but
//!   the push fails for want of a credential;
//! - **D** (opt-in off): no token key in the pod's Secret, and the push fails.
//!
//! The client's log output is captured for the run and holds the secret on no line. The test
//! prints `SKIP` where docker, kind or kubectl is missing.
#![cfg(feature = "kube-real-runtime")]

mod kube_real_support;

use std::io::{Read, Write};
use std::net::TcpStream;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, OnceLock};

use kube_real_support::forge::{Forge, USER};
use kube_real_support::*;
use micold_client::daemon_config_delivery::GITHUB_TOKEN_PREFIX;
use micold_core::connect::DaemonConnection;
use micold_core::kube::manifest::STARTUP_ENV_MOUNT;
use micold_core::kube::names::ResourceNames;
use micold_core::session::{AiCli, ShellInstanceId};

/// The repository the user's project is, and one it is not.
const GRANTED: &str = "kubetest/scoped";
const ELSEWHERE: &str = "kubetest/elsewhere";

/// The pod's own commit, pushed as this branch; and the forge's, pulled as the other.
const POD_BRANCH: &str = "pod-branch";
const FORGE_BRANCH: &str = "forge-branch";

/// What git prints when no credential was offered and it may not prompt.
const NO_CREDENTIAL: &str = "terminal prompts disabled|could not read|authentication failed";

/// A secret that exists only in this run.
fn run_secret(kind: &str) -> String {
    format!("ghp_{kind}_{}", uuid::Uuid::new_v4().simple())
}

/// The client's log output of the whole run, as text.
static LOG: OnceLock<Arc<Mutex<Vec<u8>>>> = OnceLock::new();

struct LogWriter(Arc<Mutex<Vec<u8>>>);

impl Write for LogWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.0.lock().unwrap().extend_from_slice(buf);
        Ok(buf.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

/// Capture the log output of this process from now on (all levels up to debug).
fn capture_log() -> Arc<Mutex<Vec<u8>>> {
    LOG.get_or_init(|| {
        let buffer = Arc::new(Mutex::new(Vec::new()));
        let sink = buffer.clone();
        let subscriber = tracing_subscriber::fmt()
            .with_max_level(tracing::Level::DEBUG)
            .with_ansi(false)
            .with_writer(move || LogWriter(sink.clone()))
            .finish();
        // Another subscriber already set (the harness's own) is not an error for the scan to
        // hide: the buffer stays empty and the test says so.
        let _ = tracing::subscriber::set_global_default(subscriber);
        buffer
    })
    .clone()
}

/// A started pod with a session and a shell in it.
struct Pod {
    conn: Box<DaemonConnection>,
    shell: Shell,
    project: PathBuf,
    name: String,
    ns: Namespace,
    _relay: micold_client::kube::relay::Relay,
}

/// Start a pod daemon named `name` that is given the tokens of `store` (opt-in `opt_in`) for the
/// saved projects `projects`, add a session and open its shell.
#[allow(clippy::too_many_arguments)]
async fn pod(
    client: &mut Client,
    suite: &Suite,
    name: &str,
    forge: &Forge,
    store: &std::path::Path,
    opt_in: bool,
    projects: &[&str],
) -> Pod {
    let ns = Namespace::create("kt-tok");
    let settings = settings(&ns.0, &[AiCli::ClaudeCode]);
    let projects: Vec<_> = projects.iter().map(|p| repo(p)).collect();
    let (mut conn, welcome, relay) = start_with_tokens(
        client,
        name,
        &settings,
        &Events::default(),
        store,
        &forge.base(),
        opt_in,
        &projects,
    )
    .await
    .expect("starts");
    let mut seen = String::new();
    let work = work(&mut conn, suite, &mut seen).await;
    let shell = Shell::open(&mut conn, &welcome, &work.project, work.session).await;
    Pod {
        conn,
        shell,
        project: work.project,
        name: name.to_string(),
        ns,
        _relay: relay,
    }
}

impl Pod {
    /// The keys of the pod's startup Secret (none when there is no Secret).
    fn startup_keys(&self) -> Vec<String> {
        let names = ResourceNames::new(CONTEXT, &self.ns.0, &self.name);
        match kubectl(
            &[
                "-n",
                &self.ns.0,
                "get",
                "secret",
                &names.startup_secret(),
                "-o",
                "json",
            ],
            None,
        ) {
            Ok(json) => {
                let secret: serde_json::Value = serde_json::from_str(&json).unwrap();
                secret["data"]
                    .as_object()
                    .map(|d| d.keys().cloned().collect())
                    .unwrap_or_default()
            }
            Err(e) if e.contains("NotFound") => Vec::new(),
            Err(e) => panic!("read the startup secret: {e}"),
        }
    }

    /// Push a commit made in the pod to `branch` of `repo` on the forge.
    fn push(forge: &Forge, repo: &str, branch: &str) -> String {
        format!(
            "export GIT_TERMINAL_PROMPT=0; rm -rf /tmp/push && git init -q /tmp/push && cd /tmp/push && git -c user.name=t -c user.email=t@t commit -q --allow-empty -m from-pod && git push -q {} HEAD:refs/heads/{branch}",
            forge.url(repo)
        )
    }

    /// Whether the credential helper offers a password for a request, probed with `git credential
    /// fill` in the session as git itself would ask. Prints `OFFER<n>=<count>` for each.
    async fn offers(&mut self, tag: &str, requests: &[(&str, &str, &str)]) -> Vec<usize> {
        let mut cmd = String::from(
            r#"offer() { printf 'protocol=%s\nhost=%s\npath=%s\n\n' "$1" "$2" "$3" | GIT_TERMINAL_PROMPT=0 git credential fill 2>/dev/null | grep -c '^password='; }"#,
        );
        for (i, (protocol, host, path)) in requests.iter().enumerate() {
            cmd.push_str(&format!(
                "; echo OFFER{i}\"=\"$(offer {protocol} {host} {path})"
            ));
        }
        let seen = self.shell.capture(&mut self.conn, tag, &cmd, 120).await;
        (0..requests.len())
            .map(|i| {
                field(&seen, &format!("OFFER{i}"))
                    .unwrap_or_else(|| panic!("no OFFER{i} in: {seen}"))
                    .trim()
                    .parse()
                    .expect("a count")
            })
            .collect()
    }

    /// A push that must fail for want of a credential: git says so, and the forge holds nothing.
    async fn push_fails(&mut self, tag: &str, forge: &Forge, repo: &str, branch: &str) {
        let before = forge.requests().len();
        let cmd = format!(
            "if out=$({} 2>&1); then echo \"$out\"; exit 1; else echo \"$out\" | grep -qiE '{NO_CREDENTIAL}'; fi",
            Self::push(forge, repo, branch)
        );
        self.shell.step(&mut self.conn, tag, &cmd, 120).await;
        assert_eq!(forge.branch(repo, branch), None, "{tag}: the push landed");
        let credentialed: Vec<_> = forge.requests()[before..]
            .iter()
            .filter(|r| r.credential.is_some())
            .cloned()
            .collect();
        assert!(
            credentialed.is_empty(),
            "{tag}: the pod offered a credential: {credentialed:?}"
        );
    }
}

/// The text of every file in a token store, to look for its encodings in the log.
fn store_texts(dir: &std::path::Path) -> Vec<String> {
    std::fs::read_dir(dir)
        .expect("the token store lists")
        .filter_map(|e| std::fs::read_to_string(e.ok()?.path()).ok())
        .collect()
}

/// The pod's file for the token `name`: in the startup Secret's mount, which is outside the home.
fn token_file(name: &str) -> String {
    format!("{STARTUP_ENV_MOUNT}/{GITHUB_TOKEN_PREFIX}{name}")
}

#[tokio::test(flavor = "multi_thread")]
async fn kube_real_scoped_token() {
    let test = "kube_real_scoped_token";
    let Some(suite) = suite_or_skip(test) else {
        return;
    };
    let log = capture_log();
    let mut file_texts: Vec<String> = Vec::new();
    let (secret, other_secret) = (run_secret("run"), run_secret("other"));
    let forge = Forge::start(&secret, &[GRANTED, ELSEWHERE]).expect("the fake forge starts");
    let mut client = client();

    // A: opt-in on, a token filtered to the project's repository.
    let store = token_store(&[("kube-run", &secret, &[GRANTED])]);
    file_texts.extend(store_texts(store.path()));
    let mut a = pod(
        &mut client,
        suite,
        "Token box A",
        &forge,
        store.path(),
        true,
        &[GRANTED],
    )
    .await;
    assert_eq!(
        a.startup_keys(),
        vec![format!("{GITHUB_TOKEN_PREFIX}kube-run")],
        "A: the token is delivered"
    );
    a.shell
        .step(
            &mut a.conn,
            "PUSH",
            &Pod::push(&forge, GRANTED, POD_BRANCH),
            120,
        )
        .await;
    assert!(
        forge.branch(GRANTED, POD_BRANCH).is_some(),
        "A: the push reached the forge"
    );
    let forge_commit = forge.add_commit(GRANTED, FORGE_BRANCH);
    let pull = format!(
        "export GIT_TERMINAL_PROMPT=0; rm -rf /tmp/pull && git clone -q {url} /tmp/pull && cd /tmp/pull && git fetch -q origin {FORGE_BRANCH} && test \"$(git rev-parse FETCH_HEAD)\" = {forge_commit} && git ls-remote origin {POD_BRANCH} | grep -q {POD_BRANCH}",
        url = forge.url(GRANTED)
    );
    a.shell.step(&mut a.conn, "PULL", &pull, 120).await;
    // The helper offers a credential for the granted repository, and for no other repository,
    // spelling of the host or forge.
    let port_host = forge.base().replace("http://", "");
    let offers = a
        .offers(
            "OFFERS",
            &[
                ("http", &port_host, &format!("{GRANTED}.git")),
                ("http", &port_host, &format!("{ELSEWHERE}.git")),
                ("https", "github.com", &format!("{GRANTED}.git")),
                ("http", "other.example:8080", &format!("{GRANTED}.git")),
            ],
        )
        .await;
    assert_eq!(offers, [1, 0, 0, 0], "A: the credential helper's offers");
    // The forge saw the run's secret from git, as `x-access-token`, and nothing else.
    let requests = forge.requests();
    assert!(
        requests.iter().any(|r| r.accepted),
        "A: the forge accepted a request"
    );
    for r in requests.iter().filter_map(|r| r.credential.as_ref()) {
        assert_eq!((r.0.as_str(), r.1.as_str()), (USER, secret.as_str()), "A");
    }
    // The pod holds nothing else, in two terminals of the session and in a second session.
    let file = token_file("kube-run");
    // The second session is opened only after the first session's checks: opening it makes its
    // terminal the only one viewed, so the first session's grid stops streaming.
    let mut second = None;
    for i in 0..3 {
        if i == 1 {
            a.shell
                .switch(&mut a.conn, &a.project, ShellInstanceId(2))
                .await;
        }
        if i == 2 {
            second = Some(
                Shell::open_another_session(&mut a.conn, &a.project, &mut String::new()).await,
            );
        }
        let shell = match second.as_mut() {
            Some(s) => s,
            None => &mut a.shell,
        };
        let facts = session_env_and_home(&mut a.conn, shell, &format!("FACTS{i}"), &file).await;
        assert_eq!(facts.env_secret_lines, 0, "A shell {i}: secret in env");
        assert_eq!(facts.ssh_auth_sock_lines, 0, "A shell {i}: SSH_AUTH_SOCK");
        assert!(!facts.home_has_ssh, "A shell {i}: ~/.ssh exists");
        assert!(
            !facts.home_entries.iter().any(|e| e == ".ssh"),
            "A shell {i}: {:?}",
            facts.home_entries
        );
        assert_eq!(facts.home_secret_files, 0, "A shell {i}: secret in home");
        // The token's mount is outside the home, by design.
        assert!(
            !facts.home.is_empty() && !file.starts_with(&facts.home),
            "{facts:?}"
        );
    }
    drop(a);

    // C: opt-in on, but the only token is filtered to another repository. It is delivered (the
    // daemon has that repository too) and the helper still offers nothing for the granted one.
    let store = token_store(&[("kube-other", &other_secret, &[ELSEWHERE])]);
    file_texts.extend(store_texts(store.path()));
    let mut c = pod(
        &mut client,
        suite,
        "Token box C",
        &forge,
        store.path(),
        true,
        &[GRANTED, ELSEWHERE],
    )
    .await;
    assert_eq!(
        c.startup_keys(),
        vec![format!("{GITHUB_TOKEN_PREFIX}kube-other")],
        "C: the token is delivered"
    );
    c.push_fails("PUSHC", &forge, GRANTED, "c-branch").await;
    let facts = session_env_and_home(
        &mut c.conn,
        &mut c.shell,
        "FACTSC",
        &token_file("kube-other"),
    )
    .await;
    assert_eq!(
        (
            facts.env_secret_lines,
            facts.home_secret_files,
            facts.ssh_auth_sock_lines
        ),
        (0, 0, 0),
        "C: {facts:?}"
    );
    drop(c);

    // D: opt-in off: the same store the opt-in would have delivered from, and no key in the pod.
    let store = token_store(&[("kube-run", &secret, &[GRANTED])]);
    file_texts.extend(store_texts(store.path()));
    let mut d = pod(
        &mut client,
        suite,
        "Token box D",
        &forge,
        store.path(),
        false,
        &[GRANTED],
    )
    .await;
    assert_eq!(d.startup_keys(), Vec::<String>::new(), "D: a token key");
    d.push_fails("PUSHD", &forge, GRANTED, "d-branch").await;
    let offers = d
        .offers("OFFERD", &[("http", &port_host, &format!("{GRANTED}.git"))])
        .await;
    assert_eq!(offers, [0], "D: the helper offered a credential");
    drop(d);

    // The client's own log output never held either secret.
    let text = String::from_utf8_lossy(&log.lock().unwrap()).to_string();
    assert!(
        !text.is_empty(),
        "the client's log output was captured (another subscriber is installed?)"
    );
    use base64::Engine;
    let encode = |s: &str| base64::engine::general_purpose::STANDARD.encode(s);
    let needles: Vec<String> = [&secret, &other_secret]
        .into_iter()
        .chain(file_texts.iter())
        .flat_map(|s| [s.clone(), encode(s), encode(&format!("{USER}:{s}"))])
        .collect();
    let leaked: Vec<&str> = text
        .lines()
        .filter(|l| needles.iter().any(|n| l.contains(n.as_str())))
        .collect();
    assert!(leaked.is_empty(), "secret in the client log: {leaked:?}");
}

/// A request to the forge (T021; runs with no cluster): only Basic `x-access-token:<secret>` is
/// accepted; anything else is a 401 with `WWW-Authenticate`; every credential is recorded.
#[test]
fn kube_real_forge_accepts_only_the_run_secret() {
    use base64::Engine;
    let secret = run_secret("forge");
    let forge = Forge::start(&secret, &[GRANTED]).expect("the forge starts");
    let get = |path: &str, auth: Option<String>| -> String {
        let mut stream = TcpStream::connect(forge.local_addr()).unwrap();
        let header = auth.map_or(String::new(), |a| format!("Authorization: {a}\r\n"));
        write!(
            stream,
            "GET {path} HTTP/1.1\r\nHost: forge\r\n{header}Connection: close\r\n\r\n"
        )
        .unwrap();
        let mut answer = String::new();
        stream.read_to_string(&mut answer).unwrap();
        answer
    };
    let basic = |user: &str, password: &str| {
        let encoded =
            base64::engine::general_purpose::STANDARD.encode(format!("{user}:{password}"));
        format!("Basic {encoded}")
    };
    let refs = format!("/{GRANTED}.git/info/refs?service=git-upload-pack");

    let anonymous = get(&refs, None);
    assert!(anonymous.starts_with("HTTP/1.1 401"), "{anonymous}");
    assert!(anonymous.contains("WWW-Authenticate: Basic"), "{anonymous}");
    for wrong in [
        basic(USER, "not-the-secret"),
        basic("someone", &secret),
        basic(USER, &format!("{secret}x")),
        format!("Basic {}", "!!not base64!!"),
    ] {
        let answer = get(&refs, Some(wrong.clone()));
        assert!(answer.starts_with("HTTP/1.1 401"), "{wrong}: {answer}");
    }
    let answer = get(&refs, Some(basic(USER, &secret)));
    assert!(answer.starts_with("HTTP/1.1 200"), "{answer}");
    assert!(answer.contains("# service=git-upload-pack"), "{answer}");
    // The REST listing takes the same secret, as a Bearer token, under either prefix.
    for path in ["/user/repos", "/api/v3/user/repos"] {
        let answer = get(path, Some(format!("Bearer {secret}")));
        assert!(answer.starts_with("HTTP/1.1 200"), "{answer}");
        assert!(answer.contains(GRANTED), "{answer}");
    }
    assert!(get("/user/repos", None).starts_with("HTTP/1.1 401"));

    // Real git works against it with the secret, push and pull, and not without.
    let seen_before = forge.requests().len();
    let work = tempfile::tempdir().unwrap();
    let git_in = |dir: &std::path::Path, args: &[&str]| {
        std::process::Command::new("git")
            .args(["-c", "user.name=t", "-c", "user.email=t@t"])
            .args(["-c", "credential.helper="])
            .args(args)
            .env("GIT_TERMINAL_PROMPT", "0")
            .current_dir(dir)
            .output()
            .unwrap()
    };
    let git = |args: &[&str]| git_in(work.path(), args);
    let with = forge
        .local_url(GRANTED)
        .replace("http://", &format!("http://{USER}:{secret}@"));
    let without = forge.local_url(GRANTED);
    assert!(!git(&["clone", "-q", &without, "none"]).status.success());
    assert!(git(&["clone", "-q", &with, "clone"]).status.success());
    let clone = work.path().join("clone");
    let in_clone = |args: &[&str]| git_in(&clone, args);
    assert!(in_clone(&["commit", "-q", "--allow-empty", "-m", "x"])
        .status
        .success());
    assert!(
        in_clone(&["push", "-q", "origin", "HEAD:refs/heads/pushed"])
            .status
            .success()
    );
    assert!(forge.branch(GRANTED, "pushed").is_some());
    let seen = forge.requests();
    assert!(seen.len() > seen_before);
    assert!(seen[seen_before..]
        .iter()
        .any(|r| r.path.ends_with("git-receive-pack") && r.accepted));
    let seen = &seen[..seen_before];
    assert_eq!(seen.len(), 1 + 4 + 1 + 2 + 1);
    assert_eq!(seen[0].credential, None);
    assert_eq!(
        seen[1].credential,
        Some((USER.to_string(), "not-the-secret".to_string()))
    );
    assert_eq!(seen.iter().filter(|r| r.accepted).count(), 3, "{seen:?}");
}
