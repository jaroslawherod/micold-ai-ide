//! One daemon as the registry lists it (data-model.md, *DaemonRecord*).

use std::collections::BTreeSet;

use serde::{Deserialize, Deserializer, Serialize};

use super::auth::UserId;
use super::DEFAULT_TENANT;
use crate::daemon_config::Kind;
use crate::protocol::routing::WireDaemonId;

/// The owner of a record that exists before any user does (#1948): the first owner user takes it
/// over.
pub const UNOWNED: &str = "unowned";

/// Who owns a daemon, project or session record (#1948, FR-007): [`UNOWNED`] until the first owner
/// user takes it over with `RegistryStore::reassign_unowned`, then that user.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub enum OwnerId {
    /// Made before any user existed.
    #[default]
    Unowned,
    /// Owned by this user.
    User(UserId),
}

impl OwnerId {
    /// The stored text: [`UNOWNED`] or a user id. Blank or `unowned` reads as [`OwnerId::Unowned`].
    pub fn parse(text: &str) -> Self {
        UserId::new(text).map_or(Self::Unowned, Self::User)
    }

    /// The owner as stored text.
    pub fn as_str(&self) -> &str {
        match self {
            Self::Unowned => UNOWNED,
            Self::User(user) => user.as_str(),
        }
    }
}

impl Serialize for OwnerId {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for OwnerId {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        Ok(Self::parse(&String::deserialize(deserializer)?))
    }
}

/// Whether the orchestrator can reach a daemon right now.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Health {
    /// A live link answers.
    Online,
    /// No answer within the deadline, or the orchestrator restarted.
    #[default]
    Offline,
    /// The daemon speaks another protocol version.
    NeedsUpdate,
}

/// The tenant a daemon belongs to: never empty.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(transparent)]
pub struct TenantId(String);

impl TenantId {
    /// The announced tenant, trimmed; [`DEFAULT_TENANT`] when none or blank.
    pub fn new(announced: Option<&str>) -> Self {
        let trimmed = announced.map(str::trim).unwrap_or_default();
        Self(if trimmed.is_empty() {
            DEFAULT_TENANT.to_owned()
        } else {
            trimmed.to_owned()
        })
    }

    /// The tenant as text.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Default for TenantId {
    fn default() -> Self {
        Self::new(None)
    }
}

impl<'de> Deserialize<'de> for TenantId {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let text = String::deserialize(deserializer)?;
        Ok(Self::new(Some(&text)))
    }
}

/// One daemon in the registry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DaemonRecord {
    /// Unique key, stable across relinks.
    pub id: WireDaemonId,
    /// Host, ssh, container or kubernetes.
    pub kind: Kind,
    /// Display name: trimmed, never empty (falls back to the id).
    pub name: String,
    /// The protocol version the daemon announced.
    pub protocol_version: u32,
    /// What the daemon says it can do; opaque, sorted.
    #[serde(default)]
    pub capabilities: BTreeSet<String>,
    /// Reachability.
    #[serde(default)]
    pub health: Health,
    /// Unix seconds of the last answer (wall clock, display only).
    #[serde(default)]
    pub last_seen: u64,
    /// The tenant it belongs to.
    #[serde(default)]
    pub tenant: TenantId,
    /// Who owns it; `unowned` when absent, so a `registry.json` from before owners loads. Stores
    /// write it when the record is inserted; an upsert or save keeps a stored owner.
    #[serde(default)]
    pub owner: OwnerId,
}

impl DaemonRecord {
    /// A record as announced: the name is trimmed and falls back to the id. It starts `offline`,
    /// never seen; the registry sets both when the link is confirmed.
    pub fn new(
        id: WireDaemonId,
        kind: Kind,
        name: &str,
        protocol_version: u32,
        capabilities: BTreeSet<String>,
        tenant: Option<&str>,
    ) -> Self {
        let name = name.trim();
        let name = if name.is_empty() {
            id.as_str().to_owned()
        } else {
            name.to_owned()
        };
        Self {
            id,
            kind,
            name,
            protocol_version,
            capabilities,
            health: Health::Offline,
            last_seen: 0,
            tenant: TenantId::new(tenant),
            owner: OwnerId::Unowned,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn id(text: &str) -> WireDaemonId {
        WireDaemonId::new(text).unwrap()
    }

    fn record(name: &str, tenant: Option<&str>) -> DaemonRecord {
        DaemonRecord::new(id("box"), Kind::Host, name, 81, BTreeSet::new(), tenant)
    }

    #[test]
    fn name_is_trimmed() {
        assert_eq!(record("  my box ", None).name, "my box");
    }

    #[test]
    fn blank_name_falls_back_to_the_id() {
        assert_eq!(record("   ", None).name, "box");
        assert_eq!(record("", None).name, "box");
    }

    #[test]
    fn missing_or_blank_tenant_is_the_default() {
        assert_eq!(record("n", None).tenant.as_str(), "default");
        assert_eq!(record("n", Some("  ")).tenant.as_str(), "default");
        assert_eq!(record("n", Some(" acme ")).tenant.as_str(), "acme");
    }

    #[test]
    fn health_serializes_as_its_listing_words() {
        let words: Vec<String> = [Health::Online, Health::Offline, Health::NeedsUpdate]
            .iter()
            .map(|h| serde_json::to_string(h).unwrap())
            .collect();
        assert_eq!(words, [r#""online""#, r#""offline""#, r#""needs-update""#]);
    }

    #[test]
    fn a_missing_health_reads_offline_and_unknown_fields_are_ignored() {
        let json = r#"{"id":"box","kind":"host","name":"b","protocol_version":80,
            "capabilities":["x"],"last_seen":3,"tenant":"default","extra":true}"#;
        let record: DaemonRecord = serde_json::from_str(json).unwrap();
        assert_eq!(record.health, Health::Offline);
        assert_eq!(record.last_seen, 3);
    }

    #[test]
    fn an_empty_stored_tenant_reads_as_the_default() {
        let json = r#"{"id":"box","kind":"ssh","name":"b","protocol_version":80,"tenant":""}"#;
        let record: DaemonRecord = serde_json::from_str(json).unwrap();
        assert_eq!(record.tenant.as_str(), "default");
    }

    #[test]
    fn an_owner_is_unowned_or_a_user_id() {
        assert_eq!(OwnerId::parse("unowned"), OwnerId::Unowned);
        assert_eq!(OwnerId::parse("  "), OwnerId::Unowned);
        let owner = OwnerId::parse(" u1 ");
        assert_eq!(owner, OwnerId::User(UserId::new("u1").unwrap()));
        assert_eq!(owner.as_str(), "u1");
        assert_eq!(OwnerId::Unowned.as_str(), UNOWNED);
        assert_eq!(serde_json::to_string(&owner).unwrap(), r#""u1""#);
    }

    #[test]
    fn a_record_without_an_owner_reads_unowned_and_a_stored_owner_round_trips() {
        let json = r#"{"id":"box","kind":"host","name":"b","protocol_version":80}"#;
        let old: DaemonRecord = serde_json::from_str(json).unwrap();
        assert_eq!(old.owner, OwnerId::Unowned);
        assert_eq!(record("n", None).owner, OwnerId::Unowned);
        let mut owned = old;
        owned.owner = OwnerId::parse("u1");
        let text = serde_json::to_string(&owned).unwrap();
        assert!(text.contains(r#""owner":"u1""#), "{text}");
        assert_eq!(serde_json::from_str::<DaemonRecord>(&text).unwrap(), owned);
    }
}
