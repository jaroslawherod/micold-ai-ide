//! What `micold-daemon git-credential get` answers from the configuration (#1530, FR-008, FR-010,
//! research D6, D7). Stateless: the file, the token files and the reach cache are read on every
//! call, so a replaced token is used by the very next git command (SC-004).

use std::io::{Read, Write};
use std::path::Path;
use std::process::{Command, Stdio};

use micold_core::daemon_config::matching::{choose, parse_token_file, Candidate};
use micold_core::daemon_config::{DaemonConfig, GithubToken, DEFAULT_FORGE_HOST};
use micold_core::github::GithubRepo;
use micold_core::process::no_window;
use micold_core::scoped_credential::helper::{
    repo_key, split_base, CredentialRequest, CREDENTIAL_USERNAME,
};
use micold_core::scoped_credential::pat::{Reply, Transport};
use micold_core::scoped_credential::reach::{
    api_base, discover, fingerprint, forge_base, should_fetch, ReachCache, ReachEntry,
};
use micold_core::scoped_credential::{SecretToken, UnixTime};

/// The file the reach cache lives in, under the data directory.
pub const REACH_FILE: &str = "github-reach.json";

/// GitHub's REST API over HTTPS (`ureq` 3): a 10 second budget, no redirect, a non-2xx status is an
/// `Ok` reply. The request shape is the client's (`micold_client::pat_source::UreqTransport`).
pub struct UreqTransport {
    egress: crate::session_extras::Egress,
}

impl Default for UreqTransport {
    /// The CA bundle and proxy the daemon's configuration names (#1536, FR-037).
    fn default() -> Self {
        Self {
            egress: crate::session_extras::Egress::load(
                crate::settings::current().config_path.as_deref(),
            ),
        }
    }
}

impl UreqTransport {
    fn agent_for(&self, url: &str) -> ureq::Agent {
        let host = url
            .split_once("://")
            .map_or(url, |(_, rest)| rest)
            .split(['/', ':'])
            .next()
            .unwrap_or_default();
        let base = ureq::Agent::config_builder()
            .timeout_global(Some(std::time::Duration::from_secs(10)))
            .max_redirects(0)
            .http_status_as_error(false);
        self.egress.agent(base, host)
    }
}

impl Transport for UreqTransport {
    fn get(&self, url: &str, token: &SecretToken) -> Result<Reply, String> {
        let failure = |e: ureq::Error| match e {
            ureq::Error::Timeout(_) => "timed out".to_string(),
            other => micold_core::scoped_credential::pat::short_error(&other.to_string(), token),
        };
        let mut response = self
            .agent_for(url)
            .get(url)
            .header("Authorization", format!("Bearer {}", token.expose()))
            .header("Accept", "application/vnd.github+json")
            .header("X-GitHub-Api-Version", "2022-11-28")
            .header("User-Agent", "micold-ai-ide")
            .call()
            .map_err(failure)?;
        let status = response.status().as_u16();
        let body = response.body_mut().read_to_string().map_err(failure)?;
        Ok(Reply {
            status,
            headers: Vec::new(),
            body,
        })
    }
}

/// The forge base a token is for.
fn base_of(token: &GithubToken) -> String {
    forge_base(token.host.as_deref().unwrap_or(DEFAULT_FORGE_HOST))
}

/// Whether the request is for `base`: same scheme, same host (and port), ignoring case.
fn is_for(base: &str, request: &CredentialRequest) -> bool {
    let Some((scheme, host)) = split_base(base) else {
        return false;
    };
    request.protocol.as_deref() == Some(scheme)
        && request
            .host
            .as_deref()
            .is_some_and(|h| h.eq_ignore_ascii_case(host))
}

/// The token to answer `request` with, as git's `username=`/`password=` lines; `None` when no
/// configured token applies. `cache` is where the reach is kept (none: not kept).
pub fn answer(
    config: &DaemonConfig,
    cache: Option<&Path>,
    request: &CredentialRequest,
    now: UnixTime,
    transport: &dyn Transport,
) -> Option<String> {
    let (owner, name) = repo_key(request.path.as_deref()?)?;
    let repo = GithubRepo::from_remote_url(&format!("https://github.com/{owner}/{name}"))?;
    let key = format!("{owner}/{name}");

    // The tokens of this forge, read now: (index in the file, token, expiry).
    let mut found: Vec<(&GithubToken, String, bool)> = Vec::new();
    for token in &config.github.tokens {
        if !is_for(&base_of(token), request) {
            continue;
        }
        let Some((secret, expires_at)) = token
            .token_file
            .read()
            .ok()
            .and_then(|s| parse_token_file(s.expose()))
        else {
            continue;
        };
        found.push((token, secret, expires_at.is_some_and(|at| at <= now)));
    }
    if found.is_empty() {
        return None;
    }

    let mut store = cache.map(ReachCache::load).unwrap_or_default();
    let mut changed = false;
    let mut reach: Vec<Option<Vec<String>>> = Vec::new();
    for (token, secret, expired) in &found {
        // A filtered token is used for what its filter names; only an unfiltered one needs the
        // reach, and only while it can still be used.
        let wanted = token.repos.as_deref().is_none_or(<[_]>::is_empty) && !expired;
        if !wanted {
            reach.push(None);
            continue;
        }
        let id = fingerprint(secret);
        let entry = store.entry(&id).cloned();
        let mut entry = match entry {
            Some(entry) if !should_fetch(Some(&entry), &key, now) => entry,
            known => {
                let base = api_base(token.host.as_deref().unwrap_or(DEFAULT_FORGE_HOST));
                let mut entry = known.unwrap_or(ReachEntry {
                    fingerprint: id.clone(),
                    ..ReachEntry::default()
                });
                // A failed request keeps the last answer and still counts as an attempt, so an
                // offline machine asks once a minute, not on every git command.
                entry.last_miss_fetch_at = now;
                if let Ok(repos) = discover(transport, &base, &SecretToken::new(secret)) {
                    entry.repos = repos;
                    entry.fetched_at = now;
                }
                changed = true;
                entry
            }
        };
        entry.fingerprint = id;
        reach.push(Some(entry.repos.clone()));
        store.put(entry);
    }
    if changed {
        if let Some(path) = cache {
            store.retain(
                &config
                    .github
                    .tokens
                    .iter()
                    .filter_map(|t| t.token_file.read().ok())
                    .filter_map(|s| parse_token_file(s.expose()))
                    .map(|(secret, _)| fingerprint(&secret))
                    .collect::<Vec<_>>(),
            );
            let _ = store.save(path);
        }
    }

    let candidates: Vec<Candidate<'_>> = found
        .iter()
        .zip(&reach)
        .map(|((token, _, expired), reach)| Candidate {
            name: &token.name,
            repos: token.repos.as_deref().unwrap_or(&[]),
            expired: *expired,
            reach: reach.as_deref(),
        })
        .collect();
    let used = choose(&candidates, &repo).used?;
    Some(format!(
        "username={CREDENTIAL_USERNAME}\npassword={}\n",
        found[used].1
    ))
}

/// The shell command for one helper of git's configuration, with the verb: `!cmd` is a shell
/// command, an absolute path is run as it is, and a bare name is `git credential-<name>`.
fn helper_command(helper: &str) -> String {
    if let Some(command) = helper.strip_prefix('!') {
        format!("{command} get")
    } else if helper.starts_with('/') {
        format!("'{}' get", helper.replace('\'', r"'\''"))
    } else {
        format!("git credential-{helper} get")
    }
}

/// The helpers the session's git named before the reset, from [`FALLBACK_ENV`].
pub fn fallback_helpers(value: Option<&str>) -> Vec<String> {
    value
        .unwrap_or_default()
        .lines()
        .filter(|l| !l.is_empty())
        .map(str::to_string)
        .collect()
}

/// Asks each helper in turn with the request git made; the first that answers with a password
/// wins. What a helper prints is passed through as it is.
pub fn run_fallback(helpers: &[String], request: &str) -> String {
    for helper in helpers {
        let mut child = no_window(&mut Command::new("sh"))
            .arg("-c")
            .arg(helper_command(helper))
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn();
        let Ok(child) = child.as_mut() else {
            continue;
        };
        if let Some(mut stdin) = child.stdin.take() {
            let _ = stdin.write_all(request.as_bytes());
            let _ = stdin.write_all(b"\n");
        }
        let mut out = String::new();
        if let Some(mut stdout) = child.stdout.take() {
            let _ = stdout.read_to_string(&mut out);
        }
        let _ = child.wait();
        if out.lines().any(|l| l.starts_with("password=")) {
            return out;
        }
    }
    String::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_helper_is_run_the_way_git_runs_it() {
        assert_eq!(
            helper_command("osxkeychain"),
            "git credential-osxkeychain get"
        );
        assert_eq!(
            helper_command("!gh auth git-credential"),
            "gh auth git-credential get"
        );
        assert_eq!(helper_command("/opt/h"), "'/opt/h' get");
    }

    #[test]
    fn the_fallback_list_drops_empty_lines() {
        assert_eq!(fallback_helpers(Some("a\n\n!b c\n")), ["a", "!b c"]);
        assert!(fallback_helpers(None).is_empty());
    }

    #[test]
    fn a_fallback_helper_that_answers_wins_and_one_that_does_not_is_passed() {
        let request = "protocol=https\nhost=github.com\n";
        let helpers = [
            "!echo nothing >/dev/null; true".to_string(),
            "!printf 'username=u\\npassword=p\\n'; true".to_string(),
        ];
        assert_eq!(run_fallback(&helpers, request), "username=u\npassword=p\n");
        assert_eq!(run_fallback(&helpers[..1], request), "");
    }

    use micold_core::scoped_credential::reach::REFRESH_SECS;
    use std::collections::VecDeque;
    use std::sync::Mutex;

    /// A forge that answers `/user/repos` from a script (`Err`: unreachable) and counts the asks.
    struct Forge(Mutex<VecDeque<Result<Vec<&'static str>, String>>>);

    impl Forge {
        fn new(answers: Vec<Result<Vec<&'static str>, String>>) -> Self {
            Self(Mutex::new(answers.into()))
        }
        fn left(&self) -> usize {
            self.0.lock().unwrap().len()
        }
    }

    impl Transport for Forge {
        fn get(&self, _: &str, _: &SecretToken) -> Result<Reply, String> {
            let repos = self
                .0
                .lock()
                .unwrap()
                .pop_front()
                .expect("an unscripted ask")?;
            let items: Vec<String> = repos
                .iter()
                .map(|r| format!(r#"{{"full_name":"{r}"}}"#))
                .collect();
            Ok(Reply {
                status: 200,
                headers: Vec::new(),
                body: format!("[{}]", items.join(",")),
            })
        }
    }

    /// One unfiltered token on `http://forge.test`, read from `any.tok`.
    struct Host {
        dir: tempfile::TempDir,
        config: DaemonConfig,
    }

    impl Host {
        fn new(token: &str) -> Self {
            let dir = tempfile::tempdir().unwrap();
            std::fs::write(dir.path().join("any.tok"), token).unwrap();
            std::fs::write(
                dir.path().join("daemon.toml"),
                format!(
                    "schema = 1\n\n[daemon]\nname = \"t\"\nkind = \"host\"\ndata_dir = \"{d}\"\n\n[[github.tokens]]\nname = \"any\"\ntoken_file = \"{d}/any.tok\"\nhost = \"http://forge.test\"\n",
                    d = dir.path().display()
                ),
            )
            .unwrap();
            let config = crate::config_load::load_file(&dir.path().join("daemon.toml"))
                .unwrap()
                .config;
            Self { dir, config }
        }

        fn replace_token(&self, token: &str) {
            std::fs::write(self.dir.path().join("any.tok"), token).unwrap();
        }

        fn cache_file(&self) -> std::path::PathBuf {
            self.dir.path().join(REACH_FILE)
        }

        fn cache(&self) -> ReachCache {
            ReachCache::load(&self.cache_file())
        }

        /// The password `answer` gives for `repo` at `now`.
        fn password(&self, repo: &str, now: UnixTime, forge: &Forge) -> Option<String> {
            let request = CredentialRequest {
                protocol: Some("http".into()),
                host: Some("forge.test".into()),
                path: Some(repo.into()),
            };
            answer(&self.config, Some(&self.cache_file()), &request, now, forge).and_then(|a| {
                a.lines()
                    .find_map(|l| l.strip_prefix("password=").map(String::from))
            })
        }
    }

    #[test]
    fn an_entry_older_than_15_minutes_is_asked_for_again_through_answer() {
        let host = Host::new("tok\n");
        let forge = Forge::new(vec![Ok(vec!["acme/app"]), Ok(vec!["acme/app", "acme/new"])]);
        assert_eq!(
            host.password("acme/app", 1000, &forge).as_deref(),
            Some("tok")
        );
        assert_eq!(forge.left(), 1);

        host.password("acme/app", 1000 + REFRESH_SECS - 1, &forge);
        assert_eq!(forge.left(), 1, "a fresh entry is not asked for again");

        host.password("acme/app", 1000 + REFRESH_SECS, &forge);
        assert_eq!(forge.left(), 0, "a stale entry is refreshed");
        let entry = host.cache().entries.remove(0);
        assert_eq!(entry.repos, ["acme/app", "acme/new"]);
        assert_eq!(entry.fetched_at, 1000 + REFRESH_SECS);
    }

    #[test]
    fn a_failed_request_keeps_the_last_answer_and_counts_as_an_attempt() {
        let host = Host::new("tok\n");
        let forge = Forge::new(vec![Ok(vec!["acme/app"]), Err("offline".into())]);
        host.password("acme/app", 1000, &forge);
        let later = 1000 + REFRESH_SECS;
        assert_eq!(
            host.password("acme/app", later, &forge).as_deref(),
            Some("tok"),
            "offline, the cached reach still serves the token"
        );
        let entry = host.cache().entries.remove(0);
        assert_eq!(entry.repos, ["acme/app"]);
        assert_eq!(entry.fetched_at, 1000, "a failure is not a refresh");
        assert_eq!(entry.last_miss_fetch_at, later, "but it is an attempt");
    }

    #[test]
    fn a_replaced_unfiltered_token_gets_its_own_entry_and_the_old_one_is_pruned() {
        let host = Host::new("tok-1\n");
        let forge = Forge::new(vec![Ok(vec!["acme/app"]), Ok(vec!["acme/other"])]);
        host.password("acme/app", 1000, &forge);
        let old = fingerprint("tok-1");
        assert_eq!(host.cache().entries[0].fingerprint, old);

        host.replace_token("tok-2\n");
        assert_eq!(
            host.password("acme/other", 1001, &forge).as_deref(),
            Some("tok-2")
        );
        let entries = host.cache().entries;
        assert_eq!(entries.len(), 1, "the replaced token's entry is pruned");
        assert_eq!(entries[0].fingerprint, fingerprint("tok-2"));
        assert_eq!(entries[0].repos, ["acme/other"]);
    }
}
