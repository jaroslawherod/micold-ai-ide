//! Names and ownership of a Kubernetes daemon's cluster objects (feature 686, research K5).
//!
//! Every name is a function of (context, namespace, daemon name), so two clients configuring the
//! same three find the same objects (FR-016, FR-023), and the objects can be listed for "Forget
//! locally" without asking the cluster.

use std::collections::BTreeMap;

use crate::kube::credentials::KubeCredential;
use crate::protocol::hashing::sha256_hex;
use crate::session::AiCli;

/// The label every object the app creates carries, with [`MANAGED_BY_VALUE`].
pub const MANAGED_BY_LABEL: &str = "app.kubernetes.io/managed-by";
/// The value of [`MANAGED_BY_LABEL`].
pub const MANAGED_BY_VALUE: &str = "micold-ai-ide";
/// The label naming the daemon's key.
pub const DAEMON_LABEL: &str = "micold.dev/daemon";
/// The annotation carrying the daemon's display name.
pub const DAEMON_NAME_ANNOTATION: &str = "micold.dev/daemon-name";

/// The longest slug a name contributes, leaving room for the prefix, key and suffixes within 63.
pub const SLUG_MAX: usize = 40;

/// The daemon's identity in the cluster: 8 hex of the SHA-256 of context, namespace and name.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DaemonKey(String);

impl DaemonKey {
    /// The key of the daemon `name` in `namespace` of `context`.
    pub fn of(_context: &str, _namespace: &str, _name: &str) -> Self {
        DaemonKey(String::new())
    }

    /// The key as text.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// A DNS-1123 slug of a daemon name, at most [`SLUG_MAX`] characters.
pub fn slug(_name: &str) -> String {
    String::new()
}

/// The names of one daemon's objects.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResourceNames {
    base: String,
    key: DaemonKey,
}

impl ResourceNames {
    /// The names for the daemon `name` in `namespace` of `context`.
    pub fn new(context: &str, namespace: &str, name: &str) -> Self {
        let key = DaemonKey::of(context, namespace, name);
        Self {
            base: String::new(),
            key,
        }
    }

    /// The daemon's key.
    pub fn key(&self) -> &DaemonKey {
        &self.key
    }

    /// The pod.
    pub fn pod(&self) -> String {
        self.base.clone()
    }

    /// The volume claim.
    pub fn pvc(&self) -> String {
        self.base.clone()
    }

    /// The secret holding one allowed credential (`RegistryPull` is the pull secret).
    pub fn secret(&self, _credential: &KubeCredential) -> String {
        self.base.clone()
    }

    /// The image pull secret.
    pub fn pull_secret(&self) -> String {
        self.base.clone()
    }

    /// The labels every object of this daemon carries.
    pub fn labels(&self) -> BTreeMap<String, String> {
        BTreeMap::new()
    }
}

/// The lowercase name a CLI contributes to its sign-in secret.
pub fn cli_slug(cli: AiCli) -> &'static str {
    match cli {
        AiCli::ClaudeCode => "claude",
        AiCli::Copilot => "copilot",
        AiCli::Pi => "pi",
        AiCli::Codex => "codex",
        AiCli::OpenCode => "opencode",
    }
}

/// Whether an object with `labels` belongs to the daemon `key`: both labels must match.
pub fn is_ours(_labels: &BTreeMap<String, String>, _key: &DaemonKey) -> bool {
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    fn is_dns_1123(s: &str) -> bool {
        !s.is_empty()
            && s.len() <= 63
            && s.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
            && !s.starts_with('-')
            && !s.ends_with('-')
    }

    #[test]
    fn the_key_is_eight_hex_of_the_nul_joined_triple() {
        let key = DaemonKey::of("kind-dev", "dev", "Build box");
        assert_eq!(key.as_str(), &sha256_hex(b"kind-dev\0dev\0Build box")[..8]);
        assert_eq!(key, DaemonKey::of("kind-dev", "dev", "Build box"), "equal triples, equal keys");
        assert_ne!(key, DaemonKey::of("kind-dev", "dev2", "Build box"));
        assert_ne!(
            DaemonKey::of("a\0b", "c", "d"),
            DaemonKey::of("a", "b\0c", "d"),
            "the separator keeps fields apart in practice"
        );
    }

    #[test]
    fn the_slug_is_dns_1123_and_at_most_forty_characters() {
        assert_eq!(slug("Build box"), "build-box");
        assert_eq!(slug("  Ünïcode__name!! "), "n-code-name");
        for name in ["x".repeat(40), "x".repeat(41), "a-".repeat(30), "!!!".into()] {
            let s = slug(&name);
            assert!(is_dns_1123(&s), "{name:?} → {s:?} is a DNS-1123 label");
            assert!(s.len() <= SLUG_MAX, "{name:?} → {s:?} is at most 40");
        }
    }

    #[test]
    fn resource_names_share_the_base_and_differ_by_suffix() {
        let n = ResourceNames::new("kind-dev", "dev", "Build box");
        let key = DaemonKey::of("kind-dev", "dev", "Build box");
        let base = format!("micold-build-box-{}", key.as_str());
        assert_eq!(n.pod(), base);
        assert_eq!(n.pvc(), format!("{base}-data"));
        assert_eq!(n.secret(&KubeCredential::GitConfig), format!("{base}-gitconfig"));
        assert_eq!(n.secret(&KubeCredential::GitCredentials), format!("{base}-git-credentials"));
        assert_eq!(
            n.secret(&KubeCredential::GitSshKey { key: "/k".into() }),
            format!("{base}-git-ssh")
        );
        assert_eq!(
            n.secret(&KubeCredential::SignIn { cli: AiCli::Copilot }),
            format!("{base}-signin-copilot")
        );
        assert_eq!(n.pull_secret(), format!("{base}-pull"));
        assert_eq!(
            n.secret(&KubeCredential::RegistryPull { auths: "/a".into() }),
            n.pull_secret()
        );
        let long = ResourceNames::new("c", "n", &"x".repeat(80));
        for name in [long.pod(), long.secret(&KubeCredential::GitCredentials)] {
            assert!(is_dns_1123(&name), "{name} fits a DNS-1123 label");
        }
    }

    #[test]
    fn an_object_is_ours_only_when_both_labels_match() {
        let n = ResourceNames::new("kind-dev", "dev", "Build box");
        let labels = n.labels();
        assert_eq!(labels.get(MANAGED_BY_LABEL).map(String::as_str), Some(MANAGED_BY_VALUE));
        assert_eq!(labels.get(DAEMON_LABEL).map(String::as_str), Some(n.key().as_str()));
        assert!(is_ours(&labels, n.key()));
        let mut only_managed = labels.clone();
        only_managed.remove(DAEMON_LABEL);
        assert!(!is_ours(&only_managed, n.key()), "managed-by alone is not ours");
        let mut only_key = labels.clone();
        only_key.remove(MANAGED_BY_LABEL);
        assert!(!is_ours(&only_key, n.key()), "the daemon label alone is not ours");
        let other = ResourceNames::new("kind-dev", "dev", "Other");
        assert!(!is_ours(&labels, other.key()), "another daemon's object is not ours");
        assert!(!is_ours(&BTreeMap::new(), n.key()));
    }
}
