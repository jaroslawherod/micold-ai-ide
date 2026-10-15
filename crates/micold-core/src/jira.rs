//! Jira connection rules (feature 533): the Jira-specific, render-free part of creating worktrees
//! from Jira issues.
//!
//! A [`JiraConnection`] is the site, project key and account email a project is connected with.
//! Every part is parsed, never trusted: a value that does not parse is refused with the field
//! named (FR-006), and one read back from `settings.json` that no longer parses is dropped. The API
//! token is not here: it lives in an owner-only file beside the settings and never enters
//! `Settings` or the wire (contracts/jira-connection.md §3).

use serde::{Deserialize, Deserializer, Serialize};

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
    /// entry (`known_connections`), never failing the document. On the wire (not self-describing)
    /// an entry cannot be skipped; the service has already dropped such entries, so only a
    /// development daemon feeding a release client could meet one, which is dev-only.
    pub fn parse(input: &str) -> Result<Self, FieldProblem> {
        let refuse = FieldProblem {
            field: JiraField::Site,
            reason: SITE_REFUSED,
        };
        let input = input.trim();
        let (scheme, rest) = input.split_once("://").ok_or(refuse)?;
        let scheme = scheme.to_ascii_lowercase();
        if rest.contains(['?', '#']) {
            return Err(refuse);
        }
        let authority = rest.split('/').next().unwrap_or("");
        if authority.is_empty()
            || authority.contains('@')
            || authority.chars().any(|c| c.is_whitespace())
        {
            return Err(refuse);
        }
        let (host, port) = match authority.rsplit_once(':') {
            Some((host, port)) => {
                // A real port: digits only, 1 to 65535.
                let port = Some(port)
                    .filter(|p| p.chars().all(|c| c.is_ascii_digit()))
                    .and_then(|p| p.parse::<u16>().ok())
                    .filter(|p| *p != 0)
                    .ok_or(refuse)?;
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
            return Err(refuse);
        }
        match scheme.as_str() {
            "https" => {}
            "http" if cfg!(debug_assertions) && is_loopback(&host) => {}
            _ => return Err(refuse),
        }
        Ok(Self(match port {
            Some(port) => format!("{scheme}://{host}:{port}"),
            None => format!("{scheme}://{host}"),
        }))
    }

    /// The origin as text.
    pub fn as_str(&self) -> &str {
        &self.0
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

/// The connections that still parse, entry by entry; a value that is not a map reads as empty.
/// One bad hand edit must not send the whole document to `.bak`.
pub(crate) fn known_connections<'de, D>(
    deserializer: D,
) -> Result<std::collections::BTreeMap<String, JiraConnection>, D::Error>
where
    D: Deserializer<'de>,
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
