//! `AuthStore` over the SQL database (#1948): users, identities, invites, the bootstrap code,
//! refresh tokens, certificate serials and revocations, enrollments, all of one tenant.
//!
//! Every statement binds `tenant_id`. What must happen once is one compare-and-set `UPDATE`, so
//! exactly one concurrent caller wins; what must happen together (rotate a refresh token, remove a
//! user, redeem an invite) shares one transaction that starts as a write.

use std::future::Future;
use std::sync::Arc;

use micold_core::orchestrator::{
    AuthStore, Bootstrap, CertSerial, Identity, Invite, NewEnrollment, NewInvite, NewUser,
    Provider, RefreshToken, Revocation, Role, StoreError, TenantId, TokenHash, User, UserId,
};
use micold_core::protocol::routing::WireDaemonId;
use sqlx::any::AnyRow;
use sqlx::{AnyConnection, Row as _};

use super::refusal;
use super::{to_i64, to_u64, StoreHandle};

/// Why a body stopped short of a result: a precondition the caller broke, or a row that does not
/// read back. A database failure is a `sqlx::Error` instead.
enum Fail {
    Refused(String),
    Corrupt(String),
}

type Step<T> = Result<Result<T, Fail>, sqlx::Error>;

fn refuse<T>(reason: impl Into<String>) -> Step<T> {
    Ok(Err(Fail::Refused(reason.into())))
}

fn corrupt(error: impl ToString) -> Fail {
    Fail::Corrupt(error.to_string())
}

const USER: &str = "id, display_name, role, created_at, removed_at";
const INVITE: &str = "id, token_hash, role, email, created_by, created_at, expires_at, \
    redeemed_by, redeemed_at";
const REFRESH: &str = "token_hash, user_id, family, issued_at, expires_at, revoked_at";

/// The auth state of one tenant.
pub struct SqlAuthStore {
    db: Arc<StoreHandle>,
    tenant: TenantId,
}

impl SqlAuthStore {
    /// An auth store over `db`, acting within `tenant`.
    pub fn new(db: Arc<StoreHandle>, tenant: TenantId) -> Self {
        Self { db, tenant }
    }

    fn t(&self) -> String {
        self.tenant.as_str().to_owned()
    }

    fn run<T>(&self, work: impl Future<Output = Step<T>>) -> Result<T, StoreError> {
        match self.db.run(work)? {
            Ok(value) => Ok(value),
            Err(Fail::Refused(reason)) => Err(StoreError::Refused { reason }),
            Err(Fail::Corrupt(reason)) => Err(self.db.corrupt(reason)),
        }
    }
}

fn text(row: &AnyRow, column: &str) -> Result<String, Fail> {
    row.try_get::<String, _>(column).map_err(corrupt)
}

fn opt_text(row: &AnyRow, column: &str) -> Result<Option<String>, Fail> {
    row.try_get::<Option<String>, _>(column).map_err(corrupt)
}

fn time(row: &AnyRow, column: &str) -> Result<u64, Fail> {
    row.try_get::<i64, _>(column).map(to_u64).map_err(corrupt)
}

fn opt_time(row: &AnyRow, column: &str) -> Result<Option<u64>, Fail> {
    row.try_get::<Option<i64>, _>(column)
        .map(|value| value.map(to_u64))
        .map_err(corrupt)
}

fn user_id(value: &str) -> Result<UserId, Fail> {
    UserId::new(value).map_err(corrupt)
}

fn hash_of(row: &AnyRow, column: &str) -> Result<TokenHash, Fail> {
    TokenHash::from_hex(&text(row, column)?).map_err(corrupt)
}

fn role_of(row: &AnyRow) -> Result<Role, Fail> {
    let stored = text(row, "role")?;
    Role::parse(&stored).ok_or_else(|| Fail::Corrupt(format!("role {stored:?}")))
}

fn user_of(row: &AnyRow) -> Result<User, Fail> {
    Ok(User {
        id: user_id(&text(row, "id")?)?,
        display_name: text(row, "display_name")?,
        role: role_of(row)?,
        created_at: time(row, "created_at")?,
        removed_at: opt_time(row, "removed_at")?,
    })
}

fn invite_of(row: &AnyRow) -> Result<Invite, Fail> {
    Ok(Invite {
        id: text(row, "id")?,
        token_hash: hash_of(row, "token_hash")?,
        role: role_of(row)?,
        email: opt_text(row, "email")?,
        created_by: user_id(&text(row, "created_by")?)?,
        created_at: time(row, "created_at")?,
        expires_at: time(row, "expires_at")?,
        redeemed_by: opt_text(row, "redeemed_by")?
            .map(|id| user_id(&id))
            .transpose()?,
        redeemed_at: opt_time(row, "redeemed_at")?,
    })
}

fn refresh_of(row: &AnyRow) -> Result<RefreshToken, Fail> {
    Ok(RefreshToken {
        hash: hash_of(row, "token_hash")?,
        user: user_id(&text(row, "user_id")?)?,
        family: text(row, "family")?,
        issued_at: time(row, "issued_at")?,
        expires_at: time(row, "expires_at")?,
        revoked_at: opt_time(row, "revoked_at")?,
    })
}

/// Why a one-shot token (`invites`, `enrollments`) could not be redeemed, from its row.
async fn why_not_redeemable<T>(
    conn: &mut AnyConnection,
    table: &'static str,
    key: &'static str,
    tenant: String,
    hash: &TokenHash,
    what: &str,
) -> Step<T> {
    let row = sqlx::query(&format!(
        "SELECT redeemed_at FROM {table} WHERE tenant_id = $1 AND {key} = $2"
    ))
    .bind(tenant)
    .bind(hash.as_str().to_owned())
    .fetch_optional(conn)
    .await?;
    match row {
        None => refuse(refusal::token_unknown(what)),
        Some(row) => match opt_time(&row, "redeemed_at") {
            Ok(Some(_)) => refuse(refusal::token_redeemed(what)),
            Ok(None) => refuse(refusal::token_expired(what)),
            Err(fail) => Ok(Err(fail)),
        },
    }
}

async fn user_exists(
    conn: &mut AnyConnection,
    tenant: String,
    id: &UserId,
) -> Result<bool, sqlx::Error> {
    Ok(
        sqlx::query("SELECT 1 FROM users WHERE tenant_id = $1 AND id = $2")
            .bind(tenant)
            .bind(id.as_str().to_owned())
            .fetch_optional(conn)
            .await?
            .is_some(),
    )
}

impl AuthStore for SqlAuthStore {
    fn create_user(&self, user: &NewUser) -> Result<User, StoreError> {
        self.run(async {
            sqlx::query(
                "INSERT INTO users (tenant_id, id, display_name, role, created_at) \
                 VALUES ($1, $2, $3, $4, $5)",
            )
            .bind(self.t())
            .bind(user.id.as_str().to_owned())
            .bind(user.display_name.clone())
            .bind(user.role.as_str())
            .bind(to_i64(user.created_at))
            .execute(self.db.pool())
            .await?;
            Ok(Ok(User {
                id: user.id.clone(),
                display_name: user.display_name.clone(),
                role: user.role,
                created_at: user.created_at,
                removed_at: None,
            }))
        })
    }

    fn user(&self, id: &UserId) -> Result<Option<User>, StoreError> {
        self.run(async {
            let row = sqlx::query(&format!(
                "SELECT {USER} FROM users WHERE tenant_id = $1 AND id = $2"
            ))
            .bind(self.t())
            .bind(id.as_str().to_owned())
            .fetch_optional(self.db.pool())
            .await?;
            Ok(row.as_ref().map(user_of).transpose())
        })
    }

    fn users(&self) -> Result<Vec<User>, StoreError> {
        self.run(async {
            let rows = sqlx::query(&format!(
                "SELECT {USER} FROM users WHERE tenant_id = $1 AND removed_at IS NULL ORDER BY id"
            ))
            .bind(self.t())
            .fetch_all(self.db.pool())
            .await?;
            Ok(rows.iter().map(user_of).collect())
        })
    }

    fn set_role(&self, id: &UserId, role: Role) -> Result<(), StoreError> {
        self.run(async {
            let done = sqlx::query("UPDATE users SET role = $1 WHERE tenant_id = $2 AND id = $3")
                .bind(role.as_str())
                .bind(self.t())
                .bind(id.as_str().to_owned())
                .execute(self.db.pool())
                .await?;
            if done.rows_affected() == 0 {
                return refuse(refusal::user_unknown(id));
            }
            Ok(Ok(()))
        })
    }

    fn remove_user(&self, id: &UserId, at: u64) -> Result<(), StoreError> {
        self.run(async {
            let mut tx = self.db.begin_write().await?;
            let done = sqlx::query(
                "UPDATE users SET removed_at = COALESCE(removed_at, $1) \
                 WHERE tenant_id = $2 AND id = $3",
            )
            .bind(to_i64(at))
            .bind(self.t())
            .bind(id.as_str().to_owned())
            .execute(&mut *tx)
            .await?;
            if done.rows_affected() == 0 {
                return refuse(refusal::user_unknown(id));
            }
            sqlx::query(
                "UPDATE refresh_tokens SET revoked_at = $1 \
                 WHERE tenant_id = $2 AND user_id = $3 AND revoked_at IS NULL",
            )
            .bind(to_i64(at))
            .bind(self.t())
            .bind(id.as_str().to_owned())
            .execute(&mut *tx)
            .await?;
            tx.commit().await?;
            Ok(Ok(()))
        })
    }

    fn link_identity(&self, identity: &Identity) -> Result<(), StoreError> {
        self.run(async {
            // The `WHERE` makes a link to another user change nothing; an unknown user is the
            // foreign key's refusal.
            let done = sqlx::query(
                "INSERT INTO identities (tenant_id, provider, subject, user_id, email, email_verified) \
                 VALUES ($1, $2, $3, $4, $5, $6) \
                 ON CONFLICT (tenant_id, provider, subject) DO UPDATE SET \
                 email = excluded.email, email_verified = excluded.email_verified \
                 WHERE identities.user_id = excluded.user_id",
            )
            .bind(self.t())
            .bind(identity.provider.as_str())
            .bind(identity.subject.clone())
            .bind(identity.user.as_str().to_owned())
            .bind(identity.email.clone())
            .bind(i64::from(identity.email_verified))
            .execute(self.db.pool())
            .await?;
            if done.rows_affected() == 0 {
                return refuse("the identity is linked to another user");
            }
            Ok(Ok(()))
        })
    }

    fn user_by_identity(
        &self,
        provider: Provider,
        subject: &str,
    ) -> Result<Option<User>, StoreError> {
        self.run(async {
            let row = sqlx::query(
                "SELECT u.id, u.display_name, u.role, u.created_at, u.removed_at \
                 FROM identities i JOIN users u ON u.tenant_id = i.tenant_id AND u.id = i.user_id \
                 WHERE i.tenant_id = $1 AND i.provider = $2 AND i.subject = $3 \
                 AND u.removed_at IS NULL",
            )
            .bind(self.t())
            .bind(provider.as_str())
            .bind(subject.to_owned())
            .fetch_optional(self.db.pool())
            .await?;
            Ok(row.as_ref().map(user_of).transpose())
        })
    }

    fn create_invite(&self, invite: &NewInvite) -> Result<(), StoreError> {
        self.run(async {
            sqlx::query(
                "INSERT INTO invites \
                 (tenant_id, id, token_hash, role, email, created_by, created_at, expires_at) \
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
            )
            .bind(self.t())
            .bind(invite.id.clone())
            .bind(invite.token_hash.as_str().to_owned())
            .bind(invite.role.as_str())
            .bind(invite.email.clone())
            .bind(invite.created_by.as_str().to_owned())
            .bind(to_i64(invite.created_at))
            .bind(to_i64(invite.expires_at))
            .execute(self.db.pool())
            .await?;
            Ok(Ok(()))
        })
    }

    fn redeem_invite(&self, hash: &TokenHash, by: &UserId, at: u64) -> Result<Invite, StoreError> {
        self.run(async {
            let mut tx = self.db.begin_write().await?;
            // The redeeming user must exist, like every other user a method names.
            if !user_exists(&mut tx, self.t(), by).await? {
                return refuse(refusal::user_unknown(by));
            }
            let done = sqlx::query(
                "UPDATE invites SET redeemed_by = $1, redeemed_at = $2 \
                 WHERE tenant_id = $3 AND token_hash = $4 AND redeemed_at IS NULL \
                 AND expires_at > $2",
            )
            .bind(by.as_str().to_owned())
            .bind(to_i64(at))
            .bind(self.t())
            .bind(hash.as_str().to_owned())
            .execute(&mut *tx)
            .await?;
            if done.rows_affected() == 0 {
                return why_not_redeemable(
                    &mut tx,
                    "invites",
                    "token_hash",
                    self.t(),
                    hash,
                    "invite",
                )
                .await;
            }
            let row = sqlx::query(&format!(
                "SELECT {INVITE} FROM invites WHERE tenant_id = $1 AND token_hash = $2"
            ))
            .bind(self.t())
            .bind(hash.as_str().to_owned())
            .fetch_one(&mut *tx)
            .await?;
            tx.commit().await?;
            Ok(invite_of(&row))
        })
    }

    fn set_bootstrap(&self, hash: &TokenHash, at: u64) -> Result<(), StoreError> {
        self.run(async {
            let done = sqlx::query(
                "INSERT INTO bootstrap (tenant_id, code_hash, created_at, consumed_at) \
                 VALUES ($1, $2, $3, NULL) \
                 ON CONFLICT (tenant_id) DO UPDATE SET \
                 code_hash = excluded.code_hash, created_at = excluded.created_at \
                 WHERE bootstrap.consumed_at IS NULL",
            )
            .bind(self.t())
            .bind(hash.as_str().to_owned())
            .bind(to_i64(at))
            .execute(self.db.pool())
            .await?;
            if done.rows_affected() == 0 {
                return refuse(refusal::BOOTSTRAP_ALREADY_CONSUMED);
            }
            Ok(Ok(()))
        })
    }

    fn bootstrap(&self) -> Result<Option<Bootstrap>, StoreError> {
        self.run(async {
            let row = sqlx::query(
                "SELECT code_hash, created_at, consumed_at FROM bootstrap WHERE tenant_id = $1",
            )
            .bind(self.t())
            .fetch_optional(self.db.pool())
            .await?;
            Ok(row
                .map(|row| {
                    Ok(Bootstrap {
                        code_hash: hash_of(&row, "code_hash")?,
                        created_at: time(&row, "created_at")?,
                        consumed_at: opt_time(&row, "consumed_at")?,
                    })
                })
                .transpose())
        })
    }

    fn consume_bootstrap(&self, hash: &TokenHash, at: u64) -> Result<(), StoreError> {
        self.run(async {
            let mut tx = self.db.begin_write().await?;
            let done = sqlx::query(
                "UPDATE bootstrap SET consumed_at = $1 \
                 WHERE tenant_id = $2 AND code_hash = $3 AND consumed_at IS NULL",
            )
            .bind(to_i64(at))
            .bind(self.t())
            .bind(hash.as_str().to_owned())
            .execute(&mut *tx)
            .await?;
            if done.rows_affected() == 0 {
                let row = sqlx::query("SELECT consumed_at FROM bootstrap WHERE tenant_id = $1")
                    .bind(self.t())
                    .fetch_optional(&mut *tx)
                    .await?;
                return match row {
                    None => refuse(refusal::BOOTSTRAP_NOT_SET),
                    Some(row) if matches!(opt_time(&row, "consumed_at"), Ok(Some(_))) => {
                        refuse(refusal::BOOTSTRAP_CONSUMED)
                    }
                    Some(_) => refuse(refusal::BOOTSTRAP_MISMATCH),
                };
            }
            tx.commit().await?;
            Ok(Ok(()))
        })
    }

    fn put_refresh(&self, token: &RefreshToken) -> Result<(), StoreError> {
        self.run(async {
            insert_refresh(self.db.pool().acquire().await?.as_mut(), self.t(), token).await?;
            Ok(Ok(()))
        })
    }

    fn rotate_refresh(
        &self,
        old: &TokenHash,
        new: &RefreshToken,
        at: u64,
    ) -> Result<(), StoreError> {
        self.run(async {
            let mut tx = self.db.begin_write().await?;
            let row = sqlx::query(
                "SELECT family, expires_at, revoked_at FROM refresh_tokens \
                 WHERE tenant_id = $1 AND token_hash = $2",
            )
            .bind(self.t())
            .bind(old.as_str().to_owned())
            .fetch_optional(&mut *tx)
            .await?;
            let Some(row) = row else {
                return refuse("the refresh token is unknown");
            };
            let (family, expires_at, revoked_at) = match (
                text(&row, "family"),
                time(&row, "expires_at"),
                opt_time(&row, "revoked_at"),
            ) {
                (Ok(family), Ok(expires_at), Ok(revoked_at)) => (family, expires_at, revoked_at),
                (Err(fail), _, _) | (_, Err(fail), _) | (_, _, Err(fail)) => return Ok(Err(fail)),
            };
            if revoked_at.is_some() {
                // A revoked token came back: someone holds a copy. The family's revocation is the
                // point of this call, so it is committed, and the caller is then refused.
                sqlx::query(
                    "UPDATE refresh_tokens SET revoked_at = $1 \
                     WHERE tenant_id = $2 AND family = $3 AND revoked_at IS NULL",
                )
                .bind(to_i64(at))
                .bind(self.t())
                .bind(family)
                .execute(&mut *tx)
                .await?;
                tx.commit().await?;
                return refuse("the refresh token was revoked; its family is revoked");
            }
            if at >= expires_at {
                return refuse("the refresh token has expired");
            }
            sqlx::query(
                "UPDATE refresh_tokens SET revoked_at = $1 WHERE tenant_id = $2 AND token_hash = $3",
            )
            .bind(to_i64(at))
            .bind(self.t())
            .bind(old.as_str().to_owned())
            .execute(&mut *tx)
            .await?;
            // A duplicate hash or an unknown user is the constraint's refusal, and rolls the
            // revocation of `old` back with it.
            insert_refresh(&mut tx, self.t(), new).await?;
            tx.commit().await?;
            Ok(Ok(()))
        })
    }

    fn revoke_refresh_for(&self, user: &UserId, at: u64) -> Result<u64, StoreError> {
        self.run(async {
            let done = sqlx::query(
                "UPDATE refresh_tokens SET revoked_at = $1 \
                 WHERE tenant_id = $2 AND user_id = $3 AND revoked_at IS NULL",
            )
            .bind(to_i64(at))
            .bind(self.t())
            .bind(user.as_str().to_owned())
            .execute(self.db.pool())
            .await?;
            Ok(Ok(done.rows_affected()))
        })
    }

    fn refresh(&self, hash: &TokenHash) -> Result<Option<RefreshToken>, StoreError> {
        self.run(async {
            let row = sqlx::query(&format!(
                "SELECT {REFRESH} FROM refresh_tokens WHERE tenant_id = $1 AND token_hash = $2"
            ))
            .bind(self.t())
            .bind(hash.as_str().to_owned())
            .fetch_optional(self.db.pool())
            .await?;
            Ok(row.as_ref().map(refresh_of).transpose())
        })
    }

    fn record_serial(
        &self,
        serial: &CertSerial,
        daemon: &WireDaemonId,
        issued: u64,
        expires: u64,
    ) -> Result<(), StoreError> {
        self.run(async {
            sqlx::query(
                "INSERT INTO cert_serials (tenant_id, serial, daemon_id, issued_at, expires_at) \
                 VALUES ($1, $2, $3, $4, $5)",
            )
            .bind(self.t())
            .bind(serial.as_str().to_owned())
            .bind(daemon.as_str().to_owned())
            .bind(to_i64(issued))
            .bind(to_i64(expires))
            .execute(self.db.pool())
            .await?;
            Ok(Ok(()))
        })
    }

    fn revoke_serial(&self, serial: &CertSerial, at: u64, reason: &str) -> Result<(), StoreError> {
        self.run(async {
            sqlx::query(
                "INSERT INTO cert_revocations (tenant_id, serial, revoked_at, reason) \
                 VALUES ($1, $2, $3, $4) ON CONFLICT (tenant_id, serial) DO NOTHING",
            )
            .bind(self.t())
            .bind(serial.as_str().to_owned())
            .bind(to_i64(at))
            .bind(reason.to_owned())
            .execute(self.db.pool())
            .await?;
            Ok(Ok(()))
        })
    }

    fn is_revoked(&self, serial: &CertSerial) -> Result<bool, StoreError> {
        self.run(async {
            let row =
                sqlx::query("SELECT 1 FROM cert_revocations WHERE tenant_id = $1 AND serial = $2")
                    .bind(self.t())
                    .bind(serial.as_str().to_owned())
                    .fetch_optional(self.db.pool())
                    .await?;
            Ok(Ok(row.is_some()))
        })
    }

    fn revocations(&self) -> Result<Vec<Revocation>, StoreError> {
        self.run(async {
            let rows = sqlx::query(
                "SELECT serial, revoked_at, reason FROM cert_revocations \
                 WHERE tenant_id = $1 ORDER BY revoked_at, serial",
            )
            .bind(self.t())
            .fetch_all(self.db.pool())
            .await?;
            Ok(rows
                .iter()
                .map(|row| {
                    Ok(Revocation {
                        serial: CertSerial::new(&text(row, "serial")?).map_err(corrupt)?,
                        revoked_at: time(row, "revoked_at")?,
                        reason: text(row, "reason")?,
                    })
                })
                .collect())
        })
    }

    fn create_enrollment(&self, enrollment: &NewEnrollment) -> Result<(), StoreError> {
        self.run(async {
            sqlx::query(
                "INSERT INTO enrollments (tenant_id, token_hash, created_by, created_at, expires_at) \
                 VALUES ($1, $2, $3, $4, $5)",
            )
            .bind(self.t())
            .bind(enrollment.token_hash.as_str().to_owned())
            .bind(enrollment.created_by.as_ref().map(|user| user.as_str().to_owned()))
            .bind(to_i64(enrollment.created_at))
            .bind(to_i64(enrollment.expires_at))
            .execute(self.db.pool())
            .await?;
            Ok(Ok(()))
        })
    }

    fn redeem_enrollment(
        &self,
        hash: &TokenHash,
        daemon: &WireDaemonId,
        at: u64,
    ) -> Result<(), StoreError> {
        self.run(async {
            let mut tx = self.db.begin_write().await?;
            let done = sqlx::query(
                "UPDATE enrollments SET redeemed_daemon_id = $1, redeemed_at = $2 \
                 WHERE tenant_id = $3 AND token_hash = $4 AND redeemed_at IS NULL \
                 AND expires_at > $2",
            )
            .bind(daemon.as_str().to_owned())
            .bind(to_i64(at))
            .bind(self.t())
            .bind(hash.as_str().to_owned())
            .execute(&mut *tx)
            .await?;
            if done.rows_affected() == 0 {
                return why_not_redeemable(
                    &mut tx,
                    "enrollments",
                    "token_hash",
                    self.t(),
                    hash,
                    "enrollment",
                )
                .await;
            }
            tx.commit().await?;
            Ok(Ok(()))
        })
    }
}

async fn insert_refresh(
    conn: &mut AnyConnection,
    tenant: String,
    token: &RefreshToken,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO refresh_tokens \
         (tenant_id, token_hash, user_id, family, issued_at, expires_at, revoked_at) \
         VALUES ($1, $2, $3, $4, $5, $6, $7)",
    )
    .bind(tenant)
    .bind(token.hash.as_str().to_owned())
    .bind(token.user.as_str().to_owned())
    .bind(token.family.clone())
    .bind(to_i64(token.issued_at))
    .bind(to_i64(token.expires_at))
    .bind(token.revoked_at.map(to_i64))
    .execute(conn)
    .await
    .map(drop)
}
