//! Names and ownership of a Kubernetes daemon's cluster objects (feature 686, research K5).
//!
//! Every name is a function of (context, namespace, daemon name), so two clients configuring the
//! same three find the same objects (FR-016, FR-023), and the objects can be listed for "Forget
//! locally" without asking the cluster.

use std::collections::{BTreeMap, BTreeSet};

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
    pub fn of(context: &str, namespace: &str, name: &str) -> Self {
        let joined = format!("{context}\0{namespace}\0{name}");
        DaemonKey(sha256_hex(joined.as_bytes())[..8].to_string())
    }

    /// The key as text.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// A DNS-1123 slug of a daemon name, at most [`SLUG_MAX`] characters.
pub fn slug(name: &str) -> String {
    slug_within(name, SLUG_MAX)
}

/// The longest suffix a daemon's object name takes (`-git-credentials`, `-signin-opencode`).
const LONGEST_SUFFIX: usize = 16;

/// How long the slug in object names may be so `micold-<slug>-<key><suffix>` stays a DNS-1123
/// label (63) for the longest suffix: 63 - 7 - 1 - 8 - 16.
const NAME_SLUG_MAX: usize = 63 - "micold-".len() - 1 - 8 - LONGEST_SUFFIX;

fn slug_within(name: &str, max: usize) -> String {
    let mut out = String::new();
    for c in name.trim().chars().flat_map(char::to_lowercase) {
        if c.is_ascii_lowercase() || c.is_ascii_digit() {
            out.push(c);
        } else if !out.is_empty() && !out.ends_with('-') {
            out.push('-');
        }
    }
    out.truncate(max);
    let trimmed = out.trim_end_matches('-');
    if trimmed.is_empty() {
        "daemon".to_string()
    } else {
        trimmed.to_string()
    }
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
        let base = format!(
            "micold-{}-{}",
            slug_within(name, NAME_SLUG_MAX),
            key.as_str()
        );
        Self { base, key }
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
        format!("{}-data", self.base)
    }

    /// The secret holding one allowed credential (`RegistryPull` is the pull secret).
    pub fn secret(&self, credential: &KubeCredential) -> String {
        match credential {
            KubeCredential::GitConfig => format!("{}-gitconfig", self.base),
            KubeCredential::GitCredentials => format!("{}-git-credentials", self.base),
            KubeCredential::GitSshKey { .. } => format!("{}-git-ssh", self.base),
            KubeCredential::SignIn { cli } => format!("{}-signin-{}", self.base, cli_slug(*cli)),
            KubeCredential::RegistryPull { .. } => self.pull_secret(),
        }
    }

    /// The secret holding the pod's SSH host key and the client's public key (contract §1).
    pub fn access_secret(&self) -> String {
        format!("{}-ssh-access", self.base)
    }

    /// Every secret name the daemon owns with `allowed`: the access secret first, then one per
    /// allowed credential.
    pub fn all_secrets(&self, allowed: &BTreeSet<KubeCredential>) -> Vec<String> {
        std::iter::once(self.access_secret())
            .chain(allowed.iter().map(|credential| self.secret(credential)))
            .collect()
    }

    /// The image pull secret.
    pub fn pull_secret(&self) -> String {
        format!("{}-pull", self.base)
    }

    /// The labels every object of this daemon carries.
    pub fn labels(&self) -> BTreeMap<String, String> {
        BTreeMap::from([
            (MANAGED_BY_LABEL.to_string(), MANAGED_BY_VALUE.to_string()),
            (DAEMON_LABEL.to_string(), self.key.as_str().to_string()),
        ])
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
pub fn is_ours(labels: &BTreeMap<String, String>, key: &DaemonKey) -> bool {
    labels.get(MANAGED_BY_LABEL).map(String::as_str) == Some(MANAGED_BY_VALUE)
        && labels.get(DAEMON_LABEL).map(String::as_str) == Some(key.as_str())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn is_dns_1123(s: &str) -> bool {
        !s.is_empty()
            && s.len() <= 63
            && s.bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
            && !s.starts_with('-')
            && !s.ends_with('-')
    }

    #[test]
    fn the_key_is_eight_hex_of_the_nul_joined_triple() {
        let key = DaemonKey::of("kind-dev", "dev", "Build box");
        assert_eq!(key.as_str(), &sha256_hex(b"kind-dev\0dev\0Build box")[..8]);
        assert_eq!(
            key,
            DaemonKey::of("kind-dev", "dev", "Build box"),
            "equal triples, equal keys"
        );
        assert_ne!(key, DaemonKey::of("kind-dev", "dev2", "Build box"));
        assert_ne!(
            DaemonKey::of("ab", "c", "d"),
            DaemonKey::of("a", "bc", "d"),
            "the separator keeps fields apart"
        );
    }

    #[test]
    fn the_slug_is_dns_1123_and_at_most_forty_characters() {
        assert_eq!(slug("Build box"), "build-box");
        assert_eq!(slug("  Ünïcode__name!! "), "n-code-name");
        for name in [
            "x".repeat(40),
            "x".repeat(41),
            "a-".repeat(30),
            "!!!".into(),
        ] {
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
        assert_eq!(
            n.secret(&KubeCredential::GitConfig),
            format!("{base}-gitconfig")
        );
        assert_eq!(
            n.secret(&KubeCredential::GitCredentials),
            format!("{base}-git-credentials")
        );
        assert_eq!(
            n.secret(&KubeCredential::GitSshKey { key: "/k".into() }),
            format!("{base}-git-ssh")
        );
        assert_eq!(
            n.secret(&KubeCredential::SignIn {
                cli: AiCli::Copilot
            }),
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
    fn the_access_secret_is_the_base_with_ssh_access() {
        let n = ResourceNames::new("kind-dev", "dev", "Build box");
        assert_eq!(n.access_secret(), format!("{}-ssh-access", n.pod()));
        let long = ResourceNames::new("c", "n", &"x".repeat(80));
        assert!(is_dns_1123(&long.access_secret()));
    }

    #[test]
    fn all_secrets_lists_the_access_secret_first_then_one_per_allowed_credential() {
        let n = ResourceNames::new("kind-dev", "dev", "Build box");
        assert_eq!(
            n.all_secrets(&BTreeSet::new()),
            vec![n.access_secret()],
            "with nothing allowed only the access secret"
        );
        let allowed = BTreeSet::from([
            KubeCredential::GitCredentials,
            KubeCredential::SignIn { cli: AiCli::Pi },
            KubeCredential::RegistryPull { auths: "/a".into() },
        ]);
        let all = n.all_secrets(&allowed);
        assert_eq!(all[0], n.access_secret(), "the access secret comes first");
        let rest: BTreeSet<String> = all[1..].iter().cloned().collect();
        let want: BTreeSet<String> = allowed.iter().map(|c| n.secret(c)).collect();
        assert_eq!(rest, want, "then exactly one name per allowed credential");
        assert_eq!(all.len(), 1 + allowed.len());
    }

    #[test]
    fn an_object_is_ours_only_when_both_labels_match() {
        let n = ResourceNames::new("kind-dev", "dev", "Build box");
        let labels = n.labels();
        assert_eq!(
            labels.get(MANAGED_BY_LABEL).map(String::as_str),
            Some(MANAGED_BY_VALUE)
        );
        assert_eq!(
            labels.get(DAEMON_LABEL).map(String::as_str),
            Some(n.key().as_str())
        );
        assert!(is_ours(&labels, n.key()));
        let mut only_managed = labels.clone();
        only_managed.remove(DAEMON_LABEL);
        assert!(
            !is_ours(&only_managed, n.key()),
            "managed-by alone is not ours"
        );
        let mut only_key = labels.clone();
        only_key.remove(MANAGED_BY_LABEL);
        assert!(
            !is_ours(&only_key, n.key()),
            "the daemon label alone is not ours"
        );
        let other = ResourceNames::new("kind-dev", "dev", "Other");
        assert!(
            !is_ours(&labels, other.key()),
            "another daemon's object is not ours"
        );
        assert!(!is_ours(&BTreeMap::new(), n.key()));
    }
}
