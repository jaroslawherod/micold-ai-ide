//! The Jira Cloud request (feature 533, contracts/jira-cloud-source.md §2 and §5): `JiraCloud`
//! implements [`JiraSource`] over `ureq` 3, so the HTTP client lives in the client crate alone
//! (the daemon and core do not link it, Constitution VII).
//!
//! One `POST /rest/api/3/search/jql` carries the JQL, the field list and Jira's page token, with
//! the headers `Authorization`, `Accept`, `Content-Type` and `User-Agent` and nothing else of ours
//! (FR-020). The whole request has a 10 second budget, a redirect is not followed, and every
//! failure is mapped onto [`Outcome`] at this boundary, so no transport error text (and with it no
//! header) reaches a notice unclassified. The token is used only to build the header of the
//! request in hand.
//!
//! Feature 1185 adds the filtered listing and search (the same `POST`, its JQL narrowed by the
//! chosen filters) and the filter choices (`GET` of a project resource). One private `request`
//! builds every request, so the headers and the outcome mapping exist once.
//!
//! Depends on `micold_core`, `ureq`, `base64` and `serde_json` alone, so the adapter test includes
//! this file by path.

use std::io::Read;
use std::time::Duration;

use base64::Engine;
use micold_core::github::{Issue, IssueLoadError, IssuePage};
use micold_core::jira::{
    classify, list_jql, parse_page, search_jql, ApiToken, JiraConnection, JiraSource, Outcome,
};
use micold_core::jira_filter::{
    narrowed, parse_components, parse_versions, JiraFilters, VersionChoice,
};
use micold_core::jira_query::{rejection_messages, QueryFailure, ScopedQuery};
use micold_core::jira_status::{parse_statuses, status_jql, statuses_jql, KeyedStatus};

/// How many times a refused status request is halved to find the key Jira refuses: 2^7 covers a
/// chunk of 100 (contracts/jira-status-source.md §3).
const MAX_BISECTION_DEPTH: u8 = 7;

/// Why one request failed: a `400` is told apart, because a status request answers it by halving
/// and a query shows Jira's messages from its body (feature 1186, FR-010).
enum PostError {
    /// Jira answered `400`; the body as read, up to [`MAX_REFUSAL_BYTES`].
    BadRequest(Vec<u8>),
    Load(IssueLoadError),
}

/// Bytes of a `400` answer's body read at most: Jira's messages are a few lines.
const MAX_REFUSAL_BYTES: u64 = 64 * 1024;

/// A failure as the caller sees it: a `400` is classified like any other status.
fn load_error(error: PostError) -> IssueLoadError {
    match error {
        PostError::BadRequest(_) => classify(&Outcome::Status(400)),
        PostError::Load(error) => error,
    }
}

/// How a request is sent.
enum Method<'a> {
    Get,
    Post(&'a serde_json::Value),
}

/// Where every issue search goes.
const SEARCH_PATH: &str = "/rest/api/3/search/jql";

/// The whole request's budget: connect, send and read (FR-018).
pub const REQUEST_TIMEOUT: Duration = Duration::from_secs(10);

/// Issues asked for per page.
const PAGE_SIZE: u32 = 100;

/// Issues a search beyond the loaded list asks for at most.
const SEARCH_SIZE: u32 = 50;

/// The fields a listing reads, and nothing more (contracts/jira-cloud-source.md §3).
const FIELDS: [&str; 7] = [
    "summary",
    "labels",
    "issuetype",
    "reporter",
    "description",
    "updated",
    "status",
];

/// Jira Cloud over HTTPS, with the user's account email and API token.
#[derive(Debug, Clone)]
pub struct JiraCloud {
    timeout: Duration,
}

impl Default for JiraCloud {
    fn default() -> Self {
        Self::new()
    }
}

impl JiraCloud {
    /// The adapter with the 10 second budget.
    pub fn new() -> Self {
        Self::with_timeout(REQUEST_TIMEOUT)
    }

    /// The adapter with another budget, so a test need not wait ten seconds.
    pub fn with_timeout(timeout: Duration) -> Self {
        Self { timeout }
    }

    /// Send one search and return its body, or the outcome that classifies the failure.
    fn post(
        &self,
        conn: &JiraConnection,
        token: &ApiToken,
        body: &serde_json::Value,
    ) -> Result<Vec<u8>, IssueLoadError> {
        self.send(conn, token, body).map_err(load_error)
    }

    /// [`Self::post`], keeping a `400` apart from the other failures.
    fn send(
        &self,
        conn: &JiraConnection,
        token: &ApiToken,
        body: &serde_json::Value,
    ) -> Result<Vec<u8>, PostError> {
        self.request(conn, token, Method::Post(body), SEARCH_PATH, &[])
    }

    /// `GET path` on the connection's site, with `query`, returning the body (feature 1185).
    fn get(
        &self,
        conn: &JiraConnection,
        token: &ApiToken,
        path: &str,
        query: &[(&str, &str)],
    ) -> Result<Vec<u8>, IssueLoadError> {
        self.request(conn, token, Method::Get, path, query)
            .map_err(load_error)
    }

    /// Every request to Jira: `path` on the connection's site only, with the headers
    /// `Authorization`, `Accept`, `User-Agent` (and `Content-Type` on a `POST`) and nothing else of
    /// ours, the request budget, no redirect followed, and every failure classified, a `400` kept
    /// apart (contracts/jira-filter-source.md §3).
    fn request(
        &self,
        conn: &JiraConnection,
        token: &ApiToken,
        method: Method<'_>,
        path: &str,
        query: &[(&str, &str)],
    ) -> Result<Vec<u8>, PostError> {
        let load = |outcome: Outcome| PostError::Load(classify(&outcome));
        let url = format!("{}{path}", conn.site.as_str());
        let credentials = base64::engine::general_purpose::STANDARD.encode(format!(
            "{}:{}",
            conn.email.as_str(),
            token.expose()
        ));
        let agent: ureq::Agent = ureq::Agent::config_builder()
            .timeout_global(Some(self.timeout))
            .max_redirects(0)
            .http_status_as_error(false)
            .build()
            .into();
        let authorization = format!("Basic {credentials}");
        let sent = match method {
            Method::Get => agent
                .get(&url)
                .query_pairs(query.iter().copied())
                .header("Authorization", &authorization)
                .header("Accept", "application/json")
                .header("User-Agent", "micold-ai-ide")
                .call(),
            Method::Post(body) => agent
                .post(&url)
                .query_pairs(query.iter().copied())
                .header("Authorization", &authorization)
                .header("Accept", "application/json")
                .header("Content-Type", "application/json")
                .header("User-Agent", "micold-ai-ide")
                .send(body.to_string().as_bytes()),
        };
        let mut response = match sent {
            Ok(response) => response,
            Err(error) => return Err(load(outcome_of(&error, token))),
        };
        let status = response.status().as_u16();
        match status {
            200..=299 => {}
            300..=399 => return Err(load(Outcome::Redirected)),
            400 => return Err(PostError::BadRequest(refusal_body(&mut response))),
            _ => return Err(load(Outcome::Status(status))),
        }
        response
            .body_mut()
            .read_to_vec()
            .map_err(|error| load(outcome_of(&error, token)))
    }

    /// Read the statuses of `keys` (one chunk): a `400` halves the keys until the one Jira refuses
    /// stands alone, and that key is absent from the answer.
    fn read_chunk(
        &self,
        conn: &JiraConnection,
        token: &ApiToken,
        keys: &[String],
        depth: u8,
    ) -> Result<Vec<KeyedStatus>, IssueLoadError> {
        let body = serde_json::json!({
            "jql": status_jql(&conn.project_key, keys),
            "fields": ["status"],
            "maxResults": keys.len(),
        });
        match self.send(conn, token, &body) {
            Ok(bytes) => parse_statuses(&bytes),
            Err(PostError::Load(error)) => Err(error),
            Err(PostError::BadRequest(_)) if keys.len() == 1 => Ok(Vec::new()),
            Err(PostError::BadRequest(_)) if depth >= MAX_BISECTION_DEPTH => {
                Err(classify(&Outcome::Status(400)))
            }
            Err(PostError::BadRequest(_)) => {
                let (head, tail) = keys.split_at(keys.len() / 2);
                let mut read = self.read_chunk(conn, token, head, depth + 1)?;
                read.extend(self.read_chunk(conn, token, tail, depth + 1)?);
                Ok(read)
            }
        }
    }
}

/// What a `ureq` error comes to. The reason is the transport's text with the token, should it ever
/// appear, removed.
fn outcome_of(error: &ureq::Error, token: &ApiToken) -> Outcome {
    match error {
        ureq::Error::Timeout(_) => Outcome::TimedOut,
        other => Outcome::Unreachable(other.to_string().replace(token.expose(), "…")),
    }
}

/// The body of a `400` answer, up to [`MAX_REFUSAL_BYTES`]; what could not be read is left out, so
/// a body lost in transit reads as no message.
fn refusal_body(response: &mut ureq::http::Response<ureq::Body>) -> Vec<u8> {
    let mut body = Vec::new();
    let _ = response
        .body_mut()
        .as_reader()
        .take(MAX_REFUSAL_BYTES)
        .read_to_end(&mut body);
    body
}

fn request_body(jql: &str, cursor: Option<&str>, max_results: u32) -> serde_json::Value {
    let mut body = serde_json::json!({
        "jql": jql,
        "fields": FIELDS,
        "maxResults": max_results,
    });
    if let Some(cursor) = cursor {
        body["nextPageToken"] = serde_json::Value::from(cursor);
    }
    body
}

impl JiraSource for JiraCloud {
    fn list_open(
        &self,
        conn: &JiraConnection,
        token: &ApiToken,
        cursor: Option<&str>,
    ) -> Result<IssuePage, IssueLoadError> {
        let body = request_body(&list_jql(&conn.project_key), cursor, PAGE_SIZE);
        parse_page(&self.post(conn, token, &body)?)
    }

    fn search_open(
        &self,
        conn: &JiraConnection,
        token: &ApiToken,
        text: &str,
    ) -> Result<Vec<Issue>, IssueLoadError> {
        let Some(jql) = search_jql(&conn.project_key, text) else {
            return Ok(Vec::new());
        };
        let body = request_body(&jql, None, SEARCH_SIZE);
        parse_page(&self.post(conn, token, &body)?).map(|page| page.issues)
    }

    fn statuses(
        &self,
        conn: &JiraConnection,
        token: &ApiToken,
        keys: &[String],
    ) -> Result<Vec<KeyedStatus>, IssueLoadError> {
        let mut read = Vec::new();
        for query in statuses_jql(&conn.project_key, keys) {
            read.extend(self.read_chunk(conn, token, &query.keys, 0)?);
        }
        Ok(read)
    }

    fn query_page(
        &self,
        conn: &JiraConnection,
        token: &ApiToken,
        jql: &ScopedQuery,
        cursor: Option<&str>,
    ) -> Result<IssuePage, QueryFailure> {
        let body = request_body(jql.as_str(), cursor, PAGE_SIZE);
        let bytes = self.send(conn, token, &body).map_err(|error| match error {
            PostError::BadRequest(body) => QueryFailure::Rejected(rejection_messages(&body)),
            PostError::Load(error) => QueryFailure::Load(error),
        })?;
        parse_page(&bytes).map_err(QueryFailure::Load)
    }

    fn list_filtered(
        &self,
        conn: &JiraConnection,
        token: &ApiToken,
        filters: &JiraFilters,
        cursor: Option<&str>,
    ) -> Result<IssuePage, IssueLoadError> {
        let jql = narrowed(&list_jql(&conn.project_key), filters);
        let body = request_body(&jql, cursor, PAGE_SIZE);
        parse_page(&self.post(conn, token, &body)?)
    }

    fn search_filtered(
        &self,
        conn: &JiraConnection,
        token: &ApiToken,
        text: &str,
        filters: &JiraFilters,
    ) -> Result<Vec<Issue>, IssueLoadError> {
        let Some(jql) = search_jql(&conn.project_key, text) else {
            return Ok(Vec::new());
        };
        let body = request_body(&narrowed(&jql, filters), None, SEARCH_SIZE);
        parse_page(&self.post(conn, token, &body)?).map(|page| page.issues)
    }

    fn components(
        &self,
        conn: &JiraConnection,
        token: &ApiToken,
    ) -> Result<Vec<String>, IssueLoadError> {
        let path = format!("/rest/api/3/project/{}/components", conn.project_key);
        parse_components(&self.get(conn, token, &path, &[])?)
    }

    fn versions(
        &self,
        conn: &JiraConnection,
        token: &ApiToken,
    ) -> Result<Vec<VersionChoice>, IssueLoadError> {
        let path = format!("/rest/api/3/project/{}/versions", conn.project_key);
        parse_versions(&self.get(conn, token, &path, &[])?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn list_and_search_bodies_ask_for_the_status() {
        for body in [
            request_body("project = ABC", None, PAGE_SIZE),
            request_body("project = ABC", None, SEARCH_SIZE),
        ] {
            let fields = body["fields"].as_array().expect("fields");
            assert!(fields.iter().any(|f| f == "status"), "{fields:?}");
        }
    }
}
