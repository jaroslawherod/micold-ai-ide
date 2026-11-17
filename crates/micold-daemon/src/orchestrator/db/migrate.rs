//! The schema's forward migrations (FR-003): the files under `migrations/sqlite/` and
//! `migrations/postgres/` (the same versions and descriptions), embedded at build time, applied
//! by sqlx's migrator (one transaction per migration, a checksum check).

use micold_core::orchestrator::{SchemaError, StoreError};
use sqlx::migrate::{MigrateError, Migrator};
use sqlx::{AnyConnection, AnyPool, Connection as _, Executor as _};

/// The SQLite dialect's migrations.
pub static MIGRATOR: Migrator = sqlx::migrate!("./migrations/sqlite");

/// The PostgreSQL dialect's migrations.
pub static POSTGRES_MIGRATOR: Migrator = sqlx::migrate!("./migrations/postgres");

/// Refuse a database a newer build migrated (FR-004), reading only: the caller passes a read-only
/// SQLite connection (no WAL pragma), and on PostgreSQL the read runs in a `READ ONLY`
/// transaction. A database without `_sqlx_migrations` is new, not newer. A driver error is
/// `Unavailable` at `target`.
pub async fn check_not_newer(
    migrator: &Migrator,
    conn: &mut AnyConnection,
    target: &str,
) -> Result<(), StoreError> {
    let unavailable = |error: sqlx::Error| StoreError::Unavailable {
        target: target.to_owned(),
        reason: error.to_string(),
    };
    let postgres = conn.backend_name().eq_ignore_ascii_case("postgresql");
    let mut tx = conn.begin().await.map_err(unavailable)?;
    if postgres {
        tx.execute("SET TRANSACTION READ ONLY")
            .await
            .map_err(unavailable)?;
    }
    let exists = if postgres {
        "SELECT CAST(count(*) AS BIGINT) FROM information_schema.tables \
         WHERE table_schema = current_schema() AND table_name = '_sqlx_migrations'"
    } else {
        "SELECT count(*) FROM sqlite_master WHERE type = 'table' AND name = '_sqlx_migrations'"
    };
    let exists: i64 = sqlx::query_scalar(exists)
        .fetch_one(&mut *tx)
        .await
        .map_err(unavailable)?;
    let database: Option<i64> = if exists > 0 {
        sqlx::query_scalar("SELECT max(version) FROM _sqlx_migrations")
            .fetch_one(&mut *tx)
            .await
            .map_err(unavailable)?
    } else {
        None
    };
    tx.rollback().await.map_err(unavailable)?;
    let build = migrator.migrations.last().map_or(0, |m| m.version);
    match database {
        Some(database) if database > build => Err(StoreError::Schema(SchemaError::DatabaseNewer {
            database,
            build,
        })),
        _ => Ok(()),
    }
}

/// Apply every migration of `migrator` the database lacks, forward only. PostgreSQL serialises
/// starters with sqlx's advisory lock. SQLite has none in sqlx's migrator, so a second starter
/// can lose the race with `table … already exists` or a busy database: it retries, re-reading
/// `_sqlx_migrations`, until the other starter's work is recorded and the run finds nothing to do.
pub async fn run_forward(migrator: &'static Migrator, pool: &AnyPool) -> Result<(), StoreError> {
    run_forward_with(migrator, || migrator.run(pool)).await
}

/// [`run_forward`] over any `attempt` that runs the migrator once. A test passes an attempt that
/// loses the race on demand, so the retry bound is exercised on every run, not only when two real
/// starters collide.
pub async fn run_forward_with<F, Fut>(migrator: &Migrator, mut attempt: F) -> Result<(), StoreError>
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = Result<(), MigrateError>>,
{
    // Each lost race means another starter applied (or is applying) at least one migration, so
    // one retry per migration, and one more, is enough for every starter to finish.
    let mut retries = migrator.migrations.len() + 1;
    loop {
        match attempt().await {
            Ok(()) => return Ok(()),
            Err(MigrateError::Execute(error) | MigrateError::ExecuteMigration(error, _))
                if retries > 0 && lost_the_race(&error) =>
            {
                retries -= 1;
            }
            Err(error) => return Err(schema_error(migrator, error)),
        }
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
        MigrateError::ExecuteMigration(error, version) => SchemaError::Failed {
            version,
            reason: error.to_string(),
        },
        other => SchemaError::Failed {
            version: 0,
            reason: other.to_string(),
        },
    })
}
