//! Jira connection rules (feature 533): the Jira-specific, render-free part of creating worktrees
//! from Jira issues.
//!
//! A [`JiraConnection`] is the site, project key and account email a project is connected with.
//! Every part is parsed, never trusted: a value that does not parse is refused with the field
//! named (FR-006), and one read back from `settings.json` that no longer parses is dropped. The API
//! token is not here: it lives in an owner-only file beside the settings and never enters
//! `Settings` or the wire (contracts/jira-connection.md §3).

use serde::{Deserialize, Deserializer, Serialize};

use crate::github::{load_pages, Issue, IssueListing, IssueLoadError, IssuePage, GHOST_LOGIN};
use crate::jira_filter::{JiraFilters, VersionChoice};
use crate::jira_query::{QueryFailure, ScopedQuery};
use crate::jira_status::{KeyedStatus, TicketStatus};
use crate::settings::Settings;

/// Which field of a connection a [`FieldProblem`] is about.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JiraField {
    /// The site address.
    Site,
    /// The project key.
    ProjectKey,
    /// The account email.
    Email,
    /// The API token.
    Token,
    /// The app-wide Jira base URL issue links are built on (feature 1122).
    BaseUrl,
}

/// One refused field of a connection, with the text shown beside it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FieldProblem {
    /// The refused field.
    pub field: JiraField,
    /// What to tell the user.
    pub reason: &'static str,
}

/// Refusal text of an unusable site address.
pub const SITE_REFUSED: &str =
    "Enter the https address of your Jira site, e.g. https://acme.atlassian.net.";
/// Refusal text of an unusable project key.
pub const KEY_REFUSED: &str = "Enter the project key, e.g. ABC.";
/// Refusal text of an unusable account email.
pub const EMAIL_REFUSED: &str = "Enter the email of your Atlassian account.";
/// Refusal text of an empty API token.
pub const TOKEN_REFUSED: &str = "Enter an API token.";
/// Refusal text of an unusable Jira base URL.
pub const BASE_URL_REFUSED: &str =
    "Enter the https address of your Jira, e.g. https://acme.atlassian.net.";

/// A Jira site's origin: `scheme://host[:port]`, path and trailing slash dropped.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(transparent)]
pub struct SiteOrigin(String);

impl SiteOrigin {
    /// Parse a site address. `https` only; a development build also accepts a loopback `http`
    /// origin for the manual pass's fake site, a release build never (FR-006, research R10).
    ///
    /// This is also what reads a connection back from the settings file and off the wire, so a
    /// loopback `http` entry written by a development build is dropped by a release one, entry by
    /// entry (`known_values`), never failing the document. On the wire (not self-describing)
    /// an entry cannot be skipped; the service has already dropped such entries, so only a
    /// development daemon feeding a release client could meet one, which is dev-only.
    pub fn parse(input: &str) -> Result<Self, FieldProblem> {
        let refuse = FieldProblem {
            field: JiraField::Site,
            reason: SITE_REFUSED,
        };
        let address = WebAddress::split(input).ok_or(refuse)?;
        match address.scheme.as_str() {
            "https" => {}
            "http" if cfg!(debug_assertions) && is_loopback(&address.host) => {}
            _ => return Err(refuse),
        }
        Ok(Self(address.origin()))
    }

    /// The origin as text.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// A web address split into its parts, with the checks [`SiteOrigin`] and [`JiraBaseUrl`] share:
/// a `scheme://` prefix, no query or fragment, a non-empty authority without user info or
/// whitespace, a port (when given) of 1 to 65535 in digits, and a host whose every dot-separated
/// label is letters, digits and inner hyphens. Scheme and host are lower case; the path is what
/// follows the authority, as written (empty, or starting with `/`).
struct WebAddress<'a> {
    scheme: String,
    host: String,
    port: Option<u16>,
    path: &'a str,
}

impl<'a> WebAddress<'a> {
    fn split(input: &'a str) -> Option<Self> {
        let (scheme, rest) = input.trim().split_once("://")?;
        if rest.contains(['?', '#']) {
            return None;
        }
        let (authority, path) = rest.find('/').map_or((rest, ""), |at| rest.split_at(at));
        if authority.is_empty()
            || authority.contains('@')
            || authority.chars().any(|c| c.is_whitespace())
        {
            return None;
        }
        let (host, port) = match authority.rsplit_once(':') {
            Some((host, port)) => {
                // A real port: digits only, 1 to 65535.
                let port = Some(port)
                    .filter(|p| p.chars().all(|c| c.is_ascii_digit()))
                    .and_then(|p| p.parse::<u16>().ok())
                    .filter(|p| *p != 0)?;
                (host, Some(port))
            }
            None => (authority, None),
        };
        let host = host.to_ascii_lowercase();
        // Every dot-separated label non-empty, made of letters, digits and inner hyphens.
        let label_ok = |label: &str| {
            !label.is_empty()
                && !label.starts_with('-')
                && !label.ends_with('-')
                && label.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
        };
        if !host.split('.').all(label_ok) {
            return None;
        }
        Some(Self {
            scheme: scheme.to_ascii_lowercase(),
            host,
            port,
            path,
        })
    }

    /// `scheme://host[:port]`.
    fn origin(&self) -> String {
        match self.port {
            Some(port) => format!("{}://{}:{port}", self.scheme, self.host),
            None => format!("{}://{}", self.scheme, self.host),
        }
    }
}

fn is_loopback(host: &str) -> bool {
    host == "localhost" || host == "127.0.0.1"
}

impl std::fmt::Display for SiteOrigin {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// A project's Jira base URL its issue links are built on (feature 1122, FR-011):
/// `https://host[:port][/path]`, scheme and host lower case, the path as written, no trailing `/`,
/// no user info, query or fragment. Serialized as its string; deserialized through [`Self::parse`].
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize)]
#[serde(transparent)]
pub struct JiraBaseUrl(String);

impl JiraBaseUrl {
    /// Parse a base URL, trimmed. `https` only, in every build.
    pub fn parse(input: &str) -> Result<Self, FieldProblem> {
        let refuse = FieldProblem {
            field: JiraField::BaseUrl,
            reason: BASE_URL_REFUSED,
        };
        let address = WebAddress::split(input).ok_or(refuse)?;
        if address.scheme != "https" || address.path.chars().any(char::is_whitespace) {
            return Err(refuse);
        }
        Ok(Self(format!(
            "{}{}",
            address.origin(),
            address.path.trim_end_matches('/')
        )))
    }

    /// The base URL as text, without a trailing `/`.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl<'de> Deserialize<'de> for JiraBaseUrl {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let text = String::deserialize(deserializer)?;
        Self::parse(&text).map_err(|problem| serde::de::Error::custom(problem.reason))
    }
}

/// A Jira project key: `[A-Z][A-Z0-9_]{1,9}`.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(transparent)]
pub struct ProjectKey(String);

impl ProjectKey {
    /// Parse a key: trimmed, upper-cased, then checked against Atlassian's key rule.
    pub fn parse(input: &str) -> Result<Self, FieldProblem> {
        let key = input.trim().to_ascii_uppercase();
        let mut chars = key.chars();
        let valid = chars.next().is_some_and(|c| c.is_ascii_uppercase())
            && (2..=10).contains(&key.len())
            && chars.all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '_');
        if valid {
            Ok(Self(key))
        } else {
            Err(FieldProblem {
                field: JiraField::ProjectKey,
                reason: KEY_REFUSED,
            })
        }
    }

    /// The key as text.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for ProjectKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// An Atlassian account email.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(transparent)]
pub struct AccountEmail(String);

impl AccountEmail {
    /// Parse an email: one `@`, a non-empty local part, a domain with a dot, no whitespace.
    pub fn parse(input: &str) -> Result<Self, FieldProblem> {
        let email = input.trim();
        let valid = !email.chars().any(char::is_whitespace)
            && email.matches('@').count() == 1
            && email.split_once('@').is_some_and(|(local, domain)| {
                !local.is_empty()
                    && domain.contains('.')
                    && !domain.starts_with('.')
                    && !domain.ends_with('.')
            });
        if valid {
            Ok(Self(email.to_owned()))
        } else {
            Err(FieldProblem {
                field: JiraField::Email,
                reason: EMAIL_REFUSED,
            })
        }
    }

    /// The email as text.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for AccountEmail {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// What a project is connected to Jira with, minus the token. Built only through
/// [`JiraConnection::parse`]; deserialisation re-validates.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "RawConnection")]
pub struct JiraConnection {
    /// The site.
    pub site: SiteOrigin,
    /// The project whose issues are listed.
    pub project_key: ProjectKey,
    /// The account the token belongs to.
    pub email: AccountEmail,
}

#[derive(Deserialize)]
struct RawConnection {
    site: String,
    project_key: String,
    email: String,
}

impl TryFrom<RawConnection> for JiraConnection {
    type Error = String;

    fn try_from(raw: RawConnection) -> Result<Self, String> {
        Self::parse(&raw.site, &raw.project_key, &raw.email).map_err(|problems| {
            problems
                .first()
                .map(|p| p.reason.to_owned())
                .unwrap_or_default()
        })
    }
}

impl JiraConnection {
    /// Parse all three parts, reporting every refused field at once (FR-006).
    pub fn parse(site: &str, project_key: &str, email: &str) -> Result<Self, Vec<FieldProblem>> {
        let site = SiteOrigin::parse(site);
        let key = ProjectKey::parse(project_key);
        let email = AccountEmail::parse(email);
        match (site, key, email) {
            (Ok(site), Ok(project_key), Ok(email)) => Ok(Self {
                site,
                project_key,
                email,
            }),
            (site, key, email) => Err([site.err(), key.err(), email.err()]
                .into_iter()
                .flatten()
                .collect()),
        }
    }
}

/// An Atlassian API token. Never serialised, never displayed: `Debug` redacts it and
/// [`ApiToken::expose`] is the one way to the text, used only where a request is built
/// (FR-009, contracts/jira-connection.md §3).
#[derive(Clone, PartialEq, Eq)]
pub struct ApiToken(String);

impl ApiToken {
    /// Parse a token: trimmed, and not empty (FR-006).
    pub fn parse(input: &str) -> Result<Self, FieldProblem> {
        let token = input.trim();
        if token.is_empty() {
            Err(FieldProblem {
                field: JiraField::Token,
                reason: TOKEN_REFUSED,
            })
        } else {
            Ok(Self(token.to_owned()))
        }
    }

    /// The token text, for the one place that builds a request or writes the token file.
    pub fn expose(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Debug for ApiToken {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("ApiToken(<redacted>)")
    }
}

/// What saving the Settings form does to one project's connection (feature 533, US3).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum ConnectionAction {
    /// The form leaves the connection as it is.
    #[default]
    Keep,
    /// Store this connection, and the token when a new one was typed.
    Save {
        /// The project the connection belongs to.
        project: std::path::PathBuf,
        /// The validated connection.
        connection: JiraConnection,
        /// A newly typed token; `None` keeps the stored one.
        token: Option<ApiToken>,
    },
    /// Delete the project's connection and its stored token.
    Remove {
        /// The project whose connection goes.
        project: std::path::PathBuf,
    },
}

/// The entries of a per-project map (Jira connections, base URLs) that still parse, entry by
/// entry; a value that is not a map reads as empty.
/// One bad hand edit must not send the whole document to `.bak`.
pub(crate) fn known_values<'de, D, T>(
    deserializer: D,
) -> Result<std::collections::BTreeMap<String, T>, D::Error>
where
    D: Deserializer<'de>,
    T: serde::de::DeserializeOwned,
{
    Ok(match serde_json::Value::deserialize(deserializer)? {
        serde_json::Value::Object(entries) => entries
            .into_iter()
            .filter_map(|(project, value)| serde_json::from_value(value).ok().map(|c| (project, c)))
            .collect(),
        _ => Default::default(),
    })
}

/// The key a project's connection is stored under in `Settings::jira_connections`: its path, as
/// the wire names the project elsewhere.
pub fn connection_key(project: &std::path::Path) -> String {
    project.to_string_lossy().into_owned()
}

/// The one gate every Jira control and request checks: the integration is on (FR-001).
pub fn jira_visible(settings: &Settings) -> bool {
    settings.jira_enabled
}

/// Which tracker a project's issues come from (feature 533): GitHub unless Jira is chosen.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum IssueProvider {
    /// The repository's GitHub issues.
    #[default]
    Github,
    /// The connected Jira project's issues.
    Jira,
}

/// The JQL for the open issues of `key`'s project, most recently updated first
/// (contracts/jira-cloud-source.md §3).
pub fn list_jql(key: &ProjectKey) -> String {
    format!("project = \"{key}\" AND statusCategory != Done ORDER BY updated DESC")
}

/// Longest search text a query carries, in characters.
const SEARCH_TEXT_MAX_CHARS: usize = 100;

/// The JQL that searches `key`'s open issues for `text`, or `None` when `text` is empty once
/// cleaned: control characters are removed, the text trimmed and cut at 100 characters, then `\`
/// and `"` escaped, so the text is only ever a JQL string. A text that is an issue key of this
/// project adds a `key = ` clause (contracts/jira-cloud-source.md §3).
pub fn search_jql(key: &ProjectKey, text: &str) -> Option<String> {
    let cleaned: String = text.chars().filter(|c| !c.is_control()).collect();
    let cleaned: String = cleaned.trim().chars().take(SEARCH_TEXT_MAX_CHARS).collect();
    let cleaned = cleaned.trim_end();
    if cleaned.is_empty() {
        return None;
    }
    let escaped = cleaned.replace('\\', "\\\\").replace('"', "\\\"");
    let upper = cleaned.to_ascii_uppercase();
    let key_clause = if is_key_of(&upper, key) {
        format!(" OR key = \"{upper}\"")
    } else {
        String::new()
    };
    Some(format!(
        "project = \"{key}\" AND statusCategory != Done AND \
         (summary ~ \"{escaped}*\" OR labels = \"{escaped}\"{key_clause}) ORDER BY updated DESC"
    ))
}

/// `text` (upper-cased) is `<KEY>-<digits>` of this project.
fn is_key_of(text: &str, key: &ProjectKey) -> bool {
    text.rsplit_once('-').is_some_and(|(prefix, digits)| {
        prefix == key.as_str() && !digits.is_empty() && digits.chars().all(|c| c.is_ascii_digit())
    })
}

/// Labels held per issue, as for GitHub.
const LABELS_PER_ISSUE: usize = 20;
/// Deepest ADF nesting flattened; deeper content is dropped.
const ADF_MAX_DEPTH: usize = 32;
/// Most text taken from one ADF document: far more than a description keeps.
const ADF_MAX_BYTES: usize = 8 * 1024;

/// Parse the body of `POST /rest/api/3/search/jql` into a page (contracts/jira-cloud-source.md
/// §4). A body that is not JSON or has no `issues` array is [`IssueLoadError::NotJiraCloud`]; a
/// node whose key is not `<PROJECT>-<digits>` is skipped. Jira has no total, so `total_open` is the
/// number of issues on the page.
pub fn parse_page(body: &[u8]) -> Result<IssuePage, IssueLoadError> {
    let json: serde_json::Value =
        serde_json::from_slice(body).map_err(|_| IssueLoadError::NotJiraCloud)?;
    let nodes = json["issues"]
        .as_array()
        .ok_or(IssueLoadError::NotJiraCloud)?;
    let issues: Vec<Issue> = nodes.iter().filter_map(issue_from_node).collect();
    let last = json["isLast"].as_bool().unwrap_or(false);
    let next_cursor = json["nextPageToken"]
        .as_str()
        .filter(|token| !last && !token.is_empty())
        .map(str::to_owned);
    Ok(IssuePage {
        total_open: issues.len() as u64,
        issues,
        next_cursor,
    })
}

/// One issue node, or `None` when its key or summary is unusable.
fn issue_from_node(node: &serde_json::Value) -> Option<Issue> {
    let key = node["key"].as_str()?;
    let (project, digits) = key.rsplit_once('-')?;
    if project.is_empty() || digits.is_empty() || !digits.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    let fields = &node["fields"];
    let labels = fields["labels"]
        .as_array()
        .map(|labels| {
            labels
                .iter()
                .filter_map(|label| label.as_str().map(str::to_owned))
                .take(LABELS_PER_ISSUE)
                .collect()
        })
        .unwrap_or_default();
    let mut issue = Issue::jira(
        key,
        digits.parse().ok()?,
        fields["summary"].as_str()?.to_owned(),
        labels,
        fields["updated"].as_str().unwrap_or_default().to_owned(),
    );
    if let Some(name) = fields["issuetype"]["name"].as_str() {
        issue = issue.with_type_name(name);
    }
    issue = issue.with_status(TicketStatus::from_jira(
        fields["status"]["name"].as_str(),
        fields["status"]["statusCategory"]["key"].as_str(),
    ));
    let mut text = String::new();
    flatten_adf(&fields["description"], 0, &mut text);
    Some(
        issue
            .reported_by(&reporter_of(&fields["reporter"]))
            .described(&text),
    )
}

/// The reporter's display name, else the local part of their email, else [`GHOST_LOGIN`]; never
/// the account id.
fn reporter_of(reporter: &serde_json::Value) -> String {
    let name = reporter["displayName"]
        .as_str()
        .map(str::trim)
        .unwrap_or_default();
    if !name.is_empty() {
        return name.to_owned();
    }
    reporter["emailAddress"]
        .as_str()
        .and_then(|email| email.split_once('@'))
        .map(|(local, _)| local.trim())
        .filter(|local| !local.is_empty())
        .unwrap_or(GHOST_LOGIN)
        .to_owned()
}

/// Walk an Atlassian Document Format node, taking `text` nodes and a space for every `hardBreak`
/// and every end of a node with content (a paragraph, a list item), so words of neighbouring
/// blocks do not run together.
fn flatten_adf(node: &serde_json::Value, depth: usize, out: &mut String) {
    if depth > ADF_MAX_DEPTH || out.len() >= ADF_MAX_BYTES {
        return;
    }
    match node["type"].as_str() {
        Some("text") => out.push_str(node["text"].as_str().unwrap_or_default()),
        Some("hardBreak") => out.push(' '),
        _ => {
            if let Some(children) = node["content"].as_array() {
                for child in children {
                    flatten_adf(child, depth + 1, out);
                }
                out.push(' ');
            }
        }
    }
}

/// What a Jira request came to, before any body is read: the input of [`classify`]. The shell
/// maps its HTTP client's result onto it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    /// Connect, DNS, TLS or proxy failure, with the transport's reason.
    Unreachable(String),
    /// No answer within 10 seconds.
    TimedOut,
    /// The site answered with a redirect, which is not followed.
    Redirected,
    /// An HTTP status other than success.
    Status(u16),
    /// A success whose body is not a Jira Cloud listing.
    BadBody,
}

/// Longest transport reason a notice carries, in characters.
const REASON_MAX_CHARS: usize = 80;

/// The failure a request outcome is (contracts/jira-cloud-source.md §6).
pub fn classify(outcome: &Outcome) -> IssueLoadError {
    match outcome {
        Outcome::Unreachable(reason) => {
            IssueLoadError::Unreachable(reason.chars().take(REASON_MAX_CHARS).collect())
        }
        Outcome::TimedOut => IssueLoadError::TimedOut,
        Outcome::Redirected | Outcome::BadBody => IssueLoadError::NotJiraCloud,
        Outcome::Status(401 | 403) => IssueLoadError::CredentialsRefused,
        Outcome::Status(400 | 404) => IssueLoadError::NoAccess,
        Outcome::Status(429) => IssueLoadError::RateLimited,
        Outcome::Status(status) => {
            IssueLoadError::Other(format!("Jira returned an error ({status})."))
        }
    }
}

/// Where a project's Jira issues come from: the client's HTTP source in production,
/// [`FakeJiraSource`] in tests. Paging, cap and classification are pure functions of this module.
pub trait JiraSource: Send + Sync {
    /// One page of open issues, most recently updated first; `cursor` is Jira's `nextPageToken`.
    fn list_open(
        &self,
        conn: &JiraConnection,
        token: &ApiToken,
        cursor: Option<&str>,
    ) -> Result<IssuePage, IssueLoadError>;

    /// Open issues matching `text` on Jira's side; the caller applies the local match rule.
    fn search_open(
        &self,
        conn: &JiraConnection,
        token: &ApiToken,
        text: &str,
    ) -> Result<Vec<Issue>, IssueLoadError>;

    /// The status of each of `keys` (at most 100) of the connected project; a key Jira does not
    /// return is absent from the answer (not found or not visible). Done tickets are read too.
    fn statuses(
        &self,
        _conn: &JiraConnection,
        _token: &ApiToken,
        _keys: &[String],
    ) -> Result<Vec<KeyedStatus>, IssueLoadError> {
        Ok(Vec::new())
    }

    /// One page of the issues `jql` lists, in its order; `cursor` is Jira's `nextPageToken`. A 400
    /// is `QueryFailure::Rejected` with Jira's messages (feature 1186).
    fn query_page(
        &self,
        _conn: &JiraConnection,
        _token: &ApiToken,
        _jql: &ScopedQuery,
        _cursor: Option<&str>,
    ) -> Result<IssuePage, QueryFailure> {
        Err(QueryFailure::Load(IssueLoadError::Other(
            "this Jira source cannot run a query".into(),
        )))
    }

    /// One page of open issues narrowed by `filters` (feature 1185). A source that does not
    /// filter answers [`JiraSource::list_open`] while no filter is chosen and refuses otherwise.
    fn list_filtered(
        &self,
        conn: &JiraConnection,
        token: &ApiToken,
        filters: &JiraFilters,
        cursor: Option<&str>,
    ) -> Result<IssuePage, IssueLoadError> {
        if filters.is_empty() {
            self.list_open(conn, token, cursor)
        } else {
            Err(IssueLoadError::Other(FILTERS_UNSUPPORTED.into()))
        }
    }

    /// Open issues matching `text` narrowed by `filters` (feature 1185); defaults as
    /// [`JiraSource::list_filtered`].
    fn search_filtered(
        &self,
        conn: &JiraConnection,
        token: &ApiToken,
        text: &str,
        filters: &JiraFilters,
    ) -> Result<Vec<Issue>, IssueLoadError> {
        if filters.is_empty() {
            self.search_open(conn, token, text)
        } else {
            Err(IssueLoadError::Other(FILTERS_UNSUPPORTED.into()))
        }
    }

    /// The connected project's component names, in Jira's order (feature 1185).
    fn components(
        &self,
        _conn: &JiraConnection,
        _token: &ApiToken,
    ) -> Result<Vec<String>, IssueLoadError> {
        Ok(Vec::new())
    }

    /// The connected project's unarchived fix versions, unreleased first (feature 1185).
    fn versions(
        &self,
        _conn: &JiraConnection,
        _token: &ApiToken,
    ) -> Result<Vec<VersionChoice>, IssueLoadError> {
        Ok(Vec::new())
    }
}

/// What a source that cannot filter answers a filtered request with (research R10).
pub const FILTERS_UNSUPPORTED: &str = "filters are not supported by this source";

/// Load a project's open issues page by page up to the load cap, through the paging rule GitHub
/// shares. Jira has no total, so `total_open` is the number held and the listing is `complete`
/// only when the last page left no cursor.
pub fn load(
    source: &dyn JiraSource,
    conn: &JiraConnection,
    token: &ApiToken,
) -> Result<IssueListing, IssueLoadError> {
    let paged = load_pages(|cursor| source.list_open(conn, token, cursor))?;
    Ok(IssueListing {
        total_open: paged.issues.len() as u64,
        complete: paged.exhausted,
        issues: paged.issues,
    })
}

type Script<T, E = IssueLoadError> = std::sync::Mutex<std::collections::VecDeque<Result<T, E>>>;
/// [`load`] over [`JiraSource::list_filtered`]: the same paging and cap, narrowed by `filters`
/// (FR-007).
pub fn load_filtered(
    source: &dyn JiraSource,
    conn: &JiraConnection,
    token: &ApiToken,
    filters: &JiraFilters,
) -> Result<IssueListing, IssueLoadError> {
    let paged = load_pages(|cursor| source.list_filtered(conn, token, filters, cursor))?;
    Ok(IssueListing {
        total_open: paged.issues.len() as u64,
        complete: paged.exhausted,
        issues: paged.issues,
    })
}

/// What a filtered request of [`FakeJiraSource`] asked beyond its clauses.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FilteredAsk {
    /// A listing page, from this cursor.
    List(Option<String>),
    /// A search for this text.
    Search(String),
}

/// A choice request [`FakeJiraSource`] received (feature 1185).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChoiceCall {
    /// The project's components.
    Components,
    /// The project's fix versions.
    Versions,
}

fn locked<T>(mutex: &std::sync::Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

/// A scripted [`JiraSource`] for tests: answers each call with the next scripted page, search
/// result or error, and records every call. Public (not `#[cfg(test)]`) so every crate's tests can
/// use it, like [`crate::github::FakeIssueSource`].
#[derive(Debug, Default)]
pub struct FakeJiraSource {
    pages: Script<IssuePage>,
    calls: std::sync::Mutex<Vec<Option<String>>>,
    searches: Script<Vec<Issue>>,
    search_calls: std::sync::Mutex<Vec<String>>,
    statuses: Script<Vec<KeyedStatus>>,
    status_calls: std::sync::Mutex<Vec<Vec<String>>>,
    queries: Script<IssuePage, QueryFailure>,
    query_calls: std::sync::Mutex<Vec<(String, Option<String>)>>,
    filtered_calls: std::sync::Mutex<Vec<(Vec<String>, FilteredAsk)>>,
    components: Script<Vec<String>>,
    versions: Script<Vec<VersionChoice>>,
    choice_calls: std::sync::Mutex<Vec<ChoiceCall>>,
}

impl FakeJiraSource {
    /// A source with nothing scripted.
    pub fn new() -> Self {
        Self::default()
    }

    /// Answer the next unanswered `list_open` call with `page`.
    pub fn with_page(self, page: IssuePage) -> Self {
        locked(&self.pages).push_back(Ok(page));
        self
    }

    /// Answer the next unanswered `list_open` call with `error`.
    pub fn with_error(self, error: IssueLoadError) -> Self {
        locked(&self.pages).push_back(Err(error));
        self
    }

    /// Answer the next unanswered `search_open` call with `result`.
    pub fn with_search(self, result: Result<Vec<Issue>, IssueLoadError>) -> Self {
        locked(&self.searches).push_back(result);
        self
    }

    /// Answer the next unanswered `statuses` call with `result`. With none scripted left, the last
    /// answer scripted is not repeated: a call then answers an empty list.
    pub fn with_statuses(self, result: Result<Vec<KeyedStatus>, IssueLoadError>) -> Self {
        locked(&self.statuses).push_back(result);
        self
    }

    /// Answer the next unanswered `statuses` call, after construction (a test changes a ticket).
    pub fn script_statuses(&self, result: Result<Vec<KeyedStatus>, IssueLoadError>) {
        locked(&self.statuses).push_back(result);
    }

    /// Every `statuses` call so far, as its key list.
    pub fn status_calls(&self) -> Vec<Vec<String>> {
        locked(&self.status_calls).clone()
    }

    /// Every `list_open` call so far, as its cursor.
    pub fn calls(&self) -> Vec<Option<String>> {
        locked(&self.calls).clone()
    }

    /// Every `search_open` call so far, as its text.
    pub fn search_calls(&self) -> Vec<String> {
        locked(&self.search_calls).clone()
    }

    /// Answer the next unanswered `query_page` call with `result` (feature 1186).
    pub fn with_query(self, result: Result<IssuePage, QueryFailure>) -> Self {
        locked(&self.queries).push_back(result);
        self
    }

    /// Every `query_page` call so far, as its JQL and cursor.
    pub fn query_calls(&self) -> Vec<(String, Option<String>)> {
        locked(&self.query_calls).clone()
    }

    /// Every `list_filtered` / `search_filtered` call so far, as its clauses and what it asked.
    /// Each also counts as the `list_open` / `search_open` call it is answered as.
    pub fn filtered_calls(&self) -> Vec<(Vec<String>, FilteredAsk)> {
        locked(&self.filtered_calls).clone()
    }

    /// Answer the next unanswered `components` call with `result`.
    pub fn with_components(self, result: Result<Vec<String>, IssueLoadError>) -> Self {
        self.script_components(result);
        self
    }

    /// Answer the next unanswered `components` call, after construction.
    pub fn script_components(&self, result: Result<Vec<String>, IssueLoadError>) {
        locked(&self.components).push_back(result);
    }

    /// Answer the next unanswered `versions` call with `result`.
    pub fn with_versions(self, result: Result<Vec<VersionChoice>, IssueLoadError>) -> Self {
        self.script_versions(result);
        self
    }

    /// Answer the next unanswered `versions` call, after construction.
    pub fn script_versions(&self, result: Result<Vec<VersionChoice>, IssueLoadError>) {
        locked(&self.versions).push_back(result);
    }

    /// Every choice request so far.
    pub fn choice_calls(&self) -> Vec<ChoiceCall> {
        locked(&self.choice_calls).clone()
    }
}

impl JiraSource for FakeJiraSource {
    fn list_open(
        &self,
        _conn: &JiraConnection,
        _token: &ApiToken,
        cursor: Option<&str>,
    ) -> Result<IssuePage, IssueLoadError> {
        locked(&self.calls).push(cursor.map(str::to_owned));
        locked(&self.pages).pop_front().unwrap_or_else(|| {
            Err(IssueLoadError::Other(
                "FakeJiraSource: no page scripted for this call".into(),
            ))
        })
    }

    fn search_open(
        &self,
        _conn: &JiraConnection,
        _token: &ApiToken,
        text: &str,
    ) -> Result<Vec<Issue>, IssueLoadError> {
        locked(&self.search_calls).push(text.to_owned());
        locked(&self.searches).pop_front().unwrap_or_else(|| {
            Err(IssueLoadError::Other(
                "FakeJiraSource: no search scripted for this call".into(),
            ))
        })
    }

    fn statuses(
        &self,
        _conn: &JiraConnection,
        _token: &ApiToken,
        keys: &[String],
    ) -> Result<Vec<KeyedStatus>, IssueLoadError> {
        locked(&self.status_calls).push(keys.to_vec());
        locked(&self.statuses).pop_front().unwrap_or(Ok(Vec::new()))
    }

    fn query_page(
        &self,
        _conn: &JiraConnection,
        _token: &ApiToken,
        jql: &ScopedQuery,
        cursor: Option<&str>,
    ) -> Result<IssuePage, QueryFailure> {
        locked(&self.query_calls).push((jql.as_str().to_owned(), cursor.map(str::to_owned)));
        locked(&self.queries).pop_front().unwrap_or_else(|| {
            Err(QueryFailure::Load(IssueLoadError::Other(
                "FakeJiraSource: no query scripted for this call".into(),
            )))
        })
    }

    fn list_filtered(
        &self,
        conn: &JiraConnection,
        token: &ApiToken,
        filters: &JiraFilters,
        cursor: Option<&str>,
    ) -> Result<IssuePage, IssueLoadError> {
        locked(&self.filtered_calls).push((
            filters.clauses(),
            FilteredAsk::List(cursor.map(str::to_owned)),
        ));
        self.list_open(conn, token, cursor)
    }

    fn search_filtered(
        &self,
        conn: &JiraConnection,
        token: &ApiToken,
        text: &str,
        filters: &JiraFilters,
    ) -> Result<Vec<Issue>, IssueLoadError> {
        locked(&self.filtered_calls)
            .push((filters.clauses(), FilteredAsk::Search(text.to_owned())));
        self.search_open(conn, token, text)
    }

    fn components(
        &self,
        _conn: &JiraConnection,
        _token: &ApiToken,
    ) -> Result<Vec<String>, IssueLoadError> {
        locked(&self.choice_calls).push(ChoiceCall::Components);
        locked(&self.components).pop_front().unwrap_or_else(|| {
            Err(IssueLoadError::Other(
                "FakeJiraSource: no components scripted for this call".into(),
            ))
        })
    }

    fn versions(
        &self,
        _conn: &JiraConnection,
        _token: &ApiToken,
    ) -> Result<Vec<VersionChoice>, IssueLoadError> {
        locked(&self.choice_calls).push(ChoiceCall::Versions);
        locked(&self.versions).pop_front().unwrap_or_else(|| {
            Err(IssueLoadError::Other(
                "FakeJiraSource: no versions scripted for this call".into(),
            ))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_base_url_is_stored_without_its_trailing_slash_scheme_and_host_lower_case() {
        for (input, stored) in [
            (
                "https://example.atlassian.net",
                "https://example.atlassian.net",
            ),
            (
                "https://example.atlassian.net/",
                "https://example.atlassian.net",
            ),
            (
                "  https://example.atlassian.net/ ",
                "https://example.atlassian.net",
            ),
            (
                "https://jira.example.com/jira/",
                "https://jira.example.com/jira",
            ),
            (
                "https://jira.example.com:8443/jira",
                "https://jira.example.com:8443/jira",
            ),
            ("HTTPS://Example.Com", "https://example.com"),
            (
                "https://Jira.Example.com/Jira//",
                "https://jira.example.com/Jira",
            ),
        ] {
            let parsed = JiraBaseUrl::parse(input).unwrap_or_else(|_| panic!("{input} accepted"));
            assert_eq!(parsed.as_str(), stored, "{input}");
        }
    }

    #[test]
    fn a_base_url_that_is_not_an_https_address_is_refused_on_its_field() {
        for input in [
            "http://example.com",
            "http://localhost:8080",
            "ftp://example.com",
            "example.com",
            "https://",
            "https:///jira",
            "https://user@host",
            "https://host?x=1",
            "https://host#a",
            "https://host/path?x",
            "https://host/jira#frag",
            "https://a..b",
            "https://-a.com",
            "https://a_b.com",
            "https://host:0",
            "https://host:80a",
            "https://host:",
            "",
        ] {
            let problem = JiraBaseUrl::parse(input).expect_err(input);
            assert_eq!(
                problem,
                FieldProblem {
                    field: JiraField::BaseUrl,
                    reason: BASE_URL_REFUSED
                },
                "{input}"
            );
        }
    }

    #[test]
    fn a_base_url_is_written_as_its_string_and_read_through_parse() {
        let url = JiraBaseUrl::parse("https://example.atlassian.net/").expect("valid");
        let json = serde_json::to_string(&url).expect("serializes");
        assert_eq!(json, r#""https://example.atlassian.net""#);
        let back: JiraBaseUrl = serde_json::from_str(r#""https://Example.atlassian.net/""#)
            .expect("a valid string reads back");
        assert_eq!(back, url);
        assert!(serde_json::from_str::<JiraBaseUrl>(r#""http://example.com""#).is_err());
    }
}
