//! #1530: `micold-daemon git-credential get` chooses the token from the configuration file
//! (`MICOLD_DAEMON_CONFIG`) on every call, learns each token's reach from a fake forge and caches it
//! by fingerprint, and hands over to the session's own helpers when no token applies.

use std::io::{Read, Write};
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

/// A loopback forge: `GET /user/repos` lists what the bearer token reaches.
struct Forge {
    base: String,
    requests: Arc<AtomicUsize>,
}

fn forge(reach: &'static [(&'static str, &'static [&'static str])]) -> Forge {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    let requests = Arc::new(AtomicUsize::new(0));
    let counted = requests.clone();
    std::thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(mut stream) = stream else { continue };
            let mut head = Vec::new();
            let mut byte = [0u8; 1];
            while !head.ends_with(b"\r\n\r\n") && stream.read(&mut byte).unwrap_or(0) == 1 {
                head.push(byte[0]);
            }
            let head = String::from_utf8_lossy(&head).into_owned();
            counted.fetch_add(1, Ordering::SeqCst);
            let bearer = head
                .lines()
                .find_map(|l| l.strip_prefix("authorization: Bearer "))
                .or_else(|| {
                    head.lines()
                        .find_map(|l| l.strip_prefix("Authorization: Bearer "))
                })
                .unwrap_or("")
                .trim()
                .to_string();
            let page1 = head.contains("page=1");
            let repos = reach
                .iter()
                .find(|(token, _)| *token == bearer)
                .map(|(_, repos)| *repos)
                .unwrap_or(&[]);
            let items: Vec<String> = repos
                .iter()
                .filter(|_| page1)
                .map(|r| format!(r#"{{"full_name":"{r}"}}"#))
                .collect();
            let body = format!("[{}]", items.join(","));
            let _ = write!(
                stream,
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            );
        }
    });
    Forge { base, requests }
}

struct Setup {
    dir: tempfile::TempDir,
    base: String,
}

impl Setup {
    fn new(forge: &Forge) -> Self {
        Self {
            dir: tempfile::tempdir().unwrap(),
            base: forge.base.clone(),
        }
    }

    fn path(&self, name: &str) -> PathBuf {
        self.dir.path().join(name)
    }

    fn token(&self, name: &str, content: &str) {
        std::fs::write(self.path(name), content).unwrap();
    }

    /// A host daemon's file with `tokens` as `(name, repos filter)`, each reading `<name>.tok`.
    fn config(&self, tokens: &[(&str, &[&str])]) -> PathBuf {
        let mut text = format!(
            "schema = 1\n\n[daemon]\nname = \"t\"\nkind = \"host\"\ndata_dir = \"{}\"\n",
            self.path("data").display()
        );
        for (name, repos) in tokens {
            text.push_str(&format!(
                "\n[[github.tokens]]\nname = \"{name}\"\ntoken_file = \"{}\"\nhost = \"{}\"\n",
                self.path(&format!("{name}.tok")).display(),
                self.base
            ));
            if !repos.is_empty() {
                let list: Vec<String> = repos.iter().map(|r| format!("\"{r}\"")).collect();
                text.push_str(&format!("repos = [{}]\n", list.join(", ")));
            }
        }
        let path = self.path("daemon.toml");
        std::fs::create_dir_all(self.path("data")).unwrap();
        std::fs::write(&path, text).unwrap();
        path
    }

    /// `git-credential get` for `path` on the fake forge.
    fn get(&self, config: &Path, path: &str, fallback: Option<&str>) -> String {
        let scheme_host = self.base.replace("http://", "");
        let mut command = Command::new(env!("CARGO_BIN_EXE_micold-daemon"));
        command
            .args(["git-credential", "get"])
            .env("MICOLD_DAEMON_CONFIG", config)
            .env_remove("MICOLD_GIT_FALLBACK_HELPERS")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped());
        if let Some(helpers) = fallback {
            command.env("MICOLD_GIT_FALLBACK_HELPERS", helpers);
        }
        let mut child = command.spawn().unwrap();
        child
            .stdin
            .take()
            .unwrap()
            .write_all(format!("protocol=http\nhost={scheme_host}\npath={path}\n\n").as_bytes())
            .unwrap();
        let out = child.wait_with_output().unwrap();
        assert!(out.status.success());
        String::from_utf8(out.stdout).unwrap()
    }
}

fn password(answer: &str) -> Option<&str> {
    answer.lines().find_map(|l| l.strip_prefix("password="))
}

#[test]
fn the_helper_answers_from_the_token_file_and_a_replaced_file_is_used_at_once() {
    let forge = forge(&[]);
    let s = Setup::new(&forge);
    s.token("a.tok", "tok-1\n");
    let config = s.config(&[("a", &["acme/*"])]);
    let first = s.get(&config, "acme/app.git", None);
    assert_eq!(password(&first), Some("tok-1"));
    assert!(first.contains("username=x-access-token"));
    s.token("a.tok", "tok-2\n");
    assert_eq!(password(&s.get(&config, "acme/app", None)), Some("tok-2"));
    // A repository the filter does not name, and a filtered token needs no reach request.
    assert_eq!(s.get(&config, "other/app", None), "");
    assert_eq!(forge.requests.load(Ordering::SeqCst), 0);
}

#[test]
fn the_most_specific_filter_wins_and_ties_go_to_the_first_in_the_file() {
    let forge = forge(&[("any", &["acme/app"])]);
    let s = Setup::new(&forge);
    for name in ["any", "owner", "exact", "twin"] {
        s.token(&format!("{name}.tok"), &format!("{name}\n"));
    }
    let config = s.config(&[
        ("any", &[]),
        ("owner", &["acme/*"]),
        ("exact", &["acme/app"]),
        ("twin", &["acme/*"]),
    ]);
    assert_eq!(password(&s.get(&config, "acme/app", None)), Some("exact"));
    // No exact filter for this one: `owner` and `twin` tie, the first in the file wins.
    assert_eq!(password(&s.get(&config, "acme/lib", None)), Some("owner"));
}

#[test]
fn an_unfiltered_token_is_used_for_what_it_reaches_and_the_reach_is_cached_by_fingerprint() {
    let forge = forge(&[("secret-token", &["Acme/App"])]);
    let s = Setup::new(&forge);
    s.token("any.tok", "secret-token\n");
    let config = s.config(&[("any", &[])]);

    assert_eq!(
        password(&s.get(&config, "acme/app", None)),
        Some("secret-token")
    );
    assert_eq!(
        password(&s.get(&config, "acme/app", None)),
        Some("secret-token")
    );
    assert_eq!(
        forge.requests.load(Ordering::SeqCst),
        1,
        "cached after the first"
    );

    let cache = std::fs::read_to_string(s.path("data/github-reach.json")).unwrap();
    assert!(cache.contains("acme/app"));
    assert!(!cache.contains("secret-token"), "the cache holds no secret");

    // A repository outside the reach is a miss: asked again, but at most once a minute.
    assert_eq!(s.get(&config, "acme/other", None), "");
    let after_miss = forge.requests.load(Ordering::SeqCst);
    assert_eq!(s.get(&config, "acme/other", None), "");
    assert_eq!(forge.requests.load(Ordering::SeqCst), after_miss);
}

#[test]
fn an_expired_token_is_skipped() {
    let forge = forge(&[]);
    let s = Setup::new(&forge);
    s.token("old.tok", "old\nexpires_at=1\n");
    s.token("new.tok", "new\n");
    let config = s.config(&[("old", &["acme/*"]), ("new", &["acme/*"])]);
    assert_eq!(password(&s.get(&config, "acme/app", None)), Some("new"));
}

#[test]
fn with_no_matching_token_the_sessions_own_helpers_answer() {
    let forge = forge(&[]);
    let s = Setup::new(&forge);
    s.token("a.tok", "tok\n");
    let config = s.config(&[("a", &["acme/*"])]);
    let helpers = "!cat >/dev/null; printf 'username=me\\npassword=from-keychain\\n'; true";
    assert_eq!(
        password(&s.get(&config, "other/app", Some(helpers))),
        Some("from-keychain")
    );
    assert_eq!(
        password(&s.get(&config, "acme/app", Some(helpers))),
        Some("tok")
    );
    assert_eq!(s.get(&config, "other/app", None), "");
}
