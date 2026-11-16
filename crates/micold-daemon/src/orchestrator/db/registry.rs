//! `RegistryStore` over the SQL database (#1948): `load` returns every tenant's records ordered by
//! `(tenant_id, id)`, `save` replaces the whole set in one transaction, `upsert` and `remove` are
//! one statement each so two writers never lose each other's record.

use std::collections::{BTreeSet, HashSet};
use std::sync::Arc;

use micold_core::daemon_config::Kind;
use micold_core::orchestrator::{
    DaemonRecord, Health, OwnerId, RegistryStore, StoreError, TenantId, UserId, UNOWNED,
};
use micold_core::protocol::routing::WireDaemonId;
use sqlx::any::AnyRow;
use sqlx::{AnyConnection, Row as _};

use super::{to_i64, StoreHandle};

const UPSERT: &str = "INSERT INTO daemons \
    (tenant_id, id, kind, name, protocol_version, capabilities, health, last_seen, owner_user_id) \
    VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9) \
    ON CONFLICT (tenant_id, id) DO UPDATE SET kind = excluded.kind, name = excluded.name, \
    protocol_version = excluded.protocol_version, capabilities = excluded.capabilities, \
    health = excluded.health, last_seen = excluded.last_seen";

/// The tables whose rows carry an owner; `reassign_unowned` moves them together.
const OWNED_TABLES: [&str; 3] = ["daemons", "projects", "sessions"];

/// The registry in the `daemons` table.
pub struct SqlRegistryStore {
    db: Arc<StoreHandle>,
}

impl SqlRegistryStore {
    /// A registry store over `db`; one value serves every tenant.
    pub fn new(db: Arc<StoreHandle>) -> Self {
        Self { db }
    }

    fn run<T>(
        &self,
        work: impl std::future::Future<Output = Result<T, sqlx::Error>>,
    ) -> Result<T, StoreError> {
        self.db.run(work)
    }
}

/// The text a column holds for a unit enum: its serde name.
fn name_of<T: serde::Serialize>(value: &T) -> String {
    match serde_json::to_value(value) {
        Ok(serde_json::Value::String(name)) => name,
        _ => String::new(),
    }
}

fn from_name<T: serde::de::DeserializeOwned>(name: &str) -> Result<T, String> {
    serde_json::from_value(serde_json::Value::String(name.to_owned()))
        .map_err(|e| format!("{name:?}: {e}"))
}

pub(super) async fn upsert_row(
    conn: &mut AnyConnection,
    record: &DaemonRecord,
) -> Result<(), sqlx::Error> {
    // A set of strings always serialises.
    let capabilities = serde_json::to_string(&record.capabilities).unwrap_or_else(|_| "[]".into());
    sqlx::query(UPSERT)
        .bind(record.tenant.as_str().to_owned())
        .bind(record.id.as_str().to_owned())
        .bind(name_of(&record.kind))
        .bind(record.name.clone())
        .bind(i64::from(record.protocol_version))
        .bind(capabilities)
        .bind(name_of(&record.health))
        .bind(to_i64(record.last_seen))
        // Written on insert only: the conflict branch leaves a stored owner as it is.
        .bind(record.owner.as_str().to_owned())
        .execute(conn)
        .await
        .map(drop)
}

fn record_of(row: &AnyRow) -> Result<DaemonRecord, String> {
    let text = |column: &str| row.try_get::<String, _>(column).map_err(|e| e.to_string());
    let number = |column: &str| row.try_get::<i64, _>(column).map_err(|e| e.to_string());
    let id = text("id")?;
    let capabilities: BTreeSet<String> =
        serde_json::from_str(&text("capabilities")?).map_err(|e| format!("capabilities: {e}"))?;
    let kind: Kind = from_name(&text("kind")?)?;
    let mut record = DaemonRecord::new(
        WireDaemonId::new(&id).map_err(|e| format!("id {id:?}: {e}"))?,
        kind,
        &text("name")?,
        u32::try_from(number("protocol_version")?).map_err(|e| e.to_string())?,
        capabilities,
        Some(&text("tenant_id")?),
    );
    record.health = from_name::<Health>(&text("health")?)?;
    record.last_seen = u64::try_from(number("last_seen")?).unwrap_or(0);
    record.owner = OwnerId::parse(&text("owner_user_id")?);
    Ok(record)
}

impl RegistryStore for SqlRegistryStore {
    fn load(&self) -> Result<Vec<DaemonRecord>, StoreError> {
        let rows = self.run(async {
            sqlx::query(
                "SELECT tenant_id, id, kind, name, protocol_version, capabilities, health, \
                 last_seen, owner_user_id FROM daemons ORDER BY tenant_id, id",
            )
            .fetch_all(self.db.pool())
            .await
        })?;
        rows.iter()
            .map(|row| {
                record_of(row).map_err(|reason| StoreError::Corrupt {
                    path: self.db.path(),
                    reason,
                })
            })
            .collect()
    }

    fn save(&self, records: &[DaemonRecord]) -> Result<(), StoreError> {
        self.run(async {
            let mut tx = self.db.begin_write().await?;
            for record in records {
                upsert_row(&mut tx, record).await?;
            }
            let keep: HashSet<(&str, &str)> = records
                .iter()
                .map(|r| (r.tenant.as_str(), r.id.as_str()))
                .collect();
            let existing = sqlx::query("SELECT tenant_id, id FROM daemons")
                .fetch_all(&mut *tx)
                .await?;
            for row in existing {
                let tenant: String = row.try_get("tenant_id")?;
                let id: String = row.try_get("id")?;
                if !keep.contains(&(tenant.as_str(), id.as_str())) {
                    sqlx::query("DELETE FROM daemons WHERE tenant_id = $1 AND id = $2")
                        .bind(tenant)
                        .bind(id)
                        .execute(&mut *tx)
                        .await?;
                }
            }
            tx.commit().await
        })
    }

    fn upsert(&self, record: &DaemonRecord) -> Result<(), StoreError> {
        self.run(async {
            let mut conn = self.db.pool().acquire().await?;
            upsert_row(&mut conn, record).await
        })
    }

    fn remove(&self, tenant: &TenantId, id: &WireDaemonId) -> Result<bool, StoreError> {
        self.run(async {
            sqlx::query("DELETE FROM daemons WHERE tenant_id = $1 AND id = $2")
                .bind(tenant.as_str().to_owned())
                .bind(id.as_str().to_owned())
                .execute(self.db.pool())
                .await
                .map(|done| done.rows_affected() > 0)
        })
    }

    fn reassign_unowned(&self, tenant: &TenantId, owner: &UserId) -> Result<u64, StoreError> {
        self.run(async {
            let mut tx = self.db.begin_write().await?;
            let mut daemons = 0;
            for table in OWNED_TABLES {
                let sql = format!(
                    "UPDATE {table} SET owner_user_id = $1 \
                     WHERE tenant_id = $2 AND owner_user_id = $3"
                );
                let done = sqlx::query(&sql)
                    .bind(owner.as_str().to_owned())
                    .bind(tenant.as_str().to_owned())
                    .bind(UNOWNED.to_owned())
                    .execute(&mut *tx)
                    .await?;
                if table == "daemons" {
                    daemons = done.rows_affected();
                }
            }
            tx.commit().await?;
            Ok(daemons)
        })
    }
}
