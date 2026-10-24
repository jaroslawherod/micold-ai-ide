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
//! Depends on `micold_core`, `ureq`, `base64` and `serde_json` alone, so the adapter test includes
//! this file by path.

use std::time::Duration;

use base64::Engine;
use micold_core::github::{Issue, IssueLoadError, IssuePage};
use micold_core::jira::{
    classify, list_jql, parse_page, search_jql, ApiToken, JiraConnection, JiraSource, Outcome,
};
use micold_core::jira_query::{QueryFailure, ScopedQuery};
use micold_core::jira_status::{parse_statuses, status_jql, statuses_jql, KeyedStatus};

/// How many times a refused status request is halved to find the key Jira refuses: 2^7 covers a
/// chunk of 100 (contracts/jira-status-source.md §3).
const MAX_BISECTION_DEPTH: u8 = 7;

/// Why one request failed: a `400` is told apart, because a status request answers it by halving.
enum PostError {
    BadRequest,
    Load(IssueLoadError),
}

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
        self.send(conn, token, body).map_err(|error| match error {
            PostError::BadRequest => classify(&Outcome::Status(400)),
            PostError::Load(error) => error,
        })
    }

    /// [`Self::post`], keeping a `400` apart from the other failures.
    fn send(
        &self,
        conn: &JiraConnection,
        token: &ApiToken,
        body: &serde_json::Value,
    ) -> Result<Vec<u8>, PostError> {
        let load = |outcome: Outcome| PostError::Load(classify(&outcome));
        let url = format!("{}/rest/api/3/search/jql", conn.site.as_str());
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
        let sent = agent
            .post(&url)
            .header("Authorization", format!("Basic {credentials}"))
            .header("Accept", "application/json")
            .header("Content-Type", "application/json")
            .header("User-Agent", "micold-ai-ide")
            .send(body.to_string().as_bytes());
        let mut response = match sent {
            Ok(response) => response,
            Err(error) => return Err(load(outcome_of(&error, token))),
        };
        let status = response.status().as_u16();
        match status {
            200..=299 => {}
            300..=399 => return Err(load(Outcome::Redirected)),
            400 => return Err(PostError::BadRequest),
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
            Err(PostError::BadRequest) if keys.len() == 1 => Ok(Vec::new()),
            Err(PostError::BadRequest) if depth >= MAX_BISECTION_DEPTH => {
                Err(classify(&Outcome::Status(400)))
            }
            Err(PostError::BadRequest) => {
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
        let bytes = self.post(conn, token, &body).map_err(QueryFailure::Load)?;
        parse_page(&bytes).map_err(QueryFailure::Load)
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
