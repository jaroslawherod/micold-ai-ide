//! The schema's forward migrations (FR-003): the files under `migrations/sqlite/`, embedded at
//! build time, applied by sqlx's migrator (one transaction per migration, a checksum check).

use micold_core::orchestrator::{SchemaError, StoreError};
use sqlx::migrate::{MigrateError, Migrator};
use sqlx::AnyPool;

/// The SQLite dialect's migrations.
pub static MIGRATOR: Migrator = sqlx::migrate!("./migrations/sqlite");

/// Apply every migration the database lacks, forward only. SQLite has no advisory lock in sqlx's
/// migrator, so a second starter can lose the race with `table … already exists`: it retries once,
/// after which the first starter's work is recorded and the run finds nothing to do.
pub async fn run_forward(pool: &AnyPool) -> Result<(), StoreError> {
    match MIGRATOR.run(pool).await {
        Ok(()) => Ok(()),
        Err(MigrateError::Execute(_)) => MIGRATOR.run(pool).await.map_err(schema_error),
        Err(error) => Err(schema_error(error)),
    }
}

fn schema_error(error: MigrateError) -> StoreError {
    let version_of = |version: i64| {
        MIGRATOR
            .migrations
            .iter()
            .find(|m| m.version == version)
            .map_or_else(String::new, |m| m.description.to_string())
    };
    StoreError::Schema(match error {
        MigrateError::VersionMismatch(version) => SchemaError::ChecksumChanged {
            version,
            description: version_of(version),
        },
        other => SchemaError::Failed {
            version: 0,
            reason: other.to_string(),
        },
    })
}
