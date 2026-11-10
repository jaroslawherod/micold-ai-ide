//! What `micold-daemon git-credential get` answers (contracts/git-in-container.md §2, FR-006/007).
//!
//! The request git sends it, and the repository path it names. The credential itself comes from the
//! daemon's configuration file (#1530): the grant-written files this helper once read are gone.

/// Where the container keeps its credential files: a memory-backed mount no host path shares
/// (FR-006).
pub const GIT_CREDENTIAL_ROOT: &str = "/run/micold/git";

/// The username git sends with the token.
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

/// `(scheme, host[:port])` of a base such as `https://github.com`.
pub fn split_base(base: &str) -> Option<(&str, &str)> {
    let (scheme, rest) = base.split_once("://")?;
    let host = rest.trim_end_matches('/');
    (!host.is_empty() && !host.contains('/')).then_some((scheme, host))
}

/// `owner/repo`, lower-cased, from a request path (`o/r`, `o/r.git`, `o/r/`); `None` for any other
/// shape, so nothing but two plain names ever reaches the file system.
pub fn repo_key(path: &str) -> Option<(String, String)> {
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

#[cfg(test)]
mod tests {
    use super::*;

    fn request(host: &str, path: Option<&str>) -> CredentialRequest {
        CredentialRequest {
            protocol: Some("https".into()),
            host: Some(host.into()),
            path: path.map(str::to_string),
        }
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
    fn the_repository_in_any_path_form_is_the_same_lower_case_key() {
        for path in ["o/r", "o/r.git", "o/r/", "O/R.git"] {
            assert_eq!(repo_key(path), Some(("o".into(), "r".into())), "{path}");
        }
    }

    #[test]
    fn a_path_that_is_not_two_plain_names_has_no_key() {
        for path in [
            "../r", "o/..", "./r", "o/.", "../r.git", "o/../", "o/r/x", "o",
        ] {
            assert_eq!(repo_key(path), None, "{path}");
        }
    }
}
