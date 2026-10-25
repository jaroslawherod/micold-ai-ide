//! Jira issue selector filters (feature 1185): the render-free part of narrowing the Jira issue
//! picker by reporter, label, component and fix version.
//!
//! [`JiraFilters`] holds at most one value per filter and turns them into ANDed JQL clauses
//! ([`JiraFilters::clauses`]), every value a quoted, escaped string sent whole ([`jql_string`]).
//! [`narrowed`] adds those clauses to the output of [`crate::jira::list_jql`] or
//! [`crate::jira::search_jql`] without editing either (contracts/jira-filter-source.md §1). A
//! reporter is named only by its [`AccountId`], which can only be built by parsing and never
//! prints in `Debug`, so neither a display name nor an account reaches a log (FR-006, FR-010).
//!
//! Choices are parsed here too: [`parse_components`] reads a project's components (§4).

use crate::github::IssueLoadError;

/// A value Jira may send and this feature may show or send: non-empty, no control character
/// (FR-010).
fn is_usable(value: &str) -> bool {
    !value.is_empty() && !value.chars().any(char::is_control)
}

/// A Jira Cloud account id. Built only by parsing; `Debug` redacts it; never serialised.
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct AccountId(String);

impl AccountId {
    /// The id, when `text` is non-empty and holds no control character.
    pub fn parse(text: &str) -> Option<Self> {
        is_usable(text).then(|| Self(text.to_owned()))
    }

    /// The id as Jira knows it: used only where a clause or a request is built.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Debug for AccountId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("AccountId(<redacted>)")
    }
}

/// A Jira user offered as a reporter (data-model §2). Equal by account only: two people sharing a
/// display name stay distinct.
#[derive(Clone)]
pub struct JiraUser {
    /// Who the person is; what a reporter clause names.
    pub account: AccountId,
    /// The display name shown in the picker.
    pub name: String,
    /// The connected account's own user.
    pub is_me: bool,
}

impl JiraUser {
    /// How the person is shown: the display name, or "You (name)" for the own user.
    pub fn label(&self) -> String {
        if self.is_me {
            format!("You ({})", self.name)
        } else {
            self.name.clone()
        }
    }
}

impl PartialEq for JiraUser {
    fn eq(&self, other: &Self) -> bool {
        self.account == other.account
    }
}

impl Eq for JiraUser {}

impl std::fmt::Debug for JiraUser {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("JiraUser(<redacted>)")
    }
}

/// One of the four filters, in the fixed order of clauses, the active line and the filter row.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FilterKind {
    /// Who reported the issue.
    Reporter,
    /// One of the issue's labels.
    Label,
    /// One of the issue's components.
    Component,
    /// One of the issue's fix versions.
    FixVersion,
}

impl FilterKind {
    /// Every filter, in the fixed order.
    pub const ALL: [FilterKind; 4] = [
        FilterKind::Reporter,
        FilterKind::Label,
        FilterKind::Component,
        FilterKind::FixVersion,
    ];

    /// The filter's name inside a sentence: "component", "fix version".
    pub fn noun(self) -> &'static str {
        match self {
            FilterKind::Reporter => "reporter",
            FilterKind::Label => "label",
            FilterKind::Component => "component",
            FilterKind::FixVersion => "fix version",
        }
    }

    /// The filter's field label: "Component", "Fix version".
    pub fn title(self) -> &'static str {
        match self {
            FilterKind::Reporter => "Reporter",
            FilterKind::Label => "Label",
            FilterKind::Component => "Component",
            FilterKind::FixVersion => "Fix version",
        }
    }
}

/// The chosen filter values: at most one per filter (FR-002), each usable (FR-010). Held only by
/// the open form, never serialised (FR-006).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct JiraFilters {
    reporter: Option<JiraUser>,
    label: Option<String>,
    component: Option<String>,
    fix_version: Option<String>,
}

/// Store `value` in `slot` when it is `None` or usable; `false` (slot unchanged) otherwise.
fn set_text(slot: &mut Option<String>, value: Option<String>) -> bool {
    match value {
        Some(value) if !is_usable(&value) => false,
        value => {
            *slot = value;
            true
        }
    }
}

impl JiraFilters {
    /// Choose or clear the reporter; a user whose display name is unusable is refused (`false`).
    pub fn set_reporter(&mut self, user: Option<JiraUser>) -> bool {
        match user {
            Some(user) if !is_usable(&user.name) => false,
            user => {
                self.reporter = user;
                true
            }
        }
    }

    /// Choose or clear the label; an empty value or one with a control character is refused.
    pub fn set_label(&mut self, label: Option<String>) -> bool {
        set_text(&mut self.label, label)
    }

    /// Choose or clear the component; an empty value or one with a control character is refused.
    pub fn set_component(&mut self, name: Option<String>) -> bool {
        set_text(&mut self.component, name)
    }

    /// Choose or clear the fix version; an empty value or one with a control character is
    /// refused.
    pub fn set_fix_version(&mut self, name: Option<String>) -> bool {
        set_text(&mut self.fix_version, name)
    }

    /// Clear `kind`'s value.
    pub fn clear(&mut self, kind: FilterKind) {
        match kind {
            FilterKind::Reporter => self.reporter = None,
            FilterKind::Label => self.label = None,
            FilterKind::Component => self.component = None,
            FilterKind::FixVersion => self.fix_version = None,
        }
    }

    /// Clear every value.
    pub fn clear_all(&mut self) {
        *self = Self::default();
    }

    /// No filter is chosen.
    pub fn is_empty(&self) -> bool {
        self.active().is_empty()
    }

    /// The chosen reporter.
    pub fn reporter(&self) -> Option<&JiraUser> {
        self.reporter.as_ref()
    }

    /// The chosen label.
    pub fn label(&self) -> Option<&str> {
        self.label.as_deref()
    }

    /// The chosen component.
    pub fn component(&self) -> Option<&str> {
        self.component.as_deref()
    }

    /// The chosen fix version.
    pub fn fix_version(&self) -> Option<&str> {
        self.fix_version.as_deref()
    }

    /// The chosen value of `kind`, as shown (a reporter by [`JiraUser::label`]).
    pub fn shown(&self, kind: FilterKind) -> Option<String> {
        match kind {
            FilterKind::Reporter => self.reporter.as_ref().map(JiraUser::label),
            FilterKind::Label => self.label.clone(),
            FilterKind::Component => self.component.clone(),
            FilterKind::FixVersion => self.fix_version.clone(),
        }
    }

    /// The chosen filters with their shown values, in the fixed order: what the active line and
    /// the statement name.
    pub fn active(&self) -> Vec<(FilterKind, String)> {
        FilterKind::ALL
            .into_iter()
            .filter_map(|kind| self.shown(kind).map(|value| (kind, value)))
            .collect()
    }

    /// One JQL clause per chosen filter, in the fixed order (contracts/jira-filter-source.md §1).
    pub fn clauses(&self) -> Vec<String> {
        let reporter = self
            .reporter
            .as_ref()
            .map(|user| format!("reporter = {}", jql_string(user.account.as_str())));
        let label = self
            .label
            .as_deref()
            .map(|label| format!("labels = {}", jql_string(label)));
        let component = self
            .component
            .as_deref()
            .map(|name| format!("component = {}", jql_string(name)));
        let fix_version = self
            .fix_version
            .as_deref()
            .map(|name| format!("fixVersion = {}", jql_string(name)));
        [reporter, label, component, fix_version]
            .into_iter()
            .flatten()
            .collect()
    }
}

/// `value` as one JQL string: quoted, with `\` and then `"` escaped; never trimmed, cut or
/// case-changed (FR-010, research R2).
pub fn jql_string(value: &str) -> String {
    format!("\"{}\"", value.replace('\\', "\\\\").replace('"', "\\\""))
}

/// `jql` with ` AND <clause>` for each chosen filter, inserted before its last ` ORDER BY ` (or
/// appended when there is none); `jql` itself when no filter is chosen.
pub fn narrowed(jql: &str, filters: &JiraFilters) -> String {
    let clauses: String = filters
        .clauses()
        .iter()
        .map(|clause| format!(" AND {clause}"))
        .collect();
    match jql.rfind(" ORDER BY ") {
        Some(at) => format!("{}{clauses}{}", &jql[..at], &jql[at..]),
        None => format!("{jql}{clauses}"),
    }
}

/// The names of `GET /rest/api/3/project/{key}/components`, in Jira's order: empty or
/// control-character names skipped, duplicates dropped (first kept). A body of another shape is
/// [`IssueLoadError::NotJiraCloud`] (contracts/jira-filter-source.md §4).
pub fn parse_components(body: &[u8]) -> Result<Vec<String>, IssueLoadError> {
    let json: serde_json::Value =
        serde_json::from_slice(body).map_err(|_| IssueLoadError::NotJiraCloud)?;
    let entries = json.as_array().ok_or(IssueLoadError::NotJiraCloud)?;
    let mut names: Vec<String> = Vec::new();
    for name in entries.iter().filter_map(|entry| entry["name"].as_str()) {
        if is_usable(name) && !names.iter().any(|held| held == name) {
            names.push(name.to_owned());
        }
    }
    Ok(names)
}
