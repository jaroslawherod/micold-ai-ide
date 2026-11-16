//! The one-time import of an earlier version's `<data_dir>/orchestrator/registry.json` into the
//! database (#1948, FR-010).
//!
//! One transaction inserts every record and an `imports` marker holding the file's SHA-1; the file
//! is renamed `registry.json.imported` only after it commits. A crash between the two leaves rows
//! plus a marker whose digest matches the file, which the next start recognises and only renames.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use micold_core::orchestrator::store::{JsonRegistryStore, REGISTRY_FILE};
use micold_core::orchestrator::{RegistryStore as _, StoreError, DEFAULT_TENANT};
use sha1::{Digest as _, Sha1};
use sqlx::Row as _;

use super::registry::upsert_row;
use super::{to_i64, StoreHandle};

/// The suffix a renamed, imported file carries.
pub const IMPORTED_SUFFIX: &str = ".imported";

/// What [`import_registry_json`] did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Imported {
    /// No `registry.json` was there.
    Nothing,
    /// This many records were imported and the file renamed.
    Records(usize),
    /// An earlier start imported the file but stopped before the rename; only the rename ran.
    RenamedOnly,
}

/// Import `<data_dir>/orchestrator/registry.json` into `db` when it is there, stamping the marker
/// with `now` (Unix seconds), then rename it `registry.json.imported` in place. Blocks: call it
/// from `spawn_blocking` in async code.
///
/// A file that does not parse fails as [`StoreError::Corrupt`] naming it; a registry that already
/// has rows, with no marker for this file, fails as [`StoreError::Refused`] naming the file and the
/// row count. Neither imports anything.
pub fn import_registry_json(
    db: &StoreHandle,
    data_dir: &Path,
    now: u64,
) -> Result<Imported, StoreError> {
    let json = JsonRegistryStore::in_data_dir(data_dir);
    let path = json.path();
    let bytes = match std::fs::read(&path) {
        Ok(bytes) => bytes,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Imported::Nothing),
        Err(e) => {
            return Err(StoreError::Corrupt {
                path,
                reason: e.to_string(),
            })
        }
    };
    let records = json.load()?;
    let digest = format!("sha1:{:x}", Sha1::digest(&bytes));
    let mut tenants: BTreeSet<String> = records
        .iter()
        .map(|r| r.tenant.as_str().to_owned())
        .collect();
    if tenants.is_empty() {
        tenants.insert(DEFAULT_TENANT.to_owned());
    }
    let outcome = db.run(async {
        let mut tx = db.begin_write().await?;
        let marked: i64 =
            sqlx::query("SELECT COUNT(*) FROM imports WHERE source = $1 AND digest = $2")
                .bind(REGISTRY_FILE.to_owned())
                .bind(digest.clone())
                .fetch_one(&mut *tx)
                .await?
                .try_get(0)?;
        if marked > 0 {
            return Ok(Step::Rename(Imported::RenamedOnly));
        }
        let rows: i64 = sqlx::query("SELECT COUNT(*) FROM daemons")
            .fetch_one(&mut *tx)
            .await?
            .try_get(0)?;
        if rows > 0 {
            return Ok(Step::Refuse(rows));
        }
        for record in &records {
            upsert_row(&mut tx, record).await?;
        }
        for tenant in &tenants {
            sqlx::query(
                "INSERT INTO imports (tenant_id, source, digest, imported_at) \
                 VALUES ($1, $2, $3, $4)",
            )
            .bind(tenant.clone())
            .bind(REGISTRY_FILE.to_owned())
            .bind(digest.clone())
            .bind(to_i64(now))
            .execute(&mut *tx)
            .await?;
        }
        tx.commit().await?;
        Ok(Step::Rename(Imported::Records(records.len())))
    })?;
    match outcome {
        Step::Refuse(rows) => Err(StoreError::Refused {
            reason: format!(
                "cannot import {}: the database registry already has {rows} row(s)",
                path.display()
            ),
        }),
        Step::Rename(done) => {
            std::fs::rename(&path, imported_path(&path)).map_err(|e| StoreError::Save {
                path: path.clone(),
                reason: e.to_string(),
            })?;
            Ok(done)
        }
    }
}

/// `registry.json.imported` beside `path`.
pub fn imported_path(path: &Path) -> PathBuf {
    let mut name = path.file_name().unwrap_or_default().to_os_string();
    name.push(IMPORTED_SUFFIX);
    path.with_file_name(name)
}

/// What the transaction decided.
enum Step {
    /// Rows are there with no marker for this file: refuse, naming the count.
    Refuse(i64),
    /// Rename the file, reporting this.
    Rename(Imported),
}
