//! The schema's forward migrations (FR-003): the files under `migrations/sqlite/` and
//! `migrations/postgres/` (the same versions and descriptions), embedded at build time, applied
//! by sqlx's migrator (one transaction per migration, a checksum check).

use micold_core::orchestrator::{SchemaError, StoreError};
use sqlx::migrate::{MigrateError, Migrator};
use sqlx::AnyPool;

/// The SQLite dialect's migrations.
pub static MIGRATOR: Migrator = sqlx::migrate!("./migrations/sqlite");

/// The PostgreSQL dialect's migrations.
pub static POSTGRES_MIGRATOR: Migrator = sqlx::migrate!("./migrations/postgres");

/// Apply every migration of `migrator` the database lacks, forward only. PostgreSQL serialises
/// starters with sqlx's advisory lock. SQLite has none in sqlx's migrator, so a second starter
/// can lose the race with `table … already exists`: it retries once,
/// after which the first starter's work is recorded and the run finds nothing to do.
pub async fn run_forward(migrator: &'static Migrator, pool: &AnyPool) -> Result<(), StoreError> {
    let schema_error = |error| schema_error(migrator, error);
    match migrator.run(pool).await {
        Ok(()) => Ok(()),
        Err(MigrateError::Execute(error)) if lost_the_race(&error) => {
            migrator.run(pool).await.map_err(schema_error)
        }
        Err(error) => Err(schema_error(error)),
    }
}

/// Whether another starter created what this one tried to create, or holds the write lock.
fn lost_the_race(error: &sqlx::Error) -> bool {
    let text = error.to_string();
    text.contains("already exists") || text.contains("locked") || text.contains("busy")
}

fn schema_error(migrator: &Migrator, error: MigrateError) -> StoreError {
    let version_of = |version: i64| {
        migrator
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
