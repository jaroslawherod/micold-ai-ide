//! The Jira connection's effects (feature 533, contracts/jira-connection.md §3 and §4): the
//! API token's owner-only file, and the connection change a save sends.
//!
//! The token lives only in `<data_dir>/jira/<project id>.token`, written with
//! `owner_only::write` and read nowhere but here (FR-008). It never enters `Settings`, a wire
//! frame or a log line; an error text built here names the operation, never the token.
//!
//! Feature 1186 keeps the selector's remembered mode and last run query beside it, in
//! `<project id>.query.json` (contracts/selector-source-and-storage.md §4): written owner-only at
//! each switch and run, read as the form opens, deleted with the token when the connection is
//! removed (FR-013, FR-014). An error text built for it names the OS error, never the query
//! (FR-016).
//!
//! Depends on `micold_core` and `std` alone, so the shell test includes this file by path.

use std::io;
use std::path::Path;

use micold_core::jira::{ApiToken, ConnectionAction};
use micold_core::jira_query::{RememberChange, RememberedQuery};
use micold_core::settings::{JiraChange, JiraConnectionChange};
use micold_core::store::{
    jira_query_file_name, jira_query_path, jira_token_file_name, jira_token_path,
};

/// Whether `project` has a token file in `dir`: the probe Settings runs on open and on
/// `SettingsChanged` (contracts/jira-connection.md §4). The token is never read for it.
pub fn token_stored(dir: &Path, project: &Path) -> bool {
    jira_token_path(dir, project).exists()
}

/// Write the token file, owner-only (directory 0700, file 0600 on Unix).
pub fn write_token(dir: &Path, project: &Path, token: &ApiToken) -> io::Result<()> {
    micold_core::owner_only::write(
        dir,
        &jira_token_file_name(project),
        token.expose().as_bytes(),
    )
    .map(drop)
}

/// Delete the token file; a missing file is already what was asked for.
pub fn remove_token(dir: &Path, project: &Path) -> io::Result<()> {
    remove_if_present(&jira_token_path(dir, project))
}

/// The mode and last run query `project` remembers (feature 1186, FR-013): a missing or
/// unreadable file is the default, basic and empty (Edge "Remembered data unreadable").
pub fn read_query(dir: &Path, project: &Path) -> RememberedQuery {
    std::fs::read(jira_query_path(dir, project))
        .map(|bytes| RememberedQuery::from_json(&bytes))
        .unwrap_or_default()
}

/// Remember one field: read the file, apply `change`, write it back owner-only through a temporary
/// file and a rename, so a reader never sees half a file and another window's write to the other
/// field stands (contracts/selector-source-and-storage.md §4).
pub fn remember_query(dir: &Path, project: &Path, change: &RememberChange) -> io::Result<()> {
    let next = read_query(dir, project).with_change(change);
    micold_core::owner_only::write(dir, &jira_query_file_name(project), &next.to_json()).map(drop)
}

/// The notification a failed [`remember_query`] raises: the OS error, never the query (FR-016).
pub fn remember_failed(error: &io::Error) -> String {
    format!("Couldn't remember the Jira query: {error}")
}

/// Delete the remembered file; a missing file is already what was asked for.
pub fn remove_query(dir: &Path, project: &Path) -> io::Result<()> {
    remove_if_present(&jira_query_path(dir, project))
}

/// Remove `path`; a missing file is success.
fn remove_if_present(path: &Path) -> io::Result<()> {
    match std::fs::remove_file(path) {
        Err(error) if error.kind() != io::ErrorKind::NotFound => Err(error),
        _ => Ok(()),
    }
}

/// Perform a save's connection action in the order the contract fixes: the token file first,
/// then the change the caller sends. `Ok(None)` when the action leaves the connection alone.
///
/// The error text names what failed and the OS error, never the token.
pub fn apply(
    dir: Option<&Path>,
    action: &ConnectionAction,
) -> Result<Option<JiraConnectionChange>, String> {
    match action {
        ConnectionAction::Keep => Ok(None),
        ConnectionAction::Save {
            project,
            connection,
            token,
        } => {
            if let Some(token) = token {
                let dir = dir.ok_or("no data directory to keep the API token in")?;
                write_token(dir, project, token)
                    .map_err(|error| format!("couldn't store the API token: {error}"))?;
            }
            Ok(Some(JiraConnectionChange {
                project: micold_core::jira::connection_key(project),
                change: JiraChange::Upsert(connection.clone()),
            }))
        }
        ConnectionAction::Remove { project } => {
            if let Some(dir) = dir {
                remove_token(dir, project)
                    .map_err(|error| format!("couldn't delete the API token: {error}"))?;
                remove_query(dir, project).map_err(|error| {
                    format!("couldn't delete the remembered Jira query: {error}")
                })?;
            }
            Ok(Some(JiraConnectionChange {
                project: micold_core::jira::connection_key(project),
                change: JiraChange::Remove,
            }))
        }
    }
}
