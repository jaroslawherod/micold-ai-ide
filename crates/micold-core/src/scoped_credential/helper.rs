//! What `micold-daemon git-credential get` answers (contracts/git-in-container.md §2, FR-006/007).
//!
//! The credential of the one repository lives in an in-memory file under [`GIT_CREDENTIAL_ROOT`],
//! `<root>/<owner>/<repo>` (lower case), holding `expires_at=<unix seconds>` and `token=<token>`.
//! Git asks with `useHttpPath` on, so the request names the repository; every other host, repository
//! or expired file gets no answer and git moves on (and, with no prompt, fails at once).

use std::path::{Path, PathBuf};

use super::UnixTime;

/// Where the container keeps its credential files: a memory-backed mount no host path shares
/// (FR-006).
pub const GIT_CREDENTIAL_ROOT: &str = "/run/micold/git";

/// The username git sends with an installation token.
pub const CREDENTIAL_USERNAME: &str = "x-access-token";

/// The fields of git's credential-helper request this helper looks at.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct CredentialRequest {
    pub protocol: Option<String>,
    pub host: Option<String>,
    pub path: Option<String>,
}

/// Parse git's `key=value` lines. Unknown keys are ignored; a blank line ends the request.
pub fn parse_request(input: &str) -> CredentialRequest {
    let mut request = CredentialRequest::default();
    for line in input.lines() {
        if line.is_empty() {
            break;
        }
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        match key {
            "protocol" => request.protocol = Some(value.to_string()),
            "host" => request.host = Some(value.to_string()),
            "path" => request.path = Some(value.to_string()),
            _ => {}
        }
    }
    request
}

/// The file's text for a credential: the format [`parse_credential_file`] reads.
pub fn render_credential_file(expires_at: UnixTime, token: &str) -> String {
    format!("expires_at={expires_at}\ntoken={token}\n")
}

/// `(expires_at, token)` from a credential file, or `None` when either is missing or malformed.
pub fn parse_credential_file(text: &str) -> Option<(UnixTime, String)> {
    let (mut expires_at, mut token) = (None, None);
    for line in text.lines() {
        match line.split_once('=') {
            Some(("expires_at", v)) => expires_at = v.trim().parse().ok(),
            Some(("token", v)) => token = Some(v.trim().to_string()).filter(|t| !t.is_empty()),
            _ => {}
        }
    }
    Some((expires_at?, token?))
}

/// `(scheme, host[:port])` of a base such as `https://github.com`.
fn split_base(base: &str) -> Option<(&str, &str)> {
    let (scheme, rest) = base.split_once("://")?;
    let host = rest.trim_end_matches('/');
    (!host.is_empty() && !host.contains('/')).then_some((scheme, host))
}

/// `owner/repo`, lower-cased, from a request path (`o/r`, `o/r.git`, `o/r/`); `None` for any other
/// shape, so nothing but two plain names ever reaches the file system.
fn repo_key(path: &str) -> Option<(String, String)> {
    let path = path.trim_end_matches('/');
    let path = path.strip_suffix(".git").unwrap_or(path);
    let (owner, repo) = path.split_once('/')?;
    let plain = |s: &str| {
        !s.is_empty()
            && s != "."
            && s != ".."
            && s.chars()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'))
    };
    (plain(owner) && plain(repo)).then(|| (owner.to_ascii_lowercase(), repo.to_ascii_lowercase()))
}

/// The credential file for the repository a request names, when the request is for `base`.
pub fn credential_file(root: &Path, base: &str, request: &CredentialRequest) -> Option<PathBuf> {
    let (scheme, host) = split_base(base)?;
    if request.protocol.as_deref() != Some(scheme) || request.host.as_deref() != Some(host) {
        return None;
    }
    let (owner, repo) = repo_key(request.path.as_deref()?)?;
    Some(root.join(owner).join(repo))
}

/// The answer git gets: `username=`/`password=` lines, or nothing.
pub fn answer(root: &Path, base: &str, request: &CredentialRequest, now: UnixTime) -> String {
    let Some(file) = credential_file(root, base, request) else {
        return String::new();
    };
    let Ok(text) = std::fs::read_to_string(file) else {
        return String::new();
    };
    match parse_credential_file(&text) {
        Some((expires_at, token)) if expires_at > now => {
            format!("username={CREDENTIAL_USERNAME}\npassword={token}\n")
        }
        _ => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scoped_credential::GITHUB_BASE;

    const NOW: UnixTime = 1_000;

    fn request(host: &str, path: Option<&str>) -> CredentialRequest {
        CredentialRequest {
            protocol: Some("https".into()),
            host: Some(host.into()),
            path: path.map(str::to_string),
        }
    }

    fn root_with(file: &str, content: &str) -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(file);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, content).unwrap();
        dir
    }

    fn live() -> String {
        render_credential_file(NOW + 60, "tok-1")
    }

    #[test]
    fn the_root_is_the_memory_backed_mount() {
        assert_eq!(GIT_CREDENTIAL_ROOT, "/run/micold/git");
    }

    #[test]
    fn a_request_is_parsed_up_to_the_blank_line() {
        let r = parse_request("protocol=https\nhost=github.com\npath=o/r\n\nignored=1\n");
        assert_eq!(r, request("github.com", Some("o/r")));
    }

    #[test]
    fn the_file_format_round_trips() {
        assert_eq!(
            parse_credential_file(&render_credential_file(42, "abc")),
            Some((42, "abc".into()))
        );
        assert_eq!(parse_credential_file("token=abc\n"), None);
        assert_eq!(parse_credential_file("expires_at=x\ntoken=abc\n"), None);
        assert_eq!(parse_credential_file("expires_at=4\ntoken=\n"), None);
    }

    #[test]
    fn the_repository_in_any_path_form_gets_the_token() {
        let root = root_with("o/r", &live());
        for path in ["o/r", "o/r.git", "o/r/", "O/R.git"] {
            let got = answer(
                root.path(),
                GITHUB_BASE,
                &request("github.com", Some(path)),
                NOW,
            );
            assert_eq!(got, "username=x-access-token\npassword=tok-1\n", "{path}");
        }
    }

    #[test]
    fn another_repository_host_scheme_or_no_path_gets_nothing() {
        let root = root_with("o/r", &live());
        let ask = |req: &CredentialRequest| answer(root.path(), GITHUB_BASE, req, NOW);
        assert_eq!(ask(&request("github.com", Some("o/other"))), "");
        assert_eq!(ask(&request("github.com", Some("p/r"))), "");
        assert_eq!(ask(&request("gitlab.com", Some("o/r"))), "");
        assert_eq!(ask(&request("github.com", None)), "");
        let mut http = request("github.com", Some("o/r"));
        http.protocol = Some("http".into());
        assert_eq!(ask(&http), "");
        assert_eq!(ask(&request("github.com", Some("../o/r"))), "");
        assert_eq!(ask(&request("github.com", Some("o/r/x"))), "");
    }

    #[test]
    fn an_expired_missing_or_unreadable_file_gets_nothing() {
        let root = root_with("o/r", &render_credential_file(NOW, "tok-1"));
        let req = request("github.com", Some("o/r"));
        assert_eq!(answer(root.path(), GITHUB_BASE, &req, NOW), "");
        assert_eq!(
            answer(tempfile::tempdir().unwrap().path(), GITHUB_BASE, &req, NOW),
            ""
        );
        let garbled = root_with("o/r", "not a credential file");
        assert_eq!(answer(garbled.path(), GITHUB_BASE, &req, NOW), "");
        // A directory where the file should be cannot be read as one.
        let dir = root_with("o/r/inner", "x");
        assert_eq!(answer(dir.path(), GITHUB_BASE, &req, NOW), "");
    }
}
