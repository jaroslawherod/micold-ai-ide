//! The Jira connection's effects (feature 533, contracts/jira-connection.md §3 and §4): the
//! API token's owner-only file, and the connection change a save sends.
//!
//! The token lives only in `<data_dir>/jira/<project id>.token`, written with
//! `owner_only::write` and read nowhere but here (FR-008). It never enters `Settings`, a wire
//! frame or a log line; an error text built here names the operation, never the token.
//!
//! Depends on `micold_core` and `std` alone, so the shell test includes this file by path.

use std::io;
use std::path::Path;

use micold_core::jira::{ApiToken, ConnectionAction};
use micold_core::settings::{JiraChange, JiraConnectionChange};
use micold_core::store::{jira_token_file_name, jira_token_path};

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
    match std::fs::remove_file(jira_token_path(dir, project)) {
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
            }
            Ok(Some(JiraConnectionChange {
                project: micold_core::jira::connection_key(project),
                change: JiraChange::Remove,
            }))
        }
    }
}
