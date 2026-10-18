//! The SSH access secret's content and the key-generation port (feature 686, research K3,
//! contract §1). Pure: the sequences in `start` read and write it through the `Cluster` port.

use std::collections::BTreeMap;

/// The data key of the pod's ed25519 host key (private, OpenSSH format).
pub const HOST_KEY: &str = "host_ed25519_key";
/// The data key of the host key's public line.
pub const HOST_KEY_PUB: &str = "host_ed25519_key.pub";
/// The data key of the `authorized_keys` file.
pub const AUTHORIZED_KEYS: &str = "authorized_keys";
/// The comment prefix of a client's line in `authorized_keys`.
pub const CLIENT_COMMENT: &str = "micold-client:";

/// An ed25519 key pair as OpenSSH writes it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyPair {
    /// The private key, OpenSSH format.
    pub private_openssh: String,
    /// The public key line: `ssh-ed25519 <base64> [comment]`.
    pub public_line: String,
}

/// Key generation (#806's, research K3). Tests use `fake::FakeKeyGen`.
pub trait KeyGen: Send + Sync {
    /// A fresh key pair whose public line carries `comment`.
    fn generate(&self, comment: &str) -> KeyPair;
}

/// One client's line in `authorized_keys`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthorizedKey {
    /// The client's id (the line's comment is `micold-client:<client_id>`).
    pub client_id: String,
    /// The key type and base64 body, `ssh-ed25519 <base64>`, without a comment.
    pub public_key: String,
}

/// The content of the access secret.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccessSecret {
    /// The pod's host key, generated once per daemon.
    pub host_key: KeyPair,
    /// The clients allowed to log in, one line each.
    pub authorized: Vec<AuthorizedKey>,
}

impl AccessSecret {
    /// Read the secret's data; `None` when a data key is missing.
    pub fn parse(data: &BTreeMap<String, String>) -> Option<Self> {
        let private_openssh = data.get(HOST_KEY)?.clone();
        let public_line = data.get(HOST_KEY_PUB)?.clone();
        let authorized = data
            .get(AUTHORIZED_KEYS)?
            .lines()
            .filter_map(parse_line)
            .collect();
        Some(Self {
            host_key: KeyPair {
                private_openssh,
                public_line,
            },
            authorized,
        })
    }

    /// The secret's data keys and values.
    pub fn to_data(&self) -> BTreeMap<String, String> {
        let authorized = self.authorized.iter().fold(String::new(), |mut out, key| {
            out.push_str(&format!(
                "{} {CLIENT_COMMENT}{}\n",
                key.public_key, key.client_id
            ));
            out
        });
        BTreeMap::from([
            (HOST_KEY.to_owned(), self.host_key.private_openssh.clone()),
            (HOST_KEY_PUB.to_owned(), self.host_key.public_line.clone()),
            (AUTHORIZED_KEYS.to_owned(), authorized),
        ])
    }

    /// With `public_key` authorised for `client_id`, replacing that client's line; the flag says
    /// whether anything changed (idempotent).
    pub fn with_client(mut self, client_id: &str, public_key: &str) -> (Self, bool) {
        match self
            .authorized
            .iter_mut()
            .find(|k| k.client_id == client_id)
        {
            Some(existing) if existing.public_key == public_key => return (self, false),
            Some(existing) => existing.public_key = public_key.to_owned(),
            None => self.authorized.push(AuthorizedKey {
                client_id: client_id.to_owned(),
                public_key: public_key.to_owned(),
            }),
        }
        (self, true)
    }

    /// Without `client_id`'s line; other clients' lines are untouched.
    pub fn without_client(mut self, client_id: &str) -> Self {
        self.authorized.retain(|k| k.client_id != client_id);
        self
    }
}

/// One `authorized_keys` line as a client's key; `None` for blank lines and foreign comments.
fn parse_line(line: &str) -> Option<AuthorizedKey> {
    let (public_key, id) = line.trim().rsplit_once(CLIENT_COMMENT)?;
    let public_key = public_key.trim();
    (!public_key.is_empty()).then(|| AuthorizedKey {
        client_id: id.trim().to_owned(),
        public_key: public_key.to_owned(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::kube::fake::FakeKeyGen;

    const KEY_A: &str = "ssh-ed25519 AAAAa";
    const KEY_B: &str = "ssh-ed25519 AAAAb";

    fn secret() -> AccessSecret {
        let host_key = FakeKeyGen::default().generate("host");
        AccessSecret {
            host_key,
            authorized: vec![],
        }
    }

    #[test]
    fn the_secret_keys_are_the_file_names_the_basic_images_sshd_reads() {
        // The secret is mounted at /run/micold/ssh, so its data keys are the file names.
        let sshd = include_str!("../../../../packaging/basic/sshd_config");
        let has = |line: String| sshd.lines().any(|l| l.trim() == line);
        assert!(
            has(format!("HostKey /run/micold/ssh/{HOST_KEY}")),
            "sshd_config names a different host key file than {HOST_KEY}"
        );
        assert!(
            has(format!(
                "AuthorizedKeysFile /run/micold/ssh/{AUTHORIZED_KEYS}"
            )),
            "sshd_config names a different authorized keys file than {AUTHORIZED_KEYS}"
        );
        assert_eq!(HOST_KEY_PUB, format!("{HOST_KEY}.pub"));
    }

    #[test]
    fn the_fake_key_gen_returns_fixed_pairs_in_order() {
        let gen = FakeKeyGen::default();
        let first = gen.generate("c");
        assert_eq!(first, FakeKeyGen::default().generate("c"));
        assert_ne!(first, gen.generate("c"), "the next pair differs");
        assert!(first.public_line.starts_with("ssh-ed25519 "));
        assert!(first.public_line.ends_with(" c"));
    }

    #[test]
    fn data_round_trips_with_one_line_per_client_commented_with_its_id() {
        let (s, _) = secret().with_client("c1", KEY_A);
        let (s, _) = s.with_client("c2", KEY_B);
        let data = s.to_data();
        assert_eq!(
            data.keys().map(String::as_str).collect::<Vec<_>>(),
            [AUTHORIZED_KEYS, HOST_KEY, HOST_KEY_PUB]
        );
        assert_eq!(
            data[AUTHORIZED_KEYS],
            format!("{KEY_A} micold-client:c1\n{KEY_B} micold-client:c2\n")
        );
        assert_eq!(data[HOST_KEY], s.host_key.private_openssh);
        assert_eq!(data[HOST_KEY_PUB], s.host_key.public_line);
        assert_eq!(AccessSecret::parse(&data), Some(s));
    }

    #[test]
    fn a_secret_missing_a_key_does_not_parse() {
        for key in [HOST_KEY, HOST_KEY_PUB, AUTHORIZED_KEYS] {
            let mut data = secret().to_data();
            data.remove(key);
            assert_eq!(AccessSecret::parse(&data), None, "{key}");
        }
    }

    #[test]
    fn with_client_is_idempotent_and_replaces_the_same_clients_line() {
        let (s, changed) = secret().with_client("c1", KEY_A);
        assert!(changed);
        let (s, changed) = s.with_client("c1", KEY_A);
        assert!(!changed, "already present: nothing to write");
        let (s, changed) = s.with_client("c1", KEY_B);
        assert!(changed);
        assert_eq!(
            s.authorized,
            [AuthorizedKey {
                client_id: "c1".into(),
                public_key: KEY_B.into()
            }]
        );
    }

    #[test]
    fn without_client_removes_only_that_line() {
        let (s, _) = secret().with_client("c1", KEY_A);
        let (s, _) = s.with_client("c2", KEY_B);
        let s = s.without_client("c1");
        assert_eq!(s.authorized.len(), 1);
        assert_eq!(s.authorized[0].client_id, "c2");
        assert_eq!(s.clone().without_client("nobody"), s);
    }
}
