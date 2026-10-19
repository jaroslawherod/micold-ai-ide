//! A Kubernetes daemon's settings and their validation (feature 686, data model "KubeSettings").
//!
//! Every refusal is `RegistryError::InvalidField` naming the field (FR-018). The kube context is
//! only checked for being non-empty here: whether the kubeconfig has it is a start-time question,
//! so a changed kubeconfig never makes the saved settings unreadable.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::daemon_resources::DaemonResources;
use crate::daemons::RegistryError;
use crate::kube::credentials::{KubeCredential, RawKubeCredential};
use crate::sandbox::image::ImageRef;
use crate::session::AiCli;

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

impl Quantity {
    /// The quantity as a manifest writes it: whole `Ti`, `Gi`, `Mi` or `Ki` when exact, else bytes.
    pub fn to_k8s(self) -> String {
        BINARY_UNITS
            .iter()
            .find(|(_, size)| self.0 >= *size && self.0.is_multiple_of(*size))
            .map_or_else(
                || self.0.to_string(),
                |(unit, size)| format!("{}{unit}", self.0 / size),
            )
    }

    /// Read a Kubernetes quantity (`10Gi`, `500M`, `1073741824`); `None` for anything else.
    pub fn parse_k8s(text: &str) -> Option<Self> {
        let split = text
            .find(|c: char| !c.is_ascii_digit())
            .unwrap_or(text.len());
        let (digits, suffix) = text.split_at(split);
        if digits.is_empty() {
            return None;
        }
        let multiplier = match suffix {
            "" => 1,
            _ => BINARY_UNITS
                .iter()
                .chain(DECIMAL_UNITS.iter())
                .find(|(unit, _)| *unit == suffix)
                .map(|(_, size)| *size)?,
        };
        digits
            .parse::<u64>()
            .ok()?
            .checked_mul(multiplier)
            .map(Quantity)
    }
}

/// Binary suffixes, largest first.
const BINARY_UNITS: [(&str, u64); 4] = [
    ("Ti", 1 << 40),
    ("Gi", 1 << 30),
    ("Mi", 1 << 20),
    ("Ki", 1 << 10),
];

/// Decimal suffixes a quantity may be read with.
const DECIMAL_UNITS: [(&str, u64); 4] = [
    ("T", 1_000_000_000_000),
    ("G", 1_000_000_000),
    ("M", 1_000_000),
    ("k", 1_000),
];

/// A Kubernetes daemon's settings.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(from = "RawKubeSettings")]
pub struct KubeSettings {
    /// The kubeconfig context.
    pub context: KubeContextName,
    /// The namespace the daemon's objects live in.
    pub namespace: Namespace,
    /// The image the pod runs; `None` is #806's pinned SSH image (FR-004).
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        with = "stored_image"
    )]
    pub image: Option<ImageRef>,
    /// The AI CLIs installed into the volume when the pod starts; may be empty (terminals only).
    #[serde(default)]
    pub providers: BTreeSet<AiCli>,
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
    /// Set on read when `allowed` named an item #1002 removed (the git credentials, an SSH key,
    /// FR-002). Never written; the daemon's entry turns it into a stored notice
    /// ([`Self::take_removed_git_credentials`], FR-003).
    #[serde(skip)]
    pub removed_git_credentials: bool,
}

/// The stored shape of [`KubeSettings`]: `allowed` may name the items #1002 removed.
#[derive(Deserialize)]
struct RawKubeSettings {
    context: KubeContextName,
    namespace: Namespace,
    #[serde(default, deserialize_with = "stored_image::deserialize")]
    image: Option<ImageRef>,
    #[serde(default)]
    providers: BTreeSet<AiCli>,
    #[serde(default)]
    resources: DaemonResources,
    #[serde(default)]
    node_selector: BTreeMap<LabelKey, LabelValue>,
    #[serde(default)]
    tolerations: Vec<Toleration>,
    #[serde(default = "default_volume_size")]
    volume_size: Quantity,
    #[serde(default)]
    storage_class: Option<String>,
    #[serde(default)]
    allowed: Vec<RawKubeCredential>,
}

impl From<RawKubeSettings> for KubeSettings {
    fn from(raw: RawKubeSettings) -> Self {
        let removed_git_credentials = raw
            .allowed
            .iter()
            .any(|c| matches!(c, RawKubeCredential::Removed(_)));
        Self {
            context: raw.context,
            namespace: raw.namespace,
            image: raw.image,
            providers: raw.providers,
            resources: raw.resources,
            node_selector: raw.node_selector,
            tolerations: raw.tolerations,
            volume_size: raw.volume_size,
            storage_class: raw.storage_class,
            allowed: raw
                .allowed
                .into_iter()
                .filter_map(|c| match c {
                    RawKubeCredential::Kept(kept) => Some(kept),
                    RawKubeCredential::Removed(_) => None,
                })
                .collect(),
            removed_git_credentials,
        }
    }
}

fn default_volume_size() -> Quantity {
    DEFAULT_VOLUME_SIZE
}

impl KubeSettings {
    /// Settings for `context` and `namespace` installing the `enabled` AI CLIs, with every other
    /// field at its default.
    pub fn new(context: &str, namespace: &str, enabled: &[AiCli]) -> Self {
        Self {
            context: KubeContextName(context.to_string()),
            namespace: Namespace(namespace.to_string()),
            image: None,
            providers: enabled.iter().copied().collect(),
            resources: DaemonResources::default(),
            node_selector: BTreeMap::new(),
            tolerations: Vec::new(),
            volume_size: DEFAULT_VOLUME_SIZE,
            storage_class: None,
            allowed: BTreeSet::new(),
            removed_git_credentials: false,
        }
    }

    /// Whether the stored `allowed` named a removed git credential item, clearing the fact.
    pub fn take_removed_git_credentials(&mut self) -> bool {
        std::mem::take(&mut self.removed_git_credentials)
    }

    /// The image the pod runs: the one given, else #806's pinned basic image (FR-004, FR-024).
    /// No digest is pinned yet (806 T049), so the reference is the content-hash tag.
    pub fn pod_image(&self) -> String {
        self.image.as_ref().map_or_else(
            || {
                format!(
                    "{}:{}",
                    crate::sandbox::basic_image::BASIC_IMAGE_REPOSITORY,
                    crate::sandbox::basic_image::BASIC_IMAGE_CONTENT_HASH
                )
            },
            ImageRef::to_reference,
        )
    }

    /// Whether the given image can change under a running daemon, as the sandbox flags it.
    pub fn image_is_moving(&self) -> bool {
        self.image.as_ref().is_some_and(ImageRef::is_moving)
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
        // One secret per kind: two pull files would share a secret name.
        let pulls = self
            .allowed
            .iter()
            .filter(|c| matches!(c, KubeCredential::RegistryPull { .. }))
            .count();
        if pulls > 1 {
            return Err(invalid("allowed"));
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

/// `image` is stored as the reference string; `ImageRef` has no serde of its own.
mod stored_image {
    use serde::{Deserialize, Deserializer, Serializer};

    use crate::sandbox::image::ImageRef;

    pub fn serialize<S: Serializer>(image: &Option<ImageRef>, s: S) -> Result<S::Ok, S::Error> {
        match image {
            Some(r) => s.serialize_str(&r.to_reference()),
            None => s.serialize_none(),
        }
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Option<ImageRef>, D::Error> {
        Option::<String>::deserialize(d)?
            .map(|raw| ImageRef::parse(&raw).map_err(|e| serde::de::Error::custom(e.message())))
            .transpose()
    }
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
mod quantity_tests {
    use super::*;

    #[test]
    fn a_quantity_is_written_in_the_largest_exact_binary_unit() {
        assert_eq!(Quantity(10 * GIB).to_k8s(), "10Gi");
        assert_eq!(Quantity(1024 * GIB).to_k8s(), "1Ti");
        assert_eq!(Quantity(1536 * 1024 * 1024).to_k8s(), "1536Mi");
        assert_eq!(Quantity(GIB + 1).to_k8s(), (GIB + 1).to_string());
    }

    #[test]
    fn a_quantity_is_read_with_binary_decimal_or_no_suffix() {
        assert_eq!(Quantity::parse_k8s("10Gi"), Some(Quantity(10 * GIB)));
        assert_eq!(Quantity::parse_k8s("2Ti"), Some(Quantity(2048 * GIB)));
        assert_eq!(Quantity::parse_k8s("512Mi"), Some(Quantity(512 << 20)));
        assert_eq!(Quantity::parse_k8s("4Ki"), Some(Quantity(4096)));
        assert_eq!(Quantity::parse_k8s("5G"), Some(Quantity(5_000_000_000)));
        assert_eq!(Quantity::parse_k8s("7M"), Some(Quantity(7_000_000)));
        assert_eq!(Quantity::parse_k8s("3k"), Some(Quantity(3_000)));
        assert_eq!(Quantity::parse_k8s("1T"), Some(Quantity(1_000_000_000_000)));
        assert_eq!(Quantity::parse_k8s("1073741824"), Some(Quantity(GIB)));
        assert_eq!(Quantity::parse_k8s(""), None);
        assert_eq!(Quantity::parse_k8s("Gi"), None);
        assert_eq!(Quantity::parse_k8s("1.5Gi"), None);
        assert_eq!(Quantity::parse_k8s("10Xi"), None);
    }

    #[test]
    fn writing_then_reading_is_the_same_quantity() {
        for q in [
            Quantity(GIB),
            Quantity(10 * GIB),
            Quantity(GIB + 7),
            Quantity(3 << 20),
        ] {
            assert_eq!(Quantity::parse_k8s(&q.to_k8s()), Some(q));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::daemon_resources::DaemonResources;
    use crate::sandbox::image::ImageRef;
    use crate::session::AiCli;

    fn valid() -> KubeSettings {
        KubeSettings::new("kind-dev", "dev", &[AiCli::ClaudeCode])
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
    fn the_image_is_none_by_default_and_a_given_reference_is_kept_as_is() {
        let mut s = valid();
        assert_eq!(s.image, None, "None is #806's pinned SSH image");
        assert!(!s.image_is_moving());
        let raw = "registry.local:5000/team/ssh:1.2";
        s.image = Some(ImageRef::parse(raw).unwrap());
        assert_eq!(s.validate(), Ok(()));
        let value = serde_json::to_value(&s).unwrap();
        assert_eq!(value["image"], raw, "stored as the reference given");
        let back: KubeSettings = serde_json::from_value(value).unwrap();
        assert_eq!(back, s);
    }

    #[test]
    fn the_default_pod_image_is_the_pinned_basic_image_and_a_given_one_wins() {
        let mut s = KubeSettings::new("c", "n", &[]);
        let pinned = s.pod_image();
        assert!(
            pinned.starts_with("ghcr.io/jaroslawherod/micold-basic:"),
            "{pinned}"
        );
        assert!(ImageRef::parse(&pinned).is_ok(), "{pinned}");
        s.image = Some(ImageRef::parse("ghcr.io/acme/ssh:1").unwrap());
        assert_eq!(s.pod_image(), "ghcr.io/acme/ssh:1");
    }

    #[test]
    fn a_moving_image_tag_is_flagged_as_the_sandbox_flags_it() {
        let mut s = valid();
        for raw in [
            "ghcr.io/acme/ssh:latest",
            "ghcr.io/acme/ssh",
            "ghcr.io/acme/ssh:0.30.0",
        ] {
            let reference = ImageRef::parse(raw).unwrap();
            let expected = reference.is_moving();
            s.image = Some(reference);
            assert_eq!(
                s.image_is_moving(),
                expected,
                "{raw}: same rule as sandbox::image"
            );
        }
    }

    #[test]
    fn an_unparseable_stored_image_does_not_read() {
        let raw = serde_json::json!({"context": "c", "namespace": "n", "image": "Not A Ref!"});
        assert!(serde_json::from_value::<KubeSettings>(raw).is_err());
    }

    #[test]
    fn providers_default_to_the_enabled_clis_and_may_be_empty() {
        let enabled = [AiCli::ClaudeCode, AiCli::Pi];
        let s = KubeSettings::new("kind-dev", "dev", &enabled);
        assert_eq!(s.providers, enabled.into_iter().collect());
        let none = KubeSettings::new("kind-dev", "dev", &[]);
        assert!(none.providers.is_empty());
        assert_eq!(none.validate(), Ok(()), "terminals only is allowed");
        let back: KubeSettings =
            serde_json::from_value(serde_json::to_value(&none).unwrap()).unwrap();
        assert_eq!(back, none, "an empty list stays empty");
    }

    #[test]
    fn resources_are_the_shared_daemon_resources_and_no_registry_or_arch_is_stored() {
        let s = valid();
        assert_eq!(s.resources, DaemonResources::default());
        let value = serde_json::to_value(&s).unwrap();
        for gone in ["registry", "arch"] {
            assert!(value.get(gone).is_none(), "{gone} is not a field any more");
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
    fn defaults_are_no_image_no_storage_class_and_nothing_allowed() {
        let s: KubeSettings = serde_json::from_value(serde_json::json!({
            "context": "kind-dev",
            "namespace": "dev",
        }))
        .unwrap();
        assert_eq!(s.image, None);
        assert_eq!(s.storage_class, None, "none means the cluster default");
        assert!(
            s.allowed.is_empty(),
            "nothing leaves this machine unless allowed (SP-1)"
        );
        assert_eq!(s.volume_size, DEFAULT_VOLUME_SIZE);
    }

    #[test]
    fn two_pull_files_are_refused_for_sharing_a_secret() {
        let mut s = KubeSettings::new("kind-dev", "dev", &[]);
        s.allowed
            .insert(KubeCredential::RegistryPull { auths: "/p".into() });
        assert!(s.validate().is_ok());
        s.allowed
            .insert(KubeCredential::RegistryPull { auths: "/q".into() });
        assert!(s.validate().is_err());
    }
}
