//! The pasted-token source and the per-daemon manager, against a fake GitHub, a temporary store and
//! a clock the test owns. Every token here is fake; nothing reaches a network.

use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Mutex;

use micold_core::local_time::rfc3339_utc;
use micold_core::protocol::messages::ClientMsg;

use super::*;

const T0: UnixTime = 1_800_000_000;
const DAY: u64 = 86_400;
const PAT_A: &str = "github_pat_FAKE_aaaaaaaaaaaa";
const PAT_A2: &str = "github_pat_FAKE_a2a2a2a2a2a2";
const PAT_B: &str = "github_pat_FAKE_bbbbbbbbbbbb";

fn repo(name: &str) -> GithubRepo {
    GithubRepo::from_remote_url(&format!("https://github.com/o/{name}")).unwrap()
}

fn target(project: &str, name: &str) -> CredentialTarget {
    CredentialTarget {
        project: project.into(),
        push_url: Some(format!("https://github.com/o/{name}")),
    }
}

/// A GitHub that answers the probe and the listing of any token with the expiry it is told, or
/// refuses, or is offline (the error text then carries the token, as a careless transport's might).
struct FakeGithub {
    expires_at: AtomicU64,
    offline: AtomicBool,
    requests: Mutex<Vec<String>>,
}

impl FakeGithub {
    fn new(expires_at: UnixTime) -> Arc<Self> {
        Arc::new(Self {
            expires_at: AtomicU64::new(expires_at),
            offline: AtomicBool::new(false),
            requests: Mutex::default(),
        })
    }

    fn requests(&self) -> usize {
        self.requests.lock().unwrap().len()
    }
}

impl Transport for FakeGithub {
    fn get(&self, url: &str, token: &SecretToken) -> Result<Reply, String> {
        self.requests.lock().unwrap().push(url.to_string());
        if self.offline.load(Ordering::SeqCst) {
            return Err(format!("connection refused (bearer {})", token.expose()));
        }
        if url.contains("/user/repos") {
            return Ok(Reply {
                status: 200,
                body: "[]".into(),
                ..Reply::default()
            });
        }
        let expires = rfc3339_utc(self.expires_at.load(Ordering::SeqCst) as i64)
            .replace('T', " ")
            .replace('Z', " UTC");
        Ok(Reply {
            status: 200,
            headers: vec![("GitHub-Authentication-Token-Expiration".into(), expires)],
            body: r#"{"permissions":{"push":true}}"#.into(),
        })
    }
}

type Msgs = Arc<Mutex<Vec<ClientMsg>>>;

fn sink() -> (Sink, Msgs) {
    let log: Msgs = Arc::default();
    let into = log.clone();
    (Arc::new(move |m| into.lock().unwrap().push(m)), log)
}

fn grants(log: &Msgs) -> Vec<(GithubRepo, String)> {
    log.lock()
        .unwrap()
        .iter()
        .filter_map(|m| match m {
            ClientMsg::GitCredentialGrant { repo, token, .. } => {
                Some((repo.clone(), token.expose().to_string()))
            }
            _ => None,
        })
        .collect()
}

fn revokes(log: &Msgs) -> Vec<GithubRepo> {
    log.lock()
        .unwrap()
        .iter()
        .filter_map(|m| match m {
            ClientMsg::GitCredentialRevoke { repo } => Some(repo.clone()),
            _ => None,
        })
        .collect()
}

struct Rig {
    manager: Manager,
    store: PatStore,
    github: Arc<FakeGithub>,
    clock: Arc<AtomicU64>,
    logs: Arc<Mutex<Vec<String>>>,
    _dir: tempfile::TempDir,
}

impl Rig {
    fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        let store = PatStore::new(dir.path());
        let github = FakeGithub::new(T0 + 30 * DAY);
        let clock = Arc::new(AtomicU64::new(T0));
        let now = clock.clone();
        let source = PatSource::new(
            Some(store.clone()),
            github.clone(),
            Arc::new(move || now.load(Ordering::SeqCst)),
        );
        let logs: Arc<Mutex<Vec<String>>> = Arc::default();
        let into = logs.clone();
        let log: Log = Arc::new(move |line| into.lock().unwrap().push(line.to_string()));
        Self {
            manager: Manager::new(Arc::new(source), log),
            store,
            github,
            clock,
            logs,
            _dir: dir,
        }
    }

    fn now(&self) -> UnixTime {
        self.clock.load(Ordering::SeqCst)
    }

    fn advance(&self, secs: u64) {
        self.clock.fetch_add(secs, Ordering::SeqCst);
    }

    fn store_pat(&self, name: &str, token: &str) {
        let pat = StoredPat {
            token: SecretToken::new(token),
            expires_at: T0 + 30 * DAY,
        };
        self.store.put(&repo(name), &pat).unwrap();
    }

    fn run(&mut self) {
        let now = self.now();
        self.manager.run_due(now);
    }

    fn status(&self, project: &str) -> Option<CredentialStatus> {
        self.manager
            .status(DaemonId(1), Path::new(project), self.now())
    }
}

#[test]
fn a_stored_token_is_granted_for_its_own_repository_only() {
    let mut rig = Rig::new();
    rig.store_pat("a", PAT_A);
    rig.store_pat("b", PAT_B);
    let (send, log) = sink();
    let targets = vec![target("/pa", "a")];
    rig.manager
        .targets(DaemonId(1), send, targets, None, rig.now());
    rig.run();
    assert_eq!(grants(&log), vec![(repo("a"), PAT_A.to_string())]);
    assert!(rig
        .github
        .requests
        .lock()
        .unwrap()
        .iter()
        .all(|u| !u.contains("/o/b")));
    assert!(matches!(
        rig.status("/pa"),
        Some(CredentialStatus::Held { .. })
    ));
}

#[test]
fn with_no_stored_token_the_status_says_not_set_up_and_the_daemon_is_told_to_hold_none() {
    let mut rig = Rig::new();
    rig.store_pat("a", PAT_A);
    let (send, log) = sink();
    let targets = vec![target("/pa", "a"), target("/pb", "b")];
    rig.manager
        .targets(DaemonId(1), send, targets, None, rig.now());
    rig.run();
    assert_eq!(
        rig.status("/pb"),
        Some(CredentialStatus::None(NoCredentialReason::SourceNotSetUp))
    );
    assert_eq!(grants(&log).len(), 1);
    assert_eq!(revokes(&log), vec![repo("b")]);
}

#[test]
fn a_replaced_token_is_granted_without_a_daemon_restart() {
    let mut rig = Rig::new();
    rig.store_pat("a", PAT_A);
    let (send, log) = sink();
    rig.manager
        .targets(DaemonId(1), send, vec![target("/pa", "a")], None, rig.now());
    rig.run();
    rig.store_pat("a", PAT_A2);
    rig.advance(5);
    let now = rig.now();
    rig.manager.refresh(&repo("a"), now);
    rig.run();
    let sent: Vec<String> = grants(&log).into_iter().map(|g| g.1).collect();
    assert_eq!(sent, vec![PAT_A.to_string(), PAT_A2.to_string()]);
}

#[test]
fn a_removed_token_is_withdrawn_from_the_daemon() {
    let mut rig = Rig::new();
    rig.store_pat("a", PAT_A);
    let (send, log) = sink();
    rig.manager
        .targets(DaemonId(1), send, vec![target("/pa", "a")], None, rig.now());
    rig.run();
    rig.store.remove(&repo("a")).unwrap();
    let now = rig.now();
    rig.manager.refresh(&repo("a"), now);
    rig.run();
    assert_eq!(revokes(&log), vec![repo("a")]);
    assert_eq!(
        rig.status("/pa"),
        Some(CredentialStatus::None(NoCredentialReason::SourceNotSetUp))
    );
}

#[test]
fn a_token_removed_while_disconnected_is_revoked_on_the_reconnect() {
    let mut rig = Rig::new();
    rig.store_pat("a", PAT_A);
    let (send, log) = sink();
    rig.manager
        .targets(DaemonId(1), send, vec![target("/pa", "a")], None, rig.now());
    rig.run();
    rig.manager.disconnected(DaemonId(1));
    rig.store.remove(&repo("a")).unwrap();
    let (again, log2) = sink();
    rig.manager.targets(
        DaemonId(1),
        again,
        vec![target("/pa", "a")],
        None,
        rig.now(),
    );
    rig.run();
    assert!(
        revokes(&log).is_empty(),
        "nothing is sent on a lost connection"
    );
    assert_eq!(revokes(&log2), vec![repo("a")]);
    assert!(grants(&log2).is_empty());
}

#[test]
fn at_the_expiry_the_status_is_expired_and_the_daemon_is_told_to_delete_the_token() {
    let mut rig = Rig::new();
    rig.store_pat("a", PAT_A);
    let (send, log) = sink();
    rig.manager
        .targets(DaemonId(1), send, vec![target("/pa", "a")], None, rig.now());
    rig.run();
    assert_eq!(rig.manager.next_wake(rig.now()), Some(T0 + 30 * DAY));
    rig.advance(30 * DAY);
    rig.run();
    assert_eq!(
        rig.status("/pa"),
        Some(CredentialStatus::None(NoCredentialReason::PatExpired))
    );
    assert_eq!(revokes(&log), vec![repo("a")]);
}

#[test]
fn stopping_a_daemon_deletes_its_copies_and_never_reaches_github() {
    let mut rig = Rig::new();
    rig.store_pat("a", PAT_A);
    let (send, log) = sink();
    rig.manager
        .targets(DaemonId(1), send, vec![target("/pa", "a")], None, rig.now());
    rig.run();
    let asked = rig.github.requests();
    rig.manager.stopped(DaemonId(1));
    assert_eq!(revokes(&log), vec![repo("a")]);
    assert_eq!(rig.status("/pa"), None);
    assert_eq!(rig.manager.next_wake(rig.now()), None);
    rig.advance(2 * DAY);
    rig.run();
    assert_eq!(rig.github.requests(), asked);
    assert!(
        rig.store.get(&repo("a")).is_some(),
        "the user's token stays"
    );
}

#[test]
fn an_unreachable_github_keeps_the_held_token() {
    let mut rig = Rig::new();
    rig.store_pat("a", PAT_A);
    let (send, log) = sink();
    rig.manager
        .targets(DaemonId(1), send, vec![target("/pa", "a")], None, rig.now());
    rig.run();
    rig.github.offline.store(true, Ordering::SeqCst);
    let now = rig.now();
    rig.manager.refresh(&repo("a"), now);
    rig.run();
    assert!(revokes(&log).is_empty());
    assert!(matches!(
        rig.status("/pa"),
        Some(CredentialStatus::Held { .. })
    ));
    assert_eq!(rig.logs.lock().unwrap().len(), 1);
}

#[test]
fn a_disconnect_drops_the_schedule_silently_and_a_reconnect_grants_again() {
    let mut rig = Rig::new();
    rig.store_pat("a", PAT_A);
    let (send, log) = sink();
    rig.manager
        .targets(DaemonId(1), send, vec![target("/pa", "a")], None, rig.now());
    rig.run();
    rig.manager.disconnected(DaemonId(1));
    assert_eq!(rig.status("/pa"), None);
    assert_eq!(rig.manager.next_wake(rig.now()), None);
    assert_eq!(
        log.lock().unwrap().len(),
        1,
        "no revoke on a lost connection"
    );
    let (again, log2) = sink();
    rig.advance(30);
    rig.manager.targets(
        DaemonId(1),
        again,
        vec![target("/pa", "a")],
        None,
        rig.now(),
    );
    rig.run();
    assert_eq!(grants(&log2), vec![(repo("a"), PAT_A.to_string())]);
}

#[test]
fn no_log_line_or_message_debug_carries_the_token() {
    let mut rig = Rig::new();
    rig.store_pat("a", PAT_A);
    rig.github.offline.store(true, Ordering::SeqCst);
    let (send, log) = sink();
    rig.manager
        .targets(DaemonId(1), send, vec![target("/pa", "a")], None, rig.now());
    rig.run();
    rig.github.offline.store(false, Ordering::SeqCst);
    let now = rig.now();
    rig.manager.refresh(&repo("a"), now);
    rig.run();
    assert!(!grants(&log).is_empty());
    let lines = rig.logs.lock().unwrap().clone();
    assert!(!lines.is_empty(), "the failed first check is logged");
    for line in lines {
        assert!(!line.contains(PAT_A), "{line}");
    }
    let shown = format!("{:?}", log.lock().unwrap());
    assert!(!shown.contains(PAT_A), "{shown}");
}

#[test]
fn issuing_writes_the_refreshed_expiry_back_and_revoking_is_never_a_request() {
    let rig = Rig::new();
    rig.store_pat("a", PAT_A);
    rig.github.expires_at.store(T0 + 10 * DAY, Ordering::SeqCst);
    let now = rig.clock.clone();
    let source = PatSource::new(
        Some(rig.store.clone()),
        rig.github.clone(),
        Arc::new(move || now.load(Ordering::SeqCst)),
    );
    let credential = source.issue(&repo("a"), Grant::default()).unwrap();
    assert_eq!(credential.expires_at(), T0 + 10 * DAY);
    assert_eq!(rig.store.get(&repo("a")).unwrap().expires_at, T0 + 10 * DAY);
    let asked = rig.github.requests();
    source.revoke(&credential).unwrap();
    assert_eq!(rig.github.requests(), asked);
}

#[test]
fn a_refused_token_is_not_stored_and_an_accepted_one_is() {
    let rig = Rig::new();
    let bad = check_and_store(
        &*rig.github,
        Some(&rig.store),
        &repo("a"),
        SecretToken::new("ghp_classic"),
        T0,
    );
    assert!(bad.is_err());
    assert!(rig.store.get(&repo("a")).is_none());
    let good = check_and_store(
        &*rig.github,
        Some(&rig.store),
        &repo("a"),
        SecretToken::new(PAT_A),
        T0,
    );
    assert_eq!(good, Ok(T0 + 30 * DAY));
    assert_eq!(rig.store.get(&repo("a")).unwrap().token.expose(), PAT_A);
}

#[test]
fn ureq_transport_sends_the_contract_headers_and_reads_the_reply_headers() {
    use std::io::{Read, Write};
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("http://{}/repos/o/a", listener.local_addr().unwrap());
    let server = std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        let mut raw = Vec::new();
        let mut buf = [0u8; 1024];
        while !String::from_utf8_lossy(&raw).contains("\r\n\r\n") {
            let n = stream.read(&mut buf).unwrap();
            assert!(n > 0, "closed before the headers");
            raw.extend_from_slice(&buf[..n]);
        }
        let body = "{\"message\":\"Not Found\"}";
        write!(
            stream,
            "HTTP/1.1 404 Not Found\r\nX-Test: yes\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        )
        .unwrap();
        String::from_utf8(raw).unwrap()
    });
    let reply = UreqTransport::default()
        .get(&url, &SecretToken::new(PAT_A))
        .unwrap();
    assert_eq!((reply.status, reply.header("x-test")), (404, Some("yes")));
    assert_eq!(reply.body, "{\"message\":\"Not Found\"}");
    let seen = server.join().unwrap().to_ascii_lowercase();
    assert!(seen.starts_with("get /repos/o/a http/1.1"));
    for header in [
        &format!("authorization: bearer {}", PAT_A.to_ascii_lowercase()),
        "accept: application/vnd.github+json",
        "x-github-api-version: 2022-11-28",
        "user-agent: micold-ai-ide",
    ] {
        assert!(seen.contains(header), "missing {header} in {seen}");
    }
}

#[test]
fn a_transport_failure_is_short_and_carries_no_token() {
    let closed = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("http://{}/x", closed.local_addr().unwrap());
    drop(closed);
    let error = UreqTransport::default()
        .get(&url, &SecretToken::new(PAT_A))
        .unwrap_err();
    assert!(
        !error.contains(PAT_A) && error.chars().count() <= 80,
        "{error}"
    );
}
