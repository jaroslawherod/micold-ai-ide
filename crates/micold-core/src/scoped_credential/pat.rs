//! The pasted fine-grained PAT as a credential source, pure parts (#1002, FR-005, FR-009;
//! contracts/pat-source.md §2): what makes a pasted token acceptable. The HTTP itself is a
//! [`Transport`] the client implements over `ureq`, so every judgement here runs without a network
//! and no real token is ever needed.
//!
//! The token never reaches a message, a `Debug` or a log line: [`short_error`] strips it from a
//! transport's error text, and the [`Transport`] is told the token only through [`SecretToken`].

use serde::Deserialize;

use super::{NoCredentialReason, SecretToken, UnixTime};
use crate::github::GithubRepo;

/// The prefix of a fine-grained PAT; a classic token is `ghp_…`, an OAuth one `gho_…`.
pub const FINE_GRAINED_PREFIX: &str = "github_pat_";

/// The response header GitHub reports a fine-grained token's expiry in.
pub const EXPIRATION_HEADER: &str = "github-authentication-token-expiration";

/// What one GitHub request came back with.
#[derive(Debug, Clone, Default)]
pub struct Reply {
    /// The HTTP status.
    pub status: u16,
    /// The response headers, as received.
    pub headers: Vec<(String, String)>,
    /// The body, as text.
    pub body: String,
}

impl Reply {
    /// A header's value, the name compared ignoring case.
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(n, _)| n.eq_ignore_ascii_case(name))
            .map(|(_, v)| v.as_str())
    }
}

/// How a check reaches GitHub: one authenticated GET of an [`ApiBase`] URL. An `Err` is
/// a transport failure (timeout, DNS, TLS), as short text.
pub trait Transport: Send + Sync {
    /// `GET url` with `Authorization: Bearer <token>` and the headers of contract §2.
    fn get(&self, url: &str, token: &SecretToken) -> Result<Reply, String>;
}

/// The token as pasted, trimmed once of the whitespace a paste carries.
pub fn normalize(pasted: &str) -> SecretToken {
    SecretToken::new(pasted.trim())
}

/// Whether `token` is a fine-grained PAT by its shape (contract §2, *shape*).
pub fn check_shape(token: &SecretToken) -> Result<(), NoCredentialReason> {
    let t = token.expose();
    if t.starts_with(FINE_GRAINED_PREFIX)
        && t.len() > FINE_GRAINED_PREFIX.len()
        && t.bytes().all(|b| b.is_ascii_graphic())
    {
        Ok(())
    } else {
        Err(refused("not a fine-grained token"))
    }
}

fn refused(reason: impl Into<String>) -> NoCredentialReason {
    NoCredentialReason::SourceRefused(reason.into())
}

/// A transport error as a short reason for the status, with `token` removed from it.
pub fn short_error(error: &str, token: &SecretToken) -> String {
    let mut text = error.to_string();
    if !token.expose().is_empty() {
        text = text.replace(token.expose(), "<redacted>");
    }
    let text = text.lines().next().unwrap_or_default().trim();
    text.chars().take(80).collect()
}

/// The unix time of GitHub's expiry header, `2026-11-20 00:00:00 UTC` (a numeric offset is
/// honoured too).
pub fn parse_expiration(value: &str) -> Option<UnixTime> {
    let value = value.trim();
    let (date, rest) = value.split_once(' ')?;
    let mut ymd = date.split('-');
    let (year, month, day): (i64, i64, i64) = (
        ymd.next()?.parse().ok()?,
        ymd.next()?.parse().ok()?,
        ymd.next()?.parse().ok()?,
    );
    if ymd.next().is_some() || !(1..=12).contains(&month) || !(1..=31).contains(&day) {
        return None;
    }
    let (time, zone) = rest.split_once(' ').unwrap_or((rest, "UTC"));
    let mut hms = time.split(':');
    let (h, m, s): (i64, i64, i64) = (
        hms.next()?.parse().ok()?,
        hms.next()?.parse().ok()?,
        hms.next().unwrap_or("0").parse().ok()?,
    );
    let offset = match zone.trim() {
        "UTC" | "Z" | "GMT" => 0,
        z if z.len() == 5 && (z.starts_with('+') || z.starts_with('-')) => {
            let sign = if z.starts_with('-') { -1 } else { 1 };
            let (oh, om): (i64, i64) = (z[1..3].parse().ok()?, z[3..5].parse().ok()?);
            sign * (oh * 3600 + om * 60)
        }
        _ => return None,
    };
    let unix = days_from_civil(year, month, day) * 86_400 + h * 3600 + m * 60 + s - offset;
    u64::try_from(unix).ok()
}

/// Days since 1970-01-01 of a proleptic Gregorian date (Hinnant's algorithm).
fn days_from_civil(year: i64, month: i64, day: i64) -> i64 {
    let y = if month <= 2 { year - 1 } else { year };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let doy = (153 * (month + if month > 2 { -3 } else { 9 }) + 2) / 5 + day - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

/// What a non-200 answer to `repo`'s requests means (contract §2).
fn status_reason(reply: &Reply, what: &dyn std::fmt::Display) -> NoCredentialReason {
    match reply.status {
        401 => refused("GitHub rejected the token (expired or revoked)"),
        403 if reply.header("x-ratelimit-remaining") == Some("0") => {
            NoCredentialReason::SourceUnreachable("rate limited".to_string())
        }
        403 if reply.header("x-github-sso").is_some() => {
            refused("authorise the token for the organisation's SSO in GitHub")
        }
        403 | 404 => refused(format!("the token does not reach {what}")),
        status => NoCredentialReason::SourceUnreachable(format!("GitHub answered {status}")),
    }
}

#[derive(Deserialize)]
struct RepoJson {
    #[serde(default)]
    permissions: Option<PermissionsJson>,
}

#[derive(Deserialize)]
struct PermissionsJson {
    #[serde(default)]
    push: bool,
}

/// Judge the answer to `GET /repos/{o}/{r}` (contract §2, *probe*): the token reaches the repository
/// with push, and GitHub reports a future expiry. Returns that expiry.
pub fn parse_probe(
    reply: &Reply,
    repo: &GithubRepo,
    now: UnixTime,
) -> Result<UnixTime, NoCredentialReason> {
    if reply.status != 200 {
        return Err(status_reason(reply, repo));
    }
    let json: RepoJson = serde_json::from_str(&reply.body)
        .map_err(|_| NoCredentialReason::SourceUnreachable("unexpected answer".to_string()))?;
    if !json.permissions.is_some_and(|p| p.push) {
        return Err(refused(format!("the token cannot push to {repo}")));
    }
    parse_expiry_header(reply, now)
}

#[derive(Deserialize)]
struct ListedRepo {
    full_name: String,
}

/// The listing that says which repositories a token reaches (contract §2, *reach*): the private
/// ones, which are all a token can reach that every other token cannot. A hundred is the most one
/// page lists; the Settings text says when the list may be longer.
pub const REACH_URL: &str = "https://api.github.com/user/repos?visibility=private&per_page=100";

/// Where the GitHub API is: `https://api.github.com` unless a caller passes another (#1542, FR-001,
/// FR-004). Only a test builds a non-default base, to point the check at a loopback fake; every
/// shipped caller passes [`ApiBase::default`], so the requests are byte-identical to before. Nothing
/// here reads an environment variable, a setting or a file. A token goes only to URLs built from the
/// base the check was given; a malformed base surfaces as `SourceUnreachable` from the transport.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApiBase(String);

const DEFAULT_API_BASE: &str = "https://api.github.com";

impl Default for ApiBase {
    fn default() -> Self {
        Self(DEFAULT_API_BASE.to_string())
    }
}

impl ApiBase {
    /// A base from `base`; empty (after trimming) is the default, a trailing `/` is dropped.
    pub fn new(base: &str) -> Self {
        let trimmed = base.trim().trim_end_matches('/');
        if trimmed.is_empty() {
            Self::default()
        } else {
            Self(trimmed.to_string())
        }
    }

    /// The URL of the repository probe for `repo`.
    pub fn probe_url(&self, repo: &GithubRepo) -> String {
        format!("{}/repos/{}/{}", self.0, repo.owner(), repo.name())
    }

    /// The URL of the private-repository listing ([`REACH_URL`] for the default base).
    pub fn reach_url(&self) -> String {
        format!("{}/user/repos?visibility=private&per_page=100", self.0)
    }
}

/// Judge the answer to [`REACH_URL`] (contract §2, *reach*): the full names of the private
/// repositories the token reaches. A token may reach any number of them (#1529, FR-009): nothing is
/// refused for the count, the list is for Settings to show. An answer that is not a `200` listing
/// says nothing either way, so it is an empty list; one GitHub is too busy to give is retried.
pub fn parse_reach(reply: &Reply) -> Result<Vec<String>, NoCredentialReason> {
    if reply.status == 401 {
        return Err(status_reason(reply, &"GitHub"));
    }
    // A listing GitHub is too busy or limited to give is retried (paste refused, a held token
    // kept), not passed.
    let limited = reply.status == 403 && reply.header("x-ratelimit-remaining") == Some("0");
    if reply.status == 429 || reply.status >= 500 || limited {
        return Err(NoCredentialReason::SourceUnreachable(
            "GitHub is busy".to_string(),
        ));
    }
    if reply.status != 200 {
        return Ok(vec![]);
    }
    Ok(serde_json::from_str::<Vec<ListedRepo>>(&reply.body)
        .map(|listed| listed.into_iter().map(|r| r.full_name).collect())
        .unwrap_or_default())
}

/// Every check of a token at issue time for `repo` at `now` (contract §2): shape and probe. Returns
/// the expiry GitHub reports. Nothing is stored or sent by this.
pub fn check(
    transport: &dyn Transport,
    base: &ApiBase,
    repo: &GithubRepo,
    token: &SecretToken,
    now: UnixTime,
) -> Result<UnixTime, NoCredentialReason> {
    check_shape(token)?;
    let probe = get(transport, token, base.probe_url(repo))?;
    parse_probe(&probe, repo, now)
}

fn get(
    transport: &dyn Transport,
    token: &SecretToken,
    url: String,
) -> Result<Reply, NoCredentialReason> {
    transport
        .get(&url, token)
        .map_err(|e| NoCredentialReason::SourceUnreachable(short_error(&e, token)))
}

/// What a pasted token turned out to be.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Checked {
    /// When GitHub says it expires.
    pub expires_at: UnixTime,
    /// The private repositories it reaches (`owner/name`), as GitHub lists them.
    pub reach: Vec<String>,
}

/// Every check of a token pasted in Settings (contract §2): shape, the reach listing, and, when
/// `probe` names a repository, the probe of that repository (push access). The expiry is the
/// probe's, or the listing's header when there is no repository to probe. Nothing is stored.
pub fn check_added(
    transport: &dyn Transport,
    base: &ApiBase,
    token: &SecretToken,
    probe: Option<&GithubRepo>,
    now: UnixTime,
) -> Result<Checked, NoCredentialReason> {
    check_shape(token)?;
    let listing = get(transport, token, base.reach_url())?;
    let reach = parse_reach(&listing)?;
    let expires_at = match probe {
        Some(repo) => parse_probe(&get(transport, token, base.probe_url(repo))?, repo, now)?,
        None => {
            if listing.status != 200 {
                return Err(status_reason(&listing, &"any repository"));
            }
            parse_expiry_header(&listing, now)?
        }
    };
    Ok(Checked { expires_at, reach })
}

fn parse_expiry_header(reply: &Reply, now: UnixTime) -> Result<UnixTime, NoCredentialReason> {
    let header = reply
        .header(EXPIRATION_HEADER)
        .filter(|v| !v.trim().is_empty())
        .ok_or_else(|| refused("the token has no expiry"))?;
    let expires_at =
        parse_expiration(header).ok_or_else(|| refused("the token's expiry could not be read"))?;
    if expires_at <= now {
        return Err(NoCredentialReason::PatExpired);
    }
    Ok(expires_at)
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use super::*;

    const NOW: UnixTime = 1_700_000_000;
    const PAT: &str = "github_pat_FAKE0123456789";

    fn repo() -> GithubRepo {
        GithubRepo::from_remote_url("https://github.com/o/r").unwrap()
    }

    fn reply(status: u16, headers: &[(&str, &str)], body: &str) -> Reply {
        Reply {
            status,
            headers: headers
                .iter()
                .map(|(n, v)| (n.to_string(), v.to_string()))
                .collect(),
            body: body.to_string(),
        }
    }

    fn good_probe() -> Reply {
        reply(
            200,
            &[(EXPIRATION_HEADER, "2023-11-20 00:00:00 UTC")],
            r#"{"permissions":{"push":true,"pull":true}}"#,
        )
    }

    #[test]
    fn only_a_fine_grained_shape_is_accepted_and_padding_is_trimmed_once() {
        assert!(check_shape(&SecretToken::new(PAT)).is_ok());
        for bad in [
            "ghp_abc",
            "gho_abc",
            "",
            "github_pat_",
            " github_pat_x",
            "github_pat_a b",
        ] {
            assert_eq!(
                check_shape(&SecretToken::new(bad)),
                Err(refused("not a fine-grained token")),
                "{bad:?}"
            );
        }
        assert_eq!(normalize(&format!("  {PAT}\n")).expose(), PAT);
    }

    #[test]
    fn the_expiry_header_parses_to_unix_seconds() {
        assert_eq!(
            parse_expiration("2026-11-20 00:00:00 UTC"),
            Some(1_795_132_800)
        );
        assert_eq!(parse_expiration("1970-01-02 00:00:01 UTC"), Some(86_401));
        assert_eq!(
            parse_expiration("2024-02-29 12:00:00 +0200"),
            Some(1_709_200_800)
        );
        for bad in [
            "",
            "soon",
            "2026-13-01 00:00:00 UTC",
            "2026-11-20",
            "2026-11-20 00:00:00 PST",
        ] {
            assert_eq!(parse_expiration(bad), None, "{bad:?}");
        }
    }

    #[test]
    fn the_probe_needs_push_and_a_future_expiry() {
        assert_eq!(
            parse_probe(&good_probe(), &repo(), NOW),
            Ok(parse_expiration("2023-11-20 00:00:00 UTC").unwrap())
        );
    }

    #[test]
    fn a_probe_without_push_or_expiry_or_past_is_refused() {
        let no_push = reply(
            200,
            &[(EXPIRATION_HEADER, "2023-11-20 00:00:00 UTC")],
            r#"{"permissions":{"push":false}}"#,
        );
        let none = reply(200, &[(EXPIRATION_HEADER, "2023-11-20 00:00:00 UTC")], "{}");
        for r in [no_push, none] {
            assert_eq!(
                parse_probe(&r, &repo(), NOW),
                Err(refused("the token cannot push to o/r"))
            );
        }
        let no_expiry = reply(200, &[], r#"{"permissions":{"push":true}}"#);
        assert_eq!(
            parse_probe(&no_expiry, &repo(), NOW),
            Err(refused("the token has no expiry"))
        );
        assert_eq!(
            parse_probe(&good_probe(), &repo(), 1_800_000_000),
            Err(NoCredentialReason::PatExpired)
        );
        let expiry = parse_expiration("2023-11-20 00:00:00 UTC").unwrap();
        assert_eq!(
            parse_probe(&good_probe(), &repo(), expiry),
            Err(NoCredentialReason::PatExpired),
            "a token expiring this second is expired"
        );
        assert_eq!(parse_probe(&good_probe(), &repo(), expiry - 1), Ok(expiry));
    }

    #[test]
    fn statuses_map_to_their_reasons() {
        let probe = |status, headers: &[(&str, &str)]| {
            parse_probe(&reply(status, headers, ""), &repo(), NOW).unwrap_err()
        };
        assert_eq!(
            probe(401, &[]),
            refused("GitHub rejected the token (expired or revoked)")
        );
        for status in [403, 404] {
            assert_eq!(probe(status, &[]), refused("the token does not reach o/r"));
        }
        assert_eq!(
            probe(403, &[("X-RateLimit-Remaining", "0")]),
            NoCredentialReason::SourceUnreachable("rate limited".into())
        );
        assert_eq!(
            probe(403, &[("x-github-sso", "required; url=https://x")]),
            refused("authorise the token for the organisation's SSO in GitHub")
        );
        assert_eq!(
            probe(502, &[]),
            NoCredentialReason::SourceUnreachable("GitHub answered 502".into())
        );
    }

    #[test]
    fn a_token_reaching_many_repositories_is_accepted_and_its_reach_listed() {
        let listing = |body| reply(200, &[], body);
        assert_eq!(parse_reach(&listing("[]")), Ok(vec![]));
        assert_eq!(
            parse_reach(&listing(r#"[{"full_name":"o/r"}]"#)),
            Ok(vec!["o/r".to_string()])
        );
        assert_eq!(
            parse_reach(&listing(
                r#"[{"full_name":"o/r"},{"full_name":"o/secret"},{"full_name":"p/x"}]"#
            )),
            Ok(vec!["o/r".into(), "o/secret".into(), "p/x".into()])
        );
        assert_eq!(
            parse_reach(&reply(401, &[], "")),
            Err(refused("GitHub rejected the token (expired or revoked)"))
        );
        // A listing GitHub will not give says nothing about the token; a busy one is retried.
        assert_eq!(parse_reach(&reply(403, &[], "")), Ok(vec![]));
        for busy in [
            reply(429, &[], ""),
            reply(503, &[], ""),
            reply(403, &[("x-ratelimit-remaining", "0")], ""),
        ] {
            assert!(matches!(
                parse_reach(&busy),
                Err(NoCredentialReason::SourceUnreachable(_))
            ));
        }
    }

    /// Answers each request in order and records the URLs asked.
    struct Script {
        answers: Mutex<Vec<Result<Reply, String>>>,
        asked: Mutex<Vec<String>>,
    }

    impl Script {
        fn new(mut answers: Vec<Result<Reply, String>>) -> Self {
            answers.reverse();
            Self {
                answers: Mutex::new(answers),
                asked: Mutex::new(vec![]),
            }
        }
    }

    impl Transport for Script {
        fn get(&self, url: &str, _token: &SecretToken) -> Result<Reply, String> {
            self.asked.lock().unwrap().push(url.to_string());
            self.answers.lock().unwrap().pop().expect("no more answers")
        }
    }

    #[test]
    fn a_good_token_passes_the_shape_and_probe_checks() {
        let t = Script::new(vec![Ok(good_probe())]);
        let expires = check(
            &t,
            &ApiBase::default(),
            &repo(),
            &SecretToken::new(PAT),
            NOW,
        )
        .unwrap();
        assert_eq!(
            expires,
            parse_expiration("2023-11-20 00:00:00 UTC").unwrap()
        );
        assert_eq!(
            *t.asked.lock().unwrap(),
            ["https://api.github.com/repos/o/r"]
        );
    }

    #[test]
    fn a_pasted_token_reaching_several_repositories_is_checked_without_a_probe_repository() {
        let listing = Reply {
            body: r#"[{"full_name":"o/a"},{"full_name":"o/b"}]"#.into(),
            ..good_probe()
        };
        let t = Script::new(vec![Ok(listing)]);
        let checked =
            check_added(&t, &ApiBase::default(), &SecretToken::new(PAT), None, NOW).unwrap();
        assert_eq!(checked.reach, ["o/a", "o/b"]);
        assert_eq!(
            checked.expires_at,
            parse_expiration("2023-11-20 00:00:00 UTC").unwrap()
        );
        assert_eq!(*t.asked.lock().unwrap(), [REACH_URL]);
    }

    #[test]
    fn a_refused_listing_without_a_probe_repository_gives_the_status_reason() {
        let listing = reply(403, &[("x-github-sso", "required; url=https://x")], "");
        let t = Script::new(vec![Ok(listing)]);
        assert_eq!(
            check_added(&t, &ApiBase::default(), &SecretToken::new(PAT), None, NOW),
            Err(refused(
                "authorise the token for the organisation's SSO in GitHub"
            ))
        );
    }

    #[test]
    fn a_probe_repository_is_probed_after_the_listing() {
        let listing = Reply {
            body: "[]".into(),
            ..good_probe()
        };
        let t = Script::new(vec![Ok(listing), Ok(good_probe())]);
        let checked = check_added(
            &t,
            &ApiBase::default(),
            &SecretToken::new(PAT),
            Some(&repo()),
            NOW,
        )
        .unwrap();
        assert!(checked.reach.is_empty());
        assert_eq!(
            t.asked.lock().unwrap()[1],
            "https://api.github.com/repos/o/r"
        );
    }

    #[test]
    fn the_default_base_gives_exactly_the_production_urls() {
        for base in [ApiBase::default(), ApiBase::new("")] {
            assert_eq!(base.probe_url(&repo()), "https://api.github.com/repos/o/r");
            assert_eq!(base.reach_url(), REACH_URL);
        }
    }

    #[test]
    fn a_base_is_trimmed_of_a_trailing_slash_and_keeps_a_path_prefix() {
        let base = ApiBase::new("http://127.0.0.1:9/api/v3/");
        assert_eq!(
            base.probe_url(&repo()),
            "http://127.0.0.1:9/api/v3/repos/o/r"
        );
        assert_eq!(
            base.reach_url(),
            "http://127.0.0.1:9/api/v3/user/repos?visibility=private&per_page=100"
        );
    }

    #[test]
    fn a_given_base_is_the_only_host_a_token_is_sent_to() {
        let listing = Reply {
            body: "[]".into(),
            ..good_probe()
        };
        let t = Script::new(vec![Ok(listing), Ok(good_probe()), Ok(good_probe())]);
        let base = ApiBase::new("http://127.0.0.1:9");
        check_added(&t, &base, &SecretToken::new(PAT), Some(&repo()), NOW).unwrap();
        check(&t, &base, &repo(), &SecretToken::new(PAT), NOW).unwrap();
        let asked = t.asked.lock().unwrap();
        assert_eq!(asked.len(), 3);
        assert!(asked.iter().all(|u| u.starts_with("http://127.0.0.1:9/")));
    }

    #[test]
    fn a_malformed_base_is_unreachable_from_the_transport_with_no_fallback() {
        let t = Script::new(vec![Err("invalid url".into())]);
        let base = ApiBase::new("not a url");
        let out = check(&t, &base, &repo(), &SecretToken::new(PAT), NOW);
        assert!(matches!(out, Err(NoCredentialReason::SourceUnreachable(_))));
        assert_eq!(*t.asked.lock().unwrap(), ["not a url/repos/o/r"]);
    }

    #[test]
    fn a_classic_token_is_refused_before_any_request() {
        let t = Script::new(vec![]);
        assert_eq!(
            check(
                &t,
                &ApiBase::default(),
                &repo(),
                &SecretToken::new("ghp_classic"),
                NOW
            ),
            Err(refused("not a fine-grained token"))
        );
        assert!(t.asked.lock().unwrap().is_empty());
    }

    #[test]
    fn a_transport_failure_is_unreachable_and_never_carries_the_token() {
        let t = Script::new(vec![Err(format!("tls error for Bearer {PAT}\nmore"))]);
        let Err(NoCredentialReason::SourceUnreachable(reason)) = check(
            &t,
            &ApiBase::default(),
            &repo(),
            &SecretToken::new(PAT),
            NOW,
        ) else {
            panic!("expected unreachable");
        };
        assert!(
            !reason.contains(PAT) && reason.contains("<redacted>"),
            "{reason}"
        );
        assert!(!reason.contains("more"));
    }

    #[test]
    fn no_message_or_debug_carries_the_token() {
        let token = SecretToken::new(PAT);
        let shown = format!("{token:?}");
        assert!(!shown.contains("FAKE0123"));
        let t = Script::new(vec![Ok(reply(403, &[], ""))]);
        let err = check(&t, &ApiBase::default(), &repo(), &token, NOW).unwrap_err();
        assert!(!format!("{err:?} {}", err.message()).contains("FAKE0123"));
    }
}
