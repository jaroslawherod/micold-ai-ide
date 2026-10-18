//! This client's SSH key files for one Kubernetes daemon (feature 686, FR-008, data model
//! "Client-side key files"). All of them owner-only (`micold_core::owner_only`), under the app's
//! data directory, one directory per daemon key; never in settings, the cluster or logs.

use std::io;
use std::path::{Path, PathBuf};

use micold_core::kube::access::KeyGen;
use micold_core::kube::start::ClientKey;
use micold_core::owner_only;

/// The private key file.
pub const PRIVATE_KEY: &str = "id_ed25519";
/// The public key file.
pub const PUBLIC_KEY: &str = "id_ed25519.pub";
/// The pinned host key file.
pub const KNOWN_HOSTS: &str = "known_hosts";
/// The file holding this client's id.
pub const CLIENT_ID: &str = "client_id";

/// The `HostKeyAlias` an SSH connection to the daemon uses and its `known_hosts` line names.
pub fn host_alias(daemon_key: &str) -> String {
    format!("micold-kube-{daemon_key}")
}

/// One daemon's key files.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KubeKeys {
    dir: PathBuf,
    daemon_key: String,
}

impl KubeKeys {
    /// The key files of `daemon_key` under `root` (the app's data directory for Kubernetes keys).
    pub fn new(root: &Path, daemon_key: &str) -> Self {
        // The key names a directory that `delete` removes: keep it one plain component.
        let component: String = daemon_key
            .chars()
            .map(|c| {
                if c.is_alphanumeric() || c == '-' {
                    c
                } else {
                    '_'
                }
            })
            .collect();
        Self {
            dir: root.join(if component.is_empty() {
                "_"
            } else {
                &component
            }),
            daemon_key: daemon_key.to_string(),
        }
    }

    /// The private key's path, for `ssh -i`.
    pub fn private_key(&self) -> PathBuf {
        self.dir.join(PRIVATE_KEY)
    }

    /// The `known_hosts` path.
    pub fn known_hosts(&self) -> PathBuf {
        self.dir.join(KNOWN_HOSTS)
    }

    /// This client's identity for the daemon: read from the files, or made and written when any of
    /// them is missing. `new_id` is called only to make one. The id and the key stay as they were
    /// on every later load.
    pub fn load_or_create(
        &self,
        keygen: &dyn KeyGen,
        new_id: impl FnOnce() -> String,
    ) -> io::Result<ClientKey> {
        let read = |file: &str| std::fs::read_to_string(self.dir.join(file));
        if let (Ok(id), Ok(public), Ok(_)) = (read(CLIENT_ID), read(PUBLIC_KEY), read(PRIVATE_KEY))
        {
            return Ok(ClientKey {
                id: id.trim().to_string(),
                public_key: key_without_comment(&public),
            });
        }
        let id = new_id();
        let pair = keygen.generate(&format!("micold-client:{id}"));
        owner_only::write(&self.dir, PRIVATE_KEY, pair.private_openssh.as_bytes())?;
        owner_only::write(&self.dir, PUBLIC_KEY, pair.public_line.as_bytes())?;
        owner_only::write(&self.dir, CLIENT_ID, id.as_bytes())?;
        Ok(ClientKey {
            id,
            public_key: key_without_comment(&pair.public_line),
        })
    }

    /// Pin the pod's host key: `known_hosts` becomes the one line
    /// `micold-kube-<daemon key> <host public key>`.
    pub fn write_known_hosts(&self, host_public_line: &str) -> io::Result<()> {
        let line = format!(
            "{} {}\n",
            host_alias(&self.daemon_key),
            key_without_comment(host_public_line)
        );
        owner_only::write(&self.dir, KNOWN_HOSTS, line.as_bytes()).map(|_| ())
    }

    /// Remove every file; none left is not an error.
    pub fn delete(&self) -> io::Result<()> {
        match std::fs::remove_dir_all(&self.dir) {
            Err(e) if e.kind() != io::ErrorKind::NotFound => Err(e),
            _ => Ok(()),
        }
    }
}

/// `<type> <base64>` of a public key line, dropping its comment.
fn key_without_comment(line: &str) -> String {
    line.split_whitespace()
        .take(2)
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use micold_core::kube::fake::FakeKeyGen;

    fn keys(root: &Path) -> KubeKeys {
        KubeKeys::new(root, "k1")
    }

    fn make(keys: &KubeKeys, id: &str) -> ClientKey {
        keys.load_or_create(&FakeKeyGen::default(), || id.to_string())
            .unwrap()
    }

    #[cfg(unix)]
    fn mode(path: &Path) -> u32 {
        use std::os::unix::fs::PermissionsExt;
        std::fs::metadata(path).unwrap().permissions().mode() & 0o777
    }

    #[cfg(unix)]
    #[test]
    fn every_file_and_the_directory_are_owner_only() {
        let root = tempfile::tempdir().unwrap();
        let keys = keys(root.path());
        make(&keys, "c1");
        keys.write_known_hosts("ssh-ed25519 HOST pod").unwrap();
        for file in [PRIVATE_KEY, PUBLIC_KEY, KNOWN_HOSTS, CLIENT_ID] {
            assert_eq!(mode(&root.path().join("k1").join(file)), 0o600, "{file}");
        }
        assert_eq!(mode(&root.path().join("k1")), 0o700);
    }

    #[test]
    fn known_hosts_is_one_line_naming_the_alias_and_the_key_without_its_comment() {
        let root = tempfile::tempdir().unwrap();
        let keys = keys(root.path());
        keys.write_known_hosts("ssh-ed25519 HOSTKEY micold-host-x")
            .unwrap();
        let text = std::fs::read_to_string(keys.known_hosts()).unwrap();
        assert_eq!(text, "micold-kube-k1 ssh-ed25519 HOSTKEY\n");
        keys.write_known_hosts("ssh-ed25519 OTHER").unwrap();
        let text = std::fs::read_to_string(keys.known_hosts()).unwrap();
        assert_eq!(
            text, "micold-kube-k1 ssh-ed25519 OTHER\n",
            "replaced, not appended"
        );
    }

    #[test]
    fn the_client_id_and_key_are_stable_across_loads() {
        let root = tempfile::tempdir().unwrap();
        let first = make(&keys(root.path()), "c1");
        let again = keys(root.path())
            .load_or_create(&FakeKeyGen::default(), || panic!("must not make a new id"))
            .unwrap();
        assert_eq!(first, again);
        assert_eq!(first.id, "c1");
        assert!(first.public_key.starts_with("ssh-ed25519 "));
        assert_eq!(first.public_key.split(' ').count(), 2, "no comment");
    }

    #[test]
    fn a_missing_file_makes_the_whole_identity_again() {
        let root = tempfile::tempdir().unwrap();
        let keys = keys(root.path());
        make(&keys, "c1");
        std::fs::remove_file(root.path().join("k1").join(PRIVATE_KEY)).unwrap();
        assert_eq!(make(&keys, "c2").id, "c2");
        assert!(keys.private_key().exists());
    }

    #[test]
    fn delete_removes_every_file_and_a_second_delete_is_fine() {
        let root = tempfile::tempdir().unwrap();
        let keys = keys(root.path());
        make(&keys, "c1");
        keys.write_known_hosts("ssh-ed25519 H").unwrap();
        keys.delete().unwrap();
        assert!(!root.path().join("k1").exists());
        keys.delete().unwrap();
    }

    #[test]
    fn another_daemons_files_are_untouched_by_delete() {
        let root = tempfile::tempdir().unwrap();
        let other = KubeKeys::new(root.path(), "k2");
        make(&other, "c9");
        keys(root.path()).delete().unwrap();
        assert!(other.private_key().exists());
    }
}
