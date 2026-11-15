//! `AuditStore` over the SQL database (#1948): `append` inserts one row and returns it with its
//! `seq`; reads are keyset by `seq`. No statement here updates or deletes an audit row, and the
//! schema's triggers refuse one from anywhere else (FR-011).

use std::sync::Arc;

use micold_core::orchestrator::{AuditEntry, AuditStore, NewAuditEntry, StoreError, TenantId};
use sqlx::any::AnyRow;
use sqlx::Row as _;

use super::{to_i64, to_u64, StoreHandle};

const COLUMNS: &str = "seq, at, actor, action, target, detail";

/// The audit trail of one tenant, in the `audit_log` table.
pub struct SqlAuditStore {
    db: Arc<StoreHandle>,
    tenant: TenantId,
}

impl SqlAuditStore {
    /// An audit store over `db`, acting within `tenant`.
    pub fn new(db: Arc<StoreHandle>, tenant: TenantId) -> Self {
        Self { db, tenant }
    }

    fn entry_of(row: &AnyRow) -> Result<AuditEntry, sqlx::Error> {
        Ok(AuditEntry {
            seq: row.try_get("seq")?,
            at: to_u64(row.try_get("at")?),
            actor: row.try_get("actor")?,
            action: row.try_get("action")?,
            target: row.try_get("target")?,
            detail: row.try_get("detail")?,
        })
    }

    fn entries<'q>(
        &self,
        query: sqlx::query::Query<'q, sqlx::Any, sqlx::any::AnyArguments<'q>>,
    ) -> Result<Vec<AuditEntry>, StoreError> {
        self.db.run(async {
            let rows = query.fetch_all(self.db.pool()).await?;
            rows.iter().map(Self::entry_of).collect()
        })
    }
}

impl AuditStore for SqlAuditStore {
    fn append(&self, entry: &NewAuditEntry) -> Result<AuditEntry, StoreError> {
        self.db.run(async {
            let row = sqlx::query(&format!(
                "INSERT INTO audit_log (tenant_id, at, actor, action, target, detail) \
                 VALUES ($1, $2, $3, $4, $5, $6) RETURNING {COLUMNS}"
            ))
            .bind(self.tenant.as_str().to_owned())
            .bind(to_i64(entry.at()))
            .bind(entry.actor().to_owned())
            .bind(entry.action().to_owned())
            .bind(entry.target().to_owned())
            .bind(entry.detail().to_owned())
            .fetch_one(self.db.pool())
            .await?;
            Self::entry_of(&row)
        })
    }

    fn recent(&self, limit: u32, before: Option<i64>) -> Result<Vec<AuditEntry>, StoreError> {
        let sql = format!(
            "SELECT {COLUMNS} FROM audit_log WHERE tenant_id = $1 AND seq < $2 \
             ORDER BY seq DESC LIMIT $3"
        );
        self.entries(
            sqlx::query(&sql)
                .bind(self.tenant.as_str().to_owned())
                .bind(before.unwrap_or(i64::MAX))
                .bind(i64::from(limit)),
        )
    }

    fn by_actor(&self, actor: &str, limit: u32) -> Result<Vec<AuditEntry>, StoreError> {
        let sql = format!(
            "SELECT {COLUMNS} FROM audit_log WHERE tenant_id = $1 AND actor = $2 \
             ORDER BY seq DESC LIMIT $3"
        );
        self.entries(
            sqlx::query(&sql)
                .bind(self.tenant.as_str().to_owned())
                .bind(actor.to_owned())
                .bind(i64::from(limit)),
        )
    }
}
