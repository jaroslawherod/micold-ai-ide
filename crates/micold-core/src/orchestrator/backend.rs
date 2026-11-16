//! Which store the orchestrator runs on (FR-009): the rule of `contracts/config.md`.
//!
//! No `[database]` table and a loopback-only bind: the SQLite file in the data directory. A
//! `[database]` table: PostgreSQL, on any bind. No table and a bind beyond loopback: no store, and
//! the error names the missing key. The bind key itself lands with #1732; until then the daemon
//! passes [`BindScope::LoopbackOnly`].

use std::fmt;
use std::path::{Path, PathBuf};

use super::store::STORE_DIR;
use crate::daemon_config::DatabaseConfig;

/// The SQLite file's name under `<data_dir>/orchestrator/`.
pub const SQLITE_FILE: &str = "orchestrator.db";

/// How far the orchestrator's listener reaches.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BindScope {
    /// Loopback addresses only.
    LoopbackOnly,
    /// Any non-loopback address.
    Network,
}

/// The store [`choose_store`] picked.
#[derive(Debug, Clone, PartialEq)]
pub enum StoreChoice {
    /// PostgreSQL, from the `[database]` table.
    Postgres(DatabaseConfig),
    /// The SQLite file at this path.
    Sqlite(PathBuf),
}

/// A bind beyond loopback with no `[database]` table.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MissingDatabase;

impl fmt::Display for MissingDatabase {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(
            "`database.url_file` is required when the orchestrator listens on a network address",
        )
    }
}

impl std::error::Error for MissingDatabase {}

/// The store for this configuration and bind (FR-009).
pub fn choose_store(
    database: Option<&DatabaseConfig>,
    bind: BindScope,
    data_dir: &Path,
) -> Result<StoreChoice, MissingDatabase> {
    match (database, bind) {
        (Some(database), _) => Ok(StoreChoice::Postgres(database.clone())),
        (None, BindScope::LoopbackOnly) => Ok(StoreChoice::Sqlite(
            data_dir.join(STORE_DIR).join(SQLITE_FILE),
        )),
        (None, BindScope::Network) => Err(MissingDatabase),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::daemon_config::SecretPath;

    fn database() -> DatabaseConfig {
        DatabaseConfig {
            url_file: Some(SecretPath::new("/s/url")),
            ca_file: None,
            pool_size: 5,
        }
    }

    #[test]
    fn a_database_table_picks_postgres_on_any_bind() {
        for bind in [BindScope::LoopbackOnly, BindScope::Network] {
            let choice = choose_store(Some(&database()), bind, Path::new("/d")).unwrap();
            assert_eq!(choice, StoreChoice::Postgres(database()));
        }
    }

    #[test]
    fn no_table_and_loopback_picks_sqlite_in_the_data_directory() {
        let choice = choose_store(None, BindScope::LoopbackOnly, Path::new("/d")).unwrap();
        assert_eq!(
            choice,
            StoreChoice::Sqlite(PathBuf::from("/d/orchestrator/orchestrator.db"))
        );
    }

    #[test]
    fn no_table_and_a_network_bind_fails_naming_the_key() {
        let error = choose_store(None, BindScope::Network, Path::new("/d")).unwrap_err();
        assert!(error.to_string().contains("database.url_file"), "{error}");
    }
}
