//! What each configured token reaches, as GitHub reports it, kept in `<data_dir>/github-reach.json`
//! by the token's fingerprint (#1530, FR-008, research D6). The cache holds no secret: the
//! fingerprint is the first 16 hex digits of the token's SHA-256, so a replaced token gets its own
//! entry. Refreshed when a token has no entry, every 15 minutes, and on a miss at most once a
//! minute. Offline, the last answer stays in use.

use std::io::Write;
use std::path::Path;

use serde::{Deserialize, Serialize};

use super::pat::Transport;
use super::{SecretToken, UnixTime};
use crate::protocol::hashing::sha256_hex;

/// How long an answer is trusted before it is asked for again.
pub const REFRESH_SECS: u64 = 15 * 60;
/// The least time between two requests made because a repository was not in the answer.
pub const MISS_SECS: u64 = 60;
/// A listing is cut off after this many pages of 100.
const MAX_PAGES: usize = 30;
const PAGE: usize = 100;

/// The first 16 hex digits of the token's SHA-256.
pub fn fingerprint(token: &str) -> String {
    sha256_hex(token.as_bytes())[..16].to_string()
}

/// One token's reach.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct ReachEntry {
    pub fingerprint: String,
    /// `owner/repo`, lower case.
    pub repos: Vec<String>,
    pub fetched_at: UnixTime,
    /// The last time a request was made (or tried) because a repository was missing.
    pub last_miss_fetch_at: UnixTime,
}

/// The whole cache file.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct ReachCache {
    #[serde(default)]
    pub entries: Vec<ReachEntry>,
}

impl ReachCache {
    /// The cache at `path`; a missing or damaged file is an empty cache (it is only advisory).
    pub fn load(path: &Path) -> Self {
        std::fs::read_to_string(path)
            .ok()
            .and_then(|text| serde_json::from_str(&text).ok())
            .unwrap_or_default()
    }

    /// Writes the cache atomically, mode `0600`.
    pub fn save(&self, path: &Path) -> std::io::Result<()> {
        let temporary = path.with_extension(format!("{}.tmp", std::process::id()));
        let mut options = std::fs::OpenOptions::new();
        options.write(true).create(true).truncate(true);
        #[cfg(unix)]
        std::os::unix::fs::OpenOptionsExt::mode(&mut options, 0o600);
        let text = serde_json::to_string(self).map_err(std::io::Error::other)?;
        let written = options.open(&temporary).and_then(|mut f| {
            f.write_all(text.as_bytes())?;
            f.sync_all()
        });
        match written.and_then(|()| std::fs::rename(&temporary, path)) {
            Ok(()) => Ok(()),
            Err(e) => {
                let _ = std::fs::remove_file(&temporary);
                Err(e)
            }
        }
    }

    pub fn entry(&self, fingerprint: &str) -> Option<&ReachEntry> {
        self.entries.iter().find(|e| e.fingerprint == fingerprint)
    }

    /// Replaces the entry of the same fingerprint, or adds one.
    pub fn put(&mut self, entry: ReachEntry) {
        match self
            .entries
            .iter_mut()
            .find(|e| e.fingerprint == entry.fingerprint)
        {
            Some(slot) => *slot = entry,
            None => self.entries.push(entry),
        }
    }

    /// Keeps only the entries of `fingerprints`, so a replaced token's old entry goes.
    pub fn retain(&mut self, fingerprints: &[String]) {
        self.entries
            .retain(|e| fingerprints.contains(&e.fingerprint));
    }
}

/// Whether to ask GitHub again for a token whose entry is `entry`, for `repo` (`owner/repo`, lower
/// case).
pub fn should_fetch(entry: Option<&ReachEntry>, repo: &str, now: UnixTime) -> bool {
    let Some(entry) = entry else {
        return true;
    };
    let miss_allowed = now >= entry.last_miss_fetch_at.saturating_add(MISS_SECS);
    let stale = now >= entry.fetched_at.saturating_add(REFRESH_SECS);
    miss_allowed && (stale || !entry.repos.iter().any(|r| r == repo))
}

/// The forge's web base for a token's `host`: a bare name is `https://<name>`, a URL is itself
/// without a trailing `/`.
pub fn forge_base(host: &str) -> String {
    if host.contains("://") {
        host.trim_end_matches('/').to_string()
    } else {
        format!("https://{host}")
    }
}

/// The forge's REST base: `api.github.com` for github.com, `<host>/api/v3` for a GitHub Enterprise
/// name, and a URL given as such is used as is (the tests' loopback forge).
pub fn api_base(host: &str) -> String {
    if host.contains("://") {
        host.trim_end_matches('/').to_string()
    } else if host.eq_ignore_ascii_case("github.com") {
        "https://api.github.com".to_string()
    } else {
        format!("https://{host}/api/v3")
    }
}

#[derive(Deserialize)]
struct Listed {
    full_name: String,
}

/// Every repository `token` reaches: `GET <api>/user/repos?per_page=100&page=N` until a short page.
/// Names are lower case. An `Err` is short text with no token in it.
pub fn discover(
    transport: &dyn Transport,
    api: &str,
    token: &SecretToken,
) -> Result<Vec<String>, String> {
    let mut found = Vec::new();
    for page in 1..=MAX_PAGES {
        let url = format!("{api}/user/repos?per_page={PAGE}&page={page}");
        let reply = transport.get(&url, token)?;
        if reply.status != 200 {
            return Err(format!("the forge answered {}", reply.status));
        }
        let listed: Vec<Listed> =
            serde_json::from_str(&reply.body).map_err(|_| "an unreadable listing".to_string())?;
        let last = listed.len() < PAGE;
        found.extend(listed.into_iter().map(|r| r.full_name.to_ascii_lowercase()));
        if last {
            break;
        }
    }
    Ok(found)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scoped_credential::pat::Reply;
    use std::sync::Mutex;

    #[test]
    fn the_fingerprint_is_16_hex_digits_and_not_the_token() {
        let f = fingerprint("ghp_secret");
        assert_eq!(f.len(), 16);
        assert!(f.bytes().all(|b| b.is_ascii_hexdigit()));
        assert!(!"ghp_secret".contains(&f));
        assert_ne!(f, fingerprint("ghp_other"));
    }

    fn entry(repos: &[&str], fetched: u64, miss: u64) -> ReachEntry {
        ReachEntry {
            fingerprint: "f".into(),
            repos: repos.iter().map(ToString::to_string).collect(),
            fetched_at: fetched,
            last_miss_fetch_at: miss,
        }
    }

    #[test]
    fn an_unknown_token_is_fetched_at_once() {
        assert!(should_fetch(None, "a/b", 5));
    }

    #[test]
    fn a_known_repository_is_fetched_again_only_after_15_minutes() {
        let e = entry(&["a/b"], 1000, 0);
        assert!(!should_fetch(Some(&e), "a/b", 1000 + REFRESH_SECS - 1));
        assert!(should_fetch(Some(&e), "a/b", 1000 + REFRESH_SECS));
    }

    #[test]
    fn a_miss_fetches_at_most_once_a_minute() {
        let e = entry(&["a/b"], 1000, 1000);
        assert!(!should_fetch(Some(&e), "a/c", 1000 + MISS_SECS - 1));
        assert!(should_fetch(Some(&e), "a/c", 1000 + MISS_SECS));
    }

    #[test]
    fn the_cache_round_trips_through_its_file_and_a_damaged_one_is_empty() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("github-reach.json");
        assert_eq!(ReachCache::load(&path), ReachCache::default());
        let mut cache = ReachCache::default();
        cache.put(entry(&["a/b"], 1, 2));
        cache.save(&path).unwrap();
        assert_eq!(ReachCache::load(&path), cache);
        std::fs::write(&path, "{nope").unwrap();
        assert_eq!(ReachCache::load(&path), ReachCache::default());
    }

    #[test]
    fn hosts_map_to_their_web_and_api_bases() {
        assert_eq!(forge_base("github.com"), "https://github.com");
        assert_eq!(forge_base("http://127.0.0.1:9/"), "http://127.0.0.1:9");
        assert_eq!(api_base("github.com"), "https://api.github.com");
        assert_eq!(api_base("ghe.example"), "https://ghe.example/api/v3");
        assert_eq!(api_base("http://127.0.0.1:9"), "http://127.0.0.1:9");
    }

    struct Pages(Mutex<Vec<String>>, Vec<String>);

    impl Transport for Pages {
        fn get(&self, url: &str, _: &SecretToken) -> Result<Reply, String> {
            let mut asked = self.0.lock().unwrap();
            asked.push(url.to_string());
            let body = self.1.get(asked.len() - 1).cloned().unwrap_or("[]".into());
            Ok(Reply {
                status: 200,
                headers: vec![],
                body,
            })
        }
    }

    fn page(count: usize, owner: &str) -> String {
        let items: Vec<String> = (0..count)
            .map(|i| format!(r#"{{"full_name":"{owner}/R{i}"}}"#))
            .collect();
        format!("[{}]", items.join(","))
    }

    #[test]
    fn discovery_follows_full_pages_and_lowers_the_names() {
        let transport = Pages(Mutex::default(), vec![page(100, "Acme"), page(2, "Beta")]);
        let found = discover(&transport, "http://x", &SecretToken::new("t")).unwrap();
        assert_eq!(found.len(), 102);
        assert_eq!(found[0], "acme/r0");
        assert_eq!(found[101], "beta/r1");
        assert_eq!(
            *transport.0.lock().unwrap(),
            [
                "http://x/user/repos?per_page=100&page=1",
                "http://x/user/repos?per_page=100&page=2"
            ]
        );
    }
}
