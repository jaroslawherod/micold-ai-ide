//! A Kubernetes daemon's settings and their validation (feature 686, data model "KubeSettings").
//!
//! Every refusal is `RegistryError::InvalidField` naming the field (FR-018). The kube context is
//! only checked for being non-empty here: whether the kubeconfig has it is a start-time question,
//! so a changed kubeconfig never makes the saved settings unreadable.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::daemon_image::{DaemonImageSource, DaemonResources};
use crate::daemons::RegistryError;
use crate::kube::credentials::KubeCredential;

/// One gibibyte, the smallest volume a Kubernetes daemon accepts.
pub const GIB: u64 = 1024 * 1024 * 1024;

/// The volume size a new Kubernetes daemon gets (spec Assumptions).
pub const DEFAULT_VOLUME_SIZE: Quantity = Quantity(10 * GIB);

fn invalid(field: &'static str) -> RegistryError {
    RegistryError::InvalidField { field }
}

/// A context name from the user's kubeconfig.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct KubeContextName(pub String);

/// A namespace: a DNS-1123 label.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Namespace(pub String);

/// Where a built image is pushed: `host[:port]/path`, with no tag and no digest.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct RegistryRepo(pub String);

/// The node architecture a built image targets.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NodeArch {
    /// `linux/amd64`.
    #[default]
    Amd64,
    /// `linux/arm64`.
    Arm64,
}

/// A node-selector key: `[prefix/]name` in Kubernetes label syntax.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct LabelKey(pub String);

/// A node-selector value in Kubernetes label syntax (may be empty).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct LabelValue(pub String);

/// A toleration's effect.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaintEffect {
    /// `NoSchedule`.
    NoSchedule,
    /// `PreferNoSchedule`.
    PreferNoSchedule,
    /// `NoExecute`.
    NoExecute,
}

/// How a toleration matches a taint's value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TolerationOperator {
    /// `key[=value]`: the value must equal (empty when not given).
    Equal(Option<String>),
    /// `key:Exists`: any value.
    Exists,
}

/// One toleration, written `key[=value]:Effect` or `key:Exists:Effect`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Toleration {
    /// The taint key.
    pub key: String,
    /// How the value matches.
    pub operator: TolerationOperator,
    /// The taint effect tolerated.
    pub effect: TaintEffect,
}

/// A storage size in bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Quantity(pub u64);

/// A Kubernetes daemon's settings.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KubeSettings {
    /// The kubeconfig context.
    pub context: KubeContextName,
    /// The namespace the daemon's objects live in.
    pub namespace: Namespace,
    /// Where the daemon's image comes from.
    #[serde(default)]
    pub image: DaemonImageSource,
    /// Where a built image is pushed; required when `image` is `Build`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub registry: Option<RegistryRepo>,
    /// The architecture a built image targets.
    #[serde(default)]
    pub arch: NodeArch,
    /// CPU and memory requests and limits.
    #[serde(default)]
    pub resources: DaemonResources,
    /// The pod's node selector.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub node_selector: BTreeMap<LabelKey, LabelValue>,
    /// The pod's tolerations.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tolerations: Vec<Toleration>,
    /// The size of the daemon's volume.
    #[serde(default = "default_volume_size")]
    pub volume_size: Quantity,
    /// The volume's storage class; `None` is the cluster default.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub storage_class: Option<String>,
    /// The credentials the daemon may receive.
    #[serde(default, skip_serializing_if = "BTreeSet::is_empty")]
    pub allowed: BTreeSet<KubeCredential>,
}

fn default_volume_size() -> Quantity {
    DEFAULT_VOLUME_SIZE
}

impl KubeSettings {
    /// Settings for `context` and `namespace` with every other field at its default.
    pub fn new(context: &str, namespace: &str, image: DaemonImageSource) -> Self {
        Self {
            context: KubeContextName(context.to_string()),
            namespace: Namespace(namespace.to_string()),
            image,
            registry: None,
            arch: NodeArch::default(),
            resources: DaemonResources::default(),
            node_selector: BTreeMap::new(),
            tolerations: Vec::new(),
            volume_size: DEFAULT_VOLUME_SIZE,
            storage_class: None,
            allowed: BTreeSet::new(),
        }
    }

    /// The row's endpoint: `<context> / <namespace> / <pod>` (contract §7).
    pub fn endpoint(&self, daemon_name: &str) -> String {
        let pod =
            crate::kube::names::ResourceNames::new(&self.context.0, &self.namespace.0, daemon_name)
                .pod();
        format!("{} / {} / {pod}", self.context.0, self.namespace.0)
    }

    /// Check every field; the first refusal names its field.
    pub fn validate(&self) -> Result<(), RegistryError> {
        if self.context.0.trim().is_empty() {
            return Err(invalid("context"));
        }
        if !is_dns_label(&self.namespace.0) {
            return Err(invalid("namespace"));
        }
        match &self.registry {
            Some(repo) if !is_registry_repo(&repo.0) => return Err(invalid("registry")),
            None if matches!(self.image, DaemonImageSource::Build { .. }) => {
                return Err(invalid("registry"))
            }
            _ => {}
        }
        let labels_ok = self
            .node_selector
            .iter()
            .all(|(k, v)| is_label_key(&k.0) && is_label_value(&v.0));
        if !labels_ok {
            return Err(invalid("node_selector"));
        }
        if self.volume_size.0 < GIB {
            return Err(invalid("volume_size"));
        }
        if self
            .storage_class
            .as_deref()
            .is_some_and(|c| !is_dns_subdomain(c))
        {
            return Err(invalid("storage_class"));
        }
        self.resources.validate()
    }
}

/// A DNS-1123 label: 1-63 of `[a-z0-9-]`, alphanumeric at both ends.
pub fn is_dns_label(s: &str) -> bool {
    (1..=63).contains(&s.len())
        && s.bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
        && !s.starts_with('-')
        && !s.ends_with('-')
}

/// A DNS-1123 subdomain: dot-separated labels, at most 253 characters.
fn is_dns_subdomain(s: &str) -> bool {
    s.len() <= 253 && s.split('.').all(is_dns_label)
}

/// A label name or value body: at most 63 of `[A-Za-z0-9._-]`, alphanumeric at both ends.
fn is_label_name(s: &str, allow_empty: bool) -> bool {
    if s.is_empty() {
        return allow_empty;
    }
    let alnum = |c: Option<char>| c.is_some_and(|c| c.is_ascii_alphanumeric());
    s.len() <= 63
        && s.chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'))
        && alnum(s.chars().next())
        && alnum(s.chars().last())
}

fn is_label_key(s: &str) -> bool {
    match s.split_once('/') {
        Some((prefix, name)) => is_dns_subdomain(prefix) && is_label_name(name, false),
        None => is_label_name(s, false),
    }
}

fn is_label_value(s: &str) -> bool {
    is_label_name(s, true)
}

/// `host[:port]/path` with a lowercase path and no tag or digest.
fn is_registry_repo(s: &str) -> bool {
    let Some((host, path)) = s.split_once('/') else {
        return false;
    };
    let host_ok = match host.split_once(':') {
        Some((name, port)) => {
            !name.is_empty() && !port.is_empty() && port.bytes().all(|b| b.is_ascii_digit())
        }
        None => !host.is_empty(),
    };
    let component_ok = |c: &str| {
        !c.is_empty()
            && c.bytes().all(|b| {
                b.is_ascii_lowercase() || b.is_ascii_digit() || matches!(b, b'.' | b'_' | b'-')
            })
    };
    host_ok
        && host
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'-' | b':'))
        && path.split('/').all(component_ok)
}

impl Toleration {
    /// Parse `key[=value]:Effect` or `key:Exists:Effect`.
    pub fn parse(raw: &str) -> Result<Self, RegistryError> {
        let refused = || invalid("tolerations");
        let (head, effect) = raw.rsplit_once(':').ok_or_else(refused)?;
        let effect = match effect {
            "NoSchedule" => TaintEffect::NoSchedule,
            "PreferNoSchedule" => TaintEffect::PreferNoSchedule,
            "NoExecute" => TaintEffect::NoExecute,
            _ => return Err(refused()),
        };
        let (key, operator) = match head.split_once(':') {
            Some((key, "Exists")) => (key, TolerationOperator::Exists),
            Some(_) => return Err(refused()),
            None => match head.split_once('=') {
                Some((key, value)) if is_label_value(value) => {
                    (key, TolerationOperator::Equal(Some(value.to_string())))
                }
                Some(_) => return Err(refused()),
                None => (head, TolerationOperator::Equal(None)),
            },
        };
        if !is_label_key(key) {
            return Err(refused());
        }
        Ok(Toleration {
            key: key.to_string(),
            operator,
            effect,
        })
    }
}

impl std::fmt::Display for Toleration {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let effect = match self.effect {
            TaintEffect::NoSchedule => "NoSchedule",
            TaintEffect::PreferNoSchedule => "PreferNoSchedule",
            TaintEffect::NoExecute => "NoExecute",
        };
        match &self.operator {
            TolerationOperator::Exists => write!(f, "{}:Exists:{effect}", self.key),
            TolerationOperator::Equal(Some(v)) => write!(f, "{}={v}:{effect}", self.key),
            TolerationOperator::Equal(None) => write!(f, "{}:{effect}", self.key),
        }
    }
}

impl Serialize for Toleration {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for Toleration {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let raw = String::deserialize(d)?;
        Toleration::parse(&raw).map_err(|_| serde::de::Error::custom("invalid toleration"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::daemon_image::DaemonImageSource;
    use crate::sandbox::image::ImageRef;

    fn pulled() -> DaemonImageSource {
        DaemonImageSource::Pull {
            reference: ImageRef::parse("ghcr.io/acme/micold-daemon:0.30.0").unwrap(),
        }
    }

    fn valid() -> KubeSettings {
        KubeSettings::new("kind-dev", "dev", pulled())
    }

    fn refused_field(s: &KubeSettings) -> Option<&'static str> {
        match s.validate() {
            Err(RegistryError::InvalidField { field }) => Some(field),
            _ => None,
        }
    }

    #[test]
    fn an_empty_context_is_refused_and_any_non_empty_one_is_accepted_unread() {
        let mut s = valid();
        s.context = KubeContextName("  ".into());
        assert_eq!(
            refused_field(&s),
            Some("context"),
            "a blank context names the field"
        );
        s.context = KubeContextName("not-in-any-kubeconfig".into());
        assert_eq!(
            s.validate(),
            Ok(()),
            "existence is a start-time check, not a save-time one"
        );
    }

    #[test]
    fn a_namespace_must_be_a_dns_1123_label() {
        let mut s = valid();
        for ok in ["a", &"a".repeat(63), "team-1"] {
            s.namespace = Namespace(ok.to_string());
            assert_eq!(s.validate(), Ok(()), "{ok:?} is a DNS-1123 label");
        }
        for bad in ["", &"a".repeat(64), "Dev", "-dev", "dev-", "d_v"] {
            s.namespace = Namespace(bad.to_string());
            assert_eq!(
                refused_field(&s),
                Some("namespace"),
                "{bad:?} is not a DNS-1123 label"
            );
        }
    }

    #[test]
    fn a_built_image_needs_a_registry_and_a_pulled_one_does_not() {
        let mut s = valid();
        s.image = DaemonImageSource::default();
        assert_eq!(
            refused_field(&s),
            Some("registry"),
            "Build pushes, so it needs a registry"
        );
        s.registry = Some(RegistryRepo("registry.local:5000/team".into()));
        assert_eq!(s.validate(), Ok(()));
        s.image = pulled();
        s.registry = None;
        assert_eq!(s.validate(), Ok(()), "Pull uses the reference as given");
    }

    #[test]
    fn a_registry_is_host_and_path_without_tag_or_digest() {
        let mut s = valid();
        for ok in [
            "registry.local:5000/team",
            "ghcr.io/acme",
            "localhost:5001/x/y",
        ] {
            s.registry = Some(RegistryRepo(ok.into()));
            assert_eq!(s.validate(), Ok(()), "{ok:?} is host[:port]/path");
        }
        for bad in [
            "ghcr.io/acme:1.0",
            "ghcr.io/acme@sha256:abc",
            "ghcr.io",
            "",
            "ghcr.io/Acme",
        ] {
            s.registry = Some(RegistryRepo(bad.into()));
            assert_eq!(
                refused_field(&s),
                Some("registry"),
                "{bad:?} is not a bare repository"
            );
        }
    }

    #[test]
    fn node_selector_entries_follow_label_syntax() {
        let mut s = valid();
        s.node_selector.insert(
            LabelKey("kubernetes.io/arch".into()),
            LabelValue("amd64".into()),
        );
        s.node_selector
            .insert(LabelKey("pool".into()), LabelValue("".into()));
        assert_eq!(
            s.validate(),
            Ok(()),
            "prefixed keys and empty values are label syntax"
        );
        for (k, v) in [
            ("bad key", "x"),
            ("pool", "no spaces"),
            ("-pool", "x"),
            ("p", &"v".repeat(64)),
        ] {
            let mut t = valid();
            t.node_selector
                .insert(LabelKey(k.into()), LabelValue(v.into()));
            assert_eq!(
                refused_field(&t),
                Some("node_selector"),
                "{k:?}={v:?} is refused"
            );
        }
    }

    #[test]
    fn tolerations_parse_both_forms_for_the_three_effects() {
        for effect in ["NoSchedule", "PreferNoSchedule", "NoExecute"] {
            for raw in [
                format!("gpu=true:{effect}"),
                format!("gpu:{effect}"),
                format!("gpu:Exists:{effect}"),
            ] {
                let t = Toleration::parse(&raw).unwrap_or_else(|e| panic!("{raw}: {e:?}"));
                assert_eq!(
                    t.to_string(),
                    raw,
                    "a toleration writes back as it was given"
                );
            }
        }
        assert_eq!(
            Toleration::parse("gpu=true:NoSchedule").unwrap().operator,
            TolerationOperator::Equal(Some("true".into())),
        );
    }

    #[test]
    fn a_toleration_with_another_effect_is_refused_naming_tolerations() {
        for bad in [
            "gpu=true",
            "gpu:Evict",
            "gpu:Exists",
            ":NoSchedule",
            "gpu:Maybe:NoSchedule",
        ] {
            assert_eq!(
                Toleration::parse(bad),
                Err(RegistryError::InvalidField {
                    field: "tolerations"
                }),
                "{bad:?}"
            );
        }
    }

    #[test]
    fn the_volume_is_at_least_one_gib_and_ten_by_default() {
        let mut s = valid();
        assert_eq!(s.volume_size, Quantity(10 * GIB), "the default is 10 GiB");
        s.volume_size = Quantity(GIB);
        assert_eq!(s.validate(), Ok(()), "exactly 1 GiB is accepted");
        s.volume_size = Quantity(GIB - 1);
        assert_eq!(refused_field(&s), Some("volume_size"));
    }

    #[test]
    fn defaults_are_amd64_no_storage_class_and_nothing_allowed() {
        let s: KubeSettings = serde_json::from_value(serde_json::json!({
            "context": "kind-dev",
            "namespace": "dev",
        }))
        .unwrap();
        assert_eq!(s.arch, NodeArch::Amd64);
        assert_eq!(s.storage_class, None, "none means the cluster default");
        assert!(
            s.allowed.is_empty(),
            "nothing leaves this machine unless allowed (SP-1)"
        );
        assert_eq!(s.volume_size, DEFAULT_VOLUME_SIZE);
    }
}
