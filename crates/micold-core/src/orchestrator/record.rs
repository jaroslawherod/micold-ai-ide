//! One daemon as the registry lists it (data-model.md, *DaemonRecord*).

use std::collections::BTreeSet;

use serde::{Deserialize, Deserializer, Serialize};

use super::DEFAULT_TENANT;
use crate::daemon_config::Kind;
use crate::protocol::routing::WireDaemonId;

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
}
