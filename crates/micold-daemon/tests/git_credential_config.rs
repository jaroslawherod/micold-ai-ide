//! #1530: `micold-daemon git-credential get` chooses the token from the configuration file
//! (`MICOLD_DAEMON_CONFIG`) on every call, learns each token's reach from a fake forge and caches it
//! by fingerprint, and hands over to the session's own helpers when no token applies.

use std::io::{Read, Write};
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
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
            let request_line = head.lines().next().unwrap_or("");
            if let Some(repo) = git_advertisement_repo(request_line) {
                let basic = head
                    .lines()
                    .find_map(|l| {
                        l.strip_prefix("authorization: Basic ")
                            .or_else(|| l.strip_prefix("Authorization: Basic "))
                    })
                    .unwrap_or("")
                    .trim();
                let authorised = reach.iter().any(|(token, repos)| {
                    let user = if *token == "from-keychain" {
                        "me"
                    } else {
                        "x-access-token"
                    };
                    base64(&format!("{user}:{token}")) == basic
                        && repos.iter().any(|r| r.eq_ignore_ascii_case(&repo))
                });
                let reply = if authorised {
                    let refs = format!("{SHA} refs/heads/main\0multi_ack\n");
                    let service = "# service=git-upload-pack\n";
                    let body = format!(
                        "{:04x}{service}0000{:04x}{refs}0000",
                        service.len() + 4,
                        refs.len() + 4
                    );
                    format!(
                        "HTTP/1.1 200 OK\r\nContent-Type: application/x-git-upload-pack-advertisement\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                        body.len()
                    )
                } else {
                    "HTTP/1.1 401 Unauthorized\r\nWWW-Authenticate: Basic realm=\"forge\"\r\nContent-Length: 0\r\nConnection: close\r\n\r\n".to_string()
                };
                let _ = stream.write_all(reply.as_bytes());
                continue;
            }
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

/// The commit every authorised `git ls-remote` of the fake forge lists.
const SHA: &str = "0123456789abcdef0123456789abcdef01234567";

/// `owner/repo` of a git smart-HTTP `info/refs` request line, `None` for any other request.
fn git_advertisement_repo(request_line: &str) -> Option<String> {
    let target = request_line.split(' ').nth(1)?;
    let repo = target.split_once(".git/info/refs")?.0;
    Some(repo.trim_start_matches('/').to_string())
}

fn base64(text: &str) -> String {
    use base64::Engine;
    base64::engine::general_purpose::STANDARD.encode(text)
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

    /// A host git configuration with a helper that records that it ran in `helper-ran` and
    /// answers `from-keychain`; returns the helper as git names it.
    fn plant_host_helper(&self) -> String {
        self.token(
            "planted.sh",
            &format!(
                "cat >/dev/null\ntouch '{}'\nprintf 'username=me\\npassword=from-keychain\\n'\n",
                self.path("helper-ran").display()
            ),
        );
        let helper = format!("!sh {}", self.path("planted.sh").display());
        std::fs::write(
            self.path("gitconfig"),
            format!("[credential]\n\thelper = {helper}\n"),
        )
        .unwrap();
        helper
    }

    /// `git ls-remote` of `path` on the fake forge under the environment `session_git_env` builds
    /// for `config`, over the planted host configuration.
    fn ls_remote(&self, config: &Path, path: &str, isolate: bool, helper: &str) -> Output {
        use micold_core::scoped_credential::git_env::{session_git_env, SessionGit};
        let config_text = config.to_string_lossy();
        let fallback = [helper.to_string()];
        let env = session_git_env(&SessionGit {
            bases: std::slice::from_ref(&self.base),
            daemon: env!("CARGO_BIN_EXE_micold-daemon"),
            config: &config_text,
            isolate,
            ssh_to_https: false,
            fallback: &fallback,
        });
        Command::new("git")
            .args(["ls-remote", &format!("{}/{path}.git", self.base)])
            .envs(env)
            .env("HOME", self.path("data"))
            .env("GIT_CONFIG_GLOBAL", self.path("gitconfig"))
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("GIT_TERMINAL_PROMPT", "0")
            .output()
            .unwrap()
    }

    fn helper_ran(&self) -> bool {
        self.path("helper-ran").exists()
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

/// A token with no filter that reaches `Acme/App`, and the daemon file naming it.
fn unfiltered_token() -> (Forge, Setup, PathBuf) {
    let forge = forge(&[("secret-token", &["Acme/App"])]);
    let s = Setup::new(&forge);
    s.token("any.tok", "secret-token\n");
    let config = s.config(&[("any", &[])]);
    (forge, s, config)
}

#[test]
fn an_unfiltered_token_answers_for_what_it_reaches() {
    let (_forge, s, config) = unfiltered_token();
    assert_eq!(
        password(&s.get(&config, "acme/app", None)),
        Some("secret-token")
    );
}

#[test]
fn the_reach_of_an_unfiltered_token_is_asked_once() {
    let (forge, s, config) = unfiltered_token();
    s.get(&config, "acme/app", None);
    s.get(&config, "acme/app", None);
    assert_eq!(
        forge.requests.load(Ordering::SeqCst),
        1,
        "cached after the first"
    );
}

#[test]
fn the_reach_cache_is_kept_by_fingerprint_and_holds_no_secret() {
    let (_forge, s, config) = unfiltered_token();
    s.get(&config, "acme/app", None);
    let cache = std::fs::read_to_string(s.path("data/github-reach.json")).unwrap();
    assert!(cache.contains("acme/app"));
    assert!(!cache.contains("secret-token"), "the cache holds no secret");
}

#[test]
fn a_repository_outside_the_reach_is_asked_again_at_most_once_a_minute() {
    let (forge, s, config) = unfiltered_token();
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

/// What `git ls-remote` printed, for the assertions below.
fn listing(out: &Output) -> String {
    String::from_utf8_lossy(&out.stdout).into_owned()
}

#[test]
fn git_with_the_session_environment_authenticates_with_the_chosen_token() {
    let forge = forge(&[("tok", &["acme/app"])]);
    let s = Setup::new(&forge);
    let helper = s.plant_host_helper();
    s.token("any.tok", "tok\n");
    let config = s.config(&[("any", &[])]);
    let out = s.ls_remote(&config, "acme/app", false, &helper);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert_eq!(listing(&out), format!("{SHA}\trefs/heads/main\n"));
    assert!(
        !s.helper_ran(),
        "the host helper is not asked when a token applies"
    );
}

#[test]
fn git_falls_back_to_the_host_helper_for_a_repository_no_token_reaches() {
    let forge = forge(&[("tok", &["acme/app"]), ("from-keychain", &["other/app"])]);
    let s = Setup::new(&forge);
    let helper = s.plant_host_helper();
    s.token("any.tok", "tok\n");
    let config = s.config(&[("any", &[])]);
    let out = s.ls_remote(&config, "other/app", false, &helper);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert_eq!(listing(&out), format!("{SHA}\trefs/heads/main\n"));
    assert!(s.helper_ran(), "the host helper answered");
}

/// Over a host with a planted helper and a token that reaches `acme/app` only, for a sandbox
/// session (`isolate`): returns the setup and the daemon file.
fn sandbox_over_a_planted_helper() -> (Forge, Setup, PathBuf, String) {
    let forge = forge(&[("tok", &["acme/app"]), ("from-keychain", &["other/app"])]);
    let s = Setup::new(&forge);
    let helper = s.plant_host_helper();
    s.token("any.tok", "tok\n");
    let config = s.config(&[("any", &[])]);
    (forge, s, config, helper)
}

#[test]
fn a_sandbox_session_is_refused_where_only_the_planted_host_helper_would_answer() {
    let (_forge, s, config, helper) = sandbox_over_a_planted_helper();
    let refused = s.ls_remote(&config, "other/app", true, &helper);
    assert!(
        !refused.status.success(),
        "no token reaches it and no host helper may answer"
    );
    assert_eq!(listing(&refused), "");
    assert!(!s.helper_ran(), "a sandbox clears the inherited helper");
}

#[test]
fn a_sandbox_session_is_served_by_the_chosen_token_without_the_planted_host_helper() {
    let (_forge, s, config, helper) = sandbox_over_a_planted_helper();
    let served = s.ls_remote(&config, "acme/app", true, &helper);
    assert!(
        served.status.success(),
        "{}",
        String::from_utf8_lossy(&served.stderr)
    );
    assert_eq!(listing(&served), format!("{SHA}\trefs/heads/main\n"));
    assert!(!s.helper_ran(), "a sandbox clears the inherited helper");
}
