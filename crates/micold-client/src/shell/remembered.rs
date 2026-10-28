//! A project's remembered file in the user's local data (features 1186 and 1400): read leniently,
//! written one change at a time, removed with what it belongs to.
//!
//! Each file holds one type with a lenient `from_json` (a missing or unreadable file is that type's
//! default) and a `to_json`: the Jira selector's mode and last run query (`<data_dir>/jira/<project
//! id>.query.json`), and the GitHub selector (`<data_dir>/github/<project id>.selector.json`).
//! A write reads the file, applies one change and writes it back owner-only through a temporary
//! file and a rename, so a reader never sees half a file and another window's write to another
//! field stands; with two windows the latest write wins (1400 FR-021, research R11).
//!
//! Depends on `micold_core` and `std` alone, so shell tests include this file by path. `shell::jira`
//! declares it, so `tests/jira_connection_shell.rs`, which includes `jira.rs` by path, needs no
//! change; `shell` re-exports it.

use std::io;
use std::path::Path;

use micold_core::github_filter::RememberedSelector;
use micold_core::jira_query::RememberedQuery;

/// What a remembered file holds: read leniently, written whole.
pub trait Remembered: Default {
    /// The file's bytes read leniently: what cannot be read is dropped, never an error.
    fn from_json(bytes: &[u8]) -> Self;
    /// The file's bytes.
    fn to_json(&self) -> Vec<u8>;
}

impl Remembered for RememberedQuery {
    fn from_json(bytes: &[u8]) -> Self {
        RememberedQuery::from_json(bytes)
    }
    fn to_json(&self) -> Vec<u8> {
        RememberedQuery::to_json(self)
    }
}

impl Remembered for RememberedSelector {
    fn from_json(bytes: &[u8]) -> Self {
        RememberedSelector::from_json(bytes)
    }
    fn to_json(&self) -> Vec<u8> {
        RememberedSelector::to_json(self)
    }
}

/// What `path` remembers: a missing or unreadable file is the default.
pub fn read<T: Remembered>(path: &Path) -> T {
    std::fs::read(path)
        .map(|bytes| T::from_json(&bytes))
        .unwrap_or_default()
}

/// Remember one change: read `dir/file_name`, `apply` the change, write the file back owner-only
/// (directory 0700, file 0600 on Unix). The error is the OS error alone, never the content.
pub fn remember<T: Remembered>(
    dir: &Path,
    file_name: &str,
    apply: impl FnOnce(T) -> T,
) -> io::Result<()> {
    let next = apply(read(&dir.join(file_name)));
    micold_core::owner_only::write(dir, file_name, &next.to_json()).map(drop)
}

/// Remove `path`; a missing file is already what was asked for.
pub fn remove(path: &Path) -> io::Result<()> {
    match std::fs::remove_file(path) {
        Err(error) if error.kind() != io::ErrorKind::NotFound => Err(error),
        _ => Ok(()),
    }
}
