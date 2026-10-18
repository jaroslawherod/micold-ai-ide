//! What a Kubernetes daemon may take from this machine (feature 686, FR-008). Each allowed entry
//! becomes exactly one secret and one mount at start (US3); nothing is copied unless listed.

use std::collections::BTreeMap;
use std::io;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::kube::classify::KubeFailure;
use crate::session::AiCli;

/// Where the allowed secrets that are not single files are mounted.
pub const CREDS_DIR: &str = "/run/micold/creds";

/// The daemon's `HOME` in the pod, on the volume.
pub const HOME_DIR: &str = "/data/home";

/// Where the daemon keeps its clones (research K8).
pub const REPOS_ROOT: &str = "/data/repos";

/// The `type` of a registry pull secret.
pub const DOCKER_CONFIG_TYPE: &str = "kubernetes.io/dockerconfigjson";

/// Reads a source file of a credential on this machine; the client passes its real reader (M5).
pub type ReadFile<'a> = &'a (dyn Fn(&Path) -> io::Result<String> + Send + Sync);

/// Where an allowed secret appears in the container, read-only.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Mount {
    /// The path in the container.
    pub path: String,
    /// The secret key mounted as that one file; `None` mounts the whole secret as a directory.
    pub sub_path: Option<String>,
}

/// One credential the user allowed a Kubernetes daemon to receive.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum KubeCredential {
    /// `~/.gitconfig`.
    GitConfig,
    /// `~/.git-credentials`.
    GitCredentials,
    /// One SSH private key.
    GitSshKey {
        /// The key file on this machine.
        key: PathBuf,
    },
    /// One AI CLI's sign-in.
    SignIn {
        /// Which CLI.
        cli: AiCli,
    },
    /// A registry `auths` file for pulling a private image.
    RegistryPull {
        /// The docker-config JSON on this machine.
        auths: PathBuf,
    },
}

impl KubeCredential {
    /// The credential as the form and the failures name it.
    pub fn label(&self) -> String {
        match self {
            Self::GitConfig => "the git config".into(),
            Self::GitCredentials => "the git credentials".into(),
            Self::GitSshKey { key } => format!("the SSH key {}", key.display()),
            Self::SignIn { cli } => format!("the {} sign-in", cli.provider().display_name()),
            Self::RegistryPull { .. } => "the registry credentials".into(),
        }
    }

    /// The secret's `type`.
    pub fn secret_type(&self) -> &'static str {
        match self {
            Self::RegistryPull { .. } => DOCKER_CONFIG_TYPE,
            _ => "Opaque",
        }
    }

    /// The file of a sign-in inside the pod, and its name as the secret's key.
    fn sign_in_file(cli: AiCli) -> Option<(String, String)> {
        let path = cli.provider().sandbox_auth_file(Path::new(HOME_DIR))?;
        let name = path.file_name()?.to_string_lossy().into_owned();
        Some((path.to_string_lossy().into_owned(), name))
    }

    /// Where the secret is mounted; `None` for a pull secret, which is no mount.
    pub fn mount(&self) -> Option<Mount> {
        let file = |path: String, key: &str| Mount {
            path,
            sub_path: Some(key.to_string()),
        };
        match self {
            Self::GitCredentials => Some(file(
                format!("{CREDS_DIR}/git-credentials"),
                "git-credentials",
            )),
            Self::GitSshKey { .. } => Some(Mount {
                path: format!("{CREDS_DIR}/ssh"),
                sub_path: None,
            }),
            Self::GitConfig => Some(file(format!("{HOME_DIR}/.gitconfig"), "gitconfig")),
            Self::SignIn { cli } => Self::sign_in_file(*cli).map(|(path, key)| file(path, &key)),
            Self::RegistryPull { .. } => None,
        }
    }

    /// The secret's text data, read from this machine through `read`. `home` is this machine's home
    /// directory; `hosts` are the remote hosts whose `known_hosts` lines go with an SSH key.
    pub fn secret_data(
        &self,
        home: &Path,
        hosts: &[String],
        read: ReadFile<'_>,
    ) -> Result<BTreeMap<String, String>, KubeFailure> {
        let source = |path: &Path| {
            read(path).map_err(|_| KubeFailure::CredentialSource {
                credential: self.label(),
                path: path.to_path_buf(),
            })
        };
        let one = |key: &str, text: String| BTreeMap::from([(key.to_string(), text)]);
        match self {
            Self::GitConfig => Ok(one("gitconfig", source(&home.join(".gitconfig"))?)),
            Self::GitCredentials => Ok(one(
                "git-credentials",
                source(&home.join(".git-credentials"))?,
            )),
            Self::GitSshKey { key } => {
                let known = source(&home.join(".ssh").join("known_hosts"))?;
                Ok(BTreeMap::from([
                    ("id".to_string(), source(key)?),
                    ("known_hosts".to_string(), known_hosts_for(&known, hosts)),
                ]))
            }
            Self::SignIn { cli } => {
                let (_, key) =
                    Self::sign_in_file(*cli).ok_or_else(|| KubeFailure::CredentialSource {
                        credential: self.label(),
                        path: home.to_path_buf(),
                    })?;
                let path = cli.provider().sandbox_auth_file(home).unwrap_or_default();
                Ok(one(&key, source(&path)?))
            }
            Self::RegistryPull { auths } => Ok(one(".dockerconfigjson", source(auths)?)),
        }
    }
}

/// One of each kind of credential, to find the secrets a withdrawn one left behind (the names do
/// not depend on the payload).
pub fn every_kind() -> Vec<KubeCredential> {
    let mut all = vec![
        KubeCredential::GitConfig,
        KubeCredential::GitCredentials,
        KubeCredential::GitSshKey {
            key: PathBuf::new(),
        },
        KubeCredential::RegistryPull {
            auths: PathBuf::new(),
        },
    ];
    all.extend(AiCli::ALL.map(|cli| KubeCredential::SignIn { cli }));
    all
}

/// The lines of `known_hosts` that name one of `hosts` (hashed and wildcard entries never match).
pub fn known_hosts_for(known_hosts: &str, hosts: &[String]) -> String {
    let names_host = |field: &str| {
        field.split(',').any(|entry| {
            let entry = entry.strip_prefix('[').unwrap_or(entry);
            let host = entry.split(']').next().unwrap_or(entry);
            hosts.iter().any(|h| h == host)
        })
    };
    known_hosts
        .lines()
        .filter(|line| {
            let line = line.trim();
            !line.starts_with(['#', '|', '@'])
                && line.split_whitespace().next().is_some_and(names_host)
        })
        .map(|line| format!("{line}\n"))
        .collect()
}

/// The environment the daemon runs with for the `allowed` credentials (the pod's own env does not
/// reach it: sshd starts sessions clean). Nothing is added for a credential that needs none.
pub fn daemon_env(
    allowed: &std::collections::BTreeSet<KubeCredential>,
) -> BTreeMap<String, String> {
    let mut env = BTreeMap::from([
        ("MICOLD_IDLE_STOP".to_string(), "off".to_string()),
        ("MICOLD_REPOS_ROOT".to_string(), REPOS_ROOT.to_string()),
    ]);
    if allowed.contains(&KubeCredential::GitCredentials) {
        env.insert("GIT_CONFIG_COUNT".into(), "1".into());
        env.insert("GIT_CONFIG_KEY_0".into(), "credential.helper".into());
        env.insert(
            "GIT_CONFIG_VALUE_0".into(),
            format!("store --file {CREDS_DIR}/git-credentials"),
        );
    }
    if allowed
        .iter()
        .any(|c| matches!(c, KubeCredential::GitSshKey { .. }))
    {
        env.insert(
            "GIT_SSH_COMMAND".into(),
            format!(
                "ssh -i {CREDS_DIR}/ssh/id -o IdentitiesOnly=yes \
                 -o UserKnownHostsFile={CREDS_DIR}/ssh/known_hosts -o StrictHostKeyChecking=yes"
            ),
        );
    }
    env
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::*;
    use crate::kube::names::ResourceNames;

    fn names() -> ResourceNames {
        ResourceNames::new("kind-dev", "dev", "Build box")
    }

    fn reader(files: &[(&str, &str)]) -> impl Fn(&Path) -> io::Result<String> + Send + Sync {
        let files: BTreeMap<PathBuf, String> = files
            .iter()
            .map(|(p, t)| (PathBuf::from(p), t.to_string()))
            .collect();
        move |path| {
            files
                .get(path)
                .cloned()
                .ok_or_else(|| io::Error::from(io::ErrorKind::NotFound))
        }
    }

    fn mount(path: &str, sub: Option<&str>) -> Option<Mount> {
        Some(Mount {
            path: path.into(),
            sub_path: sub.map(str::to_string),
        })
    }

    #[test]
    fn each_credential_has_one_secret_and_one_mount() {
        let ssh = KubeCredential::GitSshKey {
            key: "/k/id".into(),
        };
        let claude = KubeCredential::SignIn {
            cli: AiCli::ClaudeCode,
        };
        assert_eq!(
            KubeCredential::GitCredentials.mount(),
            mount("/run/micold/creds/git-credentials", Some("git-credentials"))
        );
        assert_eq!(ssh.mount(), mount("/run/micold/creds/ssh", None));
        assert_eq!(
            KubeCredential::GitConfig.mount(),
            mount("/data/home/.gitconfig", Some("gitconfig"))
        );
        assert_eq!(
            claude.mount(),
            mount(
                "/data/home/.claude/.credentials.json",
                Some(".credentials.json")
            )
        );
        let pull = KubeCredential::RegistryPull { auths: "/a".into() };
        assert_eq!(pull.mount(), None);
        assert_eq!(pull.secret_type(), "kubernetes.io/dockerconfigjson");
        assert_eq!(names().secret(&pull), format!("{}-pull", names().pod()));
        assert_eq!(ssh.secret_type(), "Opaque");
    }

    #[test]
    fn secret_data_is_read_from_the_sources() {
        let read = reader(&[
            ("/h/.gitconfig", "[user]"),
            ("/h/.git-credentials", "https://u:p@github.com"),
            ("/k/id", "KEY"),
            (
                "/h/.ssh/known_hosts",
                "github.com ssh-ed25519 AAA\ngitlab.com ssh-ed25519 BBB\n",
            ),
            ("/h/.claude/.credentials.json", "{}"),
            ("/a", "{\"auths\":{}}"),
        ]);
        let hosts = vec!["github.com".to_string()];
        let data = |c: KubeCredential| c.secret_data(Path::new("/h"), &hosts, &read).unwrap();
        let one = |k: &str, v: &str| BTreeMap::from([(k.to_string(), v.to_string())]);
        assert_eq!(data(KubeCredential::GitConfig), one("gitconfig", "[user]"));
        assert_eq!(
            data(KubeCredential::GitCredentials),
            one("git-credentials", "https://u:p@github.com")
        );
        assert_eq!(
            data(KubeCredential::SignIn {
                cli: AiCli::ClaudeCode
            }),
            one(".credentials.json", "{}")
        );
        assert_eq!(
            data(KubeCredential::RegistryPull { auths: "/a".into() }),
            one(".dockerconfigjson", "{\"auths\":{}}")
        );
        let ssh = data(KubeCredential::GitSshKey {
            key: "/k/id".into(),
        });
        assert_eq!(ssh["id"], "KEY");
        assert_eq!(ssh["known_hosts"], "github.com ssh-ed25519 AAA\n");
    }

    #[test]
    fn known_hosts_keep_only_the_remote_hosts_plain_lines() {
        let file = "# c\n|1|hash|x ssh-ed25519 H\n*.com ssh-rsa W\ngit.example.com,10.0.0.5 ssh-rsa A\n[git.example.com]:2222 ssh-rsa B\nother ssh-rsa C\n";
        let kept = known_hosts_for(file, &["git.example.com".to_string()]);
        assert_eq!(
            kept,
            "git.example.com,10.0.0.5 ssh-rsa A\n[git.example.com]:2222 ssh-rsa B\n"
        );
        assert_eq!(known_hosts_for(file, &[]), "");
    }

    #[test]
    fn a_missing_source_names_the_credential_and_the_path() {
        let read = reader(&[]);
        let failure = KubeCredential::GitConfig
            .secret_data(Path::new("/h"), &[], &read)
            .unwrap_err();
        assert_eq!(
            failure,
            KubeFailure::CredentialSource {
                credential: "the git config".into(),
                path: "/h/.gitconfig".into()
            }
        );
        let no_file = KubeCredential::SignIn {
            cli: AiCli::Copilot,
        };
        assert!(no_file.mount().is_none());
        assert!(matches!(
            no_file.secret_data(Path::new("/h"), &[], &read),
            Err(KubeFailure::CredentialSource { .. })
        ));
    }

    #[test]
    fn there_is_no_ssh_agent_credential() {
        let json = serde_json::to_string(&every_kind()).unwrap();
        assert!(!json.contains("agent"), "{json}");
        assert_eq!(every_kind().len(), 4 + AiCli::ALL.len());
    }

    #[test]
    fn the_daemon_env_is_what_the_allowed_credentials_need() {
        let env = |c: &[KubeCredential]| daemon_env(&c.iter().cloned().collect::<BTreeSet<_>>());
        let base = env(&[]);
        assert_eq!(
            base,
            BTreeMap::from([
                ("MICOLD_IDLE_STOP".to_string(), "off".to_string()),
                ("MICOLD_REPOS_ROOT".to_string(), "/data/repos".to_string()),
            ])
        );
        assert_eq!(
            env(&[
                KubeCredential::GitConfig,
                KubeCredential::RegistryPull { auths: "/a".into() }
            ]),
            base
        );
        let store = env(&[KubeCredential::GitCredentials]);
        assert_eq!(store["GIT_CONFIG_COUNT"], "1");
        assert_eq!(store["GIT_CONFIG_KEY_0"], "credential.helper");
        assert_eq!(
            store["GIT_CONFIG_VALUE_0"],
            "store --file /run/micold/creds/git-credentials"
        );
        assert!(!store.contains_key("GIT_SSH_COMMAND"));
        let ssh = env(&[KubeCredential::GitSshKey { key: "/k".into() }]);
        let cmd = &ssh["GIT_SSH_COMMAND"];
        assert!(cmd.contains("IdentitiesOnly=yes") && cmd.contains("StrictHostKeyChecking=yes"));
        assert!(cmd.contains("-i /run/micold/creds/ssh/id"));
        assert!(!ssh.contains_key("GIT_CONFIG_COUNT"));
    }
}
