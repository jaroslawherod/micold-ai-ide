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
    agent: ureq::Agent,
}

impl Default for UreqTransport {
    fn default() -> Self {
        let agent = ureq::Agent::config_builder()
            .timeout_global(Some(std::time::Duration::from_secs(10)))
            .max_redirects(0)
            .http_status_as_error(false)
            .build()
            .into();
        Self { agent }
    }
}

impl Transport for UreqTransport {
    fn get(&self, url: &str, token: &SecretToken) -> Result<Reply, String> {
        let failure = |e: ureq::Error| match e {
            ureq::Error::Timeout(_) => "timed out".to_string(),
            other => micold_core::scoped_credential::pat::short_error(&other.to_string(), token),
        };
        let mut response = self
            .agent
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
}
