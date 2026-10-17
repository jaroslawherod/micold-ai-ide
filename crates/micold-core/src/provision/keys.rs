//! The SSH keys of one app-owned daemon target (feature 806, FR-006, data-model *DaemonKeys*).
//!
//! The client creates a client key and a host key per daemon, authorises the client key in the
//! target and pins the host key, so the target is reached with no password, no key of the user's
//! and no agent. A daemon's keys live in its own directory and are shared with no other daemon
//! (Principle II).

use std::io;
use std::path::{Path, PathBuf};

use super::failure::{FailureCause, ProvisionFailure, Step};
use crate::owner_only;
use crate::ssh::argv::{ClientPlatform, SshCommand};
use crate::ssh::{SshFailure, SshRunner};

/// The directory holding one daemon's keys and the files derived from them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DaemonKeys {
    /// `<daemon state dir>/ssh`.
    pub dir: PathBuf,
}

/// `ssh-keygen -t ed25519 -N "" -q -f <path>`: an unencrypted key, quietly.
pub fn keygen_argv(platform: ClientPlatform, path: &Path) -> SshCommand {
    let program = match platform {
        ClientPlatform::Windows => "ssh-keygen.exe",
        _ => "ssh-keygen",
    };
    SshCommand {
        program: program.to_string(),
        args: ["-t", "ed25519", "-N", "", "-q", "-f"]
            .iter()
            .map(|s| s.to_string())
            .chain([path.display().to_string()])
            .collect(),
        env: vec![("LC_ALL".into(), "C".into())],
    }
}

impl DaemonKeys {
    /// The client's private key.
    pub fn client_key(&self) -> PathBuf {
        self.dir.join("client_ed25519")
    }
    /// The client's public key.
    pub fn client_pub(&self) -> PathBuf {
        self.dir.join("client_ed25519.pub")
    }
    /// The target's SSH host private key (mounted read-only into the target).
    pub fn host_key(&self) -> PathBuf {
        self.dir.join("host_ed25519")
    }
    /// The target's SSH host public key.
    pub fn host_pub(&self) -> PathBuf {
        self.dir.join("host_ed25519.pub")
    }
    /// The file of authorised client keys (mounted read-only into the target).
    pub fn authorized(&self) -> PathBuf {
        self.dir.join("authorized_keys")
    }
    /// The client's own `known_hosts` for this target.
    pub fn known_hosts(&self) -> PathBuf {
        self.dir.join("known_hosts")
    }

    /// Create the keys that are missing and the files derived from them. An existing pair is never
    /// overwritten; a half pair is replaced as a whole. `runner` runs `ssh-keygen`.
    pub async fn ensure(
        dir: &Path,
        platform: ClientPlatform,
        runner: &dyn SshRunner,
    ) -> Result<DaemonKeys, ProvisionFailure> {
        let keys = DaemonKeys {
            dir: dir.to_path_buf(),
        };
        owner_only::ensure_dir(dir).map_err(|e| failure(&e.to_string()))?;
        for (key, public) in [
            (keys.client_key(), keys.client_pub()),
            (keys.host_key(), keys.host_pub()),
        ] {
            if key.exists() && public.exists() {
                continue;
            }
            let _ = std::fs::remove_file(&key);
            let _ = std::fs::remove_file(&public);
            let out = runner
                .run(keygen_argv(platform, &key), Vec::new(), None, None)
                .await
                .map_err(|e| failure(&e.to_string()))?;
            if out.status != Some(0) || !key.exists() || !public.exists() {
                return Err(failure(out.stderr.trim()));
            }
            restrict(&key).map_err(|e| failure(&e.to_string()))?;
        }
        let client_pub =
            std::fs::read_to_string(keys.client_pub()).map_err(|e| failure(&e.to_string()))?;
        owner_only::write(dir, "authorized_keys", client_pub.as_bytes())
            .map_err(|e| failure(&e.to_string()))?;
        Ok(keys)
    }

    /// Rewrite `known_hosts` as `[127.0.0.1]:<port> <host public key>`: the one host key this
    /// target presents, on the port it is published on now.
    pub fn write_known_hosts(&self, port: u16) -> io::Result<()> {
        let public = std::fs::read_to_string(self.host_pub())?;
        // `<type> <base64> [comment]`: a known_hosts line has no comment.
        let key: Vec<&str> = public.split_whitespace().take(2).collect();
        let line = format!("[127.0.0.1]:{port} {}\n", key.join(" "));
        owner_only::write(&self.dir, "known_hosts", line.as_bytes()).map(|_| ())
    }
}

fn failure(detail: &str) -> ProvisionFailure {
    ProvisionFailure::new(
        Step::CreateContainer,
        FailureCause::Ssh(SshFailure::Other {
            tail: detail.lines().last().unwrap_or("").to_string(),
        }),
    )
    .with_output(detail)
}

/// A private key is readable by its owner only. On Windows the directory's owner-only ACL is
/// inherited by the file.
fn restrict(path: &Path) -> io::Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))
    }
    #[cfg(not(unix))]
    {
        let _ = path;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ssh::{ProvisionProgress, SshOutput, SshStdio};
    use futures_util::future::BoxFuture;
    use std::sync::Mutex;
    use std::time::Duration;

    /// Plays `ssh-keygen`: writes the key and `.pub` named by the `-f` argument, readable by all.
    #[derive(Default)]
    struct FakeKeygen {
        runs: Mutex<Vec<Vec<String>>>,
    }

    impl SshRunner for FakeKeygen {
        fn run(
            &self,
            cmd: SshCommand,
            _stdin: Vec<u8>,
            _deadline: Option<Duration>,
            _progress: Option<ProvisionProgress>,
        ) -> BoxFuture<'_, io::Result<SshOutput>> {
            let path = PathBuf::from(cmd.args.last().unwrap());
            let n = {
                let mut runs = self.runs.lock().unwrap();
                runs.push(cmd.args.clone());
                runs.len()
            };
            std::fs::write(&path, format!("PRIVATE-{n}")).unwrap();
            let mut public = path.clone().into_os_string();
            public.push(".pub");
            std::fs::write(public, format!("ssh-ed25519 KEY{n} comment@host\n")).unwrap();
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o644)).unwrap();
            }
            Box::pin(async {
                Ok(SshOutput {
                    status: Some(0),
                    ..SshOutput::default()
                })
            })
        }
        fn open(&self, _cmd: SshCommand) -> io::Result<SshStdio> {
            Err(io::Error::other("not used"))
        }
    }

    #[test]
    fn keygen_is_ed25519_with_no_passphrase_and_quiet() {
        let cmd = keygen_argv(ClientPlatform::Linux, Path::new("/k/client_ed25519"));
        assert_eq!(cmd.program, "ssh-keygen");
        assert_eq!(
            cmd.args,
            ["-t", "ed25519", "-N", "", "-q", "-f", "/k/client_ed25519"]
        );
        assert_eq!(
            keygen_argv(ClientPlatform::Windows, Path::new("k")).program,
            "ssh-keygen.exe"
        );
    }

    #[tokio::test]
    async fn ensure_creates_both_pairs_once_and_never_overwrites() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().join("ssh");
        let runner = FakeKeygen::default();
        let keys = DaemonKeys::ensure(&dir, ClientPlatform::Linux, &runner)
            .await
            .unwrap();
        for path in [
            keys.client_key(),
            keys.client_pub(),
            keys.host_key(),
            keys.host_pub(),
            keys.authorized(),
        ] {
            assert!(path.exists(), "{path:?}");
        }
        let before = std::fs::read_to_string(keys.client_key()).unwrap();
        DaemonKeys::ensure(&dir, ClientPlatform::Linux, &runner)
            .await
            .unwrap();
        assert_eq!(runner.runs.lock().unwrap().len(), 2, "no second keygen");
        assert_eq!(std::fs::read_to_string(keys.client_key()).unwrap(), before);
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn private_keys_are_owner_only() {
        use std::os::unix::fs::PermissionsExt;
        let tmp = tempfile::tempdir().unwrap();
        let keys = DaemonKeys::ensure(
            &tmp.path().join("ssh"),
            ClientPlatform::Linux,
            &FakeKeygen::default(),
        )
        .await
        .unwrap();
        for key in [keys.client_key(), keys.host_key()] {
            let mode = std::fs::metadata(&key).unwrap().permissions().mode();
            assert_eq!(mode & 0o777, 0o600, "{key:?}");
        }
    }

    #[tokio::test]
    async fn authorized_keys_holds_exactly_the_client_public_key() {
        let tmp = tempfile::tempdir().unwrap();
        let keys = DaemonKeys::ensure(
            &tmp.path().join("ssh"),
            ClientPlatform::Linux,
            &FakeKeygen::default(),
        )
        .await
        .unwrap();
        assert_eq!(
            std::fs::read_to_string(keys.authorized()).unwrap(),
            std::fs::read_to_string(keys.client_pub()).unwrap()
        );
    }

    #[tokio::test]
    async fn known_hosts_follows_the_port_and_has_no_comment() {
        let tmp = tempfile::tempdir().unwrap();
        let keys = DaemonKeys::ensure(
            &tmp.path().join("ssh"),
            ClientPlatform::Linux,
            &FakeKeygen::default(),
        )
        .await
        .unwrap();
        keys.write_known_hosts(40001).unwrap();
        keys.write_known_hosts(40002).unwrap();
        // The host pair is the second one generated.
        assert_eq!(
            std::fs::read_to_string(keys.known_hosts()).unwrap(),
            "[127.0.0.1]:40002 ssh-ed25519 KEY2\n"
        );
    }

    #[tokio::test]
    async fn two_daemons_share_no_file() {
        let tmp = tempfile::tempdir().unwrap();
        let runner = FakeKeygen::default();
        let a = DaemonKeys::ensure(&tmp.path().join("a/ssh"), ClientPlatform::Linux, &runner)
            .await
            .unwrap();
        let b = DaemonKeys::ensure(&tmp.path().join("b/ssh"), ClientPlatform::Linux, &runner)
            .await
            .unwrap();
        let files = |k: &DaemonKeys| {
            [
                k.client_key(),
                k.client_pub(),
                k.host_key(),
                k.host_pub(),
                k.authorized(),
            ]
        };
        for fa in files(&a) {
            assert!(!files(&b).contains(&fa));
        }
        assert_ne!(
            std::fs::read_to_string(a.client_key()).unwrap(),
            std::fs::read_to_string(b.client_key()).unwrap()
        );
    }
}
